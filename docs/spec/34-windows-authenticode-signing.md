# Spec 34 — Windows Authenticode signing

**Status: PLANNED (drafted 2026-09-09; the Azure-side procurement is in
flight — account created, identity validation submitted, OIDC app +
federated credentials + org secrets wired — the CI legs of §5 land per
§8).** Amends no wire format, no trailer, no manifest grammar, no spec 09
behavior — Authenticode is the OS platform trust plane; tebako payload
signing is untouched (§1.0). Spec 31 §7 names this document: "a future
Authenticode story would be its own spec riding the same sign-then-hash
law." Full design context: ecosystem TODO.v2-1/36; the forcing case is
the enterprise MSI (TODO.v2-1/35). Raised by the owner question
2026-09-05: "don't we need windows signing to have tebako work on
windows?" — answer: **no to run, yes to enterprise-deploy** (§0).

## 0. What Windows actually assesses

Windows has **no server-blessed application gate** (that is macOS's
Gatekeeper/notarization, spec 31 §0). Three distinct mechanisms matter:

1. **SmartScreen reputation** — triggered by the Mark of the Web (MOTW),
   which only opt-in downloaders (browsers) set. tebako's in-process
   HTTP client (ureq) never sets MOTW, so shim-dispatched runtimes and
   payloads are never SmartScreen-assessed at all. A browser-downloaded
   unsigned exe gets a reputation warning, not a refusal; reputation
   accrues **per certificate/publisher**.
2. **AppLocker / WDAC publisher rules** — the enterprise hard gate. A
   policy-locked host can refuse an unsigned binary outright. This is the
   forcing case: no signature, no execution, no warning.
3. **AV heuristics** — unsigned binaries that spawn processes and
   interpose syscalls (our preload machinery, spec 22) eat materially
   more false positives than signed ones.

The artifact plane splits exactly as spec 31 §0's does:

| artifact | kind | Authenticode | who signs |
|---|---|---|---|
| bootstrap exe | PE | yes — the user-facing download | the tebako org, once, at tebako release |
| tebako CLI tools, shim, launcher | PE | yes | the tebako org (tebako release) |
| runtime exe + runtime DLL (per engine × version × triplet) | PE | yes | the tebako org (factory release) |
| env `.tfs` image | **data** | no | — |
| payload `.tfs` slices | **data** | no | — |
| stitched self-contained package | PE (bootstrap + slots) | yes — **post-stitch only** (§1.2) | **the app publisher** |
| MSI envelope (TODO.v2-1/35) | PE (msi) | yes | **the app publisher** |

## 1. The execution-model consequences (normative)

### 1.0 Two signing planes, never conflated

Spec 09's OpenPGP trust (tebako-signer) anchors trust **inside** the
tebako ecosystem: who published this payload, may my machine load it.
Authenticode is the **OS platform plane**: may this PE execute on a
Windows host under SmartScreen/AppLocker/WDAC/AV scrutiny. Neither
substitutes for the other; a package may carry either, both, or neither.
This spec changes nothing about spec 09 (shipped: the release finalize
sign step, the trust anchor, the `.well-known` publication).

### 1.1 Payloads are data; Windows never sees them

