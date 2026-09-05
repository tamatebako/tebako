//! Strict verification (spec 15 §5) with named exit codes. The check
//! order is the spec's: tpkg structural validation (spec 02 §6) →
//! per-slot sha256 (v2) → signature (v2) → manifest schema validation
//! per slot → digest agreement (manifest `blob_sha256` vs image bytes
//! when declared). Everything is local: the trusted keyring is read from
//! `$TEBAKO_HOME` (never downloaded).
//!
//! Note on digest agreement: the check compares the declared
//! `blob_sha256` against the actual image bytes. A manifest embedded in
//! the image it describes cannot name that image's digest (the field
//! would have to hash its own bytes), so this check in practice detects
//! manifests naming a DIFFERENT blob — tampering or a stale manifest.

use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::exit_code;
use crate::package::{self, PackageInspection};
use crate::payload::{self, PayloadInspection};
use crate::{err, InfoError};

/// The outcome of one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    /// Passed.
    Pass,
    /// Failed (the exit code is the check's `fail_code`).
    Fail,
    /// Not applicable to this artifact.
    Skip,
}

impl CheckResult {
    /// The report rendering.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckResult::Pass => "ok",
            CheckResult::Fail => "FAILED",
            CheckResult::Skip => "skip",
        }
    }
}

/// One named check of a verification run.
#[derive(Debug, Clone)]
pub struct Check {
    /// Check name (`trailer`, `slot[0] sha256`, `signature`, …).
    pub name: String,
    /// Outcome.
    pub result: CheckResult,
    /// Human detail (`signer a55a…`, the mismatch, …).
    pub detail: String,
    /// Exit code when this check fails.
    pub fail_code: i32,
}

impl Check {
    fn pass(name: impl Into<String>, detail: impl Into<String>) -> Check {
        Check {
            name: name.into(),
            result: CheckResult::Pass,
            detail: detail.into(),
            fail_code: 0,
        }
    }

    fn skip(name: impl Into<String>, detail: impl Into<String>) -> Check {
        Check {
            name: name.into(),
            result: CheckResult::Skip,
            detail: detail.into(),
            fail_code: 0,
        }
    }

    fn fail(name: impl Into<String>, detail: impl Into<String>, code: i32) -> Check {
        Check {
            name: name.into(),
            result: CheckResult::Fail,
            detail: detail.into(),
            fail_code: code,
        }
    }
}

/// The first failing check's exit code (0 = all pass).
pub fn exit_code_of(checks: &[Check]) -> i32 {
    checks
        .iter()
        .find(|c| c.result == CheckResult::Fail)
        .map_or(exit_code::OK, |c| c.fail_code)
}

/// The checks as a JSON array (`checks[]` of the spec-15 §6 document).
pub fn checks_json(checks: &[Check]) -> tebako_json::Value {
    use tebako_json::Value as Json;
    Json::Array(
        checks
            .iter()
            .map(|c| {
                let result = match c.result {
                    CheckResult::Pass => "pass",
                    CheckResult::Fail => "fail",
                    CheckResult::Skip => "skip",
                };
                Json::Object(vec![
                    ("name".to_string(), Json::String(c.name.clone())),
                    ("result".to_string(), Json::String(result.to_string())),
                    ("detail".to_string(), Json::String(c.detail.clone())),
                ])
            })
            .collect(),
    )
}

/// The trust-section outcome for the container report (spec 15 §3: the
/// stored state is `unverified` until `--verify` ran; this derives the
/// outcome label from the signature check when it did).
pub fn trust_outcome(checks: &[Check]) -> Option<String> {
    let sig = checks.iter().find(|c| c.name == "signature")?;
    match sig.result {
        CheckResult::Pass => Some("trusted".to_string()),
        CheckResult::Fail if sig.fail_code == exit_code::TRUST => {
            Some("UNTRUSTED (signer not in the keyring)".to_string())
        }
        CheckResult::Fail if sig.fail_code == exit_code::SIGNATURE => {
            Some(format!("INVALID ({})", sig.detail))
        }
        _ => None,
    }
}

/// The per-check report plus the verdict line.
pub fn render_report(what: &str, checks: &[Check]) -> String {
    let mut out = format!("verify: {what}\n");
    for c in checks {
        if c.detail.is_empty() {
            out.push_str(&format!("  {}: {}\n", c.name, c.result.as_str()));
        } else {
            out.push_str(&format!(
                "  {}: {} — {}\n",
                c.name,
                c.result.as_str(),
                c.detail
            ));
        }
    }
    let code = exit_code_of(checks);
    if code == exit_code::OK {
        out.push_str("result: PASS\n");
    } else {
        out.push_str(&format!("result: FAILED (exit {code})\n"));
    }
    out
}

