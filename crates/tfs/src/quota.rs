//! The per-mount materialization ceiling's enforcement engine
//! (tebako#1102 — the resource-bounding arc's mount arm; spec 05 §4's
//! store discipline extended to the exec cache, TODO.sota2/03).
//!
//! The grammar lives in `tpkg::mount_quota` (the single owner); this
//! module is the armed process state and the in-flight accounting every
//! materialize seam enforces through:
//!
//! - the runtime driver resolves the ceiling from
//!   `~/.tebako/config.yaml` (`mounts.quota`) at boot and calls
//!   [`arm`] — the class-R resources tier, the windows tree tier, and
//!   (through [`QuotaTracker::begin`]) the persistent dlcache tier then
//!   write against it;
//! - a bare consumer (tfs-cli, a preload-shim child with no driver
//!   boot) is never armed and never enforced — the pre-quota behavior,
//!   unchanged bit for bit. That is the named boundary of the
//!   contract: the ceiling is a driver-armed policy.
//!
//! **Attribution** (the ceiling's scope): one mount's materialized
//! footprint is the sum of its persistent per-image keyed namespaces
//! under the exec-cache root — `resources/<image-key>/` (class R),
//! `trees/<image-key>/` + `trees/<image-key>-s<slot>/` (the windows
//! tier), `tebako-dlcache/<image-key>/` (the macOS persistent tier) —
//! plus the in-flight staging bytes of the write being checked. The
//! per-process `tebako-dl-<hex>` leaf (process-lifetime, reaped at
//! exit, shared across the boot's mounts) and the run's ephemeral COW
//! scratch overlay are out of scope; `resources/store-trust/` is the
//! operator's own CA material (spec 22 §4 Rule R4), unattributed.
//!
//! **Enforcement is in-flight, with atomic rename doing the atomicity.**
//! A pre-flight estimate cannot be authoritative here: the only size
//! known before streaming is backend-reported metadata — the same
//! metadata the write-once protocol distrusts enough to re-hash the
//! staged copy — and the running total is racy across concurrent boots
//! by construction (the class-R race is documented benign). So the
//! tracker measures the installed total once at write start, accounts
//! every staged chunk BEFORE it lands, and on the crossing refuses:
//! the staging artifact is abandoned (never renamed into place), the
//! journal gains `event=quota-exceeded` with the mount's identity and
//! the byte counts, and the caller fails named. Nothing partial ever
//! reaches the namespace — the tmp+rename discipline holds exactly as
//! under any other mid-write failure.
//!
//! **The fd discipline** is the journal's own (tfs::journal): the
//! journal file is opened ONCE at [`arm`] time — boot, pre-guard — and
//! a refusal is a bare `write(2)` on the cached file, so a refusal
//! raised under the context guard (the dlcache seam) never runs a path
//! operation. Journaling stays best-effort: the named error reaches the
//! caller regardless.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// The journal event every quota refusal records.
pub const QUOTA_EXCEEDED_EVENT: &str = "quota-exceeded";

/// The armed state: the ceiling, the exec-cache root it measures under,
/// and the pre-opened journal (best-effort — `None` stays silent).
struct Armed {
    ceiling: u64,
    base: PathBuf,
    journal: Option<Arc<std::fs::File>>,
}

static ARMED: RwLock<Option<Armed>> = RwLock::new(None);

/// Arm the ceiling for this process (the runtime driver's boot, after
/// the exec-cache export): [`tpkg::mount_quota::MountQuota::Limited`]
/// installs the ceiling against `base` (the boot's exec-cache root) and
/// pre-opens the journal; `Unlimited` (the config's `off`) disarms —
/// enforcement disengages entirely. Re-arming replaces the previous
/// state (in-process re-boots, tests).
pub fn arm(quota: tpkg::mount_quota::MountQuota, base: PathBuf) {
    let journal = match quota.ceiling() {
        Some(_) => crate::journal::open_journal().map(Arc::new),
        None => None,
    };
    arm_with_journal(quota, base, journal);
}

/// The arm with an explicit journal handle: the audit sink a consumer
/// supplies directly (the driver's tests inject a scratch journal;
/// production arms through [`arm`]'s journal resolution). `None`
/// journals nowhere — enforcement is unaffected (best-effort by
/// design).
pub fn arm_with_journal(
    quota: tpkg::mount_quota::MountQuota,
    base: PathBuf,
    journal: Option<Arc<std::fs::File>>,
) {
    let state = quota.ceiling().map(|ceiling| Armed {
        ceiling,
        base,
        journal,
    });
    *ARMED.write().unwrap() = state;
}

