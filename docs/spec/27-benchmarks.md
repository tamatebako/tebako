# Spec 27 — Benchmarks: the tebako-bench harness contract

Status: PLANNED (spec-first per spec 14; slice 0 spikes executed
2026-08-25 — §9 records the evidence; the suite/platforms documents,
schemas, and the `tebako-bench validate` surface land with this spec's
implementation slice; the sampler, acquisition, run engine, report
renderer, and workflow are later slices)
Depends on: 00 (locked invariants), 02 (tpkg wire format), 05
(resolution and cache), 14 (engineering process), 16 (distribution —
slim/fat), 17 (runtime driver contract), 19 (bootstrap distribution),
20 (limnifs backend)

Tebako's benchmark harness answers one question with numbers a third
party can audit: **how does the v2 tebako stack compare to the v1
packed-mn release executables on the same document, on the same
machine, in wall time, CPU time, and peak memory?** The harness is a
Rust binary (`tebako-bench`, crate `crates/tebako-bench`), three
authored documents (`benchmarks/suite.yaml`, `benchmarks/platforms.yaml`,
plus vendored fixtures), two versioned JSON Schemas
(`schema/tebako-bench-suite-v1.schema.json`,
`schema/tebako-bench-result-v1.schema.json`), and one GitHub workflow
(`.github/workflows/benchmark.yml`). This spec is the contract those
artifacts implement.

## 0. Frame — what the harness is, and is not

- **CI tooling, never shipped.** `tebako-bench` is a workspace crate
  that builds and runs in CI (and locally for development) and is
  NEVER a shipped artifact: it is not added to `.github/workflows/release.yml`'s
  binary set, not published to any release page, not installed by any
  installer. The crate root carries a comment stating this boundary.
  The audience law (§root AGENTS.md) is unaffected: nothing a user or a
  payload developer runs ever involves this crate.
