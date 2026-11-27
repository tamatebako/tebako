# Spec 15 — The info surface (payload and package introspection)

Normative specification of how users and tooling inspect payloads (`.tfs`
images) and packed binaries (tpkg executables). Status: SHIPPED
(`crates/tebako-info` + the tfs-cli/tebako-pkg/tebako-cli wiring; the
digest-agreement note in §5). §8's SBOM evidence surface (`tebako sbom`,
CycloneDX 1.6) SHIPPED with tebako#1120.

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
  per spec 02 §6 — role, not format.
- Flags render named: the header shows `, lean` and `, no-install` after
  the tpkg version, and the JSON `package.flags` array carries
  `"lean" | "signed-v2" | "no-install"` (spec 02 §5 — a frozen package
  is introspectable before you try to install it).
- Trust section: signature state (v2 / v1-unsigned), signer keyid, and
  WITH `--verify` the actual verification outcome; without it, state is
  reported as stored, labeled `unverified`.
- `--slot N` reads the image in place via the tfs mount-from-region —
  nothing is extracted.

## 4. Store/system introspection (`tebako info`) and the artifact umbrella (`tebako inspect`)

Two verbs on the product CLI (both SHIPPED; `--json` on every view,
`"info_schema": 1`):

- **`tebako inspect <artifact>`** — the §2/§3 surfaces on the product
  CLI: payload images and tpkg binaries auto-detected (trailer
  presence), the section flags (`--manifest/--provides/--requires/
  --platforms/--slot N/--backend-json`), `--json`, and `--verify` with
  the §5 exit codes. The entrypoint line names the abi line when the
  manifest declares it (`runtime: ruby ~> 3.3.0, abi arm64-darwin-23`),
  and an implementation-narrowed requirement renders its entries as
  `<implementation> <constraint>` (`runtime: ruby mri ~> 3.3.0, abi
  arm64-darwin-23`; an `any_of` list joins its entries ` | `).
- **`tebako info [topic]`** — the machine view of the STORE and, with
  `--remote`, of the world:
  - `system` (bare) — tebako version, platform, home, cached counts,
    store size.
  - `runtimes` — cached runtimes (engine, version, tebako, abi, image,
    size); `--remote` adds the runtime factory's release index (tag ×
    ruby × platform × abi × contract).
  - `payloads` — cached payloads (name, version, kind, size, origin);
    `--remote` adds every registered registry's catalog (name, kind,
    versions, default).
  - `shims` — each registered command's dispatch preview: the payload,
    version, and runtime it would run today (read-only resolution).
    (IMPLEMENTED 2026-10-07 — tebako#559: the preview gains the effective
    interp_env map — every declared interpreter-option key with its
    winning value and its provenance layer: `env` / `config` / `package`
    / `payload` / `built-in`, the spec 07 §9.1 chain rendered; a key no
    layer declares renders `built-in` with no value.)
  - `registries` — registered registries and their dispatch-cache
    freshness (fresh/stale/missing/local).
  - `store` — disk usage by section (runtimes/payloads/shims/tmp/…).
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
(manifest blob_sha256 vs image bytes when declared) → **entries
cross-check** (the L2 package manifest's `entries[]` against the
referenced slot payload: every `entries[].entrypoint` path must exist in
the slot's image — one read-only mount per referenced slot — and every
`entries[].name` must match a DECLARED entrypoint of that payload's L1
`provides.entrypoints[]` / toolkit `provides.executables[]`; the name
facet is reported "unchecked", never failing, when the slot carries no
usable L1 manifest — pre-manifest packages stay valid. A slot-less entry
(the spec 23 §13 pointer-package form) skips: its shared slice is
resolved and checked at run time. Exit 65 on a dangling path, an
undeclared name, a slot index beyond the container's slots, or an entry
naming a runtime-role slot).

> **Digest-agreement note (implementation finding).** The agreement check
> compares the declared `blob_sha256` against the actual image bytes. A
> manifest embedded in the image it describes cannot name that image's
> digest (the field would have to hash its own bytes — a fixed point no
> producer can reach), so for self-embedded manifests the check in
> practice detects a manifest naming a DIFFERENT blob (tamper or staleness
> evidence), and a clean `--verify` is reached by plain images and by
> packages whose slots carry no manifests. A producer-side rule (digest
> computed over the image with the manifest excluded, or the field made
> advisory) is a spec-03 follow-up; the info surface implements the check
> as written and reports it precisely.

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

