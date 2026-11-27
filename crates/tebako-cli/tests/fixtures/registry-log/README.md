# Golden registry-log verification transcripts (spec 41, tebako#1103)

One directory per case. `tests/registry_log_transcript.rs` runs the real
`tebako registry log verify` binary against a staged copy of the case and
pins its stdout transcript byte-for-byte against `expected.txt`, plus the
exit code in `exit.txt`. The CLI's version banner is preamble and is NOT
part of the contract (a version bump never touches these files).

Files per case:

| File            | Meaning                                                   |
|-----------------|-----------------------------------------------------------|
| `registry.yaml` | the document the checkpoint seals                         |
| `registry.log`  | the checkpoint(s) under test                              |
| `expected.txt`  | the exact verification transcript (stdout, banner aside)  |
| `exit.txt`      | 0 sound / 71 the forged-countersignature signature class  |

The cases are the spec-41 §5 acceptance shapes:

- `witnessed` — two heads, each carrying two valid signatures: the head
  key's (`key`/`sig`) and the witness key's (`wkey`/`wsig`). Verification
  passes with `witness witnessed`.
- `single-signature` — the head key alone sealed the checkpoint. Named a
  SINGLE-SIGNATURE checkpoint on the verify line, never silently read as
  witnessed, never a failure by itself (exit 0).
- `forged-witness` — the checkpoint was edited after witnessing and the
  compromised head key re-signed over the edit; the witness seal still
  covers the checkpoint as it was. The COUNTERSIGNATURE finding names the
  forgery (exit 71) while the primary still validates — the second org's
  key is exactly what stops the fork from reading as clean.

`keys/` holds the two custodies, honestly: `primary.*.asc` minted in one
home (the tamatebako stand-in), `witness.*.asc` in another (the
tebako-packages stand-in) — see docs/registry-witness-custody.md. These
are TEST-ONLY keys checked in as fixtures (the same precedent as
crates/tebako-signer's fixture keys); they sign nothing outside this
test tree.
