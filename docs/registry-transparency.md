# Registry transparency — proving a registry only ever grew

A signature on a registry tells you who wrote the bytes in front of you. It
cannot tell you what was there last month. Yanking a row and re-signing is
indistinguishable from the row never existing, and every mirror is happy to
serve the smaller file — which makes "the registry shrank a little" the one
attack nobody notices.

`tebako registry log` closes that gap with an RFC 6962 Merkle log: one
canonical row per line, a root over them, and a chain of signed heads that
only moves forward.

## The snapshot

```console
$ tebako registry snapshot tpkg-registry.yaml
app hello@1.0.0 aarch64-macos aaaa… active -
app hello@1.0.0 x86_64-linux-gnu aaaa… active -
app hello@2.0.0 universal - active -
snapshot: 3 row(s), root f2d1141e9fe986cafc207751d70145176521e764494ab77b7d8bad50951342cb
```

Every row of the registry is one line, sorted by the line itself — reindent
or re-comment the YAML and the snapshot does not move. The columns:

| column | where it comes from |
|---|---|
| `kind`, `name@version` | the document, verbatim |
| `platform` | `universal`, a vcpkg triplet (`x86_64-linux-gnu`), or `<variant-id>:<platform>` for a `variants:` arm |
| `sha256` | the row's own digest; `-` for a universal row, whose bytes are anchored by the release's `SHA256SUMS` and attestation |
| `state` | `active`, or `withdrawn` — the tombstone |
| `keyid` | the row's declared signer, or `-` |

`--out <path>` writes the lines beside the registry for publication;
`--json` gives the document with the `root`.

## The head

Each published snapshot is appended to a log as one signed head:

```
0 ts=1761000000 root=f2d1141e… rows=3 prev=- key=abcdef0123456789 sig=… wkey=0123456789abcdef wsig=…
```

`prev` is the previous head's root, so heads chain and the log can only
grow. A head whose `prev` does not match is a **fork**, named at the head
where it happens: nothing after it can be trusted.

`wkey`/`wsig` are the **countersignature slot** (tebako#1103): a second
key, held under a separate org's custody, signs the same checkpoint
payload the head key signed. Unilateral trust becomes witnessed trust —
forging a checkpoint now needs both orgs' keys. The witness for the
project's own registries is the tebako-packages org (tamatebako holds
the head key); the full custody runbook — which vault, and how the
witness key rotates — is `docs/registry-witness-custody.md`.

## Verifying

```console
$ tebako registry log verify tpkg-registry.log --registry tpkg-registry.yaml
tpkg-registry.log: 1 head(s), signature valid, witness witnessed
  the registry IS head 0: 3 rows, root f2d1141e…
log verify: sound
```

Both signatures are checked on every countersigned head, never one
instead of the other. A head sealed by the head key alone is **named a
single-signature checkpoint** on the verify line and in the JSON
document — the honest state of a pre-witness log, never silently read as
witnessed, and never a failure by itself. A countersignature that does
not validate is a forged second signature: it fails with the same
signature class as a forged primary (exit 71), named as the
COUNTERSIGNATURE, so a script can tell the two apart.

Compare the **root** and you get two answers at once: every row in the file
is in the tree, and no row was added or removed — because either would move
the root. When they disagree, you get the sentence that matters:

```
the registry is NOT what head 0 sealed: it holds 2 rows (root bbc2…)
but the head was sealed over 1 rows (root f67b…) — a row was added,
removed or edited after the seal
```

Exit codes separate the classes: `0` sound, `80` an integrity finding
(gap, fork, mismatch, a torn countersignature slot, a proof that does not
verify), **`71` a head whose signature or countersignature does not
validate** — a forged head is a different event from a broken chain.

## Proving one row

Sometimes you have one row and no registry:

```console
$ tebako registry log prove tpkg-registry.log --registry tpkg-registry.yaml --row hello@1.0.0
app hello@1.0.0 aarch64-macos …: 2 level(s), root f2d1141e…
app hello@1.0.0 x86_64-linux-gnu …: 2 level(s), root f2d1141e…
proof against the newest head root f2d1141e…: verified
```

`--verify-root <hex>` checks the proof against a root you obtained out of
band — the case that matters. A holder of one 32-byte value should never
have to trust the download that carried the row: the proof is `log2(n)`
hashes, and a third party can check it with the published algorithm.

## Withdrawal is the only way out

A row may leave a registry; it may not leave silently. `tebako registry
retire` sets `status: withdrawn` instead of deleting, so the row stays in
the snapshot forever and the tree keeps covering it. Deleting a row outright
still works with `--force`, and the next `log verify` reports the
disappearance by row name — a forced removal is exactly the alarm this log
exists to raise.

## Reference

- `tebako registry snapshot <path-or-url> [--json] [--out <path>]`
- `tebako registry log verify <log> [--registry <path>] [--json]`
- `tebako registry log prove <log> --registry <path> --row <name>@<version> [--json] [--verify-root <hex>]`
- Normative contract: `docs/spec/41-registry-transparency.md` and
  `docs/spec/schemas/registry-log.yaml`.
