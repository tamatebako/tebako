//! The store layout contract (spec 18 C13, scenarios S41/S42):
//! `~/.tebako` carries a `layout-version` stamp this module owns —
//! written on creation, checked on first access per process.
//!
//! - stamped **newer** than spoken → the upgrade refusal (S41);
//! - **older** (including a pre-versioning store, which has no stamp) →
//!   the named migration: the stamp is written and the caller announces
//!   it — never a silent mixed layout (S42);
//! - a brand-new store is born at the current version (nothing to say).
//!
//! One owner, every consumer flows: tebako-shim and tebako-cli call
//! [`check_once`] at their store entry points (the size-capped
//! tebako-bootstrap cannot link this crate and mirrors the semantics —
//! [`STORE_LAYOUT_VERSION`] is the canonical value, pinned identical by
//! both sides' tests).

use std::fmt;
use std::path::{Path, PathBuf};

/// The store layout version this tebako writes and reads (spec 18 C13).
/// Bump when the `~/.tebako` layout changes meaningfully — the stamp
/// file's whole grammar is a single decimal number, one line.
pub const STORE_LAYOUT_VERSION: u32 = 1;

/// The stamp file's name within the store root.
pub const LAYOUT_VERSION_FILE: &str = "layout-version";

/// The outcome of a layout check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutCheck {
    /// The store did not exist: created and stamped (nothing migrated).
    Created,
    /// The store existed without a (current) stamp: migrated — the
    /// caller announces the named migration.
    Migrated,
    /// Already at the spoken version.
    Current,
}

/// The store-layout refusal classes (named, both sides spelled out).
#[derive(Debug)]
pub enum StoreLayoutError {
    /// S41: the store was stamped by a newer tebako.
    Newer {
        home: PathBuf,
        found: u32,
        spoken: u32,
    },
    /// The stamp exists but does not read as one decimal number.
    Corrupt { home: PathBuf, content: String },
    /// I/O on the read or the stamp write.
    Io {
        home: PathBuf,
        op: &'static str,
        reason: String,
    },
}

impl fmt::Display for StoreLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreLayoutError::Newer {
                home,
                found,
                spoken,
            } => write!(
                f,
                "the tebako store at {} was created by a newer tebako (layout {found}); this build speaks layout {spoken} — upgrade tebako",
                home.display()
            ),
            StoreLayoutError::Corrupt { home, content } => write!(
                f,
                "the tebako store at {} carries an unreadable layout-version ({content:?}) — remove the file and rerun, or clear the store",
                home.display()
            ),
            StoreLayoutError::Io { home, op, reason } => {
                write!(f, "{reason} ({op} {})", home.display())
            }
        }
    }
}

impl std::error::Error for StoreLayoutError {}

fn io(home: &Path, op: &'static str, e: std::io::Error) -> StoreLayoutError {
    StoreLayoutError::Io {
        home: home.to_path_buf(),
        op,
        reason: e.to_string(),
    }
}

fn write_stamp(home: &Path) -> Result<(), StoreLayoutError> {
    std::fs::write(
        home.join(LAYOUT_VERSION_FILE),
        format!("{STORE_LAYOUT_VERSION}\n"),
    )
    .map_err(|e| io(home, "writing", e))
}

/// Write-on-create + check, unmemoized (tests drive this directly;
/// processes use [`check_once`]).
pub fn ensure_layout(home: &Path) -> Result<LayoutCheck, StoreLayoutError> {
    match std::fs::read_to_string(home.join(LAYOUT_VERSION_FILE)) {
        Ok(text) => {
            let trimmed = text.trim();
            let found: u32 = trimmed.parse().map_err(|_| StoreLayoutError::Corrupt {
                home: home.to_path_buf(),
                content: trimmed.chars().take(40).collect(),
            })?;
            if found > STORE_LAYOUT_VERSION {
                return Err(StoreLayoutError::Newer {
                    home: home.to_path_buf(),
                    found,
                    spoken: STORE_LAYOUT_VERSION,
                });
            }
            if found < STORE_LAYOUT_VERSION {
                // The older → current migration (S42): today the stamp is
                // the whole change — written, announced by the caller,
                // never silent.
                write_stamp(home)?;
                return Ok(LayoutCheck::Migrated);
            }
            Ok(LayoutCheck::Current)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if home.exists() {
                // A pre-versioning store: the named migration to the
                // current layout (the stamp is the migration today).
                write_stamp(home)?;
                Ok(LayoutCheck::Migrated)
            } else {
                std::fs::create_dir_all(home).map_err(|e| io(home, "creating", e))?;
                write_stamp(home)?;
                Ok(LayoutCheck::Created)
            }
        }
        Err(e) => Err(io(home, "reading", e)),
    }
}

