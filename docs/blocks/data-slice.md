# The data slice

A data slice packages content rather than code: fonts, schemas,
templates, dictionaries, model weights, datasets. It is the same
physical form as every other slice — an image plus a manifest — but it
offers no commands. Applications mount it; users never run it.

## Purpose

Applications often need large bodies of content that change on their
own schedule, not the application's. Shipping that content inside every
application binary duplicates it across every consumer and every
version. A data slice makes the content a separate, versioned artifact:

- one copy on disk, shared by every application that uses it;
- its own version line, upgraded independently of any application;
- verified with the same checksums and signatures as code.

For example, a document generator can declare a dependency on a font
collection at a specific version. The fonts download once, every
application on the machine shares them, and a font update does not
touch the applications.

## How applications use it

An application's manifest names its data dependencies along with a
version range and a mount path — effectively "mount slice X at
directory Y". At run time, the resolver finds (or downloads) a
compatible version, mounts it read-only at that path next to the
application, and the application reads the content as ordinary files.
Inside a jail, the data slice is subject to the same read-only rules as
the application image.

Most data slices are platform-free: content does not depend on the
CPU or operating system, so one image serves every machine.

## Lifecycle

1. **Press** — the content tree is packaged with a manifest giving its
   name, version, and content hash.
2. **Publish** — to a registry, next to the applications that consume
   it or on its own.
3. **Resolve** — an application's data dependency is satisfied from the
   local store if a compatible version exists, otherwise by download.
4. **Mount** — read-only, at the path the application declared.
5. **Prune** — the local store tracks which applications require each
   data slice, so cleanup removes only content nothing uses anymore.

## Implementation

The manifest and its content hashing: `crates/tpkg`. Resolution and
mount composition: `crates/tebako-resolve` and `crates/tebako-shim`.
Storage and garbage collection: the local store described in
[install & local register](install-register.md).
