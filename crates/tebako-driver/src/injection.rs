//! Child injection (spec 22 §3, Rules E2/E3): the handoff env arms every
//! future child of the runtime with the preload shim and the mounts it
//! needs to rebuild this namespace — so an exec'd descendant re-enters
//! the same interposition instead of falling back to the host.
//!
//! Three exports, all after the mounts are established:
//!
//! 1. `TEBAKO_TFS_MOUNTS` — the mount table in the shim's grammar,
//!    composed by the context (`mounts_env` — the single composer; the
//!    ruby spawn hook's per-child value comes from the same place).
//! 2. The preload pair, when the env image's layout declares
//!    `preload_shim` (schema_minor 2): the shim is materialized from the
//!    VFS to the exec cache and the platform's injection var names the
//!    host copy — `LD_PRELOAD` on ELF, `DYLD_INSERT_LIBRARIES` on macOS
//!    (replacing the boot's self-insert value: the micro interpose-dylib
//!    is bound to THIS exe's symbols — meaningless in a child; the shim
//!    is self-contained). `TEBAKO_PRELOAD_SHIM` carries the VFS spelling
//!    for the interpreter's spawn hook (the SSOT flow: the factory
//!    stages the file and emits the layout key; the driver flows it; the
//!    hook consumes it — no second hand-written path).
//! 3. `TEBAKO_RUNTIME_DLL`, when the env image's layout declares
//!    `runtime_dll` (schema_minor 3; MSYS images only): the runtime's
//!    own PE module basename, flowed VERBATIM into the handoff env on
//!    every platform (POSIX legs never read it — one code path). The
//!    reader is the tfs PE closure walk (`tfs::context`, the PR #409
//!    stream): a bare import name matching it case-insensitively is
//!    never materialized out of a payload image (spec 22 §2.1 — the
//!    OS's basename-reuse rule binds the already-loaded copy). Absent ⇒
//!    no export (an older image — the exclusion is simply off).
//!
//! Platform notes: on macOS the value reaches EVERY child in the
//! inherited env — including Apple platform binaries, which dyld
//! TERMINATES under a foreign insertion on darwin24 (darwin23 stripped
//! the variable instead). The interpreter's spawn hook drops the
//! variable per spawn whose target is restricted (tamatebako/ruby's
//! `process_c_tebako_spawn.patch` — spec 22 §3.1's named boundary), so
//! `/bin/sh` and `/usr/bin/*` survive while non-restricted host targets
//! (a third-party JRE) keep the delivery. Windows has no preload tier
//! yet (Phase W): a declaration there is parsed and flows to
//! `TEBAKO_PRELOAD_SHIM` but no injection var is set. A
//! declared-but-absent shim is the image lying about its contents — a
//! named boot error (exit 78), never a skipped injection.

use crate::driver::{join_mount, DriverError, Env};
use crate::layout::ImageLayout;
use crate::EX_TEBAKO_LAYOUT;
use tfs::context::context;

/// The spawn hook's source for the shim's in-VFS path (spec 17 §2).
pub const PRELOAD_SHIM_VAR: &str = "TEBAKO_PRELOAD_SHIM";

/// The runtime's own PE module basename for the tfs PE closure walk's
/// exclusion (spec 22 §2.1; spec 17 §2's handoff-env row).
pub const RUNTIME_DLL_VAR: &str = "TEBAKO_RUNTIME_DLL";

/// The mounts list the shim rebuilds the namespace from (spec 17 §2).
const MOUNTS_VAR: &str = "TEBAKO_TFS_MOUNTS";

/// The platform's injection variable (ELF / macOS; none elsewhere).
/// pub(crate) for the PATH launchers (spec 22 §3.2): the wrapper
/// re-arms exactly this var for its child.
#[cfg(target_os = "macos")]
pub(crate) const INJECT_VAR: &str = "DYLD_INSERT_LIBRARIES";
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) const INJECT_VAR: &str = "LD_PRELOAD";

