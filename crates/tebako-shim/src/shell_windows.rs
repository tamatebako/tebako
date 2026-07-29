//! Windows shell integration (TODO.v2-1/05): no rc files exist here — the
//! shim dir goes onto the USER PATH in the registry
//! (`HKCU\Environment`, value `Path`, REG_EXPAND_SZ), followed by a
//! `WM_SETTINGCHANGE` broadcast so NEW consoles see it without re-login.
//!
//! The same contract as the unix managed block (shell.rs):
//!
//! - install is idempotent (present → no-op);
//! - uninstall removes EXACTLY our entry — every other entry and their
//!   order are preserved;
//! - user scope only (HKCU): no admin, no machine scope (HKLM), ever;
//! - no `setx` shell-out (it truncates PATH at 1024 chars — and a
//!   shell-out besides; the registry API is one call away).
//!
//! The string algorithm ([`path_prepend`]/[`path_remove`]) is
//! platform-free and host-tested; the Win32 FFI is quarantined in this
//! file behind `#[cfg(windows)]` (the crate's only unsafe outside the
//! shim's zero-unsafe norm — mirroring the bootstrap's platform.rs
//! quarantine).

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use crate::shell::Change;
#[cfg(windows)]
use crate::{ShimError, EX_TEBAKO_IO};

/// Prepend `dir` to a registry-form PATH string (`;`-separated).
/// Idempotent: an entry equal to `dir` (case-insensitive — Windows path
/// semantics) leaves the string byte-identical. Every other entry and
/// their order are preserved.
pub fn path_prepend(existing: &str, dir: &str) -> String {
    if existing.split(';').any(|e| e.eq_ignore_ascii_case(dir)) {
        return existing.to_string();
    }
    if existing.is_empty() {
        return dir.to_string();
    }
    format!("{dir};{existing}")
}

/// Remove exactly the `dir` entries from the PATH string. Non-matching
/// entries keep their order and bytes; an absent entry is a no-op.
pub fn path_remove(existing: &str, dir: &str) -> String {
    existing
        .split(';')
        .filter(|e| !e.eq_ignore_ascii_case(dir))
        .collect::<Vec<_>>()
        .join(";")
}

// ---------------------------------------------------------------------
// Win32 FFI (cfg(windows) only)
// ---------------------------------------------------------------------

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Read the user PATH (HKCU\Environment:Path). A missing key or value is
/// an EMPTY path, not an error (fresh profiles have neither).
#[cfg(windows)]
fn read_user_path() -> Result<String, ShimError> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ,
    };
    unsafe {
        let mut key = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide("Environment").as_ptr(),
            0,
            KEY_READ,
            &mut key,
        ) != 0
        {
            return Ok(String::new());
        }
        let name = wide("Path");
        let mut value_type = 0u32;
        let mut byte_len = 0u32;
        let rc = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            std::ptr::null_mut(),
            &mut byte_len,
        );
        if rc != 0 || byte_len == 0 {
            RegCloseKey(key);
            return Ok(String::new());
        }
        let mut buf = vec![0u16; byte_len as usize / 2 + 1];
        let rc = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            buf.as_mut_ptr().cast(),
            &mut byte_len,
        );
        RegCloseKey(key);
        if rc != 0 {
            return Err(ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot read HKCU\\Environment:Path (RegQueryValueExW error {rc})"),
            ));
        }
        // REG_SZ/REG_EXPAND_SZ is NUL-terminated; cut at the first NUL.
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..end]))
    }
}

/// Write the user PATH (creating HKCU\Environment when absent).
#[cfg(windows)]
fn write_user_path(value: &str) -> Result<(), ShimError> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE,
    };
    unsafe {
        let mut key = std::ptr::null_mut();
        let mut disposition = 0u32;
        let rc = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Environment").as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            &mut disposition,
        );
        if rc != 0 {
            return Err(ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot open HKCU\\Environment for writing (RegCreateKeyExW error {rc})"),
            ));
        }
        let data = wide(value);
        let rc = RegSetValueExW(
            key,
            wide("Path").as_ptr(),
            0,
            REG_EXPAND_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        );
        RegCloseKey(key);
        if rc != 0 {
            return Err(ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot write HKCU\\Environment:Path (RegSetValueExW error {rc})"),
            ));
        }
        Ok(())
    }
}

/// Tell the shell and top-level windows that the environment changed.
/// Best-effort by design: the broadcast does not mutate RUNNING consoles
/// (the CLI says so either way), and a hung listener must never block
/// the install (SMTO_ABORTIFHUNG, 5 s).
#[cfg(windows)]
fn broadcast_env_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    unsafe {
        let mut result = 0usize;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            wide("Environment").as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

/// `tebako-shim install-shell` on Windows: prepend the shim dir to the
/// user PATH in the registry and broadcast the change.
#[cfg(windows)]
pub fn install(dir: &Path) -> Result<Change, ShimError> {
    let dir = dir.to_string_lossy().into_owned();
    let existing = read_user_path()?;
    let merged = path_prepend(&existing, &dir);
    if merged == existing {
        return Ok(Change::AlreadyPresent);
    }
    write_user_path(&merged)?;
    broadcast_env_change();
    Ok(Change::Installed)
}

/// `tebako-shim uninstall-shell` on Windows: remove exactly our entry.
#[cfg(windows)]
pub fn uninstall(dir: &Path) -> Result<Change, ShimError> {
    let dir = dir.to_string_lossy().into_owned();
    let existing = read_user_path()?;
    let trimmed = path_remove(&existing, &dir);
    if trimmed == existing {
        return Ok(Change::NotPresent);
    }
    write_user_path(&trimmed)?;
    broadcast_env_change();
    Ok(Change::Removed)
}

// ---------------------------------------------------------------------
// tests (the pure algorithm runs everywhere)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepend_inserts_once_at_the_front() {
        assert_eq!(path_prepend("", r"C:\a\shims"), r"C:\a\shims");
        assert_eq!(
            path_prepend(r"C:\Windows;C:\bin", r"C:\a\shims"),
            r"C:\a\shims;C:\Windows;C:\bin"
        );
        // idempotent, byte-identical on the second call
        let once = path_prepend(r"C:\Windows", r"C:\a\shims");
        assert_eq!(path_prepend(&once, r"C:\a\shims"), once);
    }

    #[test]
    fn prepend_matches_case_insensitively() {
        // Windows path semantics: differing only by case IS the same dir.
        let existing = r"C:\A\SHIMS;C:\Windows";
        assert_eq!(path_prepend(existing, r"C:\a\shims"), existing);
    }

    #[test]
    fn remove_takes_exactly_our_entry() {
        assert_eq!(
            path_remove(r"C:\a\shims;C:\Windows;C:\bin", r"C:\a\shims"),
            r"C:\Windows;C:\bin"
        );
        assert_eq!(
            path_remove(r"C:\Windows;C:\a\shims;C:\bin", r"C:\a\shims"),
            r"C:\Windows;C:\bin"
        );
        // absent → no-op, byte-identical
        let existing = r"C:\Windows;C:\bin";
        assert_eq!(path_remove(existing, r"C:\a\shims"), existing);
        // a PREFIX of our entry is not our entry
        assert_eq!(path_remove(r"C:\a\shims2", r"C:\a\shims"), r"C:\a\shims2");
    }

    #[test]
    fn remove_matches_case_insensitively_and_empties_cleanly() {
        assert_eq!(path_remove(r"C:\A\SHIMS", r"C:\a\shims"), "");
        assert_eq!(path_remove(r"C:\a\shims;", r"C:\a\shims"), "");
    }
}
