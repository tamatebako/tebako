# Spec 16 — Distribution and installation UX

Normative specification of how end users install tebako itself and
tebako-packaged applications. Status: decisions LOCKED 2026-07-26;
implementation roadmap 28.

## 1. Personas (MECE — every install path serves exactly one)

- **A — the app user** ("I just want metanorma"): zero tebako awareness
  required. Channels: Homebrew formula from the app's own tap, or direct
  binary download.
- **B — the tebako user**: installs the tebako CLI once, then manages all
  apps/runtimes/shims through it. Channels: our Homebrew tap, or the
  curl|sh installer.
- **C — the app developer**: ships apps via `tebako press` (+ sign +
  publish) in their own CI.

## 2. The artifacts an app ships (per release)

Both forms are produced by `tebako press` in the app's CI:

1. **Standalone binaries** `app-<ver>-<triplet>` — bootstrap + payload,
   runtime resolved at run time. **Slim is the primary** (small download;
   the runtime fetches once per machine into `~/.tebako` and is SHARED by
   every tebako app on that machine — even persona A gets one-runtime-
   per-machine, never per-app). **Fat is the optional form** for
   air-gapped/offline use.
2. **Registry payload(s)** `app-<ver>[-<triplet>].tfs` +
   `tpkg-registry.yaml` in the app's repo.

**Platform binding (locked):** an app with native extensions (metanorma
is the canonical case — many native extensions, platform-specific) ships
PER-TRIPLET payloads — built in a matrix where each leg is ONE mechanical
press step; per-platform handling is a first-class manifest feature, not
a hack. Pure-language apps ship ONE universal payload. The registry
declares the axis honestly per spec 03 §3.

## 3. Channels (locked)

### 3.1 Homebrew

- **`tamatebako/homebrew-tap`** hosts the `tebako` formula: all four
  binaries (tebako, tfs, tebako-pkg, tebako-shim) + shell completions,
  per-platform `url` + `sha256` blocks (the standard binary-formula
  pattern — no build, no bottles-to-build).
- **Each app project hosts its own tap with our template** (e.g.
  `metanorma/homebrew-tap` with a `metanorma` formula pointing at the
  app's standalone binaries). We provide the template + the `tebako
  publish` helper that bumps it.
- Brew covers macOS + Linux (linuxbrew) for personas A and B alike.

### 3.2 curl|sh installer (tebako CLI only)

`install.sh`: detects platform → downloads the binary + `SHA256SUMS` from
the tebako-rs release → **verifies sha256 before installing anything** →
installs to `~/.local/bin` (NEVER sudo) → offers the managed PATH block.
After the root-key ceremony it also verifies the detached `.asc`. This is
the Linux-without-brew path and the CI path.

### 3.3 tebako-native install

```
tebako install tfs:github:metanorma/metanorma:1.2.3
tebako install tfs+https://cdn.example.com/app.tfs?sha256=<hex>
```

- Resolves via spec 04: download → verify (sha256 pin and/or OpenPGP
  signature when present) → content-addressed cache → register every
  shim the payload's PROVIDES declares (spec 07).
- **Registry nicknames are EXPLICIT ONLY (locked):**
  `tebako add-registry tfs:github:metanorma/metanorma` binds the name
  once; thereafter `tebako install metanorma@1.2.3` resolves within
  user-registered registries. The shipped config has ZERO registries —
  no default of any kind.
- Lifecycle: `tebako use app@ver` (switch/rollback, side-by-side
  versions), `.tebako-tools.yaml` per-project pins, `tebako uninstall`,
  `tebako cache list|prune`.

### 3.4 Windows

winget/scoop manifests ride the system MSI (§7) — the same template
approach as brew taps, one container, no channel forks the binaries.

## 4. Trust per channel (locked)

- Brew/direct: sha256 in the formula/manifest (transport integrity). If
  the package is signed, the bootstrap still enforces v2 verification at
  run time — the channel need not know.
- tebako channel: sha256 + optional OpenPGP signature verified AT
  INSTALL, trust anchors in the cache, `TEBAKO_REQUIRE_SIGNED=1` for
  hardened setups.
- curl|sh: sha256-verified before install; `.asc` after the ceremony.
- Nothing anywhere installs as root, writes outside `~/.tebako` +
  `~/.local/bin` + the managed shell block, or phones home to a central
  service — registries are plain git-host releases.

## 5. The metanorma walkthrough (canonical example)

Developer side (metanorma CI): matrix over triplets — one `tebako press`
per triplet (native extensions compiled against the target runtime via
the mounted SDK) → standalones + per-triplet `.tfs` payloads → sign →
upload to `metanorma/metanorma` releases → commit `tpkg-registry.yaml`
→ bump `metanorma/homebrew-tap` formula.

User side:

```
# persona A (mac):
brew install metanorma/tap/metanorma && metanorma --version

# persona A (any OS, direct):
curl -LO .../metanorma-1.2.3-macos-arm64 && chmod +x ... && ./...

# persona B:
brew install tamatebako/tap/tebako        # or: curl -fsSL .../install.sh | sh
tebako add-registry tfs:github:metanorma/metanorma
tebako install metanorma@1.2.3            # payload + runtime resolved + shim
metanorma --version
tebako use metanorma@1.2.2                # instant rollback
```

## 6. Offline application bundles (`tebako bundle`)

Persona B's airgapped half (roadmap 82/83 — BSH's SCCM class): an
installer-facing tree composed ENTIRELY from the canonical published
artifacts, pre-positioned so the target machine may never see the
network.

```
tebako bundle <name[@ver]> --output <dir> [--config <org.yaml>] [--archive tar.gz]
```

The two hard rules:

1. **Canonical bytes, never rebuilt.** Payloads and runtimes stage
   through the store's own machinery (registry resolution, sha256 +
   signature verification per the trust rules of §4) — the bundle is a
   PRE-SEEDED store, byte-identical with what the registries published.
2. **Configurability lives in the rendered `config.yaml`, never in
   rebuilt bytes.** The `--config` org overlay layers the spec 04
   `network:` block (proxy / tls_roots / extra_ca), per-engine runtime
   preferences, per-tool defaults, and extra registries over the
   builder's own config.

Layout (`<out>` is created by the verb; an occupied path is a named
usage error, exit 65 — a bundle never overwrites):

```
<out>/bin/          the platform's CLI tool set (tebako, tebako-shim,
                    tfs, tebako-pkg — copied from beside the running
                    exe; a partial set is a named error, exit 65)
