# Registry witness key custody (spec 41 §4, tebako#1103)

The operational runbook for the transparency log's **witness key** — the
second signature on every log head that turns unilateral trust into
witnessed trust. One checkpoint, two keys, two orgs: a compromised or
coerced signer in ONE org can no longer fork the log alone.

This document owns the custody facts; spec 41 owns the wire format and
the verification rule; spec 09 owns the signature mechanism and the
trust store. Where those are silent here, they are silent by design —
never improvised around.

## 1. Who holds which key

| key | org | role | custody |
|-----|-----|------|---------|
| **head key** | `tamatebako` | signs each checkpoint (spec 41 §4's `sig`) | the registry's `signing:` block declares the public half (spec 09 §9.1); the secret half is the org's release signing subkey, held as a passphrase-protected export in the **tamatebako org's CI secrets vault** — the same custody `docs/root-ceremony.md` §7 wires for release signing |
| **witness key** | `tebako-packages` | countersigns each checkpoint (`wkey`/`wsig`) | the secret half lives ONLY in the **tebako-packages org's CI secrets vault**, as a passphrase-protected armored export; the org's registry release flow performs the countersignature |

The two keys never share a vault, an operator, or a machine. That
separation is the entire security property: whichever single org is
compromised, the other org's signature still fails to validate on a
forged checkpoint, and `registry log verify` names the forgery (exit
71). Adding a second signature inside ONE org would be ceremony, not
witnessing — the custody split is what makes the countersignature mean
anything.

**External invited witnesses are the end-state.** The first witness is
the tebako-packages org — the honest first step, because it is a real
second org with a real second vault, but still one the project owner
controls. Inviting an external organization's key as a third-party
witness is the documented direction; their multi-slot grammar is spec
41's own amendment when the first external witness joins, never a
guess in the current format.

## 2. The in-machine store shape (mirrors the head key's)

On any machine that produces or verifies checkpoints, both keys ride the
ONE documented store — the witness key gets no new store of its own:

- **Secret half** (producers only): `$TEBAKO_HOME/keys/<keyid>.key`,
  armored, mode `0600` on unix — the exact layout
  `tebako_signer::secret_key_by_keyid` reads for the head key. A
  producer-side machine holds the head key OR the witness key, never
  both: the registry operator's signer machine and the witness's
  countersigning machine are different machines in different orgs.
- **Public half** (every verifier): pinned as
  `$TEBAKO_HOME/trust/<FINGERPRINT>.pub`, one armored public key per
  file — spec 09 §9.1's pin layout, concatenated into the verification
  keyring beside the trusted keyring. The witness's fingerprint is
  published out of band (the tebako-packages org's `.github` repository
  and the well-known page, spec 09 §9's channels) so the pin is a TOFU
  confirmation, never a blind fetch.
- A checkpoint whose `wkey` is absent from the trust store verifies as
  `unknown-key` — reported, never upgraded to trusted (spec 41 §5).

## 3. The countersigning flow

1. The registry operator (tamatebako) appends the head, signs it with
   the head key, and stages the log for publication.
2. The witness flow (tebako-packages) reads the SAME canonical payload —
   `tebako-registry-log-1\n<seq>\n<ts>\n<root>\n<rows>\n<prev>\n`,
   byte-exact from the head record, never re-derived — verifies the
   primary signature and the `prev` chain first, then countersigns with
   the witness key and fills the `wkey`/`wsig` slot.
3. Publication happens only after the slot is filled. A checkpoint
   published without it is not wrong — `log verify` names it
   `single-signature` and the chain still reads sound — but it is the
   honest signal that the witness step did not run, and operators treat
   a sustained streak of single-signature heads as the witness flow's
   own alarm (the witness is down or declined to sign), not as the new
   normal.

A witness NEVER countersigns a head whose primary signature or chain it
cannot verify. Countersigning blind would invert the pattern: the
witness's "I saw this checkpoint" would mean "I checked nothing".

## 4. Rotation (successor and revocation)

Rotation follows spec 09 §8's shape — pre-made, rehearsed, never drafted
under attack:

- **Successor statement.** The witness key rotates by a successor
  statement (`TEBAKO-ROOT-SUCCESSOR-V1` format, the predecessor
  authorizing the successor), pre-made at key creation and stored with
  the private half in the tebako-packages vault. On rotation the new
  public key is re-pinned (the trust store entry is per fingerprint, so
  rotating leaves the old pin in place for verifying OLD heads and adds
  the new pin for new ones) and the change is announced out of band.
- **Old heads never re-sign.** A head countersigned by the old witness
  key stays valid under the old key's pin; rotation moves only NEW
  checkpoints onto the successor. The `prev` chain is unaffected —
  rotation is a trust-store event, not a log-format event, exactly like
  spec 41 §8's head-key rotation.
- **Revocation.** A compromised witness key is revoked by the pre-made
  self-revocation (`TEBAKO-ROOT-REVOCATION-V1`, `reason: compromise`),
  published to the same successors directory consumers apply (fail
  closed, `RevokedKey`, exit 72). Heads it honestly countersigned before
  the compromise are not evidence of anything new; heads it
  "countersigns" after are forgeries and verify as the 71 class.
- **The drill.** The rotation and revocation drills run on a scratch
  trust store before the first witnessed release and re-run on the
  orgs' ceremony cadence — the same rehearsal rule
  `docs/root-ceremony.md` §8 applies to the root: the worst day must be
  a publishing exercise, not a drafting exercise.

## 5. What never leaves the vault

The witness key's private half, its passphrase, the unpublished
successor statement, and the unpublished revocation. The fingerprint
and the armored public key are the only public artifacts. Exposure of
anything else is itself the incident the revocation exists for — and
because the head key lives in a DIFFERENT org's vault, even that
incident cannot fork the log: it can only stop the witnessing, which
the `single-signature` verdict names out loud.
