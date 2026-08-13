# Spec 07 — Shims and dispatch

Normative specification of executable registration and version
management. Status: PARTIAL — the dispatcher and version manager
(§2–§4) ship as `crates/tebako-shim` in tebako-rs (roadmap 08; retires
`mnenv`). Remote registry fetch ships with `tebako add-registry |
install` (roadmap 28.1 — the CLI resolves every registry form of
spec 04 §2), and the dispatch-time registry cache ships with roadmap 33
(the registry-default chain link resolves every registry form through
tebako-resolve behind `~/.tebako/registries/<sha>.yaml` — 24 h TTL,
`tebako update-registries`, `TEBAKO_OFFLINE` = cache-or-named-error;
`tebako add-registry` primes the cache with the bytes it fetched).
Still PLANNED: `tebako use` writing the user default (v1: author
`~/.tebako/config.yaml` directly), jail application (spec 08), and a
runtime registry (v1 downloads resolve through the `runtimes:` preference
in `config.yaml` as the exact ref).

## 0. v1 concrete choices (normative where the sections above were open)

- **Project pin file** `.tebako-tools.yaml` is a FLAT YAML mapping of
  command name → version; a nearer file that does not pin the command
  does not shadow a farther one that does.
- **`~/.tebako/config.yaml` keys:** `defaults:` (command → version),
  `registries:` (spec 04 refs), `runtimes:` (engine → `{version,
  tebako}` runtime preference). The shim never writes this file.
- **Disabled state** is shim-managed state, not authored config:
  `~/.tebako/shims/.disabled.yaml` (command → `[versions] | [all]`).
- **Installed payload record** (the dispatcher-visible manifest mirror,
  spec 03 §4 tier 3 rationale): `payloads/<name>/<version>.tfs`,
  `<version>.tfs.sha256`, `<version>.manifest.yaml`.
- **Zero-runtime dispatch** (spec 03 §2.2): the payload image itself is
  the program — the self-launching-image contract; dependency mounts
  still ride the ABI v1 argv shape minus the runtime prefix.
- **Registry payload mounts** are bare images: slot `0` (whole image),
  mount point `/` of the jail namespace.

## 1. The model

Tebako manages shims for **every executable every installed payload
provides** (spec 03 `entrypoints`). One payload may carry MULTIPLE
executables — each becomes a registered command. Four artifacts, four
jobs:

- **payload** — a signed `.tfs` image (versioned, runtime-independent).
- **runtime** — a signed runtime payload (versioned, cached,
  machine-shared).
- **registry** — a developer-hosted `tpkg-registry.yaml` (spec 04 §2).
- **dispatcher** (`tebako-shim`, a tiny static Rust binary in tebako-rs) —
  the thing on PATH that picks version + runtime per invocation and hands
  off.

## 2. The dispatch chain (per invocation of `~/.tebako/shims/<tool>`)

0. **argv0 is the selector.** One tebako-shim binary, linked per command
   name; it maps name → entrypoint in the payload's manifest.
   **Multi-command suites:** one package with N entrypoints installs N
   shims — each dispatches to its own image AND ITS OWN runtime
   requirement; two commands in one package may run different runtime
   versions simultaneously.
1. **Payload VERSION resolution** (first match wins):
   `TEBAKO_<TOOL>_VERSION` env → nearest `.tebako-tools.yaml` walking up
   from cwd (per-project pinning) → user default (`tebako use
   <tool>@<version>`) → registry's `default`.
2. **RUNTIME resolution:** the entrypoint's `runtime_requirement` →
   newest COMPATIBLE cached runtime (no download) → else download newest
   compatible (spec 05 §5). **Swapping runtimes needs no payload change**
   — the payload is immutable; only the dispatch-time choice changes
   (`tebako use --runtime ruby@3.4.2`, or a per-project pin).
3. **Hand-off:** mount payload + ZERO OR MORE runtime payloads (native
   entrypoints need none — spec 03) + declared dependency mounts
   (spec 03 §2.3), apply the jail view (spec 08), exec the entrypoint.
   Signed payloads are verified at install time, not per run.

## 3. Shell integration (no per-shell magic)

- ONE directory on PATH: `~/.tebako/shims`. One-time setup;
  `tebako shim install-shell [--shell bash|zsh|fish|csh]` inserts a
  managed BEGIN/END block into the right startup file
  (`.profile`/`.bash_profile`/`.bashrc`/`.zshrc`/`.cshrc`) prepending the
  shim dir; idempotent; `uninstall-shell` removes exactly its block.
