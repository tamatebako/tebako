//! The contract card (spec 18 §6): `tebako inspect --contract <artifact>`
//! prints an artifact's contract set — era, contract versions, mount
//! root, abi, trust — and the verdict against THIS tebako, for every
//! artifact class the loader family consumes:
//!
//! - a **package file** (tpkg trailer + the L2 contract block, read
//!   through tpkg's own `package_contract`/`verify_contract` — the
//!   single owner of the C6 grammar) plus the embedded bootstrap's
//!   self-description;
//! - a **payload image** (the L1 in-image manifest via tebako-info);
//! - a **runtime directory** (the cached release: its manifest.json
//!   card through tebako-resolve's pre-download gate — the same
//!   verdict the loader would give, S16's side-loaded case included);
//! - a **tebako-bootstrap binary** (the embedded artifact-info block —
//!   S38's read side).
//!
//! The card is a REPORT: inspection never enforces (a REFUSED verdict
//! still exits 0; `--verify` owns the strict exit codes, spec 15 §5).

use std::fmt::Write as _;
use std::path::Path;

use crate::error::TebakoError;
use crate::LAUNCHER_ABI;

const EX_TEBAKO_MANIFEST: i32 = 65;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError {
        code,
        message: message.into(),
    }
}

/// The local tebako's side of every verdict: the contract constants the
/// readers enforce (era + handoff contract from tebako-resolve — the
/// canonical owner; launcher ABI is this crate's).
pub struct Verdict {
    pub accepted: bool,
    pub reason: String,
}

pub struct ContractCard {
    pub class: &'static str,
    pub fields: Vec<(String, String)>,
    pub verdict: Verdict,
}

fn accepted(reason: impl Into<String>) -> Verdict {
    Verdict {
        accepted: true,
        reason: reason.into(),
    }
}

fn refused(reason: impl Into<String>) -> Verdict {
    Verdict {
        accepted: false,
        reason: reason.into(),
    }
}

/// Inspect `path` as one of the four artifact classes (detection:
/// directory → runtime directory; tpkg trailer → package; an embedded
/// artifact-info block → tebako-bootstrap; else payload image).
pub fn inspect(path: &Path) -> Result<ContractCard, TebakoError> {
    if path.is_dir() {
        return Ok(runtime_dir_card(path));
    }
    if crate::install::is_tpkg_package(path) {
        return package_card(path);
    }
    let bytes = std::fs::read(path).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("cannot read {}: {e}", path.display()),
        )
    })?;
    if tebako_bootstrap::artifact_info::extract(&bytes).is_some() {
        return bootstrap_card(&bytes);
    }
    image_card(path)
}

// ---------------------------------------------------------------------
// package (tpkg)
// ---------------------------------------------------------------------

fn flag_names(flags: u32) -> String {
    let mut names = Vec::new();
    if flags & tpkg::TPKG_FLAG_LEAN != 0 {
        names.push("LEAN");
    }
    if flags & tpkg::TPKG_FLAG_SIGNED_V2 != 0 {
        names.push("SIGNED_V2");
    }
    if flags & tpkg::TPKG_FLAG_NO_INSTALL != 0 {
        names.push("NO_INSTALL");
    }
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join("|")
    }
}

