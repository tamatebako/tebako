# Spec 22 — Runtime-native interposition (the generalized hooks)

Normative contract for how a tebako runtime serves interpreter-level
operations whose targets live in the VFS: loading native libraries,
executing binaries, and exposing resource files to C libraries that
bypass the interpreter's own IO. Language-agnostic in shape; ruby is
the first implementation.

**The law (locked 2026-08-11):** interception is RUNTIME-INTERNAL and
GENERIC. No per-payload and no per-gem code anywhere in the stack — a
payload that exercises these paths must need zero adapter work. The
mechanisms below are the ONLY homes for such behavior. (The v1-era
per-gem require adapters in the runtime gem are the legacy form; this
spec retires them. Migration: §7.)

## 1. The three classes (the complete taxonomy)

Every interception need reduces to exactly one of:

| Class | Trigger | Consumer examples | Fails because |
|---|---|---|---|
| **L — native-lib load** | `dlopen`/`LoadLibrary` of a VFS path | interpreter's own `dln_load`, ffi, fiddle, C extensions self-loading | the OS loader reads real files only |
| **E — exec of a VFS binary** | `execvp`/`posix_spawn`/shell strings whose argv0 is VFS-resident | jing/mn2pdf's java, inkscape CLI, any payload tool | the kernel execs real files only |
| **R — C-library resource paths** | a C lib reads a path through its own IO (outside the interpreter's patched IO) | OpenSSL reading the CA cert, ICU data files | the path doesn't exist on the host |

Anything that does not fit L/E/R is a DESIGN SMELL — stop and extend
this spec, never bolt on a one-off.

## 2. Class L — loader interposition

**Rule L1.** A load of a native library whose path resolves inside a
mounted image MUST materialize the library (and its dependency closure)
to the runtime's exec cache and load the real file. A load of a host
path MUST pass through untouched. The decision is a path-prefix check
against the mount table — nothing more.

**Rule L2.** The interposition point is the process's own loader call,
so every consumer is covered at once:

- **ELF (linux-gnu/musl):** the runtime binary defines `dlopen`/`dlsym`
  wrapper symbols; the main binary's definition preempts for the whole
  process (the interpreter, its C extensions, and any library they
  pull). `--wrap` is FORBIDDEN (it only covers the link unit's own
  calls).
- **macOS:** the interpose mechanism (`__interpose` section) — the same
  mechanism the preload layer uses for exec. dyld honors tuples only
  from dylib images, so delivery is driver self-insertion (see "Phase 1
  delivery" below).
- **Windows (msys):** inside the interpreter's own dln path (the
  patched `dln.c` — the `dln_c_dlmap_msys` route carried from v1).
  Loads that bypass `dln.c` — raw `LoadLibrary`/`LoadLibraryExA` from
  fiddle, ffi, or a C extension self-loading — are the documented edge
  (rare — evaluate per case, never silently); the per-case evaluation
  is the windows delivery record below.

**Phase 1 (POSIX) mechanics.** The interposed symbols are `dlopen` and
`dlerror`, on both ELF and macOS. `dlsym` is deliberately NOT interposed
in phase 1: the `dlopen` wrapper returns real loader handles, so `dlsym`
needs no routing — and an exe-defined `dlsym` cannot resolve its own
original on musl (no `dlvsym`) without self-recursion. `dlerror` IS
interposed: a failed VFS materialization must surface the tebako context
line (the library, the mount, the verdict) through the standard dlopen
error channel, never a stale loader message.

**Phase 1 delivery (locked 2026-08-11, verified empirically).** The
mechanics above are uniform; the DELIVERY is platform-split:

- **ELF:** the ruby `dln_c_loader_interpose` patch carries the
  exe-defined `dlopen`/`dlerror` wrappers; the main binary's definition
  preempts `libdl`'s for the whole process (the interpreter, its C
  extensions, and anything they pull). Originals resolve via
  `dlsym(RTLD_NEXT, …)`.
