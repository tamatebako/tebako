# The remote registry

The remote registry is how slices are published by authors and found by
users. It is deliberately lightweight: there is no central server to
operate. A registry is a small YAML index hosted anywhere a file can be
served — a GitHub repository's releases, a GitLab or Bitbucket project,
any git repository, any plain HTTPS location, or a local directory.

## What a registry is

A registry is one index file, `tpkg-registry.yaml`, plus the slice
files it references. The index declares the publisher's signing key and
lists its payloads: for each one, the available versions, the platforms
each version covers (or a universal marker), and for each artifact its
URL, size, checksum, and optional signature. The format is versioned so
older readers keep working.

An author publishes by building slices in CI and uploading them with
the updated index — the publish step signs every artifact and writes
the index's signing block automatically. A project with its own GitHub
organization needs no other infrastructure.

## How users reference things

References are exact and spell out their transport, never implied from
a default:

- `tfs:github:<org>/<repo>:<version>` — a GitHub release
- `tfs:gitlab:<org>/<repo>:<version>` — a GitLab release
- `tfs:bitbucket:<org>/<repo>:<version>` — a Bitbucket release
- `tfs+git://<host>/<path>.git[@<ref>][#<path-in-repo>]` — a git
  repository, with an optional branch/tag and a path inside it
- `tfs:https://<host>/<file>` — a plain HTTPS artifact (a checksum can
  be pinned in the URL)
- a local path or `file://` — used in place, nothing downloaded

There is no default registry and no shorthand that assumes one: every
reference says exactly where to go.

## How resolution works

The user registers a registry once:

```
tebako add-registry tfs:github:metanorma/metanorma
```

Registrations can carry local policy (spec 37 — the "registry book"):
`--name <alias>` gives the registry a short local handle for the
qualified `alias/name` install form, `--require-signed` makes every
install through it fail closed without a verifying signature (the
named `UnsignedRegistryPayload`, exit 70), and
`--default` marks the publish target when more than one registry is
registered:

```
tebako add-registry tfs:github:metanorma/metanorma-flavor-nist --name nist --require-signed
```

`tebako list-registries` shows the book — each registry's alias,
reference, policy flags, and cache freshness.

When more than one registered registry carries the same payload name,
an unqualified install is the named `AmbiguousRegistries` error by
design (never a priority pick). The qualified form scopes resolution
to exactly one registry by its alias:

```
tebako install nist/metanorma-bsi        # only the nist registry is consulted
```

Pins can carry the same scope: a project's `.tebako-tools.yaml` or the
`defaults:` map form accepts `registry: <alias>` naming the registry
the pin resolves through — an alias that matches no registered
registry is the named `UnknownRegistryAlias`, listing the book's
aliases.

Dependency edges carry the scope too: a payload manifest's `requires:`
entries (toolkit, data, runtime, executable) accept `registry: <alias>`
(spec 37 §3), so a payload built against a private registry resolves
its dependencies — runtimes included — through exactly that registry.
A scoped runtime edge never falls back to the public default line, and
a scoped registry marked `--require-signed` refuses an unsigned runtime
row with the same named `UnsignedRegistryPayload` as a payload row.

Runtime discovery is federated (spec 37 §8): a registry's runtime rows
resolve their downloads from GitHub, GitHub Enterprise, and GitLab
(SaaS or self-hosted) release references alike — the per-service
download-URL shape derives from the row's `release.ref`, never from a
hand-configured mirror.