/// Drop the armed state (tests; a boot never disarms — a fresh arm
/// replaces it).
pub fn disarm() {
    *ARMED.write().unwrap() = None;
}

/// The armed ceiling, when one is in effect (introspection for the
/// seams' debug lines).
pub fn armed_ceiling() -> Option<u64> {
    ARMED.read().unwrap().as_ref().map(|a| a.ceiling)
}

/// One mount's installed footprint: the sum of its persistent keyed
/// namespaces under `base` (see the module doc's attribution). Missing
/// namespaces are zero; a namespace that exists but cannot be walked is
/// a store inconsistency the caller cannot enforce against — the walk
/// fails named (never a silently zeroed measurement that would let a
/// corrupt cache write past the ceiling).
fn attributed_usage(base: &Path, image_key: &str) -> Result<u64, String> {
    let mut total = 0u64;
    for dir in [
        base.join("resources").join(image_key),
        base.join("tebako-dlcache").join(image_key),
    ] {
        total = total
            .checked_add(dir_usage(&dir)?)
            .ok_or_else(|| "the mount's materialized footprint overflows u64".to_string())?;
    }
    // The windows tier keys `trees/<key>` and, for a slot-mounted
    // package region, `trees/<key>-s<slot>` — one scan covers both
    // spellings plus their digest records. Staging siblings
    // (`.<key>.part-<pid>`) are another boot's in-flight bytes:
    // excluded — every boot enforces its own write against the
    // installed total (the documented race caveat).
    let trees = base.join("trees");
    match std::fs::read_dir(&trees) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|e| {
                    format!("cannot enumerate the trees dir '{}': {e}", trees.display())
                })?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let keyed = name.as_ref() == image_key
                    || name
                        .strip_prefix(image_key)
                        .is_some_and(|rest| rest.starts_with("-s") || rest == ".tfs-digest")
                    || name
                        .strip_prefix(&format!("{image_key}-s"))
                        .is_some_and(|rest| {
                            rest.split('.').next().is_some_and(|slot| {
                                !slot.is_empty() && slot.bytes().all(|b| b.is_ascii_digit())
                            })
                        });
                if !keyed {
                    continue;
                }
                let path = entry.path();
                let meta = std::fs::symlink_metadata(&path)
                    .map_err(|e| format!("cannot stat '{}': {e}", path.display()))?;
                total = total
                    .checked_add(if meta.is_dir() {
                        dir_usage(&path)?
                    } else {
                        meta.len()
                    })
                    .ok_or_else(|| {
                        "the mount's materialized footprint overflows u64".to_string()
                    })?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(format!(
                "cannot enumerate the trees dir '{}': {e}",
                trees.display()
            ))
        }
    }
    Ok(total)
}

/// One directory tree's regular-file byte total (staging dotfiles
/// excluded — the in-flight bytes rule; digest records counted — they
/// are installed bookkeeping). Symlinks are never followed (the tiers
/// install regular files; a foreign link is skipped, not chased).
fn dir_usage(dir: &Path) -> Result<u64, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(format!("cannot read the dir '{}': {e}", dir.display())),
    };
    let mut total = 0u64;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read the dir '{}': {e}", dir.display()))?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat '{}': {e}", path.display()))?;
        if meta.is_dir() {
            total = total
                .checked_add(dir_usage(&path)?)
                .ok_or_else(|| "the mount's materialized footprint overflows u64".to_string())?;
        } else if meta.file_type().is_file() {
            total = total
                .checked_add(meta.len())
                .ok_or_else(|| "the mount's materialized footprint overflows u64".to_string())?;
        }
    }
    Ok(total)
}

/// The refusal: what crossed, by how much, against which ceiling. The
/// Display is the named error's text; the seams map it to their own
/// error channel (the driver's exit-74 DriverError, the dlcache tier's
/// errno).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaExceeded {
    /// The mount's exec-cache segregation key (spec 22 §6's Rule R1).
    pub image_key: String,
    /// The mount's human identity (its mount point).
    pub mount: String,
    /// The installed footprint measured before this write.
    pub installed: u64,
    /// The bytes this write was about to add past the ceiling.
    pub delta: u64,
    /// The armed ceiling.
    pub ceiling: u64,
}

impl std::fmt::Display for QuotaExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mount quota exceeded: the exec-cache materialization of mount '{}' (image key {}) would cross the configured per-mount ceiling (installed {} bytes + {} incoming > ceiling {}) — raise `mounts.quota` in ~/.tebako/config.yaml or set it to `off`",
            self.mount, self.image_key, self.installed, self.delta, self.ceiling,
        )
    }
}

