# Spec 28 — Runtime variants and the run-configuration surface

**Status: PLANNED (drafted 2026-08-26; revised 2026-08-30 — the
language/implementation split: `engine` names the LANGUAGE, so mri,
jruby and truffleruby are all `ruby`, and the optional `implementation`
sub-axis distinguishes them).** Amends spec 03 §2 (the requirement and
`provides` grammars), spec 04 §2 (registry), spec 05 §3/§5 (store
layout, selection), spec 07 §0/§2/§4 (the selection chains), spec 23 §3
(the D2 `runtime:` key alignment), spec 15 (the info surface). No
wire-format change; no trailer change; the L1 manifest change is
ADDITIVE (the optional `implementation` key; the variant key still
DERIVES from the manifest's existing `runtime_requirement`). The
revision lands before any variant machinery shipped — there is nothing
to migrate; the pre-revision flat-engine spellings are retired, not
carried.

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
2. **The selection law.** The user selects a PREFERENCE (an engine, an
   implementation, a constraint, an exact pin), never an artifact.
   Resolution maps the preference to the one matching variant,
   deterministically, or fails with a named error listing the available
   variants. No silent winner.

## 1. The selector grammar (one grammar, every surface)

```
<engine>                        — any implementation of that language
                                  (default rule §4)
<engine>:<implementation>       — one implementation of the language, any
                                  line ("ruby:jruby")
<engine>[:<implementation>] <constraint>
                                — the variant whose requirement it satisfies
                                  ("ruby ~> 4.0.0", "ruby:jruby ~> 9.4";
                                  the version classes of spec 05 §5:
                                  "~> X.Y.0" ABI-line, ">= A, < B" range)
<engine>[:<implementation>]@<version>
                                — sugar: the variant baked against that
                                  runtime LINE ("ruby@4.0.6" ≡ "ruby:mri ~> 4.0.0"
                                  for a native build; ≡ the containing range
                                  for a pure-language build)
<engine>[-<implementation>]-<line>
                                — the variant id verbatim (§2)
;tebako=<line>                  — OPTIONAL suffix on any form: the runtime
                                  release line preference (spec 05 §2),
                                  e.g. "ruby ~> 4.0.0 ;tebako=0.16.10"
```

`engine` names the LANGUAGE: mri, jruby and truffleruby are all
`engine: ruby`, one engine with three implementations (spec 03 §2.2's
`provides.implementation`; §8 is normative). The implementation is the
ONLY sub-axis; it never appears without its engine (`jruby` alone is
unparseable), and an implementation unknown for the engine is a named
resolution error listing the known ones. Unparseable = a named error
listing the grammar (invariant 9). The selector NEVER names a triplet —
platform selection stays the registry's declarative job (spec 04 §2) —
and never names an artifact filename.

## 2. The variant id (derived, canonical)

From the variant's `runtime_requirement` `{engine, implementation?,
constraint, abi?}`:

- No requirement (a data slice) or a pure-language constraint with no
  `abi:` in force: the variant id is `universal` (there is exactly one
  such variant — the law of §1 makes a second one a named error). The
  implementation axis does NOT fork a pure variant: one build serves
  every implementation its requirement matches (§8).
- An ABI-line constraint `~> X.Y[.0]` (`implementation` is REQUIRED
  whenever `abi:` is in force, §8): `<engine>-<implementation>-<X.Y>`
  (`ruby-mri-3.3`, `ruby-mri-4.0`, `ruby-jruby-9.4`).
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
    default_variant: ruby-mri-3.3      # OPTIONAL; absent → §4's newest rule
    versions:
      - version: 1.16.9
        release: {ref: tfs:github:tebako-packages/metanorma:1.16.9-4}
        entrypoints: [metanorma]
        variants:
          - runtime_requirement: {engine: ruby, implementation: mri, constraint: "~> 3.3.0", abi: "@@ABI@@"}
            platforms:
              aarch64-macos: {artifact: metanorma-1.16.9-ruby3.3-macos-arm64.tfs, sha256: "…"}
              x86_64-linux-gnu: {artifact: metanorma-1.16.9-ruby3.3-linux-gnu-x86_64.tfs, sha256: "…"}
          - runtime_requirement: {engine: ruby, implementation: mri, constraint: "~> 4.0.0", abi: "@@ABI@@"}
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
   (implementation considered only when variants span implementations —
   then the registry MUST declare `default_variant`; spanning
   implementations without it is a named authoring error). The choice is
   journaled (`event=variant-selected name=… id=… source=default`).
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
value for now — a second value is a future spec, not a silent key). The
runtime row's `implementation` pins which implementation of the language
the composition builds against; absent, the §4 default rule decides at
press time and the choice is journaled.

```yaml
version: 1
runtime: {engine: ruby, implementation: mri, constraint: "~> 3.3", prefer: "ruby@3.3.12;tebako=0.16.10;image"}
slices:
  - {name: metanorma, requirement: ">= 1.16", runtime: "ruby:mri ~> 3.3.0"}   # the variant pick
  - {name: openjdk, requirement: "21"}
