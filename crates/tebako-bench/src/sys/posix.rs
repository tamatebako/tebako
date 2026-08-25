//! The sampler's POSIX mechanics — one of the crate's two FFI boundary
//! modules (all `unsafe` lives here and in `sys::windows`).
//!
//! **Why `wait4`, not `RUSAGE_CHILDREN` (spec 27 §4, amended).** The spec
//! originally read "rusage delta around each run". That is wrong for a
//! benchmark: `getrusage(RUSAGE_CHILDREN).ru_maxrss` is a *running
//! maximum* over every child the process has ever reaped, so after one
//! big child the (after − before) RSS delta is 0 forever — runs 2..N
//! would report garbage RSS. `wait4(pid, …)` instead returns the reaped
//! child's OWN rusage (utime/stime exact, maxrss the child's own peak),
//! so every run is attributed correctly no matter what ran before it. We
//! poll `wait4(WNOHANG)` on a ~2 ms tick against an `Instant` deadline;
//! on expiry the child's process group (the child is made a group leader
//! via `CommandExt::process_group(0)`, safe std) gets `SIGKILL`, then a
//! blocking `wait4` reaps it for the rusage. The std `Child` is dropped,
//! never `wait()`ed — wait4 already reaped it.
//!
//! Unit note: `ru_maxrss` is BYTES on macOS and KIBIBYTES on Linux/musl —
//! normalized to bytes at record time (`maxrss_to_bytes`).

use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::BenchError;
use crate::sampler::{ChildSpec, Sample};

/// Poll granularity of the WNOHANG loop. 2 ms keeps wall-time overshoot
/// far below any metanorma workload's runtime while adding negligible
/// scheduler noise.
const POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Run the child to completion (or timeout) and return its sample.
pub(crate) fn run_child(spec: &ChildSpec) -> Result<Sample, BenchError> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&spec.log_path)
        .map_err(|e| {
            BenchError::operational(format!(
                "sampler: cannot open log {}: {e}",
                spec.log_path.display()
            ))
        })?;
    let log_err = log.try_clone().map_err(|e| {
        BenchError::operational(format!(
            "sampler: cannot clone log handle {}: {e}",
            spec.log_path.display()
        ))
    })?;

    let mut cmd = Command::new(&spec.argv[0]);
    cmd.args(&spec.argv[1..])
        .current_dir(&spec.cwd)
        .envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        // Own process group: the timeout kill must reach the child's whole
        // tree (metanorma shells out to sub-tools), not just the direct
        // child. Group id == child pid.
        .process_group(0);

    // Wall time spans spawn→reap: the exec cost is part of what the user
    // pays. The deadline is anchored at the same instant.
    let start = Instant::now();
    let deadline = start + spec.timeout;

    let child = cmd.spawn().map_err(|e| {
        BenchError::operational(format!(
            "sampler: cannot spawn {:?} in {}: {e}",
            spec.argv[0],
            spec.cwd.display()
        ))
    })?;
    let pid = child.id() as libc::pid_t;

    let mut status: libc::c_int = 0;
    // SAFETY: `usage` is plain-old-data filled by wait4 before any read.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };

    let (exit, timed_out) = loop {
        // SAFETY: `status`/`usage` are valid out-params; `pid` is our own
        // spawned, not-yet-reaped child.
        let rc = unsafe { libc::wait4(pid, &mut status, libc::WNOHANG, &mut usage) };
        if rc == pid {
            break (decode_exit(status), false);
        }
        if rc == 0 {
            // Still running.
            if Instant::now() >= deadline {
                // SAFETY: pid is the child's group leader id; the group is
                // ours (created by this spawn). ESRCH (already exited) is
                // harmless — the blocking wait4 below reaps either way.
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
                reap_blocking(pid, &mut status, &mut usage)?;
                break (decode_exit(status), true);
            }
            std::thread::sleep(POLL_INTERVAL);
            continue;
        }
        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::EINTR) => continue,
            _ => {
                return Err(BenchError::operational(format!(
                    "sampler: wait4({pid}) failed: {err}"
                )))
            }
        }
    };
    let wall_s = start.elapsed().as_secs_f64();

    // wait4 reaped the child; std's Child must never be wait()ed now (its
    // Drop does not reap — safe to drop).
    drop(child);

    Ok(Sample {
        wall_s,
        cpu_user_s: timeval_to_s(&usage.ru_utime),
        cpu_sys_s: timeval_to_s(&usage.ru_stime),
        peak_rss_bytes: maxrss_to_bytes(usage.ru_maxrss),
        exit,
        timed_out,
    })
}

/// Blocking reap after the timeout SIGKILL; loops on EINTR.
fn reap_blocking(
    pid: libc::pid_t,
    status: &mut libc::c_int,
    usage: &mut libc::rusage,
) -> Result<(), BenchError> {
    loop {
        // SAFETY: same as the poll loop; options 0 = block until the child
        // exits (SIGKILL guarantees it does).
        let rc = unsafe { libc::wait4(pid, status, 0, usage) };
        if rc == pid {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::EINTR) => continue,
            _ => {
                return Err(BenchError::operational(format!(
                    "sampler: wait4({pid}) after SIGKILL failed: {err}"
                )))
            }
        }
    }
}

/// The shell convention: exit status, or 128+signal when signal-killed
/// (a timeout SIGKILL therefore records 137).
fn decode_exit(status: libc::c_int) -> i32 {
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        // Stopped/continued — wait4 without WUNTRACED/WCONTINUED never
        // reports those.
        -1
    }
}

fn timeval_to_s(tv: &libc::timeval) -> f64 {
    tv.tv_sec as f64 + (tv.tv_usec as f64) / 1_000_000.0
}

/// `ru_maxrss` is bytes on macOS, KiB everywhere else we run
/// (linux-gnu/musl). Normalized to BYTES at record time.
fn maxrss_to_bytes(ru_maxrss: libc::c_long) -> u64 {
    let v = u64::try_from(ru_maxrss).unwrap_or(0);
    if cfg!(target_os = "macos") {
        v
    } else {
        v.saturating_mul(1024)
    }
}

/// Total physical RAM in bytes (`runner.ram_bytes` in the result
/// document).
pub(crate) fn ram_total_bytes() -> Result<u64, BenchError> {
    // SAFETY: sysconf on these two names is always safe; errors surface
    // as -1.
    let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if pages <= 0 || page_size <= 0 {
        return Err(BenchError::operational(format!(
            "sampler: sysconf(_SC_PHYS_PAGES/_SC_PAGESIZE) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok((pages as u64).saturating_mul(page_size as u64))
}
