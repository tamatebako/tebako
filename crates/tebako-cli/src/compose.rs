//! `tebako press --compose <tebako.yaml>` (spec 23 §3 D2 / §13) — the
//! composition document's press path.
//!
//! tpkg owns the document MODEL (`tpkg::parse_compose`); this module owns
//! the press-side pipeline: the `--carry`/`--share` overrides (D5 beats
//! the document, spec 23 §13.2), the runtime-row checks, and the FULL
//! closure resolution — every slice resolved, fetched, verified and
//! pinned at build time, so the §4 lock bakes exactly what press tested
//! (run-time resolution then follows the locked digest, never fresh
//! semver).
//!
//! Press is NOT install: the bytes land in the payload cache (the same
//! trust-anchored store), but no mirrors, shims, or materialized trees
//! are written — those are the install verb's surface (spec 07 §2).

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use tebako_resolve::registry::RegistryPlatforms;
use tebako_resolve::{Fetcher, PayloadCache, ResolveError, Transport};
use tebako_shim::versions;
use tpkg::{ComposeDoc, ComposePreset, Constraint, Platform, Platforms};

use crate::error::TebakoError;
use crate::image_manifest;
use crate::install;

// The spec 06 §4 named set (the codes install.rs already owns; compose
// errors are manifest/resolution failures of the same family).
const EX_TEBAKO_MANIFEST: i32 = 65;

fn err(message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, EX_TEBAKO_MANIFEST)
}

/// One resolved composition slice: the concrete version, the carry
/// verdict, the digest pin (spec 23 §13.3 — the single `universal`
/// digest or the host triplet's row of the per-triplet map), the cache
/// path of the verified bytes (carried slices stitch from here), and
/// the canonical fetch coordinates (the lock's `source:`).
#[derive(Debug)]
pub struct ComposeSlice {
    pub name: String,
    pub version: String,
    pub carry: bool,
    /// The declared mount point; `None` = a cache prime (no consumer
    /// edge declared one — spec 23 §3's D2 rows carry no mount key).
    pub mount: Option<String>,
    pub pin: tpkg::DigestPin,
    pub cache_path: PathBuf,
    pub source: String,
}

/// Read + parse + validate the composition document (tpkg owns the
/// model; the Phase-R jail keys are refused there by name).
pub fn load(path: &Path) -> Result<(ComposeDoc, Vec<String>), TebakoError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        err(format!(
            "cannot read the compose document {}: {e}",
            path.display()
        ))
    })?;
    tpkg::parse_compose(&text).map_err(|e| err(format!("{}: {e}", path.display())))
}

/// The compose document's runtime row against the press's ruby: ruby is
/// the only engine today, and the requirement must COVER the resolved
/// interpreter version (press always resolves by constraint, spec 23
/// §4 — a disagreement is named, never a silently different runtime).
pub fn check_runtime_row(doc: &ComposeDoc, ruby_ver: &str) -> Result<(), TebakoError> {
    if doc.runtime.name != "ruby" {
        return Err(err(format!(
            "the compose runtime is '{}' — ruby is the only runtime engine today",
            doc.runtime.name
        )));
    }
    let requirement = doc
        .runtime
        .requirement
        .as_ref()
        .expect("parse_compose refuses a runtime row without a requirement");
    if !versions::from_validated(requirement).matches(ruby_ver) {
        return Err(err(format!(
            "the compose runtime requirement '{}' does not cover the pressed ruby {ruby_ver} — pass -R/--Ruby (or a Gemfile constraint) that agrees with the document",
            requirement.as_str()
        )));
    }
    Ok(())
}

/// The entrypoint selector: with a local root (-r/-e) the package's
/// entry is the app's own, so the document may only name the package
/// itself. Selecting a slice-provided command is the pointer-package
/// form — supported by the bootstrap/driver, but its press is the N7
/// milestone.
pub fn check_entrypoint(doc: &ComposeDoc, package_stem: &str) -> Result<(), TebakoError> {
    if let Some(entrypoint) = &doc.entrypoint {
        if entrypoint != package_stem {
            return Err(err(format!(
                "compose entrypoint '{entrypoint}' selects a slice-provided command — pointer-package presses (the entry rides a compose slice, not the local root) are the N7 milestone; with -r/-e the package entry is the app's own ('{package_stem}')"
            )));
        }
    }
    Ok(())
}