- **The no-shell-out law applies to tooling too** (invariant 1 is
  written about shipped artifacts; the harness obeys it by choice,
  because that uniformity is exactly what lets one implementation serve
  all seven triplets, musl and Windows included). All downloads are
  in-process HTTP (the workspace's HTTP crate), all archive handling is
  in-process (`flate2`/`tar`), all measurement is in-process FFI
  (§4). The harness never shells out to `time`, `curl`, `git`,
  `Measure-Command`, or any platform tool.
- **No vcpkg, no TFS dependency.** The crate is pure Rust. It must
  build in the pure-Rust CI legs (the `test-windows` package set) —
  it never links dwarfs-t, sqfs, or rnp. It *drives* the product
  binaries; it does not link them.
- **Never per-PR.** Benchmarks run on `workflow_dispatch` and on
  `release: published` only. A per-PR benchmark would be noise (shared
  runners) and a cost bomb, and it would put the harness on the merge
  critical path where it does not belong.
- The harness **reports**; it does not gate. There is no performance
  regression refusal in this spec. Named gaps (§6) are data, not
  failures of the product under test.

## 1. The three arms (targets)

Every benchmark row is one **target** — a way of obtaining and running
the payload. Three kinds exist; a suite declares one target of each
kind unless an arm is genuinely meaningless for the suite.

| kind | What it is | The contract it represents |
|------|-----------|---------------------------|
| `v1-exe` | A packed-mn release asset (per-triplet `.tgz` or `.exe`), sha256-verified against its `.sha256.txt` sidecar, executed directly | "Download one file, run it, zero install" — the old world |
| `v2-press` | A fat tpkg assembled in-leg from published, sha256-verified artifacts: bootstrap + payload image slot + runtime slot | The v2 artifact with the same one-file contract (spec 16's fat form) |
| `v2-managed` | `tebako install <name>@<version>` from a registry, dispatched through the shim with a warm store | v2's primary distribution form (slim/managed, spec 16) |

**Structural asymmetries are part of the report, never hidden:**

- The v1 stack merges env+app in one embedded image and extracts to the
  host at run time; v2 mounts the env image and the payload image as
  two co-mounted images (spec 17). The comparison is apples-to-apples
  at the *user contract* level (one file, one command, one document),
  not at the mechanism level.
- packed-mn's Windows asset is aibika-packed, not tebako-v1; the
  platforms document carries a `v1_note` saying so, and the report
  labels the row "old world (aibika)". It is included because it is
  what users actually downloaded.
- **Version skew is accepted and documented**: the old world is frozen
  at the packed-mn tag's metanorma-cli (v1.14.4 ↔ metanorma-cli 1.14.4)
  while the v2 payload is current (1.16.9). Rebuilding a v1 package of
  a current metanorma resurrects retired tooling and is forbidden
  (AGENTS.md §10). Every report footer carries the skew note.
- The v2 image format (`dwarfs` today, `limnifs` when the writer can
  carry the tree — §9 spike b) is recorded per result
  (`versions.image_format`); numbers across formats are never mixed in
  one statistic.

## 2. The suite document (`benchmarks/suite.yaml`)

The suite is the authored SSOT for *what runs*. YAML (invariant 6),
`schema_version: 1`, validated against
`schema/tebako-bench-suite-v1.schema.json` by `tebako-bench validate`
and cross-checked against the crate's serde model in unit tests (the
tpkg pattern: schema and model kept MECE by the cross-check).

```yaml
schema_version: 1
name: metanorma-v1-vs-v2
workloads:
  - id: compile-small-iso                 # ^[a-z0-9][a-z0-9-]*$
    source: {kind: vendored, path: benchmarks/fixtures/test-iso.adoc}
    argv: ["compile", "{doc}", "--type", "iso", "--agree-to-terms"]
    expect: {exit: 0, files: ["test-iso.xml"]}
    timeout_s: 600
  - id: compile-medium-rice
    source: {kind: git, url: https://github.com/metanorma/mn-samples-iso,
             ref: <40-hex pinned commit>, path: sources/.../document-en.adoc}
    argv: ["compile", "{doc}", "--agree-to-terms"]
    expect: {exit: 0, files: ["document-en.xml"]}
    timeout_s: 900
targets:
  - {id: v1-packed-mn, kind: v1-exe}
  - {id: v2-shim, kind: v2-managed, payload: "metanorma@1.16.9",
     registries: ["tfs:github:tebako-packages/metanorma"]}
  - {id: v2-fat, kind: v2-press, payload: "metanorma@1.16.9", fat: true,
     registries: ["tfs:github:tebako-packages/metanorma"]}
run_policy: {warmup: 1, repetitions: 5, cold_repetitions: 2, interleave: true}
```

Contract rules:

- **workloads** — a document to compile plus the expectation. `source`
  is one of:
  - `kind: vendored` — a file committed in the repo (`path` relative to
    the repo root). For tiny, CI-proven fixtures only.
  - `kind: git` — a public git-hosted tree at a **pinned 40-hex
    commit** (`ref`). The harness fetches the host's archive-of-commit
    over HTTPS in-process and extracts it in-process (never a `git`
    shell-out — invariant 1); `path` selects the document inside the
    tree. Floating refs (branch names, tags) are a named error:
    reproducibility is the point of the pin.
- `{doc}` is the ONE argv substitution: the workload document's path in
  the run's scratch directory. Anything else is literal. (The spec 26
  checks contract's `{scratch}` rule, applied to the document itself.)
- `expect`: `exit` (default 0) and `files` — scratch-relative outputs
  that must exist and be non-empty (the spec 26 §1 `expect.files`
  rule). A run that misses its expectation is recorded with
  `status: "failed"` (§6); it is not retried silently and never
  enters statistics.
- `timeout_s` is per single run. Expiry kills the child (process group
  on POSIX, job object on Windows — §4) and records
  `status: "timeout"`. A benchmark run may be slow; it may never hang
  the leg.
- `opt_in: true` workloads are skipped unless the run explicitly opts
  in (e.g. a dispatch input). Use for workloads needing credentials —
  the OIML r060 document (Futura PT fonts live in the private
  fontist-formulas repo) is the canonical case. A skipped opt-in
  workload emits NO rows at all (it is not a gap — nothing was asked).
- **targets** — see §1. `v2-managed`/`v2-press` carry `payload`
  (`name@version`) and `registries` (spec 04 references). `v1-exe`
  takes its asset from the platforms document (§3).
- **run_policy** — `warmup` full runs per (workload × target) before
  any measurement (primes the fontist/relaton/OS caches);
  `repetitions` measured warm runs per (workload × target);
  `cold_repetitions` measured cold runs (§5); `interleave: true`
  rotates targets A/B/C per iteration so runner drift decorrelates
  across arms. `interleave: false` runs each target to completion in
  turn (debugging only).

## 3. The platforms document (`benchmarks/platforms.yaml`)

The triplet → runner + asset mapping lives here, not in the workflow —
the workflow's matrix is generated FROM this document, so a platform
change is one authored edit. YAML, `schema_version: 1` (its structural
gate is the crate's serde model; a versioned JSON Schema may follow in
a later schema revision).

```yaml
schema_version: 1
packed_mn: {repo: metanorma/packed-mn, tag: v1.14.4}
triplets:
  linux-gnu-x86_64:  {runner: ubuntu-24.04,     v1_asset: metanorma-linux-x86_64.tgz,      v2_payload: true}
  linux-gnu-arm64:   {runner: ubuntu-24.04-arm, v1_asset: metanorma-linux-aarch64.tgz,     v2_payload: false}
  linux-musl-x86_64: {runner: ubuntu-24.04, container: alpine:3.21, v1_asset: metanorma-linux-musl-x86_64.tgz, v2_payload: false}
  linux-musl-arm64:  {runner: ubuntu-24.04-arm, container: alpine:3.21, v1_asset: null,    v2_payload: false}   # gap both sides
  macos-arm64:       {runner: macos-14,         v1_asset: metanorma-darwin-arm64.tgz,      v2_payload: true}
  macos-x86_64:      {runner: macos-15-intel,   v1_asset: null,                            v2_payload: false}   # gap both sides (v1 dropped after ~v1.13)
  windows-ucrt64:    {runner: windows-latest,   v1_asset: metanorma-windows-x86_64.exe,    v2_payload: true, v1_note: aibika-packed}
```

- The seven triplets are the release workflow's platform vocabulary —
  the same spelling, so a benchmark row joins against a release row
  without a translation table.
- `v1_asset: null` and/or `v2_payload: false` are not skips: each
  produces an explicit `unavailable` row per workload (§6) with the
  reason named. Gaps are data.
- `container` (musl legs) names the image the leg builds and runs
  inside — the harness itself is one pure-Rust implementation, so the
  container is the whole difference.
- The packed-mn POSIX `.tgz` layout is a **single-member gzipped tar
  carrying one executable** named `metanorma-<platform>` at the root
  (§9 spike c) — extraction is: decompress, take the one member, mark
  it executable. The Windows asset is the aibika `.exe` itself
  (optionally zip-wrapped per the release page).

## 4. The sampler semantics

One in-process sampler spawns the target and records the run. Platform
shell tools are forbidden (they are absent or flavor-divergent exactly
where the matrix needs uniformity: no `/usr/bin/time -v` on macOS,
busybox `time` on alpine, no peak working set in `Measure-Command`).
FFI is quarantined in `src/sys/{posix,windows}.rs` per the workspace
rule (unsafe only inside FFI boundary modules).

- **Wall time** — `std::time::Instant` around the spawn→wait span.
  Unit: seconds, f64, recorded as `wall_s`.
- **CPU time** —
  - POSIX: `libc::wait4(pid, …)` reaps the measured child itself, so
    the returned `rusage` is the child's OWN (utime/stime exact).
    This replaces the originally drafted `getrusage(RUSAGE_CHILDREN)`
    delta — **amended 2026-08-25, when the sampler landed**: that
    counter's `ru_maxrss` is a *running maximum* over every child the
    process has ever reaped, so after one big child the
    (after − before) RSS delta is 0 forever and runs 2..N would record
    garbage peak RSS. `wait4` has no such contamination and still
    gives exact per-child CPU. The sampler polls `wait4(WNOHANG)` on a
    ~2 ms tick against the deadline. The one-measured-child-at-a-time
    discipline still stands (it keeps wall/timeout semantics clean and
    matches the Windows handle model), even though wait4 makes
    cross-child CPU/RSS contamination impossible by construction.
  - Windows: `CreateProcessW` + `GetProcessTimes` on the child's
    process handle after wait; user/kernel 100-ns FILETIME intervals
    converted to seconds. Recorded as `cpu_user_s` / `cpu_sys_s`;
    statistics use their sum as `cpu_s`.
- **Peak RSS** —
  - POSIX: `ru_maxrss` from the same `wait4` reap (the measured
    child's own high-water mark). **Unit
    normalization at record time**: Linux/musl report KiB, macOS
    reports bytes — the sampler multiplies the Linux value by 1024 and
    every run record carries `peak_rss_bytes`, bytes, always. A result
    file with un-normalized units is a bug, not a platform difference.
  - Windows: `K32GetProcessMemoryInfo` → `PeakWorkingSetSize` (bytes,
    already normalized).
  - `ru_maxrss`/`PeakWorkingSetSize` are high-water marks of the
    child — per-run peak, not a sample series. That is the metric
    (process comparison), and it is deliberately not a Ruby-heap
    profiler: the comparison target is the process, not the
    interpreter's allocator.
- **Timeouts** kill the whole child tree (POSIX: the child's process
    group; Windows: the job object) and record `status: "timeout"`.
- One sampler process = one child at a time = one leg's sequential
  matrix. Cross-leg parallelism is the workflow matrix's job, never
  the sampler's.

## 5. Modes: warm and cold

- **warm** — the steady-state compile: store primed (v2 runtime +
  payload installed/resolved), payload caches primed by the warmup
  run(s). Warm runs measure *the compile*, and only warm runs enter
  the compile statistics.
- **cold** — the first-boot experience: before EACH cold run the
  harness wipes the per-stack caches and measures the whole thing
  (acquisition included). The wipe set:
  - `~/.tebako` — the v2 store (runtimes, payloads, registries).
    Wiping it forces re-download + re-verification; that cost IS the
    cold metric for the v2 arms. (v2-managed only — v2-press fat
    packages carry their runtime and never touch the store; v1-exe
    never either.)
  - The v1 stack's host extraction root (the v1 memfs unpacks under
    `$TMPDIR` — observed 2026-08-25 as `$TMPDIR/<content-hash>/` plus
    `tebako-runtime-*` staging dirs): v1's analog of the store, wiped
    for v1 cold runs so "cold" means first-boot on both stacks.
  - `~/.metanorma` and `~/.relaton` — the payload-side caches (fontist
    fonts, relaton bibliographic cache), present for BOTH stacks, wiped
    for every cold run of every target. Same paths both stacks: parity
    holds.
- Cold results are reported SEPARATELY as install/first-boot metrics
  and are never mixed into warm medians. `mode` on every run record is
  `"warm"` or `"cold"` (§6), and statistics are computed per mode.
- The wipe set is fixed by this spec, not by the suite: an ad-hoc
  per-suite wipe list would make cross-suite numbers incomparable.
  (If a payload-side cache outside the three named paths proves to
  matter — e.g. a legacy `~/.fontist` — it is added HERE by spec
  amendment, applied to every target alike.)

## 6. The result document (`results.json`)

One result file per triplet per run is the leg's artifact. JSON (the
results format is machine-written, machine-merged — the authored-YAML
rule covers authored documents; schemas are versioned JSON Schema per
invariant 6). Validated against
`schema/tebako-bench-result-v1.schema.json`; the schema and the crate's
serde model are cross-checked in unit tests.

