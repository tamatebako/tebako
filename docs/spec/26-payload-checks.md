# Spec 26 — Payload checks: the in-image self-validation contract

Status: PARTIAL (§1's `checks:` manifest block (schema_minor 3) and
§2/§2.1's `tebako check` engine shipped 2026-08-20 — the four target
forms (name / bare image / package / composition document), the SKIP
discipline, and the jail composition; §3's press/install gate wiring
remains. Note the §2 aggregate exit is 79, not this draft's original
72 — 72 has been `EX_TEBAKO_TRUST` since spec 09)
Depends on: 03 (payload manifest), 08 (jails), 17 (driver contract),
22 (runtime-native interposition), 23 (declarative composition),
24 (overlays), 25 (trace observability)

A **check** is a payload's own acceptance test, declared in its in-image
manifest and carried inside its image: "given my declared needs, I do my
one real thing." The metanorma formula's Homebrew test block — write a
six-line AsciiDoc, compile `--type iso`, assert the `.xml` and `.html`
exist — is the canonical shape. This spec makes that shape declarative,
in-image, and runnable at three moments by three actors from ONE
definition. Every slice kind can declare checks — executable, runtime,
and data slices alike (§1) — and a composition document can declare its
own checks for the binding it assembles (§2), so the org that ships
metanorma + its templates as two slices tests the WHOLE stack from the
one config that binds them.

## 0. Frame — what a check is, and is not

- A check is the **payload's** acceptance contract. The runtime's
  acceptance is spec 22 §8 (the factory dogfood) — a green runtime
  proves the interpreter + driver; a green check proves the payload on
  top of it. Neither substitutes for the other (MECE).
- A check **verifies** the declarations of spec 23 under load: a
  declared-but-wrong need fails the check; an undeclared-but-needed path
  is discovered by running the check under `--record` (§3, third
  moment). Declarations are D1/D2; verification is here.
- A check is not observability. Spec 25's bus answers "what happened";
  a check answers "does it work". The bus carries the check run when
  asked (`--record`), never the other way around.
- A check is **in-image, always**. Nothing about a check is fetched,
  mirrored, or synthesized. The registry (L3) mirrors resolution fields
  only; check names surface through `tfs info` (spec 15) by reading the
  manifest, like every other L1 field.

## 1. The `checks:` block (spec 03 amendment)

Additive top-level key in the in-image manifest, versioned JSON Schema,
unknown keys a named error (the spec-03 discipline):

```yaml
checks:
  html-xml:                          # the check name (per-slice unique)
    entry: /bin/metanorma            # in-image executable — MAY be the
                                     # payload's own entrypoint
    argv: ["--type", "iso", "{scratch}/test-iso.adoc", "--agree-to-terms"]
    fixtures: /__tpkg__/check/html-xml   # in-image dir; its CONTENTS land
                                         # at the scratch root (optional)
    expect:
      exit: 0                        # default 0
      files: ["test-iso.xml", "test-iso.html"]  # relative to scratch;
                                     # existence + non-empty
      stdout: "…regex…"              # optional; one pattern, must match
    needs: [...]                     # additive host needs for the check
                                     # run ONLY (rare; spec 23 §2 grammar)
    requires: { provides: [jvm] }    # composition prerequisites;
                                     # unmet ⇒ SKIP, never FAIL
    when: [windows, macos, linux]    # optional platform filter
    timeout: 120                     # seconds; expiry is a FAIL
```

Rules:

- `entry` is always an in-image path — or, on a runtime slice only, the
  reserved spelling `self` (§1.1: the runtime exe, a tebako artifact
  paired with the env image). A check never names a host system
  executable (invariant 1 — no system dependencies).
- `{scratch}` is the ONE argv substitution: the per-run host scratch
  directory (§2). The in-image fixtures path is the author's side of
  the contract; the run's side is always scratch-relative.
