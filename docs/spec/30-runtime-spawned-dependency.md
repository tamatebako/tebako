# Spec 30 — Runtime as a spawned dependency

**Status: PLANNED (drafted 2026-08-30).** Amends spec 03 §2.3 (DEPENDS
gains the kind-runtime edge), spec 07 §1/§8 (the spawn surface and shim
registration), spec 23 §11 (the java question re-settled: java promotes
to a runtime). No wire-format change; no trailer change. Requires
spec 29: the depended runtime's exe is the wrapper — a host-resident
store binary the kernel can exec directly. First instance: metanorma's
java edge (mn2pdf/jing), retiring the openjdk toolkit-slice conventions
(PROGRESS/19).

## 0. The two kinds of "a runtime needs a runtime" (MECE, forever)

- **Spawned dependency (THIS spec):** the parent's process stays owned
  by its own runtime; the depended runtime runs as a CHILD process,
  dispatched through tebako. File/stdio/loopback interchange only
  (PROGRESS/27 §1's governing law — no in-process cross-runtime
  embedding, ever).
- **Runtime-on-runtime (NOT this spec):** the depended runtime OWNS the
  process and the depending runtime contributes an env image (jruby on
  java, truffleruby-jvm on graalvm). That is composition of the boot
  itself — the TODO.jruby/01 spec — and it reuses spec 29's wrapper for
  the OWNER runtime.

## 1. The dependency edge (spec 03 §2.3 amendment)

`requires:` gains the kind-runtime edge:

```yaml
requires:
  - kind: runtime
    engine: java                      # spec 28 §8's engine axis
    implementation: temurin           # OPTIONAL (spec 28 §8)
    constraint: ">= 21"               # the spec 05 §5 version classes
    expose: [java, keytool]           # OPTIONAL — §3's shim surface
```

Resolution treats the edge like any dependency — but the artifact
resolves through the RUNTIME index and lands in the store's `runtimes/`
area (share-once, spec 05 §3). The depended runtime is **NOT co-mounted**
into the parent's VFS: its wrapper exe executes FROM THE STORE (a host
path, 0755 — kernel-visible by construction; the exe itself is never
materialized) and mounts its own env image per spec 29. The edge is
version-locked at press per spec 23 §4 (locked digest at dispatch),
exactly like any DEPENDS.

## 2. The spawn surface (normative)

Two spellings, one semantics; both live:

- **In-band (the program spawns a command).** The parent runtime's
  spawn interception (spec 22's hooks; the argv bridge on windows)
  recognizes the spawned command as a DEPENDS-exposed entry (the §1
  declaration — an undeclared command name is NEVER rewritten; it falls
  to the host PATH, journaled as a host spawn). The rewrite composes a
  full spec-17 dispatch of the depended runtime's wrapper exe from the
  store:

  ```
  <store>/runtimes/…/tebako-runtime-…-java-…[.exe]
      --tebako-image <shared mounts…> --tebako-entry <entry> <args…>
  ```

  **Argument carry-over (locked):** an argument that lexically resolves
  under one of the parent's mounts is carried by RE-MOUNTING that image
  in the child AT THE SAME mount point (the bare store file or the
  package slot, serialized per spec 17 §2.1's emit rule) — argument
  strings pass through UNREWRITTEN. On the exec-cache visibility class
  (windows), where the child cannot serve in-VFS reads, the argument's
  file is materialized to the exec cache and the argument rewritten to
  the host path — the shipped ruby-windows behavior (PROGRESS/19's
  option (b)) is this rule's first instance.
- **Operator-mediated:** `tebako run <name>:<entry> -- <args…>` — the
  generic verb, pinned for runtime entries: `<name>` is the runtime
  payload's name (e.g. `openjdk`), `<entry>` one of its declared
  entrypoints. Same dispatch, same jail, journaled the same.

Both spellings dispatch through tebako — NEVER a raw VFS path, NEVER a
host PATH fallback (§5).

## 3. Shim registration (compose, don't invent)

A depended runtime's entrypoints surface on PATH ONLY through the
depending payload's declared command exposure: the §1 `expose:` list
names the depended entries the payload surfaces; install registers one
shim per exposed name (spec 07 §1's argv0 model; the per-entrypoint
active flag applies — an exposed-but-inactive command links on
`tebako shim enable`). Dispatch of such a shim IS the §2 in-band
dispatch (argv0 → the payload's version chain → the depended runtime's
resolution). A name collision between a payload's own entries and its
exposed depended entries is a named manifest error at press; a
collision between two installed payloads stays spec 07's existing
first-wins/disabled-state model.

## 4. Jail interaction (union-of-needs, never inheritance)

The child dispatch computes its jail from ITS OWN declared needs — the
depended runtime's manifest plus the payload carrying the spawned
command, per spec 23 §5/§6 — and never inherits the parent's grants
verbatim. The spawn bridge passes NO jail env to the child;
`TEBAKO_JAIL_SOURCE` journals the child's own composition. Default deny
stands: a need nobody declared is a denial with the needs-check record
(spec 23 §8's record mode covers the discovery loop for spawned edges
identically).

## 5. Failure modes (the host-fragility killer)

- Depended runtime absent or incompatible at spawn: the resolver's
  named error (the spec 06 exit 69 shape), BEFORE the child exists —
  the parent sees a spawn failure carrying the named error's text; with
  `TEBAKO_OFFLINE=1`, cache-or-named-error (spec 05).
- An `expose:`d entry whose depended runtime resolves but whose entry
  is undeclared in the runtime's manifest: a named validation error at
  press/install (`tebako-pkg validate`; the tebako#494 cross-check
  class).
- NEVER a silent host fallback: a depended-runtime spawn that cannot
  resolve through tebako fails named; falling through to a system
  java/python is the bug class this spec exists to kill (the chocolatey
  python313 saga, PROGRESS/25, is the standing example).
- The depended runtime's `contract_version` is negotiated fail-closed
  by the dispatcher exactly as for a primary runtime (spec 06 §6;
  exit 75).

## 6. What this settles and retires

- spec 23 §11 is RE-settled: java is no longer only a payload slice —
  the openjdk runtime promotion (TODO.java/04) makes it a
  `kind: runtime` payload consumed through THIS spec's edge. The
  toolkit-slice form stays legal for library-layer cases (spec 03's
  `kind: toolkit`); the spawn-a-JVM case routes here.
- The per-runtime spawn patches stay MECHANISM, not policy: they
  implement §2's rewrite. The POLICY — what may be spawned, where it
  resolves, what jail the child gets — is declarative and lives here.