```json
{"schema_version": 1, "suite": "metanorma-v1-vs-v2", "triplet": "linux-gnu-x86_64",
 "runner": {"runs_on": "ubuntu-24.04", "arch": "x86_64", "cpus": 4, "ram_bytes": 16777216000},
 "versions": {"tebako": "0.2.4", "runtime": "0.16.9-3.3.12", "payload": "1.16.9-3",
              "packed_mn": "v1.14.4 (metanorma-cli 1.14.4)", "image_format": "dwarfs"},
 "runs": [{"workload": "compile-small-iso", "target": "v2-shim", "mode": "warm",
           "iteration": 3, "status": "ok",
           "wall_s": 12.34, "cpu_user_s": 11.8, "cpu_sys_s": 0.4,
           "peak_rss_bytes": 123456789, "exit": 0}],
 "stats": [{"workload": "compile-small-iso", "target": "v2-shim", "mode": "warm", "n": 5,
            "median_wall_s": 12.3, "min_wall_s": 11.9, "max_wall_s": 13.1,
            "stdev_wall_s": 0.4, "mean_wall_s": 12.4,
            "median_cpu_s": 12.1, "median_peak_rss_bytes": 123000000}]}
```

Contract rules:

- **Run records** carry `status`:
  - `"ok"` — expectation met; all metric fields present.
  - `"failed"` — ran, expectation missed (exit mismatch, missing/empty
    output); `exit` and whatever metrics exist are present, `error`
    names the miss. Never enters statistics.
  - `"timeout"` — killed at `timeout_s`; `wall_s` ≈ timeout, other
    metrics absent. Never enters statistics.
  - `"unavailable"` — **the named-gap record**: the arm was never
    attempted on this triplet (no v1 asset, no published payload, a
    spike-proven platform incapacity). `reason` is mandatory and
    human-readable; `mode`/`iteration`/metrics are absent. A gap is
    explicit data — silent omission is a schema violation (invariant 9:
    named, never silent).
