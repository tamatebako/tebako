//! Child injection (spec 22 §3, Rules E2/E3): the handoff env arms every
//! future child of the runtime with the preload shim and the mounts it
//! needs to rebuild this namespace — so an exec'd descendant re-enters
//! the same interposition instead of falling back to the host.
//!
//! Two exports, both after the mounts are established:
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
//!
//! Platform notes: on macOS the value only reaches non-Apple children
//! (SIP strips it for platform binaries — spec 22 §3.1's named
//! boundary). Windows has no preload tier yet (Phase W): a declaration
//! there is parsed and flows to `TEBAKO_PRELOAD_SHIM` but no injection
//! var is set. A declared-but-absent shim is the image lying about its
//! contents — a named boot error (exit 78), never a skipped injection.

use crate::driver::{join_mount, DriverError, Env};
use crate::layout::ImageLayout;
use crate::EX_TEBAKO_LAYOUT;
use tfs::context::context;

/// The spawn hook's source for the shim's in-VFS path (spec 17 §2).
pub const PRELOAD_SHIM_VAR: &str = "TEBAKO_PRELOAD_SHIM";

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
