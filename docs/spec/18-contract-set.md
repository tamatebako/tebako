# Spec 18 — The Contract Graph

**Status:** normative · supersedes the compatibility parts of specs 06 and 17
**Date:** 2026-08-01

> **The law:** nothing is *read until it breaks*. Every interface in the
> ecosystem has exactly ONE owner, ONE declarative YAML contract (a
> versioned schema), and ONE verification point. Consumers verify
> fail-closed **before** execution; a mismatch is a named error naming
> both sides. Anything undeclared is **pre-era (era 1)** and is refused
> by name — never assumed, never silently served.

This document is the complete map: every component, every interface
between them, the declarative contract on each interface, the evolution
law for every schema, and the scenario catalog that each rule must
answer. Nothing cross-component may exist outside this map.

---

## 1. The full contract graph

Boxes are components (artifacts or actors). Every arrow is a **contract
edge** labeled with its declaration document. Edges are directed:
owner → consumer.

```
        ┌────────────────────────────────────────────────────────────────┐
        │ tamatebako/ruby — SOURCE FACTORY                               │
        │  out: patched src tarball (tebako-mount-root, SHA256SUMS)      │
        └───────────────┬────────────────────────────────────────────────┘
                        │ C1 tarball-layout.yaml
                        ▼
┌──────────────┐  C19   ┌───────────────────────────────────────────────┐
│ vcpkg        │───────▶│ tebako-runtime-ruby — RUNTIME FACTORY         │
│ baseline     │ tags   │  builds: runtime exe + env image              │
└──────────────┘        │  in:  link unit (tebako-rs artifact)          │
                        │  out: runtime release (exe, image, manifest)  │
                        └──────┬───────────────┬───────────────┬────────┘
                               │ C2 release-   │ C3 exe↔image  │ C4 link-time
                               │ manifest.yaml │ (layout.yaml) │ (c_api.h, fs TU)
                               ▼               ▼               ▼
                        ┌──────────────┐  ┌─────────────┐  ┌───────────────┐
                        │ registries / │  │ runtime exe │  │ static libs   │
                        │ release idx  │  │ (driver in) │  │ (driver, tfs) │
                        └──────┬───────┘  └──────┬──────┘  └───────────────┘
                               │ C12             │ C5 handoff (spec 17,
                               │ registry.yaml   │ argv/env — contract_version)
                               ▼                 │
┌─────────────┐  C13 store    ┌─────────────────▼───────────────────────┐
│ store       │◀──────────────│ LOADER FAMILY                           │
│ (~/.tebako) │               │ bootstrap · shim · cli · tfs-cli        │
└──────┬──────┘               └───┬────────┬─────────┬────────┬─────────┘
       │ C13                      │ C6     │ C7      │ C8     │ C9
       ▼                          ▼        ▼         ▼        ▼
  everything local         package   payload   registry  runtime
                           trailer   manifests          release manifest

┌────────────────────────────────────────────────────────────────────────┐
│ PAYLOADS (all slices share the L1 in-image manifest contract)          │
│  ┌──────────────┐   ┌──────────────┐   ┌───────────────┐               │
│  │ executable   │   │ data         │   │ feature/      │               │
│  │ payload      │   │ payload      │   │ toolkit       │               │
│  │ (entrypoints)│   │ (no entries) │   │ (libs/exes    │               │
│  │              │   │              │   │  at paths)    │               │
│  └──────┬───────┘   └──────┬───────┘   └──────┬────────┘               │
│         │ C10 runtime_requirement            │ C11 provides-C-ABI      │
│         ▼                  ▼                   ▼                        │
│      runtime           consumers          consumers                   │
└────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────┐
│ TRUST:  signer → package (C14 v2 signature block)                      │
│         verifier → trust anchors / keys (C15 key ring + rollover)      │
└────────────────────────────────────────────────────────────────────────┘
```

The driver inside the runtime exe mounts the env image (C3) and each
payload image (C7), installs the jail, and hands the interpreter its
argv — the interpreter's io shims consume the tfs C API (C4), whose
header is the pinned contract both sides compile against.

---

## 2. The declarative contract model

### 2.1 Every interface has a YAML contract document

Two kinds of YAML exist and must not be conflated:

- **Schemas** — the grammars themselves, versioned, living in ONE
  registry: `docs/spec/schemas/<name>.yaml` in the product repo. Each
  schema file carries `schema: <name>` and `schema_version: <int>` and
  the evolution metadata (§3). This directory is the single source of
  truth for every grammar in the ecosystem.
- **Declarations** — the per-artifact YAML instances conforming to a
  schema (in-image manifests, layout.yaml, release manifest,
  tpkg-registry.yaml, the package L2 block, config.yaml).

Every producer validates its declarations against the schema it owns in
its own CI; every consumer validates before use and refuses invalid
input with a named error.

### 2.2 The contract set per component type

| Component | Declares (schema in `docs/spec/schemas/`) |
|---|---|
| tebako-bootstrap (binary) | an embedded self-description block (`artifact-info.yaml` appended at build): era, version, launcher_abi spoken, contract_version spoken |
| runtime exe | `runtime-manifest.yaml` (in the release): era, contract_version, mount_root, abi, image_layout, interpreter + version, built-from (source release + sha) |
| env image | in-image `/lib/tebako/layout.yaml`: era, image_layout, mount_root, interpreter api version |
| executable payload | L1 `payload-manifest.yaml`: era, identity, provides (entrypoints × per-entry runtime_requirement × abi facets), requires (closure edges × mount points), jail defaults |
| data payload | L1 `payload-manifest.yaml`: era, identity, version, content class, mount suggestions, requires |
| feature/toolkit payload | L1 as executable **plus** `provides_abi` (C ABI name+version at an in-image path, e.g. the crypto toolkit) |
| package (tpkg) | header (binary, unchanged) + L2 `package-manifest.yaml` block: era, pressed_by, reader_era required, composition summary |
| registry | `tpkg-registry.yaml`: schema_version, entries (name, versions, artifacts × abi, refs, defaults) |
| store | `~/.tebako/layout-version` + `config.yaml` schema |
| source tarball | `tebako-mount-root` + versions.yml schema + SHA256SUMS |
| release manifest (product) | `manifest.json` per tebako-rs release: platforms, sizes, sha256, era |
| vcpkg baseline | tag grammar + canonical-root map (restore script) — not YAML, owned grammar |
| dwarfs_c ABI | `dwarfs_c_abi_version()` — C header, not YAML, version export |
| tfs c_api | `c_api.h` with layout asserts — C header, compile-time contract |
| handoff (spec 17) | `TEBAKO_CONTRACT_VERSION` env export + this spec — argv/env, machine version via env |

---

## 3. The schema evolution law

Every schema in the ecosystem obeys these rules. No exceptions, no local
dialects.

1. **Two numbers, one meaning.** `schema_version` is a single integer
   MAJOR; `schema_minor` an additive counter. MAJOR breaks, MINOR adds.
2. **Readers ignore unknown fields** within their MAJOR — but must not
   invent semantics for them. A field you do not understand changes
   nothing you do.
3. **Unknown MAJOR is a named refusal.** Message: artifact, its schema +
   version, consumer, its max spoken version, "upgrade". Exit code per
   §7.
4. **Missing `schema_version` = era 1.** A named refusal — "pre-era
   document; regenerate with a current tool" — never a silent default.
5. **Type changes are MAJOR.** No silent coercion (string→list, int→
   string, kebab→snake). Authors wanting a rename: see the deprecation
   window.
6. **Deprecation window.** To rename/retype a field inside one MAJOR:
   writers emit BOTH old and new for two consecutive MINOR versions
   (`deprecated: <field>` noted in the schema); readers prefer new and
   warn on old; after two MINORs the old field is dropped in the next
   MINOR. The schema records the window dates.
7. **Critical extension.** Any container that skips unknown entries
   (tpkg extension blocks, manifest `provides`/`requires` entries, jail
   rules) supports `critical: true` on an entry: a reader that does not
   understand a *critical* entry must refuse with a named error instead
   of skipping it.
8. **Lists stay lists.** No keyed-maps for ordered/multiplicity data
   (owner rule, existing convention) — entries carry their identity
   fields (e.g. `version:`), never map keys.
9. **YAML for everything authored** (invariant 6 unchanged); JSON
   Schema documents the shape; CI validates producers against it.

### 3.1 The field lifecycle

