# Golden correlation fixtures (parity contract)

One directory per case. Any escape correlator — retrace-correlate
or a third-party implementation (tebako's Rust correlator per
docs/spec parity clause) — must produce exactly `expected.txt`
on stdout and exit with the code in `exit.txt` when run as:

    <correlator> --inside <case>/inside.json \
                 --outside <case>/outside.json \
                 --prefix <contents of <case>/prefix.txt>

Files per case:

| File          | Meaning                                          |
|---------------|--------------------------------------------------|
| `inside.json` | VFS (tfs) stream, retrace-shaped JSON            |
| `outside.json`| retrace stream (may be truncated, may be JSONL)  |
| `prefix.txt`  | the virtualization prefix argument, one line     |
| `options.txt` | OPTIONAL extra CLI flags, verbatim (--pid N,     |
|               | --window SECS, --exclude-probes); absent = none  |
| `expected.txt`| exact expected stdout (no trailing junk)         |
| `exit.txt`    | expected exit code (0 clean / 1 escapes / 2 err) |

Report line format (since v2.9.0):

    escape <path> func=<f> tid=<t> pid=<p> class=<probe|read|write|none>

class is the event classification: probe = existence leak
(QueryOpen/stat/access semantics), write = potential mutation,
read = data access.

stderr (summaries, warnings) is NOT part of the contract; stdout
and the exit code are. Paths in `inside.json`/`outside.json` may
appear in any string field; normalization (NT forms, slashes,
drive letters) is part of what these cases pin down.

Add a case by adding a directory — the CTest loop in
tools/correlate/CMakeLists.txt picks it up at configure time.
