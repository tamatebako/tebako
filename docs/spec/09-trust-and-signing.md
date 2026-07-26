# Spec 09 — Trust and signing (authentication)

Normative specification of integrity and authenticity for every binary
part: bootstrap, runtime payloads, data payloads, and release indexes.
Status: signing machinery SHIPPED (M29 phase 2); key ceremony/ops PARTIAL
(roadmap 10).

## 1. Threat model

Parts travel over mirrors/CDN and are shared between machines. HTTPS +
sha256 against a same-channel manifest protects the TRANSPORT, not the
OBJECT: whoever swaps the artifact swaps the manifest too. The tpkg
crc32 is integrity-vs-accident only — documented as non-authentic
(spec 02 §2).

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
CI secrets wired → revocation drill rehearsed.
