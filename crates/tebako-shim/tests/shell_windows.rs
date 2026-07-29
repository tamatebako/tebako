#![cfg(windows)]
//! Real-registry round-trip (TODO.v2-1/05): install prepends exactly
//! once, uninstall removes exactly our entry. A unique sentinel dir per
//! run; the drop guard uninstalls even when an assertion fails, so the
//! user PATH is left untouched either way.

use tebako_shim::shell::Change;
use tebako_shim::shell_windows;

struct Guard(std::path::PathBuf);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = shell_windows::uninstall(&self.0);
    }
}

#[test]
fn registry_roundtrip_install_remove() {
    let dir = std::env::temp_dir().join(format!("tebako-shim-selftest-{}", std::process::id()));
    let guard = Guard(dir.clone());

    assert_eq!(shell_windows::install(&dir).unwrap(), Change::Installed);
    // idempotent: the second install changes nothing
    assert_eq!(
        shell_windows::install(&dir).unwrap(),
        Change::AlreadyPresent
    );
    assert_eq!(shell_windows::uninstall(&dir).unwrap(), Change::Removed);
    assert_eq!(shell_windows::uninstall(&dir).unwrap(), Change::NotPresent);

    drop(guard);
}
