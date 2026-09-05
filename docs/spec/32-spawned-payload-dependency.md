# Spec 32 — Payload as a spawned dependency

**Status: PLANNED (drafted 2026-09-05; design-first per spec 14 — the
implementation lands with the TODO.python/03 chain: `tpkg` (the
`kind: executable` edge + schema_minor 5), `tebako-shim`
(dispatch-time edge resolution + the spawn-lock payload rows),
`tebako-cli` (install/compose arms + the expose shim registration),
`tebako-driver` (the payload-spawn plan composition), `tebako-pkg`
(the press-time cross-checks)).** Amends spec 03 §8 (the
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

- **Co-mounted slice** — `kind: toolkit` / `kind: data` / a
  `kind: executable` edge WITHOUT `expose:` (spec 03 §2.3/§8): the
  provider's image joins the parent's mount stack at the
  consumer-declared mount; its executables run per their declared
  `exec_tier` (spec 07 §8).
- **Spawned runtime** — `kind: runtime` (spec 30): the depended
  runtime's wrapper exe executes FROM THE STORE as a child process;
  never co-mounted.
- **Spawned payload (THIS spec)** — `kind: executable` WITH `expose:`:
  the provider payload is dispatched as a child process **through its
  own full spec-17 dispatch** — its own `kind: language` edge resolves
  the runtime pair, its own image co-mounts in the CHILD, the exposed
  name resolves against its own entrypoints. Never co-mounted into the
  parent.

The third case exists because an interpreter-needing console script
(xml2rfc on python, any gem console script on a sibling ruby) has NO
co-mountable exec form on every platform: the windows host tier
materializes exes, not scripts that need an interpreter plus a
site-packages tree. The child IS a kernel-visible store exe on every
platform — the windows path exists by construction, and the POSIX-only
fallback (toolkit mount + shell script) is REJECTED: the
full-toolkit goal is all-platform.

## 1. The edge grammar (spec 03 §8 amendment — schema_minor 5)

```yaml
requires:
  - kind: executable          # an executable another payload PROVIDES
    name: xml2rfc             # the capability (PROVIDES.executables match)
    constraint: ">= 3.34"     # the spec 05 §5 version classes
    expose: [xml2rfc]         # OPTIONAL — the depended entries this payload
                              # surfaces as spawned children (THIS spec)
    # mount: …                # ILLEGAL when expose is present (nothing
                              # co-mounts into the parent)
```

- Capability resolution is spec 03 §8's, unchanged: exact-name
  PROVIDES match, zero candidates → named `DependencyNotFound`, more
  than one → named `AmbiguousProvider`. `expose:` changes the DISPATCH
  of the resolved edge, never its resolution.
- Every `expose:` name MUST be a declared entrypoint of the resolved
  provider payload (its L1 `provides.entrypoints[].name`). Press and
  install cross-check (the tebako#494 class; §7's outruns rule).
- `mount:` on an expose-carrying executable edge is a named manifest
  error — parallel to the runtime edge carrying no mount key at all
  (spec 30 §1). With `expose:` absent, spec 03 §8's co-mount semantics
  stand untouched.
- The `expose:` name grammar is spec 30 §1's: bare command names — no
  path separator, no drive qualifier, never repeated. An `expose:` key
  on any other edge kind is a named manifest error.
- The expose × own-entrypoint collision refusal is spec 30 §3's,
  extended one class: a name colliding between the payload's own
  entries and ANY spawned edge's expose list (runtime or executable)
  is a named manifest error at press.
- **Producer obligation (locked):** a payload whose function requires
  the spawn edge marks the edge `critical: true` (the schema
  evolution law's critical flag). A reader predating schema_minor 5
  cannot dispatch the edge; skipping it silently would amputate the
  payload's function — the critical flag turns that skip into the
  named refusal.

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
  (spec 17 §1's app-payload rule), never the env image's.
- **Operator-mediated.** No new verb: `tebako run <provider>:<entry>`
  already IS this dispatch. The spec adds only that an in-band spawn
  of an expose-listed name composes exactly it.
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
- **Recursion.** A provider payload may itself carry spawn edges
  (runtime or executable): its own dispatch resolves and exports its
  own `TEBAKO_SPAWN_LOCK` in the child (§5), and the chain composes.
  The edges join the one dependency graph; a cycle through them is the
  resolver's existing named cycle error, never a recursion guard.

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

## 4. Jail interaction (union-of-needs, three-way)

The child dispatch computes its jail from ITS OWN declared needs —
now THREE manifests: the consumer payload's (the payload carrying the
spawned command), the provider payload's, and the provider runtime's —
per spec 23 §5/§6, and never inherits the parent's grants verbatim.
The DRIVER computes the union at plan time (`tpkg::jail::union` over
the three); the jail trio is deleted from the child env and, when the
union is not trivially open, set fresh with `TEBAKO_JAIL_SOURCE` =
`spawn-edge:<consumer>:<provider>`. When NO side declares needs the
child carries no jail env at all. Default deny stands; spec 23 §8's
record mode covers the discovery loop identically.

## 5. Resolution placement and the spawn lock (locked)

Spec 30 §2's placement rule holds with one widening: at DISPATCH the
loader resolves the payload's `kind: executable` expose edges
TRANSITIVELY — the provider payload (cache hit or download per spec 05
§5, digest-pinned) AND the provider's own `kind: language` edge (the
runtime pair) — and exports the pins in `TEBAKO_SPAWN_LOCK`. The row
grammar gains the payload row, `;`-joined in manifest order with the
runtime rows, one row per edge:

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
the runtime rows.

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

Run time reads ONE block: the loader resolves each row into the store
— the provider image into `payloads/`, the nested runtime pair into
`runtimes/` — carried → slot extract + digest-verify + install; shared
→ cache hit on the locked identity + digest, else fetch from the row's
`source` + verify + install — and exports the §5 lock. A carried
spawned payload is never mounted BY THE PARENT: its slots ride the
lock's claimed-slot set, invisible to the parent's mount composition
(the CHILD mounts them fresh). Offline discipline is spec 05's,
unchanged.

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
  the enum seat lands in schema_minor 5 with BOTH sub-forms — `expose:`
  absent keeps §8's co-mount semantics (the inkscape case), `expose:`
  present is THIS spec's spawned payload (the xml2rfc case).
- The host-python xml2rfc convention (PROGRESS/25) is retired:
  metanorma's ietf edge becomes a hermetic spawned payload, identical
  on all four platform classes, offline-capable with a warm cache.
- The POSIX-only toolkit-mount + shell-script fallback is REJECTED
  (§0) — the child dispatch is the all-platform form by construction.
- The per-runtime spawn patches stay MECHANISM, not policy (spec 30
  §6's rule, unchanged): they implement §2's rewrite; the POLICY —
  what may be spawned, where it resolves, what jail the child gets —
  is declarative and lives in spec 30 + THIS spec.
