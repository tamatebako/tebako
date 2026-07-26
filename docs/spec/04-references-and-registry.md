# Spec 04 — References and registries (L3)

Normative specification of how payloads are named, located, and listed.
Status: syntax LOCKED (multi-artifact + repo/release-class rules locked
2026-07-26); resolver implementation PARTIAL (roadmap 07/28).

## 1. The reference syntax (MECE, no default service)

One scheme family; the adapter is explicit in the scheme, never inferred:

```
SERVICE adapters (the service's ARTIFACT storage — never the git tree):
  tfs:github:owner/repo:version[#artifact]   → GitHub releases API
  tfs:gitlab:owner/repo:version[#artifact]   → GitLab releases/packages API
  tfs:bb:owner/repo:version[#artifact]       → Bitbucket downloads API

PROTOCOL adapters:
  tfs+git://host/owner/repo.git@<ref>#<path-in-repo>
      (the GIT SOURCE — any host incl. github/gitlab/bitbucket, LFS-aware;
       #path selects the file when a repo holds many images; optional when
       the repo IS the registry)
  tfs+https://cdn.example.com/tool.tfs → verbatim fetch

LOCAL:
  file:///opt/images/tool.tfs

DIGEST PIN (any class, query form — never clashes with #artifact/#path):
  ...?sha256=<hex>
```

**Repo vs releases is a class distinction, never a flag.** The git tree
is addressed ONLY by `tfs+git:`; release artifacts ONLY by
`tfs:<service>:`. Both are supported by design — a service form never
means "the repo", a git form never means "the release".

**Multi-artifact releases (locked — no magic):**

- `#artifact-name` selects one artifact within the release (same `#`
  convention as `#path-in-repo`).
- No `#`: the candidate class is `.tfs` images. Exactly ONE candidate →
  used (there is no choice to guess). ZERO → `AssetNotFound`. MORE THAN
  ONE → `AmbiguousAssets` naming every candidate so the user re-runs
  with `#name`. The adapter NEVER auto-picks by host triplet — platform
  selection is the registry's declarative job (§2).

**Dispatch rule** (deterministic, exactly one home per reference):

| scheme | adapter |
|--------|---------|
| `tfs:github:` / `tfs:gitlab:` / `tfs:bb:` | that service's releases API |
| `tfs+git:` | git protocol adapter |
| `tfs+https:` (or bare http(s)) | http fetch |
| `file://` | local file |
| anything else | **named error listing the classes** |

There is **NO default service or namespace** anywhere: every reference is
fully explicit. (Superseded forms — `tfs://github.com/owner/repo:version`
and any host-inferred shorthand — are rejected, never guessed.)

Manifest `requires` entries (spec 03) resolve through `{ref, constraint,
mount}`.

## 2. The registry (developer-hosted, zero central infrastructure)

A **registry** is ANY git host repo carrying `tpkg-registry.yaml`. The
git host's releases ARE the storage — no server to run.

**Registry resolution (locked):** `tfs:<service>:owner/repo` (no version)
resolves the registry file from the repo's DEFAULT-BRANCH ROOT via the
service contents API (`/tpkg-registry.yaml`); the pinned-immutable form
`tfs:<service>:owner/repo:version#tpkg-registry.yaml` reads it as a
release artifact (versioned with its payloads); `tfs+git://…#path` reads
it from any git ref/path. No other locations, no search, no fallback
chain — exactly one location per form.

```yaml
schema_version: 1
payloads:
  - name: metanorma
    kind: app
    versions:
      - version: 1.2.3
        platforms:                    # per-triplet for native-ext apps;
          x86_64-linux-gnu:           # "universal" for pure-language
            artifact: metanorma-1.2.3-linux-gnu-x86_64.tfs
            sha256: "…"
          aarch64-macos:
            artifact: metanorma-1.2.3-macos-arm64.tfs
            sha256: "…"
        release: {ref: tfs:github:metanorma/metanorma:1.2.3}
        signature: {keyid: "…", asc: "…"}        # opt-in
        runtime_requirement: {engine: ruby, constraint: "~> 3.3.0"}
        entrypoints: [metanorma]
    default: 1.2.3
```

- The registry MIRRORS only resolution-relevant fields (spec 03 §4 tier 3)
  — the dispatcher resolves without downloading every payload.
- **Host-triplet selection happens HERE, declaratively**: the dispatcher
  reads `platforms[host_triplet].artifact` (or `universal`), fetches THAT
  artifact from the named release. Never adapter-side guessing.
- `tebako add-registry <ref>` registers one; shipped config has ZERO
  registries (explicit only — spec 16).
- Install = resolve the registry → select the host entry → download →
  verify (sha256 and/or OpenPGP signature, spec 09) → content-addressed
  cache (`~/.tebako/payloads/<name>/<version>.tfs`) → register shims
  (spec 07).
- A developer's release flow: press → sign → upload payloads → commit
  `tpkg-registry.yaml` (a `tebako publish` helper later; manual first).

## 3. Fetch discipline

- In-process HTTP only (spec 01): ureq + rustls + webpki-roots bundled;
  OS roots opt-in via `TEBAKO_TLS_PLATFORM_ROOTS`; `file://` mirrors for
  tests and air-gapped sites.
- Git protocol via `gix` (gitoxide) when `tfs+git:` needs smart-protocol
  fetch — never the git CLI.
- Atomic fetch: download to tmp, verify digest, rename; concurrent
  fetchers coordinate via the cache lock (spec 05); a partial fetch is
  invisible. `TEBAKO_OFFLINE=1`: cache hit or hard error.
- Digest mismatch → the named sha error (exit 70); nothing enters the
  cache.