## 8. The SBOM evidence surface (`tebako sbom`) — SHIPPED (tebako#1120)

**Status block (tebako#1120, tracker tebako#1124).** The verb and this
section are the SBOM arc's keystone: CycloneDX 1.6 is the primary
format; SPDX is DEFERRED until a consumer asks with evidence (the arc's
standing rule). The runtime artifacts' own SBOMs (env image inventory +
the link-unit exe's crate set captured at factory build time) are §8a
(SHIPPED, tebako#1121); the vulnerability-scan process that consumes
these documents is tebako#1123's.

`tebako sbom` emits a **CycloneDX 1.6 JSON** bill of materials for one
payload — read-only, local-only, and deterministic by construction:

```
tebako sbom <image.tfs | name[@version]>
            [--closure <path>] [--ecosystem gem|pypi|npm|cargo|generic]
            [--triplet <triplet>] [--out <path>] [--random-serial]
```

- **Target**: a `.tfs` image path (mounted through the tfs detection
  chain — nothing is extracted), or an installed payload's
  `name[@version]` resolved read-only against the machine store (an
  exact version, else the name's newest installed; a miss is the named
  error, never a guess — `tebako info payloads` lists the cache).
- **Identity**: the payload's in-image `/__tpkg__/manifest.yaml` is the
  authoritative identity (kind, name, version, the `kind: language`
  require edge's engine, the single declared platform when exactly one).
  An image without a readable manifest falls back to the filename's
  `<name>-<version>…` stem, loudly; a stem with no version segment is a
  named error (exit 65).
- **Inventory — complete mode**: `--closure <path>` carries the
  feedstock's closure file — the complete transitive artifact inventory
  (`<artifact> <sha256>` or the sha256sum-shaped `<sha256> <artifact>`
  spelling; the hash column is recognized BY SHAPE — 64 hex — never by
  position, so both spellings parse through one grammar; `#` comments
  and blank lines skip). Each row maps to one component under the
  ecosystem grammar below; every component carries the closure's
  sha256 in its `hashes` array (`tebako:closure:complete=true`).
- **Inventory — degraded mode**: with no closure, the inventory degrades
  to the image's own gem specifications (`specifications/*.gemspec`, or
  the staged runtime layout's `lib/ruby/gems/<v>/specifications/`):
  name + version + licenses only — **no per-artifact hashes, and
  completeness is NOT claimed** (`tebako:closure:complete=false`, and a
  CycloneDX annotation against the payload's bom-ref says so in words).
  An image carrying no parseable gemspecs refuses with the named error
  pointing at `--closure` (exit 65).
- **Ecosystem mapping** (the component's purl and type): the explicit
  `--ecosystem` flag first, then the manifest's language engine
  (`ruby`→gem, `python`→pypi, `node`→npm), then `generic` with a loud
  note — never a silent guess.

| ecosystem | purl type | component type | name rule |
|-----------|-----------|----------------|-----------|
| gem | `pkg:gem/` | library | `<name>-<version>[-<platform>]`; the FIRST dash-segment starting with a digit opens the version |
| pypi | `pkg:pypi/` | library | PEP 427 wheel stem; the name is PEP 503-normalized (lowercase, `[-_.]+`→`-`); the tag triplet is evidence, never purl |
| npm | `pkg:npm/` | library | scoped names keep `/`; the leading `@` percent-encodes |
| cargo | `pkg:cargo/` | library | the same digit rule |
| generic | `pkg:generic/` | application | native tools/binaries |

  Purls are canonical `pkg:<type>/<name>@<version>` — qualifiers (gem
  platform, wheel tags) ride as component properties
  (`tebako:gem:platform`, `tebako:pypi:tags`) because vulnerability
  consumers match on name+version. Identical purls from distinct
  artifacts keep the canonical purl and get disambiguating bom-refs
  (`<purl>#artifact-N`) — the schema's bom-ref uniqueness law.
- **The document**: `metadata.component` is the payload itself (type by
  kind: app/toolkit→`application`, runtime/language→`platform`,
  data→`data`; the image's own sha256 in `hashes`; properties
  `tebako:payload:kind`, `tebako:triplet`, the manifest's signing state
  as `tebako:payload:signing`, the language engine when declared);
  `metadata.tools` is the emitting tebako version; `dependencies` is
  flat (the payload depends on every component — intra-ecosystem edges
  are the lockfile's knowledge, not the closure's); when the target was
  store-resolved, the store's `.origin` marker rides as a
  `distribution` external reference. **The SBOM DESCRIBES — it never
  anchors trust**: verification stays spec 09's chain (the `.asc`
  signature, the signed SHA256SUMS/sidecars); the signing-state
  property is the manifest's declaration, not a verification outcome.
- **The determinism law**: components sort by purl then hash; the
  `serialNumber` is a NAME-BASED UUID (v5 shape) derived from the
  document's identity content (payload purl + triplet + every
  component's purl/hash); the timestamp reads `SOURCE_DATE_EPOCH` when
  the operator pins it (the press's reproducibility convention — the
  same variable the image writer reads), the wall clock otherwise. Two
  runs on unchanged inputs are byte-identical. `--random-serial` opts
  into a per-run v4 serial (an identifier, never a token).
- **Exit codes** (the info surface's grammar): 0 emitted; 64 usage;
  65 target/closure/image/manifest malformed or missing, empty
  inventory, unparseable gemspec set; 74 I/O (store read, file
  read/write).

Consumer proof: `osv-scanner --sbom=<file>` parses the emitted document
(the metanorma closure's ≈237 components is the arc's acceptance case);
the vulnerability mapping itself is downstream tooling's job
(tebako#1123), never the verb's.

## 8a. The runtime-artifact family (`--runtime` / `--runtime-exe`) — SHIPPED (tebako#1121)

**Status block (tebako#1121, tracker tebako#1124).** A runtime release
pair (the interpreter exe + its env image) gets its own two documents,
emitted lane-agnostically from the published artifact files. The §8
payload lane is unchanged: a `kind: runtime` image through it keeps the
degraded discipline (the bundled specifications class only) and now
prints a stderr pointer at `--runtime`. The exe's crate inventory is
captured AT BUILD TIME and rides inside the link-unit tarball (spec 13
§5a) — the verb never re-derives a build's resolution after the fact,
so a release cannot silently drift from the build's real crate set.

The two documents are named after the artifact files themselves (the
family's naming rule): `<env-image-file-name>.cdx.json` and
`<exe-file-name>.cdx.json` — e.g.
`tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64.tfs.cdx.json` and
`tebako-runtime-0.17.15-ruby-4.0.9-macos-arm64.cdx.json` (the windows
artifact keeps its suffix: `….exe.cdx.json`).

**The verb grammar** — three forms, the mode grammar validated before
any I/O (violations are exit 64):

```
tebako sbom <image.tfs | name[@version]> [--closure <path>] [--ecosystem …] [--triplet <triplet>] [--out <path>] [--random-serial]
tebako sbom --runtime <env-image.tfs> [--triplet <triplet>] [--out <path>] [--random-serial]
tebako sbom --runtime-exe <exe> --crates <crate-inventory.json> --version <ver> [--name <name>] [--engine <engine> --engine-version <ver>] [--triplet <triplet>] [--out <path>] [--random-serial]
```

`--runtime` and `--runtime-exe` are mutually exclusive; the exe mode
requires `--crates` + `--version`; `--crates`/`--name`/`--engine`/
`--engine-version` are forbidden outside it; `--closure`/`--ecosystem`
are forbidden in both runtime modes (each mode's inventory source is
fixed, never mixed). The runtime modes take PATHS, never store
references.

**The env-image document** (`--runtime`):

- **Target and gate**: the env image file, mounted through the tfs
  detection chain (never extracted). It must carry a readable
  `/__tpkg__/manifest.yaml` declaring `kind: runtime`; a non-runtime
  kind or a manifestless image is the named refusal (exit 65), pointing
  at the payload lane.
- **Subject**: the image itself — type `platform`, the manifest's
  identity name+version, `pkg:generic/<name>@<version>`, the image's
  sha256 in `hashes`; properties `tebako:payload:kind=runtime`,
  `tebako:triplet` (the flag, else the manifest's declared platforms),
  `tebako:runtime:engine` / `tebako:runtime:engine_version`.
- **Components — the interpreter build**: one component per engine
  entry the manifest's runtime provides declares (type `platform`,
  `pkg:generic/<engine>@<version>`; properties
  `tebako:runtime:abi_line`, `tebako:runtime:platform`, and — when the
  manifest's `built_from` declares them — `tebako:runtime:src_sha256`
  and `tebako:runtime:patch_set`). NO binary hashes: the interpreter is
  statically linked into the exe, so its build facts are the source pin
  and the patch set; the stdlib tree is the interpreter's own versioned
  content, represented by this component. Engine entries that differ
  only by platform dedup to one row.
- **Components — the gem inventory**: the image's GEM_HOME
  specifications, BOTH classes — the bundled `specifications/*.gemspec`
  (`tebako:gem:default=false`) and the interpreter's own default gems
  `specifications/default/*.gemspec` (`tebako:gem:default=true`); the
  class is a property, never a purl facet. Name + version + licenses
  only, no per-artifact hashes. An image with no gem layout emits the
  interpreter-only document — never a refusal (a non-ruby runtime is a
  first-class shape).
- **The completeness frame**: `tebako:sbom:inventory-source=
  runtime-image` and `tebako:closure:complete=false` — NEVER "complete"
  in the closure sense, because no hashed closure exists for a runtime
  image. A CycloneDX annotation against the subject names what the
  inventory honestly IS: the interpreter build description (engine,
  version, source pin / patch set), the gem counts per class, the
  no-closure framing, and the sibling exe document's role (the exe's
  vendored crates, captured at build time).

**The exe document** (`--runtime-exe`):

- **Subject**: the exe itself — type `platform`, the EXE's OWN sha256
  in `hashes` (the capture's crates are re-subjected to the shipped
  bytes). `--version` is REQUIRED (the bytes carry no recoverable
  version); the name defaults to the exe's file name minus a windows
  `.exe` suffix (a suffix strip, never the asset-name grammar), `--name`
  overrides. Properties: `tebako:payload:kind=runtime`,
  `tebako:capture:package` + `tebako:target` (from the capture),
  `tebako:platform` (from `--triplet`, when given),
  `tebako:runtime:engine` / `tebako:runtime:engine_version` (from the
  flags, when given).
- **Components**: the capture's crate set under spec 13 §5a's mapping
  (`pkg:cargo/<name>@<version>` purls, the registry checksum in
  `hashes` when present, the license expression, the
  `tebako:crate:source` property for non-registry sources).
- **The graph is EXPLICIT**: the capture's own dependency edges plus
  one root edge from the subject to the capture's root crate — never
  the payload lane's flat form.
- **The completeness frame**: `tebako:sbom:inventory-source=
  cargo-resolution-capture`, `tebako:closure:complete=true` — the
  capture IS the build's full normal+build resolve closure, so
  completeness holds by construction; NO annotation.
  `tebako:capture:tool` / `tebako:capture:tool_version` name the
  capture's generator.

**The capture artifact** (`crate-inventory.json`, `capture_schema: 1`).
The exe document's second input and the no-drift wire: the capture is
emitted INTO the staged link-unit tree before the tar (spec 13 §5a), so
one tarball hash covers the staticlibs AND the inventory their build
resolved. The grammar's single owner is the `tebako-sbom` model crate's
`capture` module; `sbom-cargo --inventory-only` is the single
generator — the same `cargo metadata --filter-platform` + `Cargo.lock`
inputs the release pipeline's own documents consume, so the capture and
the tarball's `.cdx.json` are two serializations of ONE resolution.
Members: `capture_schema`, `tool`, `tool_version`, `package`, `target`,
`root`, `components[]` (`name`, `version`, `purl`, `sha256`|null,
`license`|null, `source`), `edges[]` (`ref`, `depends_on[]`). The
`source` vocabulary mirrors §8's mapping: `workspace` (cargo's null) ·
`registry` (the registry+/sparse+ spellings) · the verbatim `git+…` /
`path+…` strings. The consumer gates FAIL-CLOSED on `capture_schema`
(an unsupported version is the named error, exit 65) while tolerating
unknown MEMBERS (additive evolution inside a schema version); an empty
component set or a root naming no component is malformed.

**Determinism and exit codes**: §8's determinism law applies unchanged
to both documents (components sort by purl, the serial derives from the
document's identity content, the timestamp pins to `SOURCE_DATE_EPOCH`)
— two runs on unchanged inputs are byte-identical. Exit codes are §8's
grammar: 0 emitted; 64 usage (the mode grammar); 65 target/capture/
manifest malformed or missing, wrong-kind image, unsupported capture
schema; 74 I/O.