fn package_card(path: &Path) -> Result<ContractCard, TebakoError> {
    let mut f = std::fs::File::open(path).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("cannot open {}: {e}", path.display()),
        )
    })?;
    let m = tpkg::read_from(&mut f)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, format!("{}: {e}", path.display())))?;

    let mut fields: Vec<(String, String)> = vec![
        ("trailer format".to_string(), format!("{}", m.version)),
        ("launcher_abi".to_string(), m.launcher_abi.to_string()),
        ("flags".to_string(), flag_names(m.package_flags)),
        (
            "runtime_ref".to_string(),
            m.runtime_ref_str().unwrap_or_default().to_string(),
        ),
        ("slots".to_string(), m.slots.len().to_string()),
    ];

    // The L2 contract block (C6): tpkg's own reader is the single owner
    // of the grammar — the card never re-parses it.
    let contract = m.package_contract();
    match &contract {
        Ok(Some(c)) => {
            fields.push(("era".to_string(), c.contract_era.to_string()));
            fields.push(("pressed_by".to_string(), c.pressed_by.clone()));
            fields.push(("reader_era".to_string(), c.reader_era.to_string()));
        }
        Ok(None) => fields.push((
            "era".to_string(),
            "undeclared (era 1 — no contract block)".to_string(),
        )),
        Err(e) => fields.push(("era".to_string(), format!("malformed ({e})"))),
    }
    if let Ok(Some(pm)) = m.package_manifest() {
        fields.push((
            "L2 block".to_string(),
            format!(
                "schema {}, producer {} {}, {} entr{}",
                pm.schema_version,
                pm.package.producer.tool,
                pm.package.producer.tool_version,
                pm.entries.len(),
                if pm.entries.len() == 1 { "y" } else { "ies" }
            ),
        ));
        fields.push(("jail".to_string(), format!("{}", pm.jail.is_some())));
    } else {
        fields.push(("L2 block".to_string(), "absent".to_string()));
    }

    // The bootstrap stitched into the base (S38's read side).
    let bytes = std::fs::read(path).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("cannot read {}: {e}", path.display()),
        )
    })?;
    fields.push((
        "bootstrap".to_string(),
        match tebako_bootstrap::artifact_info::extract(&bytes) {
            Some(yaml) => summarize_artifact_info(yaml),
            None => "pre-era (no artifact-info block — a press must refuse it, S38)".to_string(),
        },
    ));

    fields.push((
        "trust".to_string(),
        match &m.v2 {
            Some(v2) => format!("signed v2 (keyid {})", v2.signer_keyid_hex()),
            None => "unsigned".to_string(),
        },
    ));

    // The verdict: tpkg's C6 verification first, then the trailer's ABI.
    let verdict = match m.verify_contract() {
        Ok(()) if m.launcher_abi > LAUNCHER_ABI => refused(format!(
            "package requires launcher ABI {}, this tebako speaks {LAUNCHER_ABI} — upgrade tebako",
            m.launcher_abi
        )),
        Ok(()) => accepted(format!(
            "loads under this tebako (era ≤ {}, launcher ABI {} ≤ {LAUNCHER_ABI})",
            tpkg::TPKG_CONTRACT_ERA,
            m.launcher_abi
        )),
        Err(e) => refused(format!("{e} (exit {})", e.exit_code())),
    };

    Ok(ContractCard {
        class: "package (tpkg)",
        fields,
        verdict,
    })
}

// ---------------------------------------------------------------------
// tebako-bootstrap binary
// ---------------------------------------------------------------------

/// The embedded block's declared fields (readers ignore unknown keys —
/// spec 18 §3.2).
#[derive(serde::Deserialize)]
struct ArtifactInfoView {
    era: u32,
    version: String,
    launcher_abi: u32,
    contract_version: u32,
}

fn summarize_artifact_info(yaml: &str) -> String {
    match serde_yml::from_str::<ArtifactInfoView>(yaml) {
        Ok(v) => format!(
            "era {}, version {}, launcher_abi {}, contract {}",
            v.era, v.version, v.launcher_abi, v.contract_version
        ),
        Err(e) => format!("malformed artifact-info ({e})"),
    }
}

fn bootstrap_card(bytes: &[u8]) -> Result<ContractCard, TebakoError> {
    let yaml = tebako_bootstrap::artifact_info::extract(bytes)
        .ok_or_else(|| err(EX_TEBAKO_MANIFEST, "no artifact-info block found"))?;
    let view: ArtifactInfoView = serde_yml::from_str(yaml).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("the embedded artifact-info does not parse: {e}"),
        )
    })?;
    let fields = vec![
        ("schema".to_string(), "artifact-info 1".to_string()),
        ("era".to_string(), view.era.to_string()),
        ("version".to_string(), view.version),
        ("launcher_abi".to_string(), view.launcher_abi.to_string()),
        (
            "contract_version".to_string(),
            view.contract_version.to_string(),
        ),
    ];
    let spoken_era = tebako_resolve::contract::SPOKEN_ERA;
    let spoken_contract = tebako_resolve::contract::SPOKEN_CONTRACT;
    let verdict = if view.era < 2 {
        refused("pre-era bootstrap — a press must refuse it (S38)")
    } else if view.era > spoken_era {
        refused(format!(
            "bootstrap speaks era {}, this tebako speaks era {spoken_era} — upgrade tebako",
            view.era
        ))
    } else if view.contract_version != spoken_contract {
        refused(format!(
            "bootstrap speaks contract {}, this tebako speaks contract {spoken_contract} — different contract generations",
            view.contract_version
        ))
    } else if view.launcher_abi > LAUNCHER_ABI {
        refused(format!(
            "bootstrap speaks launcher ABI {}, this tebako speaks {LAUNCHER_ABI} — upgrade tebako",
            view.launcher_abi
        ))
    } else {
        accepted(format!(
            "stitchable by this tebako (era {} ≤ {spoken_era}, launcher ABI {} ≤ {LAUNCHER_ABI}, contract {} ≤ {spoken_contract})",
            view.era, view.launcher_abi, view.contract_version
        ))
    };
    Ok(ContractCard {
        class: "tebako-bootstrap",
        fields,
        verdict,
    })
}

