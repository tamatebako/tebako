//! The payload store layout (spec 05 §3, spec 07 §2, spec 32 §5) — the
//! SSOT for the `payloads/<name>/` path grammar:
//!
//! ```text
//! ~/.tebako/payloads/<name>/<version>.tfs              # the payload image (0444)
//! ~/.tebako/payloads/<name>/<version>.tfs.sha256       # the install-time trust anchor
//! ~/.tebako/payloads/<name>/<version>.manifest.yaml    # the manifest mirror
//! ~/.tebako/payloads/<name>/<version>.tree/            # the zero-runtime materialization
//! ```
//!
//! Every consumer of the store's payload area — the shim's dispatch and
//! management, the CLI's install, the driver's spawn-time cache-only
//! resolution (spec 32 §5) — derives paths from HERE, never re-authoring
//! the grammar (invariant 10). The manifest mirror is the plan-time
//! manifest source (spec 32 §2, locked): no image mounts at plan time.

use std::path::{Path, PathBuf};

use crate::manifest::{PayloadManifest, Provides};
use crate::versions;

/// One installed payload record: the image, its trust anchor, and the
/// parsed manifest mirror.
#[derive(Debug, Clone)]
pub struct CachedPayload {
    pub name: String,
    pub version: String,
    /// `<version>.tfs` — the payload image.
    pub image: PathBuf,
    /// `<version>.tfs.sha256` — the install-time trust anchor.
    pub sha_marker: PathBuf,
    /// `<version>.manifest.yaml` — the manifest mirror (the plan-time
    /// manifest source; spec 32 §2).
    pub manifest_mirror: PathBuf,
    /// `<version>.tree/` — the zero-runtime materialization tree (spec 07
    /// §2 — install-time extraction; a run never materializes).
    pub tree: PathBuf,
    pub manifest: PayloadManifest,
}

/// The payload's store directory (`payloads/<name>/`).
pub fn payload_dir(home: &Path, name: &str) -> PathBuf {
    home.join("payloads").join(name)
}

/// The payload image path (`payloads/<name>/<version>.tfs`).
pub fn image_path(home: &Path, name: &str, version: &str) -> PathBuf {
    payload_dir(home, name).join(format!("{version}.tfs"))
}

/// The trust-anchor sidecar path (`<version>.tfs.sha256`).
pub fn sha_marker_path(home: &Path, name: &str, version: &str) -> PathBuf {
    payload_dir(home, name).join(format!("{version}.tfs.sha256"))
}

/// The manifest mirror path (`<version>.manifest.yaml`).
pub fn manifest_mirror_path(home: &Path, name: &str, version: &str) -> PathBuf {
    payload_dir(home, name).join(format!("{version}.manifest.yaml"))
}

/// The zero-runtime materialization tree (`<version>.tree/`).
pub fn tree_path(home: &Path, name: &str, version: &str) -> PathBuf {
    payload_dir(home, name).join(format!("{version}.tree"))
}

/// Reject names/versions that would escape the cache layout (they become
/// path components).
pub fn check_component(what: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ' ' | '\t' | '\r' | '\n'))
        || value == "."
        || value == ".."
    {
        return Err(format!(
            "invalid {what} \"{value}\" — it must be a single path component"
        ));
    }
    Ok(())
}

/// Installed payload versions: the `<version>.tfs` files under
/// `payloads/<name>/` (a version with no image is not installed, whatever
/// else the record holds). Sorted ascending.
pub fn installed_versions(home: &Path, name: &str) -> Result<Vec<String>, String> {
    let dir = payload_dir(home, name);
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read {}: {e}", dir.display())),
    };
    let mut versions = Vec::new();
    for entry in rd.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(version) = file.strip_suffix(".tfs") {
            versions.push(version.to_string());
        }
    }
    versions.sort();
    Ok(versions)
}

