# The freestanding boundary

Status: the boundary as the v2.8 line ships it. The self-hosted CLI — a
tamatebako package that carries the packager — is planned and its gap is
stated at the end, not assumed closed.

## The question this document answers

A tebako package promises that it runs on a host that has nothing prepared
for it. That promise is only meaningful if the set of things the host must
provide is written down, and if the set of things the product must never
reach for is enforced rather than intended. This document states both, and
names where the promise stops.

## Four audiences, four different machines

The audience rule decides what may be compiled or fetched where, and every
statement below is a consequence of it.

1. A person **running** a package needs no compilation machinery and no
   libraries beyond the platform's own. The package arrives as one file and
   starts.
2. A person building **data-only payloads** needs no compilation machinery
   either. Pressing data is an imaging operation, not a build.
3. A person building **runnable payloads** compiles only their own code. The
   interpreter, the loader and the filesystem arrive as published binaries.
4. Only a person working **on tebako itself** needs the compilers, the
   factories and the containers that produce those binaries.

Anything that would make audience 1, 2 or 3 fetch or build tebako-side
machinery is wrong by construction. Prebuilt artifacts flow downward; the
requirement to compile never flows outward.

## What a package carries

A composed package is three independently published parts, glued by the
trailer:

- the **bootstrap**, a Rust loader under the standing 3 MiB gate, which reads
  its own trailer, resolves the runtime, verifies it and hands off;
- the **runtime**, two artifacts: the interpreter executable and the
  environment image that the interpreter mounts as its own root;
- the **payload slices**, bare images that the driver mounts at their
  declared points.

The package therefore carries its interpreter and its filesystem. It does
not carry, and does not need, a compiler, a shell, a package manager, a
network client, or a copy of this toolchain.

## What the host must provide

The host provides the kernel, the platform's C library (glibc or musl on
Linux, the system libraries and ucrt on Windows, libSystem on macOS), the
dynamic loader, and a directory the program may write. It also provides the
network at install time, or a mirror when the network is absent, because
resolution is a fetch. Nothing else is required, and each of those is listed
because a package that needed more would not be freestanding.

## What the product must never require of the host

The rule is that no shipped artifact reaches for a host *tool*. The
distinction matters, because the product does spawn processes: the bootstrap
spawns the interpreter it just resolved, the launcher does the same, and the
shim dispatches to a runtime. Those are the handoff, which is the product's
whole purpose, and the spawned program is one the product published.

What the product does not do is invoke a program it did not publish. A search
of every crate's source for an invocation of `sh`, `curl`, `git` or
`mkdwarfs` finds none: HTTP is in-process (`ureq` with `rustls` and bundled
roots), imaging is in-process (the limnifs and dwarfs-t writers on their Rust
bindings), and cryptography is in-process (the vendored `rnp-rs`). A package
therefore does not care whether the host has a shell, whether `git` is
installed, or whether the platform's image tools exist.

## The gates that keep it true

Three mechanisms keep the boundary from eroding, and each is a build failure
rather than a review habit:

- the **size gate** fails any bootstrap at or above 3,145,728 bytes;
- the **export surface** is checked against a committed list, so a crate
  cannot start leaking a symbol that changes the ABI the factories link;
- `tebako doctor` answers, on a real machine, whether the store is coherent
  — locks, sidecars, orphaned artifacts — which is the same question a user
  asks when a package behaves as though the host were missing something.

## Where the boundary ends

Three operations need machinery, and they are above the line on purpose.

**Pressing** needs the CLI, because it resolves, verifies, images and signs.
That is audience 3's own machine, and the CLI arrives as a published binary
rather than a build.

**Publishing** needs a factory: the recipes, the matrices and the signing
accounts belong to the repositories that own them, never to the package.

**Signing** needs the signer, in-process, with a key the publisher holds.
Verification needs nothing beyond the sidecar digest that travels with the
artifact.

## Self-hosting: what exists, and the gap

The CLI is distributed as a downloaded binary per platform, verified by
digest. Self-hosting means going one step further: a tebako package that
*is* the CLI, installable from the public registry like any other payload.

The gap is precise, and it was measured rather than assumed. Pressing a
project whose entrypoint is a native binary — a four-line C program, on the
released 2.8.60 CLI — is *accepted*: the press embeds a ruby runtime nobody
asked for, writes a 20,816,080-byte package, and reports success. The first
execution then dies:

```
runtime ruby-4.0.6 (cached)
tebako-driver: failed to mount image '/private/tmp/nativetool/pkg'
              at '/__tfs__': Invalid argument
Tebako intialization failed.
```

The press and the boot disagree about what the artifact is: every payload
entry the driver runs today is a program the **interpreter** runs, so the
press treats the entry as a script under a provided runtime while the boot
tries to mount a runtime image that does not exist. A CLI payload has no
interpreter at all, which makes the disagreement fatal rather than unlucky.

What self-hosting therefore needs is a payload **kind**, not a flag on the
existing one: the manifest declares an entrypoint the bootstrap materializes
and executes directly, with the same digest verification, mount discipline
and jail the interpreter path already has, and the trailer carries no runtime
slot. Until that kind exists the honest behaviour is a refusal at press time
with a name that says which shape was expected — filed as tebako#925, whose
report carries this transcript.

"tebako-cli stitched" is therefore a plan with a name, and this document
says so rather than implying the box is ticked.

The same kind is what the factories would need to press their own tooling on
tebako runtimes, which is the arc's second item: the factories' work is
build orchestration, and orchestration is a program, not a script under an
interpreter.

## What this document is not

It is not a claim that the boundary has been tested against every hostile
host; that is the hermeticity arc's conformance suite, which attacks the
jail and the fetch paths per release. It is the statement of what the
boundary is, so that the suite has something exact to attack.
