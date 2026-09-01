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

The index is **per-package first** (tebako#493): the resolver's
preference order is the per-package shard → the derived monolith → the
line index. The fallbacks stay forever — pre-shard releases are
immutable and remain installable (invariant 7); a missing, unreadable,
or triple-mismatched shard falls through to the next form with the URL
recorded for the failure message.

- `<stem>.manifest.json` — the **per-package shard**, the sidecar-era
  authority. The stem is the exe asset's own name
  (`tebako-runtime-<ver>-<lv>-<triplet>` — suffix-less, §2's spelling
  rule below, on windows too). The shard is ONE manifest-entry object
  (the same fields the monolith's array items carry: the identity
  triple, `filename`, `sha256`, the additive `image` / `dll` /
  contract-set keys). The resolver derives the stem from the requested
  triple, fetches exactly one small object, and reads the contract gate
  from it (normalized to the array shape at the resolver boundary —
  tebako-resolve owns the release-card reader semantics).
- `manifest.json` — derived monolith; the same per-asset entries as an
  array, plus the additive
  image-era key `image: {filename, sha256, size_bytes}` and the additive
  `contract_version` key (bootstrap↔runtime contract, spec 06 §6).
- **The entry's `filename` is the ONLY authoritative asset spelling**
  (spec 00 §10 SSOT; tebako#456). A consumer matches the entry by the
  identity triple (`tebako_version`, `<lang>_version`, `platform`) —
  never by a synthesized name — and flows `filename` /
  `image.filename` / `dll.filename`+`install_as` verbatim into the
  download URL, the cache layout, and the pre-download contract gate.
  The `tebako-runtime-<ver>-<lv>-<triplet>[.exe]` grammar is the
  factory's to declare, not the consumer's to derive (the factory
  publishes the windows exe SUFFIX-LESS). When no entry matches the
  identity triple (a pre-identity index), the synthesized spelling
  remains the fallback for the SHA256SUMS-only era, and the
  missing-entry refusal names the identity triple — the mis-lookup is
  diagnosed as what it is, never as a contract refusal of a
  hand-invented name.
- `SHA256SUMS.txt` — line-index fallback (`<sha>  <file>`), carries the
  `<asset>.tfs` lines in the image era.
- Every payload asset also carries its own `<asset>.sha256` sidecar
  (coreutils `<sha>  <file>` — the store marker's exact shape). The
  sidecar is the per-asset pin; the shard's sha fields are re-anchored
  to the served bytes at publish time.
- **The download base is PER-ENGINE** (the chain below is PLANNED —
  TODO.v2-1/30; today a single base serves all engines, with
  `TEBAKO_RUNTIME_MIRROR` as the only override). First hit wins, and
  every download journals the base AND the channel that supplied it:
  1. `runtimes: {<engine>: {source: <base>}}` — the authored config pin
     (spec 07 §0; most specific: an operator who pins an engine's
     source means exactly that base for that engine).
  2. `TEBAKO_RUNTIME_MIRROR` (https or file://) — the operator's global
     override, applying to every engine WITHOUT a `source:` pin. When a
     pin shadows a differing mirror value the shadowing is journaled,
     loud — never silent precedence.
  3. **Registry-derived** — a registered registry (spec 04 §2) carrying
     a `kind: runtime` entry whose `engine:` (+ `implementation:` when
     the edge names one) matches, with a version satisfying the
     edge's constraint: the base derives from that version's
     `release.ref`. This is the zero-config path — a third-party
     runtime becomes resolvable from `tebako add-registry <its
     feedstock>` alone, no authored config required.
  4. The product default base
     `https://github.com/tamatebako/tebako-runtime-ruby/releases/download`
     (the ruby factory line — the back-compat floor every pre-chain
     consumer already speaks).
  An edge no channel answers is a NAMED error enumerating the channels
  tried (never a silent query of a base that hosts no such engine — a
  wrong-line 404 is diagnosed as what it is).
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

The interpreter/image file names in a cache entry keep the index entry's
`filename` / `image.filename` spellings verbatim (§2 — on windows the exe
is suffix-less; the loader execs by full path and CreateProcess needs no
`.exe`). The `[.exe]` diagram notation marks the synthesized fallback
spelling, used only when no index entry is available (fat-payload
installs, pre-identity manifests).

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
- **Lazy seeding (the scoped exception, locked 2026-08-25, tebako#460):**
  a package that CARRIES artifacts (the self-contained preset — spec 23
  §13) MAY seed the machine cache with them at run time: the runtime
  pair (exe + env image) and each carried payload slice, so later
  packages share them. The exception is exactly this wide and no wider:
  - best-effort — a seed that cannot complete NEVER blocks or fails the
    run; the carried artifacts keep serving from the package;
  - tmp + rename under the entry lock — a partial seed is invisible and
    the next run re-seeds;
  - idempotent — a same-sha entry skips;
  - trust anchors exactly as `tebako install`: trailer slot digests for
    signed (v2) packages, the computed digest for unsigned ones — the
    run's own enforcement strength, never upgraded;
  - journaled (`event=lazy-seed`), and shown by `tebako cache list`
    with the seeding package as origin;
  - `TEBAKO_OFFLINE=1` does not block seeding (no network is involved);
  - `TPKG_FLAG_NO_INSTALL` packages never seed (spec 02 §5).
  The shared-runtime preset needs no exception: its artifacts arrive
  through the ordinary resolve-download-verify-install path.

## 5. Resolution chains

**Runtime for a payload (dispatch time):** payload's declared requirement
(engine + optional implementation + constraint) → newest COMPATIBLE
runtime already cached (no download) → else download the newest
compatible from the runtime releases → verify → cache. Swapping runtimes
never touches the payload.

**Runtime for a shared-runtime package (first run):** trailer
runtime_ref → cache hit → use; miss → download from the index
(per-package shard primary, manifest.json next, SHA256SUMS fallback —
§2's order) → sha256-verify → flock'd
atomic install. (Preset names per spec 23 §13.2: *shared-runtime* is
the default preset and the successor of the deprecated name *lean*;
*self-contained* succeeds *fat*.) Self-contained: the runtime pair is
carried as two package slots — the interpreter exe and the env image
(spec 19 §6.1): the exe stages into the runtime cache, the env image
mounts from the package file through the ordinary
`<self>:<slot>:<mount>` grammar, both verified against the trailer slot
digests, and both seed the cache per §4's lazy-seed rule. (Pre-#458
packages carry the runtime as one legacy `format_id = 4` slot, extracted
and verified against `;sha256=` — readers keep serving that form;
presses never emit it.)

**Compatibility model:** pure-language payloads take a range (`>= 3.3,
< 5.0`) — any newer runtime works; native-extension payloads lock to the
ABI line they were built against, which is TWO-dimensional: the version
line (`~> 3.3.0`) AND the platform line (`abi: arm64-darwin-23` — the
runtime's own platform string; ruby: `Gem::Platform.local.to_s`, from
the staging runtime's `RbConfig::CONFIG["arch"]`). A newer line needs a
new payload build (or an SDK rebuild path); a wrong line on EITHER axis
produces a named compatibility error, never a segfault. Runtimes publish
their platform string as the per-package `abi` key in the release index
(spec 13); cached runtimes installed before the field existed stay
eligible (the compat window — a payload's abi check never fails against
an unknown line).

**The implementation axis (spec 28 §8):** `engine` names the LANGUAGE —
mri, jruby and truffleruby are all `engine: ruby`, told apart by the
runtime's `provides.implementation`. A requirement WITHOUT
`implementation` matches any cached runtime of the engine whose
`language_version` satisfies the constraint (the pure-language case: one
build, any implementation — jruby 9.4 declares `language_version:
"3.1"`, so a `>= 3.3` requirement does not match it and a `>= 3.1` one
does). A requirement WITH `implementation` reads its constraint against
that implementation's OWN version line (`{engine: ruby, implementation:
jruby, constraint: "~> 9.4"}`) and matches no other implementation. An
`abi:` in force requires `implementation` — ABI and platform lines are
per-implementation by construction; a native requirement naming no
implementation is a named manifest error (spec 03 §2.2). Compatibility
never crosses implementations silently: the chosen runtime's
implementation is journaled with the resolution event.
