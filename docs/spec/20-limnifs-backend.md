# Spec 20 — The LimniFS backend (image format 5)

Normative specification of the LimniFS image backend: the `format_id`
allocation, the detection magic, the adapter contract against the spec 11
backend seam, the compile-separation feature family, and the writer path.
**Status: SHIPPED** — the backend (reader + writer, `format_id` 5, the
`LMFS` detection arm, the `backend-limnifs` feature) landed in tebako
v0.2.0 (#371), and LimniFS is **the default image format** for
`tfs mkimage` and `tebako press` (§6; dwarfs stays first-class as a read
backend and an explicit `--format dwarfs` opt-in). Provenance: the LimniFS
team's integration proposal (2026-08-01,
`limnifs/docs/tebako-integration-proposal.md`), answered with a spec per
spec 14's design-first order.

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
4. per-slab header validation (`parse_slab_header` — magic, seal state,
   ordinal match); the slab regions are then handed to `SlabStore`
   behind `CachedSlabStore` (limnifs#192: SIEVE drop cache, 64 MiB /
   1024 entries, plus a 32 MiB seekable-frame cache) — index only at
   open, no upfront decompression.

Trait mapping (errno-valued errors, the C ABI convention):

| trait method | limnifs answer |
|---|---|
| `name()` | `c"LimniFS"` |
| `stat(path)` | `MetadataBlob` path resolution → `Inode` (mode/type/mtime/size). `ContentHandle` size: inline length for `InlineData`/`SharedInline`, summed `SliceRef` spans for `SliceMap` |
| `has_entry_or_children(path)` | the trait DEFAULT (the stat answer) — limnifs carries explicit directory entries like dwarfs/squashfs/tar, so the write gate's held-tree check (spec 11 §11) and the jail's covered-vs-held fallthrough (spec 11 §2, spec 08) behave identically to the dwarfs backend |
| `pread(path, buf, off)` | **inline drops** (`InlineData`, `SharedInline`): served straight from the metadata blob — no slab access, no decompression. **Slab drops** (`SliceMap`): only the drops intersecting the requested window are materialized, via `CachedSlabStore::decoded_range` — a seekable-container drop decodes only the covering 256 KiB frames; any other drop decodes whole once and caches (SIEVE, scan-resistant). tebako#464: a 19.5 MiB shim read in 8 KiB windows drove ~48 GiB of whole-drop re-decode pre-flip; post-flip each drop decodes at most once and the cache serves the rest. Callers clamp to EOF; short reads allowed |
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

**Runtime floor for the limnifs default (the flip line, tebako v0.3.0):**
with limnifs the default writer format (§6), a default-pressed payload
runs on any runtime whose driver ships the post-flip limnifs reader —
limnifs-core ≥ 0.3.0, i.e. a runtime built on tebako ≥ v0.3.0. The flip
is **atomic and incompatible** (limnifs#192; §8's format-evolution
proof): the 0.2.65/0.3.0 line replaced the image format without a
format-version bump, so the compatibility matrix is binary —

| writer \ reader | pre-flip reader (< 0.2.65) | post-flip reader (≥ 0.3.0) |
|---|---|---|
| pre-flip image (≤ tebako v0.2.x) | mounts | **EINVAL at mount-open** |
| post-flip image (≥ tebako v0.3.0) | **EINVAL at mount-open** | mounts |

Both directions fail CLOSED (named EINVAL, verified — never a
mis-read). The store resolves the right artifact by sha256, so a
stale-format cache entry is re-fetched by digest, never patched.
A runtime without `backend-limnifs` at all keeps the older named
`ENOTSUP`; the loader never re-routes a limnifs payload to another
format.

**Writer constraints (recipe pinned post-flip; the four historical
floor constraints LIFTED):** the pre-flip recipe (below) was shaped by
four defects in readers embedding limnifs-core < 0.2.53 / omnizip ≤
0.16.78 (limnifs#186/#187/#188, omnizip-rs#315 — all fixed or mitigated
upstream long before the flip). The post-flip reader floor (limnifs-core
≥ 0.3.0) clears every one of them. The recipe stays pinned regardless —
not from reader defects but for fleet uniformity and boot-time decode
cost (lz4 is the fastest-decoding codec in the stack), each item with
its retention reason; widening any of them is a measured decision of
its own, not a side effect of a dep bump:

