# Spec 03 — Payload manifest (L1)

Normative specification of the in-image manifest: every payload is a
**self-describing artifact**, not a blob. Status: PARTIAL (roadmap 06).

## 1. Location and form

- Path: `/__tpkg__/manifest.yaml` — inside the image, at a well-known
  location. No sidecars, no central service.
- Format: **YAML** (the locked convention for all authored configuration
  surfaces). Versioned JSON Schema: `schema/tpkg-manifest-v1.schema.json`.
- Integrity: the manifest is inside the image, so it is exactly as
  tamper-proof as the content and is covered by the image's digest and
  signature. Any consumer (bootstrap, dispatcher, `tfs` CLI, a foreign
  project) reads it through TFS itself.

## 2. The three blocks

Every manifest is **IDENTITY + PROVIDES + DEPENDS** on a common
provenance/trust layer.

### 2.1 IDENTITY (all kinds)

```yaml
schema_version: 1
kind: app | data | toolkit | runtime | language
name: metanorma
version: 1.2.3
producer: {tool: tebako-cli, tool_version: 0.16.0}
created: 2026-07-26T00:00:00Z
source: {src_sha256: "…", commit: "…", builder: "gha:run:123"}
sbom: {ref: "…"}                      # optional
digest:
  tree_hash: "…"                      # plaintext merkle root over the tree MINUS the
                                      # manifest path — semantic identity (CAS)
  blob_sha256: "…"                    # transport identity — see the fixed-point rule
signing: {state: unsigned | signed, keyid: "…", mechanism: openpgp}
encryption: {state: none}             # or per-part list {paths, algorithm, envelope_refs} — NEVER keys
annotations: {…}                      # free-form; unknown keys preserved
```

### 2.2 PROVIDES (kind-specialized)

**app** (executable):
```yaml
entrypoints:                          # ARRAY — multi-entry suites; N=1 for simple apps
  - name: metanorma                   # the command name (shim registers under this)
    path: /app/bin/metanorma          # inside the image
    args_default: []
    runtime_requirement: {engine: ruby, constraint: ">= 3.3, < 5.0"}
      # OPTIONAL — omit entirely for native entrypoints (see below);
      # range for pure-language; abi-line "~> 3.3.0" for native-extension payloads
    # native-extension entrypoints ALSO pin the platform line:
    runtime_requirement: {engine: ruby, constraint: "~> 3.3.0", abi: "arm64-darwin-23"}
      # abi = the runtime's own platform string the extensions were built
      # against (ruby: Gem::Platform.local.to_s). The version line and the
      # platform line are ORTHOGONAL — resolution checks both (spec 05 §5).
  platforms: [x86_64-linux-gnu, aarch64-macos]  # native-ext apps are triplet-bound;
                                                # universal only for pure-language
capabilities: {exec: true, read: true}
```
`capabilities` may also carry **`host`** (any kind): the host-access jail
the payload was built to need — a REQUEST the dispatch surfaces compose
with the user's tightening, never a grant to itself. spec 08 §4 is
normative for its shape (`default`, `mounts`, `argument_files`).

**Zero-runtime entrypoints (locked):** an entrypoint whose executable is
native (or self-contained) needs NO interpreter payload — inkscape-class
slices, static binaries, shell-free tools. Such apps declare no
`runtime_requirement` and the dispatcher mounts zero runtime payloads.
Runtimes (ruby today; python, julia, others tomorrow) are just one
payload kind among many — never an assumed layer.
A SUITE is one package with N entrypoints — the shim layer exposes one
command per entry, each dispatching to its own image and its own runtime
requirement (spec 07).

**runtime** (provides an interpreter):
```yaml
provides: {engine: ruby, version: 4.0.6, abi_line: "4.0", platform: x86_64-linux-gnu}
built_from: {src_sha256: "…", patch_set: "v0.2.8"}
env: {GEM_PATH: …}
capabilities: {exec: true, read: true, runtime: true}
```

**data** (read-only):
```yaml
mount_semantics: {suggested: /usr/share/fonts}
consumers: [any]
capabilities: {exec: false, read: true}
```

