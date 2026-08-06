# Spec 21 — Crypto consolidation: one crypto home per layer

Normative decision on how tebako's OpenPGP trust/encryption stack
(spec 09, spec 10) and LimniFS's format-native cryptography (spec 20 §7)
divide the work. **Status: PROPOSED DECISION** — §§1–3 assess the option
set, §4 recommends; the owner's ratification locks §4 (the ratification
PR updates this status line and applies the §5/§6 deltas in place).
Spec 20 §7's two crypto hooks ("Merkle + Ed25519 ↔ spec 09", "ENC ↔
spec 10") are DECIDED here; spec 20 recorded the shape, this spec makes
it normative.

## 1. The two stacks

**tebako (shipped).** OpenPGP via rnp-rs (vendored botan: librnp +
Botan + json-c + zlib + bzip2 from source, PQC-capable) is the ONE
signature mechanism (spec 09 §2) — trailers, manifests, release
indexes, third-party identity, the root ceremony — and the ENC
transform's envelope mechanism (spec 10 §2: PKESK DEK wrap/unwrap,
per-block AES-256-GCM, HKDF-SHA256 subtree keys). Posture: signing
machinery SHIPPED, unverified-first rollout (spec 09 preamble);
ENC SHIPPED at SUITE-1; PQC suites PLANNED with the construction locked
(spec 10 §5) and the dual-root ceremony rehearsed
(`docs/root-ceremony.md`). The bootstrap carries ZERO rnp/botan
(`openpgp-verify` off); OpenPGP verification returns via the
`tebako-crypto` toolkit slice, Ed25519-verified by the bootstrap
(roadmap 72).

**limnifs (shipped, format-native).** Per-drop AEAD by id registry —
0x01 XChaCha20-Poly1305 (mandatory baseline), 0x02 AES-256-GCM, 0x03
AES-256-OCB (in-house `limnifs-ocb3` on the `aes` crate) — with
deterministic nonces derived per (image key, slab, position) and
associated data binding every sealed drop to the image. Ed25519 over
the BLAKE3 `ManifestRoot` — the Merkle root commits the whole image —
behind the `signing` feature (ed25519-dalek; keypair mode, sigstore
deferred). A per-recipient wrap of the image master key behind the
`key-wrap` feature: the cryptographic SHAPE of HPKE Base mode
(DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, XChaCha20-Poly1305) — a
documented RFC 9180 SUBSET by its own authors, not wire-compatible
HPKE. Shamir k-of-n escrow over in-house GF(2^8) (DMS). Pure Rust, no
system deps, classical only: no ML-KEM, no ML-DSA, no OpenPGP.

One design convergence is real and recorded: limnifs's identity rule
(`DropId = BLAKE3(plaintext)`; encryption is a representation) IS spec
10 §2's "identity lives at the plaintext layer". ENC-over-limnifs
preserves limnifs dedup semantics unchanged.

## 2. The role map (four roles, one home each)

- **(a) Image-level sign/encrypt of payload bytes.** tebako: the tpkg
  trailer signature covers the whole slot (per-slot SHA-256 + the
  OpenPGP trailer signature, spec 09 §2) and ENC stacks over ANY
  backend (transforms law), limnifs included — backend-neutral.
  limnifs: image-internal signature/AEAD, free with the format, but a
  SECOND mechanism answering the same question at the same layer.
- **(b) Runtime/bootstrap artifact signing.** Release artifacts are
  exes, tarballs, and `.tfs` of ANY backend — not limnifs images.
  Only detached OpenPGP (`.asc` + signed `SHA256SUMS`, spec 09 §6)
  serves this. limnifs crypto cannot, by construction.
- **(c) Trust anchors and third-party identity.** The chain of trust,
  fingerprints, the TOFU three-channel pattern, successor/revocation
  statements, `$TEBAKO_HOME/trust` (spec 09 §§2, 8, 9) are
  OpenPGP-key-shaped, locked, and partially shipped. Raw Ed25519 keys
  carry no certification, uid, or keyserver-directory story; rebuilding
  §9 on them is a downgrade, not a consolidation.
- **(d) PQC (owner requirement: MUST).** Only the rnp/botan side serves
  it today: botan 3's ML-KEM/ML-DSA under spec 10 SUITE-2/3, the PQC
  root pre-made at the ceremony. limnifs's KEM/signature pair
  (X25519, Ed25519) is Shor-vulnerable end to end; its 256-bit
  symmetric layer is Grover-fine by construction.

## 3. The options, assessed

Criteria: the < 3 MB bootstrap gate (spec 00 invariant 2); vendored-
botan weight; slice-author UX (how many key types one author manages);
the spec 09 keyring/trust store; MECE — one crypto home per layer,
never two overlapping.

