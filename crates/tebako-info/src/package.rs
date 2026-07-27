//! Inspection of a packed binary (spec 15 §3): the tpkg container AND its
//! slot payloads. The trailer comes from crates/tpkg (byte-parity); each
//! slot image is read IN PLACE through the tfs mount-from-region — nothing
//! is extracted. Format per slot is auto-detected through the detection
//! chain (the trailer's `format_id` is a hint; `auto` means detect, and
//! the v1 `format_id = 4` is reported as `runtime (legacy role)`).

use std::path::{Path, PathBuf};

use tebako_json::Value as Json;

use crate::format::{hint_json_name, hint_name};
use crate::manifest_json::manifest_to_json;
use crate::payload::{self, PayloadInspection};
use crate::render::kind_name;
use crate::{err, thousands, InfoError, INFO_SCHEMA};

/// Inspection depth (spec 15 §3 `--depth`):
/// 0 = trailer only; 1 = + slot manifests (default with `--full`);
/// 2 = + backend metadata per slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Depth {
    /// Trailer only — no region mounts.
    Trailer = 0,
    /// Trailer + slot payload manifests.
    Manifests = 1,
    /// Trailer + manifests + backend metadata per slot.
    Backend = 2,
}

impl Depth {
    /// Parse the `--depth` value.
    pub fn parse(text: &str) -> Result<Depth, InfoError> {
        match text {
            "0" => Ok(Depth::Trailer),
            "1" => Ok(Depth::Manifests),
            "2" => Ok(Depth::Backend),
            other => Err(err(format!(
                "invalid --depth value {other:?} (want 0, 1 or 2)"
            ))),
        }
    }
}

/// One slot of the container report.
#[derive(Debug)]
pub struct SlotInspection {
    /// Slot index.
    pub index: usize,
    /// Absolute file offset of the image.
    pub offset: u64,
    /// Image size in bytes.
    pub size: u64,
    /// The trailer's `format_id` hint.
    pub format_hint: u32,
    /// Declared mount point.
    pub mount: String,
    /// Payload inspection (depth ≥ 1), or the named mount failure inside.
    pub payload: Option<PayloadInspection>,
}

/// The container inspection.
#[derive(Debug)]
pub struct PackageInspection {
    /// The binary as given.
    pub path: PathBuf,
    /// Total file size.
    pub size_bytes: u64,
    /// The parsed trailer.
    pub trailer: tpkg::Manifest,
    /// Trailer bytes on disk (header + slot table + v2 extension).
    pub trailer_bytes: u64,
    /// Bytes before slot 0 (the bootstrap portion).
    pub bootstrap_bytes: u64,
    /// Structural validation error of the trailer (displayed, not fatal).
    pub trailer_validation: Option<String>,
    /// Per-slot inspections.
    pub slots: Vec<SlotInspection>,
}

