# Spec 14 — Engineering process

The mandatory order for every change to this ecosystem, and the coding
rules that apply in every repo.

## 1. Design → implementation → validation

1. **DESIGN, spec-first.** Contract changes land in this spec set (or a
   linked `docs/*.md`) and, where applicable, a versioned schema —
   BEFORE code. Wire formats get byte diagrams; behaviors get named
   errors and exit codes up front; every new concept is assigned to
   exactly one layer (L0–L3) — the orthogonality review happens here.
2. **ORACLE PIN.** If a C++/gem predecessor exists, generate golden
   vectors/fixtures from it. No golden oracle → no parity claim.
3. **IMPLEMENT** in the owning crate/module only (MECE); invariants
   (spec 00) hold; `unsafe` only inside FFI boundary modules.
4. **UNIT + PROPERTY TESTS** in-crate (proptest: parsers never panic;
   round-trips are identity).
5. **CONTRACT / PARITY SUITE.** The ported C++ corpus runs against the
   Rust implementation (`tests/contract`, 164 tests); parity legs diff
   byte-for-byte.
6. **E2E, TIERED.** PR tier: one smoke per OS with a cached runtime
   (< 10 min). Nightly: full matrix. Weekly: adversarial (cold cache,
   network failure, corrupted downloads, jail enforcement).
7. **SIZE + HYGIENE GATES.** Bootstrap size table per platform; exported
   symbol audit (only the `tebako_*` surface leaks); clippy/fmt.
8. **RELEASE + VERIFY.** Tag → per-platform artifacts → SHA256SUMS →
   completeness gate → `tebako-pkg verify` on published assets.
9. **DOC SYNC.** The spec set and README status sections update in the
   same PR; unshipped behavior is marked PLANNED.

## 2. Ruby rules (tamatebako/ruby, tebako-runtime-ruby tooling)

- **Autoload only** — no `require_relative`, no in-library `require`;
  each namespace declares children with `autoload` in the immediate
  parent namespace's file (create it if missing).
- **No `send`/`__send__`** to private methods — test public behavior or
  make the collaborator public by design.
- **No `instance_variable_get`/`instance_variable_set`** — state enters
  via the public constructor/API.
- **No `respond_to?`** — duck-typing by smell; model classes with
  explicit, semantically-named interfaces.
- OOP/MECE, model-driven, open/closed, DRY; specs assert observable
  outcomes (files, bytes, exit codes, outputs), never mocks-of-mocks.

## 3. Rust rules (tebako-rs, dwarfs-rs)

- No shell-outs, no system/CLI dependencies: in-process HTTP
  (ureq+rustls+webpki-roots bundled; OS roots opt-in), in-process imaging
  (dwarfs-t Writer binding), in-process crypto (rnp-rs vendored).
- YAML for authored config/manifests; MECE reference syntax, no default
  service; signing/encryption strictly opt-in.
- Bootstrap discipline: `opt-level="z"`, `lto="fat"`, `codegen-units=1`,
  `panic="abort"`, `strip="symbols"`; no async runtime, no clap, no
  logging framework; `cargo bloat` in CI; the 6 MB gate is hard.
- Named errors everywhere on malformed input (no unwraps on
  trailer/exec/network paths).

## 4. Factory C/C++ rules (the three repos only)

- Modern `tebako_fs_*` API only at consumption seams (nm gates; no legacy
  symbols).
- dwarfs-t: reader + Writer behind the stable `dwarfs_c_*` C ABI; no C++
  headers/templates leak to consumers; read-only at runtime forever.
- The runtime driver and the ruby io-routing patches are the ONLY
  tebako-specific C/C++ consumers.

## 5. Verification culture

- No item is "done" without its acceptance criteria executed and green.
  Evidence over assertions — cite run ids, byte counts, nm dumps.
- Merged content is verified, not just branch content (diff merged trees
  before pushing).
- A failing assertion maps to exactly one CI tier — no redundant
  failures, no tier repeats another tier's assertions.