// ---------------------------------------------------------------------
// runtime directory (the cached release; S16's side-loaded case)
// ---------------------------------------------------------------------

fn runtime_dir_card(path: &Path) -> ContractCard {
    let mut fields: Vec<(String, String)> = Vec::new();
    let manifest_path = path.join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        fields.push((
            "era".to_string(),
            "undeclared (no manifest.json — era 1)".to_string(),
        ));
        return ContractCard {
            class: "runtime directory",
            fields,
            verdict: refused(format!(
                "pre-era: no readable release card at {} (spec 18 C2/S16 — a side-loaded runtime must carry the same contract fields, no special pleading)",
                manifest_path.display()
            )),
        };
    };

    // The exe entry anchors the card (the image is additive metadata of
    // the same entry — the loader's exact rule).
    let exe_asset = tebako_json::parse(&text).ok().and_then(|parsed| {
        if let tebako_json::Value::Array(entries) = &parsed {
            entries.iter().find_map(|e| {
                let name = e.find("filename").and_then(|f| f.as_string())?;
                (name.starts_with("tebako-runtime-") && !name.ends_with(".tfs")).then_some(name)
            })
        } else {
            None
        }
    });
    let Some(exe_asset) = exe_asset else {
        return ContractCard {
            class: "runtime directory",
            fields,
            verdict: refused(format!(
                "no tebako-runtime executable entry in {} — not a runtime cache entry",
                manifest_path.display()
            )),
        };
    };

    fields.push(("exe entry".to_string(), exe_asset.clone()));
    // abi flows (not gated — resolution filters on it, spec 05 §5).
    let abi = tebako_json::parse(&text).ok().and_then(|parsed| {
        let tebako_json::Value::Array(entries) = &parsed else {
            return None;
        };
        entries
            .iter()
            .find(|e| {
                e.find("filename").and_then(|f| f.as_string()).as_deref() == Some(&*exe_asset)
            })
            .and_then(|entry| entry.find("abi").and_then(|a| a.as_string()))
    });
    if let Some(abi) = abi {
        fields.push(("abi".to_string(), abi));
    }
    fields.push((
        "trust".to_string(),
        if path.join("sha256").is_file() {
            "sha256 marker present".to_string()
        } else {
            "no sha256 marker (side-loaded?)".to_string()
        },
    ));

    // The verdict is the loader's own gate — the exact refusal a
    // download would hit, given pre-download (spec 18 C2/S11/S12).
    match tebako_resolve::contract::gate(&text, &exe_asset) {
        Ok(Some(set)) => {
            fields.push(("era".to_string(), set.era.to_string()));
            fields.push((
                "contract_version".to_string(),
                set.contract_version.to_string(),
            ));
            fields.push(("mount_root".to_string(), set.mount_root));
            fields.push((
                "image".to_string(),
                format!(
                    "{}",
                    path.join(format!(
                        "{}.tfs",
                        exe_asset.strip_suffix(".exe").unwrap_or(&exe_asset)
                    ))
                    .is_file()
                ),
            ));
            ContractCard {
                class: "runtime directory",
                fields,
                verdict: accepted(format!(
                    "accepted by the pre-download gate (era {} ≤ {}, contract {} ≤ {})",
                    set.era,
                    tebako_resolve::contract::SPOKEN_ERA,
                    set.contract_version,
                    tebako_resolve::contract::SPOKEN_CONTRACT
                )),
            }
        }
        Ok(None) => {
            fields.push(("era".to_string(), "undeclared (era 1)".to_string()));
            ContractCard {
                class: "runtime directory",
                fields,
                verdict: refused(format!(
                    "pre-era — no contract set declared for {exe_asset} (spec 18 C2)"
                )),
            }
        }
        Err(e) => {
            fields.push(("era".to_string(), "see verdict".to_string()));
            ContractCard {
                class: "runtime directory",
                fields,
                verdict: refused(format!("{e} (exit 75)")),
            }
        }
    }
}

// ---------------------------------------------------------------------
// payload image
// ---------------------------------------------------------------------

