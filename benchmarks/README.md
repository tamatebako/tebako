# benchmarks/ — the spec 27 harness documents

**CI tooling, never shipped.** Nothing here is part of the released
product; `crates/tebako-bench` is absent from `release.yml`'s binary set
by design. The contract is `docs/spec/27-benchmarks.md`.

## The documents (SSOT — workflows read these, never hardcode)

- `suite.yaml` — the metanorma suite (v2 tebako vs v1 packed-mn): the
  compile workloads (document, argv, expectations, timeout), the three
  comparison targets (`v1-packed-mn` / `v2-shim` / `v2-fat`), the author
  loop (the `v2-tools` arm priming-presses `fixtures/author-loop/` at
  acquisition; `author-repress` re-presses it per measured run and
  `author-first-run` boots the pressed package), and the run policy
  (warmup 1, warm 5 interleaved, cold 2).
- `suite-runtime.yaml` — the runtime comparison suite (on-system vs
  tebako): five interpreter workloads per runtime (boot / fib / stdlib /
  ioread / statloop) for ruby, python, and java, plus the materialize
  cell's pair: `ruby-spawn` (an in-image `gem --version` with the host
  PATH pinned away) running on `tebako-ruby` and on `extract-ruby` (the
  `tfs-extract` arm — the same runtime ref's env image extracted whole
  by the release's `tfs` tool). The `on-system-<lang>`
  arms are the CI-provisioned toolchains, version-pinned to the tebako
  runtimes under test; the `tebako-<lang>` arms are the factory runtime
  pairs (exe + env image) fetched sha256-verified. The harness asserts
  version parity before measuring — a mismatch gaps BOTH arms of the
  pair. The declared baseline is `on-system` (the report's ratio column
  is "vs on-system").
- `covenant.yaml` — the per-axis budgets (spec 27 §11.1). Every cell
  carries a `source:` block the report's `--metrics` producer derives
  from the run's dashboard: workload suffix selectors, target prefix
  (with an optional `baseline` override for ratio/speedup), mode,
  measure (wall | rss), shape (ratio | speedup | absolute |
  teardown-delta), worst-of aggregation. The `covenant` subcommand
  evaluates the produced metrics: `over` fails the gate, `absent` is
  the named coverage gap. **The verdict artifact also carries the fleet
  witness (tebako#1071)**: per cell, `native_drift` (the native arm's
  median ÷ its own rolling median over the last ≤8 published runs, read
  from the bench-history trend feed the fan-in hands the producer via
  `--history`), `pair_delta` (the current paired per-iteration ratio ÷
  its rolling median), and the full `witness` record (the leg the
  worst-of aggregate kept + its runner image identity, `image_changed`
  vs the previous published run included). On a trip the cell's `class`
  names the reading: `fleet-move` (native drift large, pair delta
  small), `regression` (the tebako arm moved against a flat native
  arm), or `ambiguous` (both moved divergently, or the trip has no
  witness support) — adjudication is a read of the artifact. Degraded
  witnesses are explicit `null`s with the reason in `note`.
- `platforms.yaml` — WHERE: triplet → runner (+ alpine container for musl
  legs), the packed-mn tag for the v1 arm, and the named gaps
  (`v1_asset: null`, `v2_payload: false`, `runtime_gaps` for the runtime
  suite's declared gaps — python on windows-ucrt64 until its windows
  tier ships).
- `fixtures/` — vendored workload sources, byte-pinned to their upstream
  commits (see each file's comment in suite.yaml), plus `fixtures/java/`
  (the runtime suite's .java sources, compiled once in-leg with the
  on-system javac; both JVMs run the same .class files) and
  `fixtures/author-loop/` (the bare ruby project the author-loop cells
  press and boot — no Gemfile, so the re-press measures the press
  pipeline, never dependency work).

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

Merging triplet results into the report + dashboard (+ the covenant
metrics, with the fleet witness reading the bench-history trend feed):

```
./target/debug/tebako-bench report legs/*/out/results.json \
  --md report.md --json dashboard.json \
  [--metrics covenant-metrics.json --history latest-trend.json]
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
against the provisioned ruby/python/java). The fan-in job fetches the
bench-history branch's `latest/trend.json` (the fleet witness's rolling
baseline), merges each suite's per-triplet `results.json` files into
`report.md` + `dashboard.json` and `report-runtime.md` +
`dashboard-runtime.json`, evaluates the covenant — the "Gate on the
covenant" step prints every tripped cell's witness line and class —
and on release AND dispatch runs publishes all four to the
`bench-history` branch — under the per-run entry (`<tag>/` or
`run-<run-id>/`) and the `latest/` mirrors — refreshes
`latest/trend.json` (the release-over-release feed, capped at the last
20 releases), and fires a `bench-update` repository_dispatch at
tebako.org (soft-fail: the site's nightly rebuild is the fallback).

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
