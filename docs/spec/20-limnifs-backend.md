# Spec 20 — The LimniFS backend (image format 5)

Normative specification of the LimniFS image backend: the `format_id`
allocation, the detection magic, the adapter contract against the spec 11
backend seam, the compile-separation feature family, and the writer path.
**Status: PLANNED** (post-v2.0.0; work queue `TODO.prepublish/09`) —
nothing on this page is shipped. The page LOCKS the integration shape so
the implementation lands once, in the owning crates, with no design
re-litigation. Provenance: the LimniFS team's integration proposal
(2026-08-01, `limnifs/docs/tebako-integration-proposal.md`), answered
with a spec per spec 14's design-first order.

LimniFS is a pure-Rust (`#![forbid(unsafe_code)]`), content-addressed
image format: every chunk (a **drop**) is identified by
`BLAKE3(plaintext)`, compressed per content class, and read on demand
from any byte slice. Two laws bind it exactly as they bind dwarfs-t:

- **Orthogonality law** (spec 00, spec 02 §6): `format_id` 5 answers only
  "how do I read these bytes". Runtime-role and entrypoint stay L2
  manifest semantics — nothing about limnifs changes what a payload IS.
- **Transforms law** (spec 00 invariant 5, spec 11 §4): the backend is
  read-only at runtime forever, plus a creation-time writer. COW and ENC
  stack above it unchanged; limnifs never learns to write in place.

## 1. Position

LimniFS is an **additive backend** behind the spec 11 §3 backend seam —
the sixth in-tree format (after dwarfs, squashfs, zip, the tar family),
changing nothing for existing packages, runtimes, or payloads. Its
strategic value is the compile story: the dwarfs backend pulls a C++
toolchain (vcpkg/cmake per target); `limnifs-core` compiles everywhere
Rust compiles with no system dependencies, which makes a **pure-Rust
runtime distribution** possible for the first time (§5).

The runtime-side flow is untouched. The bootstrap never mounts and never
links TFS (`crates/tebako-bootstrap/src/lib.rs`: it reads the trailer via
`tpkg`, skips runtime-role slots, and hands `--tebako-image` triples to
the runtime — spec 17 §1). The driver inside the runtime exe mounts every
slot image through the same detection chain and mount constructors as
today. A limnifs payload is one more byte shape arriving at that seam.

## 2. `format_id` allocation: 5 = limnifs

- **`TPKG_FORMAT_LIMNIFS = 5`** — the next free id after 0 auto / 1
  dwarfs / 2 squashfs / 3 zip. **4 stays the legacy role wart** (spec 02
  §6): never reused, never mounted, never given new meaning. Ids ≥ 6
  remain unallocated; the format axis never carries roles again.
- The allocation lands in `crates/tpkg` (`TPKG_FORMAT_*` constants,
  `src/lib.rs`) with the structural-validation bound raised to admit 5
  (`Slot` validation today rejects `format_id > TPKG_FORMAT_RUNTIME`).
- **Documented deviation** (spec 00 invariant 8): the reference C99
  `include/tebako/tpkg.h` (v1 bootstrap repo) keeps its bound at 4 and
  rejects a format-5 slot with its NAMED invalid error. That is
  fail-closed and correct — a v1-era reader cannot read limnifs bytes, so
  it refuses by name rather than misreading. Parity ends exactly at the
  new id; the Rust `tpkg` golden vectors are extended, not altered.
- The hint surfaces follow: `tebako-info`'s `hint_name(5)` → `limnifs`;
  the detection-derived label (spec 15) is `limnifs` via the backend's
  `name()`. The trailer's `format_id` stays a HINT — magic detection is
  authoritative (spec 02, `0 = auto`; §3 below).

## 3. Detection: the `LMFS` magic

A limnifs image opens with the 16-byte manifest header; its first four
bytes are the manifest magic:

```
offset 0: "LMFS" (0x4C 0x4D 0x46 0x53)   — MANIFEST_MAGIC, image start
```

`detect_format` (`crates/tfs/src/backend.rs`) gains the limnifs arm in
the strong-magic group, per spec 11 §3's rule (strong magic first, weak
heuristics last). Probe order, locked:

1. zip (`PK\x03\x04` / `PK\x05\x06`)
2. dwarfs (`DWARFS`)
3. squashfs (`hsqs`)
4. **limnifs (`LMFS`)** — disjoint 4-byte prefix against every arm above
5. tar envelopes (gzip `\x1f\x8b\x08`, zstd `\x28\xb5\x2f\xfd`)
6. the weak tar header-checksum heuristic — always LAST
7. `Unknown` → `EINVAL` at mount

Notes:

- The 512-byte sniff window is unchanged (the header is 16 bytes).
- `LIM1` (the SLAB magic) is a SECTION magic inside the image, never an
  offset-0 image magic — detection keys on `LMFS` only.
- A slot stamped `format_id = 5` whose bytes do not start with `LMFS`
  mounts by what the magic says (the hint never overrides detection); a
  mount of `LMFS` bytes on a build without the limnifs backend fails
  closed (§5) — never a silent re-route.

