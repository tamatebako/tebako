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

**Runtime floor for the limnifs default (locked 2026-08-22):** with
limnifs the default writer format (§6), a default-pressed payload runs
on any runtime whose driver ships `backend-limnifs` — every runtime
built from the tebako product repo at or after the backend's merge
(shipped in tebako v0.2.0). The published tebako-runtime-ruby POSIX
assets meet the floor from the 2026-08-11 re-cut onward (v0.16.3 and
v0.16.4 assets alike, probed end-to-end: press → stitch → cold run of a
sha256-verified limnifs payload on a fresh store, entrypoint output
correct on both) **provided the payload observes the writer constraints
below**; windows-ucrt64 assets join with the first factory build whose
product pin carries the windows per-target override's
`backend-limnifs`. A runtime below the floor fails closed with the
named `ENOTSUP` — by design; the loader never re-routes a limnifs
payload to another format.

**Writer constraints at the floor (locked 2026-08-22; upstream state
verified at tag `limni-v0.2.54`):** five defects in the limnifs crates
and their codec stack bound what tebako writers may emit while the
floor spans pre-0.2.53 readers. All five were pinned empirically
against the published drivers (the named reason rides the
`TEBAKO_DEBUG=trace` log; the errno channel carries EINVAL/EIO). Since
the lock: **#186 and #187 are FIXED upstream in 0.2.53** (verified in
the 0.2.54 source), **#315 is mitigated writer-side** in 0.2.53 (the
decoder defect itself is still open on omnizip 0.16.78), and **#188 is
not reproducible on stock 0.2.54** (retest below). The constraints stay
verbatim while the floor's readers predate the fixes — the recipe is
cheap, and every constraint's lift condition is named. The five map
onto four constraints below — the fifth (the tournament coupling)
shares constraint 4's remedy:

1. **No shared inline table** (upstream: limnifs#186 — FIXED in limnifs
   0.2.53: `INODE_FLAG_RESERVED_MASK = 0xF0`, bit 3 documented as the
   defined `SHARED_INLINE`). limnifs-core < 0.2.53 declares
   `INODE_FLAG_SHARED_INLINE = 0x08` (its own parser consumes the flag)
   yet sets the reserved mask to `0xF8` — covering bit 3 — so every
   reader of that line rejects any inode using the writer's inline
   dedup ("reserved flag bits set"). The dedup fires whenever two or
   more files at or below the 4096-byte inline threshold share content
   — i.e. on every realistic app tree. The published 0.16.3–0.16.5
   drivers all embed limnifs-core < 0.2.53, so tebako-written images
   MUST NOT set flag 0x08 while they are the floor: both writer entry
   points run with `defaults.shared_inline = false` (the metadata blob
   is whole-blob compressed, so re-inlined duplicate blobs cost nothing
   on the wire). The knob is limnifs#189 — SHIPPED in limnifs 0.2.57
   (`WriteConfig::defaults.shared_inline`, serde-defaulted true); the
   limnifs pins ride crates.io semver and the workspace
   `[patch.crates-io]` fork block is deleted. **Lift condition:** the
   floor's readers embed limnifs-core ≥ 0.2.53 — i.e. a factory
   runtime cut built on tebako ≥ v0.2.2, the tebako release whose link
   unit first carries limnifs-core ≥ 0.2.53 (runtime 0.16.6 does NOT
   lift anything: its link unit comes from tebako v0.2.1, which embeds
   limnifs-core 0.2.51).