impl std::error::Error for QuotaExceeded {}

/// The errno the dlcache seam answers on a refusal: EDQUOT where the
/// platform names it (the honest errno — "disk quota exceeded"),
/// ENOSPC where it does not (windows carries no EDQUOT).
#[cfg(unix)]
pub fn exceeded_errno() -> i32 {
    libc::EDQUOT
}

/// The windows spelling of [`exceeded_errno`].
#[cfg(not(unix))]
pub fn exceeded_errno() -> i32 {
    libc::ENOSPC
}

/// The in-flight accountant for one write (see the module doc). Created
/// by [`QuotaTracker::begin`] against the armed state; `None` there
/// means unenforced (unarmed or `off`) and the seam writes unchecked —
/// the pre-quota behavior.
#[derive(Debug)]
pub struct QuotaTracker {
    image_key: String,
    mount: String,
    installed: u64,
    staged: u64,
    ceiling: u64,
    journal: Option<Arc<std::fs::File>>,
    exceeded: bool,
}

impl QuotaTracker {
    /// Begin accounting one write into `image_key`'s namespaces
    /// (`mount` is the journal/error identity). `None` when the process
    /// is unenforced. A measurement failure (a keyed namespace that
    /// cannot be walked) fails closed — the write is refused named
    /// rather than unmeasured.
    pub fn begin(image_key: &str, mount: &str) -> Result<Option<QuotaTracker>, String> {
        let armed = ARMED.read().unwrap();
        let Some(armed) = armed.as_ref() else {
            return Ok(None);
        };
        let installed = attributed_usage(&armed.base, image_key)?;
        Ok(Some(QuotaTracker {
            image_key: image_key.to_string(),
            mount: mount.to_string(),
            installed,
            staged: 0,
            ceiling: armed.ceiling,
            journal: armed.journal.clone(),
            exceeded: false,
        }))
    }

    /// Account `bytes` of staged content BEFORE they land. On the
    /// crossing: the journal records `event=quota-exceeded` (once —
    /// the caller aborts the write on the first refusal), the tracker
    /// latches [`Self::exceeded`], and the named refusal returns.
    pub fn account(&mut self, bytes: u64) -> Result<(), QuotaExceeded> {
        let staged = self.staged.saturating_add(bytes);
        let total = self.installed.saturating_add(staged);
        if total <= self.ceiling {
            self.staged = staged;
            return Ok(());
        }
        let exceeded = QuotaExceeded {
            image_key: self.image_key.clone(),
            mount: self.mount.clone(),
            installed: self.installed,
            delta: bytes,
            ceiling: self.ceiling,
        };
        if !self.exceeded {
            self.exceeded = true;
            journal_exceeded(self.journal.as_deref(), &exceeded);
        }
        Err(exceeded)
    }

    /// True after the first refusal — the seam distinguishes a quota
    /// refusal from an ordinary IO failure (the staging cleanup
    /// decision) without parsing the error text.
    pub fn exceeded(&self) -> bool {
        self.exceeded
    }

    /// The projected total (installed + staged) — the seam's debug
    /// line, never a second authority.
    pub fn projected(&self) -> u64 {
        self.installed.saturating_add(self.staged)
    }
}

/// The journal line: `<ts> event=quota-exceeded key=<image-key>
/// mount=<mount> installed=<bytes> delta=<bytes> ceiling=<bytes>` — a
/// bare `write(2)` on the pre-opened file, best-effort by design (the
/// named refusal reaches the caller regardless).
fn journal_exceeded(file: Option<&std::fs::File>, exceeded: &QuotaExceeded) {
    let Some(file) = file else { return };
    use std::io::Write as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = (&*file).write_all(
        format!(
            "{now} event={QUOTA_EXCEEDED_EVENT} key={} mount={} installed={} delta={} ceiling={}\n",
            exceeded.image_key,
            exceeded.mount,
            exceeded.installed,
            exceeded.delta,
            exceeded.ceiling,
        )
        .as_bytes(),
    );
}

