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
