# Parity audit — C++ libtfs (493 tests) vs tebako-rs contract suite

As of milestone 3 (`feat/milestone-3`). The C++ suite was enumerated with
`ctest -N` in libtfs-pkgwt/build (493 tests). The Rust workspace runs
**164 tests**: 146 contract cases through the `tebako_fs_*` C ABI
(64 zip + 18 multi-mount + 22 io-surface + 12 dwarfs + 20 squashfs +
9 extraction + 1 plain-C harness) + 17 tpkg format tests + 1 unit test.

Verdict: every C++ test group that exercises the **`tebako_fs_*` C ABI**
(the machine contract) is ported; the remainder are C++-implementation
internals, and are listed as consciously-not-ported with reasons.

## Ported groups

| C++ group | cases | Rust ports | where |
|---|---|---|---|
| CApiTest | 79 | **79/79** | `tests/contract/tests/zip_c_api.rs` (lifecycle, memory, file ops, dir ops, metadata, path detection, errno, getters, integration) + `io_surface.rs` (pread block, dir positioning, dlmap2file block) |
| CApiMultiMountTest | 18 | **18/18** | `multi_mount.rs` (incl. nested-shadow longest-prefix, force-close isolation, per-mount registry/positioning, multi-mount dlmap2file, extract subtrees) |
| CApiOffsetTest | 11 | **11/11** | `dwarfs_backend.rs` (+ basic dwarfs mount/stat/pread/readdir, memory mount, backend name) |
| CApiAbiVersionTest | 1 | **1/1** | `io_surface.rs::abi_version_matches_header_constant` |
| SquashFSBackendTest / SquashFSBackendMountedTest / BackendFactorySquashFSTest | 60 | **20 through the C ABI** | `squashfs_backend.rs` — all C-ABI-visible behaviors (mount/identity, corrupted/missing, read/seek/pread/eof/close, multiple fds, stat/perms/mtime, dir listing incl. nested/empty, memory+region mounts, format dispatch) |
| ExtractionTest | 23 | **11** | `io_surface.rs` (single-mount, not-mounted) + `multi_mount.rs` (multi-mount subtrees) + `extraction.rs` (content/binary/empty/nested, dest creation, overwrite, memory mount, sqfs perms + empty dir, dwarfs, spaced paths) |
| BackendFactoryTest | 80 | **intent covered** | format-detection dispatch through `tebako_fs_init_*` for zip/dwarfs/squashfs + EINVAL/ENOTSUP paths (`zip_c_api.rs`, `squashfs_backend.rs::format_detection_dispatches_all_backends`) |
| DwarfsBackendTest / DwarfsBackendMountedTest / DwarfsIntegrationTest | 41 | **intent covered** | `dwarfs_backend.rs` (12 cases: plain/offset/memory mounts, read equality vs plain mount, stat/pread/readdir, "DwarFS" name) |
| Tpkg* (Read/Validate/RoundTrip/WriteFd/WireFormat/StrError/Crc32/C99) | 31 | **covered by `crates/tpkg`** | golden vectors from the C implementation, behavioral suite, proptests — the C mini-lib's own suite is the C++ side of the same format contract |
| UnifiedInterfaceTest | 7 | **intent covered** | interchangeable-backend behavior via the format-dispatch test + identical expectations across zip/sqfs/dwarfs contract cases |

## Consciously not ported (with reasons)

| C++ group | cases | reason |
|---|---|---|
| Package* (Bundle/Insert/RoundTrip/Mkimage/Cli/Remove/SetRuntime/Info/Spec) | ~40 | tebako-pkg **binary/trailer-surgery** functionality — belongs to the future `crates/tebako-pkg` (item 25), not to the `tebako_fs_*` ABI. The tpkg *format* contract itself is covered by `crates/tpkg`. |
| Path* (Normalization/Test/Relative/Property/Join/Component/Utility/String/Comparison) | ~28 | C++ path-utility internals — not part of the C ABI. The Rust side does its own minimal normalization inside backends; there is no consumer-visible contract to port. |
| ResultTest / ResultVoidTest / ErrorTest | 29 | C++ internal `Result<T>`/`Error` types — implementation details, not ABI. The Rust side maps to errno directly at the ABI boundary (covered by every errno assertion in the ported cases). |
| LRUCacheTest | 19 | C++ backend LRU-cache implementation detail — no ABI-visible contract. (The Rust backends currently build their lookup index per mount; a cache is a future optimization, not a parity item.) |
| BackendFactoryTest (direct factory-API cases beyond the intent set) | most of 80 | The C++ suite drives the `BackendFactory` C++ API directly (create by name, registry state, force-format). Not C-ABI-visible; the factory's observable behavior (magic dispatch, EINVAL/ENOTSUP) is ported. |
| ZipBackendTest / ZipBackendMountedTest / DwarfsBackend* / SquashFSBackend* (class-internal subset) | remainder of 60+41+~40 | C++ backend-class internals (constructor/unmounted-state transitions, iterator reset mechanics, per-backend FileHandle state, large-image perf benchmarks). The ABI-visible subset is ported; `large.dwarfs`/`large.sqfs` (~10 MB fixtures) and perf cases are skipped deliberately (size vs correctness trade-off, same as the C++ suite marking them perf). |
| ExtractionTest (remainder: LargeFileCorrect, perf, exhaustive metadata cases) | remainder of 23 | The 10 MB large fixture and C++-fixture-specific metadata exhaustiveness; the semantics (perms + mtime + content + structure) are covered by the ported cases on smaller fixtures. |
| UnifiedInterfaceTest (C++ adapter classes) | remainder of 7 | C++-internal unified-interface adapter, not the C ABI. |
| HelperTest / ChainingTest | 6 | C++ helper utilities / backend chaining internals — not ABI-visible. |

## Notes

- The plain-C harness (`tests/contract/c_harness/mount_read.c`) proves the
  C ABI from a C consumer; the parity suites above call the same
  `tebako_fs_*` symbols from Rust.
- The sqfs fixtures are borrowed from libtfs's BSD-2-Clause test data;
  the dwarfs fixture likewise. The 10 MB `large.*` fixtures are excluded
  by design.
- Symbol gate: `nm -gU libtfs.{dylib,so}` must show only `tebako_*`
  (34 symbols) — checked in CI by inspection of the build artifacts.
