# The package

A tebako package is the system's deliverable: one file that runs on a
matching machine with no installer, no extraction step, and no external
dependencies.

## Structure

A package has three parts, in order:

1. **The bootstrap** — the loader executable. Every package starts with
   it, so every package is directly runnable. See
   [bootstrap](bootstrap.md).
2. **The slots** — numbered slices: the application, its data, and
   optionally a runtime. A package with no runtime slot is called
   *lean*; one that carries its runtime is called *fat*.
3. **The trailer** — a small block at the end of the file containing
   the table of contents: how many slots there are, where each one
   starts and ends, which runtime the package wants, and a checksum.

Because the table of contents sits at the end, a package can be
inspected or reassembled without parsing the payload bytes. It also
means appending data to a package does not disturb anything an
operating-system code signature has already signed.

## Flags

Three independent properties are recorded in the trailer:

- **lean** — the runtime resolves at run time (downloaded once per
  machine, then shared by all packages).
- **signed** — the package carries a publisher's signature; loaders
  verify it before trusting the contents.
- **no-install** — the publisher froze the package. It runs normally,
  but `tebako install` refuses to unpack it into the local store. This
  is for vendors who want a strictly run-only artifact.

## Operations

- **Press** (`tebako press`) — build slices from source and stack them
  into a package. One press can bundle several commands into a single
  package.
- **Surgery** (`tebako-pkg`) — modify an existing package: insert or
  remove slices, replace the runtime, reassemble from parts.
- **Inspect** (`tebako-pkg info`, `tfs info`) — show the trailer, the
  slots, the trust state, and the manifests, as text or JSON.
- **Run** — execute it. The [bootstrap](bootstrap.md) does the rest.
- **Install** (`tebako install <file>`) — unpack the slices into the
  local store so they can be upgraded or recombined later. Always an
  explicit command, never a side effect of running; see
  [install & local register](install-register.md).

## Distribution forms

A package is distributed two ways:

- **Standalone** — the complete file, for users who have nothing else
  installed. Built per platform.
- **Registry payload** — a bare slice published to a registry, for
  users who run the dispatcher. Pure-language applications ship one
  universal file; applications with native extensions ship one file per
  platform.

## Implementation

The format lives in `crates/tpkg`; surgery in `crates/tebako-pkg`;
pressing in `crates/tebako-cli`; inspection in `crates/tebako-info`;
execution in `crates/tebako-bootstrap`.