entrypoint: metanorma
```

A composition carrying a runtime MATRIX — the benchmark case — is two
composition documents differing in exactly the `slices[].runtime` (and
`runtime:`) rows; `tebako press` bakes each into its own package. Fat or
lean is orthogonal (spec 23). The spec 27 bench harness's managed arm
selects per invocation through S2 (`TEBAKO_METANORMA_RUNTIME`), the
pressed arms through two packages.

## 8. The implementation axis — one language, many implementations ((4))

**The law (owner-locked 2026-08-30):** mri, jruby and truffleruby are
one engine — `engine: ruby`. The implementation is a sub-axis of the
requirement, never an engine of its own; a user of an executable
payload may run it on ANY implementation the requirement admits. The
same law applies to every future multi-implementation language.

- The RUNTIME declares what it is: `provides: {engine: ruby,
  implementation: jruby, version: 9.4.8, language_version: "3.1", …}`
  (spec 03 §2.2). `implementation` is REQUIRED for `kind: runtime`;
  `language_version` is the language level the runtime implements (for
  mri it equals `version`).
- A PURE-language payload's ONE build serves every implementation whose
  `language_version` satisfies its constraint — the requirement simply
  omits `implementation` (`{engine: ruby, constraint: ">= 3.3, < 5.0"}`)
  and the user's implementation choice defers to runtime resolution
  (spec 05 §5). When the admissible set differs per implementation, the
  requirement is a list — `any_of` semantics, OR in declaration order:

  ```yaml
  runtime_requirement:
    - {engine: ruby, constraint: ">= 3.3, < 5.0"}                 # any implementation at that language level
    - {engine: ruby, implementation: jruby, constraint: "~> 9.5"} # jruby's own version line
  ```

  The list form is the only new L1 grammar (additive; schema_minor).
  The payload still stores as the ONE `universal` variant; the
  selector's implementation picks among the `any_of` entries at
  runtime-resolution time, and an implementation matching no entry is
  the §4 rule-2 named error.
- A NATIVE-extension payload (an `abi:` in force) is locked to ONE
  implementation's ABI by construction: `implementation` is REQUIRED
  alongside `abi` — a native requirement without it is a named manifest
  error — and the list form is forbidden (a second implementation means
  a second build, i.e. a second VARIANT: `{engine: ruby,
  implementation: mri, constraint: "~> 3.3.0", abi: …}` → variant id
  `ruby-mri-3.3`; the truffleruby build of the same payload version is
  a sibling variant `ruby-truffleruby-24.1`, published when its
  toolchain exists — TODO.truffleruby).
- truffleruby's native and jvm modes are two runtime ARTIFACTS of one
  implementation (they differ in which dependencies they can host, not
  in the language they speak): mode is NOT a selector axis — selection
  rides the release-line preference (`;tebako=`) or per-entry pins
  (spec 17). A payload needing jvm-mode interop declares that edge in
  DEPENDS (spec 03 §2.3), where the runtime-on-runtime composition spec
  pins it (TODO.jruby/01, TODO.truffleruby/03).

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
  implementation-spanning variants with no `default_variant`; a variant
  requirement carrying `abi:` with no `implementation` — all named
  registry validation errors naming the payload and version.
- Resolution: zero-match and no-triplet errors name the selector and
  the available variants (§4); an implementation unknown for the engine
  is a named error listing the known ones (§1). No fallback across
  variants — a 4.0 selector never silently runs the 3.3 build — and no
  fallback across implementations.
- The variant id is derived, so no authoring surface exists for it;
  the store migration (§6) journals every move.
- Exit codes: the spec 05/07 shapes (69 runtime/payload resolution,
  70 digest); no new codes.

## 11. Worked example — metanorma on two ruby lines, benchmarked

Bake (the feedstock matrix — one pipeline, two passes):

```
recipe.yml build.runtime: [{engine: ruby, implementation: mri, version: "3.3.12", tebako: "0.16.10"},
                           {engine: ruby, implementation: mri, version: "4.0.6",  tebako: "0.16.10"}]
→ metanorma-1.16.9-ruby3.3-*.tfs and …-ruby4.0-*.tfs, one registry
  version entry, two variants (§3): ruby-mri-3.3, ruby-mri-4.0
```

Operator (managed mode):

```
tebako install metanorma@1.16.9 --runtime "ruby:mri ~> 3.3.0"
tebako install metanorma@1.16.9 --runtime "ruby:mri ~> 4.0.0"
~/.tebako/payloads/metanorma/1.16.9/{ruby-mri-3.3.tfs, ruby-mri-4.0.tfs} (+anchors)

metanorma compile site.adoc                             # the default variant (ruby-mri-3.3 per default_variant)
TEBAKO_METANORMA_RUNTIME="ruby:mri ~> 4.0.0" metanorma compile site.adoc   # the same shim, the 4.0 build
tebako use metanorma --runtime "ruby:mri ~> 4.0.0"      # persist the flip (S5)
```

The two runs differ in exactly one store path and one runtime
resolution; every other byte of the world — registry, composition,
jail, mounts — is unchanged. That is the benchmark A/B the variant
dimension exists for.

A PURE-language payload on the same registry carries the single
`universal` variant; the operator flips implementations with no second
install at all:

```
TEBAKO_TOOL_RUNTIME="ruby:jruby" tool …    # any jruby line the requirement admits
TEBAKO_TOOL_RUNTIME="ruby" tool …          # back to the §4 default
```