- **`runner` metadata is mandatory on every result** — `runs_on` (the
  GitHub runner label), `arch`, `cpus`, `ram_bytes`. Numbers without
  their environment are not numbers.
- **`versions`** records what actually ran — resolved versions, not
  requested ones (the runtime line resolves to `0.16.9-3.3.12`, the
  payload to `1.16.9-3`). `image_format` is the v2 arms' backend
  (`dwarfs` today; §9 spike b).
- **stats** are computed per (workload × target × mode) over `status:
  "ok"` runs only, and only when `n ≥ 1`; `stdev` is the sample
  standard deviation (n−1), absent when `n < 2`. An arm whose runs all
  failed has NO stats row — the failed run records tell the story.

## 7. Statistical and reporting rules

- Defaults (the suite pins them): warmup 1, warm repetitions 5, cold
  repetitions 2, interleaved. Repetition counts below these are
  development shortcuts; CI runs use the suite values.
- Report median (headline), min, max, stdev, mean for wall; median CPU;
  median peak RSS. **min is the cross-noise-comparable figure** on
  shared runners (noise inflates, it does not deflate); the report
  labels it so.
- Speedup columns are always "vs v1-exe on the same triplet ×
  workload × mode"; a missing v1 arm (named gap) renders as "—", never
  as an invented ratio.
