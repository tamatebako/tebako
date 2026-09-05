# Spec 32 — Payload as a spawned dependency

**Status: NORMATIVE (drafted 2026-09-05; implemented 2026-09-05 —
`tpkg` (the `kind: executable` edge + schema_minor 5, the lock's
payload rows, the payload-store SSOT), `tebako-shim` (dispatch-time
edge resolution + the spawn-lock payload rows),
`tebako-driver` (the payload-spawn plan composition + the hereditary
jail ceiling), `tebako-bootstrap` (the lock's spawned payload-row
resolution), `tebako-cli` (install/compose arms + the expose shim
registration + the tightening export), `tebako-pkg`/`tebako-info` (the
press-time two-level cross-checks + the render arms)).** Amends spec 03 §8 (the
`kind: executable` edge gains the `expose:` spawn form), spec 23 §13.6
(the lock's `spawned[]` rows gain the payload row), spec 30 §0 (the
dispatch-case split becomes three-way). No wire-format change; no
trailer change. Requires spec 30 (the whole spawn machinery is reused,
never duplicated) and spec 29 (the depended runtime's exe is the
wrapper). First instance: metanorma's xml2rfc edge (TODO.python/03),
retiring the host-python convention whose fragility is the chocolatey
python313/1603 saga (PROGRESS/25) and whose universality analysis is
PROGRESS/27 §6.

## 0. The three dependency dispatch cases (MECE, forever)

A payload's DEPENDS edge resolves to one of exactly three dispatch
forms:

- **Co-mounted slice** — `kind: toolkit` / `kind: data`, and the
  co-mount surface of `kind: executable` (spec 03 §2.3/§8): the
  provider's image joins the parent's mount stack at the
  consumer-declared mount; its executables run per their declared
  `exec_tier` (spec 07 §8).
- **Spawned runtime** — `kind: runtime` (spec 30): the depended
  runtime's wrapper exe executes FROM THE STORE as a child process;
  never co-mounted.
- **Spawned payload (THIS spec)** — `kind: executable` with `expose:`:
  the provider payload is dispatched as a child process **through its
  own full spec-17 dispatch** — its own `kind: language` edge resolves
  the runtime pair, its own image co-mounts in the CHILD, the exposed
  name resolves against its own entrypoints.

The form discriminator is the matched entry's own declaration
(§1): an entry carrying `runtime_requirement` (spec 03 §2.2) is
interpreter-needing and spawn-eligible; a runtime-less entry (a
toolkit executable, a native app entrypoint) is kernel-executable and
its surface is the exec tier — spawning it would be a degenerate
dispatch and is a named error, never a fallback. The spawn case exists
because an interpreter-needing console script (xml2rfc on python, any
gem console script on a sibling ruby) has NO co-mountable exec form on
every platform: the windows host tier materializes exes, not scripts
that need an interpreter plus a site-packages tree. The child IS a
kernel-visible store exe on every platform — the windows path exists
by construction, and the POSIX-only fallback (toolkit mount + shell
script) is REJECTED: the full-toolkit goal is all-platform.

## 1. The edge grammar (spec 03 §8 amendment — schema_minor 5)

```yaml
requires:
  - kind: executable          # an executable another payload PROVIDES
    name: xml2rfc             # the capability — exact-name match against the
                              # provider's provides.executables (toolkit) OR
                              # provides.entrypoints[].name (app)
    payload: xml2rfc          # OPTIONAL — a by-name provider pin (the
                              # AmbiguousProvider escape hatch)
    constraint: ">= 3.34"     # the spec 05 §5 version classes
    mount: /opt/xml2rfc       # OPTIONAL — the VFS surface (co-mount),
                              # consumer-declared as always
    expose: [xml2rfc]         # OPTIONAL — the spawn surface (§2)
    critical: true            # the evolution law's flag — REQUIRED when the
                              # payload's function needs this edge
```

The two OPTIONAL keys are **orthogonal axes** (invariant 4): `mount`
opens the VFS surface, `expose` opens the spawn surface; either may
hold alone, both may hold together. The rules:

- An edge with NEITHER `mount` nor `expose` is a named manifest error
  (a contentless edge — it declares a dependency that opens no
  surface).
