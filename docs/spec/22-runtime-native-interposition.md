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
  is the windows delivery record below, and its phase-W resolution is
  §2.1 (the design; implementation W2).

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

The fiddle-vs-ffi question this record raised (fiddle ships IN the env
image — the interpreter's vendored source — so routing its loader macro
through the c_api is runtime-internal in the same sense the `dln.c`
patch is; ffi is a third-party gem, where that move is the per-gem code
this spec's law forbids) is SETTLED in §2.1 (phase W, locked
2026-08-15): the covered surface extends to fiddle; ffi and self-loads
are covered by DECLARATION, never by patching and never by an
interception surface windows does not have. (Superseded in one class on
2026-08-18 — the §2.1 amendment: the dln-load IAT rebind IS an
interception surface for path-carrying raw calls, so ffi/self-loads of
VFS paths are covered by interception; the declaration route stands for
bare names.)

### 2.1 Windows Class L — the phase-W design (locked 2026-08-15; implementation W2 — DELIVERED 2026-08-18, amended same day)

The delivery record above is the as-shipped baseline (phase W1); this
section is the target the W2 implementation lands, and §8's windows row
is its proof. Seven locked decisions; nothing below happens silently.

W2 DELIVERED (2026-08-18 — tamatebako/ruby#86, tebako@d4975b1c +
the PE closure walk): the PE closure walk, the
`LOAD_WITH_ALTERED_SEARCH_PATH` binding, the §5 verdict line, and the
bare-name alias route landed as designed. ONE amendment to the locked
text (the incident-13 discovery, recorded at the interposition paragraph
below): the dln-load IAT rebind gives windows a per-module interception
surface after all, so ffi/self-loads of VFS PATHS are covered by
interception — the 2026-08-15 settlement's declaration-only verdict for
that class is superseded. The declaration route stands unchanged for
BARE names.

