# benchmarks/ — the spec 27 harness documents

**CI tooling, never shipped.** Nothing here is part of the released
product; `crates/tebako-bench` is absent from `release.yml`'s binary set
by design. The contract is `docs/spec/27-benchmarks.md`.

## The documents (SSOT — workflows read these, never hardcode)

- `suite.yaml` — the metanorma suite (v2 tebako vs v1 packed-mn): the
  workloads (document, argv, expectations, timeout), the three targets
  (`v1-packed-mn` / `v2-shim` / `v2-fat`), and the run policy (warmup 1,
  warm 5 interleaved, cold 2).
- `suite-runtime.yaml` — the runtime comparison suite (on-system vs
  tebako): five interpreter workloads per runtime (boot / fib / stdlib /
  ioread / statloop) for ruby, python, and java. The `on-system-<lang>`
  arms are the CI-provisioned toolchains, version-pinned to the tebako
  runtimes under test; the `tebako-<lang>` arms are the factory runtime
  pairs (exe + env image) fetched sha256-verified. The harness asserts
  version parity before measuring — a mismatch gaps BOTH arms of the
  pair. The declared baseline is `on-system` (the report's ratio column
  is "vs on-system").
- `platforms.yaml` — WHERE: triplet → runner (+ alpine container for musl
  legs), the packed-mn tag for the v1 arm, and the named gaps
  (`v1_asset: null`, `v2_payload: false`, `runtime_gaps` for the runtime
  suite's declared gaps — python on windows-ucrt64 until its windows
  tier ships).
- `fixtures/` — vendored workload sources, byte-pinned to their upstream
  commits (see each file's comment in suite.yaml), plus `fixtures/java/`
  (the runtime suite's .java sources, compiled once in-leg with the
  on-system javac; both JVMs run the same .class files).

Validate any suite document after editing:

```
cargo run -p tebako-bench -- validate --kind suite benchmarks/suite.yaml
cargo run -p tebako-bench -- validate --kind suite benchmarks/suite-runtime.yaml
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

Appending a run's dashboards to the trend feed (the fan-in's job):

```
./target/debug/tebako-bench trend --release v2.8.8 --date 2026-09-17 \
  --dashboard dashboard.json --dashboard dashboard-runtime.json \
  [--previous trend.json] --out trend.json
```

Exit codes (spec 27 §8): `0` ok · `1` every arm failed/unavailable (the
artifacts are still written — a red matrix is a deliverable) · `2`
operational fault.

## CI

`.github/workflows/benchmark.yml` runs on `workflow_dispatch` and
`release: published` only — never per-PR. The leg matrix is generated
from `platforms.yaml`; each leg runs the metanorma suite (musl legs
inside the pinned alpine container) and the runtime suite (on the host,
against the provisioned ruby/python/java). The fan-in job merges each
suite's per-triplet `results.json` files into `report.md` +
`dashboard.json` and `report-runtime.md` + `dashboard-runtime.json`, and
on release AND dispatch runs publishes all four to the `bench-history`
branch — under the per-run entry (`<tag>/` or `run-<run-id>/`) and the
`latest/` mirrors — refreshes `latest/trend.json` (the
release-over-release feed, capped at the last 20 releases), and fires a
`bench-update` repository_dispatch at tebako.org (soft-fail: the site's
nightly rebuild is the fallback).

## Reading the numbers

Median is the headline; **min is the cross-noise-comparable figure** on
shared runners (noise inflates, never deflates). Speedups are vs the
suite's baseline arm (the metanorma suite: v1-packed-mn; the runtime
suite: on-system) on the same triplet × workload × mode; a missing
baseline arm renders "—". Cold runs (caches wiped) are install/first-boot
metrics and are reported separately from warm runs; on-system arms have
no cold story (the toolchain's provisioning time is not tebako's
comparison). Peak RSS is file-backed-inclusive — mmap'd image pages are
reclaimable under memory pressure, so a tebako arm's RSS delta overstates
its real memory cost. The old world is frozen at the packed-mn tag's
metanorma-cli while the v2 payload is current — compare ratios, not
absolutes.