fn sha256_region(path: &Path, offset: u64, size: u64) -> Result<[u8; 32], InfoError> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)
        .map_err(|e| err(format!("{}: cannot read file ({e})", path.display())))?;
    f.seek(SeekFrom::Start(offset))
        .map_err(|e| err(format!("{}: cannot seek ({e})", path.display())))?;
    let mut h = sha2::Sha256::new();
    let mut buf = [0u8; 1 << 16];
    let mut remaining = size;
    while remaining > 0 {
        let want = remaining.min(buf.len() as u64) as usize;
        let n = f
            .read(&mut buf[..want])
            .map_err(|e| err(format!("{}: read failed ({e})", path.display())))?;
        if n == 0 {
            return Err(err(format!(
                "{}: short read (slot extends past end of file)",
                path.display()
            )));
        }
        h.update(&buf[..n]);
        remaining -= n as u64;
    }
    Ok(h.finalize().into())
}

fn hex(bytes: &[u8]) -> String {
    tebako_signer::hex_lower(bytes)
}

fn trusted_keyring() -> Result<Vec<u8>, InfoError> {
    let home = tebako_signer::default_home().map_err(|e| err(e.to_string()))?;
    tebako_signer::trusted_keyring_bytes(&home).map_err(|e| err(e.to_string()))
}

/// The signature check against the local trusted keyring.
fn signature_check(
    name: &str,
    signed_bytes: &[u8],
    signature: &[u8],
    keyid_hint: &[u8; 8],
) -> Result<Check, InfoError> {
    let keyring = trusted_keyring()?;
    let outcome = tebako_signer::verify_detached(&keyring, signed_bytes, signature, keyid_hint)
        .map_err(|e| err(e.to_string()))?;
    Ok(match outcome {
        tebako_signer::VerifyOutcome::Trusted(keyid) => {
            Check::pass(name, format!("trusted, signer {keyid}"))
        }
        tebako_signer::VerifyOutcome::Untrusted(keyid) => Check::fail(
            name,
            format!("signer key {keyid} is not in the trusted keyring"),
            exit_code::TRUST,
        ),
        tebako_signer::VerifyOutcome::Invalid(keyid) => Check::fail(
            name,
            match keyid {
                Some(k) => format!("invalid signature (signer {k})"),
                None => "invalid signature".to_string(),
            },
            exit_code::SIGNATURE,
        ),
    })
}

/// The detached-signature sidecar of an artifact (`<artifact>.asc`).
fn asc_sidecar(artifact: &Path) -> PathBuf {
    artifact.with_file_name(format!(
        "{}.asc",
        artifact
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ))
}

/// Read and dearmor a detached signature sidecar.
fn read_sidecar_signature(asc: &Path) -> Result<Vec<u8>, InfoError> {
    let raw = std::fs::read(asc).map_err(|e| {
        err(format!(
            "{}: cannot read the signature ({e})",
            asc.display()
        ))
    })?;
    rnp_dearmor(&raw)
}

/// Dearmor when armored; pass binary through (the release `.asc` files are
/// armored; tolerate binary too).
fn rnp_dearmor(raw: &[u8]) -> Result<Vec<u8>, InfoError> {
    if raw.starts_with(b"-----BEGIN PGP") {
        tebako_signer::dearmor_bytes(raw).map_err(|e| err(e.to_string()))
    } else {
        Ok(raw.to_vec())
    }
}

// ---------------------------------------------------------------------
// Package verification (tebako-pkg info --verify / validate)
// ---------------------------------------------------------------------

