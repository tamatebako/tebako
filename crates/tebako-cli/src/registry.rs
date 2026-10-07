//! `tebako registry <verb>` — the registry-file maintenance surface:
//!
//!   tebako registry validate <path-or-url> [--json]
//!   tebako registry retire <registry-file> <name>@<version> [--force]
//!
//! `validate` (tebako#680) runs the EXACT client-side parse
//! ([`Registry::from_yaml`] — the fail-closed reader every install and
//! dispatch path runs, default-names-a-listed-version included) plus the
//! strict extras a CI gate wants collected in one report, so a
//! registry-touching change fails on ITS OWN check instead of breaking
//! every reader at merge. `retire` (tebako#675) removes one version row
//! from a LOCAL registry file through the publish flow's own discipline
//! (spec 18 C12: parse → mutate → re-validate → atomic write), refusing
//! while the retirement would strand an in-registry runtime edge, dangle
//! the payload's default, or remove the payload's last row — `--force`
//! overrides, with every overridden refusal spelled in the report and
//! the journal.

use std::path::{Path, PathBuf};

use tebako_resolve::registry::{Registry, RegistryRef};
use tebako_resolve::Fetcher;

use crate::error::TebakoError;

const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

// ---------------------------------------------------------------------
// retire (tebako#675)
// ---------------------------------------------------------------------