- **macOS:** dyld honors `__interpose` tuples only when they arrive in
  a DYLIB image — tuples in the main executable are silently ignored
  (verified: main-exe tuples never fire; dylib tuples apply
  process-wide; `dlopen()` of an interposer after launch does NOT
  activate it; `DYLD_INSERT_LIBRARIES` works). The driver therefore
  SELF-INSERTS at the head of `tebako_driver_boot`, before any mount
  and before the interpreter starts: it writes an embedded micro
  interpose-dylib (compiled in the product repo, binding the exe's
  `tebako_fs_*` exports via `-undefined dynamic_lookup` — one VFS
  context, no third artifact) to a content-keyed cache path, sets
  `DYLD_INSERT_LIBRARIES` plus the sentinel `TEBAKO_LOADER_INTERPOSED=1`,
  and `execv`s itself. The sentinel makes the re-exec fire exactly
  once; the re-exec precedes all mounting, so there is no double boot,
  no partial-mount window, and no launcher-ABI change.

**Windows (msys) delivery record (design-pinned 2026-08-14, phase W1;
the implementation is W2).** Windows has no process-wide preemption
surface (no exe-definition preemption as on ELF, no interpose section
as on macOS), so coverage extends exactly as far as callers that route
through the interpreter's own `dln.c`. What the msys dln dlmap route
(tamatebako/ruby `patches/*/dln_c_dlmap_msys.patch`, wired into every
msys patch manifest) covers TODAY:

- The interpreter's own native-extension load (`require` of a PE
  extension through `dln_load`/`dln_open`): a
  `tebako_path_is_embedded`-covered path materializes through
  `tebako_fs_dlmap2file` and the host twin loads via `LoadLibraryW`.
  The factory's boot smoke asserts exactly this on the msys legs
  (`load_native_extension`: racc's `cparse.so` extracts from the memfs
  and binds).
- Host passthrough per Rule L1: an uncovered host path loads
  untouched; a covered path the mounts do not HOLD answers ENOENT from
  the c_api and the host conversion serves it from the host — the same
  jail-gated passthrough precedent as POSIX.
- The extension's import of the ruby DLL binds by LAYOUT, not by
  closure walking: the loader's standard search order heads with the
  running exe's own directory, and the store entry carries the
  PE-named DLL (`x64-ucrt-ruby<ABI>.dll`, the release manifest's
  `install_as` — single owner: the factory's
  `RubyVersion#msys_dll_name`, flowed through the manifest and
  assembled next to the exe, sha256-verified, by the bootstrap's
  install in `crates/tebako-bootstrap`; the factory boot smoke mirrors
  the same materialization in-step). Upstream's
  `rb_w32_check_imported` then rejects an extension bound against a
  DIFFERENT ruby DLL with its own named "incompatible library version"
  LoadError — the windows form of the ABI-line guarantee.
- Symbol resolution needs no routing: the loads above return real
  loader handles, so `GetProcAddress` works unmodified — the same
  reasoning as phase 1's `dlsym` decision.

What fiddle/ffi NEED on windows and do not get today: both load
through RAW loader calls that never enter `dln.c` — fiddle's
`Fiddle.dlopen` is a `LoadLibrary` macro (`ext/fiddle/fiddle.h` in the
interpreter's vendored source), ffi's `FFI::DynamicLibrary.open` is
`LoadLibraryExA` (absolute paths carrying
`LOAD_WITH_ALTERED_SEARCH_PATH`). Each needs the same
materialize-then-load the dln route performs; a VFS path through
either today fails with the OS loader's own honest error (126,
module-not-found) — an honest failure, never a tebako-intercepted one,
never a silent success.

The raw-`LoadLibrary` edge cases, per case (the phase-W exit gate's
record):

| Case | Verdict |
|---|---|
| `dln_load` of a VFS-resident extension (the `require` path) | **covered** — the dlmap route above |
| host path uncovered by any mount | **covered** — passthrough untouched |
| covered but not held (a host path under a covering mount) | **covered** — ENOENT → host passthrough |
| extension importing the ruby DLL | **covered** — the exe-dir layout + `rb_w32_check_imported`'s named ABI error |
| extension importing OTHER VFS-resident DLLs (sibling vendor imports) | **documented gap** — two compounding layers: the closure walk parses Mach-O/ELF only (`crates/tfs` `exec_closure` — no PE import parsing), and plain `LoadLibraryW` does not search the loaded DLL's own directory (no `LOAD_WITH_ALTERED_SEARCH_PATH` in `dln_open`), so even a materialized sibling would not bind; the failure is the OS loader's honest 126. No proven consumer — the only in-image PE import a leg has proven is the ruby DLL itself |
| fiddle `Fiddle.dlopen` of a VFS path | **documented gap** — raw `LoadLibrary` bypasses `dln.c`; honest OS failure |
| ffi `FFI::DynamicLibrary.open` of a VFS path | **documented gap** — raw `LoadLibraryExA`; honest OS failure |
| a C extension self-loading a VFS path via raw `LoadLibrary` | **documented gap** — the same mechanism class as fiddle/ffi |
| failed materialization through `dln_open` (covered path, non-ENOENT dlmap failure — a directory, a jail-denied passthrough) | **named error — phase-W fix** — the v1-era route's `goto failed` raises `LoadError` WITHOUT the tebako verdict (no library/mount/errno context); W2 delivers the §5 verdict line through the standard error channel, matching the POSIX leg's dlerror contract |