2. **The metadata blob is lz4-HC — never brotli, never zstd, never
   store, and (for size) never fast lz4.** The 0.16.3-era reader's
   brotli decode path fails on metadata blobs beyond the small-buffer
   case ("invalid code-length code lengths (space not consumed)") —
   small trees pass, which is exactly how the defect hid behind the
   first probe. zstd is out too (upstream: omnizip-rs#315, STILL OPEN
   on omnizip 0.16.78): the omnizip-zstd decoder shipping in every
   limnifs-core line on the floor (≤ 0.2.54, tebako's own tfs included)
   mis-decodes some valid frames — a deterministic frame-checksum
   mismatch on bytes libzstd itself accepts, reproduced by a 318-byte
   metadata blob at the Fastest/Fast/Default/Better levels. Writer-side
   this is mitigated since limnifs 0.2.53: `codec::zstd`'s
   `verify_roundtrip` decompress-verifies every encoded frame and
   refuses to emit one its decoder can't read (the tournament falls
   through), so a ≥ 0.2.53 writer never ships the landmine; the recipe
   keeps zstd banned regardless — every floor READER still carries the
   broken decoder for foreign or pre-fix frames, and lz4-HC already
   meets the size budget. lz4-HC (codec 0x13) is the safe high-ratio
   codec: its frames are standard lz4 blocks, and every floor reader
   dispatches 0x13 to the SAME fast-lz4 decoder (limnifs-core's
   `Lz4HcCodec::decompress` delegates to the fast codec — no second
   decode path exists), while the hash-chain match finder keeps a
   realistic tree's blob under the readers' 1 MiB compressed-inline
   ceiling where fast lz4 does not (the native-extension e2e tree:
   830 KiB lz4-HC vs 1049 KiB fast lz4 vs 2.5 MB store; the fast-lz4
   blob overshoots constraint 3's threshold, the store blob overshoots
   the readers' hard ceiling). Both writer entry points (`tfs mkimage`,
   `tebako press`) pin `metadata_codec = "lz4-hc"`.
3. **The writer inlines metadata up to the readers' 1 MiB ceiling**
   (upstream: limnifs#187 — FIXED in limnifs 0.2.53: the
   `defaults.metadata_externalize_threshold` knob, serde-defaulted to
   just under the reader ceiling (1000 KiB = 1 MiB − 24 KiB) and
   clamped at assembly). Both floor readers bound the compressed
   inline metadata at `DEFAULT_INLINE_METADATA_MAX_BYTES = 1 MiB`;
   limnifs-write ≤ 0.2.51 externalized past its own 768 KiB threshold
   with no `WriteConfig` override. On ≥ 0.2.53 the stock default IS the
   floor-safe value, so the recipe sets no override. A tree whose
   lz4-HC blob exceeds even the ceiling fails press/mkimage with the
   named self-contained error — the documented "too large for this
   format today" boundary.
4. **Content drops ride lz4-or-store, never brotli, never zstd.** The
   same two codec defects cover content drops, not only the metadata
   blob: a brotli-compressed text drop beyond the small-buffer case
   reads back EIO on the 0.16.3 driver (38–92 KB `.rb` files reproduce
   it; v0.16.4's reader is unaffected), and any zstd drop can hit the
   omnizip decode landmine on a floor reader (a stock-0.2.54
   zstd-binary probe image EIOs on the v0.16.3 driver — retest
   artifact, limnifs#188 comment). The fifth defect sealed the recipe
   from the other direction (upstream: limnifs#188): on 0.2.51,
   removing lz4 from the compression tournament while `binary_codec`
   stayed lz4 made the writer emit a binary drop that every reader —
   tebako's own tfs included — read back as **zero bytes with a
   successful stat** (a 68 KB `.bundle` reproduced it; the runtime
   symptom was a `LoadError` on the dlmap2file path). **Retest
   2026-08-22 on stock, unpatched 0.2.54 — both exact trigger configs,
   three readers (0.2.54 tfs, both published drivers): NOT
   REPRODUCIBLE, every readback byte-exact** (reported on limnifs#188;
   fixed-where unbisected — the 0.2.53 zstd self-check and the omnizip
   0.16.78 OF-table fix are the candidates). The tournament restriction
   stays regardless: the decode defects above already forbid every
   non-lz4 codec on the floor, and the pin costs nothing until #188
   closes with a named root cause. Both writer entry points pin
   `text_codec`/`binary_codec` to lz4 and restrict the compression
   tournament to `store` + `lz4` — lz4 present in the list, nothing
   else beside store. lz4 decode is proven on the floor for both text
   and binary drops (94 KB `.rb`, 68 KB `.bundle`).

A payload written outside these constraints is not rejected by the
tooling — it is simply below the floor's guarantee: it may mount on
newer readers and fail closed (named EINVAL) on older ones.

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
