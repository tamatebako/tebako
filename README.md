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
    tebako-cli/        # the tebako packager CLI: lean/fat press, cache
                       # (item 17)
    tebako-http/       # in-process HTTPS downloads (ureq+rustls,
                       # webpki-roots bundled) shared by the CLI and the
                       # bootstrap
    tebako-bootstrap/  # the Rust bootstrap runner (item 22,
                       # < 3 MB static, size-gated)
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

### SHIPPED (milestone 8)

- **Runtime-as-image, consumer side (item 30b)**: the runtime splits into
  interpreter + `.tfs` image, and the launcher ABI learns the image era
  ([docs/runtime-as-image.md](docs/runtime-as-image.md) — the exact
  trailer/manifest/runtime_ref semantics). **`crates/tebako-bootstrap`**:
  a runtime_ref carrying the bare **`;image` flag**
  (`ruby@<rv>;tebako=<ver>[;image][;sha256=<hex>]`) resolves the
  `<asset>.tfs` alongside the executable — download via tebako-http
  (manifest `image` key primary, SHA256SUMS line fallback), sha256
  verified against the release index, installed **read-only with
  `<image>.sha256`/`<image>.origin` trusted markers** into the same cache
  entry, never extracted — and exports `TEBAKO_RUNTIME_IMAGE=<path>` to
  the runtime (the v1 handoff is byte-identical; refs without `;image`
  change nothing). **`crates/tebako-cli`**: the index parsers learn the
  additive image entries; press resolves the image into the cache
  (bootstrap interop), seeds the packaging environment by extracting the
  image **in-process through the tfs C ABI** (no `layout/` tree in the
  cache, no `--tebako-extract` for image-era releases), and emits the
  `;image` runtime_ref when the release carries an image entry (v1-era
  refs stay byte-identical, golden parity intact). Proven end-to-end:
  agent-123's published images (mirror mode) and a build-matched pair
  manufactured from the official 0.15.9 runtime both press and cold-run
  (the cache holds interpreter + image + markers only), and the current
  driver mounts the standalone image and executes its stub with **zero
  driver change**; the image-era driver patch
  ([docs/tebako-main.cpp.30b.patch](docs/tebako-main.cpp.30b.patch) —
  prefer `TEBAKO_RUNTIME_IMAGE` over the incbin image) is upstream's
  only change, needed only for runtimes that stop embedding.
- **Fixture note**: the locally-built 0.15.9 runtime in
  `tebako-runtime-ruby/runtime-packages/` is not the released binary
  (sha differs; its boot fails default-gem loading even from its own
  image — `Gem.home` undefined, bundler's ProcessLock lands on the
  read-only memfs). The gemfile scenario of the image-era e2e therefore
  runs against the build-matched official pair; the simple scenario and
  all mount/resolution proofs run against agent-123's pair.

### SHIPPED (milestone 7)