fn platforms_str(p: &tpkg::Platforms) -> String {
    match p {
        tpkg::Platforms::Universal => "universal".to_string(),
        tpkg::Platforms::Triplets(ts) => format!(
            "abi facets: {}",
            ts.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn image_card(path: &Path) -> Result<ContractCard, TebakoError> {
    let p = tebako_info::payload::inspect_image(path)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, format!("{}: {e}", path.display())))?;
    let mut fields: Vec<(String, String)> = Vec::new();
    if let Some(f) = &p.format {
        fields.push(("format".to_string(), f.label.clone()));
    }
    let verdict = match (&p.manifest, &p.mount_error) {
        (_, Some(e)) => {
            fields.push(("era".to_string(), "unreadable".to_string()));
            refused(format!("not a readable payload image: {e}"))
        }
        (Some(m), None) => {
            fields.push((
                "manifest".to_string(),
                format!("payload schema {}", m.identity.schema_version),
            ));
            fields.push(("kind".to_string(), format!("{:?}", m.identity.kind)));
            fields.push((
                "identity".to_string(),
                format!("{} {}", m.identity.name, m.identity.version),
            ));
            match &m.provides {
                tpkg::Provides::App(p) => {
                    fields.push(("entrypoints".to_string(), p.entrypoints.len().to_string()));
                    fields.push(("platforms".to_string(), platforms_str(&p.platforms)));
                }
                tpkg::Provides::Runtime(p) => {
                    fields.push(("engines".to_string(), p.provides.len().to_string()));
                }
                tpkg::Provides::Data(p) => {
                    fields.push((
                        "mount suggestion".to_string(),
                        p.mount_semantics.suggested.clone(),
                    ));
                }
                tpkg::Provides::Toolkit(p) => {
                    fields.push(("executables".to_string(), p.executables.len().to_string()));
                    fields.push(("platforms".to_string(), platforms_str(&p.platforms)));
                }
                tpkg::Provides::Other(_) => {}
            }
            fields.push((
                "trust".to_string(),
                format!(
                    "signing {:?} / encryption {:?}",
                    m.identity.signing.state, m.identity.encryption.state
                ),
            ));
            fields.push((
                "era".to_string(),
                "undeclared (era 1 — the L1 payload schema declares no era field today)"
                    .to_string(),
            ));
            if m.identity.schema_version <= tpkg::PAYLOAD_SCHEMA_VERSION {
                accepted(format!(
                    "reads under this tebako (payload schema {} ≤ {})",
                    m.identity.schema_version,
                    tpkg::PAYLOAD_SCHEMA_VERSION
                ))
            } else {
                refused(format!(
                    "payload schema {} is newer than this tebako reads ({}) — upgrade tebako",
                    m.identity.schema_version,
                    tpkg::PAYLOAD_SCHEMA_VERSION
                ))
            }
        }
        (None, None) => {
            fields.push((
                "era".to_string(),
                "undeclared (era 1 — no in-image manifest)".to_string(),
            ));
            refused("no in-image manifest (/__tpkg__/manifest.yaml) — a pre-era payload")
        }
    };
    Ok(ContractCard {
        class: "payload image",
        fields,
        verdict,
    })
}

// ---------------------------------------------------------------------
// rendering
// ---------------------------------------------------------------------

pub fn render(card: &ContractCard, path: &Path) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "contract card: {}", path.display());
    let _ = writeln!(out, "  class: {}", card.class);
    for (k, v) in &card.fields {
        let _ = writeln!(out, "  {k}: {v}");
    }
    let _ = writeln!(
        out,
        "verdict: {} — {}",
        if card.verdict.accepted {
            "ACCEPTED"
        } else {
            "REFUSED"
        },
        card.verdict.reason
    );
    out
}

