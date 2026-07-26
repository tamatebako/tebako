# Spec 02 — tpkg wire format (L0)

Normative byte-level specification of the tebako package container.
Source of truth: `crates/tpkg` (byte-exact with the reference C99
`include/tebako/tpkg.h`; golden vectors pin the layout). All v1 integers
little-endian; the trailer header sits at EOF at fixed size. The container
knows **nothing** about runtimes, entrypoints, or languages — it carries
bytes, slots, mounts, and a resolution hint. **Authenticated, signed, and
encrypted packages:** authentication/signing is the v2 extension (§4);
encryption is a TFS-layer transform of the slot images (spec 10) — the
container itself stays byte-identical either way.

## 1. File anatomy

```
[payload bytes][slot table: n × 280][v2 extension?][trailer header: 166 @ EOF]
```

## 2. Trailer header — 166 bytes, always at EOF

| offset | size | field |
|-------:|-----:|-------|
| 0   | 10  | magic `"TEBAKOTFS\0"` |
| 10  | 4   | u32 version = **1** (extensions ride flags, not version bumps) |
| 14  | 4   | u32 package_flags (bit0 LEAN, bit1 SIGNED_V2) |
| 18  | 4   | u32 slot_count (1..=8) |
| 22  | 8   | u64 slot_table_offset (absolute file offset of slot 0) |
| 30  | 128 | char runtime_ref[128] — resolution hint (spec 05 §1); empty = classic |
| 158 | 4   | u32 launcher_abi |
| 162 | 4   | u32 crc32 over [0,162) — accident integrity, NOT authenticity |

`TPKG_MAGIC_PREFIX_LEN` = 4 (`"TEBA"`) discriminates absent vs corrupt.

## 3. Slot record — 280 bytes

| offset | size | field |
|-------:|-----:|-------|
| 0  | 8   | u64 offset (absolute file offset of the image) |
| 8  | 8   | u64 size |
| 16 | 4   | u32 format_id — IMAGE FORMAT ONLY: 0=auto, 1=dwarfs, 2=squashfs, 3=zip (4=runtime: legacy role wart, §5) |
| 20 | 4   | u32 flags (0 today; future home of role bits) |
| 24 | 256 | char mount_point[256] (UTF-8, NUL-padded) |

## 4. v2 chain-of-trust extension (opt-in; flag bit1 `SIGNED_V2`)

Between slot table and header; **all its numerics big-endian** (the header
stays v1-little-endian). v1 readers ignore the flag and run the package —
trust enforcement is a reader capability, not a format barrier.

| offset | size | field |
|-------:|-----:|-------|
| 0   | 256 | per-slot SHA-256, 8 × 32 B (digest of slot i at i×32; tail zeroed) |
| 256 | 8   | signer keyid (low 64 bits of OpenPGP fingerprint, BE) |
| 264 | sig_len | OpenPGP detached signature (binary packets) |
| 264+sig_len | 4 | u32be sig_len (1..=65536) |

Canonical signed bytes: `slot table ‖ digests ‖ keyid ‖ header` —
everything except the signature and its length field.

## 5. Orthogonality note (normative)

`format_id` answers exactly one question: how to read the slot's bytes.
Whether a slot is a runtime, and which payload carries the entrypoint, are
**roles** — L2 semantics declared by manifests (spec 03), never encoded in
the format axis. The v1 `format_id = 4 (TPKG_FORMAT_RUNTIME)` marks the fat
runtime slot so v1 loaders skip handing it over as a mount; it is a role
riding in the format field, kept only for backward compatibility. The
cleanup path: slot role moves to slot flags / package manifest, and
`format_id` returns to pure image-format semantics. New semantics must
never again be smuggled into the format axis.

## 5b. Typed extension blocks (the L2 home; locked 2026-07-26)

Following OCI's model — the manifest is a separate blob BESIDE the
layers, readable without pulling any image — the package-level (L2)
manifest lives in the container but OUTSIDE every payload image. The v2
signing extension already proved the mechanism: blocks between the slot
table and the trailer header are invisible to v1 readers (they only read
the header at EOF and the slot table at its recorded offset; the bounds
check never requires the table to abut the header).

Generalized layout:

```
[bootstrap][payload slots][slot table][ext blocks…][v2 signing ext?][header @EOF]

ext block: [u32be type][u32be length][payload bytes]
  type 1 = v2 signing extension (historical: immediately before the
           header, self-delimiting from the tail via its sig_len field)
  type 2 = package manifest (YAML — spec 03 §6; identity, entrypoint/
           suite composition, package-level jail + env, per-entry
           runtime refs)
```

Rules: blocks walk forward from the end of the slot table (type+length
self-delimit); the v2 signing ext, when present, is LAST before the
header (unchanged bytes/position); unknown block types are skipped by
v1-era-aware readers (forward-compat) but rejected by `validate` with a
named error; a package manifest never duplicates payload manifests — it
references slots, and payload manifests (spec 03) stay inside images
(self-describing payloads). The 128-byte single `runtime_ref` remains
for v1-era loaders; per-entry runtime references for suites and
multi-runtime packages live in the type-2 manifest (no size cap).

## 6. Absent vs corrupt; validation

- Last-166-byte window without the `"TEBA"` prefix → `NoTrailer` (classic
  bundle; callers may fall back to offset auto-detection).
- Prefix ok, magic bad → `Magic`. Magic ok, crc bad → `Crc`.
- Version ≠ 1 → `Version`. Slot count outside 1..=8 → `Slots`.
- Structural validation mirrors the reference C `tpkg_validate()`:
  `offset+size` non-overflowing, `format_id` ≤ 4, `runtime_ref` and mount
  points NUL-terminated within their fixed fields; table fits entirely
  before the header.
- v2: flag/extension co-presence; zeroed digest tail; signature non-empty
  and ≤ 65536; signer keyid non-zero.

## 7. Errors (Rust `TpkgError` ↔ C `TPKG_ERR_*`, 1:1)

`NoTrailer`, `Magic`, `Crc`, `Version`, `Slots`, `Bounds`, `Invalid` —
`strerror` strings byte-identical with the C implementation.
