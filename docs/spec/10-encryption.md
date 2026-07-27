# Spec 10 — Encryption (confidentiality)

Normative specification of encrypted volumes: selective, in-memory-only
decryption. Status: v1 SHIPPED (roadmap 37: `EncBackend` +
`tfs encrypt`/`decrypt`/`mount`, SUITE-1; PQC suites, metadata hiding
and merkle verify-on-read remain PLANNED). Encryption is per-image
**opt-in**, never default.

## 1. Model

Encryption is a **stacking transform** like COW, not a format:
`EncBackend { base: dyn Backend, key_source }` — reads decrypt on demand,
in memory only; plaintext blocks never touch disk. Compose freely:
`Enc(COW(base))`, `COW(Enc(base))`, nested encapsulated encrypted images.
Per the transforms law (spec 00): `EncBackend` lives ONLY in the Rust TFS
— backends and dwarfs-t stay ignorant of it.

**Primitive:** OpenPGP via rnp-rs (AES-256/SHA-2; botan backend). Encrypt
AFTER compress (ciphertext doesn't compress; dwarfs blocks are
individually compressed → individually encryptable). Optional padding to
blur per-block sizes.

## 2. Key model (locked)

- **DEKs (data keys)** encrypt content blocks. A DEK IS a persisted
  OpenPGP session key — rnp's native wrap/unwrap (PKESK packets to N
  recipients) is the envelope; no custom crypto anywhere.
- **Identity lives at the PLAINTEXT layer.** Manifests carry TWO digests
  (spec 03 §2.1): the plaintext TREE hash (semantic identity — CAS
  addressing, dedup, sign/verify, audit-without-access) and the
  ciphertext blob hash (transport identity). Encryption is a per-audience
  transform; CAS never keys content-identity to any one wrapping.
- **Session keys are grant envelopes.** Sharing = wrapping the DEK to a
  recipient (ephemeral ECDH per OpenPGP) — offline, accountless,
  auditable. A wrapped DEK + manifest digest IS a capability: possessing
  it grants exactly that subtree, nothing else.
- **Revocation is prospective:** re-issue the manifest without the
  recipient (no new envelopes), or rotate the DEK for FUTURE versions
  (full re-encrypt, only on suspected DEK compromise). You cannot
  un-read what was read — documented honestly.
- **Hierarchical derivation:** subtree keys derive from a root DEK via
  HKDF with the path as info — K(/a/b) grants /a/b/** but not /a/c or /a
  (one-way). Sharing a directory subtree = handing over its derived key.
  Layered composites: per-layer DEKs for coarse audiences, HKDF subtrees
  for fine-grained ones.
- **Composition rules:** JOIN/squash of layers with different DEKs =
  authorized re-encryption under a new joined key, OR preserved per-layer
  key domains in the joined manifest (lazy join). Encapsulated images
  keep INDEPENDENT key domains — outer may be readable while inner stays
  sealed (transport vs payload audiences).
- **Selective disclosure:** per-directory/per-layer recipient mapping in
  the manifest → ONE artifact, N audiences; each key opens only its slice.
- **Metadata hiding option:** encrypt the metadata block as a unit —
  structure/filenames invisible until keyed.

## 3. Memory discipline

Decrypted blocks live in heap buffers that are `mlock`'d + zeroized on
free. Keys via agent / passphrase / PKCS#11 hardware — never written to
disk, never in swap. Confidential-computing memory (SGX/SEV/TDX) is a
documented future tier, not v1.

## 4. Serving

Prefer ciphertext end-to-end (clients decrypt locally; the server/CDN
never sees plaintext). Session wrapping per online client only when the
server is trusted with keys. Block-access patterns leak regardless (PIR
out of scope) — documented.

## 5. PQC readiness (locked)

- **Algorithm agility from day one:** every manifest/envelope/signature
  carries a CIPHER-SUITE id from a versioned registry — nothing
  hardcoded. Unknown ids fail with a named error, never a guess.
  - SUITE-1 = X25519 + Ed25519 + AES-256-GCM + SHA-256 (classical).
  - SUITE-2 = HYBRID: ML-KEM-1024+X25519 composite KEM, ML-DSA-87+Ed25519
    composite signatures, AES-256-GCM, SHA-384 — secure if EITHER family
    holds (the transition form).
  - SUITE-3 = SLH-DSA (hash-based signatures) maximal-conservatism tier.
- **HNDL:** long-lived encrypted images SHOULD use hybrid envelopes NOW.
  AES-256 keeps ~128-bit post-Grover strength; the merkle/hash layer is
  PQC-safe by construction. Signatures are the migration surface,
  envelopes the confidentiality surface — both suite-versioned, so
  migration is additive.
- Implementation path: botan 3 (rnp's backend) via rnp-rs PQC tracking +
  OpenPGP PQC drafts; recipients may hold classical AND PQC keys
  (multi-recipient envelopes are native).

## 6. Cyber/admin algebra (why this changes the game)

- Object security > channel security: artifacts self-protect over ANY
  transport/mirror (untrusted CDN, USB, email).
- Leak containment: disk snapshots/caches hold ciphertext only.
- **Crypto-shredding / kill-switch:** destroy a data key (or re-issue the
  manifest without a recipient) → content unrecoverable everywhere,
  provably — no mirror-purge problem.
- Immutable golden bases + auditable COW deltas (the overlay IS the
  change record) + signed provenance = compliance-grade fleet management.
- Honest risks: access-pattern side channels on serve; key-agent UX is
  the hard part; mlock/zeroize is the baseline, not a panacea.

## 7. CLI surface (SHIPPED v1)

`tfs encrypt / decrypt / mount --recipient / --key` — encryption
at image-creation time through the same Writer path; mount requires the
recipient key; wrong key → named `EKEY`-class error, never garbage.
SHIPPED (roadmap 37): `tfs encrypt` (root grant `--recipient`, subtree
grants `--subtree <path>=<pubkey>`; `--rewrap --key` rotates grants to a
new recipient set with the bulk ciphertext byte-identical),
`tfs decrypt` (plaintext to a tar stream — never a staging tree),
`tfs mount --key` (the unlock/grant report; FUSE/serve mounts remain
spec-11 §6 PLANNED). `tfs sign` is spec 09's surface. The v1 crypto
construction (per-block AES-256-GCM, HKDF-SHA256 subtree keys, PKESK
DEK envelopes, the `/__tpkg__/envelopes.yaml` grant manifest) is
documented normatively in `crates/tfs/src/backends_enc.rs` and
`crates/tpkg/src/envelope.rs`.
