# tebako-rs

The Rust workspace for the tebako stack: the `tebako_fs_*` C ABI
(libtfs-rs), the `tpkg` package-trailer format, and — in later milestones —
the package tooling, CLI and bootstrap. Repo strategy and naming are
**locked** in the tamatebako restructure plan (`TODO.restructure/21` —
repo map + archival gates — with the design in items 19 (libtfs-rs) and
22 (bootstrap)).

## Workspace map (locked)

```
tebako-rs/
  crates/
    tpkg/              # the manifest format — single source of truth
    tfs/               # libtfs-rs: the tebako_fs_* ABI (cdylib libtfs.so/
                       # .dylib/.dll + staticlib libtfs.a — a drop-in for
                       # C++ libtfs). Mount table, fd/dir tables, path
                       # dispatch, errno channel, backends
    tebako-pkg/        # PLANNED — tpkg trailer surgery only (bundle/
                       # unbundle/reassemble/insert-image/remove-image/
                       # set-runtime/info; item 25)
    tfs-cli/           # PLANNED — the tfs binary (generic VFS ops; item 25)
    tebako-cli/        # PLANNED — press/deploy/cache/resolve (item 17)
    tebako-bootstrap/  # PLANNED — the Rust bootstrap runner (item 22,
                       # < 2 MB static, size-gated)
  tests/contract/      # the parity oracle: the C++ libtfs c_api suite
                       # running through the Rust C ABI
```

