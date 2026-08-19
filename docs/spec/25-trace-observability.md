# Spec 25 — Trace observability: the interception bus, diagnosis, and coverage

**Status: PLANNED (owner-signed 2026-08-19 — architecture A: the in-tfs
event bus + `tebako trace` front-ends; retrace / procmon-ETW as outside
capture sources).**

A payload author virtualizing an application on tebako must answer three
questions, and today each is answered by hand-rolled one-off diagnostics
(the incident-13 msys dogfood chain: four CI rounds of `eprintln` gates
and in-probe bisection to name one Windows loader failure):

1. **Discovery** — what does my application touch, so what must I
   declare? (mounts, `needs:`, `materialize:`, entrypoints)
2. **Diagnosis** — my packaged application fails; which hop failed?
   (mount → manifest → resolve → materialize → OS bind)
3. **Coverage** — is anything my application does NOT virtualized?
   (raw syscalls, unpatched IATs, loader-internal probes — the escapes)

This spec defines one substrate and three front-ends:

- **The interception bus** (§2): every tebako interception point emits
  one structured event. The bus lives in the Rust `tfs` crate — the
  shipping VFS implementation on every platform — so it is
  cross-platform and runtime-independent by construction: events name
  syscall/loader-level facts (paths, verdicts, errnos), never
  language facts. Ruby, Julia, and Python payloads are equally covered
  because every runtime bottoms out at the same libc/loader boundary
  the bus instruments.
- **`tebako trace run`** (§4): discovery — collect a run's events and
  emit a suggested manifest.
- **`tebako trace explain`** (§5): diagnosis — replay a capture into
  the hop chain and name the first red hop.
- **`tebako trace cover`** (§6): coverage — correlate the inside stream
  with an outside syscall capture and report what escaped.

## 1. Laws

1. **Observability never gates.** A trace-channel failure degrades to a
   loud stderr note; the payload run proceeds and its exit code is
   unaffected. A tracer must never become a failure mode.
2. **Runtime-independent.** Events name OS-level facts only. No event
   may carry a language-runtime concept (a gem, a module, a Julia
   package) — runtimes are payloads; the bus sits below them.
3. **Cross-platform inside; pluggable outside.** The inside stream is
   identical on linux, macOS, and windows. Only the outside capture
   source (§6) is platform-specific, and it is an import format, never
   a runtime dependency.
4. **No shell-outs.** Captures are files parsed in-process. Arming or
   reading a trace never spawns a tool (spec 00 invariant 1).
5. **The correlation law.** The inside stream can never prove its own
   completeness: an escape is invisible from inside by definition
   (macOS 15's dyld rpath probes never reach the interposed `open` —
   the exec-closure walk exists because of this). "What isn't tebako
   intercepting" is answerable ONLY by subtracting the inside stream
   from an outside capture.
6. **MECE with spec 23 §8.** The `record` policy mode already owns
   host-path needs discovery (the audit journal → `tfs needs
   --from-journal`). This spec does not re-invent it: the jail channel
   of the bus IS that journal, formalized; §4's discovery extends the
   draft to the axes §8 does not cover (`materialize:`, entrypoints,
   runtime deps).
7. **Human gate.** A generated suggestion never edits a manifest by
   itself (spec 23 §8's law, unchanged).

## 2. The interception bus (`tfs::trace`)

**Emission points** — exactly the interception points the driver/TFS
already implements, one event per decision:

| op | emitted where | verdict values |
|----|---------------|----------------|
| `mount` | mount table insert/remove | `ok` / `error:<errno>` |
| `open` / `stat` | path dispatch (spec 11) | `image:<mount>` / `host` / `denied:<rule>` / `error:<errno>` |
| `dlopen` | `dlmap2file` / `dlalias2file` | `materialized:<host-path>` / `host` / `error:<errno>` |
| `exec` / `spawn` | exec routing (spec 17/22) | `routed:<entry>` / `host` / `error:<errno>` |
| `materialize` | exec-cache extraction | `ok:<host-path>` / `cache-hit` / `error:<errno>` |
| `jail` | policy bind / check (spec 08, 23 §8) | `allow:<rule>` / `deny:<rule>` / `record` |
| `resolve` | runtime/payload resolution (L3) | `cache` / `fetched` / `error` |

The `dlopen` event's `detail` carries the closure walk as structure —
the parsed format, the dep list, each dep's resolve verdict
(`materialized` / `host-system`) — what incident 13's round 4 added as
`eprintln` lines. Those gates convert to events; `TEBAKO_DEBUG_TFS`
remains as the human-readable degraded view over the same emission
points.

**Arming.** `TEBAKO_TRACE=<host-path>` (env) or `--tebako-trace
<host-path>` on the driver argument grammar (spec 17 — the env-scrubbed
handoff rides the argument, the lesson of incident 13 round 5). The flag
is an additive optional driver argument: spec 17's grammar extends by
option, never by version bump (the trailer-flags rule, spec 02). Both
set: the argument wins. The channel is opened and pre-registered at
driver boot, before any mount — writes are append-only, process-internal,
and NEVER policy-gated (the journal channel's discipline,
`crates/tfs/src/journal.rs`, already proven under `deny`).

**Cost when off.** Disarmed, an emission point is one
`Option::is_none()` branch. The bus compiles into the runtime/driver
side only; the bootstrap's < 3 MB size gate is untouched (spec 00
invariant 2).

