# Install and the local register

This page covers what lands on a user's machine when things are
installed, where it goes, and how the machine keeps track of it.

## One home per user

Everything tebako stores lives under one directory:
`~/.tebako` (`%LOCALAPPDATA%\tebako` on Windows, overridable with
`TEBAKO_HOME`). It is split into two kinds of content:

- **Things that matter** — configuration, the user's trust decisions
  (which signers to accept), signing keys, and the audit journal.
  Deleting these loses state that cannot be re-created.
- **Things that can be re-derived** — downloaded runtimes, payload
  slices, registry indexes, and the local register file itself. All of
  it can be rebuilt by downloading again; none of it is precious.

## The store

Downloaded artifacts are kept in a content store, organized by what
they are:

```
store/
  runtimes/<language>/<version>/<tebako-abi>/<platform>/
  toolkits/<name>/<version>/<platform>/
  payloads/<name>/<version>/<platform or universal>/
  data/<name>/<version>/
```

Versions of the same thing sit side by side; nothing overwrites
anything. Each artifact keeps two small marker files next to it: its
verified checksum (proof it was checked when it was installed) and its
origin (where it came from). Installations are atomic — a lock, a
temporary directory, then a rename — so a crash mid-install never
leaves a half-written artifact visible.

## The register

The machine keeps a single index file — the local register — listing
everything in the store: what it is, its version, its checksum, where
it came from, and what depends on it.

The register is a cache, not a source of truth. The truth is the store
itself: if the register is missing, stale, or doubted, it is rebuilt by
walking the store (`tebako cache reindex`). Tools read the register for
speed and fall back to the store when in doubt. Because every entry
carries its dependencies, the machine can also answer "what is still
needed" — cleanup removes only artifacts nothing uses.

## A shared store for the whole machine

A second, machine-wide store can sit alongside the user's (for example
`/usr/local/var/tebako`, or `%ProgramData%\tebako` on Windows), shared
by every account. Lookups check the user's store first, then the shared
one; installs go to the user's store by default and to the shared one
only explicitly. Trust decisions always stay personal — one user's
accepted signers are never inherited by another.

## Installing is a verb

Nothing lands in the store as a side effect. Running a package fills
the runtime cache (the runtime is needed to execute at all) but never
touches the payload store. Slices are installed by explicit commands:

- `tebako install <ref | name>` — fetch from a registry, verify, store,
  and register the application's commands.
- `tebako install <file>` — unpack a local package's slices into the
  store, preserving their checksums and origin. Links to PATH happen
  only with an explicit `--shims`.
- `tebako install ./myapp --shims` — both at once, explicitly.

A package pressed with the no-install flag refuses all of these; it
runs standalone and nothing more. Local files being developed are used
in place and never copied into the store at all.

## Implementation

`crates/tebako-resolve` (the store and its locking), `crates/tebako-shim`
(records and links), `crates/tebako-cli` (the install commands), and
`crates/tebako-bootstrap` (the runtime cache during runs).
