# Spec 11 — TFS: the virtual filesystem layer

Normative specification of the userland VFS. Status: core SHIPPED
(`crates/tfs`, contract suite 164 tests); COW/ENC/write-family PLANNED
(roadmap 12/13).

## 1. The model

TFS is a userland VFS over bulk-storage images — the kernel-VFS shape
(vfs → fs drivers → block layer) with image-in-file (or in-binary region)
replacing the block device. It is **self-similar**: images are files, any
file is mountable (including files inside other mounts — §5), mounts
compose and stack, and each process owns its namespace object (Plan 9
per-process namespaces, delivered as a library).

Product line: **libtfs** (the engine, `tebako_fs_*` C ABI — the Rust TFS
is the shipping implementation) · **tfs CLI** (human-facing:
`tfs : libtfs :: sqlite3 : libsqlite3`) · **tebako-pkg** (tpkg trailer
surgery — tebako's concept, not TFS's).

## 2. Mount table and dispatch

- N concurrent mounts: handles (`tebako_mount_t`, monotonic, never
  reused), each with mount point + backend instance.
- **Longest-prefix dispatch** (path-component boundary); nested mounts
  shadow outer ones; duplicate mount point → `EEXIST`.
- fd/dir tables record the owning mount; unmount-by-handle force-closes
  only that mount's fds/dirs (later use → `EBADF`).
- Legacy `init*` single-mount semantics layered on top (`EEXIST` when
  anything is mounted; `unmount()` tears down everything).
- Extraction rule: 1 mount → dest root; N mounts → per-mount
  mount-point-basename subtrees. Extraction preserves mtime + permissions
  (best effort).

## 3. Backends and capabilities (open/closed)

Backends are pluggable drivers; new formats are additive. Capability
model is honest per format — no uniform-RW pretense:

| format | RO | RW in-place | rebuild (creation-time) |
|--------|----|-------------|-------------------------|
| dwarfs-t | ✓ | — | dwarfs-t Writer (in-process) / `mkdwarfs-t` for humans |
| squashfs | ✓ | — | mksquashfs-class |
| zip | ✓ | ✓ (add/delete) | — |
| iso9660 | ✓ (PLANNED) | — | genisoimage-class |
| tar | ✓ (PLANNED) | append-only | repack |
| extN | ✓ (PLANNED) | ✓ (PLANNED) | mkfs |
| FAT | ✓ (PLANNED) | ✓ (PLANNED) | mkfs |

- Detection chain: strong magic first (zip EOCD, dwarfs, squashfs,
  iso9660 `CD001`@0x8001, ext `0xEF53`@0x438, FAT BPB), weak heuristics
  (tar) last, with per-candidate confidence; the claimed backend is
  reported.
- Mounts carry explicit mode flags: `TEBAKO_MOUNT_RO` (default),
  `_COW`, `_RW`. Writes on RO mounts → `EROFS`.

## 4. COW: the composite backend (transforms law)

`CowBackend { base: dyn Backend, overlay: dyn Backend }` — stacking, not
a format: reads fall through to base unless shadowed; writes/deletes/attr
changes land in the overlay; whiteouts (a small journal) hide base
entries. **Exists ONLY in the Rust TFS** — dwarfs-t and all backends stay
read-only (spec 00, invariant 5).

- First overlay: **HostDirBackend** (a host directory exposed as a TFS
  backend — independently useful) + whiteout journal. Disposable by
  deleting the dir.
- **Stackable, swappable, self-contained:** an overlay may itself be a
  CowBackend (layer stacks); an overlay may be a portable image file
  (detach, ship, re-attach); a base image may CARRY its overlay inside
  itself.
- **Write-side lifecycle:** writes target the fast store during use (no
  compression in the hot path); heavyweight compression is a
  POST-PROCESS at unmount or live: compress the overlay into a dwarfs
  layer (algo per content), JOIN/squash layers (resolving whiteouts), or
  CHECKPOINT (squash the overlay into a new RO layer beneath and reset —
  running snapshots). Export any layer/overlay/composite view to any
  output format via the Writer. The whiteout journal IS the audit delta.

## 5. Encapsulation (namespace closed under mounting)

Mount-source kinds: host-file, memory, file-region (offset+length), and
**VFS-file-region** — `tebako_fs_mount_from_vfs(vfs_path, mount_point,
…)` mounts an image addressed by a path INSIDE an existing mount; the new
backend reads its bytes through the owning mount's pread. "An FS includes
another FS at a directory path" is a primitive; every file in the
namespace is a potential filesystem.

## 6. Access-mechanism matrix (by transparency)

1. **link** — in-process via libtfs (fastest, deepest; the packaged-app model).
2. **fuse** — `tfs mount image /mnt --fuse` (PLANNED): real system-wide
   mount wherever FUSE exists; multi-backend + multi-mount (nested mount
   points appear as directories). The ONE mechanism with a host
   dependency.
3. **serve** — `--serve=nfs|webdav` (PLANNED): userland server; the OS's
   native client mounts it — no FUSE anywhere.
4. **shell** — interactive browse + one-shots (`tfs ls/cat/tree/stat`).
5. **exec** — `tfs exec image -- cmd` (PLANNED): VFS injected via an
   opt-in preload shim (a convenience, NOT the product ABI).
6. **extract** — materialize to disk (exists; the honest fallback).

Trade-off stated plainly: without the kernel there is no system-wide
transparent mount for unmodified arbitrary processes; TFS trades that for
zero privileges, every platform, in-process speed.

## 7. ABI surface (additive-only rule)

Current: mount/multi-mount family, stat/pread/dir family (incl.
`rewinddir`, `telldir`/`seekdir` index cookies, `dir_is_embedded`),
`extract_all`, `dlmap2file` (memfs path → host cache for dlopen),
`abi_version()` = 1. Write family (gated by mount mode): write/pwrite/
mkdir/rmdir/unlink/rename/chmod/utimens/truncate/fsync + mount-with-mode
entry points — ADDITIVE; RO-only consumers see zero change; abi version
bumps per spec 14. Exported symbols: exactly `tebako_*` (nm-verified).

## 8. libtfs for everyone

The C ABI is the product: ONE engine (Rust), ONE machine contract,
thin per-language adapters in their own ecosystems (SQLite/zlib/libgit2
model). Shared + static libs, `pkg-config`, embedding guide, per-backend
license statement (dwarfs-t: GPL-3.0; zip: BSD; squashfs-tools-ng:
LGPL-3.0 — backend selection changes a consumer's obligations; stated,
never softened).