```
        MINOR +1                    MINOR +2                    MAJOR +1
  ┌──────────────┐          ┌──────────────┐          ┌──────────────┐
  │ field added  │  rename? │ old+new both │  after   │ old dropped  │
  │ (additive,   │ ───────▶ │ written; old │ 2 MINORs │ entirely;    │
  │ readers skip)│          │ deprecated   │ ───────▶ │ readers of   │
  └──────────────┘          └──────────────┘          │ old MAJOR    │
                                                      │ are refused  │
                                                      └──────────────┘
```

---

## 4. Every interface, in full

For each edge: **owner → declaration (schema) → consumer verification →
refusal**. (C-numbers match the graph.)

### C1 · source tarball → runtime factory
Owner: tamatebako/ruby. Declaration: `tebako-mount-root` + the
`versions.yml` schema + SHA256SUMS. Verify: factory source fetcher at
configure — manifest absent/corrupt → **exit 132** ("pre-era or corrupt
source tarball — roll a new one with tamatebako/ruby ≥ v0.2.13").

### C2 · runtime factory → loader (the release card)
Owner: tebako-runtime-ruby. Declaration: `runtime-manifest.yaml`
(era, contract_version, mount_root, abi, image_layout, interpreter+ver,
exe sha ↔ image sha, built-from). Verify: loader **before download** —
era-1 manifest → refusal; contract_version > spoken → exit 75; abi
mismatch → resolution keeps searching, then named error.

### C3 · runtime exe ↔ env image (the pair)
Owner: the factory. Declaration: in-image `/lib/tebako/layout.yaml`
(era, image_layout, mount_root, interpreter api). Verify: **driver,
post-mount, before ruby init** — layout missing → era-1 refusal;
layout mismatch vs the exe's compiled expectation → **exit 78**.

### C4 · exe ↔ static libs (tebako_main, mount point, c_api)
Owner: the factory + `c_api.h`. Verified at build: layout asserts both
sides (compile = the check) + factory CI `nm` provenance assert
(tebako_main resolves from the fs TU, never the stub). No runtime
dimension (one exe bundles both).

### C5 · bootstrap → runtime (the handoff)
Owner: spec 17 (unchanged grammar). Machine version:
`TEBAKO_CONTRACT_VERSION` export + the release manifest (C2). Verify:
the negotiation (era/contract) happens in C2's step, before download —
never after exec.

### C6 · loader → package trailer
Owner: tpkg. Declaration: header (format, launcher_abi, flags) + L2
`package-manifest.yaml` (era, pressed_by, reader_era, composition).
Verify on open: unknown format/launcher_abi/flag → named error;
era mismatch → **exit 77** both directions; unknown critical extension
block → refusal (invariant 7 covers only non-critical).

### C7 · driver → payload images
Owner: the package's L2 composition + each slice's L1. Verify: duplicate
mount → EEXIST; any failure → unmount-all + named error (spec 17
unchanged); unknown `format_id` → named error; encrypted with no
toolkit → capability refusal.

### C8 · loader → registry
Owner: the registry schema. Declaration: `tpkg-registry.yaml`
(`schema_version`). Verify on read: missing version → era-1 refusal;
newer MAJOR → upgrade refusal; entry abi mismatch → filter.

### C9 · loader/shim → runtime release manifest
= C2 (the release card is the complete pre-download contract).

### C10 · executable payload → runtime
Owner: the L1 manifest. Declaration: per-entry `runtime_requirement`
(version range for pure-language; ABI line for native-extension) +
`abi` facets. Verify at dispatch: unsatisfiable → named error naming
the entrypoint and the requirement (never a segfault).

### C11 · feature payload → consumers
Owner: the L1 manifest. Declaration: `provides_abi` (name, version,
in-image path) — a consumer (e.g. the bootstrap loading the crypto
toolkit) binds only when the ABI matches, with both versions printed on
mismatch.

### C12 · everything → registries
= C8 plus publisher-side validation: `tebako publish` validates the
registry against the schema before pushing (producer-side CI).

### C13 · everything local → store
Owner: the store-layout module (one crate). Declaration:
`~/.tebako/layout-version` + `config.yaml` schema. Verify on first
access per process: newer → upgrade refusal; older → the named
migration (never a silent mixed layout).

### C14 · signer → package
Owner: tebako-signer. Declaration: the v2 signature block (in the
canonical signed region) + key ids. Verify: strict whenever signed
(unchanged); unsigned is loud-but-first-class (unchanged);
`TEBAKO_REQUIRE_SIGNED` fails closed (unchanged).

### C15 · verifier → trust anchors
Owner: the root ceremony (trust dir layout, key rollover). Declaration:
key ring with per-key ids + validity eras; old artifacts verify against
the key that signed them; revoked key → strict failure naming it.

### C16 · feedstock → product tools
Owner: tebako-packages/index. Declaration: `recipe.yml`
(`schema_version`) + **the product release the feedstock pins**
(currently tracks `main` — a drift bomb; pin the 2.0 release tag).

### C17 · jail declaration → enforcement
Owner: `tpkg::jail` + the driver. Malformed → exit 73 (unchanged);
unknown jail directive → named error (no silent drops).

### C18 · preload shim ↔ driver (spawn re-entry)
Owner: the driver (`TEBAKO_TFS_MOUNTS` grammar — the slot form included,
spec 17 §2.1; the trailer data source is `tpkg::read_from`). Verify:
unparseable → fail closed, never a half-mounted child; a packaged
payload's slot survives the hand-off (the child mounts the slot region,
never the whole package file).

