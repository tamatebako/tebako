//! The resource axis's enforcement (spec 08 §2.2 — bounded consumption).
//!
//! POSIX: the runtime driver installs the bounds around the interpreter
//! handoff (called from `apply_jail`, the same boot moment as the jail
//! itself) so they cover the interpreter and every descendant:
//!
//! - `cpu_seconds` → `setrlimit(RLIMIT_CPU)` (soft = hard; `SIGXCPU`),
//! - `memory` → `setrlimit(RLIMIT_AS)` (address-space growth fails),
//! - `processes` → `setrlimit(RLIMIT_NPROC)` (per-real-uid ceiling —
//!   `fork` fails `EAGAIN`),
//! - `wall_seconds` → the wall watchdog: a forked child that kills the
//!   payload's process group at the deadline — and, when the armer OWNS
//!   the group, when the root exits (a bounded run's tree dies with its
//!   root — the 2026-10-26 orphan class closed transitively, every
//!   watchdog watching its own armer).
//!
//! The wall is a TREE bound: the arming driver exports
//! `TEBAKO_RESOURCE_DEADLINE` (unix seconds; a contract variable, spec 08
//! §2.2) and every descendant driver boot clamps to the inherited
//! deadline — a descendant with a tighter composed wall re-groups its own
//! subtree and arms its own watchdog at `min(own, inherited)`; a
//! descendant without one arms nothing (the root's watchdog covers it).
//! The rlimits are kernel-inherited by construction.
//!
//! Windows: PLANNED (Job Objects — spec 08 §2.2). Until that arm lands a
//! boot with any bound declared fails closed, never silently unbounded.
//!
//! # The fork discipline (tebako#920's lesson)
//!
//! The watchdog is a post-`fork(2)` child of a process that already runs
//! backend worker threads, so between `fork` and `_exit` it calls ONLY
//! async-signal-safe libc entry points (`nanosleep`, `getppid`,
//! `clock_gettime`, `kill`, `write`, `open`, `close`, `_exit`) and reads
//! only memory formatted BEFORE the fork — no allocation, no locks, no
//! Rust std. Every message and the journal path are rendered into the
//! fixed-size [`WatchdogSpec`] on the safe side of the fork.

#![cfg_attr(unix, allow(unsafe_code))]

use tfs::policy::ResourceBounds;

use crate::driver::Env;

/// The inherited wall deadline (unix seconds), exported by the arming
/// driver (spec 08 §2.2's deadline inheritance — a contract variable, it
/// survives the tebako#737 scrub and flows to every descendant). The
/// name's single owner: the scrub's contract list references this const.
pub(crate) const DEADLINE_VAR: &str = "TEBAKO_RESOURCE_DEADLINE";

#[cfg(unix)]
mod imp {
    use std::path::Path;

    use tfs::policy::ResourceBounds;

    use super::DEADLINE_VAR;
    use crate::driver::Env;

    /// The event name of the watchdog's audit-journal line (the resource
    /// axis's violation record, beside `jail-deny`).
    const JAIL_RESOURCE_EVENT: &str = "jail-resource";

    /// The watchdog's poll slice (spec 08 §2.2: 50 ms — far under any
    /// declared wall, far above syscall cost).
    const POLL_NSEC: i64 = 50_000_000;

    /// Fixed-capacity byte slots carried across the fork (the child never
    /// allocates — see the module docs).
    const CAP: usize = 768;

