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
//!
//! The host-launcher tier (armed when the env image delivers the preload
//! shim; unix): each declared dependency executable is also materialized
//! through the exec cache and mirrored by a self-injecting wrapper on
//! ONE host dir (`<dl-root>/wrap-bin/`) that leads `PATH`. The wrapper
//! re-arms the platform's injection var EXPLICITLY for its child — SIP
//! strips an inherited `DYLD_INSERT_LIBRARIES` at an Apple-binary exec
//! (spec 22 §3.1's named boundary), but a variable a script sets itself
//! survives — so the shell-string form resolves through `PATH`, runs the
//! wrapper, and the shim loads into the final binary exactly as on ELF
//! (probe 2026-08-13: `/bin/sh -c <wrapper>` loads the dylib past the
//! strip). First triple order wins on a basename collision; a declared
//! executable that cannot be materialized is the image lying — a named
//! 65, never a skipped entry.

use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::driver::{join_mount, read_mounted_text, DriverError, Env};
use crate::handoff::ImageSpec;
#[cfg(unix)]
use crate::EX_TEBAKO_IO;
use crate::EX_TEBAKO_MANIFEST;
#[cfg(unix)]
use tfs::context::context;
use tpkg::{PayloadManifest, Provides, PAYLOAD_MANIFEST_PATH};

/// Prepend the dependency mounts' declared bin dirs to `PATH` — led by
/// the launcher dir when the shim was delivered (`shim_host`, unix).
/// Called per boot after the mounts are established, next to the
/// mount-vars export (spec 22 §6).
pub fn export(
    images: &[ImageSpec],
    env: &dyn Env,
    shim_host: Option<&str>,
) -> Result<(), DriverError> {
    // Each dependency image's manifest is read once: the bin dirs ride
    // PATH on every platform; the declared executables feed the launcher
    // tier where one is delivered.
    let mut deps: Vec<(String, PayloadManifest)> = Vec::new();
    for spec in images.iter().skip(1) {
        if let Some(manifest) = mounted_manifest(spec)? {
            deps.push((spec.mount.clone(), manifest));
        }
    }
    let mut dirs: Vec<String> = Vec::new();
    #[cfg(unix)]
    if let Some(shim) = shim_host {
        if let Some(dir) = materialize_launchers(&deps, shim)? {
            dirs.push(dir);
        }
    }
    #[cfg(not(unix))]
    let _ = shim_host;
    for (mount, manifest) in &deps {
        for declared in bin_dirs(&manifest.provides) {
            let dir = join_mount(mount, &declared);
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

/// The mounted image's own manifest, when readable (see the module
/// doc): no manifest declares no bins; a corrupt one is the image lying
/// about its self-description — a named 65.
fn mounted_manifest(spec: &ImageSpec) -> Result<Option<PayloadManifest>, DriverError> {
    let path = join_mount(&spec.mount, PAYLOAD_MANIFEST_PATH);
    let Ok(text) = read_mounted_text(&path) else {
        return Ok(None);
    };
    PayloadManifest::from_yaml(&text).map(Some).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            format!(
                "corrupt {PAYLOAD_MANIFEST_PATH} in the image mounted at '{}' ({e}) — the payload's self-description lies",
                spec.mount
            ),
        )
    })
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

/// The declared executable paths of one image's PROVIDES, in-image
/// absolute — the launcher tier's materialization set (the same
/// declarations `bin_dirs` flows: app entrypoints and toolkit
/// executables).
#[cfg(unix)]
fn executable_paths(provides: &Provides) -> Vec<String> {
    let paths: Vec<&str> = match provides {
        Provides::App(p) => p.entrypoints.iter().map(|e| e.path.as_str()).collect(),
        Provides::Toolkit(p) => p.executables.iter().map(|e| e.path.as_str()).collect(),
        _ => return Vec::new(),
    };
    paths.into_iter().map(str::to_string).collect()
}

/// The host-launcher tier (see the module doc): materialize each
/// declared dependency executable through the exec cache and mirror it
/// as a self-injecting wrapper under `<dl-root>/wrap-bin/`, whose PATH
/// spelling is the tier's answer. `None` when nothing is declared — no
/// wrapper dir rides PATH then.
#[cfg(unix)]
fn materialize_launchers(
    deps: &[(String, PayloadManifest)],
    shim_host: &str,
) -> Result<Option<String>, DriverError> {
    // (basename, VFS path) in triple order, first basename wins — the
    // PATH lookup's own rule.
    let mut launches: Vec<(String, String)> = Vec::new();
    for (mount, manifest) in deps {
        for path in executable_paths(&manifest.provides) {
            let Some(base) = Path::new(&path).file_name() else {
                continue;
            };
            let base = base.to_string_lossy().into_owned();
            if launches.iter().any(|(b, _)| *b == base) {
                continue;
            }
            launches.push((base, join_mount(mount, &path)));
        }
    }
    if launches.is_empty() {
        return Ok(None);
    }
    let mut ctx = context().write().unwrap();
    let root = ctx.ensure_dl_tmpdir().map_err(|e| {
        DriverError::new(
            EX_TEBAKO_IO,
            format!(
                "cannot create the exec-cache launcher dir ({})",
                crate::driver::errno_text(e)
            ),
        )
    })?;
    let wrap_dir = root.join("wrap-bin");
    std::fs::create_dir_all(&wrap_dir).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_IO,
            format!("cannot create the launcher dir '{}': {e}", wrap_dir.display()),
        )
    })?;
    for (base, vfs) in &launches {
        let host = ctx.dlmap2file(vfs).map_err(|e| {
            DriverError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "the image declares executable '{vfs}' but it cannot be materialized ({}) — the payload's self-description lies",
                    crate::driver::errno_text(e)
                ),
            )
        })?;
        let host = host.to_string_lossy().into_owned();
        force_exec_bit(Path::new(&host))?;
        let wrap = wrap_dir.join(base);
        std::fs::write(&wrap, wrapper_text(shim_host, &host)).map_err(|e| {
            DriverError::new(
                EX_TEBAKO_IO,
                format!("cannot write the launcher '{}': {e}", wrap.display()),
            )
        })?;
        chmod(&wrap, 0o755)?;
    }
    Ok(Some(wrap_dir.to_string_lossy().into_owned()))
}