**toolkit** (native layer, e.g. gtk/qt): as runtime, minus the interpreter
contract; PROVIDES names libraries/tools other payloads DEPEND on.

### 2.3 DEPENDS (`requires:`)

```yaml
requires:
  - kind: language            # a language runtime edge
    engine: ruby
    constraint: "~> 3.3.0"
  - kind: toolkit             # a native toolkit layer
    name: gtk-layer
    constraint: ">= 3.24, < 3.25"
    triplets: [aarch64-macos, x86_64-linux-gnu]   # where this dep ships
    mount: /__layers__/gtk
  - kind: data
    name: iso-codes
    constraint: ">= 2024.1"
    mount: /__app__/share/iso-codes
```

- Dependency edges may also name **provided executables** (e.g. an app
  requiring the `inkscape` command resolves against the PROVIDES of the
  inkscape payload) — the provides/requires graph resolves capabilities,
  not just payload names.
- **MOUNT RULE (locked):** the mount point is declared in the CONSUMER's
  manifest (docker-compose volume semantics): the consumer's code knows
  where it looks for things; the provider never dictates its mount
  location. The dispatcher resolves the capability, then mounts at the
  consumer-declared path — top-level or inside the app's memfs namespace.
- **Resolution algorithm:** read manifest → build the graph → topological
  order (runtime first, layers, app last) → per node: cache hit on
  (constraint × triplet) → use, else fetch (signature-verified, spec 09)
  → verify → cache (0444 + markers) → compose the mount stack with the
  manifest's declared env → exec. The SAME signed `.tfs` artifact type at
  every graph level — one coherent algebra.

### 2.4 MATERIALIZE (`materialize:`, additive — schema_minor 1)

```yaml
materialize:
  - /lib/ssl/certs/cacert.pem    # an absolute in-image path, no '..' components
```

A top-level list (any kind may declare it; old readers ignore it under
the unknown-field rule) naming **regular files a C library must read
through its own IO** — the interpreter's patched IO never gets asked, so
the bytes must exist on the host filesystem. The OpenSSL CA cert is the
canonical entry. The driver extracts each declared path after the mounts
and the jail, before the interpreter handoff, to
`<TEBAKO_EXEC_CACHE>/resources/<image-key>/<P>` — whole-file, read-only,
digest-verified. A declared path absent from the image, or not a regular
file, is the manifest lying: boot fails by name (exit 65), never a
skipped entry. The full mechanics and the trust chain are spec 22 §4
(class R); the grammar is registered in
`docs/spec/schemas/payload-manifest.yaml`.

### 2.5 LIBRARY ALIASES (`library_aliases:`, additive — schema_minor 2)

```yaml
library_aliases:
  - name: libfoo-3.dll        # the exact bare name a loader call presents —
                              # no path separator, no drive qualifier
                              # (validated at parse)
    path: /lib/libfoo-3.dll   # the in-image absolute file the name resolves to
```

A top-level list (any kind may declare it; old readers ignore it under
the unknown-field rule) naming the ONLY bare library names that resolve
to the image's own files — the declarative half of the windows Class-L
bare-name rule (semantics: spec 22 §2.1). A loader call presenting a
bare name matching no entry is a HOST reference and passes through
untouched: host-by-default, no probing, no heuristics, no silent
fallback. Matching is verbatim and case-insensitive (the windows
loader's own comparison); the name is never extension-completed (`foo`
does not match `foo.dll`). `path` must be an absolute in-image path of
a regular file, no `..` components — an absent or non-file target is
the manifest lying (a named 65 at boot, the §2.4 precedent). A
duplicate `name` within one image is a named manifest error; two
co-mounted images declaring the same `name` is a named boot error
(authoring ambiguity, the spec 17 §2 slug precedent), never a silent
winner. No platform filter: native images are triplet-bound (§3), so an
alias is platform surface by construction. The grammar is registered in
`docs/spec/schemas/payload-manifest.yaml`.

### 2.6 CHECKS (`checks:`, additive — schema_minor 3)

