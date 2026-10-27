//! The sampler's Windows mechanics — one of the crate's two FFI boundary
//! modules (all `unsafe` lives here and in `sys::posix`).
//!
//! Spawn is std (`Command` gets argv quoting right for free). Wait is a
//! handle we `OpenProcess` ourselves (std's `Child` does not expose its
//! handle): `WaitForSingleObject` on a 50 ms tick against an `Instant`
//! deadline.
//!
//! **The sample describes the child's TREE, not the child.** Windows has
//! no `exec`, so every v2 arm starts a launcher (`tebako`/`tebako-shim`)
//! that spawns the runtime exe; measuring the launcher alone reports the
//! 16 ms / 6 MiB of a shim in front of a 55 s compile (tebako#927). The
//! job object the timeout kill already needs is the accountable unit:
//! `QueryInformationJobObject(JobObjectBasicAndIoAccountingInformation)`
//! sums `TotalUserTime`/`TotalKernelTime` over every process the job has
//! ever held, and `PeakJobMemoryUsed` is the tree's peak. Only when no
//! job exists — or its accounting is unreadable — does the sampler fall
//! back to the direct child's `GetProcessTimes` +
//! `K32GetProcessMemoryInfo`, LOUDLY on stderr (invariant 9).
//!
//! The timeout kill is the same JOB OBJECT armed with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: on expiry we close it, which
//! takes the child's whole tree (metanorma spawns sub-tools). On a normal
//! exit the job is disarmed (LimitFlags=0) BEFORE its handle is closed —
//! closing an armed job would kill the (already exited) tree. CI runners
//! may nest us in a job that forbids assignment (GitHub windows-2019
//! does): then we run without a job and the timeout kill degrades to
//! `TerminateProcess` on the direct child. Either way a timeout records
//! exit 137 (the shell convention, matching the POSIX SIGKILL path).

use std::fs::OpenOptions;
use std::process::{Command, Stdio};
use std::time::Instant;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAndIoAccountingInformation,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
    JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_INFORMATION,
    PROCESS_SET_QUOTA, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, PROCESS_VM_READ,
};

use crate::error::BenchError;
use crate::sampler::{ChildSpec, Sample};

/// Poll granularity of the WaitForSingleObject loop (see posix.rs).
const POLL_INTERVAL_MS: u32 = 50;

/// The rights our metrics/timeout handle needs: query times + memory,
/// wait for exit, terminate, and (for the job) set quota.
const CHILD_RIGHTS: u32 = PROCESS_QUERY_INFORMATION
    | PROCESS_VM_READ
    | PROCESS_TERMINATE
    | PROCESS_SET_QUOTA
    | PROCESS_SYNCHRONIZE;

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
        .stderr(Stdio::from(log_err));

    // The timeout job is created before spawn so a later failure to arm it
    // cannot leave a spawned child unbounded.
    let job = create_armed_job()?;

    // Wall time spans spawn→reap (see posix.rs).
    let start = Instant::now();
    let deadline = start + spec.timeout;

    let mut child = cmd.spawn().map_err(|e| {
        close_job(job, false);
        BenchError::operational(format!(
            "sampler: cannot spawn {:?} in {}: {e}",
            spec.argv[0],
            spec.cwd.display()
        ))
    })?;
    let pid = child.id();

    // SAFETY: pid is our own freshly spawned child, so the access rights
    // are grantable; null-checked right after.
    let h: HANDLE = unsafe { OpenProcess(CHILD_RIGHTS, 0, pid) };
    if h.is_null() {
        let err = std::io::Error::last_os_error();
        close_job(job, false);
        return Err(BenchError::operational(format!(
            "sampler: OpenProcess({pid}) failed: {err}"
        )));
    }

    // Join the timeout job. Fails on CI runners that nest us in a job
    // that forbids assignment — degrade to direct-child TerminateProcess,
    // LOUDLY (never a silent fallback, invariant 9).
    // SAFETY: both handles are valid and ours.
    let job = if unsafe { AssignProcessToJobObject(job, h) } == 0 {
        let err = std::io::Error::last_os_error();
        eprintln!(
            "tebako-bench: warning: AssignProcessToJobObject({pid}) failed: {err}; \
             the timeout kill is direct-child only for this run"
        );
        close_job(job, false);
        std::ptr::null_mut()
    } else {
        job
    };

    let result = poll_child(h, deadline, job);
    let timed_out = matches!(result, PollOutcome::TimedOut { .. });
    // The job close IS the tree kill on timeout; otherwise disarm first
    // (closing an armed job would kill the tree). The sample was taken
    // above, while the job was still open — its counters die with the
    // handle.
    close_job(job, timed_out);

    // Reap via std (our OpenProcess handle is separate). On timeout the
    // exit code is forced to 137 per the shell convention, matching the
    // POSIX SIGKILL path.
    let wait = child.wait();
    let wall_s = start.elapsed().as_secs_f64();

    let sample = match result {
        PollOutcome::Exited {
            cpu_user_s,
            cpu_sys_s,
            peak_rss_bytes,
        } => match wait {
            Ok(status) => Ok(Sample {
                wall_s,
                cpu_user_s,
                cpu_sys_s,
                peak_rss_bytes,
                exit: status.code().unwrap_or(-1),
                timed_out: false,
            }),
            Err(e) => Err(BenchError::operational(format!(
                "sampler: child wait() failed: {e}"
            ))),
        },
        PollOutcome::TimedOut {
            cpu_user_s,
            cpu_sys_s,
            peak_rss_bytes,
        } => {
            // The job close (or the TerminateProcess fallback in
            // poll_child) already killed the child; drain the wait so no
            // zombie handle lingers, ignore its code.
            let _ = wait;
            Ok(Sample {
                wall_s,
                cpu_user_s,
                cpu_sys_s,
                peak_rss_bytes,
                exit: 137,
                timed_out: true,
            })
        }
        PollOutcome::Failed(e) => {
            let _ = wait;
            Err(e)
        }
    };
    close_handle_quiet(h);
    sample
}