pub fn render_json(card: &ContractCard, path: &Path) -> String {
    let fields = tebako_json::Value::Object(
        card.fields
            .iter()
            .map(|(k, v)| (k.clone(), tebako_json::Value::String(v.clone())))
            .collect(),
    );
    let doc = tebako_json::Value::Object(vec![
        (
            "info_schema".to_string(),
            tebako_json::Value::Number("1".into()),
        ),
        (
            "artifact".to_string(),
            tebako_json::Value::String(path.display().to_string()),
        ),
        (
            "class".to_string(),
            tebako_json::Value::String(card.class.to_string()),
        ),
        ("contract".to_string(), fields),
        (
            "verdict".to_string(),
            tebako_json::Value::Object(vec![
                (
                    "accepted".to_string(),
                    tebako_json::Value::Bool(card.verdict.accepted),
                ),
                (
                    "reason".to_string(),
                    tebako_json::Value::String(card.verdict.reason.clone()),
                ),
            ]),
        ),
    ]);
    format!("{}\n", tebako_json::to_string(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tebako-cli-card-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_runtime_directory_card_mirrors_the_gate() {
        let asset = "tebako-runtime-0.16.1-3.4.2-macos-arm64";
        let d = dir("rt");
        let manifest = |bits: &str| {
            format!(
                "[{{{bits}\"filename\": \"{asset}\", \"sha256\": \"{}\"}}]\n",
                "a".repeat(64)
            )
        };
        // accepted: the era-2 factory shape
        std::fs::write(
            d.join("manifest.json"),
            manifest(
                "\"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", ",
            ),
        )
        .unwrap();
        let card = runtime_dir_card(&d);
        assert!(card.verdict.accepted, "{}", card.verdict.reason);
        assert!(card
            .fields
            .iter()
            .any(|(k, v)| k == "mount_root" && v == "/__tfs__"));
        // pre-era: the contract fields stripped
        std::fs::write(d.join("manifest.json"), manifest("")).unwrap();
        let card = runtime_dir_card(&d);
        assert!(!card.verdict.accepted);
        assert!(
            card.verdict.reason.contains("pre-era"),
            "{}",
            card.verdict.reason
        );
        // newer contract: both numbers, exit 75 named
        std::fs::write(
            d.join("manifest.json"),
            manifest(
                "\"contract_era\": 2, \"contract_version\": 3, \"mount_root\": \"/__tfs__\", ",
            ),
        )
        .unwrap();
        let card = runtime_dir_card(&d);
        assert!(!card.verdict.accepted);
        assert!(
            card.verdict.reason.contains("contract_version 3"),
            "{}",
            card.verdict.reason
        );
        assert!(
            card.verdict.reason.contains("speaks contract 2"),
            "{}",
            card.verdict.reason
        );
        // no manifest at all: S16's side-loaded refusal
        let empty = dir("rt-empty");
        let card = runtime_dir_card(&empty);
        assert!(!card.verdict.accepted);
        assert!(
            card.verdict.reason.contains("S16"),
            "{}",
            card.verdict.reason
        );
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn a_bootstrap_card_reads_the_embedded_block() {
        // The blob is synthesized from the bootstrap crate's own
        // constants (the reader side); the bootstrap's own tests prove
        // the embed survives a real link + strip.
        let yaml = tebako_bootstrap::artifact_info::yaml();
        let mut blob = b"MZ fake".to_vec();
        blob.extend_from_slice(tebako_bootstrap::artifact_info::BLOCK_BEGIN.as_bytes());
        blob.extend_from_slice(yaml.as_bytes());
        blob.extend_from_slice(tebako_bootstrap::artifact_info::BLOCK_END.as_bytes());
        let card = bootstrap_card(&blob).unwrap();
        assert_eq!(card.class, "tebako-bootstrap");
        assert!(card.verdict.accepted, "{}", card.verdict.reason);
        assert!(card
            .fields
            .iter()
            .any(|(k, v)| k == "era" && v == &tebako_bootstrap::SPOKEN_ERA.to_string()));
        // a doctored era surfaces as the upgrade refusal
        let doctored = yaml.replace(&format!("era: {}", tebako_bootstrap::SPOKEN_ERA), "era: 9");
        let mut blob = Vec::new();
        blob.extend_from_slice(tebako_bootstrap::artifact_info::BLOCK_BEGIN.as_bytes());
        blob.extend_from_slice(doctored.as_bytes());
        blob.extend_from_slice(tebako_bootstrap::artifact_info::BLOCK_END.as_bytes());
        let card = bootstrap_card(&blob).unwrap();
        assert!(!card.verdict.accepted);
        assert!(
            card.verdict.reason.contains("era 9"),
            "{}",
            card.verdict.reason
        );
    }

    #[test]
    fn render_shapes() {
        let card = ContractCard {
            class: "tebako-bootstrap",
            fields: vec![("era".to_string(), "2".to_string())],
            verdict: accepted("fine"),
        };
        let text = render(&card, Path::new("/x"));
        assert!(text.contains("verdict: ACCEPTED — fine"), "{text}");
        let json = render_json(&card, Path::new("/x"));
        assert!(json.contains("\"accepted\": true"), "{json}");
        assert!(json.contains("\"info_schema\": 1"), "{json}");
    }
}
