# Spec 07 — Shims and dispatch

Normative specification of executable registration and version
management. Status: PARTIAL — the dispatcher and version manager
(§2–§4) ship as `crates/tebako-shim` in tebako-rs (roadmap 08; retires
`mnenv`). Still PLANNED: remote registry fetch (roadmap 07; v1 resolves
`file://` registry refs only), `tebako use` writing the user default
(v1: author `~/.tebako/config.yaml` directly), jail application
(spec 08), and a runtime registry (v1 downloads resolve through the
`runtimes:` preference in `config.yaml` as the exact ref).

## 0. v1 concrete choices (normative where the sections above were open)

- **Project pin file** `.tebako-tools.yaml` is a FLAT YAML mapping of
  command name → version; a nearer file that does not pin the command
  does not shadow a farther one that does.
- **`~/.tebako/config.yaml` keys:** `defaults:` (command → version),
  `registries:` (spec 04 refs), `runtimes:` (engine → `{version,
  tebako}` runtime preference). The shim never writes this file.
- **Disabled state** is shim-managed state, not authored config:
  `~/.tebako/shims/.disabled.yaml` (command → `[versions] | [all]`).
- **Installed payload record** (the dispatcher-visible manifest mirror,
  spec 03 §4 tier 3 rationale): `payloads/<name>/<version>.tfs`,
  `<version>.tfs.sha256`, `<version>.manifest.yaml`.
- **Zero-runtime dispatch** (spec 03 §2.2): the payload image itself is
  the program — the self-launching-image contract; dependency mounts
  still ride the ABI v1 argv shape minus the runtime prefix.
- **Registry payload mounts** are bare images: slot `0` (whole image),
  mount point `/` of the jail namespace.

## 1. The model

Tebako manages shims for **every executable every installed payload
provides** (spec 03 `entrypoints`). One payload may carry MULTIPLE
executables — each becomes a registered command. Four artifacts, four
jobs:

- **payload** — a signed `.tfs` image (versioned, runtime-independent).
- **runtime** — a signed runtime payload (versioned, cached,
  machine-shared).
- **registry** — a developer-hosted `tpkg-registry.yaml` (spec 04 §2).
- **dispatcher** (`tebako-shim`, a tiny static Rust binary in tebako-rs) —
  the thing on PATH that picks version + runtime per invocation and hands
  off.

## 2. The dispatch chain (per invocation of `~/.tebako/shims/<tool>`)

0. **argv0 is the selector.** One tebako-shim binary, linked per command
   name; it maps name → entrypoint in the payload's manifest.
   **Multi-command suites:** one package with N entrypoints installs N
   shims — each dispatches to its own image AND ITS OWN runtime
   requirement; two commands in one package may run different runtime
   versions simultaneously.
1. **Payload VERSION resolution** (first match wins):
   `TEBAKO_<TOOL>_VERSION` env → nearest `.tebako-tools.yaml` walking up
   from cwd (per-project pinning) → user default (`tebako use
   <tool>@<version>`) → registry's `default`.
2. **RUNTIME resolution:** the entrypoint's `runtime_requirement` →
   newest COMPATIBLE cached runtime (no download) → else download newest
   compatible (spec 05 §5). **Swapping runtimes needs no payload change**
   — the payload is immutable; only the dispatch-time choice changes
   (`tebako use --runtime ruby@3.4.2`, or a per-project pin).
3. **Hand-off:** mount payload + ZERO OR MORE runtime payloads (native
   entrypoints need none — spec 03) + declared dependency mounts
   (spec 03 §2.3), apply the jail view (spec 08), exec the entrypoint.
   Signed payloads are verified at install time, not per run.

## 3. Shell integration (no per-shell magic)

- ONE directory on PATH: `~/.tebako/shims`. One-time setup;
  `tebako shim install-shell [--shell bash|zsh|fish|csh]` inserts a
  managed BEGIN/END block into the right startup file
  (`.profile`/`.bash_profile`/`.bashrc`/`.zshrc`/`.cshrc`) prepending the
  shim dir; idempotent; `uninstall-shell` removes exactly its block.
- NO eval-init hook for switching: the dispatcher reads the project file
  itself (the mise model, not the rbenv `eval "$(… init -)"` model).
- `tebako use / disable / list / doctor` manage shims
  (link/remove/inspect/diagnose); enable/disable specific versions.

## 4. Configuration

- User config: `~/.tebako/config.yaml` (YAML — the locked convention;
  supersedes the earlier `config.json` note). Contents: defaults,
  registries, runtime preferences.
- Project pins: `.tebako-tools.yaml` at any directory — the dispatcher
  walks up from cwd; nearest wins.

## 5. Distribution forms (both produced by `tebako press`)

1. **Standalone tpkg** (always per-platform): self-contained executable
   for users WITHOUT tebako.
2. **Registry payload** (`.tfs` + registry metadata): for dispatcher
   users — ONE universal image when pure-language; per-triplet variants
   only for native-extension apps.

## 6. Retirement gate

When tebako-shim ships, **mnenv retires**; metanorma becomes the first
dogfood consumer (heavy native-ext app proving the whole stack: press →
signed `.tfs` per (version × ruby line) → registry → dispatcher).

## 7. Errors (named)

- No compatible cached/downloadable runtime → the spec 06 exit 69 shape.
- Native-ext payload on a wrong-ABI-line runtime → named compatibility
  error, never a segfault.
- Shim target payload missing/corrupt → named error pointing at
  `tebako doctor`.
