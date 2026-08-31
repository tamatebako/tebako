//! The C entry surface (spec 17 §1): the interpreter's `main` calls the
//! boot first and continues with the rewritten argv on success. This
//! module is the crate's only FFI boundary — `unsafe` lives here and
//! nowhere else (spec 14 §3).
//!
//! Two entries share one boot core:
//!
//! - [`tebako_driver_boot`] — the generic entry (any language): the
//!   caller names the runtime root.
//! - [`tebako_main`] — the ruby-runtime entry (the patched ruby's
//!   `main.c` hook): the root is the ruby memfs point, plus the
//!   build-time `miniruby` pass-through and the `TEBAKO_CONTRACT_VERSION`
//!   environment export (roadmap 45). The three compat getters
//!   ([`tebako_mount_point`], [`tebako_original_pwd`],
//!   [`tebako_is_running_miniruby`]) are exactly the surface the ruby
//!   io-routing patches and the toolchain stub reference.

#![allow(unsafe_code)]

/// The macOS boot-head self-insertion (spec 22 §2 "Phase 1 delivery"):
/// the embedded interpose-dylib, DYLD_INSERT_LIBRARIES, and the once-only
/// re-exec — see the module. Compiles out on every other target.
#[cfg(target_os = "macos")]
pub(crate) mod interpose;

use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

use libc::{c_char, c_int};

use crate::driver::ProcessEnv;
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

/// The ruby runtime root — the memfs mount point the patched ruby is
/// compiled against. The factory is the single owner of the value (the
/// source tarball's tebako-mount-root manifest → the exe's generated fs
/// TU → tebako_main forwards it here); this default is only for
/// driver-only consumers and tests and follows the factory convention:
/// `A:/t` on windows (short by owner decision — MAX_PATH headroom on
/// every in-image path), `/__tfs__` elsewhere. A boot may redirect the
/// root via `TEBAKO_MOUNT_ROOT` (spec 17 §1) when the env image's layout
/// grants `mount_root_override` — the getter below then reports the
/// effective root the boot established, never this default.
#[cfg(windows)]
const RUBY_RUNTIME_ROOT: &str = "A:/t";
#[cfg(not(windows))]
const RUBY_RUNTIME_ROOT: &str = "/__tfs__";

#[cfg(windows)]
static DEFAULT_ROOT: &[u8] = b"A:/t\0";
#[cfg(not(windows))]
static DEFAULT_ROOT: &[u8] = b"/__tfs__\0";
static EMPTY: &[u8] = b"\0";

/// The mount point the boot established (read by the io-routing patches
/// via [`tebako_mount_point`]); set once, process-lifetime.
static MOUNT_POINT: OnceLock<CString> = OnceLock::new();
/// The process cwd captured at boot ([`tebako_original_pwd`]).
static ORIGINAL_PWD: OnceLock<CString> = OnceLock::new();
/// -1 when the boot passed through for `miniruby` (the build-time tool).
static RUNNING_MINIRUBY: AtomicI32 = AtomicI32::new(0);

/// The compiled-in contract version (spec 06 §6) — the runtime factory's
/// release manifest declares the same value, and its CI fails any
/// release where the two disagree.
///
/// # Safety
/// No pointers, no state — callable from any context.
#[no_mangle]
pub unsafe extern "C" fn tebako_driver_contract_version() -> u32 {
    crate::TEBAKO_CONTRACT_VERSION
}

/// The mount point the boot established (the effective runtime root —
/// the compiled-in value or its `TEBAKO_MOUNT_ROOT` override). Before
/// any boot, the ruby default.
///
/// # Safety
/// Returns a process-lifetime C string; never free it.
#[no_mangle]
pub unsafe extern "C" fn tebako_mount_point() -> *const c_char {
    MOUNT_POINT
        .get()
        .map(|c| c.as_ptr())
        .unwrap_or(DEFAULT_ROOT.as_ptr() as *const c_char)
}