/// `tebako registry retire <registry-file> <name>@<version> [--force]`:
/// remove ONE version row from a local registry file, journal-announced.
///
/// The refusals are the auditable part (all named, all exit 65):
///
/// - `(RegistryRowStillRequired)` — the row is a `kind: runtime` row and
///   retiring it strands an in-registry edge: another payload's
///   `runtime_requirement` on the same engine (and implementation axis,
///   spec 28 §8 — the resolver's own `runtime_entries` filter) matches
///   the retiring version but NO surviving, non-withdrawn row. An edge
///   whose constraint does not parse never resolves, so it has nothing
///   to strand — `validate` flags that class instead. Edges served by
///   OTHER registries are out of scope by construction: the gate is the
///   one registry's integrity, never a global resolution preview.
/// - `(RegistryDefaultWouldDangle)` — the row is the payload's
///   `default:` (tebako#680's dangling-default class, caught here BEFORE
///   the write). `--force` repoints the default to the newest remaining
///   row.
/// - `(RegistryRowIsLast)` — the row is the payload's last; a
///   `versions: []` entry is not a valid registry, so retiring it
///   removes the payload entry too.
///
/// `--force` never silences: every overridden refusal is a loud note in
/// the returned report and on the journal line.
pub fn retire(
    home: &Path,
    registry_path: &Path,
    name: &str,
    version: &str,
    force: bool,
) -> Result<String, TebakoError> {
    let text = std::fs::read_to_string(registry_path).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot read {}: {e}", registry_path.display()),
        )
    })?;
    let mut registry = Registry::from_yaml(&text).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("{}: {e}", registry_path.display()),
        )
    })?;

    let payload_index = registry
        .payloads
        .iter()
        .position(|p| p.name == name)
        .ok_or_else(|| {
            let names = registry
                .payloads
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{} carries no payload named '{name}' — payloads: {names}",
                    registry_path.display()
                ),
            )
        })?;
    let payload = &registry.payloads[payload_index];
    if !payload.versions.iter().any(|v| v.version == version) {
        let rows = payload
            .versions
            .iter()
            .map(|v| v.version.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "payload '{name}' in {} has no version row '{version}' — rows: {rows}",
                registry_path.display()
            ),
        ));
    }

    // The DEPENDS gate: the row is a runtime edge target exactly when its
    // payload is `kind: runtime` with a discoverable engine.
    let mut stranded: Vec<String> = Vec::new();
    if payload.kind == tpkg::PayloadKind::Runtime {
        if let Some(engine) = payload.engine() {
            for consumer in &registry.payloads {
                for row in &consumer.versions {
                    let Some(req) = &row.runtime_requirement else {
                        continue;
                    };
                    if req.engine != engine {
                        continue;
                    }
                    let Ok(constraint) = tpkg::versions::parse_constraint(&req.constraint) else {
                        continue;
                    };
                    if !constraint.matches(version) {
                        continue;
                    }
                    let satisfied_elsewhere = registry
                        .runtime_entries(engine, req.implementation.as_deref())
                        .iter()
                        .flat_map(|p| p.versions.iter().map(move |v| (p.name.as_str(), v)))
                        .filter(|(pname, v)| !(*pname == name && v.version == version))
                        .filter(|(_, v)| !v.is_withdrawn())
                        .any(|(_, v)| constraint.matches(&v.version));
                    if !satisfied_elsewhere {
                        stranded.push(format!(
                            "{} {} (runtime_requirement {{engine: {}, constraint: \"{}\"}})",
                            consumer.name, row.version, req.engine, req.constraint
                        ));
                    }
                }
            }
        }
    }

    let is_last = payload.versions.len() == 1;
    let is_default = payload.default.as_deref() == Some(version);

    let mut refusals: Vec<String> = Vec::new();
    if !stranded.is_empty() {
        refusals.push(format!(
            "in-registry runtime edges resolve through it and no surviving row satisfies them: {} (RegistryRowStillRequired)",
            stranded.join("; ")
        ));
    }
    if is_last {
        refusals.push(format!(
            "it is the last version row of '{name}' — retiring it removes the payload entry from the registry (RegistryRowIsLast)"
        ));
    } else if is_default {
        refusals.push(
            "it is the payload's default: — retiring it leaves the default dangling (RegistryDefaultWouldDangle)"
                .to_string(),
        );
    }
    if !refusals.is_empty() && !force {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "cannot retire {name}@{version} from {}: {}\n  --force retires anyway; the consequences land in the report and the journal",
                registry_path.display(),
                refusals.join("; ")
            ),
        ));
    }

    let mut notes: Vec<String> = Vec::new();
    {
        let payload = &mut registry.payloads[payload_index];
        payload.versions.retain(|v| v.version != version);
        if !is_last && is_default {
            let newest = tpkg::versions::newest(payload.versions.iter().map(|v| &v.version));
            if let Some(newest) = &newest {
                notes.push(format!("the payload's default now points at {newest}"));
            }
            payload.default = newest;
        }
    }
    if is_last {
        registry.payloads.remove(payload_index);
        notes.push(format!(
            "the payload entry '{name}' was removed with its last row"
        ));
    }
    if force {
        for refusal in &refusals {
            notes.push(format!("forced past: {refusal}"));
        }
    }

    // spec 18 C12's read-back discipline: a document this build cannot
    // parse is never written.
    let out = registry
        .to_yaml()
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
    Registry::from_yaml(&out).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("the retired registry does not read back: {e}"),
        )
    })?;
    crate::publish::write_atomic(registry_path, out.as_bytes())?;

    crate::install::journal(
        home,
        &format!(
            "event=registry-row-retired registry={} payload={} version={} force={}",
            registry_path.display(),
            name,
            version,
            force
        ),
    );

    let mut report = format!(
        "retired {name}@{version} from {}\n",
        registry_path.display()
    );
    for note in &notes {
        report.push_str(&format!("note: {note}\n"));
    }
    Ok(report)
}

// ---------------------------------------------------------------------
// validate (tebako#680)
// ---------------------------------------------------------------------

/// One collected violation; `payload`/`version` are absent when the
/// document never parsed far enough to name a row.
struct Violation {
    payload: Option<String>,
    version: Option<String>,
    message: String,
}

