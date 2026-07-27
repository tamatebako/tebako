//! # libtfs-preload — the preload interposition shim (spec 07 §8, tier 1)
//!
//! The mainline native-exec mechanism: a tiny library injected into a
//! DYNAMIC native binary — `DYLD_INSERT_LIBRARIES` (Mach-O) or
//! `LD_PRELOAD` (ELF) — that maps the libc file-IO family onto the tfs
//! engine in-process, so the binary and its whole dynamic chain see the
//! mounted image with **no extraction**. The shim is a TFS consumer, never
//! a format; there is no FUSE anywhere in the delivery.
//!
//! ## Contract
//!
//! On init (library constructor, before the program's `main`):
//!
//! 1. `TEBAKO_TFS_MOUNTS=image:mount,image:mount,…` — mount each image
//!    through the engine (see [`spec`] for the grammar). Absent/empty →
//!    the shim is fully inert.
//! 2. `TEBAKO_JAIL=<spec 08 env form>` — install the `host_policy`
//!    (`open`/`deny`, docker-`-v` grants, `@` argument files; see
//!    [`tfs::policy::JailSpec`]). Installed AFTER the mounts — the mount
//!    family's image read is itself policy-gated once a policy is active.
//!
//! Misformatted env values, or an image that will not mount, are named
//! configuration errors: a clear stderr message naming the variable and
//! the offending token, exit [`spec::EX_CONFIG`] (78).
//!
//! Interposed surface: `open`, `openat`, `stat`, `lstat`, `fstat`,
//! `access`, `faccessat`, `opendir`, `readdir` (+`readdir64` on Linux),
//! `closedir`, `pread`, `read`, `lseek` (additive — stdio fseek on a
//! memfs fd must stay on the VFS), `close`, `mkdir`, `unlink`, `rename`,
//! `dlopen`. Memfs paths are served by the engine; host paths pass
//! through, gated by the SAME `host_policy` (spec 08 §3 — denied paths
//! fail `EPERM`, writes against an ro grant `EROFS`). `dlopen` of a memfs
//! library rides the engine's `dlmap2file` host cache.
//!
//! ## Honest scope (v1)
//!
//! - Dynamic binaries only (a static binary has no interposition point —
//!   spec 07 §8 tier 2 is its story). macOS first-class, linux-gnu
//!   first-class; Windows is roadmap 30 phase 2.
//! - The process tree stays in the VFS by env propagation: a child
//!   process inherits the preload variables, so its own IO is interposed
//!   too. Spawning works with HOST paths (a memfs path is not exec'able —
//!   `execve` is not virtualized in v1; the `tfs exec` launcher
//!   materializes the ENTRYPOINT through `dlmap2file`, and children
//!   re-spawn via `argv[0]`). Platform binaries under SIP (macOS) strip
//!   `DYLD_*` — they leave the VFS, as does any statically linked child.
//! - A mount at `/` is rejected (it would claim every host path and
//!   bypass the jail). Non-UTF-8 paths always pass to the host (the
//!   engine itself is UTF-8-only). `dlopen` jail-denials return NULL with
//!   the cause in errno — `dlerror()` text is not settable portably.
//! - Not interposed in v1: `execve`/`posix_spawn`, `fstatat`/`statx`,
//!   `getdents64`, `openat2`, `readdir_r`, `rewinddir`/`telldir`/`seekdir`
//!   (memfs dir streams support readdir/closedir only), and the
//!   pre-glibc-2.33 `__xstat` family.
//!
//! ## Layout
//!
//! - [`spec`] — the configuration surface: `TEBAKO_TFS_MOUNTS`
//!   ([`tfs::mount_spec`], re-exported — one grammar, one parser, shared
//!   with `tfs exec`) plus the shim's EX_CONFIG exit code (safe).
//! - [`route`] — the memfs/host-or-denied decisions over the engine
//!   (safe; unit-tested).
//! - `sys` — the interpose/dlsym layer; the ONLY unsafe module (the two
//!   safe modules `#![forbid(unsafe_code)]`).

pub mod spec;

#[cfg(unix)]
pub mod route;

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
compile_error!(
    "libtfs-preload targets macOS and linux-gnu (spec 07 §8 tier 1); other unixes are untested"
);

#[cfg(unix)]
mod sys;
