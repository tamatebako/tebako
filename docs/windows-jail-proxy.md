# Windows jail proxy paths — the declarative host-surface contract

Status: the enforcement layer and the windows platform floor are
SHIPPED (`crates/tfs` `HostPolicy`, spec 08). The declaration surface
they compose with is spec 23: its record mode and image-layer compose
are IMPLEMENTED (`TEBAKO_JAIL=record`, `tfs needs --from-journal`,
`tfs exec --compose`); the D1–D5 manifest/shim/press wiring is
Phase-R (PLANNED, owner-signed 2026-08-14). The windows Class-L load
path this document's §1/§4 rows cite is design-pinned in spec 22 §2.1
(phase W; implementation W2 — PLANNED). This document is the
windows reading of those two specs — it is not a third authority;
every rule cites its owner.

A tebako process never touches the host filesystem silently. Every
host path a payload can read or write is PROXIED through the TFS
host-passthrough under one bound policy (spec 08 §3's single choke):
declared by the slice, the composition, or the operator; resolved into
one effective policy before the interpreter runs; enforced identically
by the bootstrap, the runtime driver, the shim, and `tebako run`.
"Proxied" means exactly spec 08's grant semantics — the host path
stays at its own spelling (or at a declared VFS mount point), reads
answer the real file, writes against a read-only grant answer EROFS,
and everything undeclared answers EPERM under the deny default:
journaled, named, never a silent fallback.

## 1. The windows platform floor (proxied read-only, automatic)

Under the `deny` default every bound policy gains the **platform
floor** (spec 08 §2.1): the surface a spawned interpreter or runtime
physically cannot boot without, granted so a missing grant surfaces as
the workload's own named error — never a segfault in someone else's
library (the macOS JVM `getMacOSXLocale` SIGSEGV, spec 22 §3.4's
journal-pinned chain). On windows the floor is (spec 08 §2.1;
implementation: `crates/tfs/src/policy.rs` `platform_floor()` +
`HostPolicy::bind`):

| Host path | Why it is floor |
|---|---|
| `%SystemRoot%\System32` | the loader's DLL root — no process resolves its imports without it |
| `%SystemRoot%\SysWOW64` | the 32-bit view for 32-bit children |
| `%SystemRoot%\Fonts` | the GDI font tree GUI runtimes enumerate (the JVM's AWT init reads it) |

Semantics (all spec 08 §2.1, all in `HostPolicy::bind`):

- `%SystemRoot%` resolves AT BIND (`C:\Windows` when unset). A floor
  path absent on the host is skipped silently — it is a courtesy
  surface, not an authored request whose absence must fail the bind.
- The floor binds ONLY under the `deny` default. Under `open` it
  grants nothing the default does not already allow (`never_denies`
  keeps its exact meaning); under `record` everything is allowed
  anyway.
- Read-only, always: writes to a floor path answer EROFS. An authored
  grant covering a floor path (same or ancestor prefix) SUPERSEDES the
  floor entry — the floor never narrows what an author allowed, so
  widening a floor path to `rw` is one authored grant away; there is
  no way to drop a floor path short of not denying — that is the point
  of a floor.
- Ancestor traverse: every bound grant (floor included) derives its
  strict ancestors as exact-path reads — never prefix (no sideways
  exposure), never write — so the platform's canonicalization walks
  pass by construction.
- The floor is NEVER serialized into `TEBAKO_JAIL`: the env grammar's
  `host:mount:ro|rw` right-split cannot carry a drive-qualified
  windows spelling. Every consumer RE-DERIVES the floor at its own
  bind (supersede makes that idempotent), so a spawned child
  re-binding its inherited spec enforces exactly its parent's policy.
- The floor mount's VFS-side spelling on windows is the
  drive-qualified forward-slash form (`C:/Windows/System32`,
  `floor_mount_point()`); it is informational — enforcement matches
  host prefixes.
- The floor is also what the LIBRARY load path lives on (spec 22
  §2.1): the patched dln/fiddle route's host passthrough is a gated
  host read like any other (`dlmap2file` is in spec 08 §3's gated
  set), so a bare name meaning a System32 library (`ffi_lib 'user32'`)
  passes under `deny` exactly because the floor grants it — while a
  bare name resolving outside floor+grants is EPERM, journaled. The
  OS loader's own reads (process-boot import resolution, raw
  `LoadLibrary` from an unpatched caller) never pass a tebako route
  and are not gated — §5's honest scope.

What never joins the floor (spec 08 §2.1's boundary): the workload's
own tool tree (a JRE, a third-party install) and the user's home stay
AUTHORED grants — an operator's `deny` must not silently read-expose
private data, and the prefix grammar cannot express the "stat-only"
grant the macOS CFPreferences home probe needs. A floor entry joins
the list only with a proven platform-process consumer, cited by run —
evidence, never anticipation.

## 2. How a payload declares additional host paths (spec 23 D1)

A slice declares its host surface in its in-image manifest's `needs:`
block (spec 23 §2 — the ONLY spelling; the pre-spec-23
`capabilities.host` key is a named validation error naming the rename,
never an alias, never a dual-key merge):

```yaml
needs:
  host:
    - path: 'C:\Vendor\Tool'      # absolute, or a symbolic atom below
      access: ro                  # ro | rw
      mount: /vendor/tool         # OPTIONAL: present the host path at
                                  # this VFS point (bind-mount spelling);
                                  # absent = enforcement-only passthrough
                                  # at the host spelling
      when: [windows]             # OPTIONAL platform filter
      optional: true              # OPTIONAL: absent at bind = silently
                                  # skipped; absent WITHOUT it = a named
                                  # error (fail-closed)
      why: "the tool probes its install root at boot"   # MANDATORY
```

Symbolic atoms resolve at bind, per invocation, per user — never
baked: `%USERPROFILE%` / `$HOME`, `%TEMP%` / `$TMPDIR`, `$CWD` (the
invoking cwd), `$TEBAKO_HOME` (the store). An atom that does not
resolve fails the bind only when the need is otherwise in force; under
a platform-filter mismatch the entry is inert.

Declaration rules (spec 23 §2, fail-closed): the same canonical path
declared twice by one slice with disagreeing accesses is a named
manifest error; an ancestor's access must be at least the
descendant's (an ro ancestor may not hide an rw need); `access: rw`
on a symbolic atom outside `$TEBAKO_HOME` requires the composition's
explicit consent; a data slice carrying a `needs:` block is a named
manifest error.

Discovery (spec 23 §8): when the author does not KNOW the surface —
the usual case — run the workload under `policy: record`
(`TEBAKO_JAIL=record`): every host access is allowed and journaled
(`event=jail-allow path=<p> op=read|write`), and
`tfs needs --from-journal <journal.log>` emits a draft `needs:` block
with floor, store, and exec-cache paths already excluded (they are
automatic — never declared) and per-user paths re-substituted to
atoms. The draft is the OBSERVED MINIMUM; the author reviews it and
merges it into the slice manifest (D1) or the composition (D2). The
generator never edits a manifest itself.

## 3. How the running configuration binds them

The resolver (spec 23 §6 — the shim in managed mode, the bootstrap in
standalone, `tebako run` / `tebako press` up front) computes ONE
effective policy before exec, in order: the slice set → the needs
union (every slice's D1 `needs.host`, platform-filtered, atoms
resolved at bind) → composed with the running configuration (D2
`mounts:`/`needs:`, D4 operator config, D5 CLI flags) and the operator
tightening (declarations request; the operator tightens — spec 08
§4's locked precedence) → the needs-check → export.

- **Managed dispatch (shim):** resolved per invocation — the slice
  mirrors ∪ the runtime's release-manifest needs ∪ the composition
  document (`tebako.yaml`, D2) ∪ operator config (D4) ∪ CLI (D5).
  Swapping the configuration IS editing `tebako.yaml` or passing
  flags; nothing is baked, nothing recompiles (spec 23 §9).
- **Standalone package (bootstrap):** the press-baked D3 needs union ∪
  a lean runtime's manifest needs ∪ the operator env. An external
  composition document overrides the baked block (first hit wins:
  `--compose` on argv, `TEBAKO_COMPOSE`, the `<package>.tebako.yaml`
  sidecar — spec 23 §9; the override channels are Phase-R, the
  image-layer form is `tfs exec --compose`, implemented).
- **The shipped CLI surface today** (spec 08 §4): `tebako run` and
  `tebako-shim` take `--jail <spec>` (`open` | `deny` | `deny:arg` |
  a YAML file | the env grammar), repeatable `--mount
  <host:mount:ro|rw>`, and `--no-host`; spec 23 §3's wider D5 flag set
  (`--policy`, `--need`, `--slice`, …) is Phase-R.
- **Export and bind:** the authored part serializes to `TEBAKO_JAIL`
  (+`TEBAKO_JAIL_SOURCE`, +`TEBAKO_JAIL_JOURNAL`) — the env grammar
  `open` | `deny` | `deny;host:mount:ro|rw;@argfile…` (`crates/tpkg`
  `jail.rs` owns the authored model). The floor and the system
  self-surface are NOT serialized — every bind re-derives them. The
  driver parses `TEBAKO_JAIL` (`tfs::policy::JailSpec`) and installs
  the policy via `HostPolicy::bind` AFTER the mounts (spec 08 §3 — the
  mount family's image read is itself policy-gated once a policy is
  active). A spawned child inherits `TEBAKO_JAIL` and re-binds: same
  policy, floor included. The boot-time materialization (spec 22 §4
  class R + §2.1's library-alias extraction) runs after the policy
  installs, and the load-time dll map (the POSIX dlmap twin) writes
  mid-run: both are the system self-surface — process-internal,
  never policy-gated — while payload IO sees the exec cache
  read-only (spec 23 §5 item 3).

The default matrix (spec 23 §5): a run with ANY declaration in force
defaults to `policy: deny`; `open` is asked for by name; a run with no
declarations anywhere runs open (`never_denies`) — a first-class rule,
not a compatibility mode. The effective world under deny is exactly:
the VFS (read-only by construction, spec 11) + the platform floor (§1)
+ the system self-surface (`$TEBAKO_HOME`, `TEBAKO_EXEC_CACHE`) + the
declared needs ∪ composition/operator grants + auto-allowed argument
files. Everything else is EPERM/EROFS, journaled.

## 4. The named errors (never a silent anything)

| Boundary | Verdict |
|---|---|
| host read outside every grant (deny default) | EPERM from the IO route, journaled `event=jail-deny path=<p> op=read source=<s>`; the line names the covering declaration when one exists (`would-need=slice:<name>`) and says so when none does — the operator learns the exact key to add from the denial itself |
| write against an ro grant (floor or authored) | EROFS, journaled the same way |
| malformed `TEBAKO_JAIL`, or a bind failure (a relative mount point → EINVAL; a missing authored mount source or argument file → its canonicalization errno, ENOENT included) | exit 73 (`EX_TEBAKO_JAIL`), fail-closed, at the bootstrap/driver surface that binds it |
| a declared need the effective policy does not cover | named resolution failure BEFORE exec: `slice <name> needs <path> (<access>, why: <why>) — denied by <source>` (spec 23 §6 step 4; the exit code is pinned at implementation per the spec 05/07 table — never a fallback) |
| `capabilities.host` in a manifest | named validation error naming the rename (`→ needs.host`) |
| `record` carrying grants | EINVAL at bind — inert configuration is a named error, never silently ignored |
| a composition override naming a slice the trailer does not carry (fat package) | named error, never a silent skip (spec 23 §9) |
| a bare-name library load with no declared alias, the name outside floor+grants (deny) | EPERM from the gated `dlmap2file` route on the covered surface (patched dln/fiddle), journaled like any host read; a RAW `LoadLibrary` caller (unpatched ffi, a self-loading C extension) never passes a tebako route — the OS answers its own error, ungated and unjournaled (§5) |
| an alias-declared bare-name or VFS-path library load (spec 22 §2.1) | materialized from/to the exec cache — the system self-surface, never policy-gated; a tampered cached copy is a named 70 (the Rule-R3 verdict), and a failed materialization surfaces the tebako verdict line in the raised `LoadError`, never a bare OS error |

## 5. Honest scope

Jails are FILESYSTEM-ONLY (spec 08 §5). No network confinement is
claimed (`capabilities.network` is advisory). Coverage is every IO
route through the TFS layer — interpreted payloads and native binaries
under the preload shim alike; NOT covered: statically-linked binaries
(direct syscalls, no interposition point) and any process that escapes
the shim — stated honestly, never implied otherwise. The platform
loader's own reads never pass a tebako route on ANY platform:
process-boot import resolution and a raw `LoadLibrary`/`dlopen` from an
unpatched caller (spec 22 §2.1's documented edge) are the OS's own
file IO — not jail-gated, not journaled (the macOS dyld rpath probes
are the same class, proven macOS 15). The floor and the authored
grants govern the routes tebako actually serves; §1's floor is what
keeps the GATED host-library passthrough working under `deny`.