/// The spec-15 §5 package checks. Returns the checks plus, when the
/// trailer parsed, the container inspection (for `--full --verify`).
pub fn verify_package(
    binary: &Path,
    require_signed: bool,
) -> Result<(Vec<Check>, Option<PackageInspection>), InfoError> {
    let mut checks = Vec::new();

    // 1. tpkg structural validation (spec 02 §6).
    let trailer = match package::read_trailer(binary) {
        Ok(t) => t,
        Err(e) => {
            checks.push(Check::fail("trailer", e.0, exit_code::MALFORMED));
            return Ok((checks, None));
        }
    };
    if let Err(e) = trailer.validate() {
        checks.push(Check::fail(
            "trailer",
            format!("structural validation: {}", tpkg::strerror(e.code())),
            exit_code::MALFORMED,
        ));
        return Ok((checks, None));
    }
    checks.push(Check::pass("trailer", "structural validation (spec 02 §6)"));

    // 2. Per-slot sha256 (v2).
    if let Some(v2) = &trailer.v2 {
        for (i, slot) in trailer.slots.iter().enumerate() {
            let name = format!("slot[{i}] sha256");
            let actual = sha256_region(binary, slot.offset, slot.size)?;
            if actual == v2.slot_digests[i] {
                checks.push(Check::pass(name, "digest matches"));
            } else {
                checks.push(Check::fail(
                    name,
                    format!(
                        "digest mismatch: trailer {}, content {}",
                        hex(&v2.slot_digests[i]),
                        hex(&actual)
                    ),
                    exit_code::DIGEST,
                ));
            }
        }
    } else {
        checks.push(Check::skip(
            "slot digests",
            "unsigned (v1 trailer carries none)",
        ));
    }

    // 3. Signature (v2).
    match &trailer.v2 {
        Some(v2) => {
            let trailer_bytes = read_trailer_bytes(binary, &trailer)?;
            let region = tpkg::v2_signed_region(&trailer_bytes)
                .map_err(|e| err(tpkg::strerror(e.code()).to_string()))?;
            checks.push(signature_check(
                "signature",
                &region,
                &v2.signature,
                &v2.signer_keyid,
            )?);
        }
        None => {
            if require_signed {
                checks.push(Check::fail(
                    "signature",
                    "unsigned package (--require-signed)",
                    exit_code::SIGNATURE,
                ));
            } else {
                checks.push(Check::skip("signature", "unsigned (v1 legacy trailer)"));
            }
        }
    }

    // 4 + 5. Per-slot manifest schema validation and digest agreement.
    // Runtime payload slots (the v1 legacy role) are launchers, not image
    // payloads: never mounted, no manifest checks (spec 15 §3).
    //
    // The L2 package manifest parses once — the raw-slot set below and
    // the section-6 cross-checks share it. Slots a lock.spawned[] row
    // claims as `exe`/`dll` carry RAW BYTES (a wrapper exe / a PE dll —
    // never an image): no mount, no manifest check (spec 23 §13.6,
    // spec 30 §2). The byte identity rides the trailer slot digest
    // (check 2) and the lock's digest pin.
    let pm_result = trailer.package_manifest();
    let raw_slots: std::collections::BTreeSet<usize> = match &pm_result {
        Ok(Some(pm)) => pm
            .lock
            .iter()
            .flat_map(|lock| lock.spawned.iter())
            .flat_map(|row| {
                // Runtime rows carry the pair's exe (+ dll) raw; payload
                // rows (spec 32) carry the NESTED runtime pair's raw
                // artifacts — the provider image itself is a real image
                // and takes the normal slot-manifest check below.
                let rt = match row {
                    tpkg::LockedSpawned::Runtime(rt) => rt,
                    tpkg::LockedSpawned::Payload(p) => &p.runtime,
                };
                rt.exe
                    .slot
                    .into_iter()
                    .chain(rt.dll.as_ref().and_then(|d| d.slot))
            })
            .map(|s| s as usize)
            .collect(),
        _ => Default::default(),
    };
    let inspection = package::inspect_package(binary, package::Depth::Manifests, None)?;
    for slot in &inspection.slots {
        let name = format!("slot[{}] manifest", slot.index);
        if slot.format_hint == tpkg::TPKG_FORMAT_RUNTIME {
            checks.push(Check::skip(name, "runtime (legacy role) — never mounted"));
            continue;
        }
        if raw_slots.contains(&slot.index) {
            checks.push(Check::skip(
                name,
                "spawned runtime artifact (raw bytes — never mounted; the trailer digest and the lock pin carry its identity)",
            ));
            continue;
        }
        let p = slot
            .payload
            .as_ref()
            .ok_or_else(|| err("internal error: slot payload missing at depth 1"))?;
        if let Some(err) = &p.mount_error {
            checks.push(Check::fail(name, err.clone(), exit_code::MALFORMED));
            continue;
        }
        match (&p.manifest, &p.manifest_validation) {
            (Some(m), None) => {
                checks.push(Check::pass(name.clone(), "schema valid"));
                // 5. digest agreement (manifest blob_sha256 vs image bytes).
                let actual = sha256_region(binary, slot.offset, slot.size)?;
                let declared = &m.identity.digest.blob_sha256;
                if hex(&actual) == *declared {
                    checks.push(Check::pass(
                        format!("slot[{}] digest agreement", slot.index),
                        "blob_sha256 matches the image bytes",
                    ));
                } else {
                    checks.push(Check::fail(
                        format!("slot[{}] digest agreement", slot.index),
                        format!(
                            "manifest blob_sha256 {}… != image {}",
                            &declared[..8.min(declared.len())],
                            hex(&actual)
                        ),
                        exit_code::DIGEST,
                    ));
                }
            }
            (Some(_), Some(err)) => {
                checks.push(Check::fail(name, err.clone(), exit_code::MALFORMED));
            }
            (None, _) => {
                let detail = p
                    .manifest_validation
                    .clone()
                    .or_else(|| p.manifest_note.clone())
                    .unwrap_or_else(|| "no payload manifest".to_string());
                if p.manifest_validation.is_some() {
                    checks.push(Check::fail(name, detail, exit_code::MALFORMED));
                } else {
                    checks.push(Check::skip(name, detail));
                }
            }
        }
    }

    // 6. L2 entries[] ↔ the slot payloads' L1 entrypoints (tebako#494):
    //    every entries[].entrypoint path must EXIST in the referenced
    //    slot's image, and every entries[].name must be a DECLARED
    //    entrypoint of that payload (the name facet is unchecked, never
    //    failing, when the slot carries no usable L1 manifest —
    //    pre-manifest packages stay valid).
    match pm_result {
        Err(e) => checks.push(Check::fail(
            "package manifest",
            format!("extension block: {e}"),
            exit_code::MALFORMED,
        )),
        Ok(None) => {}
        Ok(Some(pm)) => {
            entry_checks(binary, &pm, &inspection, &mut checks)?;
            spawned_checks(&pm, &inspection, &mut checks);
        }
    }

    Ok((checks, Some(inspection)))
}