**A — limnifs-native at the image layer, rnp for the rest.** Two crypto
homes at ONE layer: two key types per slice author, two envelope
formats, two verify paths in the driver, and the in-house HPKE subset
and GF(2^8) Shamir become tebako's audit burden. PQC for limnifs-sealed
images stays impossible until limnifs grows it — a live conflict with
(d). REJECTED on MECE and (d).

**B — all-rnp; limnifs crypto unused.** Clean: one home, PQC path
intact, spec 20's `ENOTSUP`-on-`aead ≠ 0` posture unchanged. But it
leaves the boundary undocumented — the next integration proposal
re-litigates the question — and gives no named answer to the one case
where an in-image signature is genuinely additive: a BARE image moving
outside the registry channel, where the spec 05 sidecar/index anchor
does not travel with the bytes. INSUFFICIENT, not wrong.

**C — limnifs-native everywhere; limnifs grows PQC later.** Bulldozes
the locked, partially shipped spec 09 model (fingerprints, TOFU
channels, rotation/revocation, the rehearsed ceremony) for a less
expressive identity system; bets a MUST-level requirement on an
unscheduled, unaudited in-house PQC build-out in another project; and
inverts the dependency direction — tebako's trust architecture hosted
on a young format's crypto roadmap. REJECTED on (b), (c), (d).

**D — hybrid by layer, migration path documented.** One home per layer,
drawn once: L3 identity/trust = OpenPGP; L1 confidentiality = the ENC
transform; format-native crypto = the format's own business, consumed
as EVIDENCE, never as anchor. Day-one posture equals B; the seams where
limnifs crypto could later earn a role are named and gated (§4.3, §7).
RECOMMENDED.

## 4. The recommendation (D) — rules

Ratification locks this section.

### 4.1 Trust and identity: OpenPGP only (unchanged)

- The tpkg trailer signature + signed release indexes + the spec 09
  trust store are the ONLY authenticity decision points, for every
  backend including limnifs. rnp/botan stays the one signature
  mechanism (spec 09 §2); the dual-root ceremony and the SUITE-2/3
  roadmap proceed as specified.
- limnifs Ed25519 keys NEVER enter `$TEBAKO_HOME/keys` or
  `$TEBAKO_HOME/trust`. A slice author manages exactly ONE keypair —
  their OpenPGP key; the spec 09 §9 UX is untouched.

### 4.2 Confidentiality: the ENC transform only (unchanged)

- ENC stacks over limnifs exactly as over any backend (transforms law).
  tebako-pressed limnifs images are plaintext at the AEAD layer
  (`aead = 0`); encryption, when opted in, is the spec 10 transform.
- Mounting a self-encrypted limnifs image (`aead ≠ 0`) fails the named
  `ENOTSUP` (spec 20 §4) — re-affirmed as the v2.x posture, not a
  stopgap. The limnifs envelope format is NOT a DEK grant source in
  v2.x; admission requires a spec 10 addendum and ALL of §7's gates.

### 4.3 Format-native crypto as evidence (the narrow opening)

- A limnifs image-internal `SignatureBundle`, when present in a mounted
  image, MAY be verified as non-authoritative integrity evidence — only
  AFTER the spec 09 chain has decided authenticity, and only against a
  key the OpenPGP-signed metadata binds to the author identity (one
  identity, two signatures, one keyring). Absence is never an error;
  mismatch is a loud warning + journal entry, never the trust decision
  — byte identity is already the trailer digest's job (spec 09 §4).
- The binding mechanism (an author-signed statement carrying their
  limnifs image-signing pubkey) is a spec 09 addendum — PLANNED, NOT
  COMMITTED. Until it lands, tebako ignores embedded bundles.

### 4.4 The recommendation against the five criteria

- **Bootstrap gate:** nothing reaches the loader — it never carries
  rnp/botan (spec 09 preamble) and never mounts (spec 20 §1), so no
  format crypto approaches it. The roadmap-72 toolkit check (a small
  Ed25519 verify in the bootstrap) is the same primitive class an
  evidence verifier would use; sharing it is a toolkit-era decision,
  not this spec's.
- **Vendored-botan weight:** unchanged from shipped reality —
  `tebako-signer`'s consumers (`tebako-pkg`, `tfs-cli`, the `tfs` `enc`
  feature) plus, via `tebako-driver` → `tfs/enc`, every non-Windows
  runtime exe, plus the `tebako-crypto` toolkit slice (one build to
  audit, `docs/blocks/toolkit-slice.md`). The pure-Rust runtime line
  (spec 20 §5) removes the C++ OFFS toolchain but NOT botan while ENC
  ships in the driver — recorded as the concrete prize if §7's gates
  are ever met. Honest cost: Windows ENC stays dark while rnp's mingw
  build is unproven (TODO.v2-1/08) — the strongest future argument for
  the §7 envelope gate, not a reason to admit it early.
