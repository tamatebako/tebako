# Spec 04 — References and registries (L3)

Normative specification of how payloads are named, located, and listed.
Status: syntax LOCKED (multi-artifact + repo/release-class rules locked
2026-07-26); resolver + registry listing + `tebako install` SHIPPED
(roadmap 07/28.1); `tebako publish` SHIPPED (roadmap 41 — GitHub
releases + file:// mirrors; the GitLab/Bitbucket write legs and the
brew/install.sh channels remain PLANNED).

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
- **Concrete rows only (locked 2026-08-25, tebako#460).** A version
  carries a `universal:` row OR per-triplet rows — never patterns
  (`linux-*-*` is rejected at validation), never both. Identical bytes
  under multiple rows dedup naturally (content-addressed cache). A
  composition's `platforms:` assertion (spec 23 §13.3) CONSTRAINS the
  mirrored coverage — checked fail-closed against it — and NEVER
  extends it.
- **`kind: runtime` entries are edge-discoverable** (schema MINOR 1;
  PLANNED — TODO.v2-1/30): the entry carries `engine:` (+ optional
  `implementation:`), and a payload's `kind: runtime` DEPENDS edge
  (spec 30 §1) discovers it by (engine, implementation?, constraint);
  the matched version's `release.ref` derives the download base
  (spec 05 §2's per-engine chain, the zero-config path for third-party
  runtimes). Runtime entries predating MINOR 1 carry no `engine:` and
  stay invisible to edges.
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
- The capability is per-binary, not per-ecosystem. The shipped
  `tebako-bootstrap` (the 3 MiB-gated loader) links tebako-resolve with
  the `git` feature OFF: `tfs+git:` lock sources fail closed with the
  named `GitAdapterDisabled` error (steer to managed mode — `tebako
  install` / the shim — or mirror to `tfs+https:`). The toolchain
  (tebako CLI, tebako-shim, tfs CLI) always carries the full surface.
  General rule: an optional capability is a cargo feature on the owning
  crate — default ON for the toolchain, OFF for the bootstrap, named
  error when compiled out.
- Atomic fetch: download to tmp, verify digest, rename; concurrent
  fetchers coordinate via the cache lock (spec 05); a partial fetch is
  invisible. `TEBAKO_OFFLINE=1`: cache hit or hard error.
- Digest mismatch → the named sha error (exit 70); nothing enters the
  cache.

## 4. Enterprise networking — proxies and trust anchors (2026-09-06 amendment)

Every fetch the loader or the toolchain makes rides ONE client
(`tebako-http::build_agent`); the policy below is resolved once per
process from the environment merged over the `network:` section of
`~/.tebako/config.yaml` — **env wins per key** — and installed before
the first request (agent construction caches the transport). The
resolution is journaled (`event=network-config …`) with proxy
credentials REDACTED (`http://***@host:port`). There is no verify-off
spelling, and there never will be.

**Proxy.**

- Env: `HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY` / `NO_PROXY` (both
  cases), the de-facto grammar: comma list, suffix match, `*`;
  localhost is always direct. Config mirror: `network.proxy: <url>`.
- Precedence: env → config → direct. Credentials ride the URL
  (`http://user:pass@host:port`) — the only auth spelling.
- HTTPS rides CONNECT through the proxy; TLS terminates at the TARGET.
  An inspecting proxy re-encrypts — that is what the trust half below
  is for.
- Named errors, never a silent fallback: `ProxyUrlInvalid`,
  `ProxySchemeUnsupported` (no SOCKS), `ProxyAuthRequired` (a 407 the
  URL's credentials did not satisfy). Configuration errors are
  deterministic: retry loops return them immediately, never as
  `IndexUnavailable`, never retried.

**Trust anchors** — three spellings, all fail-closed:

1. `webpki` (default, unchanged): the bundled Mozilla roots.
2. `platform`: the OS store (GPO/MDM-pushed enterprise roots) — env
   `TEBAKO_TLS_PLATFORM_ROOTS` (since v2.0.0) or
   `network.tls_roots: platform`.
3. Additive: `network.extra_ca: [<pem>…]` / `TEBAKO_EXTRA_CA`
   (os-path-list separator) — the PEMs parse at client build INTO the
   bundled store (never instead of it). Combining `platform` with
   `extra_ca` is the named `ExtraCaWithPlatformRoots` error (the OS
   verifier trusts exactly the OS store — push the CA there instead).
   A malformed or unreadable PEM is `ExtraCaMalformed` /
   `ExtraCaUnreadable` at startup — never a silently skipped file.

**Capability gating** (§3's general rule applied): `network` is a
cargo feature on tebako-http — default ON for the toolchain, OFF for
the size-gated bootstrap. Compiled out, a set proxy env or
`TEBAKO_EXTRA_CA` fails closed with the named `NetworkingCompiledOut`
error identifying the variable; the bootstrap never silently ignores a
policy the operator asked for. `TEBAKO_TLS_PLATFORM_ROOTS` is NOT part
of the feature — the platform-verifier opt-in shipped in v2.0.0 and
stays always-on in every build, bootstrap included. The config mirror
is read by the toolchain (shim / CLI) at startup; the size-gated
bootstrap reads env policy only — enterprise fleets push policy as
system env (GPO/MDM), which both channels honor.

```yaml
# ~/.tebako/config.yaml — every key optional; env wins per key
network:
  proxy: http://user:pass@proxy.corp:3128  # CONNECT proxy
  tls_roots: platform                       # default: webpki
  extra_ca: [/etc/pki/corp-root.pem]        # additive; not with platform
```

The payload side (a packaged Ruby/Python process making its OWN TLS
connections) is out of the loader's scope: it rides the runtime's
`SSL_CERT_FILE` semantics — spec 22 §4.