## 4. The backend contract (the adapter)

The adapter (`crates/tfs/src/backends_limnifs.rs`, pure safe Rust — the
`unsafe`-at-FFI-boundary rule never triggers, there is no FFI) maps
`limnifs-core`'s manifest/metadata/slab model onto the spec 11 `Backend`
trait. Mount-open parses, from the image byte slice (host file, file
region, memory, or VFS-file-region — spec 11 §5's four mount-source kinds
all serve one `&[u8]` core):

1. `ManifestCursor` → `parse_manifest_header` (16-byte header);
2. `parse_metadata_reference` → the metadata locator;
3. `parse_metadata_blob` → `MetadataBlob` (path → `Inode` resolution,
   explicit directory entries);
4. `SlabView` over the slab region (`parse_slab`) — index only, no
   upfront decompression.

Trait mapping (errno-valued errors, the C ABI convention):

| trait method | limnifs answer |
|---|---|
| `name()` | `c"LimniFS"` |
| `stat(path)` | `MetadataBlob` path resolution → `Inode` (mode/type/mtime/size). `ContentHandle` size: inline length for `InlineData`/`SharedInline`, summed `SliceRef` spans for `SliceMap` |
| `has_entry_or_children(path)` | the trait DEFAULT (the stat answer) — limnifs carries explicit directory entries like dwarfs/squashfs/tar, so the write gate's held-tree check (spec 11 §11) and the jail's covered-vs-held fallthrough (spec 11 §2, spec 08) behave identically to the dwarfs backend |
| `pread(path, buf, off)` | **inline drops** (`InlineData`, `SharedInline`): served straight from the metadata blob — no slab access, no decompression. **Slab drops** (`SliceMap`): only the drops intersecting the requested window are materialized, via `SlabView::plaintext_for` — on-demand, per-class decompression. Callers clamp to EOF; short reads allowed |
| `read_dir(path)` | the inode's directory handle → `parse_directory_node` → direct children (never `.`/`..`); `ENOTDIR` on a non-directory |
| `read_link(path)` | the `Symlink` content handle's target (limnifs has symlink inodes) — spec 11 §9 router semantics apply unchanged |
| `image_info_json()` | the backend metadata surface (sections, drop counts) — feeds `tfs info --backend-json` like the dwarfs backend |
| `writable()` | `None`, forever (transforms law) |

Error mapping — `limnifs-core`'s `CoreError` to the crate's errno channel,
named, never silent:

| `CoreError` | at mount-open | while serving |
|---|---|---|
| `TooShort`, `BadMagic` | `EINVAL` (not a limnifs image) | — |
| `Corrupt` (structural invariant, checksum) | `EINVAL` | `EIO` |
| `UnsupportedFeature` (flag id, section version, AEAD-sealed slab) | `ENOTSUP` naming the feature | `ENOTSUP` |
| path lookup miss | — | `ENOENT` |

Writes to a read-only mount fail `EROFS` at the mode gate, above the
backend — unchanged.

## 5. Compile separation: the backend feature family

Backends compile in per cargo feature; a consumer ships exactly the
formats it must read, and nothing else. This spec locks the `backend-*`
feature namespace in the `tfs` crate (migrating the current
`vendored-dwarfs` / `vendored-squashfs` features, which stay as aliases
for one release and retire):

| feature | backend | toolchain |
|---|---|---|
| `backend-dwarfs-t` | dwarfs-t (FlatBuffers) | C++ (vcpkg/cmake via dwarfs-t-rs) |
| `backend-squashfs` | squashfs | C (squashfs-tools-ng; POSIX-only today) |
| `backend-zip` | zip | pure Rust |
| `backend-tar` | tar (+ gz/zst envelopes) | pure Rust |
| `backend-limnifs` | limnifs | **pure Rust, `#![forbid(unsafe_code)]`, no system deps** |

The `enc` transform feature is orthogonal and unchanged (it stacks over
any backend). Per-product defaults:

- **tebako-cli, tfs-cli, tebako-pkg** (press + tooling): ALL backends —
  the packager and the human tool read and write every format the
  ecosystem ships.
- **tebako-driver (inside every runtime exe) and libtfs-preload** (the
  runtime-side mount surface): all backends, so any package runs on any
  runtime — the same rule as today, extended by limnifs.
- **The pure-Rust runtime line** (a later distribution, its own release
  line; recorded here as the goal): `backend-limnifs` only (zip/tar may
  ride at zero toolchain cost) — no dwarfs-t, no squashfs, no vcpkg/cmake
  anywhere in the build. This is the proposal's strategic point and the
  ecosystem's exit from per-target C++ cross-compilation for runtimes.
- **tebako-bootstrap**: NO backend, ever — it never mounts (§1); it stays
  format-agnostic and inside its size gate.

**Compiled-out rule (locked):** a mount whose detected format has no
compiled-in backend fails with the NAMED `ENOTSUP`, naming the format —
the squashfs-on-Windows precedent (spec 11 §3). Never a silent re-route
to another backend, never a partial read, never a crash. The driver
surfaces it as its own named error; a lean package pressed with
`--format limnifs` run against a limnifs-less runtime fails closed with
the backend named, not with a fallback mount.

