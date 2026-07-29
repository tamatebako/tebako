# Spec 05 — Resolution and machine cache (L3)

Normative specification of runtime/payload resolution and the shared
machine cache. Status: SHIPPED for runtimes (M6–M8); payload cache PARTIAL
(roadmap 07).

## 1. runtime_ref (the resolution hint in the v1 trailer)

```
<lang>@<lang-version>;tebako=<tebako-version>[;image][;sha256=<64 hex>]
```

- Language-generic by construction; `ruby@…` is today's instance.
- `;sha256=` — fat payload checksum (the embedded runtime slot).
- `;image` — bare flag: image-era runtime (interpreter + separate `.tfs`).
  The image's expected sha256 comes from the release index — the same
  trust source as the executable's own checksum. Bare-flag form keeps
  image-era fat refs inside the 127-byte field budget.
- Refs without `;image` behave byte-identically to v1: no image lookup,
  no download, no env.

## 2. Release index (runtime factory releases)

- `manifest.json` — machine index; per-asset entries plus the additive
  image-era key `image: {filename, sha256, size_bytes}` and the additive
  `contract_version` key (bootstrap↔runtime contract, spec 06 §6).
- `SHA256SUMS.txt` — line-index fallback (`<sha>  <file>`), carries the
  `<asset>.tfs` lines in the image era.
- Base URL:
  `https://github.com/tamatebako/tebako-runtime-ruby/releases/download`;
  override `TEBAKO_RUNTIME_MIRROR` (https or file://).
- Signing of the index itself: spec 09 §5 (signed manifest closes the
  same-channel-MITM gap).

## 3. Machine cache layout (`~/.tebako`)

```
runtimes/<lang>-<lv>-<ver>-<triplet>/
  tebako-runtime-<ver>-<lv>-<triplet>[.exe]   # interpreter (0755)
  sha256 / origin                             # executable markers
  tebako-runtime-<ver>-<lv>-<triplet>.tfs     # image-era runtime (0444)
  <image>.sha256                              # trust anchor ("<sha>  <file>\n")
  <image>.origin                              # source URL
payloads/<name>/<version>.tfs                 # registry payloads (0444)
payloads/<name>/<version>.tfs.sha256          # trust anchor
shims/                                        # spec 07
config.yaml                                   # spec 07 (YAML — never JSON)
keys/                                         # press-local signing keys (spec 09)
```

## 4. Install and trust rules (locked)

- Per-entry **flock** (120 s timeout with stale-lock hint); install is
  tmp + rename — a partial install is invisible.
- The `.sha256` marker IS the trust anchor: present ⇒ the artifact was
  sha256-verified at install; re-verify only on re-fetch, **never per
  run**.
- Image-era cache entries hold artifacts only — no extracted `layout/`
  tree; press seeds its environment by extracting in-process through the
  TFS ABI and rebuilds per press.
- `TEBAKO_OFFLINE=1` — cache hit or hard error.
- `tebako cache list` / `cache prune [--all] [--older-than Nd]` manage
  the cache.
- **A run is a run** (TODO.v2-1/12): executing a package NEVER installs
  its payload slices. `tebako install <path>` installs a local package's
  payload slices explicitly — trailer slot digests are the anchors for
  signed (v2) packages, the computed digest for unsigned ones (the run's
  own enforcement strength, never upgraded); the runtime slot is NOT
  store-installed (it resolves into the runtime cache on first run, as
  always). Idempotent same-sha skip; same name+version with different
  content is never overwritten (loud warning + journal). Shims link only
  via the explicit `--shims` (spec 07). `TPKG_FLAG_NO_INSTALL` packages
  refuse (spec 02 §5).

## 5. Resolution chains

**Runtime for a payload (dispatch time):** payload's declared requirement
(engine + constraint) → newest COMPATIBLE runtime already cached (no
download) → else download the newest compatible from the runtime releases
→ verify → cache. Swapping runtimes never touches the payload.

**Runtime for a lean package (first run):** trailer runtime_ref → cache
hit → use; miss → download from the index (manifest.json primary,
SHA256SUMS fallback) → sha256-verify → flock'd atomic install. Fat: the
runtime payload slot is extracted from the package itself and verified
against `;sha256=`.

**Compatibility model:** pure-language payloads take a range (`>= 3.3,
< 5.0`) — any newer runtime works; native-extension payloads lock to the
ABI line they were built against (`~> 3.3.0`) — a newer line needs a new
payload build (or an SDK rebuild path). A wrong-line runtime produces a
named compatibility error, never a segfault.
