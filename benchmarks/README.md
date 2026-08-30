# benchmarks/ — the spec 27 harness documents

**CI tooling, never shipped.** Nothing here is part of the released
product; `crates/tebako-bench` is absent from `release.yml`'s binary set
by design. The contract is `docs/spec/27-benchmarks.md`.

## The documents (SSOT — workflows read these, never hardcode)

- `suite.yaml` — WHAT runs: the workloads (document, argv, expectations,
  timeout), the three targets (`v1-packed-mn` / `v2-shim` / `v2-fat`),
  and the run policy (warmup 1, warm 5 interleaved, cold 2).
- `platforms.yaml` — WHERE: triplet → runner (+ alpine container for musl
  legs), the packed-mn tag for the v1 arm, and the named gaps
  (`v1_asset: null`, `v2_payload: false`).
- `fixtures/` — vendored workload sources, byte-pinned to their upstream
  commits (see each file's comment in suite.yaml).

Validate either document after editing:

```
cargo run -p tebako-bench -- validate --kind suite benchmarks/suite.yaml
```

## Running

Locally (one triplet — the one you're on):

```
cargo build -p tebako-bench
./target/debug/tebako-bench run --suite benchmarks/suite.yaml \
  --platforms benchmarks/platforms.yaml --triplet macos-arm64 --out out
```

Merging triplet results into the report + dashboard:

```
./target/debug/tebako-bench report legs/*/out/results.json \
  --md report.md --json dashboard.json
```

Exit codes (spec 27 §8): `0` ok · `1` every arm failed/unavailable (the
artifacts are still written — a red matrix is a deliverable) · `2`
operational fault.

## CI

`.github/workflows/benchmark.yml` runs on `workflow_dispatch` and
`release: published` only — never per-PR. The leg matrix is generated
from `platforms.yaml`; the fan-in job merges the per-triplet
`results.json` files into `report.md` + `dashboard.json`, and on release
runs appends both to the `bench-history` branch (the trend record the
website renders).

## Reading the numbers

Median is the headline; **min is the cross-noise-comparable figure** on
shared runners (noise inflates, never deflates). Speedups are vs the
v1-packed-mn arm on the same triplet × workload × mode; a missing v1 arm
renders "—". Cold runs (caches wiped) are install/first-boot metrics and
are reported separately from warm runs. The old world is frozen at the
packed-mn tag's metanorma-cli while the v2 payload is current — compare
ratios, not absolutes.
