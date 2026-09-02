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
//! 1. `TEBAKO_TFS_MOUNTS=image[:slot]:mount,image[:slot]:mount,…` — mount
//!    each image through the engine (see [`spec`] for the grammar; the
//!    slot form mounts a package slot's region, spec 17 §2.1).
//!    Absent/empty → the shim is fully inert.
//! 2. `TEBAKO_JAIL=<spec 08 env form>` — install the `host_policy`
//!    (`open`/`deny`, docker-`-v` grants, `@` argument files; see
//!    [`tfs::policy::JailSpec`]). Installed AFTER the mounts — the mount
//!    family's image read is itself policy-gated once a policy is active.
//! 3. `TEBAKO_TRACE=<capture path>` — arm the spec 25 §2 interception
//!    bus BEFORE any mount (the driver is not the bus's only armer; the
//!    preload delivery honors the same env contract). A failed arm is a
//!    loud stderr note, never an init error (law 1: observability never
//!    gates). A spawned/exec'd child re-arms from the inherited env at
//!    its own constructor, appending to the same channel.
//!
//! Misformatted env values, or an image that will not mount, are named
//! configuration errors: a clear stderr message naming the variable and
//! the offending token, exit [`spec::EX_CONFIG`] (78).
//!
//! Interposed surface: `open`, `openat`, `stat`, `lstat`, `fstat`,
//! `fstatat` (+`fstatat64`/`__xstat`/`__lxstat`/`__fxstat`/`__fxstatat`/
//! `__xstat64`/`__lxstat64`/`__fxstat64`/`__fxstatat64`/`statx`/
//! `getdents64` and the LFS `open64`/`openat64`/`stat64`/`lstat64`/
//! `fstat64`/`pread64` family on Linux — roadmap 39; the fortified
//! `__read_chk` and `__openat_2`, and `fopen64` — tebako#439:
//! libcrypto's `o_fopen.c` defines `_FILE_OFFSET_BITS=64` itself on
//! linux, so every `BIO_new_file`/`X509_LOOKUP_load_file` binds
//! `fopen64`, never `fopen`), `access`,
//! `faccessat`, `opendir`, `readdir` (+`readdir64` on Linux),
//! `readdir_r`, `rewinddir`/`telldir`/`seekdir`, `dirfd`, `closedir`,
//! `pread`, `read`, `lseek` (additive — stdio fseek on a memfs fd must
//! stay on the VFS), `close`, `mkdir`, `unlink`, `rename`, `dlopen`,
//! `fopen` (read modes), the `realpath` family (`realpath` + macOS's
//! `realpath$DARWIN_EXTSN`; linux's `canonicalize_file_name` and
//! `__realpath_chk` — spec 07 §8: glibc's realpath walks the path
//! with libc-INTERNAL aliases no PLT interpose sees, so covered paths
//! are answered by the shim with the MOUNT spelling, lexically
//! normalized — a host canonicalization must never rewrite a VFS path,
//! the 2026-09-03 dogfood jing ClassNotFound), and
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
//!   VFS, as does any statically linked child. A macOS exec/spawn target
//!   whose Mach-O cannot load this (per-arch) dylib — an arm64e slice
//!   dyld would prefer, or no host-arch slice at all — gets a rebuilt
//!   env WITHOUT `DYLD_INSERT_LIBRARIES` for that one exec (tebako#448):
//!   dyld TERMINATES a child over an incompatible insertion otherwise.
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
//! - `fork` is not interposed, but its child side is GUARDED: a
//!   `pthread_atfork` child handler arms a process-global flag, and every
//!   shim's engine entry answers "pass through" while it is set. The
//!   engine's backends are not fork-safe — dwarfs-t's block-cache worker
//!   pool dies at `fork`, so a backend-touching call in the child (e.g.
//!   the execve materialization probe under a `/` mount) would wait on
//!   threads that no longer exist (the 2026-08-22 git-clone deadlock,
//!   runtime 0.16.4). glibc runs NO atfork handlers for `vfork(2)`, and a
//!   vfork child shares the parent's whole address space with the parent
//!   thread suspended in the kernel — so the guard carries a PID backstop
//!   (`INIT_PID`, stored by the constructor): an engine entry whose
//!   `getpid` differs from the initializing pid is a fork/vfork child in
//!   its pre-exec window and passes through (the 2026-09-03 dash
//!   vfork+lazy-init deadlock in the dogfood repro). `exec` preserves the
//!   pid, so the exec'd image's constructor re-stores the same value and
//!   the gate re-opens exactly at exec. A fork child that goes on to
//!   `exec` re-enters a
//!   fresh, healthy shim through the inherited preload env, so the
//!   child-namespace propagation above is unaffected; a child that never
//!   execs sees the host filesystem only — memfs fds it inherited answer
//!   EIO, and an inherited memfs `DIR*` passed to the host dir family is
//!   a caller bug (that path wedged before this guard).
//! - Not interposed in v1: `fork` itself (its child side is guarded —
//!   see above), `openat2` (glibc has no wrapper at
//!   all — see above), `fstatat64` on macOS (the legacy 32-bit-inode
//!   layout), the fortify open family beyond `__openat_2`
//!   (`__open_2`/`__open64_2`/`__openat64_2` on Linux — no importer
//!   observed in the 0.16.6 linux-gnu runtime), `fdopendir`
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
