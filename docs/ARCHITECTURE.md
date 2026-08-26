# Tebako Architecture

The normative behavior of the tebako ecosystem lives in the
**specification set**: [docs/spec/](spec/00-INDEX.md) — indexed, layered,
and per-topic. Start there.

This file is the **map**: which components ship, what each one links and
can do, what is deliberately absent, and the build invariants that have
bitten before. When code disagrees with this file, the code is wrong or
the file is stale — fix one of them in the same PR.

## The four tiers

### Tier 0 — the loader

**`tebako-bootstrap`** (`crates/tebako-bootstrap`) — the process entry
point of every stitched package. Reads the tpkg trailer, fetches the
lock's concrete pinned sources, seeds carried slices into the machine
cache, verifies sha256 sidecars, hands off to the runtime via the
launcher ABI. It resolves **pinned, concrete references only** — no
registries, no version ranges (those were resolved at press time and
frozen into the lock).

- Hard size gate: **< 3 MiB per platform**, enforced in the release
  pipeline (`BOOTSTRAP_SIZE_BUDGET`, `.github/workflows/lib/stage.sh`).
- Links: `tpkg`, `tebako-http`, `tebako-term`, `tebako-resolve`
  (**`default-features = false`** — the `git` feature stays OFF), no
  tfs, no imaging stack.
- `openpgp-verify` is an optional dev feature; shipped builds are
  unverified-first (`TEBAKO_REQUIRE_SIGNED=1` fails closed).

### Tier 1 — the toolchain (managed mode, full capability)

| binary | crate | role |
|--------|-------|------|
| `tebako` | `tebako-cli` | press, install, check, inspect — the packager's tool; carries the limnifs writer |
| `tebako-shim` | `tebako-shim` | argv0 dispatcher on PATH; per-invocation version chain (env → `.tebako-tools.yaml` → `config.yaml` → registry default) |
| `tfs` | `tfs-cli` | image toolbox: mkimage / extract / ls / cat / exec / needs |

The toolchain links the **full** tebako-resolve (the `git` feature ON),
the registry walk, and version-range resolution. This is where
resolution *happens*: press-time resolution writes the L2 lock that the
bootstrap later consumes verbatim.

Libraries: `tebako-pkg` (packaging), `tpkg` (wire container + manifest
model, `#![forbid(unsafe_code)]`), `tebako-resolve` (references,
registry, cache), `tebako-http` (ureq + rustls + webpki-roots),
`tebako-signer` (opt-in trust), `tebako-info`, `tebako-json`,
`tebako-term`.

### Tier 2 — the runtime pair (factory artifacts, not built here)

Every runtime is TWO artifacts from the runtime factory
(`tebako-runtime-ruby` today): the **runtime exe** (interpreter +
embedded `tebako-driver`) and the **env image** (`.tfs`). The driver
mounts exactly what the loader hands it — env image first, then payload
slices in order — applies the jail, rewrites argv. **The runtime
resolves nothing.**

### Tier 3 — payload slices

Bare `.tfs` images with in-image L1 manifests (`/__tpkg__/manifest.yaml`):
executable / data / runtime / toolkit kinds. Payloads **declare** needs,
entrypoints, and `runtime_requirement`; they never resolve.

## Reference-scheme capability matrix

Per spec 04. MECE: a scheme a binary cannot resolve is a **named
error**, never a guess and never a silent skip.

| scheme | `tebako-bootstrap` | toolchain (CLI / shim / tfs) |
|--------|--------------------|------------------------------|
| `tfs:github:` / `tfs:gitlab:` / `tfs:bb:` | ✓ (service API via tebako-http) | ✓ |
| `tfs+https://` | ✓ (`?sha256=` pin rules apply) | ✓ |
| `file://` | ✓ | ✓ |
| `tfs+git://` | **compiled out** → `GitAdapterDisabled` | ✓ (tebako-resolve feature `git`) |

## What is NOT used (the anti-map)

- **The bootstrap never**: touches registries, evaluates version ranges,
  fetches `tfs+git:`, links tfs/limnifs/dwarfs, or runs the OpenPGP
  stack (dev-only feature).
- **The runtime exe never** resolves references or fetches anything.
- **`libtfs` (the C++ library)** is a legacy parity oracle in a separate
  repo — nothing in this workspace consumes it.
- **v1 artifacts** (the C99 bootstrap, the C++ `tebako-main.cpp` driver,
  the gem orchestrator, merged env+app images) are dead — do not read
  them for guidance; the spec set is the truth.
- No shell-outs, no system dependencies in any shipped artifact
  (spec 00 invariant 1).

## Build invariants (learned the hard way — do not regress)

1. **`tebako-bootstrap` always builds in its OWN `cargo build`
   invocation**, in every workflow and script. Cargo unifies features
   within one invocation: building the bootstrap together with
   `tebako-cli`/`tebako-shim` silently re-enables tebako-resolve's `git`
   feature inside the size-gated loader (v0.2.7 release run 32940980101
   failed the 3 MiB gate on every leg for exactly this reason).
2. **Optional capability = cargo feature on the owning crate**: default
   ON for the toolchain, OFF for the bootstrap, named error when
   compiled out. First instance: tebako-resolve's `git`.
3. **The workspace `Cargo.lock` is gitignored — resolutions float.** A
   dependency line that evolves incompatibly within one semver line
   (prerelease limnifs is the case) needs an explicit upper bound on
   stable branches; the flip branch raises the floor and drops the
   bound in the same change.
4. **Never trust the main checkout.** Work in a fresh
   `git worktree add … -b <branch> origin/main`; the shared checkout
   may be arbitrarily behind (a stale-tree grep once produced a wrong
   "unused dependency" root cause).
5. Never `cargo test --release` locally (the panic=abort double-compile
   stamps a colliding `libtfs.rlib` — AGENTS.md). Test in debug, the
   CI shape.

## The specification set

- What tebako is (packaging + loading ecosystem, any runtime / platform /
  payload / composition): [spec 01](spec/01-overview.md)
- The tpkg container (byte-exact, authenticated/signed):
  [spec 02](spec/02-tpkg-wire-format.md)
- Payload manifests (IDENTITY / PROVIDES / DEPENDS):
  [spec 03](spec/03-payload-manifest.md)
- References, registries, resolution, cache:
  [spec 04](spec/04-references-and-registry.md),
  [spec 05](spec/05-resolution-and-cache.md)
- Launcher ABI and exit codes: [spec 06](spec/06-launcher-abi.md)
- Shims and version dispatch: [spec 07](spec/07-shims-and-dispatch.md)
- Jails: [spec 08](spec/08-jails.md)
- Trust/signing and encryption: [spec 09](spec/09-trust-and-signing.md),
  [spec 10](spec/10-encryption.md)
- The TFS virtual filesystem: [spec 11](spec/11-tfs-vfs-model.md)
- Comparisons with rubygems/rbenv/Homebrew/apt-dnf/snap/flatpak/AppImage/
  Docker-OCI/VMs: [spec 12](spec/12-comparisons.md)
- Factories and release pipelines: [spec 13](spec/13-factories-and-releases.md)
- Engineering process: [spec 14](spec/14-process.md)