- `fixtures` materializes to the HOST scratch root — never VFS-spelled —
  because the consumer may be the payload's own raw-surface component
  (libsass's importer reads host paths; the spec-22 class-R lesson).
- `expect.files` asserts existence + non-empty. Byte-goldens are
  FORBIDDEN (output bytes churn with dependency versions; the Homebrew
  test's `assert_path_exists` is the parity bar, invariant 8).
- A slice may declare any number of checks; an executable-kind slice
  declaring none is a press-time lint WARNING.

### 1.1 The check shape per slice kind

The grammar above is the EXEC check. Its owner per slice kind:

- **Executable slices** (metanorma, sassc, inkscape): exec checks,
  exactly as above. The check usually rides the payload's own
  entrypoint — that is the point (the acceptance exercises the shipped
  surface, not a side door).
- **Runtime slices** (ruby today; the java runtime slice of spec 23 §11
  if it ever ships): exec checks with `entry: self` — the reserved
  spelling for "the runtime exe itself, with the env image mounted".
  The runtime factory authors them into the env image's manifest (it
  owns that manifest — SSOT). A ruby runtime's minimal check:

  ```yaml
  checks:
    boot-and-stdlib:
      entry: self
      argv: ["-e", "require 'json'; puts JSON.generate({ok: 1})"]
      expect: { exit: 0, stdout: '"ok":1' }
      timeout: 60
  ```

  One run exercises the env-image mount, the load paths, a default-gem
  require, and stdout plumbing — the runtime's user-side smoke, distinct
  from the factory's full acceptance (spec 22 §8): the factory proves
  the runtime at build time; this proves it on the machine that resolved
  it. A java runtime slice would declare `argv: ["-version"]` + a
  version pattern.
- **Data slices** (fonts, org templates, dictionaries): **structural
  checks** — no `entry`, no exec, no runtime resolution at all. The
  engine mounts the image and asserts in-image invariants:

  ```yaml
  checks:
    layout:
      expect:
        image_files: ["/templates/org/cover.adoc",
                      "/templates/org/header.html"]   # in-image, exist +
                                                    # non-empty
  ```

  A check with no `entry` is structural BY GRAMMAR (MECE — one key
  decides the shape, never a `kind:` flag). Structural checks need only
  the mount: they run with no runtime, no composition, and no jail
  beyond the mount itself, so a data slice's acceptance is provable
  anywhere the image can be read.

## 2. The engine — `tebako check`

```
tebako check <name | image.tfs | package> [--check <c>] [--list]
             [--record] [--keep-scratch]
             [--runtime <exe> --runtime-image <env.tfs>]   # bare-image form
```

Resolution is the dispatch resolution, unchanged (spec 23: D1 needs,
D2/D3 composition, newest compatible cached runtime) — except that
STRUCTURAL checks (§1.1, data slices) skip it entirely: they mount the
image and assert, with no runtime and no composition. Per exec check,
in declaration order (slice checks before composition checks, §2.1):

1. `when:` filter — a non-matching platform SKIPs (loud).
2. `requires:` — the resolved composition must provide every listed
   capability; an unmet prerequisite SKIPs with the missing capability
   named. The capability set is the union, over the composition's
   slices, of the slice name, the app `entrypoints[].name`, the toolkit
   `executables[].name`/`libraries[].name`, and the runtime
   `provides.engine` (an openjdk slice provides `java`). A check fails
   ONLY when its prerequisites are present and its behavior is wrong.
3. A fresh scratch dir (host tmp) is created, auto-granted `rw` for the
   check's duration — an engine grant, never a declared need; fixtures
   materialize into it.
4. The run: the composition's effective policy ∪ the check's `needs:`;
   argv substituted; stdout/stderr captured (they are the payload's,
   teed to the report).
5. Assertions evaluated; the verdict line printed:
   `check html-xml PASS 41s` / `check pdf SKIP (no jvm in the
   composition)` / `check html-xml FAIL (expected file missing:
   test-iso.xml)`.

Aggregate exit: `0` when every selected check PASSes or SKIPs;
**exit 79 (`EX_TEBAKO_CHECK`)** when any FAILs (the code is allocated
here — this spec's draft said 72, but 72 has been `EX_TEBAKO_TRUST`
since spec 09's trust chain; 79 is the allocation, owned by
`tpkg::EX_TEBAKO_CHECK` and listed in spec 06 §4; 65 keeps
malformed-`checks:`-block, caught at press/validate time by the
schema). Timeout and engine errors are FAILs with the reason
named. `--keep-scratch` preserves the dir for debugging (its path is
printed); otherwise it is removed.

### 2.1 Composition-level checks (the binding's own contract)

A composition document (spec 23 D2 — the org's `tebako.yaml`) can
declare checks for the binding IT assembles. The slice checks prove each
slice in isolation; a composition check proves the slices work TOGETHER —
the org-template slice actually serves the metanorma compile only when
both are mounted and the path is bound right, and no slice's own check
can see that.

```yaml
# tebako.yaml (D2) — metanorma + the org's templates, one config,
# bind AND test
version: 1
runtime: { name: ruby, requirement: "~> 4.0" }
slices:
  - { name: metanorma, requirement: ">= 2.1" }
  - { name: acme-templates, requirement: "3", mount: /templates/acme }
entrypoint: metanorma
policy: deny
mounts:
  - { host: "$CWD", mount: /work, access: ro }
checks:
  org-compile:
    entry: /bin/metanorma             # any mounted slice's executable
    argv: ["--type", "acme", "{scratch}/doc.adoc", "--agree-to-terms"]
    fixtures_inline:                  # small text fixtures, self-contained
      doc.adoc: |
        = Quarterly report
        ACME Secretariat
        :docfile: doc.adoc
        :nodoc:
        :novalid:
    expect: { exit: 0, files: ["doc.xml", "doc.html"] }
    timeout: 180
```

The grammar is the slice grammar plus the two fixture sources a
composition needs (it has no image of its own):

- `fixtures_inline:` — name → content map, written into the scratch
  root. For small text fixtures (the six-line adoc) the composition
  stays self-contained.
- `fixtures_host:` — a path relative to the composition FILE (the org
  repo's checked-in fixtures), copied to scratch. Host-relative, so the
  composition moves with its repo.

Precedence is MECE: `fixtures` (in-image) belongs to slice checks;
`fixtures_inline` / `fixtures_host` belong to composition checks; a
check declaring both families is a named 65. Composition checks run with
the FULL composition mounted and its effective policy in force. A slice
check's `requires:` skips on an absent capability; a composition check
is the author's own contract — its `requires:` behaves identically
(SKIP loud), because the same composition may be checked in reduced
environments (a CI leg without the JVM slice, say).

`tebako check` on a composition runs the slice checks AND the
composition checks, slice-first (a broken slice is diagnosed before the
binding that depends on it). The report groups by owner:
`slice metanorma: html-xml PASS` / `composition: org-compile PASS`.

## 3. The three moments (one definition, three actors)

- **Press-time (dev-build side).** The feedstock's release gate:
  `tebako check out/<triplet>/<slice>.tfs --runtime … --runtime-image …`
  against the just-pressed image and the just-built (or pinned)
  runtime, per platform in the matrix. This generalizes the feedstock's
  `tools/boot_smoke` from "the entrypoint prints its version" to "the
  payload does its one real thing" — boot_smoke's assertions become the
  always-on press validations; the `checks:` block becomes the gate.
- **User side (persona A).** `tebako check metanorma` resolves the same
  composition the shim dispatch would and runs the checks under the
  USER's actual grants, host paths, and proxied filesystem — the
  "does it work on my machine" answer (`brew test` analogue).
  `tebako install --check` runs them post-install as an opt-in install
  verification. Verification at check time is a RUN, not a re-fetch:
  store artifacts stay byte-identical, the store is never mutated.
- **Discovery (the record mode's canonical exerciser).**
  `tebako check --record` runs the checks under spec 23 §8's `record`
  policy; the journal feeds `tfs needs --from-journal`. Because the
  check exercises the payload's real workload, the discovered needs are
  exactly the ones the acceptance path touches — the author reviews the
  draft (ro↔rw, `why:`) and merges it into D1/D2, per spec 23 §8's
  human gate. Spec 25's trace bus carries the same run when armed.

## 4. Worked example — metanorma (the Homebrew test, translated)

Parity source: `homebrew-metanorma/Formula/metanorma.rb`'s `test do`
block (write a minimal doc with `:nodoc: :novalid: :no-isobib:`, compile
`--type iso`, assert the xml + html exist). As declarations on the
metanorma slice:

```yaml
checks:
  html-xml:                        # the brew test, exactly
    entry: /bin/metanorma
    argv: ["--type", "iso", "{scratch}/test-iso.adoc", "--agree-to-terms"]
    fixtures: /__tpkg__/check/html-xml   # holds test-iso.adoc (6 lines)
    expect: { exit: 0, files: ["test-iso.xml", "test-iso.html"] }
    timeout: 180
  pdf:                             # the level the brew test comments out
    entry: /bin/metanorma
    argv: ["--type", "iso", "--extensions", "pdf",
           "{scratch}/test-iso.adoc", "--agree-to-terms"]
    fixtures: /__tpkg__/check/html-xml
    expect: { exit: 0, files: ["test-iso.pdf"] }
    requires: { provides: [jvm] }  # mn2pdf spawns the JVM (spec 22 class E);
                                   # no JVM slice in the composition ⇒ SKIP
    timeout: 300
```

The composition (`tebako.yaml`, spec 23 §10) decides which levels RUN:
the bare metanorma slice checks `html-xml`; metanorma + openjdk checks
both. The fonts question rides spec 23 needs (a fontist cache declared
`rw`, or fonts as a data slice) — the check verifies whichever the
composition declares, because the compile touches fonts only when the
document asks for them.

The slice-builder's side of the same frame: the RUBY runtime slice
carries its own `boot-and-stdlib` exec check (§1.1, `entry: self`), and
the org that binds metanorma + its own templates data slice declares the
`org-compile` composition check in the one config that does the binding
(§2.1) — build, bind, and test are three declarations in two files,
never code.

## 5. Relation to the neighbors (the MECE table)

| Concern | Owner |
|---|---|
| Runtime acceptance (interpreter + driver + interposition) | spec 22 §8 — the factory dogfood |
| Needs / composition declaration | spec 23 (D1/D2/D3) |
| **Payload acceptance, per slice kind (this)** | **spec 26 §1** |
| **The binding's acceptance (org compositions)** | **spec 26 §2.1** |
| Observability of any run (incl. a check run) | spec 25 |
| Write areas the check's scratch needs beyond the engine grant | spec 24 |
| Check-name discovery | spec 15 (`tfs info` reads L1) |

The ruby repo's `ci/spec22-gems` probe is a hand-rolled instance of this
contract (install → press → boot → run → assert markers); it stays the
runtime factory's acceptance. Feedstocks get the declarative form.

## 6. Platform notes

Checks run per platform in feedstock CI (the matrix), and user-side on
whatever the user has. The metanorma feedstock's `tools/smoke_verdict`
windows gaps are CLOSED by the incident-12/13 work — G1 (drive-relative
re-rooting) by the spec-17 uniform namespace, G2 (no dynamic native
extensions) by the runtime DLL + loader interpose + `library_aliases:`
(round 8, 2026-08-19: `require "sassc"` + compile green on
windows-ucrt64). The windows leg flips to enforcing when the
current-shape runtime ships; the `checks:` gate then subsumes
boot_smoke/smoke_verdict on every platform.

## 7. Error discipline

- Malformed `checks:` block: schema error at press/`tfs validate` for a
  slice, at composition load for a D2 document, exit 65 — never
  discovered at run time.
- Check FAIL: exit 79 (`EX_TEBAKO_CHECK` — re-allocated from this
  spec's draft 72, which has been `EX_TEBAKO_TRUST` since spec 09),
  the check name and the failed expectation named
  (missing file, exit code, stdout pattern, timeout) — never a bare
  nonzero.
- SKIP is loud and always names the unmet prerequisite; a SKIP never
  fails a gate.
- The engine's own failures (resolution, mount, jail bind) ride the
  existing named codes (65/68/69/73/77/78).
- Non-goals: no central check registry, no fetched checks (the image is
  the trust boundary), no byte-golden assertions, no store mutation
  during a check.