/// Load one installed payload record. `Ok(None)` when the record is
/// absent or incomplete (no image, or no trust anchor — a partial install
/// is invisible, spec 05 §3); a present-but-damaged manifest mirror is a
/// named error, never a silent skip.
pub fn get(home: &Path, name: &str, version: &str) -> Result<Option<CachedPayload>, String> {
    check_component("payload name", name)?;
    check_component("payload version", version)?;
    let image = image_path(home, name, version);
    let sha_marker = sha_marker_path(home, name, version);
    if !image.is_file() || !sha_marker.is_file() {
        return Ok(None);
    }
    let manifest_mirror = manifest_mirror_path(home, name, version);
    let text = std::fs::read_to_string(&manifest_mirror).map_err(|e| {
        format!(
            "installed payload record {name} {version} is missing its manifest mirror {} ({e}) — the record is incomplete or damaged",
            manifest_mirror.display()
        )
    })?;
    let manifest = PayloadManifest::from_yaml(&text).map_err(|e| {
        format!(
            "corrupt payload manifest mirror {} ({e}) — the installed payload record is damaged",
            manifest_mirror.display()
        )
    })?;
    Ok(Some(CachedPayload {
        name: name.to_string(),
        version: version.to_string(),
        image,
        sha_marker,
        manifest_mirror,
        tree: tree_path(home, name, version),
        manifest,
    }))
}

/// The capability a payload provides (spec 32 §1): exact-name match
/// against `provides.entrypoints[].name` ∪ `provides.executables[].name`.
fn declares_capability(manifest: &PayloadManifest, capability: &str) -> bool {
    match &manifest.provides {
        Provides::App(app) => app.entrypoints.iter().any(|e| e.name == capability),
        Provides::Toolkit(tk) => tk.executables.iter().any(|e| e.name == capability),
        _ => false,
    }
}