/// The launcher: re-arm the platform's injection var EXPLICITLY (an
/// inherited one dies at an Apple-binary exec — SIP; a script's own
/// assignment survives), then exec the materialized binary with the
/// user's argv.
#[cfg(unix)]
fn wrapper_text(shim_host: &str, target: &str) -> String {
    let var = crate::injection::INJECT_VAR;
    format!(
        "#!/bin/sh\n{var}={}\nexport {var}\nexec {} \"$@\"\n",
        shell_quote(shim_host),
        shell_quote(target)
    )
}

/// POSIX single-quoting (a literal quote rides the `'\''` idiom).
#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// OR the exec bits into the materialized copy (dlmap2file preserves
/// the image's perms, which zip-family backends report 0644 — the
/// kernel refuses those for exec).
#[cfg(unix)]
fn force_exec_bit(path: &Path) -> Result<(), DriverError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(path)
        .map_err(|e| {
            DriverError::new(
                EX_TEBAKO_IO,
                format!("cannot stat the materialized '{}': {e}", path.display()),
            )
        })?
        .permissions()
        .mode();
    chmod(path, mode | 0o111)
}

/// Set the exact mode on a launcher-tier file.
#[cfg(unix)]
fn chmod(path: &Path, mode: u32) -> Result<(), DriverError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_IO,
            format!("cannot chmod '{}': {e}", path.display()),
        )
    })
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
        export(&[], &env, None).unwrap();
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("/usr/bin")
        );
    }

    #[test]
    fn a_delivered_shim_without_dependencies_launches_nothing() {
        // The launcher tier is armed but no dependency declares an
        // executable — no wrapper dir rides PATH.
        let env = MapEnv(RefCell::new(HashMap::from([(
            "PATH".to_string(),
            "/usr/bin".to_string(),
        )])));
        export(&[], &env, Some("/x/libtfs_preload.so")).unwrap();
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("/usr/bin")
        );
    }

    #[cfg(unix)]
    #[test]
    fn executable_paths_mirror_the_bin_dir_sources() {
        let app = PayloadManifest::from_yaml(&manifest(
            "app",
            "provides:\n  entrypoints:\n    - {name: a, path: /bin/a}\n    - {name: b, path: /sbin/b}\n  \
             platforms: universal\n  capabilities: {exec: true, read: true}",
        ))
        .unwrap();
        assert_eq!(executable_paths(&app.provides), vec!["/bin/a", "/sbin/b"]);

        let data = PayloadManifest::from_yaml(&manifest(
            "data",
            "provides:\n  mount_semantics: {suggested: /usr/share/x}\n  capabilities: {exec: false, read: true}",
        ))
        .unwrap();
        assert!(executable_paths(&data.provides).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn the_wrapper_re_arms_the_injection_var_and_execs_the_target() {
        #[cfg(target_os = "macos")]
        let want = "#!/bin/sh\nDYLD_INSERT_LIBRARIES='/dl/shim.dylib'\nexport DYLD_INSERT_LIBRARIES\nexec '/dl/opt/openjdk/bin/java' \"$@\"\n";
        #[cfg(not(target_os = "macos"))]
        let want = "#!/bin/sh\nLD_PRELOAD='/dl/shim.so'\nexport LD_PRELOAD\nexec '/dl/opt/openjdk/bin/java' \"$@\"\n";
        #[cfg(target_os = "macos")]
        let got = wrapper_text("/dl/shim.dylib", "/dl/opt/openjdk/bin/java");
        #[cfg(not(target_os = "macos"))]
        let got = wrapper_text("/dl/shim.so", "/dl/opt/openjdk/bin/java");
        assert_eq!(got, want);
    }

    #[cfg(unix)]
    #[test]
    fn shell_quote_escapes_a_literal_quote() {
        assert_eq!(shell_quote("/a/b"), "'/a/b'");
        assert_eq!(shell_quote("/a'b"), "'/a'\\''b'");
    }
}
