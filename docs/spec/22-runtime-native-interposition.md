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
  mechanism the preload layer uses for exec.
- **Windows:** inside the interpreter's own dln path (the patched
  `dln.c`); C-extension self-loads via raw `LoadLibrary` are the
  documented edge (rare — evaluate per case, never silently).

**Rule L3.** Materialization reuses the spec-17 exec-closure walk
(Mach-O/ELF dependency closure, content-keyed cache dir, write-once).
The cache is per runtime image sha — a rebuilt runtime never reads a
stale extraction. Materialization failures are named errors naming the
library and the mount that served it; never a partial load.

**Rule L4.** The interposition is read-only with respect to the store:
it never installs, upgrades, or deletes anything (a run is a run).

## 3. Class E — exec interposition

**Rule E1.** The existing spawn hook (the interpreter's process-layer
patches) covers the array-form spawn of an absolute VFS path. This spec
EXTENDS coverage to the exec level: `execvp`, `execve`, `posix_spawn`.
A shell-string spawn (`/bin/sh -c "java -jar /vfs/x.jar"`) then works
unmodified — the shell parses and calls `execvp`, whose argv0 is
intercepted like any other.

**Rule E2.** Exec of a VFS-resident binary materializes the binary plus
its loader closure (the same exec cache as Class L) and execs the real
path with the original argv/env. Exec of a host path passes through.

**Rule E3.** A VFS binary that itself spawns children re-enters the
same interposition (the preload-injection inheritance already proven
for the spawn hook). Children of materialized binaries keep the VFS
view — never a silent host fallback.

## 4. Class R — declarative boot materialization

**Rule R1.** An image manifest MAY declare `materialize: [paths]`
(spec 03 §additive; `schema_minor` bump per the spec-03 rules — old
readers ignore the key, new readers enforce). The driver extracts the
listed paths to the runtime cache at boot, before the interpreter
handoff.

**Rule R2.** The canonical consumer pattern is an image-OWNED default:
the image that ships a resource also ships the configuration pointing
at its materialized location (the runtime env image's own OpenSSL cert
default is the first entry). Payloads needing host-visible resources
declare them in their own manifests; a consumer reads the materialized
path through the documented cache-root convention (§6).

**Rule R3.** Materialization is whole-file, read-only, and verified
against the image's content hashes. A listed path absent from the
image is a named boot error (the manifest lied), never a skipped entry.

## 5. Error model

All three classes keep the established exit-code discipline (spec 06):
the loader-side failures keep the loader's named codes (65–78); a
runtime-side load/exec failure surfaces as the interpreter's own error
with the tebako context line (the library/binary path, the mount, the
materialization verdict). No silent fallbacks: if a VFS load/exec/
materialization cannot complete, the named error is the outcome —
never a host-path shadow.

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
  `materialize:` entry `P` lands at `<exec-cache>/resources/<image-sha>/<P>`.
  Images that ship resources document their consumption path in their
  own manifests (spec 03 annotations).
- **The discovery surface.** `TEBAKO_MOUNT_<SLUG>` per dependency mount
  (spec 17 §2's env table; v2-1/20) — the portable way to reference a
  dependency payload's files, windows included.

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
