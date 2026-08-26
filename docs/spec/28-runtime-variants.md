# Spec 28 — Runtime variants and the run-configuration surface

**Status: PLANNED (drafted 2026-08-26).** Amends spec 04 §2 (registry),
spec 05 §3/§5 (store layout, selection), spec 07 §0/§2/§4 (the selection
chains), spec 23 §3 (the D2 `runtime:` key alignment), spec 15 (the info
surface). No wire-format change; no trailer change; the L1 manifest is
untouched (the variant key DERIVES from the manifest's existing
`runtime_requirement`).

A payload version is not one build. A native-extension payload is locked
to the runtime ABI line it was baked against (spec 05 §5), so
`metanorma@1.16.9` baked on ruby 3.3 and `metanorma@1.16.9` baked on
ruby 4.0 are two artifacts with the same identity. Today the registry,
the store, and the dispatch chains each model exactly ONE build per
version — publishing the second build is impossible. This spec adds the
**variant dimension** (identity × runtime requirement) and the **run
configuration** that selects across it, as one grammar on six surfaces.

Two laws, extending spec 23's two:

1. **The variant law.** A variant is keyed by its `runtime_requirement`
   and nothing else — no labels, no channels, no invented axes. Two
   builds of one version that declare the same requirement are the same
   variant; a registry that carries both is a named authoring error.
2. **The selection law.** The user selects a PREFERENCE (an engine, a
   constraint, an exact pin), never an artifact. Resolution maps the
   preference to the one matching variant, deterministically, or fails
   with a named error listing the available variants. No silent winner.

## 1. The selector grammar (one grammar, every surface)

```
<engine>                        — any build for that engine (default rule §4)
<engine> <constraint>           — the variant whose requirement it satisfies
                                  ("ruby ~> 4.0.0"; the version classes of
                                  spec 05 §5: "~> X.Y.0" ABI-line,
                                  ">= A, < B" range)
<engine>@<version>              — sugar: the variant baked against that
                                  runtime LINE ("ruby@4.0.6" ≡ "ruby ~> 4.0.0"
                                  for a native build; ≡ the containing range
                                  for a pure-language build)
<engine>-<line>                 — the variant id verbatim (§2)
;tebako=<line>                  — OPTIONAL suffix on any form: the runtime
                                  release line preference (spec 05 §2),
                                  e.g. "ruby ~> 4.0.0 ;tebako=0.16.10"
```

Unparseable = a named error listing the grammar (invariant 9). The
selector NEVER names a triplet — platform selection stays the registry's
declarative job (spec 04 §2) — and never names an artifact filename.

## 2. The variant id (derived, canonical)

From the variant's `runtime_requirement` `{engine, constraint, abi?}`:

- No requirement (a data slice) or a pure-language range constraint:
  the variant id is `universal` (there is exactly one such variant —
  the law of §1 makes a second one a named error).
- An ABI-line constraint `~> X.Y[.0]`: `<engine>-<X.Y>`
  (`ruby-3.3`, `ruby-4.0`, `jruby-9.4`).
- Anything else (a constraint the id rule cannot canonize): the
  registry entry is a named validation error — the grammar does not
  guess.

The id is computed, never authored: registries carry the requirement;
the id appears in store paths and diagnostics only.

## 3. Registry grammar (spec 04 §2 amendment)

A version entry gains the additive key `variants:`; the top-level
`platforms:` + `runtime_requirement:` pair is exactly the single-variant
shorthand. MECE: a version entry carries EITHER the shorthand OR
`variants:`, never both — both present is a named validation error.

```yaml
schema_version: 1
payloads:
  - name: metanorma
    kind: app
    default: 1.16.9
    default_variant: ruby-3.3        # OPTIONAL; absent → §4's newest rule
    versions:
      - version: 1.16.9
        release: {ref: tfs:github:tebako-packages/metanorma:1.16.9-4}
        entrypoints: [metanorma]
        variants:
          - runtime_requirement: {engine: ruby, constraint: "~> 3.3.0", abi: "@@ABI@@"}
            platforms:
              aarch64-macos: {artifact: metanorma-1.16.9-ruby3.3-macos-arm64.tfs, sha256: "…"}
              x86_64-linux-gnu: {artifact: metanorma-1.16.9-ruby3.3-linux-gnu-x86_64.tfs, sha256: "…"}
          - runtime_requirement: {engine: ruby, constraint: "~> 4.0.0", abi: "@@ABI@@"}
            platforms:
              aarch64-macos: {artifact: metanorma-1.16.9-ruby4.0-macos-arm64.tfs, sha256: "…"}
```

