# Spec 09 — Trust and signing (authentication)

Normative specification of integrity and authenticity for every binary
part: bootstrap, runtime payloads, data payloads, and release indexes.
Status: signing machinery SHIPPED (M29 phase 2); key ceremony/ops PARTIAL
(roadmap 10).

> **Rollout phase — unverified-first (roadmap 72).** The shipped
> tebako-bootstrap is built WITHOUT OpenPGP verification (the
> `openpgp-verify` cargo feature is off): signed packages execute with a
> loud UNVERIFIED warning + audit journal, unsigned packages with the
> legacy warning; `TEBAKO_REQUIRE_SIGNED=1` fails closed (exit 71)
> because a strict-mode request must never silently downgrade.
> OpenPGP verification returns via the crypto toolkit payload
> (`tebako-crypto`, rnp+botan full+PQC, fetched on demand and
> Ed25519-verified by the bootstrap) — this spec is the TARGET model;
> the phase-1 deltas live in spec 06 §3.

## 1. Threat model

Parts travel over mirrors/CDN and are shared between machines. HTTPS +
sha256 against a same-channel manifest protects the TRANSPORT, not the
OBJECT: whoever swaps the artifact swaps the manifest too. The tpkg
crc32 is integrity-vs-accident only — documented as non-authentic
(spec 02 §2). For the relationship to the OS-level trust gates
(Gatekeeper, Authenticode) — who validates the bootstrap versus what
tebako validates below it — see spec 12 §5.

## 2. The chain of trust

1. **Root of trust:** the tamatebako release signing keypair (OpenPGP via
   rnp-rs — the ONE signature mechanism for trailers, manifests, and
   release indexes alike: one keyring, one verify path). Root fingerprint
   published on tebako.org AND embedded in the loader/CLI at release time
   (`EMBEDDED_ROOT_FINGERPRINT`; dev override `TEBAKO_TRUSTED_ROOT`).
2. **Production (releases):** every released part (runtime payloads,
   bootstrap, src tarballs, libraries) ships a detached `.asc` per
   artifact PLUS a signed release manifest — one verify path for all.
3. **Packages:** the tpkg v2 extension (spec 02 §4) carries per-slot
   SHA-256 + signer keyid + the OpenPGP signature over the canonical
   trailer bytes.
4. **Rotation:** a signed successor-key statement
   (`TEBAKO-ROOT-SUCCESSOR-V1` — canonical lines binding predecessor →
   successor fingerprint, detached signature from the predecessor) lets
   any machine forward trust with no out-of-band step: the loader applies
   the statement chain from a trusted root and TOFU-registers the
   successor. Revocation procedure documented and rehearsed.

## 3. Opt-in rules (locked)

- Signing is **per-package opt-in**: `tebako-pkg bundle --sign[=keyid]`.
  Without it the package is plain v1, byte-identical to pre-signing — no
  key, no prompt, no ceremony. A press-local key is generated and cached
  under `$TEBAKO_HOME/keys` only on the first explicit `--sign`.
- Rewrite operations (insert-image/remove-image/set-runtime/reassemble)
  preserve the input's signing state.
- **Only the presence of a signature is optional — verification of signed
  packages is always strict** (exits 71/72, spec 06 §4).
- **v1-legacy rule:** unsigned v1 packages exist in the wild and stay
  runnable — accepted as LEGACY with a loud stderr warning + audit
  journal entry. `TEBAKO_REQUIRE_SIGNED=1` is the explicit opt-in
  hard-fail for hardened environments — never the default.
- **Fail closed everywhere:** no insecure-skip flag.
- Encryption is likewise per-image opt-in (spec 10).

## 4. Verification points

- **Press time:** the CLI verifies release signatures before using any
  part (fail closed). Trusted keyring in `$TEBAKO_HOME`; our root key
  embedded; additional keys TOFU-registered with a named prompt.
- **First run:** the loader verifies the trailer signature against the
  keyring, then each slot's sha256 before mounting/extracting (streaming,
  one pass at install time; the trust-anchor marker avoids re-hashing
  every run — spec 05 §4).
- **Runtime driver:** re-verifies an image's sha256 against the trailer
  before mount (integrity; authenticity already established by the
  trailer signature).

## 5. Release index authentication

`manifest.json` is signed (detached `.asc`); resolvers verify the index
signature before trusting its hashes — closing the gap where a MITM swaps
both package and unsigned manifest. Same keyring, same verify path.

## 6. Tooling

`tebako-pkg sign` (detached `.asc` per artifact + signed `SHA256SUMS`)
and `tebako-pkg verify` (Trusted/Untrusted/Invalid per artifact).
Factory release flows invoke `tebako-pkg sign` in CI (secrets-held
armored root export; the private key never leaves CI secrets/hardware).

## 7. Remaining ceremony (roadmap 10)