    /// What the boot decided about the wall bound (spec 08 §2.2's
    /// inheritance rule), computed in safe Rust before any fork.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum WallPlan {
        /// Nothing to arm: no wall in play, or the inherited deadline
        /// covers this subtree fully (the ancestor's watchdog owns the
        /// kill — arming a second one would let a middle layer's exit
        /// nuke the whole tree).
        Skip,
        /// Arm the watchdog: `seconds` to the deadline, exporting
        /// `deadline_unix` for descendants. `regroup` when this subtree
        /// must own its process group (the tree root, or a descendant
        /// TIGHTENING the inherited deadline — its kill must not reach
        /// the rest of the tree).
        Arm {
            seconds: u64,
            deadline_unix: u64,
            regroup: bool,
        },
    }

    /// The inheritance algebra (spec 08 §2.2), pure for tests:
    ///
    /// - no own wall → nothing to arm (an inherited deadline is the
    ///   ancestor's watchdog's business — arming a second one would let
    ///   a middle layer's exit nuke the whole tree);
    /// - an own wall at-or-looser than the inherited deadline → covered
    ///   (the ancestor's watchdog fires first);
    /// - an own wall tighter than the inherited deadline, or no
    ///   inherited deadline at all → arm on the subtree's own group.
    pub(super) fn plan_wall(own: Option<u64>, inherited: Option<u64>, now_unix: u64) -> WallPlan {
        let own_deadline = own.map(|w| now_unix.saturating_add(w));
        match (own_deadline, inherited) {
            (None, _) => WallPlan::Skip,
            // The inherited deadline is at-or-earlier — it covers this
            // subtree (an already-passed inherited deadline included:
            // the ancestor's watchdog owns the imminent kill).
            (Some(own_d), Some(inh)) if own_d >= inh => WallPlan::Skip,
            (Some(own_d), _) => WallPlan::Arm {
                seconds: own_d.saturating_sub(now_unix),
                deadline_unix: own_d,
                regroup: true,
            },
        }
    }

    /// The pre-fork rendering of everything the watchdog child may need,
    /// in fixed buffers. `len == 0` marks an absent slot (no journal).
    pub(super) struct WatchdogSpec {
        /// Wall duration the arm computed (for the journal line).
        seconds: u64,
        /// Whether the armer created its process group (the root-exit
        /// reap engages only then — spec 08 §2.2's pipeline caveat: a
        /// job-control group leader may share the group with siblings,
        /// and the root-exit kill must never take them).
        owns_group: bool,
        /// The stderr line the deadline fire writes.
        pub(super) wall_line: [u8; CAP],
        pub(super) wall_line_len: usize,
        /// The journal payload after the `<ts> ` prefix, cause=wall.
        pub(super) journal_wall: [u8; CAP],
        pub(super) journal_wall_len: usize,
        /// The journal payload after the `<ts> ` prefix, cause=root-exit.
        pub(super) journal_root: [u8; CAP],
        pub(super) journal_root_len: usize,
        /// The journal path, NUL-terminated for `open(2)`.
        pub(super) journal_path: [u8; CAP],
        pub(super) journal_path_len: usize,
    }

    fn put(dst: &mut [u8; CAP], src: &[u8]) -> usize {
        let n = src.len().min(CAP);
        dst[..n].copy_from_slice(&src[..n]);
        n
    }

    /// Render the watchdog's static payloads (safe — runs before the
    /// fork). `source` is the jail's audit source label; `journal` is the
    /// resolved journal path (`tfs::journal::journal_file`'s rule), when
    /// one resolves.
    pub(super) fn format_spec(
        seconds: u64,
        owns_group: bool,
        source: &str,
        journal: Option<&Path>,
    ) -> WatchdogSpec {
        let wall_line = format!(
            "tebako-driver: resource bound exceeded (wall_seconds={seconds}) — the payload process group was killed\n"
        );
        let journal_wall = format!(
            " event={JAIL_RESOURCE_EVENT} bound=wall_seconds value={seconds} op=kill cause=wall source={source}\n"
        );
        let journal_root = format!(
            " event={JAIL_RESOURCE_EVENT} bound=wall_seconds value={seconds} op=kill cause=root-exit source={source}\n"
        );
        let mut spec = WatchdogSpec {
            seconds,
            owns_group,
            wall_line: [0; CAP],
            wall_line_len: 0,
            journal_wall: [0; CAP],
            journal_wall_len: 0,
            journal_root: [0; CAP],
            journal_root_len: 0,
            journal_path: [0; CAP],
            journal_path_len: 0,
        };
        spec.wall_line_len = put(&mut spec.wall_line, wall_line.as_bytes());
        spec.journal_wall_len = put(&mut spec.journal_wall, journal_wall.as_bytes());
        spec.journal_root_len = put(&mut spec.journal_root, journal_root.as_bytes());
        if let Some(path) = journal {
            let mut bytes = path.to_string_lossy().as_bytes().to_vec();
            bytes.push(0);
            // One slot for the NUL: a path that fills CAP is unusable.
            spec.journal_path_len = if bytes.len() <= CAP {
                put(&mut spec.journal_path, &bytes) - 1
            } else {
                0
            };
        }
        spec
    }

    fn unix_seconds() -> u64 {
        unsafe { libc::time(std::ptr::null_mut()) as u64 }
    }

    /// The inherited deadline, if any. A malformed value is a torn
    /// channel — named, never a guessed-around boot (spec 08 §2.2).
    fn inherited_deadline(env: &dyn Env) -> Result<Option<u64>, String> {
        let Some(raw) = env.var(DEADLINE_VAR).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let v: u64 = raw.parse().map_err(|_| {
            format!(
                "resources: {DEADLINE_VAR} {raw:?} is malformed — the wall-deadline inheritance channel is torn"
            )
        })?;
        Ok(Some(v))
    }

    /// Give the watchdog a group to kill (spec 08 §2.2): returns `true`
    /// when the group was created here (definitionally exclusive to the
    /// payload tree — the root-exit reap engages), `false` when the
    /// armer already led one (job-control; it may be shared with pipeline
    /// siblings, so the root-exit reap stays out). A leader cannot
    /// re-group (POSIX) and does not need to.
    fn ensure_group_lead() -> Result<bool, String> {
        unsafe {
            let pid = libc::getpid();
            if libc::getpgid(0) == pid {
                return Ok(false);
            }
            if libc::setpgid(0, 0) == 0 {
                return Ok(true);
            }
            let e = std::io::Error::last_os_error();
            Err(format!(
                "resources: wall_seconds cannot create the process group the watchdog kills (setpgid: {e}) — refusing to arm a bound that cannot hold"
            ))
        }
    }

    /// The rlimit mapping (spec 08 §2.2's table) — one row per declared
    /// key, in the grammar's key order.
    pub(super) fn rlimit_plan(bounds: &ResourceBounds) -> Vec<(libc::c_int, u64, &'static str)> {
        let mut plan = Vec::with_capacity(3);
        if let Some(v) = bounds.cpu_seconds {
            plan.push((libc::RLIMIT_CPU, v, "cpu_seconds"));
        }
        if let Some(v) = bounds.memory_bytes {
            plan.push((libc::RLIMIT_AS, v, "memory"));
        }
        if let Some(v) = bounds.processes {
            plan.push((libc::RLIMIT_NPROC, v, "processes"));
        }
        plan
    }

    fn apply_rlimits(bounds: &ResourceBounds) -> Result<(), String> {
        for (resource, value, name) in rlimit_plan(bounds) {
            let rl = libc::rlimit {
                rlim_cur: value as libc::rlim_t,
                rlim_max: value as libc::rlim_t,
            };
            if unsafe { libc::setrlimit(resource, &rl) } != 0 {
                let e = std::io::Error::last_os_error();
                return Err(format!(
                    "resources: {name}: setrlimit({value}) failed: {e}{} — the kernel refused the bound (never silently loosened, spec 08 §2.2; a composed union may not exceed the tree's inherited hard limit)",
                    refusal_note(resource, e.raw_os_error())
                ));
            }
        }
        Ok(())
    }

    /// The platform-specific diagnosis appended to a refusal (spec 08
    /// §2.2's platform-reality note): the macOS kernel refuses RLIMIT_AS
    /// below its address-space floor (2^39 bytes on arm64), so a
    /// realistic `memory` bound is not armable there at all — named,
    /// never silently clamped.
    #[cfg(target_os = "macos")]
    fn refusal_note(resource: libc::c_int, errno: Option<i32>) -> &'static str {
        if resource == libc::RLIMIT_AS && errno == Some(libc::EINVAL) {
            " (macOS refuses RLIMIT_AS below its address-space floor — 512 GiB on arm64 — so a realistic memory bound is not armable on this platform)"
        } else {
            ""
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn refusal_note(_resource: libc::c_int, _errno: Option<i32>) -> &'static str {
        ""
    }

    /// Fork the watchdog. The child runs ONLY async-signal-safe libc
    /// calls (the module docs' discipline); the parent continues the boot.
    pub(super) fn fork_watchdog(spec: &WatchdogSpec) -> Result<(), String> {
        unsafe {
            match libc::fork() {
                -1 => {
                    let e = std::io::Error::last_os_error();
                    Err(format!(
                        "resources: cannot fork the wall watchdog: {e} — refusing to arm a bound that cannot hold"
                    ))
                }
                0 => watchdog_child(spec),
                _ => Ok(()),
            }
        }
    }

    /// The watchdog loop. Never returns; raw libc only from here.
    fn watchdog_child(spec: &WatchdogSpec) -> ! {
        unsafe {
            let armer = libc::getppid();
            let mut deadline: libc::timespec = std::mem::zeroed();
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut deadline);
            deadline.tv_sec += spec.seconds as i64;
            let slice = libc::timespec {
                tv_sec: 0,
                tv_nsec: POLL_NSEC,
            };
            loop {
                libc::nanosleep(&slice, std::ptr::null_mut());
                if libc::getppid() != armer && spec.owns_group {
                    // The root is gone: a bounded run's tree dies with its
                    // root (spec 08 §2.2 — the orphan class). Journal-only
                    // housekeeping; no stderr line.
                    fire(spec, None, true);
                }
                let mut now: libc::timespec = std::mem::zeroed();
                libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now);
                if now.tv_sec > deadline.tv_sec
                    || (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)
                {
                    fire(spec, Some(&spec.wall_line[..spec.wall_line_len]), false);
                }
                // When the root is gone but the group is NOT ours (a
                // job-control leader — pipeline siblings may share it),
                // the root-exit reap stays out and the deadline fire is
                // the only kill (spec 08 §2.2's named caveat).
            }
        }
    }

    /// Write the records and kill the group. Raw libc only; never returns.
    fn fire(spec: &WatchdogSpec, stderr_line: Option<&[u8]>, root_cause: bool) -> ! {
        unsafe {
            if let Some(line) = stderr_line {
                libc::write(
                    libc::STDERR_FILENO,
                    line.as_ptr() as *const libc::c_void,
                    line.len(),
                );
            }
            let (payload, payload_len) = if root_cause {
                (&spec.journal_root, spec.journal_root_len)
            } else {
                (&spec.journal_wall, spec.journal_wall_len)
            };
            if spec.journal_path_len > 0 {
                journal_line(spec, payload, payload_len);
            }
            libc::kill(0, libc::SIGKILL);
            libc::_exit(127);
        }
    }

    /// Append one `<ts> <payload>` line to the journal: raw
    /// `open(2)`/`write(2)`/`close(2)` on the pre-rendered path (the
    /// journal's own fd discipline — O_APPEND, one write). Best effort by
    /// design: a kill is never delayed for its record.
    fn journal_line(spec: &WatchdogSpec, payload: &[u8; CAP], payload_len: usize) {
        // <ts> rendered into a small head; the payload follows — one
        // contiguous write assembled on the stack (no allocation).
        let mut buf = [0u8; CAP];
        let ts = unsafe { libc::time(std::ptr::null_mut()) as u64 };
        let head_len = utoa(ts, &mut buf);
        let n = (head_len + payload_len).min(CAP);
        buf[head_len..n].copy_from_slice(&payload[..n - head_len]);
        unsafe {
            let fd = libc::open(
                spec.journal_path.as_ptr() as *const libc::c_char,
                libc::O_WRONLY | libc::O_APPEND | libc::O_CREAT,
                0o644,
            );
            if fd < 0 {
                return;
            }
            libc::write(fd, buf.as_ptr() as *const libc::c_void, n);
            libc::close(fd);
        }
    }

    /// Decimal `v` followed by a space, into `dst`; returns the length.
    pub(super) fn utoa(mut v: u64, dst: &mut [u8]) -> usize {
        let mut tmp = [0u8; 20];
        let mut i = tmp.len();
        loop {
            i -= 1;
            tmp[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        let digits = &tmp[i..];
        dst[..digits.len()].copy_from_slice(digits);
        dst[digits.len()] = b' ';
        digits.len() + 1
    }

    pub(super) fn enforce(
        bounds: &ResourceBounds,
        source: &str,
        env: &dyn Env,
    ) -> Result<(), String> {
        if bounds.is_empty() {
            return Ok(());
        }
        // The wall arm BEFORE the rlimits: RLIMIT_NPROC would count the
        // watchdog's own fork, so the watchdog exists first.
        let inherited = inherited_deadline(env)?;
        let now = unix_seconds();
        if let WallPlan::Arm {
            seconds,
            deadline_unix,
            regroup,
        } = plan_wall(bounds.wall_seconds, inherited, now)
        {
            let owns_group = if regroup { ensure_group_lead()? } else { false };
            let journal = tfs::journal::journal_file();
            let spec = format_spec(seconds, owns_group, source, journal.as_deref());
            fork_watchdog(&spec)?;
            env.set_var(DEADLINE_VAR, &deadline_unix.to_string());
        }
        apply_rlimits(bounds)
    }
}

#[cfg(windows)]
mod imp {
    use tfs::policy::ResourceBounds;

    use crate::driver::Env;

    /// Fail closed until the Job Object arm lands (spec 08 §2.2): a
    /// declared bound is never silently unbounded.
    pub(super) fn enforce(
        bounds: &ResourceBounds,
        _source: &str,
        _env: &dyn Env,
    ) -> Result<(), String> {
        if bounds.is_empty() {
            return Ok(());
        }
        Err(
            "resources: bounds are declared but not enforceable on windows yet — Job Object enforcement is the follow-up (spec 08 §2.2); refusing to run unbounded"
                .to_string(),
        )
    }
}

/// Install the declared bounds (called from the driver's `apply_jail`,
/// after the host policy install, before the interpreter handoff).
/// `source` is the jail's audit source label for the watchdog's journal
/// line. Named errors ride the jail family (exit 73).
pub(crate) fn enforce(bounds: &ResourceBounds, source: &str, env: &dyn Env) -> Result<(), String> {
    imp::enforce(bounds, source, env)
}

#[cfg(all(test, unix))]
mod tests {
    use super::imp::*;
    use super::DEADLINE_VAR;
    use crate::driver::{Env, ProcessEnv};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tfs::policy::ResourceBounds;

    struct MapEnv(std::sync::Mutex<HashMap<String, String>>);
    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
        }
        fn names_with_prefix(&self, prefix: &str) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect()
        }
    }

    fn bounds(
        cpu: Option<u64>,
        mem: Option<u64>,
        procs: Option<u64>,
        wall: Option<u64>,
    ) -> ResourceBounds {
        ResourceBounds {
            cpu_seconds: cpu,
            memory_bytes: mem,
            processes: procs,
            wall_seconds: wall,
        }
    }

    // The fork tests fork raw children from the multithreaded test
    // process: between fork and _exit the children call ONLY
    // async-signal-safe libc entries (the module's own discipline). All
    // specs are rendered in the parent BEFORE the fork.
    static FORK_LOCK: Mutex<()> = Mutex::new(());

    fn child_status(pid: libc::pid_t) -> libc::c_int {
        let mut status: libc::c_int = 0;
        unsafe {
            libc::waitpid(pid, &mut status, 0);
        }
        status
    }

    fn signaled_by(status: libc::c_int, sig: libc::c_int) -> bool {
        libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == sig
    }

    #[test]
    fn the_rlimit_plan_maps_each_key_to_its_resource() {
        let plan = rlimit_plan(&bounds(Some(60), Some(1024), Some(8), Some(300)));
        assert_eq!(
            plan,
            vec![
                (libc::RLIMIT_CPU, 60, "cpu_seconds"),
                (libc::RLIMIT_AS, 1024, "memory"),
                (libc::RLIMIT_NPROC, 8, "processes"),
            ]
        );
        assert!(rlimit_plan(&bounds(None, None, None, Some(5))).is_empty());
        assert!(rlimit_plan(&bounds(None, None, None, None)).is_empty());
    }

    #[test]
    fn the_wall_plan_is_the_inheritance_algebra() {
        let now = 1_000_000u64;
        // No wall anywhere → nothing in play.
        assert_eq!(plan_wall(None, None, now), WallPlan::Skip);
        // Inherited only → covered by the ancestor's watchdog.
        assert_eq!(plan_wall(None, Some(now + 50), now), WallPlan::Skip);
        // Root arm.
        assert_eq!(
            plan_wall(Some(60), None, now),
            WallPlan::Arm {
                seconds: 60,
                deadline_unix: now + 60,
                regroup: true,
            }
        );
        // Own wall looser than the inherited deadline → covered.
        assert_eq!(plan_wall(Some(600), Some(now + 60), now), WallPlan::Skip);
        // Own wall tighter → arm on the subtree's own group, clamped.
        assert_eq!(
            plan_wall(Some(30), Some(now + 600), now),
            WallPlan::Arm {
                seconds: 30,
                deadline_unix: now + 30,
                regroup: true,
            }
        );
        // An inherited deadline already passed arms nothing (the
        // ancestor's watchdog owns the imminent kill).
        assert_eq!(plan_wall(Some(60), Some(now - 5), now), WallPlan::Skip);
    }

    #[test]
    fn a_malformed_inherited_deadline_is_a_named_error() {
        let env = MapEnv(Mutex::new(HashMap::from([(
            DEADLINE_VAR.to_string(),
            "soon".to_string(),
        )])));
        let e = enforce(&bounds(None, None, None, Some(60)), "test", &env).unwrap_err();
        assert!(
            e.contains("malformed") && e.contains(DEADLINE_VAR),
            "the torn channel must be named: {e}"
        );
    }

    #[test]
    fn enforce_arms_nothing_and_touches_nothing_when_unbounded() {
        let env = MapEnv(Mutex::new(HashMap::new()));
        enforce(&ResourceBounds::default(), "test", &env).unwrap();
        assert!(env.var(DEADLINE_VAR).is_none());
    }

    #[test]
    fn setrlimit_applies_and_reads_back_in_a_child() {
        let _g = FORK_LOCK.lock().unwrap();
        unsafe {
            let pid = libc::fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                // The child proves the syscall mapping against itself
                // (raw libc only from here). Each target clamps to the
                // inherited HARD limit — setrlimit may never raise it
                // (macOS caps RLIMIT_NPROC's hard ceiling by sysctl).
                // The AS target sits AT the macOS arm64 floor (2^39 —
                // the kernel refuses anything lower there, spec 08
                // §2.2's platform-reality note) and armably below every
                // Linux hard limit.
                let want: [(libc::c_int, libc::rlim_t); 3] = [
                    (libc::RLIMIT_CPU, 600),
                    (libc::RLIMIT_AS, 1 << 39),
                    (libc::RLIMIT_NPROC, 512),
                ];
                for (i, (resource, target)) in want.iter().enumerate() {
                    let mut cur = std::mem::zeroed::<libc::rlimit>();
                    if libc::getrlimit(*resource, &mut cur) != 0 {
                        libc::_exit(80 + i as i32);
                    }
                    let value = (*target).min(cur.rlim_max);
                    let rl = libc::rlimit {
                        rlim_cur: value,
                        rlim_max: value,
                    };
                    if libc::setrlimit(*resource, &rl) != 0 {
                        libc::_exit(90 + i as i32);
                    }
                    let mut back = std::mem::zeroed::<libc::rlimit>();
                    if libc::getrlimit(*resource, &mut back) != 0
                        || back.rlim_cur != value
                        || back.rlim_max != value
                    {
                        libc::_exit(95 + i as i32);
                    }
                }
                libc::_exit(42);
            }
            let status = child_status(pid);
            assert!(
                libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 42,
                "the child must apply and read back its limits (status {status:#x} — 80+i = getrlimit failed, 90+i = setrlimit refused, 95+i = readback mismatch)"
            );
        }
    }

    /// Fork a "payload" child that arms the watchdog and then behaves as
    /// directed; return its pid. The spec is rendered in the parent, so
    /// the child runs raw libc only. The payload ALWAYS takes its own
    /// process group so a watchdog fire can never reach the test runner;
    /// the spec's `owns_group` decides only whether the root-exit reap
    /// engages (its production semantics).
    fn fork_payload(spec: &WatchdogSpec, behave: unsafe fn()) -> libc::pid_t {
        unsafe {
            let pid = libc::fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                libc::setpgid(0, 0);
                if fork_watchdog(spec).is_err() {
                    libc::_exit(93);
                }
                behave();
            }
            pid
        }
    }

    unsafe fn sleep_forever() {
        let slice = libc::timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };
        loop {
            libc::nanosleep(&slice, std::ptr::null_mut());
        }
    }

    /// Leak one sleeping grandchild, then exit cleanly (the payload root
    /// is gone; the sleeper is what the watchdog's fire must answer for).
    unsafe fn leak_and_exit() {
        if libc::fork() == 0 {
            sleep_forever();
        }
        libc::_exit(0);
    }

    /// The pipe write end `leak_report_and_exit` publishes the leaked
    /// pid through (a shared-group probe needs the leaked pid to poll —
    /// no group kill may answer for it before the deadline).
    static LEAK_PIPE_W: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

    unsafe fn leak_report_and_exit() {
        let pid = libc::fork();
        if pid == 0 {
            sleep_forever();
        }
        let fd = LEAK_PIPE_W.load(std::sync::atomic::Ordering::Relaxed);
        if fd >= 0 {
            let bytes = pid.to_ne_bytes();
            libc::write(fd, bytes.as_ptr() as *const libc::c_void, bytes.len());
        }
        libc::_exit(0);
    }

    #[test]
    fn the_watchdog_kills_the_group_at_the_deadline() {
        let _g = FORK_LOCK.lock().unwrap();
        let spec = format_spec(1, true, "test", None);
        let started = std::time::Instant::now();
        let pid = fork_payload(&spec, sleep_forever);
        let status = child_status(pid);
        let elapsed = started.elapsed();
        assert!(
            signaled_by(status, libc::SIGKILL),
            "the looper must die by the watchdog's SIGKILL (status {status:#x})"
        );
        assert!(
            elapsed >= std::time::Duration::from_secs(1),
            "fired early ({elapsed:?})"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(10),
            "fired late ({elapsed:?})"
        );
    }

    #[test]
    fn the_watchdog_reaps_the_tree_when_the_root_exits() {
        let _g = FORK_LOCK.lock().unwrap();
        // A generous wall the test never reaches: the ROOT-EXIT fire is
        // what must reap the leaked child.
        let spec = format_spec(300, true, "test", None);
        let payload = fork_payload(&spec, leak_and_exit);
        let started = std::time::Instant::now();
        let status = child_status(payload);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "the root itself exits cleanly (status {status:#x})"
        );
        // The payload's group (its own pid — it re-grouped) must empty
        // promptly: the watchdog fires on the root's exit and takes the
        // leaked sleeper with the group (and itself).
        let mut emptied = false;
        for _ in 0..30 {
            let alive = unsafe { libc::kill(-payload, 0) } == 0;
            if !alive {
                emptied = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            emptied,
            "the leaked child must die with the root within 3s (waited {:?})",
            started.elapsed()
        );
    }

    #[test]
    fn the_root_exit_reap_stays_out_of_a_shared_group() {
        let _g = FORK_LOCK.lock().unwrap();
        // owns_group = false (the job-control-leader topology): the
        // root-exit reap must NOT fire (pipeline siblings may share the
        // group) — the leaked child survives past the root's exit and
        // dies at the deadline. The payload still takes its own group
        // (fork_payload) so the deadline fire never reaches the runner.
        let spec = format_spec(2, false, "test", None);
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        LEAK_PIPE_W.store(fds[1], std::sync::atomic::Ordering::Relaxed);
        let payload = fork_payload(&spec, leak_report_and_exit);
        LEAK_PIPE_W.store(-1, std::sync::atomic::Ordering::Relaxed);
        unsafe { libc::close(fds[1]) };
        let status = child_status(payload);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "the root itself exits cleanly (status {status:#x})"
        );
        // The leaked pid rides the pipe (published before the exit).
        let mut leaked: libc::pid_t = 0;
        let mut got = 0usize;
        while got < std::mem::size_of::<libc::pid_t>() {
            let n = unsafe {
                libc::read(
                    fds[0],
                    (&mut leaked as *mut libc::pid_t as *mut u8).add(got) as *mut libc::c_void,
                    std::mem::size_of::<libc::pid_t>() - got,
                )
            };
            assert!(n > 0, "the leaked pid never arrived");
            got += n as usize;
        }
        unsafe { libc::close(fds[0]) };
        assert!(leaked > 0);
        // Poll the leaked pid: it must SURVIVE the root's exit (no reap
        // — a reap fires within ~100 ms) and die at the ~2 s deadline.
        let mut died_at = None;
        for i in 0..60 {
            let alive = unsafe { libc::kill(leaked, 0) } == 0;
            if !alive {
                died_at = Some(i);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let died_at = died_at.expect("the leaked child never died");
        assert!(
            died_at >= 15,
            "the shared-group leak must SURVIVE the root's exit (died at {}00ms — the root-exit reap fired where it must not)",
            died_at
        );
        assert!(
            died_at <= 30,
            "…and must die at the deadline (died at {}00ms)",
            died_at
        );
    }

    #[test]
    fn the_spec_renders_the_named_line_and_journal_payloads() {
        let spec = format_spec(
            60,
            true,
            "manifest+user",
            Some(std::path::Path::new("/tmp/j.log")),
        );
        let line = std::str::from_utf8(&spec.wall_line[..spec.wall_line_len]).unwrap();
        assert!(line.contains("tebako-driver: resource bound exceeded (wall_seconds=60)"));
        let wall = std::str::from_utf8(&spec.journal_wall[..spec.journal_wall_len]).unwrap();
        assert_eq!(
            wall,
            " event=jail-resource bound=wall_seconds value=60 op=kill cause=wall source=manifest+user\n"
        );
        let root = std::str::from_utf8(&spec.journal_root[..spec.journal_root_len]).unwrap();
        assert!(root.contains("cause=root-exit"));
        assert_eq!(spec.journal_path_len, 10); // "/tmp/j.log"
                                               // No journal path → no slot.
        let spec = format_spec(1, false, "test", None);
        assert_eq!(spec.journal_path_len, 0);
    }

    #[test]
    fn utoa_renders_the_timestamp_prefix() {
        let mut buf = [0u8; 32];
        let n = utoa(1_700_000_005, &mut buf);
        assert_eq!(&buf[..n], b"1700000005 ");
        let n = utoa(0, &mut buf);
        assert_eq!(&buf[..n], b"0 ");
    }

    #[test]
    fn enforce_arms_and_exports_the_deadline() {
        let _g = FORK_LOCK.lock().unwrap();
        // The armed watchdog outlives the test binary's interest and
        // fires (root-exit) when it exits — point the journal at a
        // scratch path so that fire never writes the real store's
        // journal. The scratch dir goes away before the fire, so the
        // watchdog's best-effort open simply finds nothing to append to.
        let scratch =
            std::env::temp_dir().join(format!("tebako-rsrc-test-{}", unsafe { libc::getpid() }));
        std::fs::create_dir_all(&scratch).unwrap();
        std::env::set_var("TEBAKO_JAIL_JOURNAL", scratch.join("journal.log"));
        let env = MapEnv(Mutex::new(HashMap::new()));
        // A long wall so the armed watchdog outlives the whole suite
        // (a deadline fire before the suite ends would kill the runner's
        // group); it is reaped with the process — it watches us, and the
        // root-exit fire at process exit takes only its own group.
        enforce(&bounds(None, None, None, Some(3600)), "test", &env).unwrap();
        std::env::remove_var("TEBAKO_JAIL_JOURNAL");
        let _ = std::fs::remove_dir_all(&scratch);
        let exported = env
            .var(DEADLINE_VAR)
            .expect("the deadline must be exported for descendants");
        let v: u64 = exported.parse().unwrap();
        let now = unsafe { libc::time(std::ptr::null_mut()) as u64 };
        assert!(v >= now + 3500 && v <= now + 3600, "deadline {v} now {now}");
        // The ProcessEnv facade compiles against the same Env (the shipped
        // path passes it).
        let _ = ProcessEnv;
    }
}
