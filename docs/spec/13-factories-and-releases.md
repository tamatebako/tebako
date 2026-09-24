# Spec 13 — Factories and releases

Normative specification of the production pipeline: how sources, runtimes,
and the product are built and published. Each repo builds its own
platforms and publishes its own releases — tebako-owned C/C++ is built
only in `dwarfs-t` (upstream ruby's own C builds in the two ruby
factories, as vendored upstream source).

## 1. The chain

```
upstream language release (ruby-lang.org)
   │  cron monitor (tamatebako/ruby) detects → onboards or files drift issue
   ▼
SOURCE FACTORY: tamatebako/ruby
   canonical patches + versions.yml → lint matrix → release:
   tfs-ruby-<ver>-src[-<scenario>].tar.gz + SHA256SUMS  (e.g. v0.2.8: 149 assets)
   │  workflow_run success → repository_dispatch (TEBAKO_CI_PAT_TOKEN)
   ▼
RUNTIME FACTORY: tebako-runtime-ruby
   pin-bump PR (DEFAULT_RELEASE) → matrix build per (version × triplet) →
   release: tebako-runtime-<ver>-<lang>-<triplet>[.exe] + .tfs (image era)
            + per-asset <asset>.sha256 sidecars + per-package
              <stem>.manifest.json shards (the sidecar-era authority,
              spec 05 §2)
            + per-artifact .asc on signing-enabled lines (spec 09 §5)
            (no monolith release assets — manifest.json / SHA256SUMS
              are derived consumer-side, spec 13 §2a)
   │  consumed by
   ▼
PRODUCT: tebako-rs
   press resolves runtimes from the index; bootstrap resolves at run time.
dwarfs-t (C++ format lib) → releases → dwarfs-rs (FFI crate) → tebako-rs
```

## 2. Source factory rules

- Patches are canonical unified diffs; naming carries semantics:
  `tfs-ruby-<major>-<minor>-x-<slug>.patch` (whole line) vs
  `...-<patch>-<slug>.patch` (exact release; supersedes the `-x-` form for
  that version only).
- `versions.yml`: per version — official URL + sha256 + line. Every fetch
  is sha256-verified.
- `tools/apply <version> --platform <scenario>` produces scenario trees
  (`linux-gnu`, `linux-musl`, `msys-pass1`, `msys-pass2`); the unsuffixed
  asset is byte-identical to the linux-gnu scenario (back-compat).
- Any patch that fails `git apply --check` aborts the build — loud, never
  silent.

## 2a. The release-index entry (manifest.json, locked)

One entry per runtime PACKAGE (the executable), additive forever.
**Publish shape (tebako#493, locked 2026-08-30; the finalize pass
DELETED 2026-09-12, roadmap 85):** each platform's publish invocation
writes ONLY its own payload assets plus the write-once metadata that
describes them — the per-asset `<asset>.sha256` sidecar (coreutils
`<sha>  <file>`, hashed from the local bytes) and the per-package
`<stem>.manifest.json` shard (the entry below, served standalone, every
sha field re-anchored to the served bytes; on signing-enabled lines the
shard also declares each artifact's `signature` block — the `.asc`
spellings are the factory's to declare, written at publish) — and, on
signing-enabled lines, signs them IN THE SAME INVOCATION (spec 09 §5):
every payload asset's `<asset>.asc`, every shard's
`<stem>.manifest.json.asc`, every sidecar's `<asset>.sha256.asc`. A
signature a shard declares but the invocation does not produce fails
the leg, never ships.

**No shared mutable name exists.** The monoliths (`manifest.json`,
`SHA256SUMS.txt`) are NEVER release assets: they are derivable
conveniences computed consumer-side from the asset listing plus the
shards (`tebako-pkg release-index`), for humans and audit only. A
MISSING monolith is therefore the normal state of a live release, and a
STALE one cannot exist — nothing on the release has a second writer.
Release notes are written once at release creation and never rewritten.
This kills the rendezvous: the 2026-08-29 wedge (four platform jobs
racing two shared monoliths) and the 2026-09-12 wedge (the finalize
pass burning the full metadata budget on GitHub's deleted-asset-name
422 — the delete-then-re-upload convergence loop sustaining the very
reservation it waited out) are the same physics: aggregation forces
mutation, mutation forces delete-then-replace, replace is the wedge.

**The immutability rule (locked 2026-09-12).** Every release asset name
is write-once. A bad asset is never deleted-and-replaced: the registry
entry gets `status: withdrawn` (spec 04 §2 — resolvers refuse it by
name) and the fix ships as the next patch line. Completeness is a
QUERY, not a gate: the expected matrix ⊆ the listed names, each with
sidecar + shard + `.asc` (the audit mode computes it read-only), and
the publication event is the in-repo `tpkg-registry.yaml` commit — git
arbitrates concurrency, never release-asset mutation.

Payload assets stay byte-immutable; the per-leg metadata is write-once
by construction (identical content on a re-run is a digest-match skip —
a re-run never replaces). Sidecars and shards are the authority; the
resolver reads them first (spec 05 §2).

**Bundle-era publish shape (additive, 2026-09-24 — spec 36 owns the
format).** A bundle-era line publishes THREE assets per leg —
`<stem>.tar.gz` (the bundle: exe + env image + member pins),
`<stem>.tar.gz.sha256`, `<stem>.manifest.json` — plus one `.asc` of
each on signing-enabled lines (spec 09 §5's every-served-name rule
intact). The shard gains the additive `bundle` block (spec 36 §3) and
keeps its per-member pins. The asset budget this buys: a signed catalog
publish drops from ~1,800 assets (over GitHub's 1,000-asset release
ceiling — the v0.16.28 wedge) to ~805, and a routine tips release to
~175. Release topology follows spec 36 §6: tips by default, catalog
runs are for migrations and derive their own per-line shard tags,
withdrawal bounds growth.

```json
{
  "tebako_version": "0.16.0",
  "contract_version": 2,
  "ruby_version": "3.3.7",
  "platform": "macos-arm64",
  "filename": "tebako-runtime-0.16.0-3.3.7-macos-arm64",
  "sha256": "…", "size_bytes": 38683544,
  "abi": "arm64-darwin-23",
  "signature": {"keyid": "efc3c250f7862a48",
                "asc": "tebako-runtime-0.16.0-3.3.7-macos-arm64.asc"},
  "image": {"filename": "….tfs", "sha256": "…", "size_bytes": 7658081,
            "signature": {"keyid": "efc3c250f7862a48", "asc": "….tfs.asc"}}
}
```

- `abi` is the runtime's own platform string (ruby:
  `Gem::Platform.local.to_s` — `RbConfig::CONFIG["arch"]` with the darwin
  segment hyphenated). Native-extension payloads constrain BOTH the
  version line and this line (spec 05 §5); readers that predate the key
  ignore it (the compat window — an absent `abi` never fails a check).
