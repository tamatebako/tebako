//! The tebako audit journal for jail violations (spec 08 §2: "Violations
//! are logged to the tebako audit journal with path + syscall class").
//!
//! One line per denied host-passthrough decision — and, under the record
//! policy (spec 23 §8), one line per ALLOWED decision — appended to the
//! tebako journal — the same file and line shape the bootstrap uses
//! (`$TEBAKO_HOME/journal.log`, `<unix seconds> <fields>`):
//!
//! ```text
//! <ts> event=jail-deny  path=<path> op=read|write source=<policy source>
//! <ts> event=jail-allow path=<path> op=read|write source=<policy source>
//! ```
//!
//! The location resolves as `$TEBAKO_JAIL_JOURNAL` (an explicit override —
//! also the test seam), else `$TEBAKO_HOME/journal.log`, else the platform
//! default tebako home (mirroring the bootstrap's cache-root rule).
//!
//! **The fd discipline (deadlock freedom):** the journal file is resolved
//! and opened ONCE, at policy-install time, by the caller BEFORE the
//! context guard is taken ([`open_journal`]); a denial is then a bare
//! `write(2)` on the cached file ([`journal_deny`]). No path operation
//! ever runs under the context lock — path syscalls are exactly what the
//! preload shim interposes, so journaled IO inside the guard would
//! re-enter the engine and self-deadlock (the roadmap-39 linux leg caught
//! precisely this in the statically-linked test binary). `write(2)` is not
//! part of the interposed surface, so the denial path can never re-enter.
//! Journaling is best-effort BY DESIGN: a violation is reported to the
//! caller as EPERM/EROFS regardless, and a journal that cannot be opened
//! never fails the operation it audits.
//!
//! Pure safe Rust over std::fs.

use std::path::{Path, PathBuf};

use crate::policy::HostAccess;

/// The event name every jail-denial line carries.
pub const JAIL_DENY_EVENT: &str = "jail-deny";

/// The event name every record-mode allow line carries (spec 23 §8).
pub const JAIL_ALLOW_EVENT: &str = "jail-allow";

/// Resolve and open the journal file for append (creating the tebako home
/// if needed). Call it at POLICY-INSTALL time, BEFORE the context guard is
/// taken — never from inside a `host_check` (see the module docs). `None`
/// when no home resolves or the open fails (journaling then simply stays
/// silent — the jail's answers are unaffected).
pub fn open_journal() -> Option<std::fs::File> {
    let journal = journal_path(|k| std::env::var(k).ok())?;
    if let Some(dir) = journal.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .ok()
}

/// Append one jail-denial line to the already-open journal `file`. A bare
/// `write(2)` per event — never a path operation (see the module docs);
/// best-effort: write failures are swallowed.
pub fn journal_deny(file: &std::fs::File, path: &Path, need: HostAccess, source: &str) {
    journal_event(file, JAIL_DENY_EVENT, path, need, source);
}

/// Append one record-mode allow line (spec 23 §8: the "perm all and
/// monitor" journal records every host access the record policy lets
/// through). Same fd discipline and line shape as [`journal_deny`].
pub fn journal_allow(file: &std::fs::File, path: &Path, need: HostAccess, source: &str) {
    journal_event(file, JAIL_ALLOW_EVENT, path, need, source);
}

/// The shared line writer: `<ts> event=<event> path=<p> op=read|write
/// source=<s>` — a bare `write(2)` on the pre-opened file.
fn journal_event(file: &std::fs::File, event: &str, path: &Path, need: HostAccess, source: &str) {
    use std::io::Write as _;
    let op = match need {
        HostAccess::Ro => "read",
        HostAccess::Rw => "write",
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!(
        "{now} event={event} path={} op={} source={}\n",
        path.display(),
        op,
        if source.is_empty() {
            "unattributed"
        } else {
            source
        }
    );
    let _ = (&*file).write_all(line.as_bytes());
}

/// The journal file under a tebako home (the bootstrap's convention).
pub fn journal_file_of(home: &Path) -> PathBuf {
    home.join("journal.log")
}

/// The resolved tebako home for this process (the cache-root rule shared
/// with the journal path). The needs generator excludes it (spec 23 §8 —
/// a run never declares a need on its own store).
pub fn tebako_home_dir() -> Option<PathBuf> {
    tebako_home(|k| std::env::var(k).ok())
}

/// Resolution of the journal path: the explicit override, then
/// `$TEBAKO_HOME`, then the platform default home. `lookup` abstracts the
/// environment so tests never mutate it.
fn journal_path(lookup: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(explicit) = lookup("TEBAKO_JAIL_JOURNAL").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(explicit));
    }
    tebako_home(lookup).map(|h| journal_file_of(&h))
}

