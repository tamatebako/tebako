# Spec 08 — Jails: host-access policy

Normative specification of host-filesystem access control for running
payloads. Status: PARTIAL (roadmap 09) — the TFS enforcement point (§1–§3)
is SHIPPED in `crates/tfs` (policy model, `tebako_fs_host_policy`,
per-route EPERM/EROFS gating, symlink re-validation, jail acceptance
suite); the `TEBAKO_JAIL` env form of the policy (§1) is SHIPPED with
roadmap 30's preload shim (`tfs exec --jail` carries it to native
binaries); manifest integration (§4), the remaining dispatch-surface
flags and the audit-journal logging of violations remain PLANNED.
Enforcement exists because every file access already flows through the
TFS layer — one choke point covers every consumer.

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
  (profile 1: "cwd + writes pass through"). Memfs paths are byte-for-byte
  unchanged (write opens on them stay EROFS).
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