/// The entries cross-check of [`verify_package`] (one check per L2 entry,
/// named `entry[<name>]`). Entry paths stat in one mount per referenced
/// slot; a slot whose image already failed to mount skips (its manifest
/// check carries the failure — no double jeopardy).
fn entry_checks(
    binary: &Path,
    pm: &tpkg::PackageManifest,
    inspection: &PackageInspection,
    checks: &mut Vec<Check>,
) -> Result<(), InfoError> {
    use std::collections::{BTreeMap, HashMap};

    // Batch the path stats: one mount per referenced, mountable slot.
    let mut paths_by_slot: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for e in &pm.entries {
        let Some(slot) = e.slot else { continue };
        let i = slot as usize;
        let mountable = inspection.slots.get(i).is_some_and(|s| {
            s.format_hint != tpkg::TPKG_FORMAT_RUNTIME
                && s.payload.as_ref().is_some_and(|p| p.mount_error.is_none())
        });
        if mountable {
            paths_by_slot
                .entry(i)
                .or_default()
                .push(e.entrypoint.clone());
        }
    }
    let mut exists: HashMap<(usize, String), bool> = HashMap::new();
    for (i, paths) in &paths_by_slot {
        let s = &inspection.slots[*i];
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        let found = payload::region_paths_exist(binary, s.offset, s.size, &refs)?;
        for (p, ok) in paths.iter().zip(found) {
            exists.insert((*i, p.clone()), ok);
        }
    }

    for e in &pm.entries {
        let name = format!("entry[{}]", e.name);
        let Some(slot) = e.slot else {
            checks.push(Check::skip(
                name,
                "shared slice — resolved and checked at run time (spec 23 §13)",
            ));
            continue;
        };
        let i = slot as usize;
        let Some(s) = inspection.slots.get(i) else {
            checks.push(Check::fail(
                name,
                format!(
                    "names slot {slot} but the package carries {} slot(s)",
                    inspection.slots.len()
                ),
                exit_code::MALFORMED,
            ));
            continue;
        };
        if s.format_hint == tpkg::TPKG_FORMAT_RUNTIME {
            checks.push(Check::fail(
                name,
                format!(
                    "slot {slot} is a runtime (legacy role) slot — a launcher, never an entrypoint image"
                ),
                exit_code::MALFORMED,
            ));
            continue;
        }
        let p = s
            .payload
            .as_ref()
            .ok_or_else(|| err("internal error: slot payload missing at depth 1"))?;
        if let Some(merr) = &p.mount_error {
            checks.push(Check::skip(
                name,
                format!("slot {slot} unreadable (see slot[{slot}] manifest: {merr})"),
            ));
            continue;
        }
        if !exists
            .get(&(i, e.entrypoint.clone()))
            .copied()
            .unwrap_or(false)
        {
            checks.push(Check::fail(
                name,
                format!(
                    "entrypoint path '{}' does not exist in slot {slot}'s image",
                    e.entrypoint
                ),
                exit_code::MALFORMED,
            ));
            continue;
        }
        match (&p.manifest, &p.manifest_validation) {
            (Some(m), None) => {
                let declared: Vec<&str> = match &m.provides {
                    tpkg::Provides::App(a) => {
                        a.entrypoints.iter().map(|ep| ep.name.as_str()).collect()
                    }
                    tpkg::Provides::Toolkit(t) => {
                        t.executables.iter().map(|x| x.name.as_str()).collect()
                    }
                    _ => Vec::new(),
                };
                if declared.is_empty() {
                    checks.push(Check::fail(
                        name,
                        format!("slot {slot}'s payload declares no entrypoints"),
                        exit_code::MALFORMED,
                    ));
                } else if declared.contains(&e.name.as_str()) {
                    checks.push(Check::pass(
                        name,
                        format!("path exists in slot {slot}; name declared"),
                    ));
                } else {
                    checks.push(Check::fail(
                        name,
                        format!(
                            "'{}' is not a declared entrypoint of slot {slot}'s payload (declared: {})",
                            e.name,
                            declared.join(", ")
                        ),
                        exit_code::MALFORMED,
                    ));
                }
            }
            _ => checks.push(Check::pass(
                name,
                format!("path exists in slot {slot}; name unchecked (no usable L1 manifest)"),
            )),
        }
    }
    Ok(())
}

