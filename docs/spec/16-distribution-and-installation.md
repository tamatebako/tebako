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

winget/scoop manifests pointing at the same standalone binaries (same
template approach as brew taps); the tebako CLI itself follows once the
Windows leg ships (roadmap 02).

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

## 6. Implementation gaps (roadmap 28)

- ~~`tpkg-registry.yaml` fetch/listing (the resolver tail of item 07)~~ —
  SHIPPED (28.1): the registry model + resolution in tebako-resolve and
  `tebako add-registry | list-registries | install | uninstall` in
  tebako-cli (ref + nickname forms, declarative triplet selection,
  registry sha256 pins, OpenPGP verification of signed entries, the
  v1-legacy unsigned warn, audit journal).
- ~~the dispatch-time registry cache~~ — SHIPPED (33): the shim resolves
  every registry form at dispatch behind
  `~/.tebako/registries/<sha>.yaml` (24 h TTL), `tebako
  update-registries`, `TEBAKO_OFFLINE` = cache-or-named-error.
- ~~`tebako publish` helper~~ — SHIPPED (41): press-output payloads →
  optional sign (per-artifact `<artifact>.asc`, the `<keyid, asc>`
  registry pin) → upload to the referenced GitHub release (in-process
  HTTP; file:// mirrors for rehearsal/tests) → `tpkg-registry.yaml`
  upsert → tap formula render from the vendored template → built-in
  clean-cache `tebako install` proof. The GitLab/Bitbucket write legs
  are their adapters' milestone.
- `tamatebako/homebrew-tap` formula + the app-tap template (the template
  is vendored and rendered by `tebako publish --tap`; the tap repos
  themselves stay manual).
- `install.sh` + its own CI verification.
- Docs pages (tebako.org, post-gate) mirroring §5 per audience.
