# The release train

How a runtime-line change crosses every repository between upstream and the
user's machine — the stage graph, what may run in parallel, where the gates
sit, and what to do when a stage wedges. Written for the engineer (human or
agent) driving the next train; every recovery entry names its symptom and the
exact fix. Incident citations use GitHub run ids.

## The stage graph

A train is sequenced by HARD ordering constraints; within a stage, legs run
in parallel. The canonical instance is a ruby-line change (the metanorma on
ruby 4.0 chain was six stages):

```
1  tamatebako/ruby            (source factory)
     in:  upstream tarball + patch set bump
     out: tfs-ruby-<ver>-src[-<scenario>].tar.gz + SHA256SUMS release
          (plus provenance.yaml — append-only ledger)
2  tamatebako/tebako-runtime-ruby   (runtime factory)
     in:  the src release (pinned by digest, never by tag movement)
     out: tebako-runtime-<tv>-<rv>-<triplet>[.exe|.tfs] pairs + manifest.json
   └─ may not start before stage 1's release serves the pinned digest
3  tamatebako/tebako          (the product)
     in:  runtime pairs at the version the resolver must serve
     out: the tebako CLI/toolchain release (tebako, tfs, shim, pkg, bootstrap)
   └─ probe-gated: press/accept legs run against the staged pairs
4  feedstocks (tebako-packages/*)
     in:  the product release — tools pin (digest-pinned per platform)
     out: payload re-cut tags (<version>-<revision>: additive releases,
          never tag movement)
   └─ tools.sha256 pins MUST be filled from the release's SHA256SUMS before
      any leg runs (the FILL guard is a named error)
5  flavor/packed lanes (metanorma/packed-metanorma*, private flavors)
     in:  the base payload re-cut + the tools pin
     out: slice releases, packed installers, airgap bundles
   └─ flavor acceptance probes the PUBLISHED base row; in the flip window
      (base re-cut not yet served) the gated legs skip loudly, they do not
      fail silently
6  downstream lanes (Homebrew, winget, Chocolatey, Snap, Docker)
     in:  packed installers at their published digests
     out: user-facing channel updates
```

**Parallelism windows.** Stages 2 and 3 have one window each: stage 2's
per-triplet builds are independent; stage 3's release legs are independent
once the runtime pairs are pinned. Everything else is strictly sequential
per repo — and ONE open PR per repo at a time everywhere.

## The rules

1. **Tags never move.** A published tag is immutable. A re-cut is a NEW
   revision-suffixed tag (`1.17.0-6` after `1.17.0-5`); consumers re-pin by
   digest. Moving a tag breaks every digest pin downstream and orphans the
   release object (see recovery R2).
2. **Pins are digests, not names.** Every cross-repo consumption pins
   sha256 (tools pins, runtime pins, payload rows). The registry's row for
   a version may move to the newest revision — that is a re-render, not a
   tag move.
3. **The rehearsal rule.** A release workflow change ships only after a
   rehearsal tag on a quiet day proves the full path (build → publish →
   finalize → downstream resolve). The publish-split work (tebako#649)
   still owes its rehearsal.
4. **Write-once asset names.** An asset name on a release is written once.
   The independent-publish rules (write-once names, derivation-not-mutation,
   registry-PR-as-publication-event, immutable assets + yank markers) exist
   because replaced names are the sole wedge/race surface (see R1).

## Probe gates

- **Registry flip probes** (the shared `probe-registry-row` action): a gated
  leg checks the published row actually carries its expectation (version on
  the expected constraint line) before running; in the flip window it skips
  LOUDLY with a named note.
- **Contract gates** (factories): the driver contract version is fetched
  once per run into a dedicated job and handed down as an artifact — the
  gate stays fail-closed offline.
- **Acceptance probes** (flavors): each leg probes its base row on the
  expected ABI line before compiling through the flavor.

## Failure recovery

**R1 — finalize wedge on a replaced asset name.**
Symptom: the release's finalize step loops `delete → 422 already_exists →
retry` on one asset (evidence: trr v0.16.23, runs 34668151001 and
34672299684, 2026-09-12; same class 2026-08-03, 4.5 h). Mechanism: GitHub
reserves a deleted asset's name server-side for minutes-to-hours; the
listing shows absence while the POST 422s, and premature re-upload attempts
REFRESH the reservation — the retry loop sustains the wedge it waits out.
Recovery: (1) stop the loop — cancel the workflow; (2) delete the stale
asset via the API; (3) arm the quiet-window uploader (60 min of untouched
silence, then SINGLE attempts 30 min apart, digest read-first); (4) if the
wedge persists, cut the next revision tag instead of replacing the name.
Prevention: write-once names — the independent-publish shape deletes the
replace-name pattern entirely.

**R2 — orphaned release after a tag delete.**
Symptom: the next release run's `get-release` 404s although the tag is gone
(evidence: tebako run 33228299426). Mechanism: deleting a remote tag without
deleting its release object leaves an invisible DRAFT release bound to the
dead tag. Recovery: delete the draft release object via the API, re-push the
tag (or cut the next one); prepare then creates a fresh published release.
Prevention: never delete a tag without its release object.

**R3 — mirror 404s after a tag≠line re-publish.**
Symptom: composed-name mirror fetches 404 (evidence: openjdk v2.5.1
re-published the 2.5.0 line, 2026-09-12). Mechanism: a tag on a different
line than the release's assets — the mirror derives names from the line.
Recovery: RELINE — re-publish under the correct line tag and re-derive the
mirror rows; the tag==line release guard now refuses the shape at publish.

**R4 — version skew between press toolchain and consumers.**
Symptom: acceptance legs fail at boot with
`entrypoint '<name>' not found in the mounted tree` while the press legs
were green (evidence: the 2026-10-09 flavor lane incident, tebako#744).
Mechanism: the payload was pressed in a newer image format than the
consumer lane's pinned tools can read; the install degraded to a
synthesized manifest whose bare entrypoint names resolved to wrong paths.
Recovery: bump the lane's tools pin to a release that reads the format
(digest-pinned), re-run acceptance. Prevention: tebako#744 — install must
fail closed on an unreadable image with a named format/reader mismatch;
until it ships, re-pinning a payload revision means checking the tools pin
reads that revision's format.

**R5 — stale registry serve.**
Symptom: a resolved payload version is not the newest revision; consumers
see the old bytes. Mechanism: the registry row re-renders on publish PRs;
the row you see is the last merged render. Recovery: merge the pending
registry PR (publication IS the merge); never hand-edit the served file.

## Where things live

- Stage inputs/outputs and per-stage incident history: `PROGRESS/` at the
  ecosystem root (this document's citations: `PROGRESS/37`, `PROGRESS/38`).
- The independent-publish rules: roadmap 85 (spec 09 §9, spec 13 §2a).
- The flavor/base binding grammar: spec 03 §2.8.