- **Shim links are always an explicit act** (TODO.v2-1/12): registry
  installs register the payload's `provides` (that IS the install's
  point); a local-package install (`tebako install <path>`) links only
  with the explicit `--shims`; a run — of any package, ever — links
  nothing. A one-off act never claims PATH names.
- NO eval-init hook for switching: the dispatcher reads the project file
  itself (the mise model, not the rbenv `eval "$(… init -)"` model).
- `tebako use / disable / list / doctor` manage shims
  (link/remove/inspect/diagnose); enable/disable specific versions.

## 4. Configuration

- User config: `~/.tebako/config.yaml` (YAML — the locked convention;
  supersedes the earlier `config.json` note). Contents: defaults,
  registries, runtime preferences.
- Project pins: `.tebako-tools.yaml` at any directory — the dispatcher
  walks up from cwd; nearest wins.

## 5. Distribution forms (both produced by `tebako press`)

1. **Standalone tpkg** (always per-platform): self-contained executable
   for users WITHOUT tebako.
2. **Registry payload** (`.tfs` + registry metadata): for dispatcher
   users — ONE universal image when pure-language; per-triplet variants
   only for native-extension apps.

## 6. Retirement gate

When tebako-shim ships, **mnenv retires**; metanorma becomes the first
dogfood consumer (heavy native-ext app proving the whole stack: press →
signed `.tfs` per (version × ruby line) → registry → dispatcher).

## 7. Errors (named)

- No compatible cached/downloadable runtime → the spec 06 exit 69 shape.
- Native-ext payload on a wrong-ABI-line runtime → named compatibility
  error, never a segfault.
- Shim target payload missing/corrupt → named error pointing at
  `tebako doctor`.

## 8. Native exec from inside an image (the whole-chain model, locked)

Running a NATIVE executable from a mounted payload raises two problems:
the exec'd binary loses the VFS view (it sees the host), and its dynamic
dependency chain must resolve. Old tebako never solved this (its memfs
was ruby-only; metanorma-on-tebako required host-installed inkscape).
The locked model is three tiers — **interposition-first, never FUSE**:

1. **Dynamic native tools (the mainline): the preload interposition
   shim** — `libtfs-preload`, injected via `LD_PRELOAD` (ELF),
   `DYLD_INSERT_LIBRARIES` (Mach-O), or DLL injection (Windows), mapping
   the libc/dyld file-IO family (open/stat/opendir/pread/dlopen…) onto
   `tebako_fs_*`. The launcher seeds the mount table via env
   (`TEBAKO_TFS_MOUNTS=image:mount,…`); the binary AND its whole dynamic
   chain see the mounted image — **no extraction, no chain problem**.
   retrace (in-family: linux/macOS/windows CI, v2 config-driven
   interception) is the reference technique. Bonus: interposed IO flows
   through the same `host_policy` — **jails extend to these binaries**
   (spec 08). Limit: dynamic binaries only; the shim is a TFS consumer,
   never a format. **SHIPPED (roadmap 30): `crates/libtfs-preload` +
   `tfs exec <image>[:mount] [--image …] [--jail <spec>] -- <cmd>` in
   tfs-cli — macOS and linux-gnu first-class, windows later. Coverage
   (roadmap 39):** interposed surface open/openat/stat/
   lstat/fstat/fstatat(+fstatat64/statx/__xstat/__lxstat/__fxstat/
   __fxstatat and the LFS open64/stat64/lstat64/fstat64/pread64/lseek64
   family and the pre-2.33-glibc versioned __xstat64/__lxstat64/
   __fxstat64 forms and the _FORTIFY_SOURCE __read_chk wrapper on
   linux)/access/faccessat/opendir/readdir(+readdir64)/
   readdir_r/rewinddir/telldir/seekdir/dirfd/closedir/pread/read/lseek/
   mmap(memfs fd → private anonymous mapping pre-filled from the VFS,
   +mmap64 on linux)/
   close/mkdir/unlink/rename + dlopen + execve/posix_spawn/posix_spawnp;
   `TEBAKO_JAIL` carries the spec 08 §1 env form (`open|deny` +
   `host:mount:ro|rw` grants + `@` argument files); the ENTRYPOINT (when
   in-image) is materialized through `dlmap2file` (execve needs a host
   path — one copy per exec, gc later), and execve/posix_spawn of MEMFS
   paths materialize the same way (a tool spawning an in-image helper
   needs no extraction; environ propagates, so grandchildren stay in the
   VFS); every *at shim gates its fd branch on `dirfd >= 0` (AT_FDCWD
   carries the fd-flag bit — the runtime-builds regression class, pinned
   by tests); the mmap shim gates on `fd >= 0` and `!MAP_ANONYMOUS` (fd
   -1 carries the same bit — the JVM PaX-check regression class); exec
   of a HOST binary is not policy-gated (not an IO route
   in the policy's op classes — the child's own IO stays jailed via env
   propagation); SIP platform binaries strip `DYLD_*` and leave it; a
   mount at `/` is refused (it would claim every host path and bypass
   the jail); not interposed: fork, `openat2` (glibc exposes no wrapper
   or symbol on linux-gnu — nothing to interpose; a raw `syscall(2)`
   caller bypasses userland interposition by construction), fstatat64 on
   macOS (the legacy
   32-bit-inode layout), `__fxstatat64` and the write-side
   pwrite64/ftruncate64/statvfs64 family plus
   readv/preadv/sendfile/copy_file_range on memfs fds on linux,
   fdopendir (memfs directories are never fd-opened), and
   syscall()-direct IO (raw syscalls bypass userland interposition by
   construction); `dirfd` of a memfs stream answers -1/ENOTSUP;
   `getdents64` on a memfs fd answers ENOTDIR (VFS directories enumerate
   via opendir, never fds).
