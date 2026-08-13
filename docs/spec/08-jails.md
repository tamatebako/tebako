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
one choke point covers every consumer.

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
- "Host path" means **any path the mounts do not hold** — outside every
  mount, or covered by a mount but absent from its image (spec 11 §2):
  the app payload mounts at `/` and the host filesystem stays
  reachable exactly through this rule.
- A mount's access bit applies even under an `open` default (an ro bind
  is ro in an otherwise open namespace, docker-style); longest host
  prefix wins on path-component boundaries; argument files are an exact
  read-only grant, even under deny.
- Bind-time realpath covers mount sources and argument files (a missing
  one fails the install with its errno); every gated decision
  re-canonicalizes the target, so symlinks swapped in after install
  resolve to their target and escapes fail EPERM.

## 4. Manifest integration

The payload manifest (spec 03) gains `capabilities.host` — the access the
payload was built to need (e.g. metanorma: read the input file's
directory, write the output directory). Dispatch surfaces (spec 07)
compose: manifest request ∩ user policy = effective jail.

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
  operation it audits.

## 5. Scope (honest, locked 2026-07-26)

Jails are FILESYSTEM-ONLY. `capabilities.network` is advisory metadata
(displayed by the info surface, spec 15), not an enforcement promise.
Coverage: every IO route through TFS — all interpreted payloads AND
native binaries running under the preload interposition shim
(spec 07 §8 tier 1, whose IO flows through the same `host_policy`).
NOT covered: statically-linked binaries (direct syscalls, no
interposition point) and any process that escapes the shim — stated
honestly in the manifest and docs. OS-level network confinement
(seatbelt/seccomp/WFP) is a possible later layer, never claimed today.

## 6. Executor-layer trust (locked 2026-07-27)

Tebako is the EXECUTOR of everything it runs — the parent that spawns
and owns the child. Mediating a child's syscalls (preload interposition
for dynamic children, seccomp user-notification for static children on
Linux) is the execution contract, not hijacking: a hostile party would
have to become the executor or the kernel to hold the same position,
and both are outside the jail's threat model by definition. Jails bind
the CHILD downward; they claim nothing about the executor or kernel
upward.