- The report footer carries: the runner metadata block per triplet, the
  shared-runner noise caveat ("GitHub-hosted runners are shared,
  multi-tenant machines; treat differences under ~10% as noise and read
  min alongside median"), and the version-skew note (§1).
- One result file per triplet; the report merges N triplet files into
  one markdown document + one site-ingestible dashboard JSON. Merging
  never recomputes run-level data — stats are re-derived from the
  merged run records, so a hand-edited run record cannot smuggle a
  stale stat past the report.

## 8. The tool surface and its named errors

```text
tebako-bench run --suite <suite.yaml> --platforms <platforms.yaml>
                 --triplet <t> --out <dir> [--opt-in <workload-id>]...
tebako-bench report <results.json>... --md <report.md> --json <dashboard.json>
tebako-bench validate --kind suite|result <file>
```

Exit codes (the tool is not the product; its codes are the plain CLI
shape, never the spec 06/17 contract codes):

- `0` — success; for `validate`: the document is VALID.
- `1` — for `validate`: the document is INVALID (every violation listed
  on stderr, one per line, path-prefixed). For `run`/`report`: the
  benchmark or merge completed with every arm failed/unavailable (the
  artifacts still written — a red matrix is a deliverable, not a
  crash).
- `2` — operational error: unreadable/unparseable input, unknown
  `--kind`, schema/model disagreement, unimplemented surface (the
  run/report stubs before their slices land return exit 2 with a named
  "not implemented" error), I/O failure.

Errors are named on stderr (`tebako-bench: <what> [<detail>]`), one
line, never a bare exit (invariant 9).

`validate` checks the input against BOTH gates: the versioned JSON
Schema (structure) and the crate's serde model (the same shape the run
engine consumes) — a file passing one and failing the other is a bug
in the harness, and the cross-check unit tests exist to make that
impossible.