### C19 · vcpkg baseline → consumers
Owner: dwarfs-t. Grammar: tag + canonical-root map (the restore script
is the single reader). Verified: tag presence + SHA256SUMS.

### C20 · dwarfs-rs ↔ dwarfs-t FFI
Owner: dwarfs-t header. New: `dwarfs_c_abi_version()` checked at bind
with both numbers printed on mismatch.

---

## 5. The scenario catalog

Each rule above is accountable to these cases. Each names the expected
behavior. (S-numbers are the e2e/test ids to implement.)

### 5.1 Bootstrap × package
- **S1** package era > bootstrap era → refuse: "package from a newer
  tebako (era 3) — upgrade your tebako (speaks era 2)".
- **S2** era-1 package (no L2 contract block) → refuse: "pre-era
  package — re-press with tebako ≥ 0.16.1 or run with tebako v1".
- **S3** `launcher_abi` unknown → named error, both values.
- **S4** unknown flag bit set (e.g. a future ENC_V3) → refuse: flags
  are load-bearing, never skipped.
- **S5** slot `format_id` unknown → named error.
- **S6** `TEBAKO_OFFLINE=1` and the runtime is uncached → named error
  (cache-or-error, exists).
- **S7** signed package, no toolkit → unverified-first loud warning +
  journal; `TEBAKO_REQUIRE_SIGNED=1` fails closed (both exist).
- **S8** two slots declare the same mount → EEXIST, no partial mount.
- **S9** a slot mount == the runtime root → named collision error.
- **S10** unknown **critical** extension block → refusal; unknown
  non-critical block → skipped (invariant 7).

### 5.2 Loader × runtime
- **S11** release manifest lacks contract fields (era 1) → refusal
  **before any download**: "runtime release is pre-era — rebuild with
  the current factory".
- **S12** runtime `contract_version` newer than the loader speaks →
  exit 75, both numbers.
- **S13** payload slot mount == runtime root → the C7 collision error
  (surfaces at boot, named).
- **S14** env image layout ≠ exe expectation → exit 78 (C3).
- **S15** sha256 mismatch on download → exit 70 (exists).
- **S16** locally built runtime (side-loaded directory): must carry the
  same manifest fields, else era-1 refusal — no special pleading.

### 5.3 Driver × env image
- **S17** `layout.yaml` absent → era-1 refusal.
- **S18** `layout.yaml` newer MAJOR → upgrade refusal.
- **S19** `layout.yaml` mount_root ≠ exe fs_mount_point → exit 78
  (mismatched pair, never a ruby LoadError).