**The interposition point.** Windows has no process-wide preemption
surface (no exe-definition preemption as on ELF, no interpose section
as on macOS — the record above), so the interposition point is the
interpreter's own loader path: the patched `dln.c` (the
`dln_c_dlmap_msys` route), a THIN SHIM whose only jobs are the Rule-L1
path-prefix decision and the `tebako_fs_*` call — every mechanic lives
behind the c_api in the Rust TFS (one owner; the C surface stays
decision-only, invariant 3). The covered surface extends to fiddle:
fiddle ships in the env image (the interpreter's vendored source), so
routing its loader macro (`ext/fiddle/fiddle.h`) through the same c_api
entry is runtime-internal in exactly the `dln.c` sense, from the same
patch-set home (tamatebako/ruby). ffi is a third-party gem, and a
self-loading C extension is third-party code: patching either is the
per-gem work this spec's law forbids, and process-wide interception of
their raw loader calls is a surface windows does not have. They are
covered by DECLARATION (the bare-name rule below), never by patching,
never by interception. The fork's axis is therefore not fiddle-vs-ffi
but vendored-runtime vs third-party: everything the runtime ships
routes through the c_api; everything else is a declaration plus the
OS's own mechanisms.

**AMENDMENT (2026-08-18, incident 13 — tamatebako/ruby#86).** The
"no interception surface" premise above was wrong in one place. There
is no PROCESS-wide preemption surface, but there is a per-module one:
every extension module enters the process THROUGH `dln_open` (the
`require` path — ffi's `ffi_c` and fiddle included), so the patched
`dln.c` rebinds each just-loaded module's IAT entries for
kernel32/KernelBase `LoadLibrary{A,W,ExA,ExW}` to four static shims at
load time. A shim runs the same Rule-L1 decision and the same c_api
calls as the dln route (`tebako_path_is_embedded` →
`tebako_fs_dlmap2file`, the alias union via `tebako_fs_dlalias2file`),
then invokes the captured original on the materialized host copy with
`LOAD_WITH_ALTERED_SEARCH_PATH` (unless the caller passed its own
`LOAD_LIBRARY_SEARCH_*` bits — 0x1F00 respected); `ENOENT` falls
through to the original byte-identical; any other dlmap failure returns
NULL with a named `SetLastError` (EACCES/EPERM/EROFS→5, ENOMEM→8,
EIO→1117, else→1359). Originals are captured from the slot being
replaced; already-shimmed slots are skipped; only data slots are
written (no code patching) and `VirtualProtect` is restored. This is
runtime-internal in exactly the `dln.c` sense — the interposition point
is still the interpreter's own loader path, extended to the modules
that path loads; no third-party gem is patched, and the C surface stays
decision-only (invariant 3). Consequences: ffi/self-loads of VFS PATHS
are COVERED BY INTERCEPTION (the edge table below), and the fiddle
vendored-route question is moot — the rebind covers fiddle's raw macro
without a `fiddle.h` change. The boundary, stated honestly: a module
the OS loads as a mere dependency (never through `dln_open`) gets no
rebind — its imports resolve through the PE closure walk and the
altered-search binding instead; and the rebind routes PATH-carrying
calls only — BARE names still mean host by default unless declared
(the bare-name rule below is untouched by the amendment).

**Materialization and the PE closure walk.** A VFS-resident load
materializes through `tebako_fs_dlmap2file`, whose closure walk
(`crates/tfs` `exec_closure` — the single closure-walk owner; the C
side learns nothing about PE) gains the PE import directory as a third
parsed format beside Mach-O and ELF. The walk mirrors the POSIX
semantics (the spec-17 exec-closure walk — Rules L3/E2): parse the
referring image's import descriptors (the PE analogue of
DT_NEEDED/LC_LOAD_DYLIB), resolve each import against the mounts,
materialize what the mounts hold into the dlmap layout that mirrors the
memfs tree exactly, recurse with a visited set. The PE specializations
(locked):

- **No rpath exists on PE.** A bare import name resolves against the
  IMPORTING image's own in-image directory only (the `$ORIGIN`
  analogue — the mirrored layout guarantees vendored siblings sit next
  to their importer); an import name carrying a path separator resolves
  verbatim or referrer-relative, normalized. In-image candidate
  matching is VERBATIM (the memfs is case-sensitive); the windows
  loader's case-insensitivity governs the host binding only, never the
  in-image name match. A name the mounts do not
  hold at those candidates is a HOST import — the OS loader answers for
  it exactly as before (the POSIX unresolvable-name precedent). No
  cross-mount basename probing, ever — that is a heuristic, and
  heuristics are forbidden.
- **API-set contracts** (`api-ms-win-*`, `ext-ms-win-*`) are
  pseudo-modules the OS resolves internally — never files. The walk
  skips them unconditionally; they are host surface by construction.
- **The runtime's own ruby DLL** (the PE name flowed from the factory —
  the record's single owner — through the handoff env's
  `TEBAKO_RUNTIME_DLL`, exported by the driver at boot; spec 17 §2's
  table) is never materialized out of a payload
  image: the import binds to the already-loaded module by the OS's
  basename-reuse rule (which precedes any disk search), the exe-dir
  layout copy stays the only one on disk, and `rb_w32_check_imported`
  remains the ABI-line guard. The exclusion matches BARE import names
  only (case-insensitive); a separator-carrying name whose basename
  happens to match is not excluded — the proven-consumer shape. A
  payload copy would be a dead file
  written for no binding.
- **Delay-load imports** are not walked in phase W — no proven consumer
  (the evidence-driven rule). A delay-loaded vendored sibling is the
  one sub-case the binding below does not reach: an honest OS failure,
  named here until a leg proves the consumer.
- **Byte-exact parsing.** An RVA below `SizeOfHeaders` maps 1:1 — a
  header-resident import directory parses. A malformed image parses as
  DEPENDENCY-FREE (the truncated-header contract) and the OS loader
  answers for it honestly.