impl PackageInspection {
    /// The package name for the report header: the file stem.
    pub fn package_name(&self) -> String {
        self.path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    /// True when the v2 chain-of-trust extension is present.
    pub fn is_signed(&self) -> bool {
        self.trailer.v2.is_some()
    }

    /// The trust state as stored (`v2-signed` / `unsigned`), the signer
    /// keyid (v2) and the outcome label (`unverified` until `--verify`
    /// ran — spec 15 §3).
    pub fn trust(&self) -> (String, Option<String>) {
        match &self.trailer.v2 {
            Some(v2) => ("v2-signed".to_string(), Some(v2.signer_keyid_hex())),
            None => ("unsigned (v1 legacy trailer)".to_string(), None),
        }
    }
}

/// Parse the trailer of `binary` (named errors; `NoTrailer` is its own
/// message — the container checks are `--verify`'s job).
pub fn read_trailer(binary: &Path) -> Result<tpkg::Manifest, InfoError> {
    let mut f = std::fs::File::open(binary)
        .map_err(|e| err(format!("{}: cannot read file ({e})", binary.display())))?;
    match tpkg::read_from(&mut f) {
        Ok(m) => Ok(m),
        Err(tpkg::TpkgError::NoTrailer) => Err(err(format!(
            "{}: no tpkg trailer present (not a three-part package)",
            binary.display()
        ))),
        Err(e) => Err(err(format!(
            "{}: {}",
            binary.display(),
            tpkg::strerror(e.code())
        ))),
    }
}

/// Inspect a packed binary at `depth`; `only_slot` restricts the payload
/// reads to one slot (`--slot N`).
pub fn inspect_package(
    binary: &Path,
    depth: Depth,
    only_slot: Option<usize>,
) -> Result<PackageInspection, InfoError> {
    let size_bytes = std::fs::metadata(binary)
        .map_err(|e| err(format!("{}: cannot stat ({e})", binary.display())))?
        .len();
    let trailer = read_trailer(binary)?;
    let trailer_validation = trailer
        .validate()
        .err()
        .map(|e| tpkg::strerror(e.code()).to_string());
    let bootstrap_bytes = trailer.slots.first().map_or(0, |s| s.offset);
    let trailer_bytes = tpkg::trailer_len(&trailer);

    let mut slots = Vec::with_capacity(trailer.slots.len());
    for (i, s) in trailer.slots.iter().enumerate() {
        let mount = s.mount_point_str().unwrap_or_default().to_string();
        // Runtime payload slots (the v1 legacy role) are never mounted —
        // they are launchers, not image payloads (spec 02 §5, spec 15 §3).
        let payload = if depth >= Depth::Manifests
            && s.format_id != tpkg::TPKG_FORMAT_RUNTIME
            && only_slot.map_or(true, |n| n == i)
        {
            let display = format!("{}[slot {i}]", binary.display());
            Some(payload::inspect_region(binary, s.offset, s.size, display)?)
        } else {
            None
        };
        slots.push(SlotInspection {
            index: i,
            offset: s.offset,
            size: s.size,
            format_hint: s.format_id,
            mount,
            payload,
        });
    }

    Ok(PackageInspection {
        path: binary.to_path_buf(),
        size_bytes,
        trailer,
        trailer_bytes,
        bootstrap_bytes,
        trailer_validation,
        slots,
    })
}

// ---------------------------------------------------------------------
// The full container report (spec 15 §3)
// ---------------------------------------------------------------------

fn slot_format_label(slot: &SlotInspection) -> String {
    if slot.format_hint == tpkg::TPKG_FORMAT_RUNTIME {
        return "runtime (legacy role)".to_string();
    }
    match &slot.payload {
        Some(p) => match (&p.format, &p.mount_error) {
            (Some(f), _) => f.short.clone(),
            (None, Some(_)) => format!("{} (undetected)", hint_name(slot.format_hint)),
            (None, None) => hint_name(slot.format_hint).to_string(),
        },
        None => hint_name(slot.format_hint).to_string(),
    }
}

fn slot_summary_line(p: &PayloadInspection) -> String {
    if let Some(m) = &p.manifest {
        let id = &m.identity;
        let detail = match &m.provides {
            tpkg::Provides::App(app) => {
                let eps = &app.entrypoints;
                let plural = if eps.len() == 1 {
                    "entrypoint"
                } else {
                    "entrypoints"
                };
                let runtimes = eps
                    .iter()
                    .map(|e| match &e.runtime_requirement {
                        Some(req) => format!("{} {}", req.engine, req.constraint),
                        None => "native".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("  ({} {plural}, runtime {runtimes})", eps.len())
            }
            tpkg::Provides::Runtime(rt) => {
                let engines = rt
                    .provides
                    .iter()
                    .map(|e| {
                        format!(
                            "{} {} abi {} {}",
                            e.engine,
                            e.version,
                            e.abi_line,
                            e.platform.as_triplet()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("  ({engines})")
            }
            tpkg::Provides::Data(data) => {
                format!("  (suggested mount {})", data.mount_semantics.suggested)
            }
            tpkg::Provides::Other(_) => String::new(),
        };
        return format!(
            "     kind: {}  {} {}{}\n",
            kind_name(id.kind),
            id.name,
            id.version,
            detail
        );
    }
    if let Some(err) = &p.manifest_validation {
        return format!("     manifest: invalid ({err})\n");
    }
    if let Some(err) = &p.mount_error {
        return format!("     unreadable: {err}\n");
    }
    "     (no payload manifest — plain image)\n".to_string()
}

/// The `--full` container report. `trust_outcome` replaces the stored
/// `unverified` label when `--verify` ran (spec 15 §3).
pub fn render_full(p: &PackageInspection, depth: Depth, trust_outcome: Option<&str>) -> String {
    let mut out = String::new();
    let lean = if p.trailer.is_lean() { ", lean" } else { "" };
    out.push_str(&format!(
        "package: {} (tpkg v{}{}, launcher_abi {})\n",
        p.package_name(),
        p.trailer.version,
        lean,
        p.trailer.launcher_abi
    ));
    let n_slots = p.trailer.slots.len();
    let slot_word = if n_slots == 1 { "slot" } else { "slots" };
    let v2_bytes = p
        .trailer
        .v2
        .as_ref()
        .map(|v2| tpkg::TPKG_V2_EXT_FIXED + v2.signature.len());
    let trailer_breakdown = match v2_bytes {
        Some(v2b) => format!(
            "166 header + {n_slots} {slot_word} × {} + {v2b} v2 extension",
            tpkg::TPKG_SLOT_SIZE
        ),
        None => format!(
            "166 header + {n_slots} {slot_word} × {}",
            tpkg::TPKG_SLOT_SIZE
        ),
    };
    out.push_str(&format!(
        "  size: {} B  trailer: {} B ({})\n",
        thousands(p.size_bytes),
        thousands(p.trailer_bytes),
        trailer_breakdown
    ));
    out.push_str(&format!(
        "  bootstrap: {} B (portion before slot 0)\n",
        thousands(p.bootstrap_bytes)
    ));
    let rr = p.trailer.runtime_ref_str().unwrap_or_default();
    if rr.is_empty() {
        out.push_str("  runtime_ref: (none — classic bundle)\n");
    } else {
        let hint = if p.trailer.is_lean() {
            " (resolution hint; lean)"
        } else {
            " (resolution hint)"
        };
        out.push_str(&format!("  runtime_ref: {rr}{hint}\n"));
    }
    let (state, keyid) = p.trust();
    let outcome = trust_outcome.unwrap_or("unverified");
    match keyid {
        Some(keyid) => out.push_str(&format!("  trust: {state}, signer {keyid} — {outcome}\n")),
        None => out.push_str(&format!("  trust: {state} — {outcome}\n")),
    }
    if let Some(err) = &p.trailer_validation {
        out.push_str(&format!("  validation: FAILED ({err})\n"));
    }
    out.push_str("  slots:\n");
    for slot in &p.slots {
        let format = slot_format_label(slot);
        let mount = if slot.format_hint == tpkg::TPKG_FORMAT_RUNTIME {
            "(never mounted)".to_string()
        } else if slot.mount.is_empty() {
            "(none)".to_string()
        } else {
            slot.mount.clone()
        };
        out.push_str(&format!(
            "    [{}] {} B @ {}  format: {}  mount: {}\n",
            slot.index,
            thousands(slot.size),
            thousands(slot.offset),
            format,
            mount
        ));
        if let Some(payload) = &slot.payload {
            out.push_str(&slot_summary_line(payload));
            if depth >= Depth::Backend {
                match &payload.format {
                    Some(f) => match &f.backend_json {
                        Some(json) => {
                            let compact: String =
                                json.split_whitespace().collect::<Vec<_>>().join(" ");
                            out.push_str(&format!("     backend: {compact}\n"));
                        }
                        None => out.push_str("     backend: (no metadata surface)\n"),
                    },
                    None => out.push_str("     backend: (undetected)\n"),
                }
            }
        }
    }
    if p.trailer.is_lean() {
        out.push_str(&format!(
            "    [{n_slots}] — runtime payload slots are never mounted; lean: none\n"
        ));
    }
    out
}

/// The `--slot N` report: the slot's payload through the `tfs info`
/// manifest sections (read in place via the mount-from-region — nothing
/// is extracted). Runtime payload slots (the v1 legacy role) are never
/// mounted: a named note, not an image read.
pub fn render_slot(p: &PackageInspection, n: usize) -> Result<String, InfoError> {
    let slot = p.slots.get(n).ok_or_else(|| {
        err(format!(
            "slot index {n} out of range (package has {} slot(s))",
            p.slots.len()
        ))
    })?;
    if slot.format_hint == tpkg::TPKG_FORMAT_RUNTIME {
        return Err(err(format!(
            "slot {n} is a runtime (legacy role) payload slot — never mounted, nothing to inspect"
        )));
    }
    let payload = slot
        .payload
        .as_ref()
        .ok_or_else(|| err(format!("slot {n} was not inspected (internal depth error)")))?;
    let mut out = format!(
        "package slot [{n}] of {}  ({} B @ {}, format: {}, mount: {})\n",
        p.path.display(),
        thousands(slot.size),
        thousands(slot.offset),
        slot_format_label(slot),
        if slot.mount.is_empty() {
            "(none)".to_string()
        } else {
            slot.mount.clone()
        }
    );
    out.push_str(&crate::render::manifest_view(
        payload,
        crate::render::Sections {
            manifest: true,
            provides: true,
            requires: true,
            platforms: true,
        },
    ));
    Ok(out)
}

// ---------------------------------------------------------------------
// JSON document (spec 15 §6)
// ---------------------------------------------------------------------

fn slot_json(slot: &SlotInspection, depth: Depth) -> Json {
    let mut out: Vec<(String, Json)> = vec![
        ("index".to_string(), Json::Number(slot.index.to_string())),
        ("offset".to_string(), Json::Number(slot.offset.to_string())),
        ("size".to_string(), Json::Number(slot.size.to_string())),
        (
            "format".to_string(),
            Json::String(hint_json_name(slot.format_hint).to_string()),
        ),
        ("mount".to_string(), Json::String(slot.mount.clone())),
    ];
    if let Some(p) = &slot.payload {
        if let Some(f) = &p.format {
            out.push(("detected_format".to_string(), Json::String(f.label.clone())));
        }
        if let Some(err) = &p.mount_error {
            out.push(("mount_error".to_string(), Json::String(err.clone())));
        }
        if let Some(m) = &p.manifest {
            out.push(("manifest".to_string(), manifest_to_json(m)));
            if let Some(err) = &p.manifest_validation {
                out.push(("manifest_validation".to_string(), Json::String(err.clone())));
            }
        } else if let Some(note) = &p.manifest_note {
            out.push(("manifest_note".to_string(), Json::String(note.clone())));
        } else if let Some(err) = &p.manifest_validation {
            out.push(("manifest_error".to_string(), Json::String(err.clone())));
        }
        if depth >= Depth::Backend {
            if let Some(f) = &p.format {
                if let Some(json) = &f.backend_json {
                    if let Ok(parsed) = tebako_json::parse(json) {
                        out.push(("backend".to_string(), parsed));
                    }
                }
            }
        }
    }
    Json::Object(out)
}

/// The package as one JSON document (`"info_schema": 1`).
pub fn package_json(p: &PackageInspection, depth: Depth, trust_outcome: Option<&str>) -> Json {
    let mut flags = Vec::new();
    if p.trailer.is_lean() {
        flags.push(Json::String("lean".to_string()));
    }
    if p.trailer.v2.is_some() {
        flags.push(Json::String("signed-v2".to_string()));
    }
    let (state, keyid) = p.trust();
    let mut trust = vec![
        ("state".to_string(), Json::String(state)),
        (
            "outcome".to_string(),
            Json::String(trust_outcome.unwrap_or("unverified").to_string()),
        ),
    ];
    if let Some(keyid) = keyid {
        trust.insert(1, ("keyid".to_string(), Json::String(keyid)));
    }
    Json::Object(vec![
        (
            "info_schema".to_string(),
            Json::Number(INFO_SCHEMA.to_string()),
        ),
        (
            "artifact".to_string(),
            Json::Object(vec![
                (
                    "path".to_string(),
                    Json::String(p.path.display().to_string()),
                ),
                ("kind".to_string(), Json::String("package".to_string())),
                ("size".to_string(), Json::Number(p.size_bytes.to_string())),
            ]),
        ),
        (
            "package".to_string(),
            Json::Object(vec![
                (
                    "version".to_string(),
                    Json::Number(p.trailer.version.to_string()),
                ),
                ("flags".to_string(), Json::Array(flags)),
                (
                    "launcher_abi".to_string(),
                    Json::Number(p.trailer.launcher_abi.to_string()),
                ),
                (
                    "runtime_ref".to_string(),
                    Json::String(p.trailer.runtime_ref_str().unwrap_or_default().to_string()),
                ),
                (
                    "bootstrap_bytes".to_string(),
                    Json::Number(p.bootstrap_bytes.to_string()),
                ),
                (
                    "trailer_bytes".to_string(),
                    Json::Number(p.trailer_bytes.to_string()),
                ),
            ]),
        ),
        ("trust".to_string(), Json::Object(trust)),
        (
            "slots".to_string(),
            Json::Array(p.slots.iter().map(|s| slot_json(s, depth)).collect()),
        ),
    ])
}
