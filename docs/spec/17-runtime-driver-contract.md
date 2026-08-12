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
  order; the table is longest-prefix and nested mounts are legal; any
  failure unmounts everything — never a partial mount.
- **The uniform VFS namespace (locked 2026-08-06):** declared mount
  points are POSIX absolute paths on every platform — in manifests, in
  trailer slot records, and on this wire. On windows the namespace
  presents on its own drive: the driver qualifies every declared mount
  `<mount>` onto the runtime root's drive (`<drive><mount>`) before any
  mount, union, or entry computation. The runtime root is a
  per-platform baked default owned by the runtime factory (ruby:
  `/__tfs__` on POSIX, `A:/t` on windows — short by owner decision,
  MAX_PATH headroom on every in-image path). An interpreter's C-level
  path expansion re-roots drive-relative paths (`/...`) onto the
  process cwd drive; only drive-qualified paths are stable across
  expansion, so qualifying is what keeps payload paths inside the VFS.
  The wire grammar therefore never carries a drive letter: `<mount>` is
  always the declared form, and a declared mount naming a drive is
  malformed.
- **Run-time root override (`TEBAKO_MOUNT_ROOT`, locked 2026-08-08):**
  the baked root is the default, never the only spelling. When
  `TEBAKO_MOUNT_ROOT` is set in the runtime's environment, the driver
  mounts the env image at that root instead and reports it from
  `tebako_mount_point` (the io-routing patches and the interpreter's
  rbconfig follow — era-2 rbconfig emits
  `ENV["TEBAKO_MOUNT_ROOT"] || <baked>`). The override is validated
  before any mount — an absolute path (`/…` or drive-qualified `X:/…`),
  no trailing slash, no `..` — a malformed value is exit 65 naming the
  variable. The override is then gated post-mount on the env image's
  layout grant (`mount_root_override: true`, layout schema_minor 1): an
  image without the grant predates the override era (its rbconfig is
  pinned to the baked root), so the driver refuses with exit 78 naming
  both the override and the image — never a boot whose load paths point
  at an unmounted root. Declared payload mounts qualify onto the
  override's drive exactly as they do the baked root's.
- **Mount modes (locked 2026-08-04):** every mount is `exclusive`
  (default) or `union`, declared per slot in the package manifest's
  `mounts:` block (spec 03 §6). An exclusive mount onto an occupied
  point is a named error (EEXIST). A union mount onto an occupied
  point merges the trees: directories combine, file conflicts resolve
  by the declared precedence (`after-env` — over the env image — or
  `after:<slot>`); union members are read-only and the union set
  (point + members + precedence) is journaled at boot. The driver
  reads the mode from the running package's OWN trailer (the `<self>`
  manifest block) — mount semantics never ride the argv grammar, so
  the launcher ABI is unchanged and drivers predating this section
  refuse a union package loudly (EEXIST), never silently. Payloads
  handed over without a package manifest (shim dispatch, bare images)
  are always exclusive.
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
| `TEBAKO_EXEC_CACHE` | spec 22 §6: the boot's exec-cache root — materialized binaries/libraries live under it for the process's lifetime; read-only to payloads |
| `TEBAKO_PRELOAD_SHIM` | spec 22 §3: the preload shim's in-VFS path, flowed from the env image's `preload_shim` layout grant — the interpreter's spawn hook reads it (never a hand-written copy); the driver additionally arms `LD_PRELOAD` (ELF) / `DYLD_INSERT_LIBRARIES` (macOS) with the materialized host copy |
| `TEBAKO_MOUNT_<SLUG>` | spec 22 §6 + v2-1/20: per co-mounted payload image, its physical mount point (drive-qualified on windows). SLUG is the mount's mechanical uppercase form: `/tools/inkscape` → `TEBAKO_MOUNT_TOOLS_INKSCAPE`; two mounts slugging alike is a named boot error (65). The root mount `/` exports nothing — `TEBAKO_MOUNT_ROOT` stays the mount-root override (§1) |

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