- Capability resolution is spec 03 §8's, unchanged in outcome and
  widened in source: exact-name match against `provides.executables`
  AND `provides.entrypoints[].name`; zero candidates → named
  `DependencyNotFound`; more than one → named `AmbiguousProvider`
  listing candidates. `payload:` pins the provider by name — the §8
  escape hatch, extended to the spawn form; press cross-checks the
  named payload indeed provides the capability.
- `expose:` present selects the spawn form: every exposed name must
  resolve to a provider entrypoint CARRYING `runtime_requirement`.
  A matched entry that is runtime-less (a toolkit
  `provides.executables` entry, a native app entrypoint) is a named
  resolution error at press/install — authoring confusion, never a
  silent exec-tier fallback.
- `expose:` present requires `name ∈ expose` — the depended capability
  must be surfaced. A violation is a named manifest error at parse.
- `mount:` present co-mounts the provider image at the
  consumer-declared path; the provider's NON-exposed executables run
  per their own `exec_tier`. **Precedence (locked):** an exposed name
  NEVER takes the exec-tier path — the spawn dispatch owns it. A
  non-exposed duplicate name across two co-mounted images stays spec
  17 §2's dependency bin-dir order.
- The `expose:` name grammar is spec 30 §1's: bare command names — no
  path separator, no drive qualifier, never repeated. An `expose:` key
  on any other edge kind is a named manifest error.
- The expose × own-entrypoint collision refusal is spec 30 §3's,
  extended one class: a name colliding between the payload's own
  entries and ANY spawned edge's expose list (runtime or executable)
  is a named manifest error at press.
- **Producer obligation (locked):** a payload whose function requires
  the edge marks it `critical: true` (the schema evolution law's
  critical flag). A reader predating schema_minor 5 cannot dispatch
  the edge; skipping it silently would amputate the payload's
  function — the critical flag turns that skip into the named refusal.

## 2. The child plan (normative)

The spawn surface is spec 30 §2's — both spellings, one semantics,
with the payload case riding the SAME machinery:

- **In-band.** The parent runtime's spawn interception recognizes the
  spawned command as an expose-listed entry of an executable edge (an
  undeclared command name is NEVER rewritten — it falls to the host
  PATH, journaled as a host spawn). The rewrite composes **the
  provider payload's own managed dispatch** as the child — identical
  in shape to what `tebako run <provider>:<entry> -- <args…>` composes
  today:

  ```
  <store>/runtimes/…/tebako-runtime-…-<provider's engine>-…[.exe]
      --tebako-image <provider image>:<slot|->:<mount> …
      --tebako-entry <exposed name> <args…>
  ```

  The child exe is the provider's OWN resolved runtime's
  store-resident wrapper (kernel-visible by construction); the
  provider image co-mounts in the child per the provider's standard
  dispatch composition; the entry resolves against the provider
  image's `provides.entrypoints` — the FIRST `--tebako-image` triple
  (spec 17 §1's app-payload rule), never the env image's. The
  declared `args_default` composes as in any dispatch of the provider.
- **Operator-mediated.** No new verb: `tebako run <provider>:<entry>`
  already IS this dispatch. The spec adds only that an in-band spawn
  of an expose-listed name composes exactly it.
- **The plan-time manifest source (locked).** The driver's spawn map
  reads the provider's entrypoint declarations from the STORE
  MANIFEST MIRROR (the host-side `payloads/<name>/<version>.manifest.yaml`;
  embedded wins, else synthesized loudly — spec 05's mirror rule). No
  image mounts at plan time. One completion path, locked: a record the
  BOOTSTRAP seeded (lock-row resolution lands the image + the trust
  anchor; the size-gated loader carries no image reader, so no mirror)
  is completed ONCE at the first spawn resolution — the driver
  scratch-mounts the provider image, reads the embedded manifest
  (authoritative, spec 05 §3), and writes the mirror through the
  store's tmp+rename discipline. This is the install-time extraction
  deferred, not a plan-time read: the steady state stays mirror-only,
  and the unpinned capability scan never takes this path (a scan
  candidate without a mirror is the named damaged-record error).
- **Argument carry-over (locked):** spec 30 §2's rule applies
  verbatim — an argument lexically resolving under one of the PARENT's
  mounts is carried by re-mounting that image in the child AT THE SAME
  mount point, the argument string unrewritten; on the exec-cache
  visibility class (windows) the file materializes to the exec cache
  and the argument rewrites to the host path. xml2rfc reading the
  parent's in-VFS `.xml` is this rule's first instance.
- **The child environment (locked):** spec 30 §2's strip applies
  verbatim (the parent's whole injection surface dies with the plan);
  the plan then sets `TEBAKO_RUNTIME_IMAGE` to the provider runtime's
  env image. The child boot recomputes every `TEBAKO_MOUNT_*` from its
  OWN mounts (spec 17 §2's table) — none are inherited.
- **Recursion and the child's spawn lock (locked).** A provider
  payload may itself carry spawn edges (runtime or executable). The
  child has NO loader — so the parent's dispatch resolves the
  provider's spawn edges TRANSITIVELY (the loader fetches the
  provider, reads its L1 mirror, resolves its edges cache-or-fetch per
  spec 05 §5), and the plan's env-op block SETS a fresh
  `TEBAKO_SPAWN_LOCK` for the child carrying the provider's own
  resolved pins (§5's row grammar, composed recursively). The strip
  deletes the parent's lock; the child never sees it. The edges join
  the one dependency graph; a cycle through them is the resolver's
  existing named cycle error, never a recursion guard.

**The plan FFI.** `tebako_spawn_runtime_plan(command, args, …)` (spec
30 §2) is the whole wire, unchanged: the DRIVER's spawn map now keys
exposed payload entries alongside exposed runtime entries; a payload
hit plans the §2 composition above. Return contract unchanged: **0**
not ours, **1** planned (child exe + NUL-packed argv + env-op block),
**-1** named spawn error.

## 3. Shim registration and the PATH launcher tier

Spec 30 §3 applies verbatim with "depended runtime's entrypoints"
widened to "depended entries — a runtime edge's exposed entrypoints
(spec 30) or an executable edge's exposed provider entries (THIS
spec)":

- Install registers one shim per exposed name (spec 07 §1's argv0
  model; the per-entrypoint active flag applies). Dispatch of such a
  shim IS the §2 in-band dispatch: argv0 → the CONSUMER payload's
  version chain → the executable edge's resolution → the provider's
  dispatch as the child.
- The PATH launcher tier (spec 22 §3's wrap-bin) plans exposed payload
  entries at the parent's boot exactly like exposed runtime entries: a
  plan that succeeds bakes the exec script; a name that fails to plan
  bakes the fail-closed exit-69 stub — never a host-PATH fallback.
  Windows has no launcher tier; the argv bridge is the whole surface
  there.

## 4. Jail interaction (union-of-needs, three-way, ceiling-hereditary)

The child dispatch computes its jail from ITS OWN declared needs —
THREE manifests: the consumer payload's (the payload carrying the
spawned command), the provider payload's, and the provider runtime's —
per spec 23 §5/§6, and never inherits the parent's grants verbatim.
The DRIVER computes the union at plan time (`tpkg::jail::union` over
the three); the jail trio is deleted from the child env and, when the
union is not trivially open, set fresh with `TEBAKO_JAIL_SOURCE` =
`spawn-edge:<consumer>:<provider>`. When NO side declares needs the
child carries no jail env at all. Default deny stands.

- **The platform floor (spec 08 §2.1) applies identically.** Its
  rationale IS this case: a spawned interpreter cannot boot without
  the per-platform read-only floor, so the child's deny-mode policy
  gains it exactly as a primary dispatch does.
- **Operator tightening is HEREDITARY (locked).** Spec 08 §2's
  precedence — user policy always wins — extends transitively: every
  tightening directive on the parent's dispatch (`--jail`, `--no-host`,
  `--mount …:ro`) intersects the child's recomputed union. Tightening
  only removes grants, so the intersection is always well-defined; a
  spawned child NEVER holds a grant the operator denied the parent.
  (A child spawned from a jailed parent under `--no-host` runs
  host-blind even if its three manifests declare needs — the needs
  surface in the journal for the discovery loop, never as grants.)
  The wire: `TEBAKO_JAIL_TIGHTENING` carries the parent's raw
  user-tightening env spec (exported by the shim's dispatch and by
  `tebako run` whenever a user tightening exists); the driver captures
  it at boot and intersects it over every spawned child's union — a
  `record` tightening dominates wholesale, mirroring
  `tpkg::jail::effective` — and the plan's env-op block never deletes
  the key, so the ceiling inherits onward to deeper spawns.
- **Record-mode attribution.** The child's jail journal lines carry
  the `spawn-edge:<consumer>:<provider>` source, so
  `tfs needs --from-journal` (spec 24) attributes the child's records
  to the correct side of the edge — the discovery loop (spec 23 §8)
  works identically for spawned payloads.
- **Extension point (reserved semantics, no grammar yet).** An
  edge-local jail CEILING may ride the executable edge in a future
  MINOR (narrowing-only: intersect with the union, never widen; still
  subject to the hereditary operator ceiling). This spec pre-locks the
  semantics so the grammar has room; the key is NOT defined here and
  readers MUST NOT invent it.

## 5. Resolution placement and the spawn lock (locked)

Spec 30 §2's placement rule holds with one widening: at DISPATCH the
loader resolves the payload's `kind: executable` expose edges
TRANSITIVELY — the provider payload (cache hit or download per spec 05
§5, digest-pinned), the provider's own `kind: language` edge (the
runtime pair), and recursively the provider's own spawn edges (§2) —
and exports the pins in `TEBAKO_SPAWN_LOCK`. The row grammar gains the
payload row, `;`-joined in manifest order with the runtime rows, one
row per edge:

```
<engine>=<language_version>:<tebako_version>                       ; runtime row (spec 30 — unchanged)
<payload>@<payload_version>=<engine>=<language_version>:<tebako_version>   ; payload row (THIS spec)
```

The discriminator is MECE: a subject containing `@` is a payload row
(payload names and versions never collide with the bare-engine
subject grammar — `@` appears in neither engine names nor the runtime
row's subject); a bare subject is spec 30's runtime row. The payload
row's value nests the provider's resolved runtime pair EXACTLY as a
runtime row spells it. The lock carries identity pins only; the
digest anchor stays the store's `.sha256` sidecar (spec 05 §3), as for
the runtime rows — verification happens at fetch/install, never per
run.

**The install verb (locked).** `tebako install` of the consumer
resolves the FULL transitive spawn closure eagerly — provider payload
into `payloads/`, nested runtime pair into `runtimes/`, recursively —
so a warm install is an offline-capable first run (`TEBAKO_OFFLINE=1`
discipline unchanged: cache-or-named-error).

At SPAWN the driver resolves CACHE-ONLY and never downloads: a locked
payload row resolves to exactly the pinned provider version (and its
pinned runtime pair); a pinned provider or runtime gone from the store
is a named error, never a silent re-pick. An UNLOCKED edge (a
hand-rolled dispatch, a test harness) resolves the newest compatible
CACHED provider and runtime, the picks journaled.

The edge SOURCE per loader mirrors spec 30 §2: the shim reads the
installed payload's L1 manifest mirror; the bootstrap reads the L2
lock's `spawned[]` payload rows (§6) — press mirrors the edges into
the lock and `tebako-pkg validate` cross-checks the mirror against
slot 0's L1, the tebako#494 class.

## 6. The lock rows (spec 23 §13.6 amendment)

A payload's expose-carrying `kind: executable` edge joins the lock's
`spawned[]` like a runtime edge — press version-locks the edge and
records the dispatch channel. One row per edge:

```yaml
lock:
  spawned:
    - payload: xml2rfc              # the resolved provider payload's name
      constraint: ">= 3.34"         # mirrored from the L1 edge (press validate cross-checks)
      expose: [xml2rfc]             # mirrored from the L1 edge
      version: "3.34.0"             # the press-time pick
      carry: true                   # the same two words as §13.1
      image: {slot: 3, sha256: …}   # carried: the provider image's trailer slot + digest pin
                                    # (universal-or-triplet-map per §13.3)
      runtime:                      # the provider's OWN language edge, resolved — the
        engine: python              # §13.6 runtime row, nested (implementation rides when
        constraint: ">= 3.10"       # the provider's L1 edge names one; constraint mirrored
        version: "3.13.15"          # from the PROVIDER's manifest, cross-checked against it)
        tebako: "2.1.10"
        carry: true
        exe:   {slot: 4, sha256: …}
        image: {slot: 5, sha256: …}
      # carry: false instead: no slots — `source:` records the press-resolved
      # download base for the payload image and (nested) the runtime pair,
      # replayed verbatim per §13.6's shared-row rule; digest pins stand.
```

The press cross-check is TWO-LEVEL (the tebako#494 class, extended):
the row's `constraint`/`expose` mirror the CONSUMER's L1 edge, and the
nested `runtime:` row's `constraint` mirrors the PROVIDER's L1
`kind: language` edge — press reads the provider's manifest to check
it. Run time reads ONE block: the loader resolves each row into the
store — the provider image into `payloads/`, the nested runtime pair
into `runtimes/` — carried → slot extract + digest-verify + install;
shared → cache hit on the locked identity + digest, else fetch from
the row's `source` + verify + install — and exports the §5 lock. A
carried spawned payload is never mounted BY THE PARENT: its slots ride
the lock's claimed-slot set, invisible to the parent's mount
composition (the CHILD mounts them fresh). Offline discipline is spec
05's, unchanged.

## 7. Failure modes

Every spec 30 §5 mode applies, generalized; the payload case adds:

- Provider payload absent or incompatible at spawn: the resolver's
  named error (the spec 06 exit 69 shape), BEFORE the child exists.
  With `TEBAKO_OFFLINE=1`, cache-or-named-error.
- The provider's runtime absent or incompatible — including the
  ABI-line mismatch class (spec 28; a native-wheel provider on the
  wrong interpreter line, TODO.python/04's named error) — is the
  resolver's named error, never a segfault, before the child exists.
- An `expose:`d entry whose provider resolves but whose name is
  undeclared in the provider's `provides.entrypoints`: a named
  validation error at press/install (the tebako#494 cross-check
  class); at spawn, the outruns rule is spec 30 §5's — a named spawn
  error naming the consumer payload and the command (the PATH launcher
  tier bakes it as the exit-69 stub at boot).
- An `expose:` list whose match is runtime-less (a toolkit executable
  or native entrypoint): the §1 named resolution error at
  press/install — never a silent exec-tier fallback.
- An executable edge declaring neither `mount` nor `expose`: the §1
  named manifest error at parse.
- NEVER a silent host fallback: an expose-listed spawn that cannot
  resolve through tebako fails named; falling through to a system
  python is the bug class this spec exists to kill (PROGRESS/25's
  chocolatey python313/1603 saga is the standing example).
- The provider runtime's `contract_version` is negotiated fail-closed
  by the child's boot exactly as for a primary runtime (spec 06 §6;
  exit 75).
- Spawn-time resolution is CACHE-ONLY (§5): the driver never
  downloads, and `TEBAKO_OFFLINE` changes nothing.

## 8. What this settles and retires

- spec 03 §8's `kind: executable` edge gains its implemented form:
  the enum seat lands in schema_minor 5; the capability match widens
  to `provides.executables` ∪ `provides.entrypoints[].name`; `mount`
  and `expose` are the two orthogonal surfaces (co-mount per §8's
  original semantics, spawn per THIS spec).
- The host-python xml2rfc convention (PROGRESS/25) is retired:
  metanorma's ietf edge becomes a hermetic spawned payload, identical
  on all four platform classes, offline-capable with a warm cache.
- The POSIX-only toolkit-mount + shell-script fallback is REJECTED
  (§0) — the child dispatch is the all-platform form by construction.
- The per-runtime spawn patches stay MECHANISM, not policy (spec 30
  §6's rule, unchanged): they implement §2's rewrite; the POLICY —
  what may be spawned, where it resolves, what jail the child gets —
  is declarative and lives in spec 30 + THIS spec.

**The audience mapping (locked):** the feedstock maintainer (audience
3 — runnable-payload developers) authors the edge; the press operator
chooses `carry` (a fat package grows by the provider image plus the
nested runtime pair — spec 23 §13.1's existing choice, never a new
mechanism); the end user (audience 1) sees nothing new — dispatch
resolves, the cache warms at install, offline works. No tebako-side
machinery compiles or configures on any of audiences 1–3's machines
(the five laws hold; this spec adds declarations, not machinery).