```yaml
checks:
  html-xml:                        # the check name — [A-Za-z0-9][A-Za-z0-9._-]*
    entry: /bin/metanorma          # in-image executable; "self" on kind
                                   # runtime only (the runtime exe itself)
    argv: ["--type", "iso", "{scratch}/test-iso.adoc", "--agree-to-terms"]
    fixtures: /__tpkg__/check/html-xml   # exec-only; CONTENTS land at the
                                         # host scratch root
    expect: {exit: 0, files: ["test-iso.xml", "test-iso.html"]}
    timeout: 180
  layout:                          # no entry ⇒ STRUCTURAL (the data-slice
    expect:                        # shape): the engine mounts and asserts
      image_files: [/templates/org/cover.adoc]   # in-image, exist + non-empty
```

A top-level map (any kind may declare checks; old readers ignore it under
the unknown-field rule) naming the payload's own acceptance contracts —
"given my declared needs, I do my one real thing". ONE key decides the
shape (MECE, never a `kind:` flag): `entry` present ⇒ an exec check;
absent ⇒ a structural check, which declares no `argv`/`fixtures` and
asserts via `expect.image_files`. `expect.files` are scratch-relative
existence + non-empty assertions; byte-golden assertions do not exist by
construction. A malformed block is a named validation error at press /
`tfs validate` (exit 65), never discovered at run time. The semantics —
the engine, the three moments, SKIP/FAIL discipline — are spec 26; the
grammar is registered in
`docs/spec/schemas/payload-manifest.yaml`.

## 3. Platform axis (locked, vcpkg-triplet form)

`platforms` is EITHER `"universal"` (pure-ruby/data) OR an explicit list:

- `aarch64-macos`, `x86_64-macos`
- `x86_64-linux-gnu`, `aarch64-linux-gnu`
- `x86_64-linux-musl`, `aarch64-linux-musl`
- `x86_64-windows-ucrt` (`aarch64-windows-ucrt` reserved)

ONE `Platform` type (tpkg crate) owns the triplet ↔ release-asset-name
mapping (`aarch64-macos` ↔ `macos-arm64`, `x86_64-linux-gnu` ↔
`linux-gnu-x86_64`, `x86_64-windows-ucrt` ↔ `windows-ucrt64`, …) —
dispatcher, release tooling, and registry consume the SAME mapping.

## 4. The three tiers (no duplicated authority)

1. **In-image manifest** (this spec) — THE rich layer; signed/encrypted
   with the image.
2. **tpkg trailer** (spec 02) — stays minimal: mount/exec directives,
   per-slot digests, signature; REFERENCES manifests by image digest,
   never duplicates them.
3. **Registry manifest** (`tpkg-registry.yaml`, spec 04) — MIRRORS only
   resolution-relevant fields (kind, name, version, constraints,
   platforms, entrypoints) so the dispatcher resolves without downloading
   every payload.

## 5. Production and consumption