/// Record the root a boot established (the effective value, after any
/// `TEBAKO_MOUNT_ROOT` override). Called once per boot by the driver
/// core; a repeat boot keeps the first value (process-lifetime state).
pub(crate) fn set_mount_point(root: &str) {
    if let Ok(c) = CString::new(root) {
        let _ = MOUNT_POINT.set(c);
    }
}

/// The process cwd captured at boot; empty before any boot.
///
/// # Safety
/// Returns a process-lifetime C string; never free it.
#[no_mangle]
pub unsafe extern "C" fn tebako_original_pwd() -> *const c_char {
    ORIGINAL_PWD
        .get()
        .map(|c| c.as_ptr())
        .unwrap_or(EMPTY.as_ptr() as *const c_char)
}

/// -1 when the boot passed through for `miniruby`, 0 otherwise.
///
/// # Safety
/// No pointers, plain load — callable from any context.
#[no_mangle]
pub unsafe extern "C" fn tebako_is_running_miniruby() -> c_int {
    RUNNING_MINIRUBY.load(Ordering::SeqCst)
}

/// The generic boot. `argc`/`argv` point at the process argv (argv[0]
/// included — the handoff grammar scans past it); on success they are
/// REPLACED with the driver-owned rewritten argv (leaked deliberately:
/// the boot runs once, pre-interpreter, and the memory lives exactly as
/// long as the process). `runtime_root` is the mount point the
/// interpreter was compiled against.
///
/// Returns 0 on success; a named loader exit code (65–74, 78) on failure,
/// with the named message on stderr and nothing left mounted. 78 is the
/// spec-18 C3 env-image layout refusal (the pair check runs post-mount,
/// before any handoff).
///
/// # Safety
/// C ABI entry point: `argc`, `argv`, `*argv`, and `runtime_root` must
/// be valid per the C contract (`argv[i]` NUL-terminated strings,
/// `runtime_root` NUL-terminated).
#[no_mangle]
pub unsafe extern "C" fn tebako_driver_boot(
    argc: *mut c_int,
    argv: *mut *mut *mut c_char,
    runtime_root: *const c_char,
) -> c_int {
    if runtime_root.is_null() {
        return EX_TEBAKO_MANIFEST;
    }
    boot_impl(argc, argv, runtime_root)
}

/// The ruby-runtime entry (the patched ruby `main.c` hook's behavior).
/// Identical to [`tebako_driver_boot`] with the ruby memfs root, plus:
///
/// - the `miniruby` pass-through: the build-time tool links the same
///   patched `main.c` and must run as a plain interpreter (no mounts, no
///   exports — the v1 `tebako_main` behavior);
/// - the `TEBAKO_CONTRACT_VERSION` environment export (roadmap 45): the
///   runtime is authoritative for the contract it speaks, so an
///   inherited value is always overwritten;
/// - the mount-point and original-pwd getters, set for the io-routing
///   patches.
///
/// NOT a C export: the `tebako_main` C symbol belongs to the runtime
/// factory's generated fs TU, which forwards to [`tebako_driver_boot`]
/// with the exe's own compiled-in mount point (the factory is the single
/// owner of the runtime root; the driver carries only this default for
/// driver-only consumers and tests — the two can never drift).
///
/// # Safety
/// Same contract as [`tebako_driver_boot`], minus `runtime_root`.
pub unsafe extern "C" fn tebako_main(argc: *mut c_int, argv: *mut *mut *mut c_char) -> c_int {
    if argc.is_null() || argv.is_null() {
        return EX_TEBAKO_IO;
    }
    let argvp = unsafe { *argv };
    if argvp.is_null() {
        return EX_TEBAKO_IO;
    }
    let n = unsafe { *argc };
    if n < 1 {
        return EX_TEBAKO_IO;
    }
    let argv0 = unsafe { CStr::from_ptr(*argvp) }.to_string_lossy();
    // Re-decide on every call: the flag is process-global, and a fresh
    // boot must not inherit a previous boot's verdict (test processes
    // boot repeatedly).
    RUNNING_MINIRUBY.store(0, Ordering::SeqCst);
    if argv0.contains("miniruby") {
        RUNNING_MINIRUBY.store(-1, Ordering::SeqCst);
        return 0;
    }
    std::env::set_var(
        "TEBAKO_CONTRACT_VERSION",
        crate::TEBAKO_CONTRACT_VERSION.to_string(),
    );
    let root = CString::new(RUBY_RUNTIME_ROOT).expect("static root is NUL-free");
    boot_impl(argc, argv, root.as_ptr())
}

