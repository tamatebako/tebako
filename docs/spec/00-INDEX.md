# Tebako Specifications — Index

This directory is the **normative specification set** for the tebako
packaging and loading ecosystem. It supersedes prose in READMEs and the
historical `TODO.restructure/` plans. Code that disagrees with these specs
is wrong; a spec that disagrees with shipped, tested reality is stale —
fix one of them in the same PR. Unshipped behavior is marked **PLANNED**;
partial coverage **PARTIAL**; shipped and tested **SHIPPED**.

## Vocabulary

- **payload** — a single TFS image (`.tfs`) with an in-image manifest.
- **runtime** — a payload of kind `runtime`: provides an interpreter.
- **package / tpkg** — the three-part executable: bootstrap + slots + trailer.
- **bootstrap** — the Rust loader (`tebako-bootstrap`), process entry point.
- **TFS** — the userland virtual filesystem layer (spec 11).
- **shim** — a registered command on PATH that dispatches into a payload.

## Layer model (every concept lives at exactly one layer)

| Layer | Name | Contents | Spec |
|-------|------|----------|------|
| L0 | Wire container | tpkg trailer, slots, bootstrap bytes | 02 |
| L1 | Payload manifest | IDENTITY / PROVIDES / DEPENDS, in-image YAML | 03 |
| L2 | Package semantics | runtime role, entrypoint choice, jail policy, mount composition | 03, 07, 08 |
| L3 | Resolution & trust | references, registry, cache, mirrors, signatures, encryption | 04, 05, 09, 10 |

**Orthogonality law:** runtime-or-not and entrypoint location are
orthogonal to the image format. `format_id` answers only "how do I read
these bytes". Roles are L2, declared by manifests — never encoded in the
format axis (the v1 `TPKG_FORMAT_RUNTIME` slot type is a legacy role wart,
spec 02 §6).

## Reading order

1. [01 — System overview](01-overview.md) — what tebako is, repos, crates, capabilities
2. [02 — tpkg wire format](02-tpkg-wire-format.md) — byte-exact container spec
3. [03 — Payload manifest](03-payload-manifest.md) — IDENTITY / PROVIDES / DEPENDS
4. [04 — References and registries](04-references-and-registry.md) — MECE reference syntax
5. [05 — Resolution and cache](05-resolution-and-cache.md) — runtime_ref, release index, machine cache
6. [06 — Launcher ABI](06-launcher-abi.md) — bootstrap → runtime handoff, exit codes
7. [07 — Shims and dispatch](07-shims-and-dispatch.md) — executable registration and version management
8. [08 — Jails](08-jails.md) — host-access policy, bind-mount semantics
9. [09 — Trust and signing](09-trust-and-signing.md) — chain of trust, authenticated releases
10. [10 — Encryption](10-encryption.md) — encrypted volumes, key model, PQC
11. [11 — TFS virtual filesystem](11-tfs-vfs-model.md) — mounts, backends, COW, encapsulation
12. [12 — Comparisons with other technologies](12-comparisons.md) — tebako vs the field
13. [13 — Factories and releases](13-factories-and-releases.md) — source/runtime factories, drift loop, pipelines
14. [14 — Engineering process](14-process.md) — design → implementation → validation order, coding rules
15. [15 — The info surface](15-info-command.md) — payload and package introspection (`tfs info` / `tebako-pkg info`), verification exit codes, JSON contract
16. [16 — Distribution and installation](16-distribution-and-installation.md) — personas, channels (brew/curl|sh/tebako install), slim/fat, trust per channel
17. [17 — Runtime driver contract](17-runtime-driver-contract.md) — the language-agnostic loader↔runtime surface (argv, env, IO, exit codes)
20. [20 — LimniFS backend](20-limnifs-backend.md) — image format 5: detection, the backend adapter contract, backend cargo features, the writer path (PLANNED)
21. [21 — Crypto consolidation](21-crypto-consolidation.md) — one crypto home per layer: OpenPGP keeps trust and identity, ENC keeps confidentiality, limnifs-native crypto is evidence, never anchor (PROPOSED DECISION)
22. [22 — Runtime-native interposition](22-runtime-native-interposition.md) — the generalized hooks: loader/exec/resource interposition inside the runtime, the documented interface, and the death of per-gem adapters (the spec-18 contract's runtime-internal half)
23. [23 — Declarative composition and needs resolution](23-declarative-composition.md) — the fully declarative slice stack: D1 needs / D2 composition doc / D3 press-baked union / D4-D5 operator surfaces; deny-safe by default, the needs-check law (PLANNED)
24. [24 — Declarative overlays](24-declarative-overlays.md) — write areas and key bindings: the gated COW write gate, `TEBAKO_OVERLAYS` / `TEBAKO_DECRYPT`, the record-mode fold-in, exit 68 (PARTIAL)

## Locked invariants (all specs subordinate to these)

1. No shell-outs, no system dependencies, in any shipped artifact.
2. Loader size gate < 3 MB per platform, enforced in CI.
3. Tebako-owned C/C++ only in `dwarfs-t` (upstream ruby's own C source
   is vendored, not ours). Everything else Rust / pure Ruby / Docker.
4. Orthogonality law (above).
5. Transforms law: write (COW) and encryption (ENC) overlays exist ONLY in
   the Rust TFS. dwarfs-t is read-only + creation-time Writer; no backend
   learns to write.
6. YAML for all authored config/manifests; versioned JSON Schemas.
7. Signing and encryption are per-package opt-in; verification of signed
   artifacts is always strict; v1 readers never break on v2 packages.
8. Golden parity vs any C++/gem predecessor, or a documented deviation.
9. MECE reference syntax; no default service; named errors and named exit
   codes, never silent fallbacks.
