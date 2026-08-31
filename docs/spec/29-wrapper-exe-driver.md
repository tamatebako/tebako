# Spec 29 — The wrapper-exe driver pattern (repacked runtimes)

**Status: PLANNED (drafted 2026-08-30).** Amends spec 17 §6 (the driver
executes in two patterns) and spec 03 §2.2 (the runtime env image's
`layout:` block gains the additive `interpreter` and `visibility` keys,
schema_minor — the same class as `mount_root_override`). No wire-format
change; no trailer change; no registry change. First instance: the
openjdk runtime promotion (ecosystem TODO.java/04); the pattern is the
standing answer for every runtime tebako does not compile from source.

## 0. The problem

The spec-17 driver is linked INTO the interpreter exe at factory time
(the v2 link unit). That requires owning the interpreter build. Repacked
runtimes ship upstream bytes (Temurin openjdk, truffleruby prebuilts) —
the driver cannot be linked into bytes tebako did not compile. Without a
specified wrapper pattern every repacked runtime would invent its own
handoff. This spec is the ONE home of the pattern; specs 17/19/22
reference it, nothing re-authors it.

## 1. The wrapper-exe model

A runtime whose exe artifact is tebako-owned: a small Rust binary
embedding `crates/tebako-driver` in STANDALONE mode. The store pair is
unchanged (spec 05 §3): `tebako-runtime-<ver>-<lang>-<triplet>[.exe]` is
the wrapper (0755); the env image holds the real interpreter (`bin/java`,
the truffleruby home, …).

The wrapper receives the EXACT spec-17 wire:

```
<runtime> --tebako-image <self|image-path>:<slot|->:<mount> ...
          [--tebako-trace <host-path>]
          --tebako-entry <argv0> <user args...>
```

plus the spec-17 §2 environment (`TEBAKO_RUNTIME_IMAGE`, `TEBAKO_JAIL*`,
`TEBAKO_TRACE`, `TEBAKO_EXEC_CACHE`, the mount/PATH rows). It performs
the mounts, the jail install, and the argv rewrite identically to the
linked driver, then executes the real interpreter declared by the env
image (§2). The launcher ABI and the loader NEVER learn which pattern a
runtime uses — wire-identical (spec 00 invariant: no second grammar).
The release manifest declares `contract_version: 2` exactly as a linked
runtime does (spec 17 §6).

**Process semantics.** On POSIX the wrapper execs — the interpreter
REPLACES the wrapper process (no extra process, signals behave
naturally). On windows (no exec) the wrapper spawns the interpreter as a
child, waits, and propagates the child's exit code verbatim; console
control events reach the child through the shared console. The wrapper
never swallows, remaps, or invents exit codes (§4).

**argv composition.** The entry is resolved per spec 17 §1 (against the
first `--tebako-image` mount, or the runtime root with no image specs);
the wrapper then composes

```
[<interpreter path (§2)>, <args_default…>, <entry resolved>, <user args…>]
```