enum PollOutcome {
    Exited {
        cpu_user_s: f64,
        cpu_sys_s: f64,
        peak_rss_bytes: u64,
    },
    TimedOut {
        cpu_user_s: f64,
        cpu_sys_s: f64,
        peak_rss_bytes: u64,
    },
    Failed(BenchError),
}

/// WaitForSingleObject polling against the deadline; on expiry the caller
/// kills via the job close — here we only terminate directly when there
/// is no job. Both resource branches sample BEFORE the job is closed:
/// its counters die with the handle.
fn poll_child(h: HANDLE, deadline: Instant, job: HANDLE) -> PollOutcome {
    let no_job = job.is_null();
    loop {
        // SAFETY: h is our valid process handle.
        let rc = unsafe { WaitForSingleObject(h, POLL_INTERVAL_MS) };
        if rc == WAIT_OBJECT_0 {
            return match tree_metrics(h, job) {
                Ok((cpu_user_s, cpu_sys_s, peak_rss_bytes)) => PollOutcome::Exited {
                    cpu_user_s,
                    cpu_sys_s,
                    peak_rss_bytes,
                },
                Err(e) => PollOutcome::Failed(e),
            };
        }
        if rc == WAIT_FAILED {
            return PollOutcome::Failed(BenchError::operational(format!(
                "sampler: WaitForSingleObject failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        debug_assert_eq!(rc, WAIT_TIMEOUT);
        if Instant::now() >= deadline {
            let metrics = tree_metrics(h, job);
            if no_job {
                // SAFETY: h is our valid process handle; failing that, the
                // sample is a timeout either way.
                unsafe {
                    TerminateProcess(h, 137);
                }
            }
            return match metrics {
                Ok((cpu_user_s, cpu_sys_s, peak_rss_bytes)) => PollOutcome::TimedOut {
                    cpu_user_s,
                    cpu_sys_s,
                    peak_rss_bytes,
                },
                Err(e) => PollOutcome::Failed(e),
            };
        }
    }
}

/// user/kernel CPU seconds + peak RSS bytes for the MEASURED TREE: the
/// job's totals when a job is open (every process it has held, terminated
/// ones included), the direct child's own counters otherwise.
fn tree_metrics(h: HANDLE, job: HANDLE) -> Result<(f64, f64, u64), BenchError> {
    if !job.is_null() {
        match query_job_metrics(job) {
            Ok(totals) => return Ok(totals),
            Err(e) => eprintln!(
                "tebako-bench: warning: the job's accounting is unreadable ({e}); \
                 this sample describes the direct child only"
            ),
        }
    }
    query_metrics(h)
}

/// user/kernel CPU seconds + peak working-set bytes for the whole job —
/// the accountable unit on a platform with no `exec` (tebako#927).
fn query_job_metrics(job: HANDLE) -> Result<(f64, f64, u64), BenchError> {
    let mut acct: JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: job is our valid job handle; `acct` is a valid out-param of
    // the passed size for this information class, filled before any read.
    if unsafe {
        QueryInformationJobObject(
            job,
            JobObjectBasicAndIoAccountingInformation,
            &mut acct as *mut JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION as *mut _,
            std::mem::size_of::<JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION>() as u32,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(BenchError::operational(format!(
            "sampler: QueryInformationJobObject(basic accounting) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: as above, for the extended-limit class (`PeakJobMemoryUsed`).
    if unsafe {
        QueryInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *mut _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(BenchError::operational(format!(
            "sampler: QueryInformationJobObject(peaks) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok((
        large_integer_to_s(acct.BasicInfo.TotalUserTime),
        large_integer_to_s(acct.BasicInfo.TotalKernelTime),
        limits.PeakJobMemoryUsed as u64,
    ))
}

/// A job's `LARGE_INTEGER` times are 100-ns intervals; convert to seconds.
fn large_integer_to_s(ticks: i64) -> f64 {
    ticks as f64 / 10_000_000.0
}

/// user/kernel CPU seconds + peak RSS bytes from the (still open)
/// process handle. Queryable after the child has exited.
fn query_metrics(h: HANDLE) -> Result<(f64, f64, u64), BenchError> {
    let mut creation: FILETIME = unsafe { std::mem::zeroed() };
    let mut exit: FILETIME = unsafe { std::mem::zeroed() };
    let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
    let mut user: FILETIME = unsafe { std::mem::zeroed() };
    // SAFETY: h is our valid process handle; the FILETIMEs are valid
    // out-params filled before any read.
    if unsafe { GetProcessTimes(h, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(BenchError::operational(format!(
            "sampler: GetProcessTimes failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    // SAFETY: h is our valid process handle; `counters` is a valid
    // out-param of the passed size, filled before any read.
    if unsafe {
        K32GetProcessMemoryInfo(
            h,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    } == 0
    {
        return Err(BenchError::operational(format!(
            "sampler: K32GetProcessMemoryInfo failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok((
        filetime_to_s(&user),
        filetime_to_s(&kernel),
        counters.PeakWorkingSetSize as u64,
    ))
}

/// A FILETIME is 100-ns intervals; convert to seconds.
fn filetime_to_s(ft: &FILETIME) -> f64 {
    let ticks = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    ticks as f64 / 10_000_000.0
}

/// A job object armed with KILL_ON_JOB_CLOSE. Creation failure is an
/// operational error (an unbounded child is worse than a named failure).
fn create_armed_job() -> Result<HANDLE, BenchError> {
    // SAFETY: null attributes/name create an unnamed job with default
    // security; null-checked right after.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(BenchError::operational(format!(
            "sampler: CreateJobObjectW failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    set_job_kill_on_close(job, true).inspect_err(|_| {
        close_handle_quiet(job);
    })?;
    Ok(job)
}

fn set_job_kill_on_close(job: HANDLE, armed: bool) -> Result<(), BenchError> {
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    info.BasicLimitInformation.LimitFlags = if armed {
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    } else {
        0
    };
    // SAFETY: job is our valid job handle; `info` points at a valid
    // struct of the passed size for the ExtendedLimitInformation class.
    let ok: BOOL = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        return Err(BenchError::operational(format!(
            "sampler: SetInformationJobObject(KILL_ON_JOB_CLOSE={armed}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

/// Close the job, disarming first unless the close is meant to kill.
fn close_job(job: HANDLE, kill: bool) {
    if job.is_null() {
        return;
    }
    if !kill {
        // A failure to disarm must not leak a kill: if disarm fails we
        // still close, and the (already exited) child is unaffected.
        let _ = set_job_kill_on_close(job, false);
    }
    close_handle_quiet(job);
}

fn close_handle_quiet(h: HANDLE) {
    // SAFETY: h is a handle we opened; CloseHandle failure leaves nothing
    // to do here.
    unsafe {
        CloseHandle(h);
    }
}

/// Total physical RAM in bytes (`runner.ram_bytes` in the result
/// document).
pub(crate) fn ram_total_bytes() -> Result<u64, BenchError> {
    // SAFETY: `status` is a valid out-param; dwLength must be set first.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(BenchError::operational(format!(
            "sampler: GlobalMemoryStatusEx failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(status.ullTotalPhys)
}