- **Author UX / trust store:** one key type, one keyring, one verify
  path — §4.1.
- **MECE:** every crypto question has exactly one home: WHO signed it →
  spec 09; WHO may read it → spec 10; HOW the bytes are sealed → the
  format. The third never answers the first two.

## 5. Spec 09 amendments (deltas — applied at ratification)

- §2 (chain of trust): append — image-internal format-native signatures
  (e.g. a limnifs Ed25519 bundle over the `ManifestRoot`) are NEVER
  trust anchors; the trailer/index chain is the only authenticity
  decision point.
- §4 (verification points): append — an embedded bundle MAY be verified
  as non-authoritative evidence post-chain, against an OpenPGP-bound
  key only; absence is not an error; mismatch warns + journals
  (spec 21 §4.3).
- §9 (third-party identity): append — one OpenPGP keypair per author
  identity; format-native keys are not registered, pinned, or
  distributed through the trust store.

## 6. Spec 10 amendments (deltas — applied at ratification)

- §1 (model): append — ENC is the ONLY tebako decryption path;
  self-encrypted limnifs images (`aead ≠ 0`) are not a grant source and
  fail `ENOTSUP` at mount (spec 20 §4, spec 21 §4.2).
- §2 (key model): append — the limnifs HPKE-style envelope is NOT a DEK
  grant format in v2.x; admission = a spec 10 addendum + the spec 21 §7
  gates.
- §5 (PQC readiness): append — any admitted format-native envelope or
  signature enters as NEW suite ids in the same versioned registry;
  hybrid (ML-KEM+X25519, ML-DSA+Ed25519) is the minimum tier for
  long-lived images, per HNDL.
- Key management: Shamir k-of-n escrow (limnifs DMS) is OUT OF SCOPE
  for tebako keys; backup/escrow follows `docs/root-ceremony.md`.
  Recorded as a possible future grant-escrow mechanism, not committed.

## 7. The PQC gate list (what limnifs would need)

Admission of ANY limnifs-native crypto into tebako's trust or
confidentiality paths requires all of:

1. **ML-DSA (FIPS 204) signing** — e.g. the pure-Rust RustCrypto
   `ml-dsa` crate — as a new `SignMode` + suite id. The
   `SignatureBundle` must size-flex: ML-DSA-65 signatures are ~3.3 KB
   against Ed25519's 64 B. Composite ML-DSA+Ed25519 per spec 10
   SUITE-2's either-holds shape.
2. **ML-KEM (FIPS 203) key establishment** — e.g. the pure-Rust
   RustCrypto `ml-kem` crate — behind an EXPLICIT KEM id in the
   envelope format (today's X25519 is implicit), in hybrid
   X25519+ML-KEM composite form (the SUITE-2 shape).
3. **Registry agility made explicit.** AEAD ids are already versioned
   (0x01–0x03 plus the 0xFF extended sentinel; 256-bit symmetric needs
   no change under Grover). Envelope KEM ids and `SignMode` ids need
   the same treatment so suites arrive additively, never as a wire
   break.
4. **Audit posture.** tebako consumes third-party crypto as audited
   crates or via the toolkit slice. The in-house compositions (the
   RFC 9180 SUBSET, the GF(2^8) Shamir) may stay format-internal;
   entering tebako's trust path wants full RFC 9180 conformance (e.g.
   a vetted `hpke` crate) or equivalent external review with published
   test vectors.
5. **Escrow specified for hybrid keys.** Shamir over shares is
   byte-generic — the math holds — but splitting COMPOSITE keys needs a
   documented construction before it is a grant-escrow candidate.

## 8. Evidence and acceptance

- This spec decides design only. Its ratification implies no code
  change beyond the §5/§6 delta edits; implementation work, if any,
  enters through the normal spec-14 order.
- Done means (this PR): this file + the 00-INDEX registration. The
  §5/§6 deltas and the status-line flip land in the ratification PR.
- Merge-order note: spec 20 is PR 369. This spec references spec 20 by
  name and does NOT edit its file while 369 is open; when 369 merges,
  its §7 crypto hooks are updated to point here ("Recorded" →
  "Decided in spec 21").
- Named-error and exit-code surfaces are unchanged: `ENOTSUP` for
  `aead ≠ 0` stays spec 20's; exits 71/72 stay spec 06 §4's.