- The image nests under `image` (name + sha + size); the exe's own
  digest is the entry's `sha256`. `contract_version` negotiates the
  launcher semantics (spec 17).
- `signature` (additive, opt-in per artifact — spec 09 §5): the exe
  carries the entry-level `signature`, the env image `image.signature`,
  a windows `dll` its own — each `{keyid, asc}` mirroring the registry
  model (spec 04 §2): `keyid` the signer's 16-lowercase-hex PRIMARY
  keyid (spec 09 §9's primary-vs-subkey rule), `asc` the detached
  sidecar's exact asset name within the same release (declared by the
  factory, flowed verbatim — consumers never synthesize it, exactly like
  `filename`). Absent marks a pre-signing release line — keep-forever
  (§8), installable under spec 09 §4's unsigned-fetch rule. Present ⇒
  the named `.asc` exists in the same release: a declaration without its
  signature asset is an invalid signing state (spec 09 §4 — exit 71 at
  fetch, and the publish fails before that, above).

## 3. Drift loop (SHIPPED pending PR merge: ruby#41, runtime-ruby#17)

- Monitor (daily cron): diff upstream releases vs versions.yml → new
  versions onboard (versions.yml entry + patch selection + lint) → PR →
  tag → release-src → dispatch. Failing patches → named issue carrying
  the failing hunk output.
- Publish → pin bump: `runtime-pin-bump.yml` (on release-src success)
  dispatches `bump-source-pin.yml` in the runtime repo → branch
  `bot/source-pin-<tag>` → PR (no auto-merge; the matrix runs on the PR).
- Idempotent: re-runs with nothing new do nothing; no force-pushes.

## 4. Runtime factory rules

- Builds consume ONLY the source factory's releases (sha256-verified).
- Runtime entry driver: modern `tebako_fs_*` API only (nm gate: no legacy
  symbols); mounts each payload slot via multi-mount; the image-era
  driver prefers `TEBAKO_RUNTIME_IMAGE` (spec 06 §2).
- Image-era releases dual-publish interpreter + `.tfs` (back-compat);
  `.tfs` is a dwarfs-t-native (FlatBuffers metadata) image — upstream
  dwarfs cannot read it, so it carries OUR extension; `.dwarfs` is
  reserved for upstream-compatible images.
- Boot-smoke class (roadmap 19) runs per built runtime before publish:
  boot, stat family (statx regression), IO, bundler, locks.

## 5. Product release pipeline (tebako-rs; roadmap 01/02)

- Tag → prepare → per-platform NATIVE builds (macOS arm64/x86_64,
  linux-gnu x86_64/arm64, linux-musl x86_64/arm64 via cargo-zigbuild,
  windows ucrt64) → size-gate table (bootstrap < 3 MB, hard fail) →
  SHA256SUMS + completeness gate (expected asset set; a partial upload
  fails the release) → signed (spec 09) once ceremony lands.
