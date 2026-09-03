# Spec 31 — macOS signing and notarization

**Status: PLANNED (drafted 2026-09-04; implementation lands per §8's
order).** Amends spec 03 §2 (the L1 manifest gains the `signing:` block),
spec 23 §2 (a second non-host sibling declaration) and §4 (press merges
the entitlement union). No wire-format change; no trailer change; no
change to spec 09's trust layer — Apple signing and tebako payload
signing are orthogonal planes (§1.0). First proving ground: packed-mn
(the v1 notarization user). Windows and Linux appear here only for
comparison (§7) — macOS is the only server-blessed application gate among
the supported platforms.

## 0. What Gatekeeper actually assesses

macOS assesses **Mach-O code that carries the `com.apple.quarantine`
xattr** — set only by quarantine-opt-in downloaders (browsers, Mail),
never by curl, gh, or tebako's in-process HTTP client. Data files are
never assessed. That one fact splits the artifact plane cleanly:

| artifact | kind | Apple signature | who signs |
|---|---|---|---|
| bootstrap exe | Mach-O | yes — it is the user-facing download | the tebako org, once, at tebako release |
| tebako CLI tools | Mach-O | yes | the tebako org (tebako release) |
| runtime exe (per engine × version × arch) | Mach-O | yes (§5 — defense in depth) | the tebako org (factory release) |
| env `.tfs` image | **data** | no | — |
| payload `.tfs` slices | **data** | no | — |
| stitched self-contained package | Mach-O (bootstrap + slots) | yes — **post-stitch only** (§1.2) | **the app publisher** |

## 1. The execution-model consequences (normative)

### 1.0 Two signing planes, never conflated

Spec 09's OpenPGP trust (tebako-signer) anchors trust **inside** the
tebako ecosystem: who published this payload, may my machine load it.
Apple signing is the **OS platform gate**: may this Mach-O execute on a
macOS host at all. Neither substitutes for the other; a package may
carry either, both, or neither. This spec changes nothing about spec 09.

### 1.1 Payloads are data; Gatekeeper never sees them

The interpreter exe is signed once by the tebako org; every program it
ever runs is mounted data. Notarization attaches to the *interpreter*,
never to the *program*: a payload update (metanorma 1.16.9 → 1.16.10)
re-touches no Apple machinery. An interpreted payload on macOS is
therefore **more** platform-independent than a native app, not less —
its executable carrier is shared, already-signed infrastructure.

### 1.2 Stitch-then-sign, never sign-then-stitch