/// The tebako home: `$TEBAKO_HOME` > the platform default (the bootstrap's
/// cache-root rule: `%LOCALAPPDATA%\tebako` / `%USERPROFILE%\.tebako` on
/// Windows, `~/.tebako` elsewhere).
fn tebako_home(lookup: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(home) = lookup("TEBAKO_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(home));
    }
    #[cfg(windows)]
    {
        if let Some(home) = lookup("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(home).join("tebako"));
        }
        if let Some(home) = lookup("USERPROFILE").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(home).join(".tebako"));
        }
        None
    }
    #[cfg(not(windows))]
    {
        lookup("HOME")
            .filter(|v| !v.is_empty())
            .map(|home| PathBuf::from(home).join(".tebako"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn lookup_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn journal_path_prefers_the_explicit_override() {
        let p = journal_path(lookup_of(&[
            ("TEBAKO_JAIL_JOURNAL", "/tmp/explicit.log"),
            ("TEBAKO_HOME", "/home/t"),
        ]));
        assert_eq!(p, Some(PathBuf::from("/tmp/explicit.log")));
    }

    #[test]
    fn journal_path_uses_tebako_home_then_the_default() {
        let p = journal_path(lookup_of(&[("TEBAKO_HOME", "/home/t")]));
        assert_eq!(p, Some(PathBuf::from("/home/t/journal.log")));
        #[cfg(not(windows))]
        {
            let p = journal_path(lookup_of(&[("HOME", "/home/u")]));
            assert_eq!(p, Some(PathBuf::from("/home/u/.tebako/journal.log")));
            assert_eq!(journal_path(lookup_of(&[])), None);
        }
    }

    #[test]
    fn journal_line_shape() {
        let dir = std::env::temp_dir().join(format!("tfs-journal-test-{}", std::process::id()));
        let log = dir.join("journal.log");
        std::env::set_var("TEBAKO_JAIL_JOURNAL", &log);
        let file = open_journal().expect("journal opens");
        std::env::remove_var("TEBAKO_JAIL_JOURNAL");
        journal_deny(&file, Path::new("/etc/hosts"), HostAccess::Ro, "manifest");
        journal_deny(&file, Path::new("/x"), HostAccess::Rw, "user");
        drop(file);
        let text = std::fs::read_to_string(&log).unwrap();
        let mut lines = text.lines();
        let line = lines.next().unwrap();
        let (ts, rest) = line.split_once(' ').unwrap();
        assert!(
            ts.bytes().all(|b| b.is_ascii_digit()) && !ts.is_empty(),
            "{line}"
        );
        assert_eq!(
            rest,
            "event=jail-deny path=/etc/hosts op=read source=manifest"
        );
        assert_eq!(
            lines.next().unwrap().split_once(' ').unwrap().1,
            "event=jail-deny path=/x op=write source=user"
        );
        assert!(lines.next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn journal_allow_line_shape() {
        // The record mode's audit line (spec 23 §8): same shape as a
        // denial, `event=jail-allow` — the `tfs needs` generator consumes
        // both events from one journal. (Private dir: the deny test above
        // shares this process — a shared journal path would interleave.)
        let dir =
            std::env::temp_dir().join(format!("tfs-journal-allow-test-{}", std::process::id()));
        let log = dir.join("journal.log");
        std::env::set_var("TEBAKO_JAIL_JOURNAL", &log);
        let file = open_journal().expect("journal opens");
        std::env::remove_var("TEBAKO_JAIL_JOURNAL");
        journal_allow(
            &file,
            Path::new("/home/u/.ssh/config"),
            HostAccess::Ro,
            "record",
        );
        journal_allow(&file, Path::new("/tmp/x"), HostAccess::Rw, "record");
        drop(file);
        let text = std::fs::read_to_string(&log).unwrap();
        let mut lines = text.lines();
        let line = lines.next().unwrap();
        let (ts, rest) = line.split_once(' ').unwrap();
        assert!(
            ts.bytes().all(|b| b.is_ascii_digit()) && !ts.is_empty(),
            "{line}"
        );
        assert_eq!(
            rest,
            "event=jail-allow path=/home/u/.ssh/config op=read source=record"
        );
        assert_eq!(
            lines.next().unwrap().split_once(' ').unwrap().1,
            "event=jail-allow path=/tmp/x op=write source=record"
        );
        assert!(lines.next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