2. **Static/self-contained tools**: tebako is the EXECUTOR, not an
   injector — it spawns and owns every child, so mediating a child's
   syscalls is the execution contract itself, not hijacking (a
   side-loader would need to BECOME the executor or the kernel to hold
   the same position — outside the threat model by definition). As the
   executor, tebako mediates statics two ways:
   (a) **Kernel-mediated VFS (Linux, tier 2a)**: `SECCOMP_RET_USER_NOTIF`
   — the executor installs a seccomp filter at spawn; the kernel
   forwards the child's file syscalls (openat/statx/…) to the executor,
   which serves them from the mounted TFS images and enforces the same
   `host_policy` (spec 08) — full VFS + jails for static binaries, no
   extraction, child unaware. Cost: per-syscall user/kernel round trips
   (io-heavy tools notice); platform: Linux only (the API does not
   exist elsewhere).
   (b) **Extract-to-exec-cache (everywhere, tier 2b)**:
   `~/.tebako/exec/<sha256>/…`, 0755, verified at install — the
   manifest-declared closure, content-addressed, shared. On
   macOS/Windows (no executor-side syscall mediation API) this is the
   static path; such binaries see only the host FS (no VFS, no jails —
   a platform-API statement, not a trust statement).
   Nuances: a static binary BUILT against the `tebako_fs_*` ABI carries
   the VFS inside itself (the "libtfs for everyone" embedding story —
   build-time property, not imposable on an arbitrary binary). Prefer
   tier 1 (dynamic) for io-heavy tools; tier 2a covers the rest on
   Linux.
3. **Interpreted payloads (ruby)**: in-process io-routing patches
   virtualize file IO — the existing mechanism, no exec at all.

- **FUSE is NOT an exec mechanism.** It remains an optional,
  host-dependent HUMAN convenience (`tfs mount`, spec 11 access
  matrix #2) — never on the package/exec path. (Correction 2026-07-26:
  tebako has never used FUSE; interposition is the tebako way.)
- ptrace/syscall-tracing (retrace-v1-style tracing for vulnerability
  research) is a debugging tool, not an exec foundation.

## 9. Environment composition (locked 2026-07-27)

Env across the mount stack (runtime + toolkit layers + data layers +
app) composes by RULE, never by accident:

- **Path-class vars** (`GEM_PATH`, `PATH`-in-image, `TEBAKO_LD_PATH`):
  app payload first, then toolkit layers in `requires` order, then the
  runtime — earlier wins. The runtime's own defaults come LAST (it is
  the fallback, not the override).
- **Scalar vars**: the app's manifest wins over toolkit, toolkit over
  runtime; the dispatcher's computed values (TEBAKO_RUNTIME_IMAGE,
  TEBAKO_TFS_MOUNTS, jail env) always win over every manifest.
- **Host env scrub**: on handoff the dispatcher removes the variables
  the press scrubbed (RUBYOPT/RUBYLIB/BUNDLE_* — the M7 scrub list) so
  the host environment never leaks semantics into the payload; other
  host vars pass through unless the jail says otherwise.
- Per-entrypoint overrides (suite entries, spec 03 §6) apply last for
  that entry only.