- `tebako press` / `tfs mkimage` embed the manifest at build time (kind
  inferred: app for pressed apps; data for plain images; runtime for
  runtime packages — `provides` filled from the factory's versions data).
- `tfs info` prints the manifest; `tfs stat` exposes digests.
- The dispatcher (spec 07) resolves runtime compatibility FROM the
  manifest (`runtime_requirement` vs cached runtimes' `provides`).
- The bootstrap reads entrypoint/requirement from the manifest when
  present; the trailer stays the minimal path.

## 6. The package manifest (L2, extension block type 2)

Locked 2026-07-26 (OCI model: manifest beside the layers, readable
without backend knowledge — spec 02 §5b). The package manifest is a YAML
extension block in the container, OUTSIDE every payload image. It owns
composition; payload manifests (this spec) own self-description.

```yaml
schema_version: 1
package: {name: metanorma, version: 1.2.3, producer: {…}, created: …}
entries:                          # one per invocable command (N=1 for simple apps)
  - name: metanorma
    slot: 0                       # which payload image
    entrypoint: metanorma         # which PROVIDES entrypoint inside it
    runtime_ref: ruby@3.4.2;tebako=0.15.9   # per-entry — suites/multi-runtime
  - name: mn2pdf
    slot: 1
    entrypoint: mn2pdf
    runtime_ref: ruby@3.3.7;tebako=0.15.9
jail: {…}                         # package-level request (spec 08)
env: {…}                          # package-level env (composition rules: spec 07)
mounts:                           # per-slot mount semantics (locked 2026-08-04)
  - slot: 0
    point: /__tfs__
    mode: union                   # exclusive (default) | union; cow/enc reserved
    precedence: after-env         # union only: this image shadows the env image
```

- `runtime_ref` per entry kills the 128-byte single-field limit (suites,
  multi-runtime packages); the trailer's v1 field stays for v1 loaders.
- v1-era packages without the block behave exactly as today (stub.rb /
  local conventions); the block is additive.
- `mounts` is optional; a slot without a `mounts` row mounts
  **exclusive** (spec 17's historical behavior — a duplicate point is a
  named error). `mode: union` merges the image over the images already
  mounted at `point` with declared precedence — read-only images only,
  directories merge, file conflicts resolve by precedence order (the
  env image is always lowest). `precedence` values: `after-env` (over
  the runtime's env image — the pressed-app form) or `after:<slot>`
  (over another payload slot). The union set is journaled at boot
  (spec 17 §1). `cow`/`enc` are RESERVED mode spellings on the same
  axis (the transforms law: overlays exist only in the Rust TFS) and
  are named errors until their specs land.

**toolkit** (native layer, e.g. inkscape/gtk — the distro-ports model,
spec 13 §9):
```yaml
provides:
  executables: [{name: inkscape, path: /bin/inkscape, version: 1.3}]
  libraries: [{name: xml2, path: /lib, abi: "2.12"}]
exec_tier: dynamic | wrapped | tfs-native | static   # spec 07 §8
exec_closure: [/bin, /lib, /share/inkscape]          # static tier only
capabilities: {exec: true, read: true}
```
`exec_tier` tells the dispatcher HOW to run its executables (preload
shim / link-wrapped / already-TFS-native / extract-closure) — consumers
never care which path a tool took.

## 7. The digest fixed-point rule (locked 2026-07-26)

A manifest inside the image it describes CANNOT name that image's
sha256 (self-reference has no fixed point). Therefore:

- `tree_hash` is computed over the payload tree EXCLUDING
  `/__tpkg__/` (manifest-excluded merkle root — well-defined, and it is
  what CAS/dedup/signing use). The construction (SHA-256, 4 KiB chunks,
  length-prefixed domain-separated serialization) is locked as
  `tfs-merkle-1` in `crates/tpkg/src/merkle.rs`; `tfs mkimage` and
  `tebako press` stamp it at image creation, `tfs info --verify`
  recomputes and compares (roadmap 37, SHIPPED; verification-on-READ
  inside the VFS is a later milestone).
- `blob_sha256` is NOT stored inside an embedded manifest: it lives one
  tier out — in the registry mirror (tier 3) and/or the tpkg trailer's
  per-slot digest array (tier 2, spec 02 §4). Producers fill it there;
  consumers verify the image against the OUTER record, never against a
  self-digest. An embedded manifest carrying a `blob_sha256` is read as
  advisory provenance only (e.g. the digest of the source image it was
  built from), never as a verification input (spec 15 §5).

## 8. Executable dependency edges (locked 2026-07-27)

`requires` gains a fourth edge kind for the provides/requires capability
graph (the inkscape case):

```yaml
requires:
  - kind: executable          # an executable another payload PROVIDES
    name: inkscape
    constraint: ">= 1.3"
    mount: /opt/inkscape      # consumer-declared, as always
```

Resolution: the dispatcher searches installed + available payloads for a
PROVIDES.executables entry matching `name` + `constraint` (exact name,
never a prefix/guess). Exactly one candidate → use it. Zero → named
`DependencyNotFound(executable, constraint)` with a registry hint. More
than one → named `AmbiguousProvider` listing candidates (payload,
version, source registry) — the user pins with a payload-kind edge
instead. The providing payload is mounted at the consumer-declared
`mount`; its executables run per its own `exec_tier` (spec 07 §8).
