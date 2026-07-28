# Tebako root key ceremony (roadmap 36)

The operational runbook for creating the tamatebako root of trust —
the expanded procedure behind spec 09 §7. Executed **once**, offline,
by the project owner, before signed releases ship. Everything here is
deliberate, rehearsed, and witnessed; nothing is improvised.

The ceremony produces, in order:

1. The **classical root** — Ed25519 OpenPGP keypair (the day-one anchor).
2. The **PQC root** — ML-DSA-65 OpenPGP keypair (generated the same day;
   the harvest-now-decrypt-later and Shor-class threats are countered by
   having the successor exist BEFORE it is needed).
3. The **successor statement** classical → PQC (spec 09 §2 rotation
   format `TEBAKO-ROOT-SUCCESSOR-V1`, signed by the classical root) —
   pre-made, stored offline, unpublished. Quantum or cryptanalytic
   failure of Ed25519 becomes a publish operation, not a crisis.
4. **Per-key revocation certificates** (spec 09 §8 self-revocation
   format `TEBAKO-ROOT-REVOCATION-V1`) — pre-made for BOTH roots.
5. The **ML-KEM-768 encryption subkey** on the classical root (spec 10;
   encryption exists from day one, so an encrypted image made in year
   one never requires a key invented in year three).
6. The **public artifacts**: armored public keys + fingerprints, wired
   into `EMBEDDED_ROOT_FINGERPRINT`, published on tebako.org, and the
   CI secrets that let factories sign without the private keys ever
   leaving hardware.

Terminology: rnp (via the tebako-crypto toolkit, full botan with PQC)
is the ONE signature mechanism (spec 09 §2). All commands below are
illustrative rnp CLI invocations; the toolkit payload is the supported
environment for running them.

## 1. Participants and environment

- **Owner** (executes), **witness** (observes and countersigns the
  ceremony log — one trusted person minimum).
- An **air-gapped machine**: freshly booted from trusted read-only
  media, network interfaces disabled or physically absent, no
  persistence. Work in a ramdisk/tmpfs; nothing touches disk.
- **Two hardware tokens** minimum (classical root lives on one, PQC
  root on the other — different vendors preferred, so a firmware
  compromise cannot take both).
- **Two offline backup media** (encrypted USB or printed armored
  exports in tamper-evident storage), stored in physically separate
  locations.
- The ceremony log: paper. Every step, every fingerprint, every
  command outcome is written down and countersigned by the witness.

## 2. Pre-flight checklist

- [ ] tebako-crypto toolkit payload fetched in advance and its sha256
      verified against the release index on a SECOND machine (the
      toolkit itself must be authentic before it may mint the root).
- [ ] Toolkit + verified checksum transferred to the air-gap via
      read-only media.
- [ ] ramdisk workspace: `mkdir /tmp/ceremony && cd /tmp/ceremony`
      (tmpfs, never swap-backed).
- [ ] `umask 077`; confirm no network: any lookup must fail.
- [ ] Entropy sanity: generate a throwaway key, inspect it, discard.

## 3. Generate the roots

Classical (Ed25519, day-one anchor):

```
rnp --generate-key --userid "tamatebako root (classical) <root@tebako.org>" \
    --expert   # primary: eddsa/ed25519; NO encryption subkey here
```

PQC (ML-DSA-65, quantum-safe successor):

```
rnp --generate-key --userid "tamatebako root (pqc) <root@tebako.org>" \
    --expert   # primary: mldsa65/ed25519 composite per the crypto-refresh
```

Record both 40-hex fingerprints in the ceremony log, twice, from two
independent read-backs (`rnp --list-keys --with-sigs` and an armored
export re-import on the second machine — the fingerprints must agree
across implementations of the read path, not just across commands).

## 4. The encryption subkey (ML-KEM-768, day one)

```
rnp --edit-key <classical-fp>   # add subkey: mlkem768/x25519 composite
```

spec 10's encrypted images require a recipient key that exists from the
start. Adding it later would split history into pre/post eras; adding
it now costs nothing.

## 5. Pre-make the succession and the revocations

Successor statement (classical authorizes PQC), spec 09 §2 format:

```
-----BEGIN TEBAKO SUCCESSOR STATEMENT-----
format: TEBAKO-ROOT-SUCCESSOR-V1
predecessor: <classical-fp>
successor: <pqc-fp>
created: <unix seconds>
-----BEGIN PGP SIGNATURE-----
<detached signature by the CLASSICAL root>
```

Self-revocations (spec 09 §8 format `TEBAKO-ROOT-REVOCATION-V1`,
`reason: compromise`), one per root, each signed by the key it revokes.

Store all three documents with the private keys. Publish NONE of them.
They exist so that the two worst days (cryptographic break, key
compromise) are publishing exercises executed calmly, not drafting
exercises executed under attack.

## 6. Move the keys to hardware; make the backups

- Classical private key → hardware token A; PQC private key → token B.
- Armored exports (private, passphrase-protected) → backup media ×2,
  geographically separated.
- After transfer: secure-delete the ramdisk copies, power off the
  air-gap (tmpfs dies with it), witness countersigns the log.

The private keys now exist ONLY on the tokens and the two backups.
The passphrases exist ONLY in the owner's head and one sealed physical
envelope.

## 7. Wire the public half (online machine)

1. `crates/tebako-bootstrap/src/lib.rs`:
   `EMBEDDED_ROOT_FINGERPRINT = "<CLASSICAL-FP>"` (the constant is
   empty until this moment — empty means dev/unsigned-first mode).
2. tebako.org: publish both fingerprints, the classical armored public
   key, and the policy text ("releases verify against the classical
   root; the PQC root is the announced successor class").
3. GitHub org `.github` repo + keyservers: the armored public key
   (fingerprint-addressed distribution, spec 09 §9 channel 3).
4. CI secrets: armored, passphrase-protected export of a **signing
   subkey** (NOT the root primary) — factories sign releases with the
   subkey; the root primary never leaves the tokens. Compromise of CI
   then costs a subkey revocation, not the root.

## 8. Rehearse (before the first signed release)

On a test machine with `TEBAKO_TRUSTED_ROOT` pointing at a scratch
keyring:

- [ ] Sign a scratch artifact with the subkey; verify it resolves to
      the embedded root (the chain path, spec 09 §4).
- [ ] Publish the pre-made PQC successor statement into the successors
      dir of the scratch keyring; confirm trust forwards
      (TOFU-register, audit journal entry).
- [ ] Publish a pre-made revocation; confirm the revoked fingerprint
      is rejected (`RevokedKey`, exit 72) even on previously-valid
      artifacts.
- [ ] Restore the scratch keyring; log the drill in the ceremony log.

Revocation/rotation must be muscle memory BEFORE they are needed —
spec 09 §8 makes this drill part of the ceremony, not an appendix.

## 9. What never leaves the room

Root private keys (token-bound), the passphrases, the unpublished
successor statement, the unpublished revocations, the ceremony log's
private-key sections. The fingerprints and public keys are the ONLY
ceremony outputs that are public — everything else is a secret whose
exposure is itself the incident the revocations exist for.