/// Export the child-injection env (see the module doc). Called per boot
/// after the mounts and the jail, before the interpreter handoff.
/// Returns the shim's materialized HOST path when one was delivered —
/// the §3.2 launchers embed it; `None` when the image declares no shim.
pub fn export(
    env: &dyn Env,
    declaration: Option<&ImageLayout>,
    runtime_root: &str,
) -> Result<Option<String>, DriverError> {
    if let Some(mounts) = context().read().unwrap().mounts_env() {
        env.set_var(MOUNTS_VAR, &mounts.to_string_lossy());
    }
    // The runtime's own PE module name (schema_minor 3): flowed
    // verbatim into the handoff env on EVERY platform — POSIX legs
    // never read it, so one code path serves all. The reader is the
    // tfs PE closure walk (`tfs::context`, the PR #409 stream), which
    // excludes a bare import name matching it case-insensitively (spec
    // 22 §2.1). Absent ⇒ no export (an older image — the exclusion is
    // simply off).
    if let Some(dll) = declaration.and_then(|d| d.runtime_dll.as_deref()) {
        env.set_var(RUNTIME_DLL_VAR, dll);
    }
    let Some(rel) = declaration.and_then(|d| d.preload_shim.as_deref()) else {
        return Ok(None); // no env image or an older image — nothing to inject with
    };
    let vfs = join_mount(runtime_root, rel);
    let host = context()
        .write()
        .unwrap()
        .dlmap2file(&vfs)
        .map_err(|e| {
            DriverError::new(
                EX_TEBAKO_LAYOUT,
                format!(
                    "env image declares preload_shim '{rel}' but '{vfs}' cannot be materialized ({}) — the declaration lies: rebuild the runtime with the current factory",
                    crate::driver::errno_text(e)
                ),
            )
        })?;
    // The spawn hook reads the VFS spelling (it materializes per child
    // through the same dlmap cache — one copy on disk).
    env.set_var(PRELOAD_SHIM_VAR, &vfs);
    let host = host.to_string_lossy().into_owned();
    #[cfg(unix)]
    env.set_var(INJECT_VAR, &host);
    Ok(Some(host))
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

    #[test]
    fn no_declaration_exports_no_preload_vars() {
        // Nothing mounted: the mounts list stays unset too (the context
        // serializes only file-backed mounts).
        context().write().unwrap().unmount();
        let env = MapEnv(RefCell::new(HashMap::new()));
        let delivered = export(&env, None, "/__tfs__").unwrap();
        assert!(delivered.is_none());
        let m = env.0.borrow();
        assert!(!m.contains_key(PRELOAD_SHIM_VAR));
        assert!(!m.contains_key(MOUNTS_VAR));
        assert!(!m.contains_key(RUNTIME_DLL_VAR));
    }

    /// A declaration carrying only the fields under test (the pair-check
    /// fields are layout.rs's surface, not this module's).
    fn declaration(runtime_dll: Option<&str>) -> ImageLayout {
        ImageLayout {
            schema_version: 1,
            era: 2,
            image_layout: 1,
            mount_root: "/__tfs__".to_string(),
            interpreter_api_version: "3.4".to_string(),
            mount_root_override: false,
            preload_shim: None,
            runtime_dll: runtime_dll.map(str::to_string),
        }
    }

    #[test]
    fn a_declared_runtime_dll_flows_to_the_handoff_env_on_every_platform() {
        // No cfg gate: POSIX legs never read the var, so the export is
        // one code path everywhere (spec 17 §2's row). The declared
        // spelling flows verbatim — the reader (the tfs PE closure
        // walk) lowercases for the windows loader's comparison.
        let env = MapEnv(RefCell::new(HashMap::new()));
        let layout = declaration(Some("x64-ucrt-ruby340.dll"));
        let delivered = export(&env, Some(&layout), "/__tfs__").unwrap();
        assert!(delivered.is_none(), "no preload shim declared");
        assert_eq!(
            env.0.borrow().get(RUNTIME_DLL_VAR).map(String::as_str),
            Some("x64-ucrt-ruby340.dll")
        );
    }

    #[test]
    fn an_absent_runtime_dll_exports_nothing() {
        // An older image: the closure walk's exclusion is simply off.
        let env = MapEnv(RefCell::new(HashMap::new()));
        let layout = declaration(None);
        export(&env, Some(&layout), "/__tfs__").unwrap();
        assert!(!env.0.borrow().contains_key(RUNTIME_DLL_VAR));
    }

    #[cfg(unix)]
    #[test]
    fn the_inject_var_is_the_platforms_preload_name() {
        #[cfg(target_os = "macos")]
        assert_eq!(INJECT_VAR, "DYLD_INSERT_LIBRARIES");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(INJECT_VAR, "LD_PRELOAD");
    }
}