Once a payload is installed, the store remembers WHICH registry
resolved it (spec 37 §7's origin binding — a `.tfs.registry` marker
next to the artifact, holding the registry's canonical reference). From
then on, bare installs and dispatch-time version chains for that
payload consult its origin registry only: a same-named payload showing
up in another registered registry is never silently treated as a newer
release of yours. Switching is an explicit act — `tebako install
<alias>/<name>` reinstalls and rebinds the origin, and the journal
records the switch (`event=origin-rebind`). If the new registry's
declared digest disagrees with the bytes already installed, the rebind
is refused (`Sha256Mismatch`, exit 70): uninstall first, then install
from the new registry — bytes are never re-tagged to a registry that
never published them. An authored `registry:` scope on a pin outranks
the binding, exactly like the qualified install form.

The index is fetched, its signing key is shown for confirmation, and
the key is pinned to that registry. After that:

- `tebako install metanorma` resolves the name against the registered
  registries, picks the newest version (or a pinned one), verifies the
  signature against the pinned key, and installs.
- Registry indexes are cached locally with a time-to-live and refreshed
  on demand; offline mode uses the cache or fails with a named error.

Trust is anchored by confirmation, not by infrastructure: the first
fetch shows the publisher's fingerprint, the user confirms it, and
every later artifact from that identity must verify against the pinned
key. A key that changes unexpectedly is a hard failure naming both
fingerprints; legitimate rotation carries a signed successor statement
that forward-verifies.

## Credentials for private registries (spec 37 §5)

Registries and payloads hosted on private repositories need a token.
The `credentials:` section of `~/.tebako/config.yaml` declares them —
by environment variable NAME only; a secret never appears in the
config file:

```yaml
registries:
  - ref: tfs:github:acme/private-flavors
    name: acme
credentials:
  - registry: acme              # tier 1: keyed by the registry's alias
    token_env: ACME_GH_TOKEN
  - host: ghe.corp.internal     # tier 2: keyed by an exact host
    token_env: GHE_TOKEN
```

Each entry names exactly one selector — `registry:` (a registry-book
alias) or `host:` (an exact host, ports allowed). Both or neither is a
named config error (`InvalidCredentialEntry`, exit 65), and so is a
malformed `token_env` or two entries naming the same selector
(`DuplicateCredentialSelector`, both entries named). Config validation
is fail-closed at load, before any fetch.

At fetch time the lookup is two-tier, then ambient, then anonymous:

1. A tier-1 entry whose alias is the registry that directed the fetch —
   and only within its **confinement**: the hosts of that registry's
   own reference (a GitHub registry confines to `api.github.com` and
   `github.com`; a self-hosted ref confines to its one host). The token
   never leaks to another host: a confined-out URL falls through to the
   host chain, never to the tier-1 token.
2. A tier-2 entry matching the URL's exact host (no suffix games — a
   subdomain of a named host is a different host).
3. The ambient `TEBAKO_GITHUB_TOKEN` / `GITHUB_TOKEN`, on the GitHub
   API host only (the pre-spec-37 behavior).
4. Anonymous.

The header shape follows the service: `Authorization: Bearer` for
GitHub (and for plain-HTTPS hosts), `PRIVATE-TOKEN` for GitLab, and
HTTP Basic of the raw `user:app-password` value for Bitbucket. A
matched entry whose env var is unset attaches nothing and stops the
chain — there is no fall-through past a named credential.

Every credentialed fetch journals its decision to `journal.log` as
`event=fetch host=<host> credential=<class>` where the class is
`token:env:<VAR>` or `anonymous` — the credential CLASS only, never a
value. Book-empty machines journal nothing new.

A service's 401/403 is never retried as a download failure: it becomes
the named `CredentialRequired` (exit 69), naming the host, the
directing registry's alias, the env var a matched entry wanted, and the
`credentials:` steer.

## Relationship to everything else

The registry is only a finding mechanism. Verification, storage, and
execution all happen locally, described in
[install & local register](install-register.md). A registry can be
deleted and recreated from the published slices; the slices are the
product, the index is a convenience over them.

## Implementation

Reference parsing and transport (HTTPS, git, file — all in-process, no
CLI tools): `crates/tebako-resolve`. Registry management and install:
`crates/tebako-cli`. Trust pinning and verification:
`crates/tebako-signer`.