**The exec-cache lifecycle (windows).** A loaded DLL is LOCKED by the
OS for the process's lifetime, so the POSIX idiom — a per-process
`tebako-dl-<hex>` tmpdir reaped by the exit cleanup, legal there
because unlink-while-loaded is — cannot apply: the reaper's delete
fails on every still-loaded file. On windows, dlmap materializations
are therefore LEAVE-IN-PLACE and CONTENT-KEYED:
`<TEBAKO_EXEC_CACHE>/dlls/<image-key>/<P>` with the Rule-R1
`<image-key>` idiom, so a rebuilt image never reads a stale extraction
(Rule L3) and every process sharing the image shares one copy.
Write-once and verified by the Rule-R3 mechanics unchanged (stage,
hash in flight, the `<P>.tfs-digest` record renamed before the content,
per-boot rehash on reuse, a named 70 on mismatch); the driver-side
alias extraction and the load-time shim compute the path through the
SAME tfs entry (one path authority — invariant 10). Never unlinked at
load, never unlinked at exit: reclamation is the store's cache
maintenance (spec 05 §4's `tebako cache prune`), which reaps an
`<image-key>` no store entry resolves to and SKIPS a file the OS still
has locked — the lock is the liveness proof; a locked survivor is
retried on the next pass, never force-deleted, never an error surfaced
to a run.

**The binding: absolute-path rewrite + `LOAD_WITH_ALTERED_SEARCH_PATH`;
`AddDllDirectory` is rejected.** A covered load call receives the
absolute materialized path and carries `LOAD_WITH_ALTERED_SEARCH_PATH`,
so the module's own directory leads the OS's search for its import
tree — the PE analogue of `$ORIGIN`/`@loader_path`, and exactly what
the mirrored closure layout feeds. `AddDllDirectory` was the
alternative and is rejected on three grounds: it mutates PROCESS-GLOBAL
loader state, so a payload's vendored directory would shadow host DLLs
for every later bare-name load by any consumer — the silent-shadow
class (invariant 9) — and two payloads' directories would race on
insertion order; it takes effect only for loads carrying
`LOAD_LIBRARY_SEARCH_*` flags (or a `SetDefaultDllDirectories`
global), so the raw-`LoadLibrary` surface it was meant to rescue would
not consult it anyway; and its cookies demand a removal discipline the
OS's own file locks make unreliable. The rewrite's known OS rules are
stated, not hidden: an import basename matching an already-loaded
module binds to the loaded copy (module reuse precedes any disk
search), and KnownDLLs always bind to System32 — both are the OS's
documented precedence, host-by-default working as intended. The
transitive binding (a vendored sibling two levels deep) is pinned
empirically on the W2 dogfood, matching how §3.1's matrix was pinned.

**The bare-name rule (the fiddle-vs-ffi fork, settled).** A loader call
presenting a BARE name — no path separator, no drive qualifier
(`ffi_lib 'user32'`, `Fiddle.dlopen 'foo'`) — means HOST by default,
always: the name passes to the OS loader untouched — no VFS probe, no
extension completion, no search-order trickery. Bare names are
overwhelmingly the host-FFI idiom (system libraries); a bare name
carries no mount information, so any VFS reading of it would be a guess
(MECE forbids); and probing the VFS first would let a payload's
vendored `zlib1.dll` silently shadow the host's for every consumer (the
silent-fallback class). The ONE exception is declared: a bare name
matching a co-mounted image's `library_aliases:` entry EXACTLY
(verbatim, case-insensitive — the windows loader's own comparison;
`foo` does not match `foo.dll`) means payload-vendored. The
declaration — not a heuristic — is what makes the decision decidable at
all. Grammar and validation: spec 03 §2.5 (additive; old readers ignore
the key); the ambiguity rules (a duplicate within one image is a named
manifest error; two co-mounted images declaring one name is a named
boot error 65) are the spec 17 §2 slug precedent — never a silent
winner. Rule L1 itself is unchanged: aliases are a NAME-routing rule
for path-less calls, applied before L1's check runs; a rewritten alias
becomes an absolute host path whose load then follows the binding
above — it is not an L1 passthrough. Mechanically:

- On the COVERED surface (dln, fiddle) the shim checks the alias union
  first through exactly one c_api entry, `tebako_fs_dlalias2file`: it
  applies the bare-name grammar itself (spec 03 §2.5 SSOT), matches the
  union verbatim case-insensitive (no extension completion), and
  answers the alias's registered boot-materialized absolute path; a
  match rewrites the call to that path and loads it with the binding
  above. `NULL`+`ENOENT` (a path-carrying or undeclared name) passes
  the name to the host untouched; any other errno raises the §5
  verdict. The union reaches the TFS at BOOT, never by env var: the
  driver registers every co-mounted image's (name → materialized path)
  pairs into the tfs context — plain strings, so tfs stays tpkg-free.
- On the RAW surface (ffi, self-loads) nothing is intercepted: the
  driver materializes every declared alias at boot — the class-R pass,
  the same write-once/digest/per-boot-rehash mechanics — and PREPENDS
  the materialized directories to the process `PATH` (the §3.2 wiring's
  library form; EVERY co-mounted image contributes — the env image and
  the app payload included, unlike bin dirs — because any consumer in
  the process may
  present the name; the lead order is locked: launcher dir → §3.2 bin
  dirs → alias dirs → the inherited `PATH`). The OS's own standard
  search order then resolves
  the declared name for any caller, interception-free and
  per-gem-code-free. The OS's precedence is stated honestly: the alias
  guarantees AVAILABILITY on the search path, not precedence over the
  OS's leading dirs — a declared name colliding with a System32 DLL
  binds the host copy by OS rule, an aliasing mistake the declaration
  surface cannot fix and the record-mode journal makes visible.
- Discovery rides spec 23 §8's record-mode idiom: the patched load path
  JOURNALS every bare-name verdict (`event=lib-load name=<n>
  verdict=host|alias`), emitted by the c_api side where the verdict is
  made, so the author learns the exact spelling to
  declare from the journal instead of guessing. The needs-generator
  ignores these lines (they are not needs).

**Named errors and exit codes** (spec 06 §4 / spec 17 §4, unchanged
taxonomy). Loader-side (boot): an alias whose `path` the image does not
hold, or not a regular file, is a named 65 (the manifest lied); a name
carrying a path separator is the same named 65 at parse; the
cross-image ambiguity is a named 65 at boot; a tampered materialized
copy is a named 70 with the Rule-R3 remedy; a host IO failure writing
the cache is a named 74. The TFS closure walk itself reports through the
errno channel only — these named codes are emitted by their owning
layers (the tpkg parse, the driver boot, the Rule-R3 materialize path);
there is no parallel taxonomy inside tfs. Runtime-side (the covered load path): the
interpreter's own `LoadError` carries the tebako verdict line (§5) —
the v1-era bare `goto failed` is retired. Raw surface (post-amendment):
a PATH-carrying raw call is intercepted by the IAT rebind — a covered
miss returns NULL with a named `SetLastError` (5 / 8 / 1117 / 1359) the
caller (ffi, fiddle, a self-loading extension) surfaces through its own
error channel, never a tebako-authored message. A BARE-name raw call
stays the OS's own honest error (126 / `0x8007007E`), never
intercepted, the remedy named in the table below — declare the alias.

**The raw-`LoadLibrary` edge, per case** (the phase-W target; the W1
record above is the as-shipped baseline):