One design question this record deliberately does NOT settle (a W2
decision, taken with the owner — recorded so no leg of it happens
silently): fiddle ships IN the env image (the interpreter's vendored
source), so routing its loader macro through the c_api would be
runtime-internal in the same sense the `dln.c` patch is; ffi is a
third-party gem, where that move is the per-gem code this spec's law
forbids. Whether fiddle gains the vendored route and ffi/self-loads
stay a permanent documented gap is named here, undecided.

**Rule L3.** Materialization reuses the spec-17 exec-closure walk
(Mach-O/ELF dependency closure, content-keyed cache dir, write-once).
The cache is per runtime image sha — a rebuilt runtime never reads a
stale extraction. Materialization failures are named errors naming the
library and the mount that served it; never a partial load.

**Rule L4.** The interposition is read-only with respect to the store:
it never installs, upgrades, or deletes anything (a run is a run).

## 3. Class E — exec interposition

**Rule E1.** The interpreter's spawn hook (the process-layer patches)
covers the array-form spawn of an absolute VFS path: materialize
parent-side, point the exec pair at the host copy, inject the child
with the preload shim and the current mounts. This spec EXTENDS
coverage to the exec level — `execve`/`posix_spawn` ride the same
interpose surface as Class L, so every in-process exec caller is
covered at once, and libc `execvp`/`posix_spawnp` PATH loops resolve
through the interposed `execve`/`stat`. What each platform DELIVERS is
pinned empirically in §3.1.

### 3.1 The per-platform delivery matrix (locked 2026-08-13, verified empirically)