## 6. The writer path

- **`tfs mkimage --format limnifs <srcdir> -o <img>`** — in-process via
  `limnifs-write` (`write_directory`), the same rule as the dwarfs-t
  Writer: no `limni` binary, no PATH lookup, no shell-out. The spec 03 §7
  `tree_hash` stamping is format-neutral (the staged hardlink mirror
  ahead of writer selection) and applies unchanged; output replacement
  keeps the mkdwarfs `--force` parity. `dwarfs` stays the default format;
  the unsupported-format named error lists the new supported set.
- **`tebako press --format limnifs`** (PLANNED flag; default `dwarfs`) —
  routes the packager's image build (`crates/tebako-cli/src/image.rs`)
  through the limnifs writer and stamps slot `format_id = 5` at the
  stitch sites (single press, suite press, deploy) in place of
  `TPKG_FORMAT_DWARFS`. Nothing else about press changes.
- **Format-neutrality of the manifest (orthogonality law):** the
  in-image payload manifest (spec 03) declares identity / provides /
  requires and NEVER names an image format; runtime-role stays out of
  the format axis (4 remains the wart); entrypoint resolution (spec 17
  §1) resolves against the first `--tebako-image` mount regardless of
  backend. Payload artifacts in the store stay `.tfs`-named and
  byte-identical with the registry artifact (spec 05) — `.lim` is the
  limnifs ecosystem's own extension, not the store's.

## 7. Roadmap hooks (recorded, NOT committed)

Each hook names its owning spec; none is a commitment of this spec.

- **BLAKE3 CAS ↔ spec 03 §7 `tree_hash`.** `tfs-merkle-1` (SHA-256,
  tpkg-owned) stays THE `tree_hash` for every backend. LimniFS's
  `ManifestRoot` (BLAKE3 over section hashes) is image-internal identity;
  a future spec-03 addendum may admit hash-algorithm agility. Recorded.
- **Merkle + Ed25519 ↔ spec 09.** Package-level trust stays the tpkg
  chain (trailer digests + OpenPGP). LimniFS image-internal signatures
  would be an additive inner evidence layer, not a replacement. Recorded.
- **ENC ↔ spec 10.** Tebako's `EncBackend` stacks over limnifs unchanged
  (transforms law). LimniFS's own AEAD representations (aead id ≠ 0) are
  an image-format concern: a mount of a self-encrypted image without key
  support fails `ENOTSUP` by name (§4). Tebako-side encryption stays the
  spec-10 transform. Recorded.
- **Remote locators (http / s3 / ipfs) → a future streaming spec.**
  Fetch-on-access, cache, and jail interaction are undesigned; the first
  adapter mounts LOCAL bytes only (§4's four mount-source kinds).
  Recorded.
- **Delta updates via content-addressed dedup.** `DropId` sharing across
  image versions makes `TODO.deferred/03` (payload delta updates)
  structural rather than bolted-on; its re-entry criteria are unchanged,
  as is the store's byte-identical rule. Recorded.

## 8. Parity and acceptance

**Golden expectations** (spec 00 invariant 8) — the dwarfs backend is the
parity oracle. Same tree in → same logical VFS answers out; byte-identity
is per-backend, semantics are shared (the backend-pair parity class of
`TODO.prepublish/09`):

- `stat`/`readdir`/`pread`/`read_link` semantics per §4, including mtime
  truncation (limnifs `mtime_ns` → `RawStat` seconds), direct-children
  readdir, EOF clamping, and the symlink rules of spec 11 §9.
- The write gate and jail gate behave identically
  (`has_entry_or_children` = the stat answer; covered-but-not-held paths
  fall through, policy-gated).
- Mount modes: RO default; COW stacks (backend-agnostic composite);
  RW → `ENOTSUP`.

**Done means** (one stacked change set, in the owning crates only):

1. `crates/tpkg`: `TPKG_FORMAT_LIMNIFS = 5`; the validation bound admits
   5 with the C99-reference deviation documented (§2); golden vectors
   extended. `crates/tebako-info`: hint and label surfaces (§2).
2. `crates/tfs`: `backends_limnifs` behind `backend-limnifs`; the
   detection arm (§3); the three mount constructors wired (§4); the
   `backend-*` feature family with per-product defaults (§5); the
   compiled-out `ENOTSUP` path tested.
3. Writers: `tfs mkimage --format limnifs` and
   `tebako press --format limnifs` (§6), with the named-error surfaces
   updated.
4. Contract + parity suites: the backend contract suite runs against
   limnifs; the backend-pair golden class vs dwarfs is green; no
   regressions in existing backends.
5. e2e: a payload pressed `--format limnifs` boots through the runtime
   driver and runs its entrypoint; the boot-smoke syscall surface passes
   against the limnifs-mounted payload; `tebako inspect` reports the
   format as `limnifs` and the hint as `5`.