/// spec 32 §1/§5: scan the store for installed payloads providing
/// `capability` with a version matching `constraint` — exact-name match
/// against each mirror's `provides.entrypoints[].name` ∪
/// `provides.executables[].name`. Zero candidates is the caller's named
/// `DependencyNotFound`; more than one is the caller's named
/// `AmbiguousProvider`. A damaged mirror fails the scan closed (named,
/// naming the record).
pub fn find_capability_providers(
    home: &Path,
    capability: &str,
    constraint: &crate::Constraint,
) -> Result<Vec<CachedPayload>, String> {
    let constraint = versions::from_validated(constraint);
    let payloads_dir = home.join("payloads");
    let rd = match std::fs::read_dir(&payloads_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read {}: {e}", payloads_dir.display())),
    };
    let mut providers = Vec::new();
    for entry in rd.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        for version in installed_versions(home, &name)? {
            if !constraint.matches(&version) {
                continue;
            }
            if let Some(record) = get(home, &name, &version)? {
                if declares_capability(&record.manifest, capability) {
                    providers.push(record);
                }
            }
        }
    }
    Ok(providers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_home(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tpkg-payload-store-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ))
    }

    /// Seed one payload record: image + trust anchor + mirror.
    fn seed(home: &Path, kind: &str, name: &str, version: &str, provides: &str) {
        let dir = payload_dir(home, name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{version}.tfs")), b"image").unwrap();
        std::fs::write(
            dir.join(format!("{version}.tfs.sha256")),
            format!("{:x}\n", 0u8),
        )
        .unwrap();
        let manifest = format!(
            "identity:\n  schema_version: 1\n  kind: {kind}\n  name: {name}\n  version: \"{version}\"\n\
            \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-09-05T00:00:00Z\"\n\
            \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
            \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
            \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
            provides:\n{provides}"
        );
        std::fs::write(dir.join(format!("{version}.manifest.yaml")), manifest).unwrap();
    }

    #[test]
    fn the_path_grammar_is_the_store_layout() {
        let home = Path::new("/h");
        assert_eq!(
            payload_dir(home, "xml2rfc"),
            PathBuf::from("/h/payloads/xml2rfc")
        );
        assert_eq!(
            image_path(home, "xml2rfc", "3.34.0"),
            PathBuf::from("/h/payloads/xml2rfc/3.34.0.tfs")
        );
        assert_eq!(
            sha_marker_path(home, "xml2rfc", "3.34.0"),
            PathBuf::from("/h/payloads/xml2rfc/3.34.0.tfs.sha256")
        );
        assert_eq!(
            manifest_mirror_path(home, "xml2rfc", "3.34.0"),
            PathBuf::from("/h/payloads/xml2rfc/3.34.0.manifest.yaml")
        );
        assert_eq!(
            tree_path(home, "xml2rfc", "3.34.0"),
            PathBuf::from("/h/payloads/xml2rfc/3.34.0.tree")
        );
    }

    #[test]
    fn check_component_refuses_escapes() {
        for bad in ["", "a/b", "a\\b", ".", "..", "a b"] {
            assert!(check_component("payload name", bad).is_err(), "{bad:?}");
        }
        assert!(check_component("payload name", "xml2rfc").is_ok());
    }

    #[test]
    fn installed_versions_lists_images_only() {
        let home = tmp_home("versions");
        seed(
            &home,
            "app",
            "xml2rfc",
            "3.34.0",
            "  entrypoints: [{name: xml2rfc, path: /bin/xml2rfc}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        seed(
            &home,
            "app",
            "xml2rfc",
            "3.30.1",
            "  entrypoints: [{name: xml2rfc, path: /bin/xml2rfc}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        // A mirror with no image is not an installed version.
        std::fs::write(
            payload_dir(&home, "xml2rfc").join("9.9.9.manifest.yaml"),
            "identity: {}\n",
        )
        .unwrap();
        assert_eq!(
            installed_versions(&home, "xml2rfc").unwrap(),
            vec!["3.30.1".to_string(), "3.34.0".to_string()]
        );
        assert_eq!(installed_versions(&home, "absent"), Ok(Vec::new()));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn get_requires_a_complete_record_and_names_a_damaged_one() {
        let home = tmp_home("get");
        seed(
            &home,
            "app",
            "xml2rfc",
            "3.34.0",
            "  entrypoints: [{name: xml2rfc, path: /bin/xml2rfc, runtime_requirement: {engine: python, constraint: \">= 3.10\"}}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        let record = get(&home, "xml2rfc", "3.34.0").unwrap().unwrap();
        assert_eq!(record.version, "3.34.0");
        assert!(declares_capability(&record.manifest, "xml2rfc"));
        assert!(!declares_capability(&record.manifest, "other"));
        // Absent and incomplete records are Ok(None) — invisible.
        assert!(get(&home, "xml2rfc", "9.9.9").unwrap().is_none());
        std::fs::remove_file(sha_marker_path(&home, "xml2rfc", "3.34.0")).unwrap();
        assert!(get(&home, "xml2rfc", "3.34.0").unwrap().is_none());
        // A present-but-corrupt mirror is a named error, never a skip.
        std::fs::write(sha_marker_path(&home, "xml2rfc", "3.34.0"), "0\n").unwrap();
        std::fs::write(
            manifest_mirror_path(&home, "xml2rfc", "3.34.0"),
            "not: [a manifest\n",
        )
        .unwrap();
        let err = get(&home, "xml2rfc", "3.34.0").unwrap_err();
        assert!(err.contains("corrupt payload manifest mirror"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn find_capability_providers_matches_entrypoints_and_executables() {
        let home = tmp_home("scan");
        // An app payload providing the capability via an entrypoint.
        seed(
            &home,
            "app",
            "xml2rfc",
            "3.34.0",
            "  entrypoints: [{name: xml2rfc, path: /bin/xml2rfc, runtime_requirement: {engine: python, constraint: \">= 3.10\"}}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        // A toolkit payload providing it via an executable (spec 32 §1's
        // widened capability source).
        seed(
            &home,
            "toolkit",
            "xml2rfc-tk",
            "3.34.0",
            "  executables: [{name: xml2rfc, path: /bin/xml2rfc}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        // An out-of-constraint version of the same provider.
        seed(
            &home,
            "app",
            "xml2rfc",
            "3.30.1",
            "  entrypoints: [{name: xml2rfc, path: /bin/xml2rfc}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        // An unrelated payload.
        seed(
            &home,
            "app",
            "metanorma",
            "1.2.3",
            "  entrypoints: [{name: metanorma, path: /bin/metanorma}]\n  platforms: universal\n  capabilities: {exec: true, read: true}\n",
        );
        let c = crate::Constraint::new(">= 3.34").unwrap();
        let hits = find_capability_providers(&home, "xml2rfc", &c).unwrap();
        let mut names: Vec<_> = hits.iter().map(|p| p.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["xml2rfc".to_string(), "xml2rfc-tk".to_string()]);
        // Zero candidates is the caller's DependencyNotFound data.
        assert!(find_capability_providers(&home, "absent-tool", &c)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