| Platform | The runtime process | Array form, absolute VFS path | Shell string (`system("java -jar /vfs/x.jar")`) |
|---|---|---|---|
| **ELF (gnu/musl)** | exe-defined Class-L symbols; the runtime itself is never preload-injected | the spawn hook materializes and injects the child (`LD_PRELOAD` + `TEBAKO_TFS_MOUNTS` in its env) | **works unmodified** — the handoff env's `LD_PRELOAD` injects every child at its exec, `/bin/sh` included; the shell's PATH search and `execvp` loop route through the interposed surface |
| **macOS** | the driver's self-inserted interpose dylib (§2 phase 1) | the same hook; the materialized child is a non-Apple binary, so `DYLD_INSERT_LIBRARIES` is honored | **named boundary, enforced by the spawn hook** — an inherited `DYLD_INSERT_LIBRARIES` is FATAL to Apple platform binaries on darwin24 (dyld TERMINATES `/bin/sh` and `/usr/bin/cc` under a foreign insertion — factory run 31699651270; darwin23 stripped the variable instead), so the interpreter's spawn hook DROPS the variable per spawn whose target is restricted (any shell form; anything resolving into Apple's system binary dirs). The JVM behind a shell string then answers its own `Unable to access jarfile` — an honest host failure, never a tebako-intercepted one. darwin24 x86_64 CI runners HONOR the insertion into sh's exec child (runs 31685052887/31692800485) — a relaxed-SIP artifact the scrub deliberately erases: every host behaves like the strictest one |
| **windows** | — | deferred with windows Class L (§7 order) | deferred |

Where the launcher tier (§3.2) is armed, the shell-string form works
on every macOS host — the launcher's explicit re-arm passes the hook's
scrub exactly as it passes dyld's strip. Where it is not (no shim
delivered, or the image declares no executables), the macOS consumption
pattern for a dependency's binary is the array form with the absolute
path (resolved through §3.2's surface); unmodified shell-string
consumers of VFS binaries remain an ELF capability. (darwin24 x86_64 CI
runners tolerated the insertion past `/bin/sh` — a relaxed-SIP
artifact; the hook's scrub erases it so every host behaves like the
strictest one.) A shell string whose operands are all host paths
behaves exactly as on any host — the boundary is the VFS-operand case
only.

**Rule E2.** Exec of a VFS-resident binary materializes the binary plus
its loader closure (the same exec cache as Class L —
`TEBAKO_EXEC_CACHE`, §6) and execs the real path with the original
argv/env. Exec of a host path passes through. A covered path the
mounts do not hold answers ENOENT: the exec fails honestly, and a libc
PATH loop simply moves to its next candidate.

**Rule E3.** A VFS binary that itself spawns children re-enters the
same interposition: on ELF the inherited `LD_PRELOAD` covers every
descendant's exec; on macOS the spawn hook's child-env injection
covers the array-form chain and the runtime's own surface covers
in-process exec callers. Children of materialized binaries keep the
VFS view — never a silent host fallback.

### 3.2 Bare-name resolution — the composition layer wires PATH

A bare command name (`system("java …")`, mnconvert's form) resolves
with no payload code learning tebako: **the driver prepends every
co-mounted dependency image's declared bin dirs to `PATH` in the
handoff env** — the dirname of each entrypoint path in the image's own
manifest (the image declares, the driver flows; no second copy of the
knowledge anywhere). On ELF the interposed exec loop then resolves the
bare name through the VFS (§3.1). The explicit-reference surface for
everything else — windows-safe and shell-free — is
`TEBAKO_MOUNT_<SLUG>` per dependency mount (§6; v2-1/20), for payload
authors who compute paths themselves.

**The host-launcher tier** (armed when the env image delivers the
preload shim, §3; unix): the driver additionally materializes each
dependency's declared executables through the exec cache and mirrors
them as self-injecting launchers on ONE host dir
(`<exec-cache-leaf>/wrap-bin/`) that LEADS `PATH`. A launcher is a
one-line POSIX script that re-arms the platform's injection var
explicitly and execs the materialized binary: SIP strips an INHERITED
`DYLD_INSERT_LIBRARIES` at an Apple-binary exec (§3.1's named
boundary), but a variable a script sets itself survives — so the
shell-string form (`system("java …")`) resolves through `PATH`, runs
the launcher, and the shim loads into the final binary exactly as on
ELF (probe 2026-08-13: `/bin/sh -c <launcher>` loads the dylib past
the strip; the bare-name and shell-string forms both work). On ELF the
launchers ride over the inherited `LD_PRELOAD` (harmless — the same
dir leads `PATH`); windows has no launcher tier yet (§7's order).
Triple order wins on a basename collision; a declared executable that
cannot be materialized is the image lying (a named 65), never a
skipped entry.

### 3.3 The class-E proof fixture

The boot-smoke fixture jar is PRECOMPILED, checked into the factory's
fixtures directory next to its `.java` source and a regeneration note
(sha256-pinned). No CI leg needs a JDK: the fixture is a data file to
the smoke, exactly like the C fixtures' compiled form on legs without
a compiler.

### 3.4 Spawned children and the jail — the platform floor (locked 2026-08-14)

A spawned child inherits the process's jail through the handoff env's
`TEBAKO_JAIL`, and the preload shim enforces it inside the child. What
the child may read on the host is therefore exactly what the bound
policy grants — and under a scratch-only jail
(`deny;<scratch>:<scratch>:rw`) a materialized JVM could not finish its
own boot: its locale/framework init reads under `/usr`, the denial
surfaced as a NULL deref, and the process died with a SIGSEGV at
`getMacOSXLocale` — never a named error (phase-E dogfood, 2026-08-13).
The jail's failure mode for a missing grant MUST be a policy verdict
(EPERM on an authored path), not a segfault in someone else's library.

The answer is spec 08 §2.1's **platform floor**: every policy bound
under the `deny` default gains the platform's read-only boot surface —
macOS `/usr`, `/System`, `/Library`; windows `%SystemRoot%\System32`,
`SysWOW64`, `Fonts`; other unix nothing yet (entries are added only
with a proven consumer). The floor applies at `HostPolicy::bind`, so
the driver, the shim, the bootstrap, and `tebako run` enforce it
identically with no per-surface work; an authored grant covering a
floor path supersedes it (the floor never narrows); and because every
bind re-derives it, a child re-binding its inherited `TEBAKO_JAIL`
enforces exactly its parent's policy. The operator burden of "jail
policy must include platform grants" is retired.

What the floor does NOT grant — by design — is the workload's own tool
tree and the user's home. The journal-pinned evidence chain (macOS,
openjdk 21.0.2, 2026-08-14): with the floor bound, a scratch-only jail
fails the JVM launcher's `jvm.cfg` open with its own named error (the
JRE tree is the operator's tool, never platform surface); with the JRE
granted `ro`, the CFPreferences locale probe stats the passwd-entry
home (the `HOME` env does not redirect it) and aborts with the JVM's
named `InternalError: platform encoding not initialized` — the journal
carries `deny /Users/<u> read`, never a segfault. The booted-child
stack under a `deny` jail is therefore three named ingredients: the
floor (system surface, automatic) + the tool tree (an authored `ro`
grant — the JRE) + the user domain (an authored home read, or a
redirected home the policy grants). Home never joins the floor: an
operator's `deny` must not silently read-expose private data, and the
prefix grammar cannot express the "stat-only" grant the probe needs.
The acceptance leg (`jailed_exec`, §3) runs exactly
`deny;<scratch>:<scratch>:rw;<jre>:<jre>:ro;<home>:<home>:ro`: the JVM
boots and runs the VFS jar, and every remaining journal denial is a
non-fatal fallback the operator actually named (`/etc/localtime` → UTC,
`hsperfdata` → skipped, the TMPDIR parent, the `.hotspotrc` probes).
The floor's promise is the end of the segfault class: every missing
grant surfaces as the workload's own named error, pinned in the audit
journal — never a crash in someone else's library.

## 4. Class R — declarative boot materialization

**Rule R1.** An image manifest MAY declare `materialize: [paths]`
(spec 03 §2.4; `payload-manifest.yaml` schema_minor 1 — old readers
ignore the key, new readers enforce). Entries are absolute in-image
paths of regular files, carry no `..` components (validated at parse),
and any kind may declare. The driver extracts the listed paths after
the mounts and the jail, before the interpreter handoff — in both boot
shapes (the standalone env-image boot and the `--tebako-image`
grammar); the env image's own declarations extract first (the cert
case), then each payload's in triple order. Each declared path `P`
lands at `<TEBAKO_EXEC_CACHE>/resources/<image-key>/<P>`, where
`<image-key>` is the exec cache's segregation idiom (the store
sidecar's sha prefix when the image came from the store, else a key
derived from the image path — the same rule the cache root itself
uses).

