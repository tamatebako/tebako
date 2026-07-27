# Spec 17 — The runtime driver contract (any language)

Normative contract between a tebako runtime (the interpreter binary a
runtime payload provides) and the tebako loader/dispatcher. Language-
agnostic: ruby is the first implementation; python/julia follow this
exact contract (roadmap 22's "add a language" playbook).

## 1. Invocation surface

```
<runtime> --tebako-image <self|image-path>:<slot|->:<mount> ...
          --tebako-entry <argv0> <user args...>
```

- `--tebako-image` triples mount payloads BEFORE the interpreter
  starts: `<self>` = read the image from the running executable's own
  tpkg slot `<slot>`; `<image-path>` = read from a file (the image-era
  `TEBAKO_RUNTIME_IMAGE` case uses slot `-` with a path).
- `--tebako-entry` separates loader args from user args; `argv0` is the
  entrypoint inside the mounted tree.
- Everything before the first `--tebako-*` is the loader's; everything
  after `--tebako-entry` is the user's, verbatim.

## 2. Environment

| var | meaning |
|-----|---------|
| `TEBAKO_RUNTIME_IMAGE` | image-era: absolute path of the runtime's own `.tfs` (driver mounts it instead of any embedded image) |
| `TEBAKO_TFS_MOUNTS` | `image:mount,…` — mounts to establish (preload-shim path) |
| `TEBAKO_JAIL` | jail policy env form (spec 08) — the driver/preload enforces it |
| `TEBAKO_JAIL_SOURCE` | audit label of the policy's origin (`manifest` / `user` / `manifest+user`, or the exporting surface) — journaled with every denial (spec 08 §2) |
| `TEBAKO_JAIL_JOURNAL` | explicit audit-journal path (default: `$TEBAKO_HOME/journal.log`) |

## 3. File IO semantics

The runtime's IO MUST route mounted paths through the TFS layer
(`tebako_fs_*` or an equivalent in-process VFS): read-only payload
images; host paths pass through subject to the jail policy; writes to
payload images fail EROFS. Two implementation patterns (spec 07 §8):
patched-interpreter io-routing (ruby) or the preload interposition shim
(unmodified dynamic binaries).

## 4. Exit codes

The runtime preserves the loader's named codes (65–74) when the failure
is loader-side; runtime-side failures use the interpreter's own codes.
`--tebako-extract` is the runtime-side option riding the user-arg
passthrough: dump mounted images to disk and exit 0.

## 5. Provenance

A runtime payload's manifest (spec 03) declares `provides` {engine,
version, abi_line, platform} + `built_from` — the dispatcher's
compatibility check consumes exactly these fields.
