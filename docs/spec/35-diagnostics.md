# Spec 35 — Diagnostics (`tebako doctor`)

Status: PROPOSED (2026-09-12, roadmap 87)
Owner: tamatebako/tebako (tebako-cli, composing tebako-shim)

## 1. Principle

One command turns "it doesn't work" into a paste-able diagnostic. Every
enterprise ticket starts here.

- **Read-only.** Doctor never fixes, never mutates the store, never
  writes config. Findings carry the exact remediation command or config
  block as text.
- **Named verdicts, never silence.** Every check emits a line:
  `ok:` / `note:` / `problem:`. A check that cannot run says why
  (`note: offline — network probes skipped (TEBAKO_OFFLINE=1)`).
- **Compose, never duplicate.** The dispatch section IS tebako-shim's
  doctor report through its structured API; the CLI never re-implements
  a shim check. New planes live exactly one layer down from their
  subject (§5).

## 2. The sections

| Section | Plane | Checks |
|---------|-------|--------|
| `store` | tebako-cli (spec 05/18) | TEBAKO_HOME resolvable + writable; store layout version current; disk headroom; no stale install locks (the 120 s flock, spec 05 §4); every installed artifact's `.sha256` sidecar VERIFIES (runtime exe + env image, payload images) — existence is the shim's check, digest agreement is this one |
| `dispatch` | tebako-shim (spec 07) | shim dir on PATH; every shim link resolves to an installed payload+version; payload records (image + trust anchor + manifest mirror); config parse; routing health (collisions, dangling pins, disabled-but-pinned) |
| `network` | tebako-cli (spec 04's enterprise-networking amendment) | the effective netconfig resolution (env vs config, first-hit-per-key) printed; one probe per distinct remote host among {the runtime index, configured registries}; TLS served-chain issuer vs the effective root set (§3) |
| `trust` | tebako-cli (spec 09) | `TEBAKO_REQUIRE_SIGNED` state; the embedded root's fingerprint; unsigned artifacts in the store listed loudly (allowed by default, listed because the support question "is everything here signed?" must be one command) |
| `registries` | tebako-shim freshness + tebako-cli reachability | per configured ref: cached? fresh (24 h TTL)? file existence for `file://`; reachable? (a network probe, skipped offline) |

## 3. The TLS interception verdict (normative)

The enterprise failure mode (metanorma-iho#531): a corporate proxy
re-signs TLS, the bundled webpki roots reject the chain, and the user
cannot tell a broken proxy from a broken tool.

For each probed host the doctor compares the SERVED chain's issuer
against the EFFECTIVE root set (bundled webpki roots; platform roots
when `network: tls_roots: platform` / its env spelling is resolved;
`TEBAKO_EXTRA_CA` additions). The verdicts:

- chain verifies under the effective roots → `ok: tls <host>: chain verifies`
- chain verifies under the PLATFORM roots but NOT the effective roots →
  interception named, remediation printed verbatim:

  ```
  problem: tls api.github.com: the served chain is rejected by the effective roots but accepted by the platform store — a TLS-intercepting proxy is in the path
    remediation: set `network: tls_roots: platform` in ~/.tebako/config.yaml (or TEBAKO_EXTRA_CA=/path/to/corp-ca.pem) — spec 04's enterprise-networking amendment
  ```

- chain verifies under neither → `problem: tls <host>: the served chain verifies under neither the effective nor the platform roots — do not bypass; investigate before trusting`

The probe is one HTTPS GET per host (never more; no writes, no state).
`TEBAKO_OFFLINE=1` skips the whole section with a named note. A probe
that times out is `note:` (the network may be down; that is not a
doctor problem), carrying the elapsed deadline.

## 4. Output contract

Text (default), sectioned:

```
store:     ok / problem lines…
dispatch:  …
network:   …
trust:     …
registries:…
tebako doctor: no problems found        # exit 0
tebako doctor: 3 problem(s)             # exit 1
```

`--json`: one JSON document on stdout (the banner stays on stderr, per
the spec 15 §6 machine contract), `doctor_schema: 1`, a `sections`
array of `{name, findings: [{severity, text}]}`, and a top-level
`problems` count. The JSON shape is versioned by `doctor_schema`;
additions bump it additively, removals never happen within a major.

Exit codes: `0` healthy · `1` problems found (advice only — the
findings name their own remediations) · usage errors ride the CLI's
uniform usage path (USAGE on stderr, exit 1). Doctor itself never
fails closed on a sick store: a check that errors mid-probe degrades to
a `problem:` line naming the error, never to a crash.

## 5. Composition rule (where code lives)

- **tebako-shim** exports the structured report
  (`manage::doctor_report(ctx) -> DoctorReport`); `tebako-shim doctor`'s
  text output renders FROM that report (one renderer, byte-parity with
  the pre-refactor output asserted by the shim's existing tests).
- **tebako-cli** renders the full five-section output: the dispatch
  section is the shim's report verbatim; store/network/trust/registries
  are the CLI's own planes.
- Anything a dispatch-time failure could implicate belongs to the shim's
  report first (the CLI inherits it); anything about the broader store,
  the network path, or trust policy belongs to the CLI.

## 6. Non-goals

- No fixing (advice only, exact commands given).
- No network mutation (read-only probes only; nothing is installed,
  refreshed, or downloaded).
- No new dependencies.
- Not a linter for payload manifests (that is `tebako check`, spec 26).
