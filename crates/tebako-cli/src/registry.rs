//! `tebako registry <verb>` — the registry-file maintenance surface:
//!
//!   tebako registry retire <registry-file> <name>@<version> [--force]
//!
//! `retire` (tebako#675) removes one version row from a LOCAL registry
//! file through the publish flow's own discipline (spec 18 C12: parse →
//! mutate → re-validate → atomic write), refusing while the retirement
//! would strand an in-registry runtime edge, dangle the payload's
//! default, or remove the payload's last row — `--force` overrides, with
//! every overridden refusal spelled in the report and the journal.

use std::path::Path;

use tebako_resolve::registry::Registry;

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
