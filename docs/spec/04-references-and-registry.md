# Spec 04 — References and registries (L3)

Normative specification of how payloads are named, located, and listed.
Status: syntax LOCKED; resolver implementation PARTIAL (roadmap 07).

## 1. The reference syntax (MECE, no default service)

One scheme family; the adapter is explicit in the scheme, never inferred:

```
SERVICE adapters:
  tfs:github:owner/repo:version        → GitHub releases API
  tfs:gitlab:owner/repo:version        → GitLab releases/packages API
  tfs:bb:owner/repo:version            → Bitbucket downloads API

PROTOCOL adapters:
  tfs+git://host/owner/repo.git@<ref>#<path-in-repo>
      (git protocol, any host, LFS-aware; #path selects the file when a
       repo holds many images; optional when the repo IS the registry)
  tfs+https://cdn.example.com/tool.tfs → verbatim fetch

LOCAL:
  file:///opt/images/tool.tfs

DIGEST PIN (any class, query form — never clashes with #path):
  ...?sha256=<hex>
```

**Dispatch rule** (deterministic, exactly one home per reference):

| scheme | adapter |
|--------|---------|
| `tfs:github:` / `tfs:gitlab:` / `tfs:bb:` | that service's API |
| `tfs+git:` | git protocol adapter |
| `tfs+https:` (or bare http(s)) | http fetch |
| `file://` | local file |
| anything else | **named error listing the classes** |

There is **NO default service or namespace** anywhere: every reference is
fully explicit. (Superseded forms — `tfs://github.com/owner/repo:version`
and any host-inferred shorthand — are rejected, never guessed.)

A repo may be a COLLECTION: `#path` addresses any image inside it, and the
repo's `tpkg-registry.yaml` lists them all. Manifest `requires` entries
(spec 03) resolve through `{ref, constraint, mount}`.

## 2. The registry (developer-hosted, zero central infrastructure)

A **registry** is ANY git host repo carrying `tpkg-registry.yaml` (in-repo
or attached to its releases). The git host's releases ARE the storage —
no server to run.

```yaml
schema_version: 1
payloads:
  - name: metanorma
    kind: app
    versions:
      - version: 1.2.3
        platforms: universal            # or per-triplet entries
        ref: tfs:github:metanorma/metanorma:1.2.3
        sha256: "…"
        signature: {keyid: "…", asc: "metanorma-1.2.3.tfs.asc"}
        runtime_requirement: {engine: ruby, constraint: ">= 3.3, < 5.0"}
        entrypoints: [metanorma]
    default: 1.2.3
```

- The registry MIRRORS only resolution-relevant fields (spec 03 §4 tier 3)
  — the dispatcher resolves without downloading every payload.
- `tebako add-registry <ref>` registers one; our own tools ship built-in
  defaults.
- Install = resolve the manifest → download payload (signed per spec 09)
  → verify → cache (`~/.tebako/payloads/<name>/<version>.tfs`) → link the
  shim(s) (spec 07).
- A developer's release flow: press → sign → upload payload + update
  `tpkg-registry.yaml` in their repo (a `tebako publish` helper later;
  manual first).

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