/// The spawned-edge cross-check of [`verify_package`] (spec 30 §2, spec
/// 32 §6, spec 23 §13.6; one check per L2 lock row plus one per
/// unmirrored L1 edge, named `spawned[<engine|payload>]`). The lock's
/// `spawned[]` rows are the bootstrap's ONLY edge source (the size gate
/// bars in-image reads), so press mirrors the app payload's L1
/// `requires[].kind: runtime` edges (runtime rows) and its
/// expose-carrying `requires[].kind: executable` edges (payload rows)
/// into them and this check pins the mirror both ways: every row mirrors
/// an L1 edge (runtime rows: engine + implementation parity; payload
/// rows: the payload pin parity — the constraint verbatim, the locked
/// version satisfying it, the expose set verbatim either way) and every
/// L1 edge has its row. A payload row's NESTED runtime row mirrors the
/// PROVIDER's L1 `kind: language` edge: cross-checked against the
/// carried provider image's manifest when that slot is inspectable,
/// skip-loud otherwise. The mirror source is the PRIMARY entry's slot
/// image; a package whose app slot carries no usable L1 manifest skips —
/// pre-manifest packages stay valid (the entry cross-check's rule).
fn spawned_checks(
    pm: &tpkg::PackageManifest,
    inspection: &PackageInspection,
    checks: &mut Vec<Check>,
) {
    let rows: &[tpkg::LockedSpawned] = match pm.lock.as_ref() {
        Some(lock) => &lock.spawned,
        None => &[],
    };
    let app_l1 = pm
        .entries
        .first()
        .and_then(|e| e.slot)
        .and_then(|slot| inspection.slots.get(slot as usize))
        .and_then(|s| s.payload.as_ref())
        .filter(|p| p.mount_error.is_none());
    let usable = match app_l1 {
        Some(p) => match (&p.manifest, &p.manifest_validation) {
            (Some(m), None) => Some(m),
            _ => None,
        },
        None => None,
    };
    let Some(l1) = usable else {
        for row in rows {
            checks.push(Check::skip(
                format!("spawned[{}]", spawned_label(row)),
                "the app payload carries no usable L1 manifest — the spawned-edge mirror is unchecked"
                    .to_string(),
            ));
        }
        return;
    };

    let edges: Vec<(&str, Option<&str>, &tpkg::Constraint, &[String])> = l1
        .requires
        .iter()
        .filter_map(|r| match r {
            tpkg::Requirement::Runtime {
                engine,
                implementation,
                constraint,
                expose,
            } => Some((
                engine.as_str(),
                implementation.as_deref(),
                constraint,
                expose.as_slice(),
            )),
            _ => None,
        })
        .collect();

    // The spec-32 axis: expose-carrying `kind: executable` edges mirror
    // into payload rows — (capability, provider pin, constraint, expose).
    let exec_edges: Vec<(&str, Option<&str>, &tpkg::Constraint, &[String])> = l1
        .requires
        .iter()
        .filter_map(|r| match r {
            tpkg::Requirement::Executable {
                name,
                payload,
                constraint,
                expose,
                ..
            } if !expose.is_empty() => Some((
                name.as_str(),
                payload.as_deref(),
                constraint,
                expose.as_slice(),
            )),
            _ => None,
        })
        .collect();

    for row in rows {
        let tpkg::LockedSpawned::Runtime(row) = row else {
            if let tpkg::LockedSpawned::Payload(p) = row {
                spawned_payload_check(p, &exec_edges, inspection, checks);
            }
            continue;
        };
        let name = format!("spawned[{}]", row.engine);
        let edge = edges.iter().find(|(engine, implementation, ..)| {
            *engine == row.engine.as_str() && *implementation == row.implementation.as_deref()
        });
        let Some((_, _, constraint, expose)) = edge else {
            checks.push(Check::fail(
                name,
                "the lock's spawned row mirrors no `kind: runtime` edge of the app payload's L1 manifest — re-press with a current tebako".to_string(),
                exit_code::MALFORMED,
            ));
            continue;
        };
        if constraint.as_str() != row.constraint.as_str() {
            checks.push(Check::fail(
                name,
                format!(
                    "the constraint mirror differs — L1 declares \"{}\", the lock carries \"{}\" — re-press with a current tebako",
                    constraint.as_str(),
                    row.constraint.as_str()
                ),
                exit_code::MALFORMED,
            ));
            continue;
        }
        if !tpkg::versions::from_validated(&row.constraint).matches(&row.version) {
            checks.push(Check::fail(
                name,
                format!(
                    "the locked version {} does not satisfy the mirrored constraint \"{}\" — re-press with a current tebako",
                    row.version,
                    row.constraint.as_str()
                ),
                exit_code::MALFORMED,
            ));
            continue;
        }
        let declared: std::collections::BTreeSet<&str> =
            expose.iter().map(String::as_str).collect();
        let mirrored: std::collections::BTreeSet<&str> =
            row.expose.iter().map(String::as_str).collect();
        if declared != mirrored {
            checks.push(Check::fail(
                name,
                format!(
                    "the expose mirror differs — L1 declares [{}], the lock carries [{}] — re-press with a current tebako",
                    declared.into_iter().collect::<Vec<_>>().join(", "),
                    mirrored.into_iter().collect::<Vec<_>>().join(", ")
                ),
                exit_code::MALFORMED,
            ));
            continue;
        }
        checks.push(Check::pass(
            name,
            format!(
                "mirrors the L1 edge; the locked version {} satisfies \"{}\"",
                row.version,
                row.constraint.as_str()
            ),
        ));
    }

    for (engine, implementation, ..) in &edges {
        let mirrored = rows.iter().any(|row| {
            let tpkg::LockedSpawned::Runtime(rt) = row else {
                return false;
            };
            rt.engine.as_str() == *engine && rt.implementation.as_deref() == *implementation
        });
        if !mirrored {
            checks.push(Check::fail(
                format!("spawned[{engine}]"),
                "the app payload's L1 manifest declares this spawned-runtime edge but the lock carries no row — a standalone package would never resolve it; re-press with a current tebako".to_string(),
                exit_code::MALFORMED,
            ));
        }
    }

    for (name, _, _, _) in &exec_edges {
        let mirrored = rows.iter().any(|row| {
            let tpkg::LockedSpawned::Payload(p) = row else {
                return false;
            };
            payload_row_matches_edge(p, name, &exec_edges)
        });
        if !mirrored {
            checks.push(Check::fail(
                format!("spawned[{name}]"),
                "the app payload's L1 manifest declares this expose-carrying `kind: executable` edge but the lock carries no payload row — a standalone package would never resolve it; re-press with a current tebako".to_string(),
                exit_code::MALFORMED,
            ));
        }
    }
}