- Each variant carries its own `platforms` map; a variant published for
  fewer triplets than its sibling is legitimate (resolution on a missing
  triplet is the spec 04 no-entry named error, scoped to that variant).
- Old readers on a variants-only entry see a version with no top-level
  `platforms:` and fail with the existing no-artifact named error — a
  clean break, loudly diagnosed, never a mis-resolution. (Legacy is
  dropped, not carried — the spec 23 discipline.)
- `signature:` stays per-version and covers every variant's artifacts
  (spec 09 unchanged).

## 4. Selection semantics (normative)

Given `(name, version, selector?)`:

1. No selector: the payload entry's `default_variant` if declared; else
   the variant whose requirement's line is NEWEST by version order
   (engine considered only when variants span engines — then the
   registry MUST declare `default_variant`; spanning engines without it
   is a named authoring error). The choice is journaled
   (`event=variant-selected name=… id=… source=default`).
2. A selector: the variants whose requirement it satisfies, per the §1
   forms. Zero matches → a named error listing every available variant
   id + its requirement (exit 69's spec 06 shape, payload class). More
   than one → the newest matching line wins, journaled (a constraint
   intersecting two lines is a preference, not an ambiguity — the
   registry-level duplicate is the only ambiguity, and it is a named
   error at registry load, per §1's variant law).
3. The chosen variant's requirement feeds spec 05 §5 unchanged: newest
   COMPATIBLE cached runtime → else download → verify → cache. The
   `;tebako=` suffix tightens the release-line preference inside that
   chain (the `runtimes:` config preference's per-invocation form).

## 5. The six surfaces (one grammar each, one precedence)

The selector grammar of §1 appears on exactly six surfaces; per command,
first hit wins top-down:

| # | Surface | Form | Scope |
|---|---------|------|-------|
| S1 | CLI flag | `tebako run\|install\|use … --runtime "<selector>"` | this invocation / act |
| S2 | Env | `TEBAKO_<TOOL>_RUNTIME="<selector>"` | this process tree |
| S3 | Project pin | `.tebako-tools.yaml` value `{version: …, runtime: "<selector>"}` (scalar stays version-only — spec 07 §0) | walking up from cwd, nearest that pins the command |
| S4 | Composition | `tebako.yaml` (D2, spec 23 §3) — per-slice `slices[].runtime: <selector>` and the runtime row itself | the composition |
| S5 | User default | `~/.tebako/config.yaml` `defaults:` value, same mapping form as S3 | this user |
| S6 | Registry default | §4 rule 1 | everyone |

S1–S5's precedence composes with the spec 07 §2 version chain unchanged:
the version chain resolves the version, the runtime chain resolves the
variant, neither shadows the other's links. `tebako use <tool>
--runtime "<selector>"` writes S5 (the `tebako use` writer ships with
this spec's Phase-A; until then the file is authored directly, spec 07's
standing note).

## 6. Store layout (spec 05 §3 amendment)

```
payloads/<name>/<version>/<variant-id>.tfs              # 0444
payloads/<name>/<version>/<variant-id>.tfs.sha256       # trust anchor
payloads/<name>/<version>/<variant-id>.manifest.yaml    # the mirror
```

Variants coexist byte-identically under one version directory; every
variant keeps its own anchor and mirror. Pre-spec-28 stores hold
`payloads/<name>/<version>.tfs` flat files: the installer MIGRATES on
first touch (move into `<version>/universal.tfs` resp. the derived id,
rewriting nothing else; journaled), and a flat file found at dispatch is
treated as the `universal` variant in place — read-compat only, writes
always land in the new layout.

## 7. The D2 amendment (spec 23 §3 alignment)

Two keys in the composition document align with spec 05's field names —
`runtime: {name: …, requirement: …}` becomes `runtime: {engine: …,
constraint: …}` (the old spellings are a named validation error naming
the rename — the spec 23 MECE discipline, no dual-key read). The runtime
row gains the optional `prefer: "<runtime ref>"` (an exact spec 05 §1
ref inside the constraint — the pin packed-mn's `versions.yaml` already
authors by hand) and `policy: newest-compatible` (the default, the only
value for now — a second value is a future spec, not a silent key).

```yaml
version: 1
runtime: {engine: ruby, constraint: "~> 3.3", prefer: "ruby@3.3.12;tebako=0.16.10;image"}
slices:
  - {name: metanorma, requirement: ">= 1.16", runtime: "ruby ~> 3.3.0"}   # the variant pick
  - {name: openjdk, requirement: "21"}
entrypoint: metanorma
```

A composition carrying a runtime MATRIX — the benchmark case — is two
composition documents differing in exactly the `slices[].runtime` (and
`runtime:`) rows; `tebako press` bakes each into its own package. Fat or
lean is orthogonal (spec 23). The spec 27 bench harness's managed arm
selects per invocation through S2 (`TEBAKO_METANORMA_RUNTIME`), the
pressed arms through two packages.

## 8. Multi-engine payloads ((4) — MRI vs JRuby)

Engine is a first-class axis of the requirement, not a new concept:

- A NATIVE-extension payload supports a second engine by carrying a
  second VARIANT (`{engine: jruby, constraint: "~> 9.4"}` → variant id
  `jruby-9.4`). The build differs, the grammar does not.
- A PURE-language payload whose ONE build runs on several engines
  declares `runtime_requirement` as a list — `any_of` semantics, OR in
  declaration order:

  ```yaml
  runtime_requirement:
    - {engine: ruby, constraint: ">= 3.3, < 5.0"}
    - {engine: jruby, constraint: "~> 9.4"}
  ```

  The list form is the only new L1 grammar (additive; schema_minor). A
  native-extension payload (an `abi:` in force) carrying the list form
  is a named manifest error — an ABI lock is single-engine by
  construction. The payload still stores as the ONE `universal`
  variant; the selector's engine picks among the `any_of` entries at
  runtime-resolution time, and an engine matching no entry is the §4
  rule-2 named error.

## 9. CLI and info surface (spec 15 amendment)

- `tebako install <name>@<ver> [--runtime "<selector>"]` — installs the
  selected variant (default per §4). Run twice with two selectors to
  hold both. Idempotent same-sha skip per anchor (spec 05 §4).
- `tebako info <name>@<ver>` lists every variant: id, requirement,
  triplets, install state — the resolution view, before any download.
- `tebako run`, `tebako use`: §5's S1/S5.
- The shim dispatch reads S2→S3→S5→S6 per invocation (S1/S4 are the
  `tebako run` / composition fronts of the same chain). A shim-disabled
  variant is not a concept — disable stays per version (spec 07 §0).

## 10. Error discipline

- Registry load: duplicate variant key inside one version; `variants:`
  plus the top-level shorthand; `default_variant` naming no variant;
  engine-spanning variants with no `default_variant` — all named
  registry validation errors naming the payload and version.
- Resolution: zero-match and no-triplet errors name the selector and
  the available variants (§4). No fallback across variants — a 4.0
  selector never silently runs the 3.3 build.
- The variant id is derived, so no authoring surface exists for it;
  the store migration (§6) journals every move.
- Exit codes: the spec 05/07 shapes (69 runtime/payload resolution,
  70 digest); no new codes.

## 11. Worked example — metanorma on two ruby lines, benchmarked

Bake (the feedstock matrix — one pipeline, two passes):

```
recipe.yml build.runtime: [{engine: ruby, version: "3.3.12", tebako: "0.16.10"},
                           {engine: ruby, version: "4.0.6",  tebako: "0.16.10"}]
→ metanorma-1.16.9-ruby3.3-*.tfs and …-ruby4.0-*.tfs, one registry
  version entry, two variants (§3)
```

Operator (managed mode):

```
tebako install metanorma@1.16.9 --runtime "ruby ~> 3.3.0"
tebako install metanorma@1.16.9 --runtime "ruby ~> 4.0.0"
~/.tebako/payloads/metanorma/1.16.9/{ruby-3.3.tfs, ruby-4.0.tfs} (+anchors)

metanorma compile site.adoc                         # the default variant (ruby-3.3 per default_variant)
TEBAKO_METANORMA_RUNTIME="ruby ~> 4.0.0" metanorma compile site.adoc   # the same shim, the 4.0 build
tebako use metanorma --runtime "ruby ~> 4.0.0"      # persist the flip (S5)
```

The two runs differ in exactly one store path and one runtime
resolution; every other byte of the world — registry, composition,
jail, mounts — is unchanged. That is the benchmark A/B the variant
dimension exists for.
