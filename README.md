# tebako

**One file. Any app. Every OS.**

tebako packs an application and everything it needs into a single
executable — or a small executable plus shared, machine-cached runtime
images — that runs with zero installation, zero system dependencies, and
zero knowledge that tebako exists. It is a packaging and loading
ecosystem: any runtime (ruby today, more languages later), any platform
(linux gnu/musl, macOS, Windows), any payload (apps, data, native tools),
any composition (payloads recursively mounting payloads at paths).

```
you write an app        →   tebako press       →   one file (or file + cache)
your users              →   run it             →   it just works
```

- **Stitch** N payload images + a tiny Rust bootstrap into one binary (fat),
  or a small binary that resolves its parts on first run (lean).
- **Share** runtimes machine-wide: a runtime downloads once into
  `~/.tebako` and every tebako app on the machine reuses it.
- **Shim** every executable a payload provides becomes a first-class
  command (per-version, per-project pins).
- **Jail** declaratively what host filesystem a payload can see or write
  (docker `-v` semantics, ro/rw, enforced by the VFS layer).
- **Trust** opt-in: OpenPGP signatures + per-slot SHA-256 + encrypted
  images (per-subtree, in-memory-only decryption). Unsigned stays
  first-class.

## Install

```console
$ brew install tamatebako/tap/tebako          # macOS / Linux (brew)
$ curl -fsSL https://tebako.org/install.sh | sh   # anywhere else (sha256-verified)
```

Or download a binary for your platform from the
[releases](https://github.com/tamatebako/tebako/releases) page.

## Quickstart

Package your app:

```console
$ tebako press -r ./myapp -e bin/myapp -o myapp
$ ./myapp                        # lean: runtime fetches once, then cached
```

Install an app someone else published (a `.tfs` payload in a registry):

```console
$ tebako add-registry tfs:github:tebako-packages/index
$ tebako install metanorma@1.16.9
$ metanorma --version            # its runtime + deps resolved automatically
```

Use a native tool from a payload without installing it (runs from its
image through the interposition shim — no extraction):

```console
$ tfs exec inkscape.tfs -- /bin/inkscape --version
```

## Architecture

A tebako package is **three parts** stitched into one executable:

```
┌──────────────────────────────────────────────────────────────┐
│ 1. BOOTSTRAP (Rust, < 3 MB) — the process entry point.       │
│    Parses the tpkg trailer, verifies signatures/digests,     │
│    resolves the runtime (cache / payload / download), and    │
│    hands off to it.                                          │
├──────────────────────────────────────────────────────────────┤
│ 2. RUNTIME — the interpreter payload (a runtime is just a    │
│    payload that provides one). Fat: embedded. Lean/image-era:│
│    resolved from the machine-wide cache or downloaded once.  │
├──────────────────────────────────────────────────────────────┤
│ 3. PAYLOADS — 1..8 TFS images (your app, data, native tools).│
│    Mounted read-only by the runtime through the TFS layer;   │
│    each carries a YAML manifest (IDENTITY/PROVIDES/DEPENDS). │
└──────────────────────────────────────────────────────────────┘
A tpkg trailer (166-byte header at EOF + slot table + optional signed
extension blocks) describes parts 2–3. Details: docs/spec/02.
```

Layers:

- **tpkg** — the wire container format (bootstrap + slots + trailer),
  byte-specified, versioned, optionally signed (`docs/spec/02`).
- **TFS** — the userland virtual filesystem (`tebako_fs_*` C ABI, Rust):
  multi-mount images with longest-prefix routing, backends
  (dwarfs-t / zip / tar / squashfs), a Rust COW overlay, and the
  enforcement point for jails (`docs/spec/11`).
- **Manifests** — every payload is self-describing: `kind`
  (app/runtime/toolkit/data), `provides` (executables, libs, interpreter),
  `requires` (language/toolkit/data/executable deps with
  consumer-declared mounts), platforms, provenance, signing/encryption
  state (`docs/spec/03`).
- **Resolution** — MECE references (`tfs:github:`/`tfs:gitlab:`/`tfs:bb:`,
  `tfs+git://`, `tfs+https://`, `file://`, `?sha256=` pins — no default
  service) + a content-addressed, sha256-anchored machine cache
  (`docs/spec/04`, `docs/spec/05`).
- **Dispatch** — shims for every provided executable; version selection
  per env > project file > user default > registry default; runtime
  compatibility by manifest constraint (`docs/spec/07`).
- **Trust** — opt-in signing (OpenPGP chain of trust with rotation),
  per-slot digests, strict verification; opt-in encryption (stacking
  EncBackend, DEK envelopes, HKDF subtree keys) (`docs/spec/09`,
  `docs/spec/10`).

Repos are few and purpose-split: this one (`tamatebako/tebako`, the
Rust stack) plus three C/C++ **factories** that only build their own
releases: `tamatebako/ruby` (patched ruby sources),
`tebako-runtime-ruby` (compiled ruby runtimes), `tamatebako/dwarfs-t`
(the DwarFS-T image format library). Everything else is Rust. Tooling
payloads (inkscape & co.) are feedstocks in the
[`tebako-packages`](https://github.com/tebako-packages) org.

## Commands

| command | what it does |
|---------|--------------|
| `tebako press` | package an app (lean/fat; `--suite` for multi-command packages) |
| `tebako install <ref\|name@ver>` | install a payload from a registry + register its shims |
| `tebako use / list / doctor` | manage shims, versions, and health |
| `tebako publish` | press → sign → upload → registry → tap formula (developer flow) |
| `tebako-pkg` | tpkg trailer surgery: bundle/unbundle/reassemble/insert/remove/info/validate/sign/verify |
| `tfs` | the TFS image tool: info/ls/tree/cat/stat/extract/find/mkimage/mount/exec |
| `tebako-shim` | the dispatcher binary behind every shim on PATH |

Full reference: [`docs/spec/`](docs/spec/00-INDEX.md) (the normative spec
set — wire format, manifests, references, cache, launcher ABI, shims,
jails, trust, encryption, TFS, comparisons, factories, distribution).

## Status

- **Shipped**: the full Rust stack — packager, loader (macOS/Linux),
  runtime-as-image, opt-in signing + rotation, multi-mount TFS with
  tar/zip/squashfs/dwarfs-t backends + COW overlay, host-access jails,
  the preload exec shim, payload manifests + registries + shims, suite
  packages, encryption (opt-in), and the ruby runtime factory line
  (ruby 3.1–4.0, 7 platforms).
- **In flight**: Windows release leg, remote-registry niceties, and the
  TFS-FUSE driver.

License: BSD-2-Clause. Note the DwarFS backend links dwarfs-t (GPL-3.0)
— see `docs/spec/11` §8 for what that means for binaries you ship.
