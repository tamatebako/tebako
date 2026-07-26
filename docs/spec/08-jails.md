# Spec 08 — Jails: host-access policy

Normative specification of host-filesystem access control for running
payloads. Status: PLANNED (roadmap 09). Enforcement exists because every
file access already flows through the TFS layer — one choke point covers
every consumer.

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

## 4. Manifest integration

The payload manifest (spec 03) gains `capabilities.host` — the access the
payload was built to need (e.g. metanorma: read the input file's
directory, write the output directory). Dispatch surfaces (spec 07)
compose: manifest request ∩ user policy = effective jail.
