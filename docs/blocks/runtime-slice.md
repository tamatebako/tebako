# The runtime slice

A runtime slice supplies the interpreter a packaged application runs
on. Today that interpreter is Ruby, built with tebako support compiled
in; other languages follow the same model.

The runtime slice exists so that packages do not each carry their own
interpreter. One runtime is downloaded once per machine and shared by
every package that needs it. Packages stay small, and upgrading Ruby
does not require rebuilding any application.

## Two files, one unit

A runtime slice is published as two files with one name:

1. **The runtime executable** — the interpreter itself, patched so it
   can mount slices and run from them.
2. **The runtime image** (`.tfs`) — the interpreter's library tree:
   the standard library, bundled gems, and support files, mounted
   read-only when the interpreter starts.

Both are versioned together and published per platform: macOS (Apple
silicon and Intel), Linux (glibc and musl, x86_64 and arm64), and
Windows. The musl builds are dynamically linked against musl by design
and document a minimum musl version (musl ≥ 1.2.3, e.g. Alpine ≥ 3.17).

## How a package gets its runtime

Each package records a runtime reference: the language and version it
wants, and the tebako generation. On first run the bootstrap looks for
a compatible runtime in the machine cache. If none is present, it
downloads the newest compatible one from the runtime release, verifies
its checksum against the release's published index, and installs it
under a lock so two packages installing at once cannot corrupt the
cache. Later runs start instantly from the cache.

A package built fat skips all of this: its runtime is one of its slots,
verified and used in place.

## Versioning and compatibility

Applications declare a range, not an exact runtime. A pure-Ruby
application typically accepts any recent line (for example
`>= 3.3, < 5.0`); an application with native extensions locks to the
Ruby ABI line it was built against. This means a runtime upgrade almost
never requires an application rebuild — and when an application is run
against an incompatible runtime, the result is a clear compatibility
error, not a crash.

The loader and runtime also carry a small contract version. If a future
runtime changes how it expects to be launched, an old bootstrap refuses
it with a named error instead of mis-launching it; older runtimes
without the field are treated as the first contract generation and run
normally.

## How runtimes are made

Runtimes are built by a dedicated factory repository. For each Ruby
version and platform, patched Ruby sources are compiled, packaged with
their image, and smoke-tested by actually booting them and exercising
the filesystem, gems, and locking behavior. Only after that passes are
they published as a release with a machine-readable index.

## Implementation

The factory is `tebako-runtime-ruby`; the per-version patches live in
`tamatebako/ruby`; the download and cache logic lives in
`crates/tebako-bootstrap`.