/// `tebako registry validate <path-or-url> [--json]`: the client-side
/// registry parse as a scriptable gate. Returns the report text and the
/// verdict code — 0 when the registry is valid, EX_TEBAKO_MANIFEST (65)
/// on any violation; the INPUT being unreadable/unfetchable is an error
/// (74 / the resolve mapping), never a verdict. The human form prints
/// one line per violation; the `--json` form is a
/// `registry_validate_schema: 1` document on stdout (the banner moves to
/// stderr in main, so scripts read stdout alone).
pub fn validate(input: &str, json: bool) -> Result<(String, i32), TebakoError> {
    let bytes = read_registry_bytes(input)?;
    let text = String::from_utf8(bytes).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("{input}: {e} decoding the registry file"),
        )
    })?;

    let mut violations: Vec<Violation> = Vec::new();
    let mut payloads = 0usize;
    let mut rows = 0usize;
    match Registry::from_yaml(&text) {
        // The exact client-side refusal, verbatim — the whole point of
        // the gate is that THIS message is what every reader would hit.
        Err(e) => violations.push(Violation {
            payload: None,
            version: None,
            message: e.to_string(),
        }),
        Ok(registry) => {
            payloads = registry.payloads.len();
            rows = registry.payloads.iter().map(|p| p.versions.len()).sum();
            // The strict extras from_yaml deliberately does not enforce
            // (reader leniency is a compat surface — pre-MINOR readers
            // ignore additive keys); the gate collects them all in one
            // report so a registry PR fails its own CI leg.
            for p in &registry.payloads {
                for v in &p.versions {
                    if let Some(req) = &v.runtime_requirement {
                        if let Err(e) = tpkg::versions::parse_constraint(&req.constraint) {
                            violations.push(Violation {
                                payload: Some(p.name.clone()),
                                version: Some(v.version.clone()),
                                message: format!(
                                    "runtime_requirement.constraint does not parse: {e}"
                                ),
                            });
                        }
                        if req.abi.is_some()
                            && req.implementation.is_none()
                            && p.implementation().is_none()
                        {
                            violations.push(Violation {
                                payload: Some(p.name.clone()),
                                version: Some(v.version.clone()),
                                message: "runtime_requirement.abi is spelled with no implementation axis anywhere (spec 28 §8 — an abi is per-implementation by construction)"
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    let ok = violations.is_empty();
    let code = if ok { 0 } else { EX_TEBAKO_MANIFEST };
    let text = if json {
        validate_json(input, ok, payloads, rows, &violations)
    } else if ok {
        format!("{input}: OK ({payloads} payload(s), {rows} version row(s))\n")
    } else {
        let mut out = String::new();
        for v in &violations {
            match (&v.payload, &v.version) {
                (Some(p), Some(ver)) => {
                    out.push_str(&format!("{input}: {p} {ver}: {}\n", v.message))
                }
                _ => out.push_str(&format!("{input}: {}\n", v.message)),
            }
        }
        out
    };
    Ok((text, code))
}

/// The input's bytes: an existing local path reads directly; anything
/// else parses as a spec 04 §2 registry reference (`file://` mirrors,
/// `tfs:<svc>:owner/repo`, the release-artifact/git/https/oci forms) and
/// fetches through the in-process fetcher — never a shell-out.
fn read_registry_bytes(input: &str) -> Result<Vec<u8>, TebakoError> {
    let path = PathBuf::from(input);
    if path.is_file() {
        return std::fs::read(&path)
            .map_err(|e| err(EX_TEBAKO_IO, format!("cannot read {input}: {e}")));
    }
    let reference = RegistryRef::parse(input).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "'{input}' is not an existing file and does not parse as a registry reference: {e}"
            ),
        )
    })?;
    Fetcher::new()
        .fetch_registry(&reference)
        .map_err(crate::install::map_resolve)
}

fn validate_json(
    input: &str,
    ok: bool,
    payloads: usize,
    rows: usize,
    violations: &[Violation],
) -> String {
    use tebako_pkg::{json_to_string, JsonValue as J};

    let s = |v: &str| J::String(v.to_string());
    let n = |v: u64| J::Number(v.to_string());
    let errors: Vec<J> = violations
        .iter()
        .map(|v| {
            let mut obj = Vec::new();
            if let Some(p) = &v.payload {
                obj.push(("payload".to_string(), s(p)));
            }
            if let Some(ver) = &v.version {
                obj.push(("version".to_string(), s(ver)));
            }
            obj.push(("message".to_string(), s(&v.message)));
            J::Object(obj)
        })
        .collect();
    let doc = J::Object(vec![
        ("registry_validate_schema".to_string(), n(1)),
        ("registry".to_string(), s(input)),
        ("ok".to_string(), J::Bool(ok)),
        ("payloads".to_string(), n(payloads as u64)),
        ("versions".to_string(), n(rows as u64)),
        ("errors".to_string(), J::Array(errors)),
    ]);
    format!("{}\n", json_to_string(&doc))
}