**Rule R2.** The canonical consumer pattern is an image-OWNED default:
the image that ships a resource also ships the configuration pointing
at its materialized location (the runtime env image's own OpenSSL cert
default is the first entry). Payloads needing host-visible resources
declare them in their own manifests; a consumer reads the materialized
path through the documented cache-root convention (§6).

**Rule R3.** Materialization is whole-file, read-only, and verified.
The mechanics (locked):

- **Write-once.** The first boot to need `P` streams it from the
  mounted image to a per-process staging file, hashing in flight with
  the tfs-merkle-1 file construction, re-hashes the staged copy, and
  refuses to install a copy that does not hash to the bytes the image
  served. The digest record `<P>.tfs-digest` is renamed into place
  BEFORE the content file, so a crash never leaves content without its
  record — and content without a record is foreign by construction.
  Later boots reuse the existing copy.
- **Per-boot verification.** A reused copy is served only after it
  re-hashes to its recorded digest. A mismatch, a missing record, or a
  corrupt record is the cache tampered or corrupt — a named 70 (spec
  06 §4's sha256-mismatch code), never a silently served corruption.
  The remedy is named in the error: remove the image's resources
  directory to force re-extraction.
- **The trust chain.** The image itself is verified at fetch/install
  (spec 09); the record pins the cache copy to the bytes the image
  served; the per-boot rehash pins the copy to the record.
- **Read-only.** The installed copy is made read-only after the rename.
- **Named failures.** A listed path absent from the image, or not a
  regular file, is a named 65 (the manifest lied), never a skipped
  entry.

## 5. Error model

All three classes keep the established exit-code discipline (spec 06):
the loader-side failures keep the loader's named codes (65–78); a
runtime-side load/exec failure surfaces as the interpreter's own error
with the tebako context line (the library/binary path, the mount, the
materialization verdict). No silent fallbacks: if a VFS load/exec/
materialization cannot complete, the named error is the outcome —
never a host-path shadow.

For an interposed load the context line rides the dlerror channel: the
interposed `dlerror` (§2) answers a failed VFS `dlopen` with the tebako
line, so the caller's standard `dlopen`/`dlerror` handling reports the
tebako verdict instead of whatever the loader last recorded.

## 6. The documented interface (the stable surface)

Payload authors and runtime factories may rely on, forever:

- **Path rule.** A native library or executable inside a mounted image
  loads/execs exactly as if it were on disk, from any caller in the
  process. No gem- or payload-specific action is ever required.
- **The exec cache root.** `TEBAKO_EXEC_CACHE` (read-only to payloads):
  the directory where materialized binaries/libraries live for the
  process's lifetime. Its content is an implementation detail; its
  existence and per-image-sha segregation are contractual.
- **The materialized-resource convention.** A manifest's
  `materialize:` entry `P` lands at `<exec-cache>/resources/<image-key>/<P>`
  (`<image-key>` per Rule R1). The sidecar `<P>.tfs-digest` is cache
  bookkeeping — the verification record, never a consumption path.
  Images that ship resources document their consumption path in their
  own manifests (spec 03 annotations).
- **The discovery surface.** `TEBAKO_MOUNT_<SLUG>` per dependency mount
  (spec 17 §2's env table; v2-1/20) — the portable way to reference a
  dependency payload's files, windows included. **The slug grammar is
  mount-path-derived (locked 2026-08-14):** drop the drive qualifier
  (`A:/tools/x` slugs like `/tools/x`), uppercase every ASCII
  alphanumeric and map every other character to `_`, trim leading and
  trailing underscores (`/opt/openjdk` → `OPT_OPENJDK` →
  `TEBAKO_MOUNT_OPT_OPENJDK`); interior separators are NOT collapsed
  (`/a//b` → `A__B`). A mount whose slug is empty after the trim —
  the root mount — slugs `ROOT` and exports NOTHING: `TEBAKO_MOUNT_ROOT`
  is the mount-root OVERRIDE var (spec 17 §1), so the name is reserved,
  never emitted. Union members sharing one physical point get one var;
  two DIFFERENT physical points slugging alike is an authoring
  ambiguity — a named boot error, never a silent winner. The value is
  the physical mount point (drive-qualified on windows),
  re-rooting-proof.
- **Dependency `PATH` wiring.** The handoff env's `PATH` leads with the
  launcher dir (`<exec-cache-leaf>/wrap-bin/`, when the shim is
  delivered — §3.2's host-launcher tier) followed by every co-mounted
  dependency image's declared bin dirs. Bare-name exec of a
  dependency's tool needs no payload code, and the shell-string form
  works unmodified past the SIP strip (§3.1) exactly as on ELF.

Everything else (the mount table layout, the closure-walk order, the
cache's on-disk naming) is implementation detail and may change between
runtime releases.

## 7. Migration and parity (invariant 8)

- The require-hook MECHANISM in the runtime gem stays (a legitimate
  extension point); the maps SHIP EMPTY once a class lands.
- During migration the adapter wins where present and the
  interposition catches the rest. An adapter's deletion merges only
  with the dogfood green WITHOUT it (metanorma + fontist are the
  oracle pair: ffi/fiddle/excavate-load + java-exec + cert-resource
  are all exercised there).
- Landing order: Class L POSIX → Class E → Class R → windows L. Each
  lands behind its dogfood proof; a failed proof keeps the adapter.

## 8. Acceptance

- A payload whose gems load native libraries through ffi, fiddle, AND
  the interpreter's own ext loader works on a runtime built with zero
  per-gem adapters (the fontist payload is exactly this).
- A payload binary spawning another payload's binary by shell string
  works with no adapter (the metanorma → openjdk java case).
- The runtime gem's require maps are empty; its remaining content is
  the memfs cache helpers.
- The v2 dogfood suite is green with the maps empty on every
  published platform.