- **S20** missing stdlib content inside a well-formed image → content
  bug (factory boot-smoke's domain), NOT a contract case — the contract
  layer does not mask it.

### 5.4 Driver × payloads
- **S21** duplicate mount across payloads → EEXIST, unmount-all.
- **S22** payload mount nested INSIDE another mount's tree → legal
  (spec 17); longest-prefix dispatch; jail sees the union.
- **S23** payload image backend this driver lacks → named error
  ("format X unavailable in this build").
- **S24** encrypted payload, crypto toolkit absent → capability refusal
  (named; install the toolkit).
- **S25** entrypoint missing in the image → named error with the
  expected path.
- **S26** entrypoint's runtime_requirement unsatisfiable → named error
  (never a crash).

### 5.5 Shim × payloads
- **S27** two installed payloads PROVIDE the same command → the
  registered owner wins; `tebako install` of the second names the
  conflict; explicit `tebako shim enable/disable` overrides (exists).
- **S28** version chain: `TEBAKO_<TOOL>_VERSION` → nearest
  `.tebako-tools.yaml` → config `defaults:` → registry default
  (exists, unchanged).
- **S29** shim points at a removed payload version → named error
  listing installed versions.
- **S30** entrypoint needs a runtime with no compatible cached copy →
  download+verify+cache, or offline named error.
- **S31** one package with two entrypoints needing DIFFERENT runtimes
  (e.g. ruby tool + java tool) → per-entry resolution against each
  entry's own requirement (grammar already supports it; e2e proves it).

### 5.6 Payload × payload
- **S32** requires cycle (A→B→A) → named cycle error at install.
- **S33** unsatisfiable range (`B >= 2`, only `B 1.x` published) →
  named error naming both.
- **S34** nested composition (A mounts B, B mounts C inside) → legal;
  resolved deepest-first.
- **S35** required entrypoint renamed upstream → unsatisfiable named
  error at install (the closure check is content-aware).
- **S36** abi facet mismatch (`abi: arm64-darwin-23` on x86_64) →
  resolution filters, then named error if nothing remains.
- **S37** data payload version requiring newer registry schema → C8's
  refusal.
- **S62** shared slice unresolvable at run time (uncached + offline, or
  no compatible artifact) → the loader's named resolution error BEFORE
  the handoff; the boot never starts (carried slices have no such case).
- **S63** locked-digest mismatch on a shared slice (registry bytes ≠ the
  press-time lock) → fail-closed trust refusal (exit 70 class): the lock
  is the pin, never fresh semver (spec 23 §4/§13).
- **S64** crash mid-lazy-seed → tmp+rename keeps the partial seed
  invisible; the run is unaffected; the next run re-seeds (spec 05 §4).

### 5.7 Press × inputs
- **S38** `TEBAKO_BOOTSTRAP` override with an era-1 bootstrap binary →
  the press reads the binary's embedded `artifact-info.yaml` and
  refuses; a package is only ever stitched from a declared bootstrap.
- **S39** image format the writer can't embed → named error.
- **S40** output path exists → named error unless `--force`.

### 5.8 Store
- **S41** layout newer than the binary → upgrade refusal.
- **S42** layout older → the named migration runs or refuses
  explicitly; never a silent mix.
- **S43** lock held past 120 s → stale-lock hint (exists).
- **S44** crash mid-install → tmp+rename keeps it invisible (exists).

### 5.9 Registry
- **S45** schema newer MAJOR → upgrade refusal.
- **S46** no `schema_version` → era-1 refusal ("republish the
  registry").
- **S47** entry abi has no host match → filtered, named if empty.
- **S48** registry unreachable, cache older than 24 h TTL → named error
  (exists).

### 5.10 Factory
- **S49** tarball without `tebako-mount-root` → exit 132.
- **S50** manifest with conflicting roots → named error (exists).
- **S51** link unit vs patches ABI drift → compile error + the `nm`
  provenance assert (the stub-shadow class).
- **S52** release publish with an incomplete asset set → the
  completeness gate fails (exists).

### 5.11 Trust
- **S53** signed artifact, key revoked → strict failure naming the key.
- **S54** root rollover → old artifacts verify against their signing
  key's era; new artifacts the new root; both declared in the key ring.
- **S55** `TEBAKO_REQUIRE_SIGNED=1` with unsigned → fail closed (exists).
- **S56** tampered extension block inside the signed region → signature
  failure (canonical region covers it, exists).

### 5.12 Evolution (the schema lifecycle cases)
- **S57** reader meets unknown MINOR field → ignores it, semantics
  unchanged (regression-tested per schema).
- **S58** unknown MAJOR → refusal (S1's shape) per schema.
- **S59** field rename: two MINORs of dual-write, readers prefer new
  and warn on old, then drop (a schema records the window).
- **S60** field type change → MAJOR bump; no coercion anywhere.
- **S61** `critical: true` entry a reader doesn't know → refusal;
  non-critical → skip (S10's rule generalized).

---

## 6. The verification sequences

### 6.1 Run of a stitched package

```
bootstrap
  ├─ read trailer: format/abi/flags (C6) ───────── refuse(77)/named
  ├─ resolve runtime: registry (C8) → store (C13) → release card (C2)
  │    ├─ era/contract check ───────────────────── refuse(75)
  │    └─ download + sha256 + cache (trust at fetch, never per run)
  └─ handoff (C5, unchanged grammar)
driver
  ├─ mount env image at exe root
  ├─ layout.yaml check (C3) ───────────────────── exit 78
  ├─ mount payloads (C7) ──────────────────────── EEXIST/unmount-all
  ├─ jail install (C17) ───────────────────────── exit 73
  └─ argv → interpreter
```

### 6.2 Shim dispatch

```
argv0
  ├─ resolve payload + entrypoint (C12) ───────── S27/S29 errors
  ├─ version chain (S28)
  ├─ per-entry runtime_requirement (C10) ──────── S26/S30
  ├─ runtime release card (C2) ────────────────── refuse(75)
  └─ exec runtime (C5 handoff)
```

### 6.3 Install / publish

```
tebako install <ref>
  ├─ registry read (C8) ───────────────────────── S45/S46/S48
  ├─ dep-walk the requires closure (C13) ──────── S32–S36
  ├─ download + verify + tmp/rename (S44)
  └─ register shims (S27)

tebako publish
  ├─ validate declarations against schemas (C12)
  ├─ sign if configured (C14)
  └─ push artifacts + registry entry
```

---

## 7. Exit codes (allocation)

| Code | Meaning | Raised by |
|---|---|---|
| 65–74, 76 | (existing loader codes) | bootstrap/resolve |
| 75 | runtime contract mismatch / pre-era runtime | loader, pre-download |
| 77 | package/payload contract-era mismatch (either direction) | any trailer/manifest reader |
| 78 | env image layout mismatch | driver |
| 79 | payload check FAIL aggregate (`EX_TEBAKO_CHECK`, spec 26 §2) | tebako check |
| — | ABI mismatch (C11 provides_abi, C20 FFI): named refusal, both versions printed; exit code unallocated — pinned at implementation | binder |
| 73 | jail malformed | driver |
| 132 | pre-era/corrupt source tarball | runtime factory |

---

## 8. Locked non-goals

- **No old-path readers.** Era 1 is refused, not adapted.
- **No silent skipping of load-bearing fields.** Non-critical skips are
  for decoration, never semantics (the `critical` rule).
- **No type coercion in readers.**
- **No second grammar for the handoff** (spec 17 stands; this is the
  declaration layer around it).
- **No per-run trust checks** (fetch/install only, unchanged).

## 9. Implementation map (owner → work)

| # | Work | Owner |
|---|---|---|
| 1 | `docs/spec/schemas/` registry: every schema above, with `schema_version` + evolution metadata | tebako-rs |
| 2 | tpkg L2 `package-manifest.yaml` block (era, pressed_by, reader_era) + reader enforcement (77) | tebako-rs |
| 3 | runtime release `runtime-manifest.yaml` full card (+ `mount_root`, `image_layout`) | tebako-runtime-ruby |
| 4 | in-image `layout.yaml` + driver check (78) | tebako-runtime-ruby + tebako-driver |
| 5 | loader pre-download refusal on missing contract fields | tebako-bootstrap/resolve |
| 6 | `tebako inspect`: the contract card + verdict for any artifact | tebako-cli |
| 7 | `dwarfs_c_abi_version()` + bind check | dwarfs-t + dwarfs-rs |
| 8 | store `layout-version` + read check | tebako-rs |
| 9 | registry `schema_version` enforce (publish validates, resolve refuses) | tebako-cli + resolve |
| 10 | factory `nm` provenance assert; exit 132 | tebako-runtime-ruby |
| 11 | feedstock product pin (post-2.0 tag) | tebako-packages |
| 12 | scenario catalog → e2e suite (S1–S61) | tebako-rs + factory |
| 13 | `tebako info` embeds `artifact-info.yaml` in the bootstrap binary (S38) | tebako-rs release |
