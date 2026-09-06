# Spec 33 — Runtime-on-runtime composition

**Status: PLANNED (drafted 2026-09-07 — ecosystem TODO.jruby/01).**
Amends spec 03 §2.3 (the `kind: runtime` edge's discriminator), spec 17
§1 (mount order and entry resolution gain the depending-runtime clause),
spec 23 §6 (the needs union covers both runtimes), spec 28 §8 (the
mode bullet's pointer lands here), spec 29 §7 (the non-goal is defined
here), spec 30 §0 (the second kind's home). No wire-format change; no
trailer change; no registry change. The L1 manifest change is ADDITIVE
(the `on_runtime` block — payload-manifest schema_minor 6). First
instances: truffleruby-jvm on graalvm (TODO.truffleruby/03), jruby on
java (TODO.jruby/02).

## 0. The problem, and the MECE table that settles it

A jruby "runtime" is not a process owner: the process belongs to the JAVA
runtime; jruby contributes an env image (the jruby home) and an entry
rewrite (`org.jruby.Main`). Spec 30 §0 names the two kinds of "a runtime
needs a runtime" and owns the first (spawned child). This spec owns the
second: **composition of the boot itself** — and it invents no third
artifact class to do it. The spawn/composition classes, permanently MECE:

| class | declarer | process owner | spec |
|---|---|---|---|
| spawned runtime edge | a non-runtime payload's `kind: runtime` edge with `expose:` | the payload's own runtime | 30 |
| spawned payload edge | a `kind: executable` edge's `expose:` form | the provider payload's runtime | 32 |
| runtime-on-runtime | a `kind: runtime` payload's `kind: runtime` edge WITHOUT `expose:` | the DEPENDED runtime | THIS |

The discriminator is the declarer's kind plus the edge's `expose:` key —
never a new edge kind, never a new payload class. The depending runtime
is an ordinary `kind: runtime` payload in every other respect: same store
area, same registry entry shape, same trust anchors, same spec-05
resolution chain.

## 1. The composition rule

- A `kind: runtime` payload MAY declare at most ONE **owner edge**: a
  `requires:` entry `{kind: runtime, engine, implementation?,
  constraint}` with NO `expose:`. A second owner edge on one payload is
  a named manifest error; an owner edge on a non-runtime payload is not
  a concept (its `expose:`-less `kind: runtime` edge is spec 30's,
  exposed or not — see §0's table; for a non-runtime declarer an
  `expose:`-less edge simply surfaces no shim names). A runtime payload
  MAY ADDITIONALLY carry spec-30 spawned edges (`expose:` present) —
  unchanged.
- **Resolution is recursive.** The owner edge resolves through the
  spec 05 §2/§5 chain exactly like a primary runtime (per-engine
  download base, cache-first, share-once into `runtimes/`), and
  version-locks at press per spec 23 §4 like any edge.
- **At dispatch the OWNER provides the process.** Its exe (the spec-29
  wrapper, or a linked exe) receives the spec-17 wire; the depending
  runtime contributes its env image. The loader composes the handoff:
  `TEBAKO_RUNTIME_IMAGE` = the OWNER's env image; the depending
  runtime's env image rides as the FIRST `--tebako-image` triple at its
  declared mount (§2); payload images follow in triple order. **The wire
  grammar is unchanged** — the composition is loader-side; the driver
  discovers the depending env image's role from its mounted manifest
  (the `on_runtime` block), never from a new wire token.
- **Mount order** (spec 17 §1 amended): the owner's env image first,
  then the depending runtime's env image, then payload triples in argv
  order. Longest-prefix dispatch, nested mounts legal, unmount-all on
  any failure; the EEXIST law extends — the depending runtime's declared
  mount landing on an occupied point is a named boot error, never a
  partial mount.
- **Entry resolution** (spec 17 §1 amended): `--tebako-entry` resolves
  against the first `--tebako-image` mount whose manifest is NOT the
  depending runtime's env image — ordinarily the app payload. With no
  payload triples the entry resolves against the depending runtime's own
  mount (the runtime's self-boot smoke form: `jruby -v`). An
  `on_runtime` boot with NEITHER payload triples nor a resolvable entry
  is the spec-17 named error 65, unchanged.

## 2. The `on_runtime` block (the L1 grammar — schema_minor 6)

The depending runtime's in-image manifest declares, under its
`RuntimeProvides` block (spec 03 §2.2), additive:

```yaml
provides:
  provides: {engine: ruby, implementation: jruby, version: "9.4.8.0",
             language_version: "3.1", abi_line: "9.4", platform: aarch64-macos}
  entrypoints: […]
  on_runtime:
    mount: /__runners__/jruby     # POSIX-absolute (spec 17 §1's uniform
                                  # namespace); where MY env image mounts
    argv_template:                # the argv between the owner's interpreter
      - "-classpath"              # token and the entry (§3)
      - "{mount}/lib/jruby.jar"
      - "org.jruby.Main"
    owner_contract: ">= 2"        # OPTIONAL — the constraint the owner's
                                  # contract_version must satisfy (§4);
                                  # default ">= 2"
```

- The block is REQUIRED when an owner edge exists and FORBIDDEN
  otherwise — both directions are named manifest errors (the composition
  declares its rewrite or it is not a composition).
- `mount` is declared by the DEPENDING runtime — its own home knows
  where its template paths resolve (spec 03 §2.3's MOUNT RULE applied to
  this class: the consumer of the mount declares it). `mount: /` is
  invalid at validation; a mount equal to the owner's root fails at boot
  as §4's EEXIST, by construction.
- `argv_template` is a list of strings. The single placeholder
  `{mount}` expands to this boot's effective mount point (drive-
  qualified on windows per spec 17 §1's qualification rule). An unknown
  `{…}` placeholder, or a template element that after expansion escapes
  the depending runtime's mount, is a named boot error (65).
- The template is DATA, not code: the driver and the wrapper crate carry
  no per-runtime knowledge (spec 29 §7's law extends — no per-runtime
  argv hardcoding either). jruby's classpath, truffleruby's launcher
  class, and every future instance are manifest rows.

## 3. The entry rewrite (spec 29 §1's composition, extended)

On an `on_runtime` boot the wrapper composes:

```
[<owner interpreter (the owner env's layout.interpreter)>,
 <argv_template expanded…>, <args_default…>, <entry resolved>, <user args…>]
```

The template composes FIRST (it is interpreter-directed: the classpath,
the launcher class); then the entry's own `args_default` (entry-directed,
spec 03 §2.2 — its single owner stays the runtime side, tebako#503; here
the DEPENDING runtime's manifest); then the resolved entry; then the
user's args, verbatim. Worked: a jruby entry `/app/bin/tool` maps to
`java -classpath <jruby.jar> org.jruby.Main /app/bin/tool <args…>` with
no per-runtime driver code.

The bare-name entry form (spec 17 §1's keyword rule) resolves against
the DEPENDING runtime's `provides.entrypoints` on an `on_runtime` boot
(the user-facing commands are the depending runtime's). There is NO
direct-program form on an `on_runtime` boot — spec 29 §1's bare-name
composition (exec the declared program with the interpreter token
dropped) belongs to owner-pattern boots; here every entry composes
through the template, because the depending runtime's "programs" are
inputs to the owner's interpreter, not executables. The owner's own
entrypoints (the JVM's `keytool`, …) are not surfaced through this boot;
they stay spawnable through spec 30 edges, unchanged.

## 4. Failure semantics (fail-closed, never a fallback)

- The owner edge unresolvable at dispatch (cache miss with
  `TEBAKO_OFFLINE=1`, or no compatible version anywhere): the resolver's
  named error (the spec 06 exit-69 shape) BEFORE any exec — the
  depending runtime never starts looking for a host java.
- The owner's `contract_version` not satisfying `owner_contract`
  (default `>= 2`): fail-closed at dispatch, the exit-75 negotiation
  class, naming both contract versions.
- `on_runtime.mount` colliding at boot: EEXIST, unmount everything,
  named error (spec 17 §1's law) — never a partial mount.
- A malformed `on_runtime` block (unknown placeholder, escaping
  template path, `mount: /`): named boot error 65 naming the key.
  Shape violations — the block without an owner edge, an owner edge
  without the block, two owner edges — are named MANIFEST errors at
  press/`tebako-pkg validate` (the tebako#494 cross-check class).
- NEVER a fallback: a depending runtime whose owner is missing does not
  run standalone, does not run on an implementation the edge excludes
  (a temurin-only store facing `implementation: graalvm` is spec 28
  §8's named error), and never drops the template to "just boot
  something".

## 5. Jail, lock, store (the composed run's bookkeeping)

- **Jail:** the needs union (spec 23 §6 step 2) gains the DEPENDING
  runtime's release-manifest needs beside the owner's — one union, one
  effective policy, computed before exec; the owner and the depending
  runtime run in ONE process under ONE policy.
- **Spawn lock:** untouched. The owner resolution is a primary-class
  resolution, not a spawned edge; `TEBAKO_SPAWN_LOCK` keeps spec 30 §2's
  semantics for the payload's OWN spawned edges, and spec 30 §2's
  child-env strip does not apply to this handoff (there is no child —
  the owner IS the booted process).
- **Press/lock:** the owner edge joins `lock.spawned[]` (spec 23 §13.6)
  with `expose:` ABSENT — the row's engine/implementation/constraint
  mirror the L1 edge, and the carried shape is the same two-slot pair
  (the owner's exe + env image as slots) beside the depending runtime's
  own pair; `tebako-pkg validate` cross-checks the row against the
  depending runtime's L1 edge exactly as for a payload's spawned edge.
  At run time the loader resolves the row into the store per §13.6
  (carried → slot install + digest-verify; shared → cache-hit on the
  locked identity else fetch+verify), then composes §1's handoff.
- **Store:** UNCHANGED layout. The owner and the depending runtime are
  ordinary `runtimes/` entries; share-once holds in both directions (N
  depending runtimes share one owner; M payloads share the depending
  runtime).

## 6. Selection (spec 28's surface — nothing re-authored here)

The depending runtime is selected as a payload's runtime by the spec 28
grammar and nothing else: `ruby:jruby@9.4`, `ruby:truffleruby`, the
`any_of` requirement list — with the ABI line the depending runtime's
OWN (spec 28 §8's law: implementation is a sub-axis of the requirement;
the owner edge's engine is orthogonal to the payload-facing selector).
truffleruby's native/jvm modes stay NON-axes (spec 28 §8, pinned): the
jvm-mode artifact is exactly the one carrying an owner edge to
`java:graalvm >= 24`; selection between modes rides `;tebako=` or
per-entry pins, and the edge's implementation filter makes a temurin-only
store the named error of §4, never a slow silent run.

## 7. Worked example — truffleruby-jvm on graalvm

The depending runtime's manifest (excerpt):

```yaml
requires:
  - {kind: runtime, engine: java, implementation: graalvm, constraint: ">= 24"}
provides:
  provides: {engine: ruby, implementation: truffleruby, version: "34.0.1",
             language_version: "3.4", abi_line: "34", platform: aarch64-macos}
  on_runtime:
    mount: /__runners__/truffleruby
    argv_template: ["-classpath", "{mount}/lib/truffleruby.jar", "org.truffleruby.Main"]
```

Dispatch of a payload whose `runtime_requirement` is
`{engine: ruby, implementation: truffleruby, constraint: "~> 34"}`:

1. The shim resolves the truffleruby-jvm runtime (spec 05 §5), reads its
   L1 mirror, finds the owner edge.
2. It resolves `java:graalvm >= 24` — cache hit or download; a
   temurin-only store is the spec 28 §8 implementation named error.
3. It checks the owner's `contract_version` against `owner_contract`
   (`>= 2`) — fail-closed, exit 75.
4. Handoff: exec graalvm's wrapper with
   `TEBAKO_RUNTIME_IMAGE=<graalvm env.tfs>`,
   `--tebako-image <truffleruby env.tfs>:-:/__runners__/truffleruby`,
   `--tebako-image <app payload>:-:/`, `--tebako-entry <entry>`.
5. The wrapper mounts the graalvm env first, then truffleruby's env,
   then the payload; discovers `on_runtime` in the first triple's
   mounted manifest; composes
   `[java, -classpath, /__runners__/truffleruby/lib/truffleruby.jar,
   org.truffleruby.Main, <entry>, <args…>]`; execs java.

One process, one policy, two env images, zero per-runtime code.

## 8. Non-goals

- No new wire token, no trailer change, no registry grammar change, no
  third artifact class.
- No multi-owner composition: one boot, at most one owner edge. A
  runtime needing two process owners is two payloads.
- No owner edges on non-runtime payloads — that surface is spec 30/32's,
  unchanged.
- The PROGRESS/27 §1 law stands and is not in tension: spec 30's
  file/stdio/loopback rule governs INTERCHANGE BETWEEN two runtimes;
  here the owner runs the depending runtime's language IN-PROCESS as its
  own interpreter (one JVM, one language-facing runtime) — no
  cross-runtime embedding is introduced.