<out>/home/         the staged TEBAKO_HOME, verbatim store grammar:
                    payloads/, runtimes/, registries/ caches, shims/,
                    config.yaml, journal.log
<out>/BUNDLE.yaml   the descriptor (schema_version 1): payload identity,
                    platform, the staged runtimes, the command list an
                    installer materializes onto PATH
```

Staging semantics:

- The payload installs through `tebako install`'s own path — the eager
  closure (dependencies, spawned-edge runtimes, spec 30/32) included.
- The PRIMARY runtime is additionally **warmed**: install alone leaves
  the payload's own `kind: language` edge to first dispatch, which an
  offline machine cannot do — the bundle resolves it at stage time
  through the dispatch resolver (cache → pins → mirror → registries →
  default, spec 05 §2).
- After staging, `runtimes:` in the shipped config is **re-pinned from
  the staged reality** (one pin per staged engine, newest line): the
  bundle's config never names a runtime it does not carry. An operator
  pin for an engine the bundle did not stage survives verbatim.
- Unix shims are rewritten to the bundle-relative
  `../../bin/tebako-shim` — the tree relocates as one piece; windows
  shims are byte copies and self-contained already.
- The whole tree stages under `<parent>/.bundle-staging-<pid>` and
  renames into place: a failed bundle never leaves a half-tree.
- `--archive tar.gz` packs the tree next to itself (symlinks preserved —
  they ARE the dispatch surface). The zip form is PLANNED (it lands
  with the windows installer templates); requesting it is a named error.

Bundles build for the HOST platform only — a cross-target bundle is the
per-platform CI leg's output, not a flag. The installer's side: drop the
tree, point `TEBAKO_HOME` at `home/` (or relocate its contents into the
user store), put `bin/` + `home/shims/` on PATH. Trust verification
happened at stage time; the target machine verifies nothing new.

## 7. Installer packages (MSI / pkg) — roadmap 83

The OS-native installer is a CONTAINER over already-published bytes, for
the audience a curl|sh or a brew tap does not reach (managed fleets,
MDM/SCCM/Jamf, first-run-warn-free downloads). Two orthogonal axes:

**WHO ships (identity).** The same parameterized templates
(`templates/installers/`) serve the tebako system installer (Case A — the
tamatebako identity) and a client product (Case B — packed-mn class — the
client's own Azure Trusted Signing account, Apple Developer ID
certificates, and OpenPGP key; no tamatebako identity leaks into a client
product). Every installer carries THREE independently verified signature
surfaces: the container (Authenticode / Developer ID Installer), the
binaries inside (the release's already-signed exes), and the tebako-plane
payload signatures for any staged content (spec 09).

**WHAT ships (content shape).**

| Shape | Content | Network at install | Network at first run | For |
|---|---|---|---|---|
| Tool-only | the 4 PATH tools (tebako, tebako-shim, tfs, tebako-pkg), signed | — | per-use (runtimes/payloads) | **tebako the system** |
| Fat exe | self-contained preset (runtime as a slot) | — | — | single-runtime apps only |
| Bundle | §6's pre-seeded store inside the container | — | — | **apps with spawned-runtime edges (metanorma)** |
| Web bootstrapper | container that runs `tebako install` post-install | required | — | online convenience |

A fat exe cannot carry spawned runtimes (metanorma spawns java + python
as SEPARATE runtimes; no exe slot carries them) — the bundle is the
installer content for serious apps. The system installer needs no
fat/lean decision at all: the tool ships no payloads by construction.

The locked rules:

1. **Containers ship signed-only.** The MSI ships exactly when spec 34's
   `WINDOWS_SIGNING_ENABLED` armed the pipeline (Azure Artifact Signing
   signs MSI natively); the pkg ships exactly when spec 31 §5a's
   `APPLE_INSTALLER_SIGNING_ENABLED` AND §5's `APPLE_SIGNING_ENABLED` are
   both armed (a signed-but-unnotarized pkg never ships). Unsigned-first
   (invariant 7) continues to cover the loose binaries; an unsigned
   installer CONTAINER is a trap for exactly the audience containers
   serve, so the legs build + rehearse it always (the template's CI
   signal) and ship it never-unarmed.
2. **Container content = the release's own bytes.** The legs download
   this release's tools back from the release and verify each against
   the platform leg's sign-then-hash fragments before wrapping — a
   corrupted or stale input fails the leg, named.
3. **Sign-then-hash extends to containers** (spec 34 §1.3): the
   installer fragments hash the FINAL signed container bytes; finalize
   folds them into SHA256SUMS / sidecars / manifest.json's `installers`
   array like any other released part, and spec 09's release signing
   covers them (one .asc per container).
4. **Install rehearsal is the ship gate, always**: msiexec install → the
   installed `tebako --version` runs → uninstall clean; `installer -pkg`
   → payload lands + `/etc/paths.d` entry written → the installed tool
   runs. A broken template fails the leg, never a user machine.
5. Per-machine install root only (`Program Files\<product>`,
   `/opt/<product>`); per-user state (the `~/.tebako` store, shims)
   stays per-user — the installers install NO user-scope files. PATH
   lands system-wide (MSI Environment row / `/etc/paths.d`), removed on
   uninstall.
6. winget/scoop (roadmap 28's §3.4) ride the MSI asset; brew and
   install.sh stay the developer paths. No channel forks the binaries —
   every channel ships the SAME signed bytes.

## 8. Implementation gaps (roadmap 28)

- ~~`tpkg-registry.yaml` fetch/listing (the resolver tail of item 07)~~ —
  SHIPPED (28.1): the registry model + resolution in tebako-resolve and
  `tebako add-registry | list-registries | install | uninstall` in
  tebako-cli (ref + nickname forms, declarative triplet selection,
  registry sha256 pins, OpenPGP verification of signed entries, the
  v1-legacy unsigned warn, audit journal).
- ~~the dispatch-time registry cache~~ — SHIPPED (33): the shim resolves
  every registry form at dispatch behind
  `~/.tebako/registries/<sha>.yaml` (24 h TTL), `tebako
  update-registries`, `TEBAKO_OFFLINE` = cache-or-named-error; a stale
  cache + a failed refresh serves the stale bytes loud (stderr +
  journal — spec 05 §4, roadmap 86), never a hard error while a cache
  exists.
- ~~`tebako publish` helper~~ — SHIPPED (41): press-output payloads →
  optional sign (per-artifact `<artifact>.asc`, the `<keyid, asc>`
  registry pin) → upload to the referenced GitHub release (in-process
  HTTP; file:// mirrors for rehearsal/tests) → `tpkg-registry.yaml`
  upsert → tap formula render from the vendored template → built-in
  clean-cache `tebako install` proof. The proof home inherits the
  publisher's registered registries AND its `runtimes:` preferences
  (spec 30: the install pre-stages `kind: runtime` edges; a pref-less
  edge on a non-default engine line can never resolve). The
  GitLab/Bitbucket write legs are their adapters' milestone.
- `tamatebako/homebrew-tap` formula + the app-tap template (the template
  is vendored and rendered by `tebako publish --tap`; the tap repos
  themselves stay manual).
- `install.sh` + its own CI verification.
- Docs pages (tebako.org, post-gate) mirroring §5 per audience.