/// One lock for every test mutating the process-global armed state —
/// this module's own tests, the context.rs dlcache quota test
/// (`crate::quota::ARMED_TEST_LOCK`), and the driver's seams tests.
#[cfg(test)]
pub(crate) static ARMED_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tfs-quota-test-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    /// Open a journal at an explicit path for the arm-with-journal
    /// tests (never the process env — the journal module's own env
    /// tests race it).
    fn open_test_journal(path: &Path) -> Option<Arc<std::fs::File>> {
        Some(Arc::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap(),
        ))
    }

    #[test]
    fn unarmed_is_unenforced() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        disarm();
        assert_eq!(armed_ceiling(), None);
        assert!(QuotaTracker::begin("deadbeef", "/app").unwrap().is_none());
    }

    #[test]
    fn unlimited_disarms() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        arm_with_journal(tpkg::mount_quota::MountQuota::Unlimited, temp("unl"), None);
        assert_eq!(armed_ceiling(), None);
        assert!(QuotaTracker::begin("deadbeef", "/app").unwrap().is_none());
        disarm();
    }

    #[test]
    fn the_tracker_accounts_and_crosses_named() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        let base = temp("cross");
        let journal = base.join("journal.log");
        arm_with_journal(
            tpkg::mount_quota::MountQuota::Limited(1024),
            base.clone(),
            open_test_journal(&journal),
        );
        assert_eq!(armed_ceiling(), Some(1024));
        let mut tracker = QuotaTracker::begin("deadbeef", "/app").unwrap().unwrap();
        tracker.account(600).unwrap();
        assert_eq!(tracker.projected(), 600);
        let err = tracker.account(500).unwrap_err();
        assert_eq!(err.image_key, "deadbeef");
        assert_eq!(err.mount, "/app");
        assert_eq!(err.installed, 0);
        assert_eq!(err.delta, 500);
        assert_eq!(err.ceiling, 1024);
        assert!(tracker.exceeded());
        // The journal recorded the refusal once.
        let text = std::fs::read_to_string(&journal).unwrap();
        let line = text.lines().next().unwrap();
        let (_, rest) = line.split_once(' ').unwrap();
        assert_eq!(
            rest,
            "event=quota-exceeded key=deadbeef mount=/app installed=0 delta=500 ceiling=1024"
        );
        assert_eq!(text.lines().count(), 1, "{text}");
        disarm();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_installed_total_covers_every_keyed_namespace() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        let base = temp("usage");
        write(&base.join("resources/deadbeef/lib/cert.pem"), 100);
        write(&base.join("resources/deadbeef/lib/cert.pem.tfs-digest"), 65);
        write(&base.join("tebako-dlcache/deadbeef/lib/libx.dylib"), 200);
        write(&base.join("trees/deadbeef/lib/core.txt"), 300);
        write(&base.join("trees/deadbeef.tfs-digest"), 64);
        write(&base.join("trees/deadbeef-s0/bin/app"), 400);
        write(&base.join("trees/deadbeef-s0.tfs-digest"), 64);
        // Not attributable: another image's namespaces, the operator
        // store-trust bundle, a staging sibling, a foreign tree.
        write(&base.join("resources/cafebabe/lib/cert.pem"), 1000);
        write(&base.join("resources/store-trust/cert-merged-x.pem"), 500);
        write(&base.join("trees/.deadbeef.part-1234/tmp"), 700);
        write(&base.join("trees/other/lib/y"), 900);
        // A per-process leaf never joins the total.
        write(&base.join("tebako-dl-1234abcd/libz.so"), 800);
        assert_eq!(
            attributed_usage(&base, "deadbeef").unwrap(),
            100 + 65 + 200 + 300 + 64 + 400 + 64
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_refusal_leaves_the_staging_excluded_from_the_next_measure() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        let base = temp("remeasure");
        write(&base.join("resources/deadbeef/a.pem"), 900);
        arm_with_journal(
            tpkg::mount_quota::MountQuota::Limited(1024),
            base.clone(),
            None,
        );
        // A 900-byte installed total leaves 124 bytes of headroom.
        let mut tracker = QuotaTracker::begin("deadbeef", "/app").unwrap().unwrap();
        tracker.account(124).unwrap();
        assert!(tracker.account(1).is_err());
        assert_eq!(tracker.projected(), 1024);
        // Re-beginning measures the same installed total (the refused
        // bytes never installed).
        let tracker = QuotaTracker::begin("deadbeef", "/app").unwrap().unwrap();
        assert_eq!(tracker.installed, 900);
        disarm();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_unwalkable_namespace_fails_closed() {
        let _g = ARMED_TEST_LOCK.lock().unwrap();
        let base = temp("unwalkable");
        // A FILE where the resources namespace's parent must be a dir:
        // the walk cannot answer, so the write is refused named.
        write(&base.join("resources"), 1);
        arm_with_journal(
            tpkg::mount_quota::MountQuota::Limited(1024),
            base.clone(),
            None,
        );
        let err = QuotaTracker::begin("deadbeef", "/app").unwrap_err();
        assert!(err.contains("resources"), "{err}");
        disarm();
        let _ = std::fs::remove_dir_all(&base);
    }
}
