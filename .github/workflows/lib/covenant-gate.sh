#!/bin/bash
# covenant-gate.sh — the release-side consultation of the performance
# covenant (spec 27 §11.1). finalize dispatched benchmark.yml for this
# release (BENCH_RUN_ID); this script waits on that run and fails the
# release pipeline when the covenant gate tripped there.
#
# Verdict reading: the bench fan-in's "Gate on the covenant" step fails
# exactly when the verdict is `fail`, and the report-bundle artifact
# (covenant-verdict.json) uploads with if: always() so the numbers
# survive the trip. A run whose gate step never produced a conclusion
# is NOT a pass — a covenant that did not run cannot vouch.
set -euo pipefail

: "${BENCH_RUN_ID:?the dispatched benchmark run id is required}"
: "${GH_TOKEN:?}"

echo "covenant gate: release ${TAG:-<no tag>} reads benchmark run $BENCH_RUN_ID"

deadline=$(( $(date +%s) + 13200 ))  # 3.5 h — under the job's 4 h timeout
while true; do
  st=$(gh run view "$BENCH_RUN_ID" --json status --jq .status)
  echo "  run $BENCH_RUN_ID: $st"
  [ "$st" = "completed" ] && break
  if [ "$(date +%s)" -gt "$deadline" ]; then
    echo "::error::benchmark run $BENCH_RUN_ID did not complete within 3.5 h — the covenant verdict never landed"
    exit 1
  fi
  sleep 300
done

# gh run view --json jobs paginates the full job list itself.
gate=$(gh run view "$BENCH_RUN_ID" --json jobs \
  --jq '[.jobs[] | select(.name=="report") | .steps[] | select(.name=="Gate on the covenant") | .conclusion] | first // "absent"')

# The numbers, whichever way the gate went (the bundle uploads with
# if: always() precisely so a trip still shows its verdict).
if gh run download "$BENCH_RUN_ID" -n report-bundle -D covenant-report 2>/dev/null; then
  cat covenant-report/covenant-verdict.json || true
fi

case "$gate" in
  success)
    echo "covenant verdict: PASS (run $BENCH_RUN_ID)"
    ;;
  failure)
    echo "::error::covenant verdict: FAIL — the bench fan-in's covenant gate tripped for ${TAG:-this release} (benchmark run $BENCH_RUN_ID). The release assets are published and immutable; the breach is recorded against the tag and in the run above. Investigate before the next release."
    exit 1
    ;;
  *)
    echo "::error::covenant gate found no verdict step in run $BENCH_RUN_ID (conclusion: $gate) — a covenant that did not run is not a pass"
    exit 1
    ;;
esac