1. **No shared inline table** — `defaults.shared_inline = false` at
   both writer entry points. Historical: pre-0.2.53 readers rejected
   inode flag 0x08 via their reserved mask (limnifs#186, fixed in
   0.2.53; the knob shipped in 0.2.57 as limnifs#189). Retained: one
   handle kind keeps the structural test trivial and the wire cost is
   nil (the metadata blob is whole-blob compressed, so duplicate
   inline blobs compress away). **Lifted**, i.e. safe to enable
   against any post-flip reader, when a measured tree shows the dedup
   win matters.
2. **The metadata blob is lz4-HC** — `metadata_codec = "lz4-hc"`.
   Historical: a 0.16.3-era reader's brotli path failed beyond the
   small-buffer case; the omnizip-zstd decoder (≤ 0.16.78) mis-decoded
   valid frames (omnizip-rs#315). lz4-HC (codec 0x13) dispatches to the
   same fast-lz4 decoder on every reader while the hash-chain match
   finder keeps a realistic tree's blob under the 1 MiB inline ceiling
   (fast lz4 and store both overshoot). Retained on its own merits.
3. **The writer inlines metadata up to the readers' 1 MiB ceiling** —
   stock since limnifs 0.2.53 (the
   `defaults.metadata_externalize_threshold` default, 1000 KiB, is the
   floor-safe value; limnifs#187). A tree whose lz4-HC blob exceeds the
   ceiling fails press/mkimage with the named self-contained error —
   the documented "too large for this format today" boundary.
4. **Content drops ride lz4-or-store** — `text_codec`/`binary_codec`
   pinned lz4, the compression tournament restricted to `store` +
   `lz4`. Historical: the two codec defects above (plus limnifs#188's
   tournament coupling, not reproducible since 0.2.54). Retained: lz4
   decode is the fastest in the stack, which is what a boot-path reader
   wants. Tebako-pressed images carry NO seekable containers today:
   the recipe's categorizer-less chunk path never sets the seekable
   flag (limnifs#195). The reader serves foreign containers
   frame-bounded; tebako#464's bounded cost on tebako-written images
   comes from the SIEVE drop cache (each drop decodes whole once and
   is cached — the repeated re-decode is gone; the per-drop cold
   window waits on the upstream chunk-path fix).

A payload written outside these constraints is not rejected by the
tooling — it is simply below the recipe's guarantee.

## 6. The writer path

- **`tfs mkimage [--format dwarfs|limnifs] <srcdir> -o <img>`** —
  in-process via `limnifs-write` (`write_directory`) for limnifs, the
  dwarfs-t `Writer` for dwarfs; the same rule for both: no `limni` /
  `mkdwarfs` binary, no PATH lookup, no shell-out. **`limnifs` is the
  default format** — the `--format` flag is optional and selects dwarfs
  only when given explicitly. The spec 03 §7 `tree_hash` stamping is
  format-neutral (the staged hardlink mirror ahead of writer selection)
  and applies unchanged; output replacement keeps the mkdwarfs `--force`
  parity. The unsupported-format named error lists the supported set
  (`dwarfs, limnifs`).
- **`tebako press [--format dwarfs|limnifs]`** (shipped in v0.2.0;
  **default `limnifs`**) — routes the packager's image build
  (`crates/tebako-cli/src/image.rs`) through the chosen writer and stamps
  the app-image slots with the matching `format_id` hint (5 limnifs / 1
  dwarfs) at the stitch sites (single press, suite press, deploy).
  Nothing else about press changes. Dwarfs remains a first-class
  explicit opt-in (`--format dwarfs`) and a supported read backend
  forever — existing dwarfs packages, runtimes, and payload artifacts are
  untouched.
- **Format-neutrality of the manifest (orthogonality law):** the
  in-image payload manifest (spec 03) declares identity / provides /
  requires and NEVER names an image format; runtime-role stays out of
  the format axis (4 remains the wart); entrypoint resolution (spec 17
  §1) resolves against the first `--tebako-image` mount regardless of
  backend. Payload artifacts in the store stay `.tfs`-named and
  byte-identical with the registry artifact (spec 05) — `.lim` is the
  limnifs ecosystem's own extension, not the store's.

## 6b. Rationale: why LimniFS is the default (normative)

The default image format is the one every payload takes when the operator
says nothing, so the default must be the format with the least toolchain
exposure, the least build cost, and no loss of capability. LimniFS meets
that bar and dwarfs-t does not; the reasons, in order of weight:

1. **The default path is pure Rust** (`#![forbid(unsafe_code)]`, zero
   system dependencies). Reading AND WRITING the default format needs no
   C++ toolchain, no vcpkg, no cmake — spec 00 laws 1/3 in their
   strongest form. The C++ toolchain exposure shrinks to opt-in formats:
   dwarfs stays first-class via `--format dwarfs`, and the dwarfs-t
   backend remains in every default feature set, so existing images read
   everywhere they read today.
2. **Build wall-clock and reproducibility.** The dwarfs-t vcpkg ports are
   the CI cost driver (per-target C++ cross-compilation); `limnifs-core`
   / `limnifs-write` compile everywhere Rust compiles and resolve from
   crates.io with semver-pinned versions — the default path never
   consumes an unpinned source tree. (CARVE-OUT LIFTED 2026-08-23: the
   limnifs#189 `shared_inline` knob shipped in limnifs 0.2.57, the
   limnifs deps resolve from crates.io semver-pinned at 0.2.57, and the
   workspace `[patch.crates-io]` block that redirected `limnifs-write`
   to the `tamatebako/limnifs` `tebako-floor-gate` fork is deleted.)
3. **Content-addressed storage.** Every drop is `BLAKE3(plaintext)`:
   image-internal integrity, dedup across and within images, and the
   structural delta-update story (§7) come from the format, not from
   bolted-on machinery.
4. **Read efficiency.** Decompression is on-demand and per content class;
   inline drops serve small files straight from the metadata blob with no
   slab access (§4).
5. **Unambiguous detection.** The strong `LMFS` magic is disjoint from
   every existing probe arm (§3) — no heuristic, no misclassification.
6. **Nothing else changes.** The orthogonality law holds (`format_id`
   answers only "how do I read these bytes"); payload artifacts stay
   `.tfs`-named and byte-identical with the registry artifact (spec 05);
   existing dwarfs packages and runtimes are untouched; a limnifs payload
   on a limnifs-less runtime fails closed with the named `ENOTSUP` (§5) —
   never a silent re-route to another writer's format.

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
- **Format-evolution proof:** limnifs 0.3.0 evolved the image format
  **incompatibly in place** (limnifs#192 — the seekable-container
  layout IS the v1 format now; there is no "v2". Readers on the new
  line reject pre-flip bytes at mount-open, verified EINVAL, never a
  mis-read). Tebako rides the flip **atomically**: one release (v0.3.0)
  moves the reader and both writers to the new line together, and every
  cached artifact's trust anchor stays its sha256 sidecar — a
  stale-format cache entry fails closed at mount and is re-fetched by
  digest, never mis-read. The committed fixture pair pins both sides:
  `limnifs-preflip.lmfs` (FROZEN pre-flip bytes; the test asserts
  fail-closed at mount-open — its generator is retired because the
  pinned writer can no longer emit the old layout) and
  `limnifs-slab-v1.lmfs` (the v1 layout; regenerates with the pinned
  writer, and every reader change MUST keep mounting it with identical
  `stat`/`readdir`/`pread` answers). A future incompatible evolution
  repeats this doctrine: frozen old fixture + fail-closed test + new
  live fixture + atomic release.

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