Appending slots to a signed bootstrap invalidates its CodeDirectory.
Therefore `tebako press` (self-contained mode) emits an **unsigned**
stitched exe; signing is a post-press step in the publisher's pipeline
(§4). Shared-runtime (lean) packages skip this entirely: the bootstrap
bytes the user runs are the tebako org's already-signed release bytes.
Press never signs; no `--codesign` flag exists. (The `--sign` flag, when
implemented, means spec 09's OpenPGP package signing — the other plane.)

### 1.3 The store download plane is unquarantined by construction

The bootstrap's in-process downloader (ureq) sets no quarantine xattr,
and a bare CLI Mach-O does not propagate quarantine to files it writes —
so cached runtime exes and materialized subprocess executables (spec 29's
exec-cache tier, e.g. openjdk's `bin/java`) exec without assessment.
This is the same mechanism class as npm/pip/brew shims. Byte-identical
materialization preserves any upstream Mach-O signature (an Adoptium java
stays Adoptium-signed inside the copy).

### 1.4 Hardened runtime changes the dlopen rules

A hardened-runtime process by default refuses unsigned dylibs. A tebako
runtime dlopens native extension `.bundle`s that live as *mounted data*
and materialize unsigned — so a signed runtime's entitlement set MUST
carry `disable-library-validation` (§2). The day signing lands, the
acceptance canary re-runs: a nokogiri `.bundle` load under the signed
hardened-runtime exe (§6). Unsigned/adhoc builds need nothing — which is
why pre-signing builds probe clean and prove nothing about the signed
shape.

### 1.5 Entitlements are derived and merged, never authored per-pipeline

A slice manifest knows what it needs (`jit`, `dlopen-unsigned`, the
preload tier). Press merges the entitlement union from the composed
slices' manifests and emits the merged `.entitlements` plist for the
signing pipeline — the same "payload declares its needs" philosophy as
spec 23's jail/bind model, reviewable in the repo, SSOT-honest.

## 2. The `signing:` block (spec 03 §2 / spec 23 §2 amendment)

Additive L1 manifest key (schema_minor bump; old readers ignore it, new
readers enforce). One spelling, one grammar — MECE:

```yaml
signing:
  macos:
    entitlements:
      - id: disable-library-validation
        why: "native extension .bundles materialize unsigned from the image"
      - id: allow-jit
        why: "YJIT (spec: TODO.yjit) — MAP_JIT on arm64"
```

- `id:` is drawn from the CLOSED vocabulary of §3 — an unknown id is a
  named manifest validation error (fail-closed; Apple accepts arbitrary
  entitlement keys, but a signing profile is a security contract and a
  typo must not silently widen it).
- `why:` is MANDATORY per entry (the same discipline as spec 23 §2's
  host needs).
- The block is inert on non-macOS targets (like `when: [macos]` host
  needs): recorded, never applied, never an error.

## 3. The entitlement vocabulary (closed, normative)

| id | Xcode key | when a slice declares it |
|---|---|---|
| `disable-library-validation` | `com.apple.security.cs.disable-library-validation` | the process dlopens unsigned or materialized code — **EVERY runtime slice** (gem `.bundle`s), and any package using spec 29's preload tier (the unsigned preload dylib) |
| `allow-jit` | `com.apple.security.cs.allow-jit` | a JITing runtime (MRI with YJIT; any future jit) — MAP_JIT on arm64 |
| `allow-unsigned-executable-memory` | `com.apple.security.cs.allow-unsigned-executable-memory` | a JIT without MAP_JIT support |
| `allow-dyld-environment-variables` | `com.apple.security.cs.allow-dyld-environment-variables` | spec 29's preload tier under hardened runtime (`DYLD_INSERT_LIBRARIES` is stripped without it) |
| `disable-executable-page-protection` | `com.apple.security.cs.disable-executable-page-protection` | last resort only; press emits a LOUD warning when the merged set contains it |

Derivation rule (normative): press ADDS `disable-library-validation` to
the merged set whenever the composition carries a runtime slice and no
slice declared it — with a named info line, never silently. All other
entries come only from explicit declaration.

## 4. The publisher pipeline (fat apps; the packed-mn reference)

Lean (shared-runtime) apps do **nothing** — they ship data slices; users
run the tebako org's signed bootstrap and runtime exes. Fat
(self-contained) apps:

```
tebako press …                      # unsigned stitched exe + merged .entitlements
codesign --sign "Developer ID Application: <publisher>" \
         --options runtime --timestamp \
         --entitlements <merged>.entitlements <stitched exe>
zip <stitched exe> && xcrun notarytool submit <zip> \
         --key <api.p8> --key-id … --issuer … --wait
```

- The publisher signs with **its own** Developer ID Application identity
  (metanorma/ribose for packed-mn), never the tebako org's. Secrets live
  in the publisher's repo.
- Notarytool latency (minutes) belongs in publish pipelines, never in
  press or in run paths.
- Stapling: bare exes cannot be stapled; notarize the distribution zip
  (Gatekeeper also resolves tickets online). A pkg/dmg wrapper for
  stapled offline UX is optional polish, docs-level.
- The composite action `tamatebako/tebako//actions/sign-notarize`
  packages exactly this sequence; the feedstock template inherits it.

## 5. The tebako-org pipeline (tebako + factory releases)

- **Sign-then-hash is mandatory.** Signing embeds a timestamp; digests
  must cover the final signed bytes. Every macOS leg signs and notarizes
  BEFORE the finalize job computes SHA256SUMS/manifest.json — the pin
  flow (factory contract.yml, feedstock versions.yaml mirrors) hashes
  signed bytes. A workflow that hashes unsigned bytes and signs after is
  a bug on arrival.
- Runtime exes (factory macOS legs) sign with the §3-derived runtime
  profile (`disable-library-validation` + `allow-jit` when the build has
  YJIT) before the `.sha256` sidecars are written.
- Store artifacts are signed **even though** §1.3's downloads are
  unquarantined: corporate/MDM profiles increasingly assess unquarantined
  exec too, and the signature is the durable identity statement.
- Gating: the signing steps run exactly when the repo variable
  `APPLE_SIGNING_ENABLED=true` is set; absent secrets with the gate on is
  a fast workflow failure with a named step error, never a partial
  release. Required secrets: the Developer ID Application certificate
  (p12 + password), the App Store Connect API key (.p8, key id, issuer
  id).

## 6. Acceptance (fail-closed, one CI tier each)

1. **The dylib canary** — a signed hardened-runtime runtime exe loads
   nokogiri's `.bundle` from a mounted image (dogfood covers it). Failure
   signature: `code signature … invalid` / dlopen refusal.
2. **The JIT canary** — with `allow-jit` merged, `RUBY_YJIT_ENABLE=1
   <signed runtime> -e 'abort unless RubyVM::YJIT.enabled?'` passes on
   arm64 (TODO.yjit/01's phase-0 probe, re-run against the signed exe).
3. **The notarization gate** — `spctl -a -vv -t execute <exe>` and
   `xcrun stapler validate <zip>` (where stapled) pass in the release
   workflow before SHA256SUMS is computed.
4. **Press validation** — `--platform macos-*` with a runtime slice and
   no `disable-library-validation` in the merged set after derivation is
   unreachable (§3 derives it); an unknown entitlement id fails the
   manifest validation with the named error; entitlement flags on
   non-macOS targets are a named press error.

## 7. The other platforms (comparison, non-normative)

Windows: Authenticode signing is a reputation input to SmartScreen, not a
server-blessed gate — nothing here applies; a future Authenticode story
would be its own spec riding the same sign-then-hash law. Linux: no
platform gate at all.

## 8. Landing order

1. This spec + the spec 23 §2 sibling note (docs-only PR).
2. The tpkg model + press merge + plist emission + §6.4 validation
   (product PR).
3. The tebako release workflow macOS signing legs behind
   `APPLE_SIGNING_ENABLED` (CI PR; secrets are the owner's task).
4. The factory macOS runtime signing leg (after the YJIT phase-0 PR —
   the entitlement profile depends on the YJIT verdict).
5. The packed-mn signing leg (publisher secrets — the metanorma org).
