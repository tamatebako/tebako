# Spec 23 — Declarative composition and needs resolution

**Status: PLANNED (owner-signed 2026-08-14 with the MECE/no-legacy
amendment, the record mode, and the post-bake swap channel; §8/§9).**
The §8 record mode and the §9 swap channel are IMPLEMENTED at the image
layer (`TEBAKO_JAIL=record`, `tfs needs --from-journal`, `tfs exec
--compose` — tfs crate + tfs-cli); the D1–D5 manifest/shim/press wiring
is Phase-R. Dogfooded 2026-08-14: a JVM boot recorded, drafted, and
replayed under deny with zero hand edits and zero unexpected denials.
**Amended 2026-08-25 (owner-locked, tebako#460): §13 — the composition
spectrum: per-slice `carry`, the `self-contained`/`shared-runtime`
presets, and the `platforms:` coverage assertion. PLANNED; lands with
the implementation PR (spec 14 order).**

A tebako run is a composition of one runtime (exe + env image) and N
payload slices, executed under one host-access policy. This spec makes
that composition **fully declarative**: every dependency, every mount
point, every proxied host path, and every permission is WRITTEN DOWN in
exactly one document per concern, and the system resolves the whole
stack into one effective policy before any byte of the interpreter
runs. Two laws follow:

1. **The declaration law.** Nothing is proxied, mounted, or granted
   that was not declared — by the slice (its needs), by the composition
   (the run document), or by the operator (config/CLI). The platform
   floor and the system self-surface (§5) are declared once, by the
   system, in code.
2. **The needs-check law.** A declared need is a contract. If the
   effective policy does not cover it, resolution FAILS BEFORE EXEC
   with a named error naming the slice, the path, the `why`, and the
   denying source. A need never surfaces as a mid-run EPERM, and never
   as a crash (spec 08 §2.1's floor already ended the segfault class).

## 1. The declarative stack (five documents, one owner each)

| # | Document | Owner | Answers | Consumers |
|---|----------|-------|---------|-----------|
| D1 | Slice manifest `needs:` (L1, in-image `/__tpkg__/manifest.yaml`) | the slice's feedstock | "what host surface do I need, and why" | resolver reads the mirror; press reads the image |
| D2 | Composition document `tebako.yaml` | the composition author (app developer / repo) | "what runs with what, where, how visible" | shim dispatch, `tebako run`, `tebako press` |
| D3 | Package manifest needs union (L2, baked at press) | `tebako press` | "the resolved needs of everything stitched" | bootstrap |
| D4 | Store running config (`~/.tebako/config.yaml`) | the operator | defaults, registries, per-invocation tightening | shim, bootstrap |
| D5 | CLI flags | the invoker | one-shot D2 row | `tebako run` |

SSOT: every fact lives in exactly one row. A slice's needs are NEVER
re-authored into D2/D3 — they are resolved FROM D1 (D3 bakes a copy
computed by press; the registry mirror carries D1's resolution fields,
spec 04). D4/D5 may tighten or add grants; they may not edit a slice's
declaration.

## 2. D1 — the slice `needs:` block (spec 03 amendment)

Additive manifest key (schema_minor bump; old readers ignore it, new
readers enforce). **`needs:` is the ONLY spelling** — MECE: one key,
one grammar, one semantics. The pre-spec-23 `capabilities.host` model
field is renamed to it in the same change; no alias, no dual-key union,
no compat read of the old name. A manifest carrying the old key is a
named validation error (`rename capabilities.host → needs.host`), never
a silent merge.

```yaml
needs:
  host:
    - path: /opt/vendor-tool        # absolute, or a symbolic atom below
      access: ro                    # ro | rw
      mount: /vendor/tool           # OPTIONAL: present the host path at
                                    # this VFS point (bind-mount spelling);
                                    # absent = enforcement-only passthrough
      when: [macos]                 # OPTIONAL platform filter
      optional: true                # OPTIONAL: absent at bind = silently
                                    # skipped (probe artifacts; the §8
                                    # generator marks them). Absent WITHOUT
                                    # this marker = a named error
                                    # (fail-closed).
      why: "the tool probes its install root at boot"   # MANDATORY
```

Symbolic atoms (resolved at bind, per invocation, per user — never
baked): `$HOME` / `%USERPROFILE%`, `$TMPDIR` / `%TEMP%`, `$CWD` (the
invoking cwd), `$TEBAKO_HOME` (the store — see §5). An atom that does
not resolve (e.g. `$HOME` unset with no passwd entry) fails the bind
only when the need is otherwise in force; under a platform filter
mismatch the entry is inert.

Rules (MECE, fail-closed):

- Same canonical path declared twice by one slice: the accesses must
  agree, else a named manifest error.
- Binding ANY host grant derives its strict ancestors as exact-path
  traverse reads (spec 08 §2.1): canonicalization walks pass by
  construction, so a `$CWD ro` need is sufficient for a workload that
  canonicalizes its working directory at boot (the JVM, journal-pinned
  2026-08-14).
- Ancestor/descendant declarations: the ancestor's access must be at
  least the descendant's (an ro ancestor may not hide an rw need).
- `access: rw` on a symbolic atom outside `$TEBAKO_HOME` requires the
  composition's explicit consent (§6 step 4) — declared write access to
  user surface is visible by construction.

The D1 manifest also carries one sibling declaration that is NOT a host
need: `library_aliases:` (grammar spec 03 §2.5), the windows Class-L
bare-name rule's allowlist (semantics spec 22 §2.1). It routes a bare
loader name to an in-image file; it never widens host surface, so the
needs-check law does not apply to it — the declaration law does: no
bare name is rewritten that was not declared, and an undeclared name is
a host reference by default, never a probe. The §8 record loop covers
it: the patched load path journals every bare-name verdict, so the
author discovers the spelling to declare from the journal rather than
guessing.

## 3. D2 — the composition document (the docker-compose analogue)

`tebako.yaml` — YAML, versioned JSON Schema, discovered walking up from
the cwd (evolving `.tebako-tools.yaml`; version pins ride here as
`slices[].requirement` and `defaults:`), or named with `-f`.

```yaml
version: 1
preset: shared-runtime        # self-contained | shared-runtime (§13);
                              # DEFAULT shared-runtime; presets set carry
                              # defaults, per-slice carry: overrides them
runtime:                      # the runtime slice reference + requirement
  name: ruby                  # … or the shorthand: ref: "ruby@~> 3.3"
  requirement: "~> 3.3"
  carry: false                # §13 — share the runtime (machine cache);
                              # true = the two-slot carried shape (spec 19)
  platforms: [macos-arm64, linux-gnu-x86_64]   # OPTIONAL coverage
                              # ASSERTION (§13.3): narrows, never extends
slices:                       # executable + data payload slices
  - name: metanorma           # … or ref: "metanorma@>= 2.1"
    requirement: ">= 2.1"
    carry: true               # §13 — bytes ride in the package
  - name: openjdk
    requirement: "21"
    carry: true
  - ref: "ourorg-templates@3"
    carry: false              # resolved per machine/org at first run
    platforms: universal      # assertion: fail at press if not universal
entrypoint: mnconvert         # which declared entrypoint runs
policy: deny                  # deny (DEFAULT here) | open — see §5
mounts:                       # operator bind-mounts (docker -v)
  - host: ./data
    mount: /data
    access: ro
needs:                        # composition-level additions, D1 grammar
  host:
    - { path: "$HOME/.fontist", access: rw, why: "fontist cache" }
```

`ref:` is the shorthand spelling of `name:` + `requirement:` —
`name@constraint` where the constraint is the requirement grammar; one
semantics, two spellings (the expanded form wins when both are present —
a conflict is a named validation error). `carry:` and `platforms:` are
the §13 axes; both are OPTIONAL, both default to the payload's own
declaration and the preset.

D5 (CLI) is this document written inline: `tebako run` accepts
`--image/--slice name@req`, `--runtime`, `--entrypoint`, `--mount
host:mount:ro|rw` (repeatable), `--policy open|deny`, `--need
path:ro` (repeatable). Every flag maps 1:1 onto a D2 key; `-f` and
flags compose, flags win (documented precedence, spec 07 amendment).
Press gains the §13 surface: `--mode=self-contained|shared-runtime`
(the preset; `lean|fat` accepted as deprecated aliases with a named
warning, never silent), `--carry=all|none|<name,…>` and
`--share=<name,…>` (per-slice overrides of the authored `carry:` —
explicit invocation beats authored defaults, spec 07's precedence).
`--carry=none` (even the app payload shared) is the extreme pointer
package — legal, documented as requiring a registry at first run.

## 4. D3 — the press-baked union

`tebako press` consumes D2 (+ the resolved D1 set) and bakes into the
type-2 package manifest: the resolved slice set (names, versions,
sha256s, mount points) and the **needs union** (§6). A standalone
package is then fully self-describing: the bootstrap reads ONE block at
run time, composes it with the operator env (spec 08 §4), runs the
needs-check, and hands off. Lean packages additionally union the
fetched runtime's release-manifest needs at resolve time — the same
grammar, the same code path.

**The lock (§13).** Press ALWAYS resolves the full composition closure
at build time — the runtime ref plus every payload ref, transitively —
and locks, per slice, into the L2 manifest: the concrete version, the
`carry` verdict, and the digest pin — the single `universal` digest OR
the per-target-triplet digest map (§13.3). What press tested is what
runs, always: a shared slice resolves at run time BY THE LOCKED DIGEST,
never by fresh semver (fail-closed on mismatch). Carry changes where
the bytes come from, never WHAT runs.

## 5. The default policy — jail-safe by default

- A run with ANY declaration in force (a composition document, a slice
  with needs, a CLI flag) defaults to `policy: deny`. `open` must be
  asked for by name.
- A run with NO declarations anywhere (a bare package, a bare
  dispatch) runs open with no policy exported (`never_denies`). This
  is a first-class rule of the default matrix — declarations are what
  turn the jail on — not a compatibility mode.
- Under the deny default the effective world is exactly:
  1. the VFS: the env image + every resolved slice image, read-only by
     construction (spec 11; not policy-gated at all);
  2. the **platform floor** (spec 08 §2.1) — read-only, automatic;
  3. the **system self-surface**: `$TEBAKO_HOME` (the store) and
     `TEBAKO_EXEC_CACHE` — read-only at bind; the system's own writes
     (install, materialize) happen before the policy installs. Our
     surface, our declaration, in code — never a payload's burden. The
     load-time materialization writes (spec 22's dlmap surfaces,
     windows' leave-in-place dll map included, spec 22 §2.1) are
     process-internal and never policy-gated — and neither are the
     reads-back of that same tree (a materialized interpreter's own
     home reads; tebako#502, 2026-08-30 — the per-process hex leaf is
     unauthorable as a grant, and the tree's content is image content
     by construction); the cache stays read-only to payload IO;
  4. the union of declared needs (§2), symbolic atoms resolved;
  5. the composition's `mounts:`/`needs:` (D2/D5) and operator config;
  6. `argument_files: auto` — payload arguments naming existing host
     files become ro grants (spec 08 §4, shipped in the shim).
- Everything else: EPERM/EROFS, journaled (`event=jail-deny path=…
  op=… source=…`), and the journal line names the covering
  declaration when one exists (`would-need=slice:openjdk`); when none
  does, the line says so — the operator learns the exact key to add
  from the denial itself.

## 6. The resolution algorithm (normative)

The resolver (shim in managed mode, bootstrap in standalone, `tebako
run`/`press` up front) computes, IN ORDER:

1. **Slice set**: the entrypoint slice + its transitive `depends:` (at
   the resolved versions, spec 05) + the runtime. Data slices join the
   mount composition; they declare no needs (a data slice with a needs
   block is a named manifest error).
2. **Needs union**: every slice's D1 `needs.host`, platform-filtered,
   symbolic atoms resolved at bind time. Conflict rules per §2. The
   runtime's release-manifest needs union identically.
3. **Compose with the running configuration**: `effective = needs-union
   ∪ mounts ∪ operator-grants`, then apply the operator tightening
   (`policy`, `--no-host`, explicit denies) as the intersecting scope
   (spec 08 §4's precedence: declarations request, the operator
   tightens).
4. **Needs-check**: for every declared need, verify coverage by the
   effective policy (floor and system surface count). An uncovered need
   is a NAMED RESOLUTION FAILURE before exec: `slice <name> needs
   <path> (<access>, why: <why>) — denied by <source>`, resolver exit
   code per the spec 05/07 table (pinned at implementation; never a
   fallback). A need an rw-consent rule (§2) blocks fails the same way
   with the consent path named.
5. **Export**: the authored part serializes to `TEBAKO_JAIL`
   (+`TEBAKO_JAIL_SOURCE`, +`TEBAKO_JAIL_JOURNAL`); the floor and the
   system surface are NOT serialized — every bind re-derives them
   (spec 08 §2.1), so a spawned child re-binding its inherited spec
   enforces exactly its parent's policy.

## 7. Under every combination

| Combination | Resolver | Needs sources | Policy default |
|---|---|---|---|
| Runtime slice (env image) | — (resolved, never the composer) | its release manifest | n/a |
| Executable payload slice | — (resolved) | its D1 `needs:` | n/a |
| Data payload slice | — (resolved) | none permitted | n/a |
| Standalone package (bootstrap) | bootstrap | D3 baked union ∪ lean runtime manifest ∪ operator env | deny when D3 carries any needs/mounts |
| Managed dispatch (shim) | shim | resolved slice mirrors ∪ runtime manifest ∪ D2/D4/D5 | deny when anything is declared |
| `tebako run` | CLI front end | D2/D5 ∪ slice mirrors | deny when anything is declared |
| Spawned child (spec 22 class E) | inherits `TEBAKO_JAIL`; the preload re-binds | the PARENT's composition — a child never re-resolves | the parent's |

The child row is the spec-22 consequence: a spawned payload-carried
tool (the openjdk slice's `bin/java`) declares its boot needs in ITS
manifest (`$HOME ro`, why: the darwin CFPreferences probe — the
journal-pinned 2026-08-14 chain), the parent's composition covers them
by union, and the inherited policy just works. A spawned HOST tool
(host `java` from `JAVA_HOME`) is running-configuration surface: the
composition or operator declares it (`mounts:`/`needs:` in D2/D5).

## 8. Discovery — the record mode ("perm all and monitor")

A slice author usually does not KNOW the host surface their executable
touches (the JVM's passwd-home probe was invisible until the journal
named it, 2026-08-14). Discovery is a first-class policy mode, not a
guess:

- **`policy: record`** (D2/D5; env form: `TEBAKO_JAIL=record`) — a
  third policy default beside `open`/`deny`. Under `record` every host
  passthrough check is ALLOWED and journaled:
  `event=jail-allow path=<p> op=read|write` on the same journal file.
  Nothing is denied, so the workload cannot crash from policy during
  discovery (the pre-floor JVM segfault class is impossible here).
  Spawned children inherit the spec and re-bind it — their accesses
  append to the same journal (the preload re-derivation, spec 08 §2.1).
- **The generator**: `tfs needs --from-journal <journal.log>` reads a
  record journal — folding in `jail-deny` lines too, since a denial is
  an UNMET need — and emits a draft D1/D2 `needs:` block: one entry
  per observed path, aggregated on the atom-SUBSTITUTED form so raw
  variants of one path (`/T/x` vs `/T//x`) merge; access = the
  strongest observed op (any write ⇒ `rw`); floor, store, and
  exec-cache paths EXCLUDED (they are automatic — never declared; since
  tebako#502 the exec-cache tree is not even journaled — the exclusion
  covers pre-fix journals);
  symbolic atoms re-substituted (`/Users/alice/…` → `$HOME/…`,
  longest prefix wins); relative and empty paths (cwd- or
  dirfd-relative probes — not declarable) OMITTED and counted in the
  header, so the reviewer declares `$CWD` explicitly when the payload
  wants it; ro entries that are strict ancestors of other drafted
  entries COLLAPSE (the bind's traverse set, spec 08 §2.1, covers
  their reads; an rw ancestor stays); paths absent at generation time
  marked `optional: true` (§2); each entry carrying `why: "TODO —
  observed: <r> read, <w> write"` for the author to replace with the
  real reason.
- **The human gate**: the record shows the OBSERVED MINIMUM. The author
  reviews the draft — flipping ro↔rw where production differs from the
  observation, deleting noise, filling `why` — and merges it into the
  slice manifest (D1) or the composition (D2). The generator never
  edits a manifest itself.
- `record` is a development mode: it journals at full volume and
  installs no enforcement. Shipping a composition with `policy:
  record` in force is a named lint warning; the store's audit journal
  marks record-mode runs.

## 9. Post-bake composition swap (no slice recompiles)

Slice images are immutable, content-addressed, trust-anchored — a
configuration change NEVER mutates or rebuilds one. Configuration
lives in the composition layer, which is external by construction and
therefore swappable at run time:

- **Managed mode** (shim): nothing is baked — the composition document
  and the registry mirrors are read per invocation. Swapping config IS
  editing `tebako.yaml` (or passing flags). Nothing to add.
- **Standalone packages** (bootstrap): the press-baked D3 block is the
  DEFAULT composition, and an external document overrides it — first
  hit wins: `--compose <path>` (argv before `--`), then
  `TEBAKO_COMPOSE=<path>`, then a sidecar `<package>.tebako.yaml` next
  to the package file. The override REPLACES the baked
  policy/mounts/needs/entrypoint and the requirements of SHARED slices;
  the CARRIED slice set is physical (trailer slots) — an override naming
  a carried slice the trailer does not carry is a named error, never a
  silent skip (§13).
- **Validation**: the override is the same D2 document with the same
  versioned schema; the bootstrap parses it (fail-closed, sysexits
  `EX_CONFIG`), resolves, needs-checks (§6.4), and only then execs.
- **Audit**: an external composition in force is journaled at boot:
  `event=composition source=external(<path>) sha256=<digest>` (the
  baked block in force: `source=baked`). The run's provenance always
  names which configuration ran it.
- **Trust**: a package signature covers its slices and trailer, NOT the
  composition — the composition is operator domain (the operator can
  already tighten or open the jail; spec 09 is unchanged). What the
  override cannot do is conjure slice CONTENT: unsigned slices are not
  introduced by a signed package's override; the trust checks of spec
  09 apply to any newly referenced slice exactly as at press.
- **Implemented at the image layer (2026-08-14)**: `tfs exec
  --compose <file.yaml>` speaks the D2 image-layer subset —
  `images:`/`policy:`/`mounts:`/`needs:` and nothing else (an unknown
  top-level key is a named error pointing at the shim layer);
  `$HOME`/`$TMPDIR`/`$CWD` atoms in host and image paths expand at
  compose time; needs entries lower to identity host-mount grants;
  `--compose` combined with `--image`/`--jail` is a named error (one
  composition source per run); the run's provenance rides
  `TEBAKO_JAIL_SOURCE` into the audit journal. The bootstrap/shim
  override channels above (argv `--compose`, `TEBAKO_COMPOSE`, the
  sidecar file) and the `event=composition` audit event are Phase-R.

## 10. Worked example — metanorma, fully declarative

```yaml
# openjdk feedstock manifest (D1)
needs:
  host:
    - { path: "$HOME", access: ro, when: [macos],
        why: "JVM CFPreferences locale probe (darwin); journal-pinned 2026-08-14" }
```

```yaml
# tebako.yaml (D2)
version: 1
runtime: { name: ruby, requirement: "~> 3.3" }
slices:
  - { name: metanorma, requirement: ">= 2.1" }
  - { name: openjdk, requirement: "21" }
entrypoint: mnconvert
policy: deny
mounts:
  - { host: "$CWD", mount: /work, access: ro }
```

Resolved effective policy (what the JVM child inherits):

```
deny
+ floor (/usr, /System, /Library — macOS; automatic)
+ system ($TEBAKO_HOME, $TEBAKO_EXEC_CACHE — automatic)
+ $HOME ro (openjdk's declared need, macOS-filtered)
+ $CWD ro at /work (the composition's bind-mount)
+ argument-files auto
```

The metanorma and ruby slices declare nothing — their worlds are
in-image. The spec-22 `jailed_exec` probe's hand-assembled
`deny;scratch:rw;jre:ro;home:ro` string is exactly what this stack
composes from declarations; the probe becomes `policy: deny` plus the
declarations above.

## 11. The java question, settled in this frame

There is no java RUNTIME slice and none is needed: java is a payload
slice (openjdk-feedstock images) co-mounted and spawned by the ruby
runtime via spec 22 class E. A java runtime slice (kind `runtime`,
interpreter exe + env image, launcher ABI) would slot into this spec
unchanged — its manifest would declare the JVM's needs symbolically —
and becomes relevant only when java is a package's ENTRY language.

(Amended 2026-08-30 by spec 30 — normative and implemented: the java
runtime slice now EXISTS per spec 29's wrapper pattern; the spawn-a-JVM
case routes through spec 30's spawned-dependency edge — a
`kind: runtime` DEPENDS, dispatch through the store-resident wrapper,
never a co-mounted VFS java. The toolkit-slice form above stays legal
for library-layer cases; the needs-declaration point of this paragraph
stands unchanged.)

## 12. Error discipline

- Named errors at every boundary (manifest validation, union conflict,
  needs-check, bind, override parse) — never a silent fallback
  (invariant 9).
- One spelling only: `needs:`. The old `capabilities.host` key is a
  named validation error naming the rename — no alias, no dual-key
  merge (MECE; legacy spellings are dropped, not carried).
- Zero-declaration runs follow §5's open rule as a first-class default.
- The grammar is YAML with versioned JSON Schemas (invariant 6); the
  env serialization stays the spec 08 form (authored only — §6 step 5),
  extended by exactly one token: `record` as a policy default (§8).

## 13. The composition spectrum — carried and shared (owner-locked 2026-08-25, tebako#460)

**Status: PLANNED** — lands with the implementation PR (spec 14 order).
Fat and lean are the same architecture: ONE composition pipeline whose
only dial is which resolved slices are physically embedded as trailer
slots. "Fat" meant *fully resolved with no hanging dependencies*;
"lean" was a range. The locked words make the dial obvious.

### 13.1 The two per-slice words

Every resolved slice (the runtime pair or a payload) is either:

- **carried** — its bytes ride inside the package (a trailer slot), or
- **shared** — resolved at first run into the machine cache
  (`~/.tebako`), verified against the press-time locked digest (§4's
  lock), and shared with every other package that carries or needs the
  same slice.

Carried slices seed the same cache lazily (the scoped spec 05 §4
exception: best-effort, tmp+rename, journaled, idempotent same-sha
skip, never blocks or fails the run), so both paths converge — a
self-contained download is a cache prime with a runnable side effect.
The runtime pair seeds the runtime cache the same way (spec 19's
two-slot shape).

### 13.2 The two package presets

- **self-contained** — carries its full closure: runtime exe + env
  image + every payload slice. Zero network, empty cache, one file.
  (Replaces "fat".)
- **shared-runtime** — carries the app payload(s), shares the runtime
  and any slices marked shared. (Today's packed-mn "lean"; the DEFAULT
  preset.)

`lean`/`fat` remain accepted as deprecated aliases (a named warning,
never silent) mapping to `shared-runtime`/`self-contained`. A preset
sets the DEFAULT carry verdict for every slice; a slice's authored
`carry:` overrides it; D5's `--carry`/`--share` override the document.

### 13.3 The `platforms:` coverage assertion

The truth lives in exactly two places, and the composition is not one
of them:

- **The payload declares** its own coverage (spec 03 §3, already
  locked): `platforms: universal` OR an explicit triplet list.
  "Universal" = pure-language/data bytes, one artifact for every
  triplet. Partial independence (arch-free, libc-free) is expressed by
  ENUMERATION — list every triplet the bytes serve; honest and dumb.
- **The registry mirrors** concrete rows only (spec 04): a `universal:`
  row OR per-triplet rows — never patterns, never both. Wildcard
  pattern rows (`linux-*-*`) are REJECTED; expansion belongs to
  authored manifests at publish time.

The composition's per-slice `platforms:` key is an ASSERTION, checked
fail-closed against the payload's declared/mirrored coverage:

- Omitted = the payload's declaration rules (the common case: runtimes
  and native payloads are triplet-bound by nature).
- Present = press/dispatch verifies the assertion is COVERED by the
  declaration. A lack of coverage — for the assertion or for the host
  triplet — is a named error naming the slice, the triplet, and the
  declared coverage. Never a silent nearest-platform fallback
  (spec 00 §9). The assertion narrows; it NEVER extends.

The lock (§4) records the outcome: per slice, the single `universal`
digest or the per-target-triplet digest map. Run-time resolution is
then: host triplet → locked digest → carried-slot verify /
shared fetch+verify. Carry/shared stays orthogonal: a slice of any
coverage class may be carried or shared.

**The honesty proof.** A `universal` claim with native bytes inside is
a lie that only fails on someone's machine — unless checked. The
spec-26 payload-check framework is the proof: feedstock CI runs a
universal slice's in-image acceptance check on EVERY target triplet;
native bytes in a "universal" image fail there, before publication.

### 13.4 Rules that make this safe

- The L2 lock records the digest of every slice, carried or shared —
  run-time resolution verifies against it, fail-closed. Carry/shared
  never changes WHAT runs, only where the bytes come from.
- Jail defaults are unaffected by carry/shared: mount points and
  permissions are declared identically either way (§2/§5).
- Trust is uniform across both paths: the same sha256 anchors, the same
  unsigned-is-loud + journal rules, the same `TEBAKO_REQUIRE_SIGNED=1`
  fail-closed; verification at fetch/install — never per run.
- Transitive payload DEPENDS (payloads needing payloads) resolve
  through the same registry path with cycle detection (spec 18 §5.6);
  mount-point arbitration and jail-by-default are this spec's §5–§7
  unchanged.

### 13.5 SSOT and error additions

| Value | Owner |
|---|---|
| `carry` / preset / `platforms:` assertion grammar | THIS section (D2 §3 mirrors it) |
| The lock's digest-map shape | the L2 package-manifest schema (spec 02 §5b type-2 block; `schema/tebako-compose-v1.schema.json` stays the D2 document's) |
| Coverage declaration | spec 03 §3 (unchanged — the assertion references it) |
| Registry row shape (concrete only) | spec 04 (amended: one sentence) |
| Lazy-seed mechanics | spec 05 §4 (amended: the scoped exception) |
| Two-slot carried runtime | spec 19 (amended) + tebako#458 |
| Lean/fat alias warning | the named-warning vocabulary (invariant 9); exit codes unchanged — carry choices never gate a run |

## 14. The settings registry — one setting, three channels, one SSOT (owner-directed 2026-08-26, tebako#400)

Every user-facing setting is declarable through up to THREE channels:
the CLI flag, the environment variable, and the compose-document key.
Ad-hoc per-channel wiring is how the channels drift into two meanings,
so declaration and resolution each live in exactly one place:

- **Declaration:** `tpkg::settings` (the registry) names each setting
  ONCE — config key, env spelling, CLI flag, default, one doc line —
  and declares WHICH channels carry it. A machine-level knob (a cache
  root) has no business in a git-shared document; a package-policy bit
  the press bakes MUST be declarable in the repo-carried document.
  `--help`, the schema, and the docs render from the registry; a
  setting present on a channel but absent from the registry is a bug on
  arrival (invariant 10).
- **Resolution:** `tpkg::settings::resolve_bool` (boolean settings) and
  `tpkg::settings::resolve_sign` (the `sign` setting — its CLI channel
  carries an optional `=<keyid>`), fixed precedence
  **CLI → environment → compose document → default**. Every channel is
  tri-state (present-true / present-false / absent), so a repo-declared
  `quiet_notices: true` stays overridable per invocation
  (`--no-quiet-notices`, `TEBAKO_QUIET_NOTICES=0`). An env value that
  does not parse is a NAMED error, never a silent default
  (invariant 9).
- **Baking:** settings whose semantics belong to the PACKAGE (not the
  machine) bake into the trailer at press; run-time reads the trailer,
  never the channels — a pressed package behaves identically on every
  machine (the developer's declaration is environment-independent by
  construction, and the repo carries it).

The registry's citizens:

| setting | CLI | env | compose key | baked form |
|---|---|---|---|---|
| `quiet_notices` | `--quiet-notices` / `--no-quiet-notices` | `TEBAKO_QUIET_NOTICES` | `quiet_notices:` | `TPKG_FLAG_QUIET_NOTICES` (bit 3, spec 02 §5a) — suppress the unsigned-legacy-trailer warning (spec 09) and the progress lines (spec 06 §5) on every run |
| `sign` | `--sign[=<keyid>]` / `--no-sign` | `TEBAKO_SIGN` | `sign:` | `TPKG_FLAG_SIGNED_V2` (bit 1) + the v2 chain-of-trust extension (spec 02 §4) — sign the package trailer at press (spec 09 §9); unsigned v1 stays the default, an opt-out that overrides a lower channel is loud (warning + audit journal) |

Every NEW setting declares its channels in the registry first; existing
channels migrate into the registry as they are touched.