- **`crates/tebako-cli` — the `tebako` packager CLI** (item 17's
  SELF-HOSTING design): a port of the reference gem's lean/fat press
  (tebako-chainwt `lib/tebako`). `tebako press -r <root> -e <entry>
  [-o pkg] [-p prefix] [-c cwd] [-R ruby] [-m lean|fat] [--image
  path:mount]...` resolves the prebuilt runtime into the shared cache
  (gem-identical `~/.tebako` layout, flock'd installs, manifest.json /
  SHA256SUMS index, TEBAKO_OFFLINE / mirror env vars, error codes
  120–125), seeds the packaging environment from the runtime's extracted
  layout, deploys the application **under the runtime itself** (the
  stub-driver image stitched onto an empty base and exec'd via
  `--tebako-image`, with the RUBYOPT/RUBYLIB/BUNDLE_* env scrub), images
  the app and stitches a three-part package (LEAN flag, launcher ABI 1,
  `ruby@<rv>;tebako=<v>` runtime_ref, fat mode adds the FORMAT_RUNTIME
  payload slot + `;sha256=`). Scenarios: simple script and Gemfile
  (lockfile-pinned bundler per `BUNDLED WITH`; Gemfile `ruby` directive
  honored). `tebako cache list` / `cache prune [--all] [--older-than Nd]`
  manage the runtime cache. **Everything in-process (owner rule)**: all
  downloads via `crates/tebako-http` (ureq + rustls, webpki-roots
  bundled, HTTPS-only, `file://` mirrors, OS trust opt-in via
  `TEBAKO_TLS_PLATFORM_ROOTS` — no curl anywhere) and all images via the
  dwarfs-t `Writer` binding (dwarfs-t-rs) — no mkdwarfs binary, no PATH
  lookup; produced images carry dwarfs-t-native FlatBuffers metadata and
  are named `.tfs` (`fs.tfs`, `deploy-driver.tfs`; `.dwarfs` stays for
  upstream-compatible images). tfs-cli's `mkimage` moved to the same
  Writer. **Golden parity vs the gem**: same fixture, same prefix — the
  press stdout and the packaged binaries' output are byte-identical (CI
  leg checks out `tamatebako/tebako` feat/ci-slim-model and diffs
  side-by-side; the RuntimeSdk provisioning lines and the two sides'
  image-build lines are filtered — see the crate README).
  **Documented deviations**: the bootstrap portion defaults to the
  in-workspace Rust tebako-bootstrap (`--bootstrap`/`TEBAKO_BOOTSTRAP`
  override; otherwise the C++ release is resolved like the gem does); the
  RuntimeSdk/src-release subsystem is not ported (pure-ruby bundler flows
  never need it — native-extension deploy is a later milestone), and
  neither are the gem/gemspec scenarios, classic mode, `.tebako.yml`, or
  the gem's unconditional 5 s press pause (kept only when a warning is
  actually printed); images stitch densely (tpkg slots carry absolute
  offsets; the gem's 8-byte padding is cosmetic).

### SHIPPED (milestone 6)

- **`crates/tebako-bootstrap` — the Rust bootstrap runner** (item 22): a
  port of the C99 `tebako-bootstrap.c` v0.2.0 contract. Trailer parse
  (crates/tpkg), launcher-ABI check (v1), runtime_ref parse
  (`type@version;tebako=<abi>[;sha256=<hex>]`), shared-cache hit, **fat**
  payload extraction from its own executable (SHA256-verified against the
  `;sha256=` parameter), **lean** download from the release mirror
  (manifest.json primary, SHA256SUMS.txt fallback; `TEBAKO_OFFLINE` cache
  mode; `TEBAKO_RUNTIME_MIRROR`), per-entry install lock (flock, 120 s
  timeout with the stale-lock hint), atomic install (tmp + rename,
  sha256/origin metadata), and the launcher-ABI-v1 exec handoff
  (`--tebako-image <self>:<slot>:<mount>` … `--tebako-entry <argv0> <user
  args…>`; runtime payload slots never handed over; `--tebako-extract` is
  a runtime-side option that rides the passthrough like the C++). Named
  exit codes 65/66/67/69/70/74 with the C++ message bodies. The ten
  `self-test.sh` scenarios ported as integration tests, plus a direct
  parity run of the same fixtures against the C++ oracle binary
  (`TEBAKO_CPP_BOOTSTRAP`).
- **SIZE — the 3 MB gate (item 22, hard gate; budget raised 2 MB → 3 MB
  by the owner when the download path went in-process)**: release profile
  `opt-level="z", lto="fat", codegen-units=1, panic="abort",
  strip="symbols"`; no async runtime, no clap. Measured:

  | platform | Rust bootstrap | C++ v0.2.0 | budget |
  |---|---|---|---|
  | macOS arm64 | **1,238,384 B (1.18 MB)** | 53,536 B | < 3 MB ✔ (2.5× under) |

  (CI publishes the per-platform artifact sizes on every run and fails
  at ≥ 3 MB.)
- **HTTP/TLS choice (owner rule: no curl anywhere)**: all downloads are
  in-process via `crates/tebako-http` — ureq + rustls (ring) with
  webpki-roots **bundled** (the OS trust store is opt-in via
  `TEBAKO_TLS_PLATFORM_ROOTS`), HTTPS-only, redirects ≤ 5, `file://`
  mirrors. Size cost in the artifact: **≈ +867 KB** vs the previous
  curl-CLI path (371,776 B → 1,238,384 B on macOS arm64) — the owner
  extended the budget 2 MB → 3 MB to pay for it. The C++ bootstrap
  shells out to curl; the Rust side no longer does anywhere.
  Windows exec/lock ports land with the Windows CI leg (v1 ships
  macOS/Linux like the rest of the matrix).

### SHIPPED (milestone 5)

- **`crates/tfs-cli` — the `tfs` binary, the generic VFS image tool** (item
  25's tfs : libtfs :: sqlite3 : libsqlite3; the image-operation half of
  the C++ tebakofs — trailer surgery stays in tebako-pkg). Exact C++
  output/exit-code/stream-split parity, verified against the tebakofs
  oracle: `info` (plain-archive summary; `--json` for backend metadata
  JSON — item 24's image_info_json on dwarfs), `ls`/`ls -r`/`ls -l`,
  `tree`, `cat` (pread-chunked, no full materialization), `stat`,
  `extract` (whole-archive via `tebako_fs_extract_all` with mtime
  preservation + selected paths with the stderr warning split), `find`
  (fnmatch), `mkimage` (shells out to mkdwarfs via `TEBAKO_MKDWARFS`/PATH
  — **the dwarfs WRITER is deliberately not bound; binding it is a
  separate future decision**, like the tebako-pkg golden leg CI downloads
  the released mkdwarfs/tebakofs binaries per platform).
  Contract discovery folded in: the zip backend is **explicit-entry-only**
  through the C ABI (implicit parents of deeper entries list nothing and
  ENOENT — verified 1:1 against the oracle), and ZIP DOS timestamps are
  interpreted in the local timezone (libzip semantics).

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

1. crates.io publication of `tpkg`/`tfs` (after the API settles
   post-parity); cbindgen `tpkg.h` with it.
2. tebako-bootstrap Windows exec/lock port + windows CI leg (v1 shipped
   macOS/Linux); tfs-cli interactive shell + serve/exec modes (later).
3. tebako-cli: classic press mode, gem/gemspec scenarios, the
   RuntimeSdk/src-release subsystem (native-extension deploy), .tebako.yml.

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