Naming is locked: crates are `tfs`, `tebako-pkg`, `tebako-cli`,
`tebako-bootstrap`, `tpkg` ("-rs" belongs on repos, not crates). The DwarFS
binding is NOT in this workspace: it lives in its own repo
[`tamatebako/dwarfs-rs`](https://github.com/tamatebako/dwarfs-rs) and is
consumed as an ordinary external dependency (git/path dep early, crates.io
when published). dwarfs-t stays C++ forever; `libdwarfs_c` is the only
Rust-consumable surface for it.

## Status

### SHIPPED (milestone 4)

- **`crates/tebako-pkg` — the package (tpkg) trailer surgery CLI** (item 25's
  scoping: TPKG TRAILER operations ONLY — generic image ops belong to the
  future tfs-cli). Subcommands with the exact C++ tebakofs semantics
  (flags, output formats, exit codes, stderr bodies):
  - `info <archive>` — trailer dump (slots/offsets/mounts/lean/abi/
    runtime_ref) or plain-archive summary via the tfs C ABI
  - `bundle --bootstrap <exe> --image <img[:mountpoint]>... -o <file>
    [--runtime-ref <ref>] [--lean] [--launcher-abi <n>]`
  - `unbundle <binary> -o <dir>` (bootstrap.bin + image-N.bin +
    manifest.json with per-part crc32)
  - `reassemble <dir> -o <file>`
  - `insert-image <binary> <img[:mountpoint]>` /
    `remove-image <binary> <slot>` /
    `set-runtime <binary> <runtime-file>` (atomic in-place rewrites)
  Built on `crates/tpkg` (trailer byte-parity; a streaming `tpkg::Crc32`
  was added for the part checksums) and `crates/tfs` (the info fallback).
  **Golden parity vs the C++ tool (proven byte-for-byte)**: bundle output,
  info output, manifest.json, unbundled parts, reassemble round-trip,
  insert/remove/set-runtime rewrites — all byte-identical to tebakofs
  (libtfs main). Golden tests auto-detect the oracle (`TEBAKOFS_CPP`, the
  libtfs build tree, or PATH) and skip without it; the round-trip and CLI
  suites are oracle-free.

### SHIPPED (milestone 3)

- **tfs: SquashFS backend via `crates/sqfs-sys`** — hand-written FFI to
  libsquashfs (squashfs-tools-ng), same discipline as dwarfs-t-sys: ~15
  externs pinned by `abi_check.c` (`_Static_assert`s over struct
  sizes/offsets/constants), a tiny `shim.c` keeping the variable-layout
  `sqfs_compressor_config_t` C-side, and a Rust-side memory-backed
  `sqfs_file_t` (mirrors the C++ one). Installed via vcpkg with the SAME
  overlay port libtfs uses (squashfs-tools-ng 1.3.2, gzip/lz4/xz/zstd
  decompressors) — version parity on both sides of the gate. Feature flag
  `vendored-squashfs` (default on). squashfs-tools-ng is POSIX/autotools
  only (no Windows — same restriction as the C++ side).
- **Contract parity audit** ([docs/parity.md](docs/parity.md)): every
  C-ABI-visible C++ test group ported (CApiTest 79/79, CApiMultiMountTest
  18/18, CApiOffsetTest 11/11, abi version 1/1, squashfs 20 through the
  ABI, extraction 11), the remainder consciously not ported with reasons
  (C++-internal types, path utils, tebako-pkg binary functionality,
  perf/large fixtures).
- **Extraction parity**: `tebako_fs_extract_all` now preserves mtime as
  well as permissions (best effort); ported the C++ `ExtractionTest`
  mappable cases incl. sqfs permissions/empty-dir and spaced paths.
- **Contract suite: 164 tests** (64 zip + 18 multi-mount + 22 io-surface
  + 12 dwarfs + 20 squashfs + 9 extraction + 1 C harness + 17 tpkg
  + 1 unit).

### SHIPPED (milestone 2)

- **tfs: dwarfs backend via the external
  [`dwarfs-rs`](https://github.com/tamatebako/dwarfs-rs) crate** (crates
  `dwarfs-t`/`dwarfs-t-sys`; currently a *path* dependency — becomes a git
  dependency when dwarfs-rs has a release, crates.io later). Mount from
  file / memory / region (offset+length — the C++ semantics: dwarfs
  regions open in place, zip regions read into memory), stat, pread,
  directory iteration. The milestone-1 ENOTSUP cases are now real: dwarfs
  images mount with backend name "DwarFS" and pass the ported
  `CApiOffsetTest` suite; squashfs stays ENOTSUP. Feature flag:
  `vendored-dwarfs` (default on; `--no-default-features` keeps the build
  pure-cargo with dwarfs mounts failing ENOTSUP).
- **tfs: multi-mount** — `tebako_mount_t` handles (monotonic, never
  reused), `tebako_fs_mount_from_file` / `_from_file_at` / `_from_memory`,
  `tebako_fs_unmount_handle` (force-closes only the mount's own fds/dirs →
  EBADF, releases the mount point), longest-prefix dispatch (nested mounts
  shadow outer ones), legacy `init*` single-mount semantics on top (EEXIST
  when anything is mounted, `tebako_fs_unmount()` tears down everything),
  extract rules (1 mount → dest root; N mounts → per-mount
  mount-point-basename subtrees).
- **tfs: remaining io surface** — `tebako_fs_dir_is_embedded`,
  `rewinddir`, `telldir`/`seekdir` (index-based cookies),
  `tebako_fs_extract_all`, `tebako_fs_dlmap2file` (per-process tmpdir,
  memfs-path→host cache keyed by full path, caller-`free()`d returned
  string, teardown removal at exit), and `tebako_fs_abi_version()`
  (== `TEBAKO_FS_ABI_VERSION` = 1, byte-for-byte with libtfs main
  0f3e444).
- **Contract suite: 116 ported cases** — 64 zip (M1) + 18 multi-mount +
  22 io-surface + 12 dwarfs offset/backend, plus the plain-C harness.

### SHIPPED (milestone 1)

- **`crates/tpkg` — complete.** Parse / serialize / validate / crc32 for
  the tpkg v1 trailer, byte-exact with the reference C implementation
  (`include/tebako/tpkg.h` in libtfs): golden vectors generated by that C
  implementation (field offsets asserted), absent-vs-corrupt
  discrimination (`TEBA` prefix → `NoTrailer`, broken magic → `Magic`,
  bad crc → `Crc`), bounds/version/slot-count/validation errors, and
  proptest round-trips (encode→parse identity, parser never panics on
  garbage). Error codes and strings are 1:1 with `TPKG_ERR_*` /
  `tpkg_strerror()`.
  *cbindgen note:* the generated C `tpkg.h` target lands with
  `crates/tebako-bootstrap` (item 22), the first real C-side consumer.
- **`crates/tfs` — the `tebako_fs_*` C ABI core.** Thread-local errno
  channel, `FsContext` mount table + fd/dir tables, longest-prefix
  dispatch, `Backend` trait, ZIP backend (pure-Rust [`zip`] crate — the
  ABI contract is behavioral, not library-level; no native libzip).
  Exported symbols: exactly the `tebako_*` functions (nm-verified; nothing
  else leaks).

### PLANNED (next milestones, in order)

1. `crates/tfs-cli` (generic image ops: mkimage/ls/cat/tree/stat/extract/
   find — the non-trailer half of the C++ tebakofs).
2. `crates/tebako-cli` (item 17), `crates/tebako-bootstrap` (item 22,
   size-gated < 2 MB), cbindgen `tpkg.h` with it.
3. crates.io publication of `tpkg`/`tfs` (after the API settles
   post-parity).

### v2 notes (recorded decisions)

- **Byte paths**: v1 validates path arguments as UTF-8 at the FFI boundary
  (non-UTF-8 → `EINVAL`). Ruby integration will need byte paths (`OsStr`
  on POSIX — ruby paths are raw bytes, not guaranteed UTF-8). The v2 C API
  revision is where this lands; the errno surface does not change.
- **ZIP mtime**: interpreted as UTC (ZIP DOS timestamps carry no zone);
  libzip (C++ libtfs) uses process-local time. The contract never compares
  mtimes; documented in `backends_zip.rs`.

## Parity-oracle approach

`tebako_fs_*` and `tpkg` are **byte-level contracts** (locked). Tests come
from the C++ side and run unchanged against the Rust implementation:

- C ABI: the C++ `tests/test_c_api.cpp` cases are ported against the same
  fixtures with the same expectations (errno values included); the full
  493-test suite follows the same pattern (item 19 acceptance gate).
- tpkg: golden vectors are bytes produced by the reference C
  implementation, asserted offset-by-offset; a mismatch means the contract
  moved and both sides must move together.
- The dwarfs backend binds `dwarfs_c_*` via `dwarfs-rs`; never a
  workspace-local dwarfs-sys.

## Building

```console
$ export DWARFS_RS_VCPKG_ROOT=/path/to/vcpkg   # dwarfs + squashfs backends
$ cargo test --workspace    # tpkg + tfs + contract (incl. the C harness)

$ cargo test -p tfs --no-default-features   # pure-cargo, no vcpkg
```

The native backends come from vcpkg: dwarfs-t via dwarfs-rs's CMake
submodule build (env `DWARFS_RS_VCPKG_ROOT`) and squashfs-tools-ng via
sqfs-sys's manifest+overlay port (env `SQFS_SYS_VCPKG_ROOT`, falling back
to `DWARFS_RS_VCPKG_ROOT` / `VCPKG_ROOT`). First cold builds compile the
native dep chains; the vcpkg archive cache makes later builds fast.
Without default features the workspace is pure-cargo. CI: ubuntu-24.04 +
macos-14 with the dwarfs-t vcpkg baseline pinned plus an archive cache
keyed on both manifests, and a separate `--no-default-features` leg.

## License

BSD-2-Clause (same as the tebako C++ project). Note the DwarFS backend
links dwarfs-t / DwarFS, which is GPL-3.0 — see the dwarfs-rs README for
the plain statement of what that means for binaries.
