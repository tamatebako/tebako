# AGENTS.md — Working rules for tebako-rs (and all tamatebako repos)

These rules are owner-locked (2026-07-27). They apply to humans and agents alike.

## Pull requests

- **One PR per repository at a time.** Never queue several open PRs per repo.
- **Merge immediately when checks pass.** No sitting PRs — a green check
  means merge now, not later. Work intended for main lands through one
  branch at a time; rebase, verify, merge.
- Fix forward on main when instructed ("merge now, fix after") — the
  follow-up fix is its own immediate item.

## Work decomposition

- **Stack work into big coherent items** — no many little tasks. An item
  covers a subsystem end-to-end (model + CLI + tests + docs), not a file
  or a function. If two tasks touch the same crates, they are one item.

## Agent worktree discipline

- Parallel work happens in `git worktree`s, one per agent
  (`git worktree add <path> -b <branch> origin/main`). **Never share the
  main checkout** — agents checking out branches in it clobber each
  other (this caused real bugs: commits landing on the wrong branch).
- **Never grep/patch against the main checkout assuming it is the
  head** — it may be arbitrarily behind `origin/main`. Investigate in a
  fresh worktree. A stale-tree grep once "proved" a load-bearing
  dependency unused.
- `GIT_EDITOR=true` for every git operation (no interactive editors).
- Before pushing in a shared repo, check `git branch --show-current` and
  `git fetch origin`; push the intended ref (`git push origin <sha>:refs/heads/main`
  when the checkout is elsewhere).

## Hard technical rules (owner-locked)

- **C/C++ exists only in three factory repos**: tamatebako/ruby,
  tebako-runtime-ruby, dwarfs-t. Everything else is Rust, pure Ruby, or
  Docker. No CMake anywhere else.
- **No shell-outs, no system dependencies in shipped artifacts.** In-process
  HTTP (ureq+rustls+webpki-roots bundled), in-process imaging (dwarfs-t
  Writer), in-process crypto (rnp-rs vendored). No curl/git/mkdwarfs/PATH
  lookups at runtime.
- **Optional capability = cargo feature on the owning crate** — default ON
  for the toolchain, OFF for the size-gated `tebako-bootstrap`, and a
  NAMED error when compiled out (first instance: tebako-resolve's `git` →
  `GitAdapterDisabled`). Consequence: **the bootstrap always builds in its
  own `cargo build` invocation** — cargo unifies features within one
  invocation, and building the bootstrap beside tebako-cli/tebako-shim
  silently re-enables opted-out features in it (v0.2.7 release run
  32940980101 failed the 3 MiB gate on every leg for this reason).
- **The workspace `Cargo.lock` is gitignored — resolutions float.**
  Dependencies that evolve incompatibly inside one semver line get an
  explicit upper bound on stable branches; the bump branch raises the
  floor and drops the bound in the same change.
- **YAML for all authored config/manifests**, with versioned JSON Schemas.
- **MECE reference syntax, no default service**: `tfs:github:/tfs:gitlab:/
  tfs:bb:` / `tfs+git://` / `tfs+https://` / `file://`, `?sha256=` pins.
  Unparseable = named error, never a guess.
- **Signing/encryption opt-in**; verification of signed artifacts always
  strict; v1 readers never break on v2 packages.
- **COW/ENC transforms live ONLY in the Rust TFS.** dwarfs-t is read-only
  + creation-time Writer; no backend learns to write.
- **Golden parity**: anything ported from a C++/gem predecessor is
  byte-compared or the deviation is documented in the README.

## Ruby rules (tamatebako/ruby, tebako-runtime-ruby tooling)

Autoload only (defined in the immediate parent namespace file) — no
`require_relative`/in-library `require`; no `send`; no
`instance_variable_get/set`; no `respond_to?`; model-driven, MECE,
open/closed, DRY; specs for public behavior.

## Build/test invocation rule (the libtfs.rlib collision)

- **Never run `cargo test --release` locally.** The workspace release
  profile sets `panic = "abort"`; cargo builds test-harness units with
  `panic=unwind`, so every lib reachable from a test target compiles
  TWICE, and both tfs units stamp the shared unhashed `libtfs.rlib`
  (crate-type `["cdylib","staticlib","rlib"]` + cargo issue 6313). The
  result is a rotating cast of E0308/E0460/E0463 that looks like a code
  bug and is not. Test in debug (CI's shape) — the durable fix is
  roadmap 71 (tfs-capi split).

## Verification culture

- No item is "done" without its acceptance executed and green — cite run
  ids, byte counts, nm dumps. Evidence over assertions.
- Verify the MERGED content, not just the branch (diff merged trees).
- A failing assertion maps to exactly one CI tier; tiers never repeat
  each other.
- Patches in tamatebako/ruby must compile before they release
  (compile-smoke gate; the v0.2.8 lesson).

## The spec set

Normative behavior lives in `docs/spec/00-INDEX.md` (00–17). Contract
changes land there FIRST (with wire diagrams and named errors), then
code. Unshipped behavior is marked PLANNED.
