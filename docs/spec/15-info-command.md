# Spec 15 — The info surface (payload and package introspection)

Normative specification of how users and tooling inspect payloads (`.tfs`
images) and packed binaries (tpkg executables). Status: PLANNED.

## 1. Principle

Every artifact in the ecosystem is self-describing (spec 03), and the
info surface must expose ALL of it — container, manifest, declarations,
trust state, and derived facts — in both human and machine form, without
ever mutating the artifact or the cache. Info is read-only; verification
is a named, explicit mode with strict exit codes.

Two MECE surfaces (no third tool):

- **`tfs info`** — standalone payload images.
- **`tebako-pkg info`** — packed binaries (the tpkg container AND its
  slot payloads).

Default output of both stays byte-parity with the C++ oracle where such
parity exists (golden rule); every richer view is an explicit new flag.

## 2. `tfs info <image>` — payload introspection

| flag | meaning |
|------|---------|
| (default) | current summary (parity, unchanged) |
| `--manifest` | pretty-print `/__tpkg__/manifest.yaml` parsed: IDENTITY (kind, name, version, producer, created, source digests, sbom, tree_hash + blob_sha256), signing state, encryption state, annotations |
| `--provides` | kind-specialized PROVIDES: entrypoints (name, path, args_default, runtime_requirement) for app; provides {engine, version, abi_line, platform} for runtime; mount_semantics for data; toolkit provides |
| `--requires` | the DEPENDS graph: each edge as `kind:name:constraint → mount` |
| `--platforms` | `universal` or the triplet list with release-asset-name mapping |
| `--json` | ALL of the above as one JSON document (schema-versioned: `"info_schema": 1`) |
| `--verify` | spec 03 validation: schema-valid manifest, digests well-formed, signing state vs actual signature block; report per check |
| `--backend-json` | backend-level metadata (the existing dwarfs metadata JSON: block sizes, compression, entry counts, uuid/created) |

Derived facts (computed, labeled DERIVED in human mode):
- **shims**: the command names this payload would register (from
  entrypoints).
- **runtime compatibility**: for app payloads, the runtime constraint;
  against `~/.tebako/runtimes` — `satisfied-by: <cached runtime>` or
  `requires-download: <newest compatible>` or `incompatible: <reason>`
  (named, never silent).
- **dependency closure**: payload names reachable via requires (1 level;
  full closure is the dispatcher's job — info stays shallow).

Human mode example:

```
image: metanorma-1.2.3.tfs
  format: dwarfs-t (flatbuffers metadata)  ro  42.1 MB
  kind: app  name: metanorma  version: 1.2.3
  platforms: universal
  digests: blob_sha256 9c37…  tree_hash (none)
  signing: signed (keyid a55a664f8270cd7a)
  encryption: none
  provides:
    entrypoint metanorma → /app/bin/metanorma  runtime: ruby >= 3.3, < 5.0
  requires:
    toolkit:inkscape >= 1.3 → /opt/inkscape
  derived:
    shims: metanorma
    runtime: satisfied-by ruby-3.4.2-0.15.9-macos-arm64 (cached)
```

## 3. `tebako-pkg info <binary>` — package introspection

| flag | meaning |
|------|---------|
| (default) | current trailer dump (parity, unchanged) |
| `--full` | full container report (below) |
| `--slot N` | inspect slot N's payload: format detection + the `tfs info` manifest/provides/requires sections, read through the trailer offsets |
| `--json` | everything as one JSON document (`"info_schema": 1`) |
| `--verify` | strict verification with named exit codes (§5) |
| `--depth 0\|1\|2` | 0 = trailer only; 1 = + slot manifests (default with `--full`); 2 = + backend metadata per slot |

Full container report:

```
package: metanorma (tpkg v1, lean, launcher_abi 1)
  size: 3,842,112 B  trailer: 446 B (166 header + 1 slot × 280)
  bootstrap: 1,825,504 B (portion before slot 0)
  runtime_ref: ruby@3.4.2;tebako=0.15.9 (resolution hint; lean)
  trust: v2-signed, signer a55a664f8270cd7a — Trusted (root 81C7…DFC0)
  slots:
    [0] 2,016,608 B @ 1,825,504  format: dwarfs  mount: /
         kind: app  metanorma 1.2.3  (1 entrypoint, runtime ruby >= 3.3, < 5.0)
    [1] — runtime payload slots are never mounted; lean: none
```

Rules:
- Format per slot: auto-detect through the tfs detection chain
  (dwarfs-t/flatbuffers vs upstream dwarfs/thrift distinguished, squashfs,
  zip, tar) — never trust `format_id` alone (it is a hint; `auto` means
  detect).
- The v1 `format_id = 4` slot is reported as `runtime (legacy role)`
  per spec 02 §5 — role, not format.
- Trust section: signature state (v2 / v1-unsigned), signer keyid, and
  WITH `--verify` the actual verification outcome; without it, state is
  reported as stored, labeled `unverified`.
- `--slot N` reads the image in place via the tfs mount-from-region —
  nothing is extracted.

## 4. Registry/cache introspection (same surface, other objects)

- `tebako cache list --json` — cached runtimes and payloads with their
  trust anchors, origins, sizes (extends the existing command additively).
- `tfs info <dir>` on a cache entry — same as the image form (a cache
  entry IS artifacts + markers).

## 5. Verification and exit codes

`--verify` / `tebako-pkg validate` run the checks and exit strictly:

| code | meaning |
|-----:|---------|
| 0 | all checks pass |
| 65 | trailer/manifest missing, malformed, or schema-invalid |
| 70 | sha256 mismatch (slot digest vs content, manifest digest vs image) |
| 71 | signature invalid (or unsigned under `--require-signed`) |
| 72 | signer key not in the trusted keyring |

Checks: tpkg structural validation (spec 02 §6) → per-slot sha256 (v2) →
signature (v2) → manifest schema validation per slot → digest agreement
(manifest blob_sha256 vs image bytes when declared).

## 6. Machine contract (JSON)

One JSON document, `"info_schema": 1`, keys: `artifact` {path, kind:
package|image, size}, `package` {version, flags, launcher_abi,
runtime_ref, bootstrap_bytes, trailer_bytes}, `trust` {state, keyid,
outcome}, `slots[]` {index, offset, size, format, detected_format, mount,
manifest…}, `manifest` (spec 03 mapped 1:1), `derived` {shims[],
runtime_compat, dependency_names[]}, `checks[]` (--verify only: {name,
result, detail}). JSON consumers pin to `info_schema`, not to field order.

## 7. Non-goals

No mutation (set/edit belongs to tebako-pkg surgery), no full dependency
closure resolution (dispatcher), no network fetches (info is local-only;
`--verify` against the keyring never downloads keys).
