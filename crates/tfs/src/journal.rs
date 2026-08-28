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

/// The event name every VFS write-gate denial carries (spec 24 §5): a
/// write into a held tree the composition did not open (EROFS), or —
/// with `class=ekey` — a read of a sealed path outside every opened
/// grant (ENOKEY).
pub const VFS_DENY_EVENT: &str = "vfs-deny";

/// The event name record-mode VFS writes carry (spec 24 §6): under
/// `policy: record` a write into a held tree lands in the run's scratch
/// overlay and is journaled, never denied.
pub const VFS_WRITE_EVENT: &str = "vfs-write";

/// The event name of the boot-time overlay binding record (spec 24 §8
/// audit): which store serves which mount, and whether the binding was
/// declared or ephemeral.
pub const OVERLAY_EVENT: &str = "overlay";

/// The event name of the boot-time decrypt binding record (spec 24 §8
/// audit): which recipient reference opened which grants on a mount. Key
/// MATERIAL never touches a journal (spec 11 §11's log discipline).
pub const DECRYPT_EVENT: &str = "decrypt";

/// The event name every library-load verdict line carries (spec 22
/// §2.1 phase W2 — the windows bare-name alias rule's audit, in spec 23
/// §8's record-mode idiom).
pub const LIB_LOAD_EVENT: &str = "lib-load";

/// The event name of the press-side signing opt-out record (spec 09 §9):
/// an explicit `--no-sign` / `TEBAKO_SIGN=0` overrode a lower channel's
/// `sign` declaration — a trust downgrade must never be silent, so the
/// drop is journaled (and warned on stderr) at press-time resolution.
pub const PRESS_SIGN_OPT_OUT_EVENT: &str = "press-sign-opt-out";

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

/// Append one library-load verdict line — `<ts> event=lib-load name=<n>
/// verdict=host|alias`. The windows bare-name alias rule's record
/// (spec 22 §2.1 phase W2): emitted where the verdict is MADE (the
/// `tebako_fs_dlalias2file` decision), never at boot — a boot decides
/// nothing. Rides the same journal file under the record policy (the §8
/// discovery instrument: one record-mode run shows the author every
/// bare name a loader presented and which way it went); the `tfs needs`
/// generator skips the event (its parser knows only the jail events).
/// Same fd discipline as [`journal_deny`].
pub fn journal_lib_load(file: &std::fs::File, name: &str, verdict: &str) {
    use std::io::Write as _;
    let line = format!(
        "{} event={LIB_LOAD_EVENT} name={name} verdict={verdict}\n",
        unix_now()
    );
    let _ = (&*file).write_all(line.as_bytes());
}

/// The journal timestamp: unix seconds (0 before the epoch — a clock
/// failure never fails the audited operation).
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The shared line writer: `<ts> event=<event> path=<p> op=read|write
/// source=<s>` — a bare `write(2)` on the pre-opened file.
fn journal_event(file: &std::fs::File, event: &str, path: &Path, need: HostAccess, source: &str) {
    let op = match need {
        HostAccess::Ro => "read",
        HostAccess::Rw => "write",
    };
    journal_fields(
        file,
        &format!(
            "event={event} path={} op={} source={}",
            path.display(),
            op,
            if source.is_empty() {
                "unattributed"
            } else {
                source
            }
        ),
    );
}

/// Append one VFS write-gate denial line (spec 24 §5):
/// `<ts> event=vfs-deny op=read|write path=<p> mount=<mp>[ class=ekey]`.
/// The path is the FULL namespace path (the mount-relative form is the
/// needs generator's job); `class` is the denial's errno class when it
/// is not the plain EROFS write gate (`ekey` = ENOKEY, the sealed read).
/// Same fd discipline and best-effort rule as [`journal_deny`].
pub fn journal_vfs_deny(
    file: &std::fs::File,
    path: &Path,
    need: HostAccess,
    mount: &str,
    class: Option<&str>,
) {
    let op = match need {
        HostAccess::Ro => "read",
        HostAccess::Rw => "write",
    };
    let class = class.map(|c| format!(" class={c}")).unwrap_or_default();
    journal_fields(
        file,
        &format!(
            "event={VFS_DENY_EVENT} op={op} path={} mount={mount}{class}",
            path.display()
        ),
    );
}

/// Append one record-mode VFS write line (spec 24 §6):
/// `<ts> event=vfs-write path=<p> mount=<mp>` — the write landed in the
/// run's scratch overlay; the payload observed a writable world it never
/// owned.
pub fn journal_vfs_write(file: &std::fs::File, path: &Path, mount: &str) {
    journal_fields(
        file,
        &format!(
            "event={VFS_WRITE_EVENT} path={} mount={mount}",
            path.display()
        ),
    );
}

/// Append the boot-time overlay binding record (spec 24 §8 audit):
/// `<ts> event=overlay mount=<mp> store=<dir> source=<declared|ephemeral>`.
pub fn journal_overlay(file: &std::fs::File, mount: &str, store: &Path, source: &str) {
    journal_fields(
        file,
        &format!(
            "event={OVERLAY_EVENT} mount={mount} store={} source={source}",
            store.display()
        ),
    );
}

