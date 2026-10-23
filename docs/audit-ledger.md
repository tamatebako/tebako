# The audit ledger — what ran here, and how to prove it

Every tebako process leaves a record. The loader writes what it fetched
and verified; the driver writes what it mounted and what the jail refused;
the shim writes what it dispatched. All of it lands in one file:

```
~/.tebako/journal.log        ($TEBAKO_JAIL_JOURNAL overrides the path)
```

`tebako audit` reads it — and can anchor it so nobody can quietly change
it later. This page is how to use both halves.

## Reading the ledger

No filters means the whole file, verbatim, in file order:

```console
$ tebako audit
1761000000 event=fetch origin=https://…/mn.tfs sha256=abc123
1761000001 event=runtime-fetch-verified sha256=abc123 verdict=ok
1761000002 event=composition mount=/app slot=0
1761000004 event=payload-installed name=metanorma version=1.17.0
4 of 4 record(s) in /home/u/.tebako/journal.log
```

The records on stdout are the file's own bytes, so you can pipe them
somewhere without losing anything; the `N of M` line goes to stderr.

Filters compose. This is the useful one when you are auditing a download:

```console
$ tebako audit --event fetch,runtime-fetch-verified,payload-signature-trusted --since 2026-10-20T00:00:00Z
```

| flag | what it keeps |
|------|---------------|
| `--since <t>` / `--until <t>` | unix seconds or `YYYY-MM-DDTHH:MM:SSZ` (UTC). Local time is never guessed |
| `--event <a,b,c>` | exactly these event names; repeatable |
| `--artifact <needle>` | records whose body contains the needle — a payload name, a version, a sha256, a URL fragment |
| `--tail <n>` | the last `n` matches, in file order |

`--count` prints just the number. `--json` gives a stable document
(`audit_schema: 1`) on stdout alone, with the records, their parsed
fields, the total the file holds, and any damage:

```console
$ tebako audit --event jail-deny --json | jq '.matched, .records[0].fields.path'
```

`--strict` turns damage into a failure (exit 80). Without it, damage is
reported but a query still succeeds — an operator diagnosing a host should
see everything, including the lines nothing could parse.

**A query writes nothing.** It does not stamp the store layout, install
network config, or append to the journal. You can run it on a host whose
`~/.tebako` was written by an older tebako.

## Anchoring it

An append-only file with no anchor proves nothing: anyone can delete a
run's records and the file still looks like a ledger. The anchor is a
chained digest stored beside the journal:

```console
$ tebako audit --seal
sealed 812 record(s) of /home/u/.tebako/journal.log at 1761000123 — head 1f0c… (signer abcdef0123456789)
$ tebako audit --verify
/home/u/.tebako/journal.log: sealed
  the chain matches the signed seal of 1761000123 over 812 record(s) (signer abcdef0123456789)
  seal 1761000123 covers 812 record(s); the journal holds 815 now; 3 unsealed
  signature: valid
```

- Each seal covers the whole file as it stands. Later records are the
  "unsealed tail" and are reported as such — never hidden.
- The chain is `h_i = SHA-256(h_{i-1} ‖ line_i)` over exact bytes, so any
  edit, deletion or insertion shifts every later digest.
- The seal is signed with this machine's press-local key, and that key is
  registered in the store's own keyring first: the machine verifies what
  it wrote. `TEBAKO_SIGN=0` gives you an UNSIGNED seal — still useful,
  and it says out loud that it detects edits, not forgery.

If something changed, `--verify` names the interval instead of shrugging:

```console
$ tebako audit --verify
/home/u/.tebako/journal.log: tampered
  records 411–812 were altered: the seal of 1761000123 covers 812 record(s) and no longer matches (no earlier seal narrows it further)
  seal 1761000123 covers 812 record(s); the journal holds 812 now; 0 unsealed
  signature: valid
Tebako script failed: … [80]
```

Two more cases worth knowing: a seal covering more lines than the file
holds reports records **GONE**; and a valid chain under a seal whose
signature does not validate reports a tamper — a re-seal is exactly what
forgery looks like, so a matching chain never overrides a bad signature.

## The auditor's walk

Someone with no tebako knowledge, offline, with only public artifacts:

1. `tebako audit --json > ledger.json` — the host's decisions.
2. `tebako audit --event fetch,runtime-fetch-verified` — every byte source
   and its verdict.
3. Check a `sha256` from step 2 against the registry row and its
   attestation (`gh attestation verify`), then reproduce the payload image
   from its recipe (see the reproducible-press rule in the spec set).
4. `tebako audit --verify` — the ledger's own integrity.
5. `tebako audit --event composition,overlay,decrypt` against the
   package's declared dependency tree (`tebako closure`).
6. Archive `ledger.json` and `journal.log.seal`; a later `--verify` on the
   archived pair proves the archive is what the host wrote.

## The events you will actually filter on

| question | events |
|---|---|
| fetch and verify | `fetch`, `delta-fetch`, `runtime-fetch-verified`, `runtime-index-verified`, `payload-signature-trusted` |
| install and bind | `payload-installed`, `payload-uninstalled`, `origin-rebind`, `registry-key-pinned` |
| mount and compose | `composition`, `overlay`, `decrypt`, `lazy-seed` |
| resolve and exec | `runtime-edge-resolved`, `executable-edge-resolved`, `spawn-shim-note` |
| policy | `jail-deny`, `jail-allow`, `vfs-deny`, `vfs-write`, `lib-load` |

The list is a census, not a whitelist: an event a future tebako writes is a
first-class record here, filterable by name, and it never breaks a reader.

## Reference

- `tebako audit [--since] [--until] [--event] [--artifact] [--tail] [--count] [--json] [--strict]`
- `tebako audit --seal [--json]`
- `tebako audit --verify [--json] [--strict]`
- Exit codes: 0 sealed, 80 ledger finding (tamper, bad seal signature,
  `--strict` with damage), 74 unreadable file or no seal yet, 64 usage.
- Normative contract: `docs/spec/40-audit-ledger.md` and
  `docs/spec/schemas/audit-ledger.yaml`.