/// The shared boot core (both entries). `runtime_root` is a valid
/// NUL-terminated string (checked by the callers).
fn boot_impl(argc: *mut c_int, argv: *mut *mut *mut c_char, runtime_root: *const c_char) -> c_int {
    if argc.is_null() || argv.is_null() {
        return EX_TEBAKO_IO;
    }
    let argvp = unsafe { *argv };
    if argvp.is_null() {
        return EX_TEBAKO_IO;
    }
    // macOS loader interposition (spec 22 §2, "Phase 1 delivery"): at
    // the head of the boot — before any mount, before the jail, before
    // the interpreter starts — re-exec once with the embedded interpose
    // dylib inserted. Loud-continue on any failure. Never reached by
    // miniruby: tebako_main's build-time pass-through returns above.
    #[cfg(target_os = "macos")]
    interpose::self_insert(argvp);
    let n = unsafe { *argc };
    if n < 0 {
        return EX_TEBAKO_IO;
    }
    let root = unsafe { CStr::from_ptr(runtime_root) }
        .to_string_lossy()
        .into_owned();
    // The mount-point export is set by the driver core AFTER the
    // TEBAKO_MOUNT_ROOT override resolves — never here (the baked value
    // would be locked in before the override is known).
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(c) = CString::new(cwd.to_string_lossy().as_ref()) {
            let _ = ORIGINAL_PWD.set(c);
        }
    }
    let raw: &[*mut c_char] = unsafe { std::slice::from_raw_parts(argvp, n as usize) };
    let args: Vec<String> = raw
        .iter()
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        .collect();

    match crate::driver::boot(&args, &root, &ProcessEnv) {
        Ok(outcome) => {
            let mut cstrings = Vec::with_capacity(outcome.argv.len());
            for s in &outcome.argv {
                let Ok(c) = CString::new(s.as_str()) else {
                    eprintln!("tebako-driver: argv value contains an interior NUL");
                    return EX_TEBAKO_MANIFEST;
                };
                cstrings.push(c);
            }
            let mut ptrs: Vec<*mut c_char> =
                cstrings.iter().map(|c| c.as_ptr() as *mut c_char).collect();
            ptrs.push(std::ptr::null_mut());
            let argv_out = Box::leak(ptrs.into_boxed_slice()).as_mut_ptr();
            // The strings the pointers reference live exactly as long:
            // process-lifetime, never freed.
            std::mem::forget(cstrings.into_boxed_slice());
            unsafe {
                *argv = argv_out;
                *argc = outcome.argv.len() as c_int;
            }
            0
        }
        Err(e) => {
            eprintln!("tebako-driver: {}", e.message);
            e.code
        }
    }
}

