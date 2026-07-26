# Spec 13 — Factories and releases

Normative specification of the production pipeline: how sources, runtimes,
and the product are built and published. Each repo builds its own
platforms and publishes its own releases — nothing builds C/C++ outside
the three factories.

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
            + manifest.json + SHA256SUMS (+ signatures, spec 09)
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

## 9. The toolkit factory (distro-maintainer model, locked 2026-07-26)

Making third-party native software work inside tebako payloads is a
**packaging concern, owned by a factory repo** — the same discipline
distro maintainers apply when patching software into their ports tree,
and the same shape as tamatebako/ruby:

```
tamatebako/toolkits/
  packages/<name>/
    recipe.yml          # upstream url+sha256, versions, build system,
                        # link mode, deps (other toolkits), platforms
    patches/            # per-version patch sets, ruby-factory naming rules
    manifests/          # payload manifest templates (provides/exec tier)
```

Recipe-declared axes:

- **link mode → exec tier** (spec 07 §8): `dynamic` (tier 1, preload
  shim at run time — default), `wrapped` (link-time interposition
  archive inside the binary), `tfs-native` (source patches +
  libtfs linked — the ruby model; survives static linking), `static`
  (plain static, extraction closure, no TFS).
- **relocatability**: dynamic tools build with `$ORIGIN/../lib` RPATH —
  any mount/extract location works, no install-time rewriting.
- **deps**: other toolkit payloads via the spec-03 requires graph
  (inkscape depends on libxml2/poppler payloads, consumer-mounted).

CI: build matrix per triplet (one mechanical leg each), boot-smoke per
tool, release per (tool × version) with the payload manifest filled
from the recipe. First reference port: inkscape (the metanorma case).