/// The check-line label of a spawned lock row: the engine for a runtime
/// row (spec 30), the provider payload's name for a payload row
/// (spec 32).
fn spawned_label(row: &tpkg::LockedSpawned) -> &str {
    match row {
        tpkg::LockedSpawned::Runtime(rt) => rt.engine.as_str(),
        tpkg::LockedSpawned::Payload(p) => p.payload.as_str(),
    }
}

/// The mirror-identity rule of a payload row against the L1
/// expose-carrying executable edges (spec 32 §6): the constraint
/// verbatim, the expose SET verbatim, and the payload pin parity — an
/// edge that names `payload:` must name THIS provider; an unpinned edge
/// mirrors by capability + constraint + expose.
fn payload_row_matches_edge(
    row: &tpkg::LockedSpawnedPayload,
    edge_name: &str,
    edges: &[(&str, Option<&str>, &tpkg::Constraint, &[String])],
) -> bool {
    edges.iter().any(|(name, pin, constraint, expose)| {
        *name == edge_name
            && constraint.as_str() == row.constraint.as_str()
            && (pin.is_none() || *pin == Some(row.payload.as_str()))
            && expose
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>()
                == row
                    .expose
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>()
    })
}

/// The per-row check of a spawned-PAYLOAD lock row (spec 32 §6): the row
/// must mirror an expose-carrying L1 `kind: executable` edge (the
/// [`payload_row_matches_edge`] rule) and the locked provider version
/// must satisfy the mirrored constraint. The NESTED runtime row mirrors
/// the PROVIDER's L1 `kind: language` edge: the locked version pair must
/// satisfy the nested constraint, and the nested constraint is
/// cross-checked against the carried provider image's own manifest when
/// that slot is inspectable (skip-loud otherwise — a shared provider is
/// resolved at dispatch, never at verify).
fn spawned_payload_check(
    row: &tpkg::LockedSpawnedPayload,
    edges: &[(&str, Option<&str>, &tpkg::Constraint, &[String])],
    inspection: &PackageInspection,
    checks: &mut Vec<Check>,
) {
    let name = format!("spawned[{}]", row.payload);
    if !edges
        .iter()
        .any(|(edge_name, ..)| payload_row_matches_edge(row, edge_name, edges))
    {
        checks.push(Check::fail(
            name,
            "the lock's spawned payload row mirrors no expose-carrying `kind: executable` edge of the app payload's L1 manifest — re-press with a current tebako".to_string(),
            exit_code::MALFORMED,
        ));
        return;
    }
    if !tpkg::versions::from_validated(&row.constraint).matches(&row.version) {
        checks.push(Check::fail(
            name,
            format!(
                "the locked provider version {} does not satisfy the mirrored constraint \"{}\" — re-press with a current tebako",
                row.version,
                row.constraint.as_str()
            ),
            exit_code::MALFORMED,
        ));
        return;
    }
    checks.push(Check::pass(
        name,
        format!(
            "mirrors the L1 executable edge; the locked provider version {} satisfies \"{}\"",
            row.version,
            row.constraint.as_str()
        ),
    ));

    // The nested runtime row: the provider's OWN language edge, resolved
    // (spec 32 §6). Version-pair satisfaction always; the constraint
    // mirror cross-check when the provider image is inspectable here.
    let rt = &row.runtime;
    let rt_name = format!("spawned[{}].runtime", row.payload);
    if !tpkg::versions::from_validated(&rt.constraint).matches(&rt.version) {
        checks.push(Check::fail(
            rt_name,
            format!(
                "the nested locked runtime version {} does not satisfy the nested constraint \"{}\" — re-press with a current tebako",
                rt.version,
                rt.constraint.as_str()
            ),
            exit_code::MALFORMED,
        ));
        return;
    }
    let provider_l1 = row
        .image
        .slot
        .and_then(|slot| inspection.slots.get(slot as usize))
        .and_then(|s| s.payload.as_ref())
        .filter(|p| p.mount_error.is_none())
        .and_then(|p| match (&p.manifest, &p.manifest_validation) {
            (Some(m), None) => Some(m),
            _ => None,
        });
    let Some(provider_l1) = provider_l1 else {
        checks.push(Check::skip(
            rt_name,
            format!(
                "the provider image is not inspectable here — the nested runtime mirror is unchecked (the locked version {} satisfies the nested constraint \"{}\")",
                rt.version,
                rt.constraint.as_str()
            ),
        ));
        return;
    };
    let language_edge = provider_l1.requires.iter().find_map(|r| match r {
        tpkg::Requirement::Language { engine, constraint } if *engine == rt.engine => {
            Some(constraint)
        }
        _ => None,
    });
    match language_edge {
        Some(c) if c.as_str() == rt.constraint.as_str() => checks.push(Check::pass(
            rt_name,
            format!(
                "mirrors the provider's L1 language edge; the locked runtime {} ({}) satisfies \"{}\"",
                rt.version, rt.tebako, rt.constraint.as_str()
            ),
        )),
        Some(c) => checks.push(Check::fail(
            rt_name,
            format!(
                "the nested runtime constraint mirror differs — the provider's L1 declares \"{}\", the lock carries \"{}\" — re-press with a current tebako",
                c.as_str(),
                rt.constraint.as_str()
            ),
            exit_code::MALFORMED,
        )),
        None => checks.push(Check::fail(
            rt_name,
            format!(
                "the provider payload declares no `kind: language` edge for engine \"{}\" — the nested runtime row mirrors nothing — re-press with a current tebako",
                rt.engine
            ),
            exit_code::MALFORMED,
        )),
    }
}