/// The spawned-runtime planner (spec 30 §2) — the interpreter's spawn
/// hook calls this at the head of a spawn: a bare `command` name the app
/// payload's `requires[].expose` surfaces is resolved against the
/// store's runtimes (the dispatch lock pins, never a download) and
/// planned into a full child boot; anything else passes through.
///
/// `args_packed`/`args_len` carry the user arguments as a NUL-packed
/// block (ruby's `argv_str` wire). Returns:
///
/// - `0` — pass-through: not a bare name, or no edge exposes it. The
///   out pointers are untouched; the caller runs its own spawn.
/// - `1` — planned: `*out_exe` is the runtime exe (a NUL-terminated
///   string), `*out_argv`/`*out_argv_len` the NUL-packed child argv
///   (argv[0] included), `*out_env`/`*out_env_len` the NUL-packed env
///   operations (`KEY=VALUE` sets, bare `KEY` deletes — in application
///   order). Every out block is libc-`malloc`'d; the caller frees.
/// - `-1` — a NAMED error: `*out_error` is the malloc'd message; the
///   spawn must NOT fall through to a host binary of the same name.
///
/// # Safety
/// C ABI entry point: `command` must be a valid NUL-terminated string,
/// `args_packed` readable for `args_len` bytes (may be null when
/// `args_len == 0`), and every out pointer valid for one write.
#[no_mangle]
pub unsafe extern "C" fn tebako_spawn_runtime_plan(
    command: *const c_char,
    args_packed: *const c_char,
    args_len: usize,
    out_exe: *mut *mut c_char,
    out_argv: *mut *mut c_char,
    out_argv_len: *mut usize,
    out_env: *mut *mut c_char,
    out_env_len: *mut usize,
    out_error: *mut *mut c_char,
) -> c_int {
    if command.is_null()
        || out_exe.is_null()
        || out_argv.is_null()
        || out_argv_len.is_null()
        || out_env.is_null()
        || out_env_len.is_null()
        || out_error.is_null()
    {
        return -1;
    }
    if args_len > 0 && args_packed.is_null() {
        return -1;
    }
    let command = unsafe { CStr::from_ptr(command) }
        .to_string_lossy()
        .into_owned();
    let raw: &[u8] = if args_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(args_packed as *const u8, args_len) }
    };
    let args: Vec<String> = raw
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty())
        .map(|e| String::from_utf8_lossy(e).into_owned())
        .collect();
    let mount_var_keys: Vec<String> = std::env::vars()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with("TEBAKO_MOUNT_"))
        .collect();
    match crate::spawn::plan(&command, &args, &mount_var_keys) {
        Ok(None) => 0,
        Ok(Some(plan)) => {
            let exe = plan.argv[0].clone();
            let argv_block = pack_nul(plan.argv.iter());
            let env_block = pack_nul(plan.env_ops.iter().map(|(k, v)| match v {
                Some(value) => format!("{k}={value}"),
                None => k.clone(),
            }));
            let (Some(exe_p), Some(argv_p), Some(env_p)) = (
                malloc_cstr(&exe),
                malloc_bytes(&argv_block),
                malloc_bytes(&env_block),
            ) else {
                return spawn_ffi_error(out_error, "out of memory packing the spawn plan");
            };
            unsafe {
                *out_exe = exe_p;
                *out_argv = argv_p;
                *out_argv_len = argv_block.len();
                *out_env = env_p;
                *out_env_len = env_block.len();
            }
            1
        }
        Err(message) => spawn_ffi_error(out_error, &message),
    }
}

/// Join strings into the NUL-packed wire block (each element
/// NUL-terminated — the receiver splits on NUL).
fn pack_nul(items: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        out.extend_from_slice(item.as_ref());
        out.push(0);
    }
    out
}

/// libc-`malloc` a byte copy (the caller frees). `None` on allocation
/// failure. The NUL-packed wire blocks carry NUL bytes BY DESIGN — the
/// interior-NUL refusal lives in `malloc_cstr` (single strings only).
fn malloc_bytes(bytes: &[u8]) -> Option<*mut c_char> {
    let len = bytes.len().max(1);
    let p = unsafe { libc::malloc(len) } as *mut c_char;
    if p.is_null() {
        return None;
    }
    if !bytes.is_empty() {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len()) };
    }
    Some(p)
}

/// libc-`malloc` a NUL-terminated string copy (the caller frees).
/// `None` on allocation failure or an interior NUL.
fn malloc_cstr(s: &str) -> Option<*mut c_char> {
    let c = CString::new(s).ok()?;
    let bytes = c.as_bytes_with_nul();
    let p = unsafe { libc::malloc(bytes.len()) } as *mut c_char;
    if p.is_null() {
        return None;
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len()) };
    Some(p)
}

/// Fill `*out_error` with the named message and answer `-1`.
fn spawn_ffi_error(out_error: *mut *mut c_char, message: &str) -> c_int {
    match malloc_cstr(message) {
        Some(p) => unsafe {
            *out_error = p;
        },
        None => unsafe {
            *out_error = std::ptr::null_mut();
        },
    }
    -1
}