**Children.** Spawned children inherit the armed channel by fd
inheritance where the platform allows (POSIX); on windows each child's
driver re-opens the same path for append (the record mode's
re-derivation, spec 23 §8). All writers append atomically; events may
interleave — every event carries `pid`/`tid` so the front-ends regroup.

## 3. The event envelope (schema v1)

One JSON object per line (JSONL; streamable, diffable, robust to a
crashed tail). Authored configs stay YAML (invariant 6); the event
STREAM is telemetry and is JSON. The versioned schema lives at
`docs/spec/schemas/trace-event.yaml` (lands with phase T1); the
normative shape:

```yaml
v: 1                    # envelope version — additive only, never bumped
ts: <rfc3339μs>         # driver clock
pid: <int> ; tid: <int>
op: <table §2>
path: <vfs-or-host path, exactly as dispatched>
verdict: <table §2>
detail: { ... }         # per-op: dlopen deps, jail rule, exec argv0, …
dur_us: <int>           # the interception's own cost
errno: <int, optional>  # when verdict names an error
```

CI validates every emitted event against the schema (a property test
drives the whole op matrix and validates the stream).

## 4. `tebako trace run` — discovery

Runs an application composition under `policy: record` with the bus
armed, then synthesizes a **suggested manifest** — a commented YAML
fragment, never applied (law 7):

- host reads → `needs:`/`jail:` ro grants (this half is spec 23 §8's
  generator, unchanged — the bus's jail channel feeds it);
- host writes → rw grants (strongest-observed-op wins, §8's rule);
- a VFS-resident file read through a RAW host fd (a native library's
  own `fopen` — the exec-cache answer consumed it) → a `materialize:`
  candidate — the sassc class-R case, caught automatically instead of
  discovered by a failing import;
- an in-image `dlopen` whose deps all resolve in-image → a NOTE
  "closure-covered — nothing to declare";
- a spawned host-absolute executable → an entrypoint/runtime-dep note;
- floor, store, and exec-cache paths are excluded (§8's exclusion law).

Output: the draft fragment on stdout (or `--out`), each entry carrying
its `why:` evidence line (`observed: <n> events, first at <ts>`). The
author reviews, flips ro↔rw, deletes noise, fills `why`.

## 5. `tebako trace explain` — diagnosis

Replays a capture (or a live run's stream) into the hop chain —
mount → manifest read → resolve → materialize → OS bind — and prints
the first hop whose verdict is red, with the signature table seeded
from the incident corpus:

| signature in the stream | named hop |
|---|---|
| no `mount` event for the env image + prelude-class child stderr | env image never mounted (handoff env lost) |
| `dlopen` verdict naming the OS loader's module-not-found (windows error 126; the POSIX `dlopen` ENOENT analogue) with every dep `materialized`\|`host-system` | the OS bind itself — bisect candidates printed |
| `jail` `deny` immediately preceding a dependent `open` error | policy denial (the EACCES class) |
| `materialize` `error` | exec-cache write failure |

The table lives in data (a YAML map in the CLI crate), extended as
incidents teach new signatures — never hard-coded into the bus.

## 6. `tebako trace cover` — coverage

The escapes report (law 5). Inputs: an inside capture (the bus) and an
outside capture of the same run:

- **POSIX**: a retrace capture (riboseinc/retrace, the org's
  LD_PRELOAD syscall tracer — consumed as a file; a JSONL output mode
  upstream is a contribution candidate, never a fork).
- **windows**: a procmon CSV/XML export (offline, no privileges at
  trace time beyond the capture itself); an ETW importer is a later
  phase.

Algorithm: canonicalize host paths both sides (same atoms as spec 23
§8's generator); subtract every outside-observed op that an inside
event already accounts for; the remainder is the escapes list, grouped
by surface class (fs / exec / dlopen / spawn), with a per-class
coverage percentage. Exit code 0 = no escapes, 1 = escapes found —
CI-gateable. Intentionally-unserved surfaces (documented passthroughs,
e.g. the platform floor) are declared in the composition so the report
separates KNOWN-UNSERVED from ESCAPE.

## 7. Testing and gates

- Golden fixtures: the incident-13 dogfood captures (real `[tfs]`
  streams) converted to the schema must round-trip through the
  validator and through `explain` (whose red hop must match the
  incident's hand-derived answer).
- Property: driving the public tfs op matrix emits exactly one
  schema-valid event per op.
- Correlator: synthetic inside/outside pairs with deliberate escapes
  (a raw-`syscall(2)` fixture binary — built by CI, not shipped) must
  be caught at 100%.
- The factory's msys dogfood converts from `TEBAKO_DEBUG_TFS` gates to
  the bus as phase T1's dogfood — the harness stops hand-rolling
  diagnostics.
- Size/hygiene: bootstrap gate untouched; no new dependencies outside
  the workspace (`tebako-json` for emission); `unsafe` nowhere (the bus
  is safe Rust over the journal's fd).

## 8. Phasing

| phase | contents | dogfood |
|-------|----------|---------|
| T1 | the bus + schema + `run` (discovery) | the msys dogfood rides the bus; a payload author onboards a fresh gem without hand diagnostics |
| T2 | `explain` (diagnosis) | replay the incident-13 captures; the named hop matches history |
| T3 | `cover` (certification) | a dogfood leg under retrace + procmon; escapes report in CI |

Each phase ships behind its own PR chain against this spec; the spec
moves PLANNED → PARTIAL → SHIPPED per phase (spec 00's status law).
