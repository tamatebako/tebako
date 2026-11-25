# Spec 08 — Jails: host-access policy

Normative specification of host-filesystem access control for running
payloads. Status: SHIPPED (roadmaps 09 + 30 + 35) — the TFS enforcement
point (§1–§3) is SHIPPED in `crates/tfs` (policy model,
`tebako_fs_host_policy`, per-route EPERM/EROFS gating, symlink
re-validation, jail acceptance suite); the `TEBAKO_JAIL` env form of the
policy (§1) is SHIPPED with roadmap 30's preload shim (`tfs exec --jail`
carries it to native binaries); manifest integration (§4 —
`capabilities.host` in `crates/tpkg`), the press/dispatch surfaces
(`tebako press --jail` writing the package manifest's `jail:` block,
`tebako run`/`tebako-shim` `--jail|--mount|--no-host`), the bootstrap's
manifest∩user composition at handoff, and the audit-journal logging of
violations (`event=jail-deny`) are SHIPPED with roadmap 35. Enforcement
exists because every file access already flows through the TFS layer —
one choke point covers every consumer. The RESOURCE axis (§2.2 — bounded
consumption: `cpu_seconds` / `memory` / `processes` / `wall_seconds`) is
SHIPPED for the runtime-driver delivery on both arms — POSIX (rlimits +
the forked wall watchdog; the 2026-11-22 amendment, closing the
2026-10-26 scar: a jail probe's child ran 96.9 % CPU for 1d21h
unnoticed — the jail confined access but nothing bounded consumption)
and windows (Job Objects + the watchdog thread; 2026-11-23,
tebako#1101). The preload-only (`tfs exec`) arm is PLANNED, named in
§2.2 and §7 — never silently unbounded. The §1 deny rule's device-node
amendment (2026-11-25, tebako#1128 — the fixed device set is exempt from
every deny stance) is SHIPPED with the verdict-layer change and the
conformance suite's `devnull`/`devnullspawn` vectors.

## 1. Policy model (docker `-v` semantics)

```yaml
jail:
  default: open | deny          # open = today's behavior (cwd + writes pass through)
  mounts:
    - host: $HOME/sources
      mount: /work
      access: ro | rw
  argument_files: auto-allowed  # the input file you hand the command is allowed even under deny
```

**Three profiles:**
1. **full access** (`open`) — host cwd + writes pass through (today).
2. **directory-scoped** — cwd mapped to `/work`, rw; nothing else.
3. **file-scoped tight jail** (`deny` + argument files only) — run an
   untrusted payload against one input, nothing else.

**The `TEBAKO_JAIL` env form (the form the runtime driver and the preload
shim consume — the loader's wire spelling of a policy).** ABNF:

```abnf
policy      = default *(";" grant)
default     = "open" / "deny"
grant       = host-grant / argfile-grant / net-grant / resource-grant
host-grant  = host-path ":" mount-path ":" access
host-path   = 1*(%x21-3A / %x3C-7E)        ; any visible char except ';'
mount-path  = 1*(%x21-3A / %x3C-7E)        ; the in-VFS point, same alphabet
access      = "ro" / "rw"
argfile-grant = "@" 1*(%x21-7E)            ; an argv file path, read-allowed
net-grant   = "net:" net-host [":" port]   ; an outbound peer grant (the
                                           ; network axis, §2)
net-host    = 1*(%x21-29 / %x2B-39 / %x3C-7E) ; a dns name or IP — no ':' or ';'
port        = 1*DIGIT
resource-grant = "resources:" assignment *("," assignment) ; §2.2 — the
                                           ; consumption bounds
assignment  = resource-key "=" resource-value
resource-key = "cpu_seconds" / "memory" / "processes" / "wall_seconds"
resource-value = 1*DIGIT [ memory-unit ]   ; decimal; zero is out of range
memory-unit = ("K" / "M" / "G") [ "B" ]    ; memory only, case-insensitive,
                                           ; 1024-based — a suffix on any
                                           ; other key is malformed
```

`open` grants host cwd + writes (today's behavior); `deny` is the
default-deny floor. Each `;host:mount:access` grants the host path at the
mount point with the named access; `;@path` auto-allows an argument file
even under deny. The grammar's alphabet excludes `;` (the grant
separator); a windows drive-qualified floor spelling is NOT carried (the
right-split cannot — every consumer re-derives the floor at its own
bind, §2's inheritance rule). Examples: `deny;/home/u/src:/work:rw`,
`deny;/home/u/src:/work:rw;@/home/u/in.csv`, `open`. A network grant is
its own form: `deny;net:example.com` (the host, any port),
`deny;net:example.com:443` (one port), and the two compose with host
grants in any order. The resource grant (§2.2) is one comma-separated
directive — `deny;resources:cpu_seconds=60,wall_seconds=300`,
`open;resources:memory=256M,processes=64` — order-free with every other
grant form, duplicate keys a named error. Like `net:`, a `resources:`
token is never `host:mount:ro|rw` (a RELATIVE host path literally
starting `resources:` is pathological and loses the token).

**The device-node exemption (2026-11-25 amendment, tebako#1128).** The
grammar governs host filesystem CONTENT; device files — `/dev/null`,
`/dev/zero`, `/dev/random`, `/dev/urandom` on POSIX; `NUL` on windows —
are process-essential kernel devices, not host content: no secret leaks
through them, and no payload widens its reach by opening them. The fixed
set is therefore always permitted read+write under EVERY policy — a deny
stance cannot deny it, and no grant narrows it (an ro bind over `/dev`
leaves `/dev/null` writable) — with no new grammar and no payload
opt-in: the exemption IS the jail semantics, applied at the verdict
(`HostPolicy::check`) ahead of every grant. The match is exact on the
canonical path: a lookalike (`/dev/nullify`), every other device node
(`/dev/sda`), and the device directory's enumeration all fall to the
policy's verdict — named-error discipline holds for anything the set
does not name. Like the floor (§2.1), the set amends only with a proven
platform-process consumer, never by anticipation. The evidence is the
metanorma tight-jail dogfood (run 45585078751, aarch64-macos):
`system(…, out: File::NULL)` — the gem ecosystem's adapter-probe idiom —
died `EPERM` on `/dev/null` under `deny;<cwd>:rw`, a jail that broke
programs without buying any confinement.

## 2. Semantics

- Writes to an `ro` mount fail `EROFS`; paths outside every mount fail
  `EPERM`. Payload images themselves are always ro (spec 11).
- Symlinks resolve inside the granted view: realpath checked at bind
  time, re-validated on each open.
- **Precedence (locked):** the package's manifest REQUESTS access
  (`capabilities.host`); the user can always TIGHTEN it
  (`tebako run --jail pkg`, `--mount src:/work:rw`, `--no-host`) — user
  policy always wins. Signed/trusted payloads may get wider latitude by
  policy choice, never by bypass.
- Violations are logged to the tebako audit journal with path + syscall
  class.
- **Network (the second axis — §1's grammar gains `net:` grants):**
  an outbound `connect` to an ungranted peer fails the policy's named
  refusal and journals the peer, exactly the fs axis's shape. Deny is the
  floor: with no `net:` grant, every outbound connection is refused. DNS
  resolution is allowed unconditionally — a query leaks a name, never
  payload bytes, and a grant names hosts whose addresses must resolve
  (the documented carve-out, in the manifest). User tightening intersects
  the manifest's network request exactly as on the fs axis: the tighter
  answer for every peer survives; the preload shim judges, never the
  payload.

## 2.1 The platform floor (locked 2026-08-14)

Every policy bound under the `deny` default gains the **platform
floor**: a fixed per-platform set of read-only grants covering the
surface a spawned interpreter or runtime physically cannot boot
without. The floor exists because the failure mode for a missing
platform grant was never a policy verdict — under a scratch-only jail
(`deny;<scratch>:<scratch>:rw`) the JVM crashed with a SIGSEGV at
`getMacOSXLocale` (its locale/framework init NULL-derefs when reads
under `/usr` are denied; spec 22 phase-E dogfood, 2026-08-13). A jail
whose denial crashes the child in someone else's library is wrongly
constructed: denying the platform surface never produced a working
process, so the floor grants it always.

- **Entries (evidence-driven):**
  - macOS — `/usr`, `/System`, `/Library` (the proven set above).
  - windows — `%SystemRoot%\System32` (the loader's DLL root; no
    process resolves its imports without it), `%SystemRoot%\SysWOW64`
    (the 32-bit view for 32-bit children), `%SystemRoot%\Fonts` (the
    GDI font tree GUI runtimes enumerate — the JVM's AWT init reads
    it). `%SystemRoot%` resolves at bind (`C:\Windows` when unset).
  - Other unix — none today; an entry joins a list only with a proven
    platform-process consumer, cited by run. The lists are amended by
    evidence, never by anticipation.
- **Supersede rule:** an authored mount covering a floor path (same or
  an ancestor prefix) supersedes the floor entry — the floor never
  narrows what the author allowed. Widening a floor path to `rw` is
  therefore one authored grant away; there is no way to drop a floor
  path short of not denying (see below) — that is the point of a floor.
- **Bind semantics:** the floor applies at `HostPolicy::bind` (§3's
  single choke), only under the `deny` default — under `open` it grants
  nothing the default does not already allow, so the no-policy path
  stays byte-identical (`never_denies` keeps its exact meaning). A
  floor path absent on the host is skipped silently (it is a courtesy
  surface, not an authored request whose absence must fail the bind).
- **Ancestor traverse (locked 2026-08-14; extended 2026-10-08):** every
  bound grant — authored mount, floor entry, or `@file` argument-file
  grant — implies its strict ancestors are
  traversable: each ancestor answers an exact-path READ — never a
  prefix (no sideways exposure), never a write. Canonicalization walks
  are universal: the JVM reads its cwd and every ancestor at VM init,
  and the factory's jailed_exec leg died with "Could not determine
  current working directory" when the chain was denied (PR #95 macOS
  legs). The `deny:arg` profile's file grants join the same rule
  (2026-10-08, tebako#539): a granted input file you cannot stat your
  way to is unusable — the JVM died at VM init under `deny:arg` with a
  granted file in a denied cwd chain. The set is derived at bind —
  never authored, never serialized.
  It is what makes an authored `$CWD`-style grant sufficient in
  practice: the payload names the directory it works in; the platform
  walk to reach it passes by construction.
- **Inheritance:** floor mounts are NOT serialized into `TEBAKO_JAIL` —
  the env grammar's `host:mount:ro|rw` right-split cannot carry a
  windows drive-qualified floor spelling, and it never needs to: every
  consumer re-derives the floor at its own bind (supersede makes that
  idempotent), so a spawned child's inherited spec plus its bind yields
  a policy identical to the parent's. Enforcement treats authored and
  floor mounts as one longest-prefix set.
- **The boundary (what never joins):** the floor is SYSTEM surface
  only. The workload's own tool tree (a JRE, a third-party install)
  and the user's home stay authored grants — an operator's `deny` must
  not silently read-expose private data, and the prefix grammar cannot
  express the "stat-only" grant the macOS CFPreferences home probe
  needs (spec 22 §3.4's journal-pinned chain, 2026-08-14: floor →
  named `jvm.cfg` error → named `InternalError` → boot with `<jre>:ro`
  + `<home>:ro`). The floor's promise is the end of the segfault
  class: every missing grant surfaces as the workload's own named
  error, pinned in the audit journal — never a crash in someone else's
  library.

## 2.2 The resource axis — bounded consumption (locked 2026-11-22; the windows arm, 2026-11-23)

The jail's first axis confines ACCESS (fs, network). The resource axis
bounds CONSUMPTION. The evidence for requiring it is ours: on
2026-10-26 a jail probe's child ran 96.9 % CPU for **1 day 21 hours**
before anyone noticed — the jail held, nothing bounded the runaway.
Hermetic is confined AND bounded; docker has cgroups, and the jail needs
the userland equivalent or "safer than a container" is false. Like the
jail, bounding is **opt-in**: a policy without `resources:` is
byte-identical to before (unbounded — today's behavior, always).

**The four bounds (MECE; the only spellable keys).** The YAML block form
(the payload manifest's `capabilities.host` and the package manifest's
`jail:` block, §4):

```yaml
resources:
  cpu_seconds: 60        # CPU time ceiling, seconds
  memory: 256M           # address-space ceiling, bytes — bare integer or
                         # a K/M/G (1024-based, case-insensitive, optional
                         # trailing B) suffix; both forms are bytes
  processes: 64          # process-count ceiling
  wall_seconds: 300      # wall-clock ceiling, seconds
```

The env form is one directive of comma-separated assignments
(`resources:cpu_seconds=60,memory=256M,processes=64,wall_seconds=300`),
order-free; any key may appear alone. Malformed input is a named error,
exit 73 (`EX_TEBAKO_JAIL`) — unknown or duplicated keys, empty or
non-numeric values, zero, `u64` overflow, a unit suffix on a non-memory
key — never a guess.

**Enforcement (the runtime driver — SHIPPED on POSIX and windows; the
preload-only arm PLANNED below).** The runtime driver installs the
resource arm in `apply_jail` — the same boot moment as the jail itself
(after the mounts, before the interpreter handoff), so the limits cover
the interpreter and every descendant it spawns (on POSIX the rlimits
inherit across fork/exec; on windows every child the interpreter spawns
is inside the parent's Job Object — either arm's preload-delivered
native child of a bounded tree is therefore kernel-bounded and never
re-applies). The mapping:

| key | POSIX mechanism | windows mechanism | the payload-visible answer at the bound |
|-----|-----------------|-------------------|------------------------------------------|
| `cpu_seconds` | `setrlimit(RLIMIT_CPU)` soft = hard | `JOB_OBJECT_LIMIT_PROCESS_TIME` (per-process user-mode) | POSIX: `SIGXCPU` death (128+24 from the parent); windows: the kernel terminates the process at the ceiling (no signal spelling — the kernel's chosen exit) |
| `memory` | `setrlimit(RLIMIT_AS)` — Linux (the macOS kernel refuses realistic values, below) | `JOB_OBJECT_LIMIT_PROCESS_MEMORY` (per-process commit) | address-space growth fails — the interpreter's own allocator error (ruby: `NoMemoryError`) |
| `processes` | `setrlimit(RLIMIT_NPROC)` | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` (per-JOB — the per-tree primitive POSIX lacks) | POSIX: `fork`/`spawn` fails `EAGAIN`; windows: children past the ceiling are terminated at creation |
| `wall_seconds` | the driver's watchdog (below) | the driver's watchdog thread (below) + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | the named stderr line + the tree's kill (POSIX: the process group's `SIGKILL`; windows: `TerminateJobObject` at exit 137 — one verdict on both arms) |

The windows mapping is deliberate where the primitives do not line up:
`cpu_seconds` rides the per-process user-mode clock
(`JOB_OBJECT_LIMIT_PROCESS_TIME`) because Windows offers no user+kernel
ceiling — the job-wide aggregate (`JOB_OBJECT_LIMIT_JOB_TIME`) would let
one hungry child spend the tree's whole budget, and CPU rate control
throttles where the axis promises death (a throttled runaway is still
running at the deadline). `memory` is per-process commit
(`JOB_OBJECT_LIMIT_PROCESS_MEMORY`), never the per-job aggregate
(`JOB_OBJECT_LIMIT_JOB_MEMORY`): a sibling's footprint must not kill an
innocent process — the POSIX arm's per-process shape. `processes` is the
one bound where windows is the stronger arm:
`JOB_OBJECT_LIMIT_ACTIVE_PROCESS` is per-JOB — genuinely per-tree, the
primitive POSIX lacks without cgroups — so a bound at or below the
tree's current size forbids exactly the new children, never the uid's
unrelated processes. The job's `KILL_ON_JOB_CLOSE` flag arms exactly
when the watchdog arms (`wall_seconds`, below): the job's sole handle is
the arming process's, so the root's exit closes it and the kernel reaps
the tree — the windows spelling of die-with-root.

A `setrlimit` the kernel refuses (e.g. a spawned-runtime child whose
composed union would exceed the tree's inherited hard limit) fails the
boot by name, exit 73 — a bound is never silently loosened; on windows
the same posture covers the Job Object calls (a create, limit-set, or
assign the kernel refuses — e.g. a spawned-runtime child nesting under
the ancestor's job).

**Platform reality (macOS).** The macOS kernel refuses
`setrlimit(RLIMIT_AS)` for any value below its address-space floor —
2^39 bytes on arm64, the user map's maximum extent (measured 2026-11-22:
2^38 → `EINVAL`, 2^39 accepted; `RLIMIT_DATA` and `RLIMIT_RSS` share the
refusal and enforce nothing) — and nothing at or above the floor is a
bound. A `memory` declaration that cannot hold therefore fails the boot
by name (exit 73 — never a silent clamp): the axis is enforceable on
Linux; on macOS the named refusal IS the answer (§7 row 30), and row
26's control arm declares its memory bound on Linux only.

**The honesty notes (what the bounds ARE):**
- `cpu_seconds` counts CPU, not wall: a sleeping payload never reaches
  it. Pair with `wall_seconds` for runaways that do not burn. On windows
  the clock charges user-mode time only
  (`JOB_OBJECT_LIMIT_PROCESS_TIME`); kernel-mode time is uncharged —
  Windows exposes no user+kernel ceiling.
- `memory` covers the WHOLE process — the runtime's own footprint is
  inside the budget (a runtime that needs 300 MB dies under
  `memory: 256M` at boot; budget the interpreter).
- `processes` is `RLIMIT_NPROC`, a **per-real-uid** ceiling, not a
  per-tree one (linux counts tasks, macOS processes) — POSIX userland
  has no per-tree process primitive without cgroups (the named
  non-goal, §5). Declare it against the uid's current count; a bound at
  or below that count forbids ALL forking. On windows the same key is
  per-JOB — genuinely per-tree (`JOB_OBJECT_LIMIT_ACTIVE_PROCESS`, the
  table above).
- `wall_seconds` is the tree's wall-clock budget (below). Clock-set
  jumps skew the cross-process inheritance note below by their jump —
  wall is not a precision instrument across `adjtime`.

**The wall watchdog (the `wall_seconds` mechanism, POSIX).** The
tree-root driver — the first driver boot in the process tree that sees
the bound — ensures process-group ownership (`setpgid(0,0)` when not
already a group leader; a group leader cannot re-group and does not need
to), then forks a watchdog child and exports
`TEBAKO_RESOURCE_DEADLINE=<unix seconds>` (a contract variable — it
survives the tebako#737 scrub and flows to every descendant). The
watchdog polls on a 50 ms slice and fires on either condition:

1. **Deadline** — the named line
   (`tebako-driver: resource bound exceeded (wall_seconds=<n>) — the
   payload process group was killed`) on stderr, an
   `event=jail-resource bound=wall_seconds op=kill cause=wall` line in
   the audit journal (best effort, raw `open(2)`/`write(2)` — the
   watchdog is a post-fork child and touches no shared state), then
   `SIGKILL` to the process group.
2. **Root exit** — the arming process is gone (the watchdog watches its
   parent's pid): the same kill with `cause=root-exit`. **A bounded
   run's tree dies with its root** — the orphan class of the 2026-10-26
   scar closed by construction, transitively: every armed watchdog
   watches its own armer, so a middle layer's death reaps its subtree.

**The wall watchdog, windows.** The same plan algebra and the same
`TEBAKO_RESOURCE_DEADLINE` contract variable; the mechanism is a thread
inside the arming process, not a fork. The tree-root driver creates the
Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, assigns itself,
exports the deadline, and spawns the watchdog thread, which sleeps to
the deadline and fires: the same named stderr line (`tebako-driver:
resource bound exceeded (wall_seconds=<n>) — the payload process tree
was killed (the job object)`), the same `event=jail-resource
bound=wall_seconds op=kill cause=wall` journal line, then
`TerminateJobObject(job, 137)` — 137 because it is the POSIX arm's
observed number (128+9): one verdict on both arms. The root-exit reap
needs no watcher and writes no journal line: the job's sole handle is
the arming process's, so the root's exit closes it and the kernel kills
the tree (`KILL_ON_JOB_CLOSE`) — die-with-root enforced below userland;
nothing survives the root to journal `cause=root-exit`, and nothing
needs to.

**Deadline inheritance.** A descendant driver boot (the spec 30
spawned-runtime child) reads the inherited deadline: a descendant whose
composed `wall_seconds` is TIGHTER than the inherited remaining budget
re-groups its own subtree (a `setpgid` on POSIX, a nested Job Object on
windows) and arms its own watchdog at
`min(own, inherited)`; a descendant without a tighter wall arms nothing
(the root's watchdog covers it). The tree's ceiling therefore holds
through every composition — no spawn chain outlives the root's declared
wall. A malformed `TEBAKO_RESOURCE_DEADLINE` is a named error (exit 73):
the inheritance channel never degrades silently. `wall_seconds` is the
only inherited bound; the rlimits are kernel-inherited by construction
and the per-process composition (`union`, §4) still applies — clamped
by the kernel's inherited hard limits (above).

**Named boundaries (never implied):**
- A payload that calls `setsid(2)` deliberately leaves the watched
  group. POSIX userland without cgroups cannot hold a determined
  escapee; the escape requires the payload's own syscall and is OUT OF
  SCOPE (§5), exactly the static-binary class on the fs axis.
- A wall-bounded payload that LEADS a job-control pipeline group cannot
  re-group (POSIX forbids a group leader `setpgid`), so its watchdog's
  kill covers the whole job group — wrap it (`( pkg )`) to isolate.
- When the arming process was not already a group leader (a CI
  supervisor's child), re-grouping moves the payload out of the
  supervisor's signal group: the supervisor's SIGTERM/Ctrl-C no longer
  reaches it and the declared wall remains the ceiling. This is the
  tree-kill mechanism's price; it engages only when `wall_seconds` is
  declared.
- **Windows: SHIPPED (2026-11-23, tebako#1101)** — Job Objects with
  `JOB_OBJECT_LIMIT_PROCESS_TIME` / `JOB_OBJECT_LIMIT_PROCESS_MEMORY` /
  `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` (+ `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
  when the watchdog arms) and the watchdog thread's
  `TerminateJobObject(…, 137)` wall verdict. The honesty deltas against
  the POSIX arm: `cpu_seconds` charges user-mode time only; `memory` is
  per-process commit, never the per-job aggregate; `processes` is
  per-JOB — genuinely per-tree, the primitive POSIX lacks; and the
  `setsid` escape class has no windows spelling — the arm grants no
  breakaway (`JOB_OBJECT_LIMIT_BREAKAWAY_OK` /
  `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` are never set), so a windows
  payload cannot leave its job. The grammar is stable across both arms.
- **The preload-only delivery (`tfs exec`): PLANNED** — the preload
  constructor is the named seam for the same rlimits; today the
  `tfs exec` surfaces REFUSE a `resources:`-carrying spec by name (never
  a silent drop), and a preload-delivered child of a driver-rooted
  bounded tree is kernel-inherited-bounded (it re-parses the spec, so
  the directive parses there by design).
- `record` composes with `resources:` (bounds are not grants — the
  record rejection covers mounts, net grants, and argument files only);
  the watchdog's kill lands in the same journal.

## 3. Enforcement point (the single choke)

New TFS C ABI entry:

```c
int tebako_fs_host_policy(int default_open /* 0 = deny */,
                          const tebako_host_mount_t* mounts, size_t n_mounts,
                          const char* const* arg_files, size_t n_arg_files);
```

- The host-passthrough path of every IO route consults the policy:
  prefix match + access bit per mount. `tebako_fs_stat/open/opendir/…`
  return `EPERM` (denied) / `EROFS` (ro write).
- The policy is about HOST paths; memfs mounts are unaffected.
- Implemented in the Rust TFS (`crates/tfs`) — the same layer that owns
  routing, so bootstrap, runtime driver, `tebako run`, and `tfs` all
  enforce identically with no per-app work.

### Shipped semantics (crates/tfs, roadmap 09)

- The policy is **process state, not namespace state**:
  `tebako_fs_unmount` does not reset it (fail-closed); a later
  `tebako_fs_host_policy` call replaces it. Install it after the payload
  mounts are established — the mount family's image read is itself
  policy-gated once a policy is active.
- Gated host-passthrough decisions: `open` (read and write modes),
  `stat`, `opendir`, `dlmap2file`, `extract_all`'s destination (a host
  write), and the mount family's image read. An allowed host path keeps
  the historic answer ENOENT ("not ours, pass through") for reads AND
  writes — under `open` a host write attempt therefore answers ENOENT,
  not EROFS, so the consumer's pass-through can perform the write
  (profile 1: "cwd + writes pass through"). Held memfs content is
  byte-for-byte unchanged (write opens on it stay EROFS).
- The per-process materialization tree is NOT a host path for policy
  purposes (tebako#502, 2026-08-30): a dlmap-marker path
  (`…/tebako-dl-<hex>/…` — closure copies and the home-layout
  whole-tree roots `tebako-home-<handle>/…`) the image does not answer
  is the runtime's own extraction served by its host copy. Every route
  (`open`/`stat`/`opendir`/`dlmap2file`) answers it ENOENT without
  consulting the policy: the hex leaf is per-process (unauthorable at
  bind — the path does not exist to canonicalize), the content rode in
  through the ungated extraction writes, and the tree is removed at
  process exit. This is what lets a wrapper-pattern runtime (spec 22
  §3's exec cache) compose with a `deny` jail.
- "Host path" means **any path the mounts do not hold** — outside every
  mount, or covered by a mount but absent from its image (spec 11 §2):
  the app payload mounts at `/` and the host filesystem stays
  reachable exactly through this rule.
- **The consumer must route, not just the engine** (tebako#917): a
  runtime whose IO shims call this family only for the paths the mounts
  COVER hands every uncovered path to the kernel, where this policy
  cannot see it. Such a shim asks `int tebako_fs_host_guard(void)` — 1
  when the installed policy judges an uncovered path (a deny or record
  default) — and routes every absolute path through the family while it
  answers 1. An allowed path still answers ENOENT, the pass-through
  signal these shims already honour, so an unguarded run is unchanged
  and a guarded one denies where it said it would. Coverage is a
  namespace test; a namespace test is not an enforcement point.
- A mount's access bit applies even under an `open` default (an ro bind
  is ro in an otherwise open namespace, docker-style); longest host
  prefix wins on path-component boundaries; argument files are an exact
  read-only grant, even under deny.
- The device-node set (§1's 2026-11-25 amendment, tebako#1128) answers
  allowed for read AND write ahead of every grant — the verdict's first
  check after canonicalization, so the deny stance cannot deny it and an
  ro bind over `/dev` does not narrow it. The needs generator (spec 23
  §8) excludes the same set: a manifest never claims a device node.
- Bind-time realpath covers mount sources and argument files (a missing
  one fails the install with its errno); every gated decision
  re-canonicalizes the target, so symlinks swapped in after install
  resolve to their target and escapes fail EPERM.

## 4. Manifest integration

The payload manifest (spec 03) gains `capabilities.host` — the access the
payload was built to need (e.g. metanorma: read the input file's
directory, write the output directory) — whose block carries a `network:`
list of outbound peers the payload was built to need (deny-by-default
with explicit grants; a data-only payload needs no network). Dispatch
surfaces (spec 07)
compose: manifest request ∩ user policy = effective jail, on both axes.

Since the 2026-11-22 amendment the block also carries `resources:` (§2.2
— the four consumption bounds), authored in the YAML block form. The
composition rules on the resource axis mirror the access axes' locked
precedence: **intersect (request ∩ user tightening) is per-key MIN** —
the tighter bound wins; a key present on either side survives (the
user's silence is not a tightening; the manifest's silence keeps the
user's bound) — and **union (the spec 30 §4 spawn bridge) is per-key
MAX**, because the child must be able to do what EITHER side declared,
still clamped by the kernel's inherited hard limits and by the
inherited wall deadline (§2.2).

### Shipped surfaces (roadmap 35)

- **Model:** `crates/tpkg/src/jail.rs` owns the authored shape (YAML
  block ↔ `TEBAKO_JAIL` env form ↔ `--jail` cli spec: `open` | `deny` |
  `deny:arg` | a YAML file | the env grammar) and the locked precedence
  algebra (`intersect` / `effective`): deny defaults win; each side's
  grants are capped by the other side's allowance at the same prefix (ro
  is sticky, a user `--no-host` drops request grants); argument files
  union and `auto-allowed` resolves argv files into read-only grants at
  dispatch. The payload manifest's `capabilities.host` and the package
  manifest's `jail:` block are both this type.
- **Press:** `tebako press --jail <spec>` writes the policy into the
  type-2 package manifest's `jail:` block (spec 02 §5b / spec 03 §6);
  block-less packages are byte-identical to before.
- **Bootstrap:** composes package `jail:` ∩ `TEBAKO_JAIL` at handoff and
  exports the effective policy as `TEBAKO_JAIL` (+ `TEBAKO_JAIL_SOURCE`
  = `manifest` | `user` | `manifest+user`, + `TEBAKO_JAIL_JOURNAL` →
  `$TEBAKO_HOME/journal.log`). Malformed policy fails closed, exit 73
  (`EX_TEBAKO_JAIL`). No policy: nothing exported (legacy byte-identical).
- **Dispatch:** `tebako run <pkg> [--jail <spec>] [--mount
  <host:mount:ro|rw>]... [--no-host] [--] [args...]` and the same flags on
  `tebako-shim` (consumed before `--`; the payload mirror carries
  `capabilities.host` from install time).
- **Audit journal (§2):** every denial in the TFS layer appends `<ts>
  event=jail-deny path=<p> op=read|write source=<s>` to the tebako
  journal (`TEBAKO_JAIL_JOURNAL` > `$TEBAKO_HOME/journal.log` >
  `~/.tebako/journal.log`), best-effort — journaling never fails the
  operation it audits. One exception, at policy-install time (locked
  2026-11-14): when `TEBAKO_JAIL_JOURNAL` explicitly names the journal
  and that file cannot be opened for append, a policy that can deny
  refuses to install — a named error, not a run — because a denial
  without its record is indistinguishable from a policy that never
  engaged. The implicit default-home journal keeps the best-effort
  shape, and per-event write failures on an opened journal never fail
  the audited operation.

## 5. Scope (honest, locked 2026-07-26; the network axis joins, 2026-11-07; the platform spawn boundary, 2026-11-11)

Jails cover the FILESYSTEM and the NETWORK. The `network:` list is the
enforcement contract: deny-by-default with explicit grants, judged
in-process at the preload's socket family — the §1 `net:` grant names an
outbound peer, and a `connect` to an ungranted peer fails the policy's
named refusal and journals it, exactly the fs axis's shape. Coverage:
every IO route through TFS — all interpreted payloads AND native binaries
running under the preload interposition shim (spec 07 §8 tier 1, whose IO
flows through the same `host_policy`); the exe shape (spec 22 §3.6)
exports the socket family, so an interpreted payload's own `connect`
binds to the judgment. The judged family is `connect`/`sendto`/`sendmsg`
plus the `getaddrinfo` observation (the name → address bridge a name
grant judges a numeric peer through). NOT covered: statically-linked
binaries (direct syscalls, no interposition point), any process that
escapes the shim, DNS resolution (allowed unconditionally in v1 of
the axis: a query leaks a name, never payload bytes, and a grant names
hosts whose addresses must resolve), AF_UNIX peers (the fs axis's
territory, never net-judged), the legacy resolver (`gethostbyname` —
a payload resolving through it judges by numeric spelling only),
macOS's `connectx` (no importer observed), and INBOUND service
(`bind`/`listen`/`accept` — the axis is outbound peers; a jailed payload
that listens is v2 territory) — all stated honestly in the manifest
and docs. OS-level confinement (seatbelt/seccomp/WFP) stays a possible
later layer, never claimed.

The shim's coverage is SYMBOL-level (tebako#439): a jail holds only for
the libc entry points the shim interposes — the named list in
`crates/libtfs-preload` (lib.rs "Interposed surface"), which includes
the LFS64/fortify/versioned twins (`open64`/`openat64`/`fopen64`,
`__read_chk`/`__openat_2`, the `__xstat` family through `__fxstatat64`)
because glibc hands `_FILE_OFFSET_BITS=64`/`_FORTIFY_SOURCE`/pre-2.33
callers DISTINCT symbols that never touch the plain names (the #439
hole: OpenSSL's `o_fopen.c` defines `_FILE_OFFSET_BITS=64` itself, so
every `BIO_new_file`/`X509_LOOKUP_load_file` bound `fopen64` straight to
libc). The WRITE class rides the same gate (tebako#444, closing the
pre-existing hole the #439 read-class audit left): `open`/`openat` with
`O_WRONLY`/`O_CREAT`/`O_RDWR` were policy-gated by the engine's open
from the start, and `fopen`'s write/append/update modes (`w`/`a` bases,
every `+` update form including `r+`, the `x`-forms) plus `creat` now
route through `host_policy` BEFORE the real libc call touches the host
— a jailed write outside an rw grant is EPERM by name, a write against
an ro grant EROFS, and a write naming a memfs-held path is EROFS under
any policy (payload images are always read-only). Known-uncovered at
this layer: the remaining fortify open family
(`__open_2`/`__open64_2`/`__openat64_2` — no importer observed in the
0.16.6 linux-gnu runtime) and raw `syscall(2)` IO. Separately, the
runtime EXE's own statically-linked native libraries are gated only for
symbols the exe itself defines (spec 22 §2: `dlopen`/`dlerror` today —
defining the file-IO family in the exe is follow-up wiring in the
runtime factory, tebako-runtime-ruby); an
interpreter's patched IO and preload-delivered children are covered.

**The platform spawn boundary (tebako#1021):** a jailed payload's
UNPLANNED host child — an arbitrary host binary the interpreter spawns
outside any declared edge (ruby's `IO.popen(["cat", secret])`) — is the
one place the two platforms diverge today. On POSIX the preload tier
(`LD_PRELOAD` / `DYLD_INSERT_LIBRARIES`, spec 22 §3) inherits into the
child: the interposed exec judges the exec target and the child's own
reads route through the same bound policy, so the shell-out vector is
judged exactly like an in-process one. Windows has no injection tier yet
(spec 22, phase W): a spawned host child would run plain-host with the
bound policy invisible to it, so under any non-trivial policy the
unplanned spawn boundary is FAIL-CLOSED — the interpreter's spawn path
asks `int tebako_fs_spawn_host_permitted(void)` and refuses the spawn by
name when it answers 0. The export answers 1 exactly while the bound
policy is TRIVIALLY OPEN — the open default with no mounts and no
argument files, i.e. there is nothing to enforce; a process that never
bound a policy is trivially open by construction — and 0 under any
non-trivial one. The refusal is strictly narrower than POSIX by
necessity: POSIX can judge the exec TARGET and permit a granted binary,
while on windows the child's READS would escape, so the whole unplanned
spawn is refused. PLANNED children are unaffected: the spec 30/32
spawned runtime/payload edges re-apply their own jail union in-process
from the `TEBAKO_JAIL` trio (spec 30 §4), so no inheritance is needed.
The divergence is tracked as tebako#1021 until the windows injection
tier lands.

## 6. Executor-layer trust (locked 2026-07-27)

Tebako is the EXECUTOR of everything it runs — the parent that spawns
and owns the child. Mediating a child's syscalls (preload interposition
for dynamic children, seccomp user-notification for static children on
Linux) is the execution contract, not hijacking: a hostile party would
have to become the executor or the kernel to hold the same position,
and both are outside the jail's threat model by definition. Jails bind
the CHILD downward; they claim nothing about the executor or kernel
upward.

## 7. Conformance: the attack matrix (locked 2026-10-23)

A jail is only as good as the attacks it has been shown to stop. Every
vector below is either **blocked and proven by a named test**, or
**named out of scope with the reason** — there is no third state. The
host-filesystem vectors live in `crates/tfs/tests/jails.rs`; the
environment vector lives beside its implementation
(`crates/tebako-driver/src/env_scrub.rs`); the resource axis's grammar
lives beside its owners (`crates/tpkg/src/jail.rs` the authored model,
`crates/tfs/src/policy.rs` the engine mirror — one grammar, one message
set) and its platform mechanisms beside their armer
(`crates/tebako-driver/src/resources.rs`), with the end-to-end arms
(fork bomb, infinite looper, under-bounds control) in the release-gate
suite's resource arm (`ci/jail-conformance.sh`). All run on every
platform leg of every CI run and therefore on every release: a
regression reddens the release matrix, not a dashboard nobody reads.

| # | vector | verdict | the proof |
|---|--------|---------|-----------|
| 1 | lexical escape: `..`, doubled separators, `.` components inside a grant | blocked | `dot_dot_paths_normalize_lexically` |
| 2 | symlink escape: a link inside a granted tree pointing at a denied sibling (directory and file level) | blocked (EPERM) | `symlink_escape_attempt_fails` |
| 3 | TOCTOU: the tree is swapped for a symlink AFTER the policy was bound | blocked (EPERM) | `symlink_swap_after_bind_fails_on_revalidation` — every open re-validates through realpath |
| 4 | case folding: a denied path asked for with different case on a case-insensitive host | blocked (EPERM) | `case_variants_of_a_denied_path_are_denied` — the decision is taken on the canonical path (macOS `realpath` returns the on-disk case), so it never depends on what the filesystem thinks about case |
| 5 | write into a held (image) tree | blocked (EROFS) | `ro_mount_refuses_writes_with_erofs` |
| 6 | write outside every grant under an rw mount | blocked (EPERM) | `directory_scoped_mount_rw_works_and_sibling_denies` |
| 7 | enumeration of a denied host directory | blocked (EPERM, nothing listed) | `deny_all_cannot_enumerate_or_read_but_memfs_is_unaffected` |
| 8 | reads/stats of a denied host file | blocked (EPERM, never ENOENT — the denial is a decision, not a miss) | `deny_all_cannot_enumerate_or_read_but_memfs_is_unaffected` |
| 9 | extract-all as an ungated host write | blocked (gated) | `extract_all_is_a_gated_host_write` |
| 10 | covered-but-not-held path under a `/` mount (the mount must not become a hole) | gated (the host decision runs) | `covered_but_not_held_paths_fall_through_to_the_host_decision` |
| 11 | image reads through the policy | gated | `mount_family_reads_the_image_through_the_policy` |
| 12 | unmount under a live policy | fail-closed | `policy_survives_unmount_fail_closed` |
| 13 | malformed policy spec | fail-closed (named) | `policy_install_validates_its_arguments` |
| 14 | argument-file grant: only the declared file, nothing beside it | blocked | `tight_jail_allows_only_the_argument_file` |
| 15 | packager/CI credentials reaching the payload: `TEBAKO_GITHUB_TOKEN` and every other packager-side `TEBAKO_*` variable past the handoff | blocked (blanked, never logged; `env_pass` is the explicit opt-in) | `the_packager_token_is_scrubbed_and_the_contract_kept`, `the_env_pass_opt_in_carries_its_declared_names`, `the_factory_smoke_contract_survives_the_scrub` (spec 03 §2.10, tebako#737) |
| 16 | Windows alternate data stream (`file.txt:stream`) on a denied base | PLANNED — the decision rides the base path (a stream suffix is not a path component), and the proof lands with the windows leg that can exercise it | — (named, not claimed) |
| 17 | statically-linked binary, or raw `syscall(2)` IO | OUT OF SCOPE — no interposition point; §5 states it in the manifest and the docs | §5 |
| 18 | fortify open family still uncovered (`__open_2`/`__open64_2`/`__openat64_2`) | OUT OF SCOPE — no importer observed in the shipped runtimes; the symbol list is the interposed surface's SSOT | §5 |
| 19 | the runtime EXE's own statically-linked stat/open definitions | OUT OF SCOPE today — gated only for the symbols the exe defines (spec 22 §2); follow-up wiring lives in the runtime factory | §5 |
| 20 | a payload's own sockets (network confinement) | blocked (EPERM) from the 2026-11-07 amendment — the §1 `net:` grant + the preload's socket family (`connect`/`sendto`/`sendmsg` judged, `getaddrinfo` observed for the name bridge; DNS itself never gated); a `connect` to an ungranted peer is the policy's named refusal | `net::tests::deny_judges_by_numeric_and_observed_name`, `net::tests::ipv6_peers_judge_by_observed_name`, `policy::tests::net_check_denies_only_under_deny_and_matches_port`, `policy::tests::jail_spec_parses_net_grants` + `net_grants_env_round_trip`, `needs::tests::network_lines_draft_peer_entries_preferring_the_name` |
| 21 | a jailed payload opening a listener (inbound service: `bind`/`listen`/`accept`) | OUT OF SCOPE in v1 — the axis judges OUTBOUND peers; inbound confinement is a later layer, named here so nobody reads the axis as bidirectional | §5 |
| 22 | an unplanned spawned HOST child (ruby's `IO.popen(["cat", secret])`) reading what the policy denies the parent | blocked — POSIX by the inherited preload tier (the exec target and the child's own reads are both judged); windows by the fail-closed spawn boundary (§5): under any non-trivial policy the spawn itself is refused. The end-to-end windows leg rides the runtime-side patch (tebako#1021; the conformance suite's shellout line carries a named KNOWN-GAP until a published runtime pair calls the export) | `spawn_host_permitted_is_the_platform_spawn_boundary`, `policy::tests::trivially_open_is_exactly_the_unauthored_open_default`, the contract C harness's `spawn_host_permitted=1` line |
| 23 | a fork bomb under a declared `processes` bound | bounded — `fork` fails `EAGAIN` at the `RLIMIT_NPROC` ceiling; the bomb dies by name at the bound and the watchdog reaps its sleeper children with the root (§2.2's die-with-root) | the conformance suite's resource arm (`ci/jail-conformance.sh` — POSIX, `--pair`); `resources::tests` (the rlimit mapping + the syscall-level fork probes) |
| 24 | an infinite-looper under a declared `wall_seconds` bound | killed at the bound — the watchdog's named stderr line, the `event=jail-resource` journal line, the process group's `SIGKILL`; the observed exit is the signal | the conformance suite's `wall` arm; `resources::tests::the_watchdog_kills_the_group_at_the_deadline` |
| 25 | an infinite-looper under a declared `cpu_seconds` bound | `SIGXCPU` death at the `RLIMIT_CPU` ceiling | the conformance suite's `cpu` arm; the `resources::tests` rlimit mapping |
| 26 | an under-bounds control run (generous `cpu_seconds`/`memory`/`processes`/`wall_seconds` declared, nothing stressed) | passes byte-identically to the unbounded run — the axis never perturbs an in-budget payload (the memory declaration rides on Linux only; macOS omits it — the kernel's RLIMIT_AS floor, §2.2) | the conformance suite's `control` arm |
| 27 | windows resource enforcement | SHIPPED (2026-11-23, tebako#1101) — Job Objects (§2.2): `PROCESS_TIME` / `PROCESS_MEMORY` / `ACTIVE_PROCESS` (+ `KILL_ON_JOB_CLOSE` when the wall arms), the watchdog thread's exit-137 wall verdict, the kernel's close-at-exit reap; the fail-closed stub is gone | the conformance suite's windows resource arm (`ci/jail-conformance.sh` — windows, `--pair`); `resources::tests` (the job-limit mapping, the shared deadline algebra) |
| 28 | the preload-only delivery (`tfs exec`) of the resource axis | PLANNED — the preload constructor is the named seam; today the `tfs exec` surfaces refuse a `resources:`-carrying spec by name, and a driver-rooted bounded tree's native children are kernel-inherited-bounded (§2.2) | §2.2 |
| 29 | a payload calling `setsid(2)` to escape the wall group | OUT OF SCOPE — POSIX userland without cgroups cannot hold a determined escapee (the escape needs the payload's own syscall); cgroup-grade per-tree accounting is the named non-goal (§5) | §2.2 |
| 30 | a declared `memory` bound on macOS | refused by name at boot (exit 73) — the kernel refuses `RLIMIT_AS` below its 2^39-byte address-space floor (arm64); never a silent clamp | the conformance suite's macOS `memory-refusal` arm; `resources::tests`' rlimit mapping |
| 31 | a memory hog under a declared `memory` bound (Linux) | address-space growth fails at the `RLIMIT_AS` ceiling — the interpreter's own allocator answer (ruby: `NoMemoryError`, or its fatal-allocation abort), never a silent overshoot | the conformance suite's Linux `memory` arm |
| 32 | a fixed-set device node under a deny stance (`/dev/null`, `/dev/zero`, `/dev/random`, `/dev/urandom`; `NUL` on windows) — the `system(…, out: File::NULL)` idiom (tebako#1128) | exempt from the 2026-11-25 §1 amendment — always read+write under every policy, no grant narrows it; a lookalike (`/dev/nullify`) or any other node (`/dev/sda`) stays the policy's verdict | `policy::tests::device_nodes_are_exempt_under_deny_all` + `device_exemption_is_not_narrowed_by_a_grant` (the windows arm `nul_is_exempt_under_deny_all`), `jails.rs`'s `device_nodes_pass_through_under_deny_all_but_lookalikes_are_refused`, the driver boot leg `jail_deny_exempts_the_device_nodes`, the conformance suite's `devnull`/`devnullspawn` vectors |

Row 16 is the one open vector that is IN scope and not yet proven: it is
named here so nobody reads the matrix as "done", and it does not close
until a windows test stands behind it. Row 28 names the resource axis's
remaining open arm the same way — the axis ships driver-first (POSIX and
windows) and says so.