/// The process-wide first-access check (C13: once per process). Memoized
/// per home path — later calls in the same process report
/// [`LayoutCheck::Current`] (the post-check state).
pub fn check_once(home: &Path) -> Result<LayoutCheck, StoreLayoutError> {
    static CHECKED: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<PathBuf>>> =
        std::sync::OnceLock::new();
    let checked = CHECKED.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeSet::new()));
    let key = home.to_path_buf();
    let already = checked
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains(&key);
    if already {
        return Ok(LayoutCheck::Current);
    }
    let outcome = ensure_layout(home)?;
    checked
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(key);
    Ok(outcome)
}

/// The named migration line (S42) every consumer prints the same way
/// (stderr; the owner rule for the message text).
pub fn migration_message(home: &Path) -> String {
    format!(
        "migrated the tebako store at {} to layout {STORE_LAYOUT_VERSION} (stamped layout-version; the store predates layout versioning — spec 18 C13)",
        home.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tebako-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_new_store_is_created_and_stamped() {
        let home = dir("new");
        assert_eq!(ensure_layout(&home).unwrap(), LayoutCheck::Created);
        assert_eq!(
            std::fs::read_to_string(home.join(LAYOUT_VERSION_FILE)).unwrap(),
            format!("{STORE_LAYOUT_VERSION}\n")
        );
        // second access: current
        assert_eq!(ensure_layout(&home).unwrap(), LayoutCheck::Current);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pre_versioning_store_is_the_named_migration() {
        let home = dir("legacy");
        std::fs::create_dir_all(home.join("runtimes")).unwrap();
        assert_eq!(ensure_layout(&home).unwrap(), LayoutCheck::Migrated);
        assert_eq!(
            std::fs::read_to_string(home.join(LAYOUT_VERSION_FILE)).unwrap(),
            format!("{STORE_LAYOUT_VERSION}\n")
        );
        // the migration message names the store and the version
        let msg = migration_message(&home);
        assert!(msg.contains("migrated"), "{msg}");
        assert!(msg.contains("layout 1"), "{msg}");
        assert_eq!(ensure_layout(&home).unwrap(), LayoutCheck::Current);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_newer_stamp_is_the_upgrade_refusal() {
        let home = dir("newer");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(LAYOUT_VERSION_FILE), "99\n").unwrap();
        let err = ensure_layout(&home).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("layout 99"), "{msg}");
        assert!(msg.contains("speaks layout 1"), "{msg}");
        assert!(msg.contains("upgrade tebako"), "{msg}");
        // the stamp is untouched (no silent downgrade either)
        assert_eq!(
            std::fs::read_to_string(home.join(LAYOUT_VERSION_FILE)).unwrap(),
            "99\n"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_corrupt_stamp_is_named_not_guessed() {
        let home = dir("corrupt");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(LAYOUT_VERSION_FILE), "banana\n").unwrap();
        let err = ensure_layout(&home).unwrap_err();
        assert!(matches!(err, StoreLayoutError::Corrupt { .. }));
        assert!(err.to_string().contains("banana"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn check_once_memoizes_per_home() {
        let home = dir("once");
        std::fs::create_dir_all(&home).unwrap();
        assert_eq!(check_once(&home).unwrap(), LayoutCheck::Migrated);
        // the process already checked this home: reported current without
        // re-migration (the stamp is on disk either way)
        assert_eq!(check_once(&home).unwrap(), LayoutCheck::Current);
        let _ = std::fs::remove_dir_all(&home);
    }
}