The interpreter exe is signed once by the tebako org; every program it
ever runs is mounted data. Authenticode attaches to the *interpreter*,
never to the *program*: a payload update re-touches no Windows trust
machinery. The store download plane is unassessed by construction —
in-process fetches set no MOTW, and a signed exe's signature does not
propagate to files it writes (the same mechanism class as npm/pip/brew
shims; spec 31 §1.3's macOS twin).

### 1.2 Stitch-then-sign, never sign-then-stitch

Appending slots/trailer to a signed PE invalidates its Authenticode
signature (the hash covers the whole file modulo the signature's own
region — same law as Apple's CodeDirectory, spec 31 §1.2). Therefore
`tebako press` (self-contained mode) emits an **unsigned** stitched exe;
signing is a post-press step in the publisher's pipeline (§6). Press
never signs; no `--signtool` flag exists. (`--sign`, spec 09, is the
other plane.)

### 1.3 Sign-then-hash is mandatory

Signing embeds a timestamped countersignature; digests must cover the
final signed bytes. Every Windows leg signs BEFORE the finalize job
computes SHA256SUMS / manifest.json / `.sha256` sidecars — the pin flow
(factory contract.yml, feedstock versions.yaml mirrors) hashes signed
bytes. A workflow that hashes unsigned bytes and signs after is a bug on
arrival (spec 31 §5's law, restated for the Windows legs).

### 1.4 Timestamping is mandatory

The signature MUST carry an RFC 3161 countersignature (SHA-256 timestamp
digest). Artifact Signing's profile certificates are short-lived by
design; the timestamp countersignature is what makes the signature
verify after the signing cert expires or rotates. An unsigned-timestamp
signature is an acceptance failure (§7.2), not a warning.

## 2. The signing-technology decision (decision record)

**DECISION: Azure Artifact Signing** (formerly Trusted Signing), driven
from GitHub Actions with OIDC workload-identity federation — no stored
client secret, no hardware token.

| candidate | verdict | evidence |
|---|---|---|
| **Azure Artifact Signing** | **CHOSEN** | CI-native (`azure/login` OIDC + `Azure/artifact-signing-action`, Windows runners); certs chain to the Microsoft Trusted Root Program → baseline "known publisher" SmartScreen reputation from day one; timestamping built in (`http://timestamp.acs.microsoft.com`, §1.4); no secret material at rest (federated credentials scoped per repo × the `windows-signing` environment); Basic tier ≈ $9.99/mo per profile |
| OV certificate (+osslsigncode) | rejected | SmartScreen reputation builds from zero over months; cert custody/renewal on the org; osslsigncode covers OV from non-Windows legs but cannot speak to cloud-HSM-backed services — the Artifact Signing dlib wants signtool or the REST API regardless |
| EV certificate (HSM token) | rejected | instant reputation, but the hardware token must be physically attached to a runner — incompatible with GitHub-hosted CI without a self-hosted Windows runner holding the token; procurement weeks and cost for no capability Artifact Signing lacks |

Non-goals: Linux (no platform gate at all, spec 31 §7); package-level
signing (spec 09's plane); payload images (data, §1.1).

## 3. The trust inventory (normative names)

Org **secrets** (tamatebako org, visibility SELECTED — `tebako`,
`tebako-runtime-ruby`, `tebako-runtime-python`; wired 2026-09-09):

- `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID` — the
  OIDC identity of the Entra app registration. No `AZURE_CLIENT_SECRET`
  exists anywhere (federation replaces it).

Org **variables** (same repo set): `AZURE_ENDPOINT` (wired
2026-09-09), `AZURE_SIGNING_ACCOUNT_NAME`, `AZURE_SIGNING_CERT_PROFILE`.
Endpoint, account, and profile names are configuration, not credentials
— they identify public resources and appear in workflow logs; they live
as variables, not secrets. (This supersedes TODO.v2-1/36's draft
runbook, which listed them as secrets.)

Azure side: one Entra app registration holding the **Artifact Signing
Certificate Profile Signer** role on the account, with federated
credentials scoped per repository × entity **Environment**
`windows-signing`. The GitHub environment is the attestation boundary:
Azure tokens are issuable only to a job running in that environment of
that repo — environment protection rules tighten further without any
Azure change.

## 4. The gate (normative)

Signing legs run exactly when the repo variable
`WINDOWS_SIGNING_ENABLED=true` is set. Absent secrets/variables with the
gate on is a fast named step failure, never a partial release; unsigned
remains the default everywhere else (development branches, forked runs).
The gate flips per repo only when the secrets exist AND this spec's legs
have landed — the same discipline as `APPLE_SIGNING_ENABLED` (spec 31
§5). The parity gates compare unsigned bytes, so signing legs run AFTER
the gates' inputs are built and BEFORE the finalize hashes (§1.3).

## 5. The tebako-org pipeline (tebako + factory releases)

```
windows leg (windows-2022 runner):
  … build produces staged PE artifacts (unsigned) …
  - uses: azure/login@v2            # OIDC; environment: windows-signing
    with: { client-id, tenant-id, subscription-id }      # from §3 secrets
  - uses: Azure/artifact-signing-action@v0
    with:
      endpoint: ${{ vars.AZURE_ENDPOINT }}
      trusted-signing-account-name: ${{ vars.AZURE_SIGNING_ACCOUNT_NAME }}
      certificate-profile-name: ${{ vars.AZURE_SIGNING_CERT_PROFILE }}
      files-folder: <staging dir of PE artifacts>
      file-digest: SHA256
      timestamp-rfc3161: http://timestamp.acs.microsoft.com
      timestamp-digest: SHA256
  - verify: signtool verify /pa /v <each signed artifact>   # §7.1, §7.2
  … finalize: SHA256SUMS / manifest.json / .sha256 sidecars …
```

- **tebako release**: every PE asset of the release — the bootstrap, the
  CLIs (`tebako`, `tebako-pkg`, `tfs`), the shim, the runtime launcher —
  enumerated by extension (`.exe`, `.dll`), never by name list (a new
  tool binary must not ship unsigned by omission).
- **Factory releases** (tebako-runtime-ruby first; tebako-runtime-python
  when its Windows legs land): the runtime exe AND the runtime DLL (the
  ucrt split — WDAC/EDR check DLL signatures too).
- The signing job carries `permissions: id-token: write, contents: read`
  and `environment: windows-signing`; no other job gets `id-token`.

## 6. The publisher pipeline (fat apps + the MSI)

Lean apps do **nothing** — users run the tebako org's already-signed
bootstrap and runtime exes; the payload is data (§1.1). Fat
(self-contained) apps sign the stitched exe **post-stitch** (§1.2) with
the **publisher's own** Artifact Signing profile or EV/OV identity —
metanorma/ribose for packed-mn, never the tebako org's identity; secrets
live in the publisher's org. The MSI envelope (TODO.v2-1/35) signs in the
same step. The action shape is §5's, unchanged.

## 7. Acceptance (fail-closed, one CI tier each)

1. **The signature gate** — `signtool verify /pa` on every signed
   artifact in the release workflow, before SHA256SUMS is computed.
2. **The timestamp gate** — verification asserts the RFC 3161
   countersignature is present (§1.4); an untimestamped signature fails
   the leg.
3. **The enumeration gate** — the staged set is enumerated by extension;
   a PE artifact that reached finalize unsigned fails the release.
4. **The gate-off parity tier** — with `WINDOWS_SIGNING_ENABLED` unset
   the legs skip cleanly and the release ships unsigned exactly as today
   (no behavior change, proven by CI on the spec's own PR).
5. **The gate-on-fast-failure tier** — gate on with any §3
   secret/variable absent: named step error before any signing call.
6. **Reputation evidence (non-gating)** — SmartScreen/AppLocker behavior
   of the signed artifacts is documented over the first releases (the
   TODO.v2-1/35 policy-locked VM install is the publisher-side proof).

## 8. Landing order

1. This spec + the spec 31 §7 cross-reference (docs-only PR).
2. The tebako release Windows signing legs behind
   `WINDOWS_SIGNING_ENABLED` (CI PR) — after the owner's certificate
   profile exists (identity validation, §3).
3. The factory Windows runtime legs (tebako-runtime-ruby's ucrt64 legs
   first — they ship today; tebako-runtime-python's when the msys port
   lands).
4. The packed-mn publisher leg (its own org's profile; the MSI is the
   forcing case, TODO.v2-1/35).