- Reference shape: parsanol-rs release-binary.yml (native runners,
  dtolnay toolchain, upload-release-asset) + our additions (musl
  zigbuild, size gate, completeness gate).
- `manifest.json` in the bootstrap-release shape so existing resolution
  consumes Rust artifacts unchanged.
- rnp-rs consumed from crates.io with `vendored` — zero system librnp
  provisioning anywhere.

## 6. Matrix and tier policy (locked)

- Exhaustiveness lives where it's cheap: source factory lints ALL
  versions per push; runtime factory builds ALL (version × triplet) on
  release/weekly; consumers test LINE coverage
  {oldest supported, newest 3.x, newest major}, not patchlevel coverage.
- PR tier: boundary lines × tier-1 platforms (< 10 min). Weekly: full
  lines × all platforms. Release: exhaustive.
- `concurrency` group per workflow+ref with `cancel-in-progress: true`;
  preflight smoke gates the matrix spawn; `fail-fast: false`; caches
  keyed by content hash; upstream-drift-prone heavy legs
  `continue-on-error` with weekly triage; shared setup in composite
  actions.

## 7. Cross-repo triggers

`repository_dispatch` via the `TEBAKO_CI_PAT_TOKEN` secret (existing
mechanism; no new secrets). Bot PRs use the PAT so pull_request CI fires.

## 8. Runtime retirement (locked 2026-07-26)

Factory releases are KEEP-FOREVER: no deletion, no retention clock. A
manifest/runtime_ref pinning a missing runtime (typo, pre-release ref)
fails with the named exit-69 class error plus a hint to inspect
available versions — never a silent fallback to a different runtime.
Cache GC stays user-driven (`tebako cache prune`).

## 9. The toolkit factory: the `tebako-packages` org (locked 2026-07-26)

GitHub releases are per-repo tag namespaces — a single ports repo would
force every tool into one version line. The toolkit factory is therefore
the **conda-forge feedstock model**: a dedicated `tebako-packages`
GitHub org with ONE REPO PER PACKAGE, each owning its recipe, patch
sets, and release line:

```
tebako-packages/inkscape/         # one repo per package (a "feedstock")
  Tebakofile            # upstream url+sha256, versions, build system,
                        # link mode, deps (other toolkits), platforms
  patches/              # per-version patch sets, ruby-factory naming rules
  manifests/            # payload manifest templates (provides/exec tier)
  .github/workflows/    # build matrix per triplet + boot-smoke + release
tebako-packages/index/            # the catalog: a registry-of-registries
  tpkg-registry.yaml    # lists each package's registry ref + summary
```

- **Release lines**: per-repo tags — `<upstream-version>` (e.g. `1.3.2`),
  packaging revision only for a re-release of the same upstream
  (`1.3.2-2`). Releases carry the per-triplet payload artifacts +
  that repo's `tpkg-registry.yaml` (default-branch root, or pinned as
  an artifact — spec 04 §2).
- **References**: `tfs:github:tebako-packages/inkscape:1.3.2` (registry
  mode) or `…#inkscape-1.3.2-macos-arm64.tfs` (direct artifact mode).
- **Discovery**: `tebako add-registry tfs:github:tebako-packages/index`
  — one explicit registry exposes the whole catalog; nickname installs
  (`tebako install inkscape@1.3.2`) resolve through it.
- **Recipe axes** (unchanged from the ports model): upstream
  url+sha256, patch sets, build system, link mode → exec tier
  (`dynamic` | `wrapped` | `tfs-native` | `static`), `$ORIGIN/../lib`
  RPATH for dynamic builds, deps on other toolkit payloads via the
  spec-03 requires graph (inkscape → libxml2/poppler payloads).
- **Release hosting (locked):** every package's built payloads —
  platform-specific or platform-free (universal) — are published as
  GitHub releases **in the package's own feedstock repo** (artifacts +
  SHA256SUMS + its `tpkg-registry.yaml`). Nothing is hosted centrally:
  the index carries only catalog entries pointing at each feedstock's
  own releases.
- CI per repo: build matrix per triplet (one mechanical leg each),
  boot-smoke per tool, release per (tool × version) with the payload
  manifest filled from the recipe.
- **The proven feedstock line (2026-07-27):** `hello` (native trivial),
  `fontist` (app, triplet-bound), `inkscape` (toolkit, heavy dynamic),
  `metanorma` (the flagship app: 260 gems, inkscape DEPENDS wired,
  full PDF compile through the payload). First reference port: inkscape
  (the metanorma case). Org creation is an owner action; repo/feedstock
  creation then follows the template in `tebako-packages/index`.