| Case | Verdict |
|---|---|
| `dln_load` of a VFS-resident extension (the `require` path) | **covered** — materialize + the PE closure walk + the binding above |
| host path uncovered by any mount | **covered** — passthrough untouched |
| covered but not held (a host path under a covering mount) | **covered** — ENOENT → host passthrough (the W1 precedent) |
| extension importing the ruby DLL | **covered** — the exe-dir layout + the OS basename-reuse rule + `rb_w32_check_imported`'s named ABI error; the walk's exclusion above keeps a vendored copy off disk |
| extension importing OTHER VFS-resident DLLs (sibling vendor imports) | **covered (W2)** — the PE closure walk materializes the siblings next to their importer; `LOAD_WITH_ALTERED_SEARCH_PATH` binds them. Closes the W1 two-layer gap |
| fiddle `Fiddle.dlopen` of a VFS path or a declared alias | **covered (W2)** — the dln-load IAT rebind (the §2.1 amendment) routes fiddle's raw `LoadLibrary` macro through Rule-L1; no `fiddle.h` change was needed |
| ffi `FFI::DynamicLibrary.open` / a C-extension self-load of a DECLARED alias's bare name | **covered by declaration (W2)** — the boot-extracted copy resolves through the OS's own `PATH` search; zero interception, zero per-gem code |
| ffi / self-load of an UNDECLARED bare name | **host by default** — the OS answers for it; the rule's whole point |
| ffi / self-load of a VFS PATH (any separator) | **covered (W2 — amended 2026-08-18)** — the dln-load IAT rebind (the §2.1 amendment) routes the raw `LoadLibrary`/`LoadLibraryExA` through Rule-L1: materialize-then-load with the PE closure walk and the altered-search binding; a covered dlmap failure returns NULL with a named `SetLastError` the caller surfaces through its own error channel. Never a silent success |
| a delay-loaded vendored sibling | **documented gap (phase W)** — the walk skips delay-load imports (no proven consumer); an honest OS failure |
| failed materialization on the covered path (a directory, a jail-denied passthrough) | **named error** — `LoadError` carrying the §5 verdict line plus the OS error text; the v1-era `goto failed` wart is retired |