/// The D5 overrides (spec 23 §13.2): `--carry=all|none|<name,…>` and
/// `--share=<name,…>` rewrite the document's per-slice `carry:` verdicts
/// (explicit invocation beats authored defaults). Names resolve against
/// the runtime row plus the slice rows; an unknown name, a
/// carry/share conflict, or `--share` naming the local app are named
/// errors — never a silent no-op.
pub fn apply_overrides(
    doc: &mut ComposeDoc,
    carry: Option<&str>,
    share: Option<&str>,
    app_name: &str,
) -> Result<(), TebakoError> {
    fn declared_names(doc: &ComposeDoc) -> String {
        let mut names: Vec<&str> = vec![doc.runtime.name.as_str()];
        names.extend(doc.slices.iter().map(|s| s.name.as_str()));
        names.join(", ")
    }
    fn set(
        doc: &mut ComposeDoc,
        name: &str,
        value: bool,
        app_name: &str,
    ) -> Result<(), TebakoError> {
        if doc.runtime.name == name {
            doc.runtime.carry = Some(value);
            return Ok(());
        }
        if let Some(slice) = doc.slices.iter_mut().find(|s| s.name == name) {
            slice.carry = Some(value);
            return Ok(());
        }
        if name == app_name && !value {
            return Err(err(format!(
                "--share names the app payload '{name}' — the app is pressed from the local root and always rides in the package; to share it, publish it as a payload and compose it by name (pointer packages are the N7 milestone)"
            )));
        }
        Err(err(format!(
            "--{} names '{name}' which the composition does not declare (declared: {})",
            if value { "carry" } else { "share" },
            declared_names(doc)
        )))
    }
    let list = |flag: Option<&str>| -> Vec<String> {
        flag.map(|f| {
            f.split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect()
        })
        .unwrap_or_default()
    };
    let carry_names = list(carry);
    let share_names = list(share);
    if let Some(name) = carry_names.iter().find(|n| share_names.contains(n)) {
        return Err(err(format!(
            "--carry and --share both name '{name}' — a slice is either carried or shared, never both"
        )));
    }
    match carry {
        Some(flag) if flag.trim() == "all" || flag.trim() == "none" => {
            let value = flag.trim() == "all";
            doc.runtime.carry = Some(value);
            for slice in doc.slices.iter_mut() {
                slice.carry = Some(value);
            }
        }
        _ => {
            for name in &carry_names {
                set(doc, name, true, app_name)?;
            }
        }
    }
    for name in &share_names {
        set(doc, name, false, app_name)?;
    }
    Ok(())
}