/// Append the boot-time decrypt binding record (spec 24 §8 audit):
/// `<ts> event=decrypt mount=<mp> recipient=<pgp:keyid> grants=<ids>` —
/// the recipient is the key REFERENCE, never material.
pub fn journal_decrypt(file: &std::fs::File, mount: &str, recipient: &str, grants: &str) {
    journal_fields(
        file,
        &format!("event={DECRYPT_EVENT} mount={mount} recipient={recipient} grants={grants}"),
    );
}

/// Append one press-side signing opt-out line (spec 09 §9): `<ts>
/// event=press-sign-opt-out by=<cli|env> overridden=<env|compose>` — the
/// channel carrying the winning opt-out and the highest lower channel
/// whose `sign` declaration it dropped. Emitted where the decision is
/// MADE (the press's settings resolution), never at run time; same
/// best-effort discipline as [`journal_deny`].
pub fn journal_press_sign_opt_out(file: &std::fs::File, by: &str, overridden: &str) {
    journal_fields(
        file,
        &format!("event={PRESS_SIGN_OPT_OUT_EVENT} by={by} overridden={overridden}"),
    );
}

/// The raw line writer: `<unix seconds> <fields>\n` — a bare `write(2)`
/// on the pre-opened file; write failures are swallowed (best-effort by
/// design: the journaled answer reaches the caller regardless).
fn journal_fields(file: &std::fs::File, fields: &str) {
    use std::io::Write as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = (&*file).write_all(format!("{now} {fields}\n").as_bytes());
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

    #[test]
    fn vfs_and_binding_line_shapes() {
        // The spec-24 vocabulary (§5/§6/§8): vfs-deny (with and without
        // the ekey class), record-mode vfs-write, and the boot-time
        // overlay/decrypt binding records. No env mutation — the file is
        // handed in directly.
        let dir = std::env::temp_dir().join(format!("tfs-journal-vfs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("journal.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        journal_vfs_deny(
            &file,
            Path::new("/app/var/cache/x"),
            HostAccess::Rw,
            "/app",
            None,
        );
        journal_vfs_deny(
            &file,
            Path::new("/data/fonts/licensed/f.ttf"),
            HostAccess::Ro,
            "/data",
            Some("ekey"),
        );
        journal_vfs_write(&file, Path::new("/app/var/cache/y"), "/app");
        journal_overlay(&file, "/app", Path::new("/tmp/ov/app"), "ephemeral");
        journal_decrypt(&file, "/data", "pgp:3c8dba971d2b4f01", "g1,g2");
        drop(file);
        let text = std::fs::read_to_string(&log).unwrap();
        let bodies: Vec<&str> = text.lines().map(|l| l.split_once(' ').unwrap().1).collect();
        assert_eq!(
            bodies,
            vec![
                "event=vfs-deny op=write path=/app/var/cache/x mount=/app",
                "event=vfs-deny op=read path=/data/fonts/licensed/f.ttf mount=/data class=ekey",
                "event=vfs-write path=/app/var/cache/y mount=/app",
                "event=overlay mount=/app store=/tmp/ov/app source=ephemeral",
                "event=decrypt mount=/data recipient=pgp:3c8dba971d2b4f01 grants=g1,g2",
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn press_sign_opt_out_line_shape() {
        // spec 09 §9: the loud opt-out's audit record — who opted out and
        // whose declaration was dropped. No env mutation — the file is
        // handed in directly.
        let dir =
            std::env::temp_dir().join(format!("tfs-journal-optout-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("journal.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        journal_press_sign_opt_out(&file, "cli", "env");
        journal_press_sign_opt_out(&file, "env", "compose");
        drop(file);
        let text = std::fs::read_to_string(&log).unwrap();
        let bodies: Vec<&str> = text.lines().map(|l| l.split_once(' ').unwrap().1).collect();
        assert_eq!(
            bodies,
            vec![
                "event=press-sign-opt-out by=cli overridden=env",
                "event=press-sign-opt-out by=env overridden=compose",
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn journal_lib_load_line_shape() {
        // The bare-name alias rule's verdict line (spec 22 §2.1 phase
        // W2, spec 23 §8's idiom): name + verdict, no path/op fields.
        // (Private dir: the deny/allow tests above share this process —
        // a shared journal path would interleave.)
        let dir =
            std::env::temp_dir().join(format!("tfs-journal-libload-test-{}", std::process::id()));
        let log = dir.join("journal.log");
        std::env::set_var("TEBAKO_JAIL_JOURNAL", &log);
        let file = open_journal().expect("journal opens");
        std::env::remove_var("TEBAKO_JAIL_JOURNAL");
        journal_lib_load(&file, "libfoo-3.dll", "alias");
        journal_lib_load(&file, "user32", "host");
        drop(file);
        let text = std::fs::read_to_string(&log).unwrap();
        let mut lines = text.lines();
        let line = lines.next().unwrap();
        let (ts, rest) = line.split_once(' ').unwrap();
        assert!(
            ts.bytes().all(|b| b.is_ascii_digit()) && !ts.is_empty(),
            "{line}"
        );
        assert_eq!(rest, "event=lib-load name=libfoo-3.dll verdict=alias");
        assert_eq!(
            lines.next().unwrap().split_once(' ').unwrap().1,
            "event=lib-load name=user32 verdict=host"
        );
        assert!(lines.next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