**Rule L3.** Materialization reuses the spec-17 exec-closure walk
(Mach-O/ELF dependency closure, content-keyed cache dir, write-once —
PE's import directory joins as the third parsed format on windows,
§2.1). The cache is per runtime image sha — a rebuilt runtime never
reads a stale extraction. Materialization failures are named errors
naming the library and the mount that served it; never a partial load.

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
| **windows** | — | the §3.2 host tier materializes the target through the exec cache (whole-tree for a home-layout mount, the file itself otherwise) and the host copy execs — no injection exists on the platform, so the child runs plain-host against its declared tree; **argument tokens naming embedded paths are bridged to their host twins by the interpreter's spawn hook (§3.2's argv bridge)** | **works through the §3.2 host tier's `PATH` lead** — `CreateProcess`'s own PATH search finds the materialized host copy (a home-layout tool's tree came with it by declaration); nothing re-arms, there is no injection var; argument tokens naming embedded paths ride the same argv bridge |

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

**Rule E4 (the data-file sibling rule).** A covered materialization
target that parses as NO load module (no Mach-O/ELF/PE magic — a
genuine data file: a schema, a keystore, a native tool's resource) has
no dependency closure to walk, but its consumer can address sibling
resources RELATIVE to the materialized host path — a JVM resolving a
RelaxNG `<include href="other.rng">` against the schema's directory
(the packed-mn#251 jing failure: the bridged schema landed alone and
its same-dir include chain never did) — on the HOST, where the VFS
view does not reach (§3.1's named boundary). The parent in-image
directory's FILES therefore materialize alongside the target into the
same mirrored host tree: non-recursive (subdirectories never sweep —
no proven consumer; the §3.2 whole-tree answer already exists for
that shape), capped (512 files / 64 MiB per sweep, each cap noted
under `TEBAKO_DEBUG_TFS`), and never fatal to the target's own
materialization (a sibling's extraction failure is noted, not raised
— the consumer's own error stands). A file WITH load-module magic
that fails to parse is a malformed module, not a data file: it rides
alone exactly as before this rule — its consumer is the OS loader,
which raises its own honest error (the exec-side analogue of §2.1's
truncated-header contract).

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
authors who compute paths themselves. On windows the same `PATH` lead
carries the LIBRARY bare-name surface: the driver additionally prepends
every co-mounted image's boot-materialized library-alias directories
(§2.1's bare-name rule), so the OS loader's own search resolves a
declared bare name for any caller in the process — interception-free.

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
dir leads `PATH`). The tier's materialization routes through the SAME
`exec_materialize` home-tree decision the spawn surface and the windows
host tier use: a home-annotated mount's executable execs from its
materialized whole-tree home, never the closure mirror — the mirror
strands a self-locating prefix (a materialized JVM's `java.home` without
`conf/` dies at JCE boot listing `conf/security/policy`; the packed-mn
ISO leg, 2026-08-28).

**The windows host tier** (armed unconditionally on windows — there is
no preload shim to deliver and no injection var to re-arm, so the tier
IS the delivery): the driver materializes each dependency's declared
executables through the exec cache via the same `exec_materialize`
routing the spawn surface uses — a mount whose in-image manifest
carries the home annotation (`identity.annotations.java_home`, the
SSOT for "my home tree must run beside me") extracts WHOLE once per
boot and answers the executable's in-tree host twin (a JVM's
`lib/modules`, `jvm.dll`, `jmods/` never ride a linked-library
closure, so only the tree answer boots a working java); any other
mount answers the file itself, whose DLL bare-name loads then resolve
through §2.1's alias dirs already on `PATH`. Each materialized
executable's PARENT DIR leads the §3.2 `PATH` prepend, in triple order
(the locked lead stays byte-stable: windows host bins → §3.2 bin dirs
→ §2.1 alias dirs → inherited). The declared path carries the real
suffix on the wire (the feedstock writes `/bin/java.exe` — the
manifest names the in-image byte exactly), so `CreateProcess`'s own
PATH search finds it; no wrapper script exists because nothing needs
re-arming. Triple order wins on a basename collision; a declared
executable that cannot be materialized is the image lying (a named
65), never a skipped entry. **The named boundary:** a materialized
windows child runs plain-host — no VFS re-entry exists on the
platform. Its own home tree is present BY DECLARATION; host resources
beyond that (a document tree, a fonts dir) reach it only through spec
23's declared host binds, never through the VFS.

**The windows argv bridge** (the boundary's other half, delivered in
the interpreter's spawn hook — tamatebako/ruby's
`process_c_tebako_spawn*.patch`, the `_WIN32` branch; ruby#102/#103,
locked 2026-08-27): a host-tier child runs plain-host, so it cannot see
VFS paths handed to it as ARGUMENTS (ruby-jing / mn2pdf:
`java -jar <payload-resident jar>` — the jar string resolves through
ruby's VFS and the host `java.exe` cannot read it; the packed-mn
windows acceptance died on exactly that, `Unable to access jarfile
A:/__tfs__/…`). The hook therefore bridges parent-side, before the
spawn: any argument token naming an embedded path
(`tebako_path_is_embedded`) is materialized through
`tebako_fs_exec_materialize` — the exec-target hook's own primitive, so
a home-layout mount extracts whole-tree and any other mount answers the
file from the dlmap cache — and the token is rewritten to the host
twin. Array form rebuilds `invoke.cmd.argv_buf` (NUL-separated) and the
`argv_str` pointer vector exactly as `rb_exec_fillarg` built them;
shell form tokenizes the command line on unquoted blanks honoring
double-quoted regions and rewrites only when some token bridges, with
separators preserved byte-for-byte. One wrapping layer of double quotes
is stripped for the probe; a bridged token is always re-quoted (a
materialized path may carry spaces). Non-embedded tokens pass through
byte-identical; a token naming a covered path the mounts do not hold
keeps its spelling and the child answers honestly — the exec-target
hook's own pass-through discipline, never a silent rewrite. **Patch
twinning (the silent-shadowing trap, ruby#103):** tamatebako/ruby's
PatchSelection drops a base patch when an `_msys` sibling targets the
same file, so every `process.c` spawn patch MUST ship an `_msys` twin
while any `_msys` patch family touches `process.c` (today:
`process_c_clock_guard_msys` on 3.2/3.3/3.4) — a missing twin compiles
a windows runtime WITHOUT the bridge and no build step fails; the
packed-mn acceptance is the tripwire and a selection lint gate is
tracked debt.

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

Amended 2026-08-30 (tebako#502): the acceptance shape above predates
the wrapper-runtime pattern. A home-annotated runtime payload
materializes the interpreter's home INTO the per-process dl tree (§3's
exec cache), and the booted JVM's own reads then name
`…/tebako-dl-<hex>/tebako-home-<n>/…` — process-internal storage that
never reaches the gate (spec 08 §3's exemption). A wrapper runtime
therefore composes with a `deny` jail with NO tool-tree grant at all;
the authored `<jre>:ro` ingredient remains the spelling for an
operator-INSTALLED host JRE (the non-wrapper pattern this section was
locked on).

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
path through the documented cache-root convention (§6). The one
env-surface exception is the cert convention (`ssl/cert.pem`): the
driver — the single owner of where the materialized copy landed —
exports `SSL_CERT_FILE` at boot per spec 17 §2's table (an image-side
default pointing at the in-VFS spelling is unreadable by libcrypto's
native CRT IO on the store-less boot, the #437 failure).

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

On windows the verdict channel is the `LoadError` message itself: the
patched dln route (§2.1) raises carrying the tebako context line
— the library, the mount, the materialization verdict — with the OS's
own error text appended (`FormatMessage`), never a bare 126. This is
the phase-W fix for the v1-era `goto failed` wart the W1 record named,
and the exact analogue of the POSIX dlerror contract. Raw
`LoadLibrary` callers rebound through the IAT shim (the §2.1 amendment)
surface a covered miss as the shim's named `SetLastError` code through
their own error channel; bare-name callers (the uncovered surface) get
the OS's own honest error — never an intercepted one, never a silent
success.

## 6. The documented interface (the stable surface)

Payload authors and runtime factories may rely on, forever:

- **Path rule.** A native library or executable inside a mounted image
  loads/execs exactly as if it were on disk, from any caller in the
  process. No gem- or payload-specific action is ever required.
- **The exec cache root.** `TEBAKO_EXEC_CACHE` (read-only to payloads):
  the directory where materialized binaries/libraries live. The
  lifecycle is platform-split (§2.1): on POSIX a materialization lives
  in a per-process tmpdir reaped at exit (unlink-while-loaded is legal
  there); on windows a loaded DLL is OS-locked, so materializations are
  leave-in-place, content-keyed per image, and reaped by the store's
  cache maintenance — never unlinked under a running loader. Its
  content is an implementation detail; its existence and per-image-sha
  segregation are contractual.
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
  works unmodified past the SIP strip (§3.1) exactly as on ELF. On
  windows the boot-materialized library-alias directories complete the
  same lead (§2.1) — every co-mounted image contributing, the env image
  and the app payload included; the locked order is launcher dir →
  dependency bin dirs → alias dirs → the inherited `PATH`.

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
- Landing order: Class L POSIX → Class E → Class R → windows L (§2.1 —
  the phase-W design: the PE closure walk, the dln-load IAT rebind, the
  alias bare-name rule) → windows exec (§3.2's host tier — no win32
  spawn hook; the driver materializes by declaration and `PATH` carries
  the answer). Each lands behind its dogfood
  proof; a failed proof keeps the adapter.

## 8. Acceptance

- A payload whose gems load native libraries through ffi, fiddle, AND
  the interpreter's own ext loader works on a runtime built with zero
  per-gem adapters (the fontist payload is exactly this).
- A payload binary spawning another payload's binary by shell string
  works with no adapter (the metanorma → openjdk java case).
- Windows argv bridge (§3.2): on the msys legs, a payload spawning a
  host-tier dependency with an EMBEDDED-path argument works in both
  forms with no adapter — metanorma → jing (`java -jar <payload jar>`,
  array form) and mn2pdf's shell form; the packed-mn windows acceptance
  (a real document compile to PDF) is the gate, and an un-bridged
  runtime fails it with the host's own `Unable to access jarfile`,
  never a tebako error.
- The runtime gem's require maps are empty; its remaining content is
  the memfs cache helpers.
- The v2 dogfood suite is green with the maps empty on every
  published platform.
- Windows Class L (phase W2's proof; the design is §2.1 — PLANNED): on
  the msys legs, a payload whose gems load native libraries through the
  interpreter's ext loader AND fiddle works with zero per-gem adapters;
  a DECLARED alias's bare-name load binds the boot-materialized copy
  for a raw `LoadLibrary` caller (the ffi form) through the OS's own
  search; a vendored sibling import two levels deep binds through the
  mirrored closure + `LOAD_WITH_ALTERED_SEARCH_PATH`; and a failed VFS
  materialization surfaces the §5 verdict line in the raised
  `LoadError`, never the v1-era bare `goto failed`.
