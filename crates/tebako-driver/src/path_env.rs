//! Dependency bin dirs on `PATH` (spec 22 §3.2): a bare command name
//! (`system("java …")`, mnconvert's form) resolves with no payload code
//! learning tebako — the driver prepends every co-mounted DEPENDENCY
//! image's declared bin dirs to `PATH` in the handoff env. Each image
//! declares its own executables (the in-image manifest:
//! `provides.entrypoints[].path` for an app, `provides.executables[].path`
//! for a toolkit); the driver flows the dirnames, joined under the mount
//! point, in triple order — the image declares, the driver flows, no
//! second copy of the knowledge anywhere.
//!
//! The FIRST triple is the app payload the entry resolves against
//! (spec 17 §1) — its own bins are the entrypoint's business and are
//! never prepended. A mounted image without a readable manifest declares
//! no bins (plain images mount fine — the boot-smoke fixture case); a
//! corrupt manifest is the image lying about its self-description — a
//! named 65. On ELF the interposed exec loop then resolves the bare name
//! through the VFS (spec 22 §3.1); everything else takes the explicit
//! `TEBAKO_MOUNT_<SLUG>` surface (spec 22 §6).

use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::driver::{join_mount, read_mounted_text, DriverError, Env};
use crate::handoff::ImageSpec;
use crate::EX_TEBAKO_MANIFEST;
use tpkg::{PayloadManifest, Provides, PAYLOAD_MANIFEST_PATH};

/// Prepend the dependency mounts' declared bin dirs to `PATH`. Called
/// per boot after the mounts are established, next to the mount-vars
/// export (spec 22 §6).
pub fn export(images: &[ImageSpec], env: &dyn Env) -> Result<(), DriverError> {
    let mut dirs: Vec<String> = Vec::new();
    for spec in images.iter().skip(1) {
        for declared in declared_bin_dirs(spec)? {
            let dir = join_mount(&spec.mount, &declared);
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    if let Some(joined) = compose(env.var("PATH"), &dirs)? {
        env.set_var("PATH", &joined.to_string_lossy());
    }
    Ok(())
}

/// The PATH value with `dirs` prepended ahead of the inherited value
/// (`None` when there is nothing to prepend — PATH then rides through
/// untouched, never rewritten).
fn compose(existing: Option<String>, dirs: &[String]) -> Result<Option<OsString>, DriverError> {
    if dirs.is_empty() {
        return Ok(None);
    }
    let mut paths: Vec<PathBuf> = dirs.iter().map(PathBuf::from).collect();
    if let Some(existing) = existing {
        paths.extend(std::env::split_paths(OsStr::new(&existing)));
    }
    std::env::join_paths(&paths).map(Some).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot prepend the dependency bin dirs to PATH: {e}"),
        )
    })
}

/// The declared bin dirs of one mounted image, as in-image absolute
/// spellings (see the module doc).
fn declared_bin_dirs(spec: &ImageSpec) -> Result<Vec<String>, DriverError> {
    let path = join_mount(&spec.mount, PAYLOAD_MANIFEST_PATH);
    let Ok(text) = read_mounted_text(&path) else {
        return Ok(Vec::new());
    };
    let manifest = PayloadManifest::from_yaml(&text).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            format!(
                "corrupt {PAYLOAD_MANIFEST_PATH} in the image mounted at '{}' ({e}) — the payload's self-description lies",
                spec.mount
            ),
        )
    })?;
    Ok(bin_dirs(&manifest.provides))
}

/// The pure extraction: app entrypoints and toolkit executables declare
/// executable-providing bins; runtime/data/language provides declare
/// none. In-image paths are absolute (manifest-validated); the bin dir
/// is the path's parent, first occurrence wins, no duplicates.
fn bin_dirs(provides: &Provides) -> Vec<String> {
    let paths: Vec<&str> = match provides {
        Provides::App(p) => p.entrypoints.iter().map(|e| e.path.as_str()).collect(),
        Provides::Toolkit(p) => p.executables.iter().map(|e| e.path.as_str()).collect(),
        _ => return Vec::new(),
    };
    let mut dirs: Vec<String> = Vec::new();
    for parent in paths.iter().filter_map(|path| Path::new(path).parent()) {
        let dir = parent.to_string_lossy().into_owned();
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct MapEnv(RefCell<HashMap<String, String>>);

    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }
    }

    fn manifest(kind: &str, provides: &str) -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: {kind}\n  name: x\n  version: \"1\"\n  \
             producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-13T00:00:00Z\"\n  \
             digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
             signing: {{state: unsigned}}\n  encryption: {{state: none}}\n{provides}\n",
            z = "0".repeat(64)
        )
    }

    #[test]
    fn app_entrypoints_and_toolkit_executables_declare_bins() {
        let app = PayloadManifest::from_yaml(&manifest(
            "app",
            "provides:\n  entrypoints:\n    - {name: a, path: /bin/a}\n    - {name: b, path: /sbin/b}\n  \
             platforms: universal\n  capabilities: {exec: true, read: true}",
        ))
        .unwrap();
        assert_eq!(bin_dirs(&app.provides), vec!["/bin", "/sbin"]);

        let toolkit = PayloadManifest::from_yaml(&manifest(
            "toolkit",
            "provides:\n  executables:\n    - {name: java, path: /bin/java}\n    - {name: javac, path: /bin/javac}\n  \
             platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}",
        ))
        .unwrap();
        // Both executables share /bin — declared once.
        assert_eq!(bin_dirs(&toolkit.provides), vec!["/bin"]);
    }

    #[test]
    fn data_and_runtime_declare_no_bins() {
        let data = PayloadManifest::from_yaml(&manifest(
            "data",
            "provides:\n  mount_semantics: {suggested: /usr/share/x}\n  capabilities: {exec: false, read: true}",
        ))
        .unwrap();
        assert!(bin_dirs(&data.provides).is_empty());
    }

    #[test]
    fn a_root_level_executable_declares_the_image_root() {
        let toolkit = PayloadManifest::from_yaml(&manifest(
            "toolkit",
            "provides:\n  executables:\n    - {name: x, path: /x}\n  \
             platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}",
        ))
        .unwrap();
        assert_eq!(bin_dirs(&toolkit.provides), vec!["/"]);
    }

    #[test]
    fn compose_prepends_ahead_of_the_inherited_path() {
        let joined = compose(
            Some("/usr/bin:/bin".to_string()),
            &["/opt/openjdk/bin".to_string()],
        )
        .unwrap()
        .unwrap();
        let want =
            std::env::join_paths(["/opt/openjdk/bin", "/usr/bin", "/bin"].map(PathBuf::from))
                .unwrap();
        assert_eq!(joined, want);
    }

    #[test]
    fn compose_without_inheritance_and_without_dirs() {
        let joined = compose(None, &["/opt/openjdk/bin".to_string()])
            .unwrap()
            .unwrap();
        assert_eq!(joined, OsString::from("/opt/openjdk/bin"));
        assert!(compose(Some("/usr/bin".to_string()), &[])
            .unwrap()
            .is_none());
    }

    #[test]
    fn export_without_dependency_mounts_touches_nothing() {
        let env = MapEnv(RefCell::new(HashMap::from([(
            "PATH".to_string(),
            "/usr/bin".to_string(),
        )])));
        export(&[], &env).unwrap();
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("/usr/bin")
        );
    }
}