— the spec-17 §1 rewrite with the interpreter standing at index 0 and
the entrypoint's declared `args_default` (spec 03 §2.2) composed
between, so a jar entry `{path: /app/jing.jar, args_default: ["-jar"]}`
execs `java -jar <entry> <user args…>`. Entrypoints whose interpreter
takes the entry positionally declare an empty `args_default`. There is
no wrapper-specific argv grammar. The linked driver composes
identically (tebako#503): args_default's single owner is the runtime
side — loaders (the shim, the bootstrap) never carry it.

**The bare-name entry (spec 30 §2).** When the entry token is a bare
non-`self` NAME (spec 17 §1's keyword form), the composition changes
shape: the env image's own `provides.entrypoints` declares the program
(a JVM tool, a helper exe); the wrapper execs the DECLARED program
directly — materialized per §3 when the kernel needs a host path —
with the declared `args_default` composed and the interpreter token
dropped entirely (no bridge entry, no script position). The spawned
process IS the declared program:
`[<program>, <args_default…>, <user args…>]`.

## 2. The interpreter-path declaration

The env image's in-image manifest declares the interpreter under the
`layout:` block (spec 03 §2.2; schema_minor, additive):

```yaml
layout:
  interpreter: /bin/java          # in-image path, the spec-17 §1 uniform namespace
  visibility: preload             # OPTIONAL; absent = the locked default order (§3)
  mount_root_override: true       # unchanged
```

- The path is in-image (POSIX-absolute on every platform, drive-
  qualified onto the runtime root by the driver per spec 17 §1) and
  resolves against the env image's mount. A `layout:` block without
  `interpreter` is the linked pattern; a wrapper-pattern runtime image
  MUST declare it — absence under the wrapper pattern is a named boot
  error (exit 65) naming the key. A declared path that does not resolve
  inside the env mount is a named boot error (65) naming the path and
  the mount.
- Nothing in the wrapper binary hardcodes an interpreter path; the
  wrapper crate carries no per-runtime knowledge (MECE: the path's owner
  is the runtime's own manifest; the factory authors it once, spec 13).

## 3. Kernel visibility — how the interpreter and its children see the bytes

`execve(2)` and `CreateProcessW` resolve HOST paths: an in-process VFS
mount is invisible to the kernel. The wrapper therefore cannot exec the
interpreter out of the mount directly; the interpreter's own reads of
its home tree (jmods, `libjvm`, gem homes) need a defined answer; and
the interpreter's CHILDREN never see in-process mounts at all (the
PROGRESS/19 spawn class). The mechanisms are spec 07 §8's locked tiers,
applied to the runtime's own interpreter — **interposition-first, never
FUSE** (spec 07 §8's locked law stands: FUSE stays the `tfs mount`
human convenience, never on the exec path). A runtime declares ONE
mechanism in its env-image manifest (`layout.visibility`); the boot
honors it or fails closed — never a silent fallback (invariant 9):

| mechanism (spec 07 §8 tier) | what the wrapper does | platform reach / default |
|---|---|---|
| `preload` (tier 1 — THE POSIX DEFAULT) | materialize only the interpreter exe's load closure (spec 22 §2.1's walk); exec it with `LD_PRELOAD`/`DYLD_INSERT_LIBRARIES` armed on libtfs-preload + `TEBAKO_TFS_MOUNTS` (spec 22 §3) so the unmodified interpreter's libc IO resolves through the VFS — the env image stays MOUNTED, never extracted | linux + macOS dynamic interpreters (the JVM's libjava/libjli/libzip are in spec 07 §8's shipped coverage list) |
| `seccomp-notify` (tier 2a) | the wrapper-as-executor installs the `SECCOMP_RET_USER_NOTIF` filter at spawn and serves the static interpreter's file syscalls from the mounts, enforcing the same `host_policy` | linux only; statically-linked interpreters |
| `exec-cache` (tier 2b — THE windows DEFAULT, universal fallback) | materialize the interpreter home tree from the mounted env image into the content-addressed exec cache (spec 22 §6), then spawn the host copy (a JVM needs no VFS once its home is materialized) | universal; the only windows answer (no interposition API) and the POSIX answer for statics without tier 2a |

- The store artifact stays pristine in every mechanism: the exec cache
  is DERIVED, content-keyed, disposable (the spec 22 §6 exception —
  execution materialization is not extraction of the store image).
- Absent a declaration, the locked default order applies — tier 1 for
  dynamic interpreters on POSIX, tier 2b on windows and for statics
  without tier 2a — journaled at boot (`event=visibility mechanism=…
  image=…`). A DECLARED mechanism unusable on the host (`preload` on
  windows or under a SIP-stripped context, `seccomp-notify` off linux)
  is a named boot error (65) naming mechanism + fact — the factory
  validates the declaration against the runtime's platform list at
  press.
- The user-side override rides the composition/config surfaces (spec 23
  §3's runtime row gains the optional `visibility:` key, validated
  against the manifest's declared support — a narrowing only, never a
  mechanism the runtime wasn't built for).
- Grandchildren: under `exec-cache` the interpreter's children are
  ordinary host processes; VFS-resident tools they spawn reach them
  through the spec 17 §2 PATH composition (launcher dir → dependency bin
  dirs → alias dirs) plus the declarative materialization that shipped
  for ruby-windows (PROGRESS/19's option (b)). Under `preload`/
  `seccomp-notify`, descendants re-enter the VFS per spec 22 §3 / the
  tier-2a env propagation — the wrapper arms the machinery identically
  to the linked driver.

## 4. Failure semantics

- Loader-side failures keep the loader's named codes (65–76) exactly as
  spec 17 §4: mount failure → unmount everything + named error, never a
  partial mount; jail parse/bind failures → 73; a malformed/missing
  interpreter declaration, an unresolvable interpreter path, or an
  unusable declared visibility → 65 naming the key/path/mechanism.
- Runtime-side failures are the interpreter's own codes, unmodified:
  POSIX exec makes this literal; the windows wrapper exits with the
  child's exit code and never manufactures codes of its own. A spawn
  FAILURE — CreateProcess rejecting the materialized exe — is
  loader-side, exit 65.
- `--tebako-extract` keeps spec 17 §4's semantics; the embedded driver
  answers it (dump mounted images to disk, exit 0) BEFORE the
  interpreter is executed.

## 5. Size and discipline

- The wrapper is NOT the bootstrap: it does not fall under the < 3 MB
  loader gate (spec 00 invariant 2). A budget is declared and measured
  in CI: single-digit MB per platform artifact, reported on every
  release; a regression past the budget fails the release build. The
  wrapper embeds the driver plus the TFS backends; the release ships it
  per-platform like the other tools — static-musl and the gnu-floor
  build on linux targets, platform-native elsewhere (the openjdk
  promotion, TODO.java/04, consumes the per-platform assets) — the
  audience rule is unchanged (nothing of ours compiles on a user's
  machine).
- Rust discipline per spec 14: `#![forbid(unsafe_code)]` outside the FFI
  boundary modules; named errors on every mount/exec/materialize path
  (no unwraps there); the bootstrap size profile (`opt-level="z"`, fat
  LTO, one codegen unit, `panic="abort"`, stripped symbols).

## 6. Relationship to spec 17

Spec 17 §6 gains: the driver executes in one of two patterns — LINKED
into the interpreter exe (factory-built runtimes, the reference form) or
as the WRAPPER exe defined here (repacked runtimes); both declare the
same contract (`contract_version: 2`), and the launcher ABI never learns
which pattern a runtime uses.

## 7. Non-goals

- No trailer change; no registry change; no launcher-ABI change.
- No new L1 grammar beyond the additive `layout.interpreter` +
  `layout.visibility` keys (schema_minor — the `mount_root_override`
  class).
- FUSE on the exec path — spec 07 §8's locked law stands
  (interposition-first; FUSE stays the `tfs mount` human convenience).
- No per-runtime code in the wrapper crate: a runtime's particulars
  live in its env-image manifest (interpreter path, visibility,
  entrypoint `args_default`).
- A runtime whose process owner is ANOTHER runtime (jruby on java,
  truffleruby-jvm on graalvm) is NOT this pattern: it is
  runtime-on-runtime composition (ecosystem TODO.jruby/01), which rides
  spec 03 DEPENDS and reuses this wrapper for the OWNER runtime.