/// Re-read the raw trailer bytes (the v2 signed region is computed over
/// them) — mirrors tebako-pkg's `signature_status` read.
fn read_trailer_bytes(binary: &Path, trailer: &tpkg::Manifest) -> Result<Vec<u8>, InfoError> {
    use std::io::{Read, Seek, SeekFrom};
    let tlen = tpkg::trailer_len(trailer);
    let mut f = std::fs::File::open(binary).map_err(|e| {
        err(format!(
            "{}: cannot re-read the trailer ({e})",
            binary.display()
        ))
    })?;
    f.seek(SeekFrom::End(-(tlen as i64)))
        .and_then(|_| {
            let mut buf = vec![0u8; tlen as usize];
            f.read_exact(&mut buf).map(|_| buf)
        })
        .map_err(|e| {
            err(format!(
                "{}: cannot re-read the trailer ({e})",
                binary.display()
            ))
        })
}

// ---------------------------------------------------------------------
// Image verification (tfs info --verify)
// ---------------------------------------------------------------------

/// The spec-15 §5 image checks (spec 03 validation: schema-valid
/// manifest, digests well-formed, signing state vs the actual signature
/// block).
pub fn verify_image(image: &Path, require_signed: bool) -> Result<Vec<Check>, InfoError> {
    let mut checks = Vec::new();
    let p: PayloadInspection = payload::inspect_image(image)?;

    // 1. The image must mount.
    if let Some(err) = &p.mount_error {
        checks.push(Check::fail("image", err.clone(), exit_code::MALFORMED));
        return Ok(checks);
    }
    checks.push(Check::pass(
        "image",
        format!(
            "mounts ({})",
            p.format
                .as_ref()
                .map_or("unknown".into(), |f| f.label.clone())
        ),
    ));

    // 2. Manifest schema (spec 03). Absent = a named note, not an error.
    let manifest = match (&p.manifest, &p.manifest_validation) {
        (Some(m), None) => {
            checks.push(Check::pass("manifest", "schema valid"));
            Some(m)
        }
        (Some(_), Some(err)) => {
            checks.push(Check::fail("manifest", err.clone(), exit_code::MALFORMED));
            None
        }
        (None, _) => {
            let detail = p
                .manifest_validation
                .clone()
                .or_else(|| p.manifest_note.clone())
                .unwrap_or_else(|| "no payload manifest".to_string());
            if p.manifest_validation.is_some() {
                checks.push(Check::fail("manifest", detail, exit_code::MALFORMED));
            } else {
                checks.push(Check::skip("manifest", detail));
            }
            None
        }
    };

    // 3. Signing state vs the actual signature block (`<image>.asc`) —
    //    before digest agreement, mirroring the package check order
    //    (signature → manifest schema → digest agreement).
    let declared_signed = manifest
        .map(|m| m.identity.signing.state == tpkg::SigningState::Signed)
        .unwrap_or(false);
    let declared_keyid = manifest.and_then(|m| m.identity.signing.keyid.clone());
    let asc = asc_sidecar(image);
    if declared_signed || require_signed || asc.is_file() {
        if !asc.is_file() {
            let why = if declared_signed {
                "manifest declares signing but no signature block"
            } else {
                "unsigned image (--require-signed)"
            };
            checks.push(Check::fail(
                "signature",
                format!("{why} ({} not found)", asc.display()),
                exit_code::SIGNATURE,
            ));
        } else {
            let data = std::fs::read(image)
                .map_err(|e| err(format!("{}: cannot read file ({e})", image.display())))?;
            let sig = read_sidecar_signature(&asc)?;
            let mut hint = [0u8; 8];
            if let Some(keyid) = &declared_keyid {
                if let Ok(bytes) = keyid_bytes(keyid) {
                    hint = bytes;
                }
            }
            let check = signature_check("signature", &data, &sig, &hint)?;
            // A trusted signature must also name the declared keyid (the
            // signer renders uppercase; the manifest holds lowercase hex).
            if let (CheckResult::Pass, Some(keyid)) = (&check.result, &declared_keyid) {
                if !check.detail.to_lowercase().contains(&keyid.to_lowercase()) {
                    checks.push(Check::fail(
                        "signature",
                        format!("manifest keyid {keyid} does not match the signer"),
                        exit_code::SIGNATURE,
                    ));
                } else {
                    checks.push(check);
                }
            } else {
                checks.push(check);
            }
        }
    } else {
        checks.push(Check::skip(
            "signature",
            "unsigned (no signature block declared)",
        ));
    }

    // 4. Digest agreement (manifest blob_sha256 vs image bytes, when a
    //    valid manifest declares it).
    if let Some(m) = manifest {
        let actual = sha256_region(image, 0, p.size_bytes)?;
        let declared = &m.identity.digest.blob_sha256;
        if hex(&actual) == *declared {
            checks.push(Check::pass(
                "digest agreement",
                "blob_sha256 matches the image bytes",
            ));
        } else {
            checks.push(Check::fail(
                "digest agreement",
                format!(
                    "manifest blob_sha256 {}… != image {}",
                    &declared[..8.min(declared.len())],
                    hex(&actual)
                ),
                exit_code::DIGEST,
            ));
        }
    }

    // 5. Tree hash (spec 03 §7): recompute the manifest-excluded merkle
    //    root over the mounted image and compare against the declared
    //    tree_hash. For ENCRYPTED images the declared tree_hash is the
    //    PLAINTEXT identity (spec 10 §2) — recomputation then needs the
    //    recipient key, so the check skips with a named reason rather
    //    than grading ciphertext against a plaintext digest.
    if let Some(m) = manifest {
        if m.identity.encryption.state == tpkg::EncryptionState::Encrypted {
            checks.push(Check::skip(
                "tree hash",
                "encrypted image: tree_hash is the plaintext identity (spec 10 §2); recomputing needs the recipient key",
            ));
        } else {
            match recompute_tree_hash(image) {
                Ok(rendered) if rendered == m.identity.digest.tree_hash => {
                    checks.push(Check::pass(
                        "tree hash",
                        "tree_hash matches the recomputed merkle root",
                    ));
                }
                Ok(rendered) => {
                    let declared = &m.identity.digest.tree_hash;
                    checks.push(Check::fail(
                        "tree hash",
                        format!(
                            "manifest tree_hash {}… != recomputed {}",
                            &declared[..12.min(declared.len())],
                            &rendered[..12.min(rendered.len())]
                        ),
                        exit_code::DIGEST,
                    ));
                }
                Err(reason) => {
                    // A recomputation that cannot run (special entries
                    // the merkle construction does not cover, symlink
                    // targets the backend cannot read) is a capability
                    // state, not evidence of tampering.
                    checks.push(Check::skip("tree hash", reason));
                }
            }
        }
    }

    Ok(checks)
}