## 9. Spike evidence (2026-08-25, macos-arm64, macOS 14.1.1)

The slice-0 spikes this design rests on, executed before this spec
landed:

- **(a) Fat press works on macos-arm64 — with `--format dwarfs`, not
  the default.** `tebako press -r <metanorma-cli 1.16.9 app> -e main.rb
  -m fat --format dwarfs` (v0.2.4 release binary) produced a 393.8 MB
  fat package that boots and compiles the small ISO fixture to the full
  output set (exit 0). The DEFAULT press format (`--format limnifs`)
  fails at image build: the limnifs writer refuses the staged tree's
  symlink (`git-5.1.0/.claude/skills -> ../.github/skills`,
  "unsupported file type", exit 255). v2-fat CI legs therefore press
  (or bundle) with dwarfs until the limnifs writer grows symlink
  support AND the size ceiling lifts.
- **(b) limnifs cannot carry the metanorma payload tree today.**
  `tfs mkimage --format limnifs` on the extracted 1.16.9 payload
  (29,846 files, 643 MB, zero symlinks) fails: the tree's metadata
  externalizes at **6,952,795 bytes** — the writer's inline threshold
  is 1 MiB − 24 KiB and the readers' hard ceiling is 1 MiB
  (`limnifs-core 0.2.61`'s `DEFAULT_INLINE_METADATA_MAX_BYTES`; the
  writer's `metadata_externalize_threshold` knob, limnifs#187, can
  raise the WRITE side but every published v0.16.x-era runtime reads
  with the default, so lifting it produces images the field cannot
  read). limnifs-write 0.2.61 (2026-08-24) is the newest published —
  upstream has NOT lifted the ceiling. Consequence: v2 arms report
  dwarfs-backend numbers for metanorma; a limnifs arm is a named gap
  until a small-payload suite or a lifted ceiling changes the facts.
  Never fake it.
- **(c) packed-mn v1.14.4 `metanorma-darwin-arm64.tgz` is a
  single-member gzipped tar** (343.8 MB → one 355.4 MB Mach-O arm64
  executable, mode 0755, member name `metanorma-darwin-arm64`), sha256
  verified against its `.sha256.txt` (bare-hash format). **The v1 exe
  does not run on this machine**: exec → v1 runtime extracts its memfs
  to `$TMPDIR/<content-hash>/` → AMFI kills the process loading the
  extracted `nokogiri.bundle` (`cs_invalid_page ... final status
  0x23020200, denying page sending SIGKILL`, exit 137; reproduced with
  a fresh TMPDIR). Whether the `macos-14` CI runner behaves the same is
  an empirical question the leg answers; if it does, the v1 macOS arm
  becomes a named gap with this reason. The harness records the
  outcome; it does not work around it.