/// Resolve the document's full closure (spec 23 §4/§6): every slice row
/// plus the transitive `requires:` walk of the embedded manifests, each
/// fetched, verified, and pinned. Top-level rows resolve in document
/// order; a re-encountered name must AGREE with the locked version
/// (one version per package) and an edge-declared `mount:` upgrades a
/// cache-prime row to a mounted one (conflicting mounts are named).
/// Deps discovered in the walk are carried unless the document lists
/// them (their row's verdict stands — doc rows resolve first).
///
/// `pub` for the integration tests (the `install_with` precedent); the
/// transport injection keeps the suite on `file://` mirrors.
pub fn resolve_closure<T: Transport>(
    home: &Path,
    fetcher: &Fetcher<T>,
    doc: &ComposeDoc,
    preset: ComposePreset,
    host: Platform,
) -> Result<Vec<ComposeSlice>, TebakoError> {
    struct Pending {
        name: String,
        requirement: Option<Constraint>,
        carry: bool,
        mount: Option<String>,
        platforms: Option<Platforms>,
        consumer: Option<String>,
    }
    impl Pending {
        fn from_doc(doc: &ComposeDoc, preset: ComposePreset) -> VecDeque<Pending> {
            doc.slices
                .iter()
                .map(|s| Pending {
                    name: s.name.clone(),
                    requirement: s.requirement.clone(),
                    carry: s.carry.unwrap_or_else(|| preset.default_carry(false)),
                    mount: None,
                    platforms: s.platforms.clone(),
                    consumer: None,
                })
                .collect()
        }
    }

    let mut work: VecDeque<Pending> = Pending::from_doc(doc, preset);
    let mut resolved: Vec<ComposeSlice> = Vec::new();
    let cache = PayloadCache::with_root(home);

    while let Some(pending) = work.pop_front() {
        if let Some(existing) = resolved.iter_mut().find(|s| s.name == pending.name) {
            if let Some(requirement) = &pending.requirement {
                if !versions::from_validated(requirement).matches(&existing.version) {
                    return Err(err(format!(
                        "slice '{}' is required at '{}' (by {}) but the composition already locked {} — one version per package",
                        pending.name,
                        requirement.as_str(),
                        pending.consumer.as_deref().unwrap_or("the document"),
                        existing.version
                    )));
                }
            }
            match (&existing.mount, &pending.mount) {
                (None, Some(mount)) => existing.mount = Some(mount.clone()),
                (Some(a), Some(b)) if a != b => {
                    return Err(err(format!(
                        "slice '{}' is required mounted at both '{a}' and '{b}' — one mount point per slice",
                        pending.name
                    )));
                }
                _ => {}
            }
            continue;
        }

        let mut found = install::find_in_registries(home, fetcher, &pending.name)?;
        let (reg_ref, payload) = match found.len() {
            0 => {
                return Err(err(format!(
                    "compose slice '{}'{} is not carried by any registered registry — register one with: tebako add-registry <ref>",
                    pending.name,
                    pending
                        .consumer
                        .map(|c| format!(" (required by {c})"))
                        .unwrap_or_default()
                )));
            }
            1 => found.pop().expect("len == 1 checked"),
            n => {
                return Err(err(format!(
                    "compose slice '{}' is listed by {n} registered registries (AmbiguousRegistries):\n{}\n  narrow it with an explicit registry set",
                    pending.name,
                    found
                        .iter()
                        .map(|(r, _)| format!("    - {r}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )));
            }
        };
        match payload.kind {
            tpkg::PayloadKind::Runtime | tpkg::PayloadKind::Language => {
                return Err(err(format!(
                    "compose slice '{}' is a runtime payload — the composition's runtime: row owns the engine (spec 23 §3)",
                    pending.name
                )));
            }
            _ => {}
        }

        let version = match &pending.requirement {
            Some(requirement) => {
                let eval = versions::from_validated(requirement);
                payload
                    .versions
                    .iter()
                    .map(|v| v.version.as_str())
                    .filter(|v| eval.matches(v))
                    .max_by(|a, b| versions::compare(a, b))
                    .map(str::to_string)
                    .ok_or_else(|| {
                        err(format!(
                            "no published version of '{}' satisfies '{}' (available: {})",
                            pending.name,
                            requirement.as_str(),
                            payload
                                .versions
                                .iter()
                                .map(|v| v.version.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    })?
            }
            None => payload
                .default_version()
                .map(|v| v.version.clone())
                .ok_or_else(|| {
                    err(format!(
                        "registry {reg_ref} pins no default for '{}' — name a requirement in the compose document",
                        pending.name
                    ))
                })?,
        };
        let entry = payload.version(&version).expect("selected above");

        // The §13.3 coverage assertion, checked against the registry's
        // mirrored coverage rows (fail-closed, never a silent fallback).
        let declared = match &entry.platforms {
            RegistryPlatforms::Universal => Platforms::Universal,
            RegistryPlatforms::PerTriplet(map) => {
                Platforms::Triplets(map.keys().copied().collect())
            }
        };
        tpkg::check_platforms_assertion(&pending.name, &declared, pending.platforms.as_ref(), host)
            .map_err(|e| err(e.to_string()))?;

        let plan =
            install::plan_from_registry_entry(&reg_ref, &payload, Some(&version), Some(host))?;

        // Fetch → verify → cache: a hit stands on its trust anchor
        // (spec 05 §4); a miss fetches and verifies BEFORE anything
        // enters the cache. No mirrors/shims — press is not install.
        let cached = match cache
            .get(&plan.name, &plan.version)
            .map_err(install::map_resolve)?
        {
            Some(cached) => cached,
            None => {
                if tebako_resolve::cache::offline() {
                    return Err(install::map_resolve(ResolveError::Offline {
                        what: format!("payload {}@{}", plan.name, plan.version),
                    }));
                }
                let fetched = fetcher
                    .fetch(&plan.reference)
                    .map_err(install::map_resolve)?;
                let (cached, _status) = cache
                    .install(
                        &plan.name,
                        &plan.version,
                        plan.expected_sha256.as_deref(),
                        || Ok(fetched),
                    )
                    .map_err(install::map_resolve)?;
                cached
            }
        };

        // The lock's pin (spec 23 §13.3): the single universal digest,
        // or the host triplet's row — press verifies the host's bytes;
        // other triplets are never asserted.
        let pin = match &entry.platforms {
            RegistryPlatforms::Universal => tpkg::DigestPin::One(cached.sha256.clone()),
            RegistryPlatforms::PerTriplet(_) => tpkg::DigestPin::PerTriplet(BTreeMap::from([(
                host.release_asset_name().to_string(),
                cached.sha256.clone(),
            )])),
        };

        // The embedded manifest (tier 1, authoritative) cross-checks the
        // registry's identity and drives the transitive walk; an image
        // without one contributes no edges.
        let mut deps = Vec::new();
        if let Some(text) = image_manifest::read_embedded_manifest(&cached.path)? {
            let manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
                err(format!(
                    "the embedded manifest of {} does not parse: {e}",
                    cached.path.display()
                ))
            })?;
            if manifest.identity.name != plan.name || manifest.identity.version != plan.version {
                return Err(err(format!(
                    "the embedded manifest declares {} {} but the composition resolved {} {} — the registry entry is inconsistent with the payload it names",
                    manifest.identity.name, manifest.identity.version, plan.name, plan.version
                )));
            }
            for requirement in &manifest.requires {
                let (name, constraint, mount) = match requirement {
                    // The runtime axis: the composition's runtime: row
                    // owns it — never walked here.
                    tpkg::Requirement::Language { .. } => continue,
                    tpkg::Requirement::Toolkit {
                        name,
                        constraint,
                        mount,
                        ..
                    } => (name, constraint, mount),
                    tpkg::Requirement::Data {
                        name,
                        constraint,
                        mount,
                    } => (name, constraint, mount),
                };
                let authored = doc.slices.iter().find(|s| &s.name == name);
                deps.push(Pending {
                    name: name.clone(),
                    requirement: Some(constraint.clone()),
                    carry: match authored {
                        Some(row) => row.carry.unwrap_or_else(|| preset.default_carry(false)),
                        // Discovered deps ride in the package unless the
                        // document says otherwise (spec 23 §13.2).
                        None => true,
                    },
                    mount: mount.clone(),
                    platforms: authored.and_then(|row| row.platforms.clone()),
                    consumer: Some(plan.name.clone()),
                });
            }
        }

        resolved.push(ComposeSlice {
            name: plan.name,
            version: plan.version,
            carry: pending.carry,
            mount: pending.mount,
            pin,
            cache_path: cached.path,
            source: plan.reference.to_string(),
        });
        work.extend(deps);
    }
    Ok(resolved)
}

/// The tebako home the compose closure resolves against — tebako-shim
/// owns the rule ($TEBAKO_HOME > platform default).
pub(crate) fn tebako_home() -> Result<PathBuf, TebakoError> {
    let env: BTreeMap<String, String> = std::env::vars().collect();
    tebako_shim::tebako_home(&env).map_err(install::map_shim)
}