/// Recompute the payload tree hash (spec 03 §7) over a fresh mount of
/// the image. `Err` is the named reason recomputation cannot run.
fn recompute_tree_hash(image: &Path) -> Result<String, String> {
    let mount = tfs::mount::build_from_file(&image.to_string_lossy(), "/mnt")
        .map_err(|e| format!("cannot mount for recomputation (errno {e})"))?;
    let walk = tfs::tree_walk::BackendTree(&*mount.backend);
    tpkg::tree_digest(&walk)
        .map(|d| tpkg::render_tree_hash(&d))
        .map_err(|e| match e {
            libc::ENOTSUP => {
                "tree contains entries recomputation cannot cover (special files, or symlink targets this backend cannot read)".to_string()
            }
            e => format!("recomputation failed (errno {e})"),
        })
}

/// Parse a 16-hex keyid into bytes (named error otherwise).
fn keyid_bytes(keyid: &str) -> Result<[u8; 8], InfoError> {
    if keyid.len() != 16 {
        return Err(err(format!("invalid keyid {keyid:?} (want 16 hex)")));
    }
    let mut out = [0u8; 8];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&keyid[2 * i..2 * i + 2], 16)
            .map_err(|_| err(format!("invalid keyid {keyid:?} (want 16 hex)")))?;
    }
    Ok(out)
}