Production root key ceremony (offline, hardware-held) →
`EMBEDDED_ROOT_FINGERPRINT` filled → fingerprint published on tebako.org →
CI secrets wired → revocation drill rehearsed. The operational runbook —
dual-root (Ed25519 classical + ML-DSA-65 PQC), pre-made successor and
revocation statements, day-one ML-KEM encryption subkey, hardware
storage, and the rehearsal — is `docs/root-ceremony.md` (roadmap 36).

## 8. Revocation (locked 2026-07-27)

Rotation (successor statement) moves trust forward; REVOCATION kills a
key with no successor — for compromise or retirement:

```
-----BEGIN TEBAKO REVOCATION STATEMENT-----
format: TEBAKO-ROOT-REVOCATION-V1
fingerprint: <40-hex revoked root>
created: <unix seconds>
reason: compromise | retired | superseded
-----BEGIN PGP SIGNATURE-----
<detached signature from the REVOKED key itself (self-revocation)
 — or from the surviving PREDECESSOR root in a chain>
```

- A self-revocation is proof of possession; a predecessor-signed
  revocation covers a lost key (the predecessor authorized the chain).
- Consumers apply revocations from the successors dir alongside
  rotation statements: a revoked fingerprint is rejected even if it
  previously verified (fail-closed, named `RevokedKey` → exit 72).
- Revocations are additive files (`<fingerprint>.revoke.asc`), never
  edited history; the audit journal records every application.
- The ceremony doc (roadmap 36) includes rehearsing this once.

## 9. Third-party identity and key distribution

First-party slices verify against the embedded tamatebako root (§2).
Third-party authors are not tamatebako; their slices and fat/slim
binaries verify against THEIR OWN keys, and the question that decides
everything is how a user's machine learns the right fingerprint. The
method is the three-channel pattern — no channel alone is trusted, two
channels agreeing are the trust event:

1. **The registry (location-as-identity, primary channel).** A registry
   ref pins an authenticated location (`tfs:github:metanorma/metanorma`
   is GitHub's guarantee that this is the metanorma org). The registry
   index carries a `signing:` block at its head: armored public key,
   fingerprint, and an optional canonical key URL. `tebako add-registry`
   fetches the index, displays the fingerprint and the cross-check URLs,
   and asks for confirmation (TOFU). On confirmation the key is pinned
   to `$TEBAKO_HOME/trust/<fingerprint>.pub`, keyed by the registry ref.
   `--yes` non-interactive flows must supply the expected fingerprint out
   of band (env/config) — never a blind yes.
2. **Out-of-band confirmation (second channel, never auto-trusted).**
   Authors publish the same fingerprint where their audience already
   trusts them: `https://<author-domain>/.well-known/tebako-key.asc`
   (the well-known convention), their docs/README, and for GitHub-hosted
   projects the org's `.github` repository (zero new infrastructure).
   The CLI prints these URLs for the user to compare; it never upgrades
   trust from the second channel silently.
3. **Fingerprint-addressed directories (no trust needed).** OpenPGP
   keyservers are used as directories: fetch BY fingerprint, authenticate
   the fetched key against the channels above. A swapped key has the
   wrong fingerprint by construction, so the directory itself needs no
   trust.

**Continuity is the security property.** After the first confirmation,
every artifact from that identity must verify against the pinned key.
A key mismatch fails closed (`SignerKeyChanged`, exit 72) and displays
both fingerprints; a valid successor chain (§8 rotation) forwards
automatically after displaying the chain proof, re-pinning the new key.

**Per artifact class:**

- Registry index: verified against the pinned registry key on every
  refresh.
- Payload slices: signed trailer verified against the author's pinned
  key; per-slot digests chain off it (§4).
- Runtime slices (tamatebako): the embedded root — nothing to register.
- Third-party fat/slim binaries: verified at install/import against the
  pinned author key (the same verify path); the author may ALSO
  OS-codesign with their own Developer ID — Apple/Microsoft gates then
  vouch at execution time, orthogonally and complementarily (spec 12 §5).

**Developer ergonomics (target UX; status in braces).** `tebako keygen`
creates a keypair in `$TEBAKO_HOME/keys/` and prints the fingerprint
with the exact text to paste into docs + the well-known location
[partial: key creation exists]; `tebako press` signs automatically when
a default key exists, `--no-sign` to opt out [partial]; `tebako publish`
signs every artifact + the index and writes the `signing:` block into
the registry index automatically [shipped in install-UX]; `tebako key
rotate` creates + signs + publishes a successor statement in one command
[planned]; revocation follows §8 [format locked].

**User ergonomics.** First-party: zero interaction, ever (embedded
root). Third-party: one informed consent per author (the TOFU prompt),
then silent verification forever. `tebako trust list|show|remove`
manage the trust store [planned]; `tebako doctor` re-verifies installed
artifacts against pinned keys [roadmap 50]. Unverified artifacts always
produce the loud warning + journal (§3); `TEBAKO_REQUIRE_SIGNED=1`
refuses them outright.
