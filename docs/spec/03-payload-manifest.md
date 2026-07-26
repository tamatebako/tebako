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
  tree_hash: "…"                      # plaintext merkle root — semantic identity (CAS)
  blob_sha256: "…"                    # transport identity
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
  platforms: [x86_64-linux-gnu, aarch64-macos]  # native-ext apps are triplet-bound;
                                                # universal only for pure-language
capabilities: {exec: true, read: true}
```
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
```

- `runtime_ref` per entry kills the 128-byte single-field limit (suites,
  multi-runtime packages); the trailer's v1 field stays for v1 loaders.
- v1-era packages without the block behave exactly as today (stub.rb /
  local conventions); the block is additive.

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
