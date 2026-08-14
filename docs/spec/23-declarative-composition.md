# Spec 23 — Declarative composition and needs resolution

**Status: PLANNED (owner-signed 2026-08-14 with the MECE/no-legacy
amendment, the record mode, and the post-bake swap channel; §8/§9).**

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
- Ancestor/descendant declarations: the ancestor's access must be at
  least the descendant's (an ro ancestor may not hide an rw need).
- `access: rw` on a symbolic atom outside `$TEBAKO_HOME` requires the
  composition's explicit consent (§6 step 4) — declared write access to
  user surface is visible by construction.

## 3. D2 — the composition document (the docker-compose analogue)

`tebako.yaml` — YAML, versioned JSON Schema, discovered walking up from
the cwd (evolving `.tebako-tools.yaml`; version pins ride here as
`slices[].requirement` and `defaults:`), or named with `-f`.

```yaml
version: 1
runtime:                      # the runtime slice reference + requirement
  name: ruby
  requirement: "~> 3.3"
slices:                       # executable + data payload slices
  - name: metanorma
    requirement: ">= 2.1"
  - name: openjdk
    requirement: "21"
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

D5 (CLI) is this document written inline: `tebako run` accepts
`--image/--slice name@req`, `--runtime`, `--entrypoint`, `--mount
host:mount:ro|rw` (repeatable), `--policy open|deny`, `--need
path:ro` (repeatable). Every flag maps 1:1 onto a D2 key; `-f` and
flags compose, flags win (documented precedence, spec 07 amendment).

## 4. D3 — the press-baked union

`tebako press` consumes D2 (+ the resolved D1 set) and bakes into the
type-2 package manifest: the resolved slice set (names, versions,
sha256s, mount points) and the **needs union** (§6). A standalone
package is then fully self-describing: the bootstrap reads ONE block at
run time, composes it with the operator env (spec 08 §4), runs the
needs-check, and hands off. Lean packages additionally union the
fetched runtime's release-manifest needs at resolve time — the same
grammar, the same code path.

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
     surface, our declaration, in code — never a payload's burden;
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
  record journal and emits a draft D1/D2 `needs:` block: distinct
  canonical paths, access = the strongest observed op (write > read),
  floor and system-surface paths EXCLUDED (they are automatic — never
  declared), symbolic atoms re-substituted (`/Users/alice/…` →
  `$HOME/…`, the invoking cwd → `$CWD`), each entry carrying
  `why: "TODO — observed <n> <op> access(es)"` for the author to
  replace with the real reason.
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
  policy/mounts/needs/entrypoint (and, for lean packages, the slice
  requirements); a fat package's slice SET is physical (trailer slots)
  — an override naming a slice the trailer does not carry is a named
  error, never a silent skip.
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
