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
//! `fstatat` (+`fstatat64`/`__xstat`/`__lxstat`/`__fxstat`/`__fxstatat`/
//! `statx`/`getdents64` and the LFS `open64`/`stat64`/`lstat64`/`fstat64`/
//! `pread64` family on Linux — roadmap 39), `access`,
//! `faccessat`, `opendir`, `readdir` (+`readdir64` on Linux),
//! `readdir_r`, `rewinddir`/`telldir`/`seekdir`, `dirfd`, `closedir`,
//! `pread`, `read`, `lseek` (additive — stdio fseek on a memfs fd must
//! stay on the VFS), `close`, `mkdir`, `unlink`, `rename`, `dlopen`, and
//! `execve`/`posix_spawn`/`posix_spawnp` (memfs paths materialize through
//! the `dlmap2file` host cache; roadmap 39). Memfs paths are served by
//! the engine; host paths pass through, gated by the SAME `host_policy`
//! (spec 08 §3 — denied paths fail `EPERM`, writes against an ro grant
//! `EROFS`). `dlopen` of a memfs library rides the engine's `dlmap2file`
//! host cache. Every *at shim gates its fd branch on `dirfd >= 0` —
//! AT_FDCWD (-100) carries the TEBAKO_FD_FLAG bit, so a bare bit test
//! would misroute it (pinned by `route::tests::at_fdcwd_is_not_a_memfs_fd`).
//!
//! ## Honest scope (v1)
//!
//! - Dynamic binaries only (a static binary has no interposition point —
//!   spec 07 §8 tier 2 is its story). macOS first-class, linux-gnu
//!   first-class; Windows is roadmap 30 phase 2.
//! - The process tree stays in the VFS by env propagation: a child
//!   process inherits the preload variables, so its own IO is interposed
//!   too. Spawning works with HOST paths and — since roadmap 39 — with
//!   MEMFS paths: execve/posix_spawn of an in-image binary materializes
//!   it through `dlmap2file` (one copy per exec, gc later); a bare name
//!   is a host PATH search (memfs dirs are not in it). `exec` of a host
//!   binary is NOT policy-gated (it is not an IO route in the policy's op
//!   classes — the child's own IO stays jailed via env propagation).
//!   Platform binaries under SIP (macOS) strip `DYLD_*` — they leave the
//!   VFS, as does any statically linked child.
//! - A mount at `/` is legitimate (the app payload mounts there, spec
//!   17): covered-but-not-held paths fall through to the host with the
//!   policy gate consulted, exactly like any other mount. Non-UTF-8 paths
//!   always pass to the host (the engine itself is UTF-8-only). `dlopen`
//!   jail-denials return NULL with the cause in errno — `dlerror()` text
//!   is not settable portably.
//! - `dirfd` of a memfs stream answers -1/ENOTSUP (there is no host fd
//!   behind it; the stream itself works through the readdir family).
//!   `getdents64` on a memfs fd answers ENOTDIR (memfs fds are regular
//!   files only — VFS directories enumerate via opendir, never fds).
//!   glibc exposes NO `openat2` wrapper/symbol, so there is nothing to
//!   interpose on linux-gnu (a raw `syscall(2)` caller bypasses userland
//!   interposition by construction).
//! - Not interposed in v1: `fork`, `openat2` (glibc has no wrapper at
//!   all — see above), `fstatat64` on macOS (the legacy 32-bit-inode
//!   layout), the LFS `__xstat64`/`__lxstat64`/
//!   `__fxstat64`/`__fxstatat64` versioned forms on Linux, `fdopendir`
//!   (a host fdopendir passes through whole; a memfs directory can never
//!   be fd-opened, so it never arises), and syscall()-direct IO (raw
//!   `syscall(2)` calls bypass userland interposition by construction —
//!   musl/rust-std-on-gnu ride the libc wrappers, which ARE covered).
//!   Memfs exec materialization leaks one dl-cache copy per exec (gc is
//!   a later milestone, stated honestly in the spec).
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
