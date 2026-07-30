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
  tpkg slot `<slot>`; `<image-path>` = read from a file. **Slot tokens:**
  a bare file (no trailer) mounts whole — slot `0` ≡ `-`; a packaged
  file mounts the numeric slot's trailer-described region, and `-` on a
  packaged file is a named error. A runtime-role slot
  (`format_id = TPKG_FORMAT_RUNTIME`) is never mounted. (The image-era
  `TEBAKO_RUNTIME_IMAGE` case is a bare path mounted whole — the `-`
  semantics without a triple.)
- **Mount order:** the env image first, then payload triples in argv
  order; the table is longest-prefix and nested mounts are legal; a
  duplicate mount point is a named error; any failure unmounts
  everything — never a partial mount.
- `--tebako-entry` separates loader args from user args; `argv0` is the
  entrypoint inside the mounted tree, resolved against the FIRST
  `--tebako-image` mount (the app payload) — or against the runtime root
  when no image spec is given. The driver verifies the entry's presence
  against the mounts the boot itself established (named error 65 when
  absent); an entry outside them belongs to the interpreter's own
  startup. A bare NAME (no `/` — e.g. `ruby`) is the interpreter
  keyword: the boot starts the interpreter itself with the user's args
  and drops the keyword (the deploy shims' re-entry form). With images
  but NO `--tebako-entry` at all, the boot mounts and starts the
  interpreter with its own args (the smoke form).
- The interpreter's own args are skipped (and `--tebako-extract` stays
  theirs); an unknown `--tebako-*` flag is a named error, never silently
  ignored.
- Everything before the first `--tebako-*` is the loader's; everything
  after `--tebako-entry` is the user's, verbatim. On success the driver
  replaces the process argv with
  `[<original argv0>, <entry resolved in the VFS>, <user args…>]` — the
  program name stays at index 0 so the interpreter parses its argv
  conventionally and takes the entry as its script.

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

## 6. Implementation and contract version

The reference implementation is `crates/tebako-driver` (Rust, staticlib
+ rlib) — the v1 C++ `tebako-main` driver is retired. Runtimes whose
driver implements this document's widened grammar (image-path triples,
bare-file slot tokens, env-image-first multi-mount, direct
`--tebako-entry` execution) declare **`contract_version` 2** in their
release manifest (spec 06 §6); the compiled-in constant
(`tebako_driver_contract_version()`) and the manifest must agree.
