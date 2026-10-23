# Tebako and its neighbours: an honest comparison

This chapter compares tebako with the tools a reader is most likely to have
used for the same job, and it states where each of them is the better choice.
It is written from the implementation, not from a marketing position: every
structural claim below names the mechanism that produces it, and every number
carries the measurement it came from.

The comparison is limited to one question. Which tool should a person use to
ship a program that runs on a machine where the program's language runtime is
not installed? Disk economy, start-up time and rebuild cost follow from the
answer, and they are treated afterwards.

## The three shapes

Every tool in this comparison produces one of three shapes.

**A container image** packages a whole userland: the program, its libraries, and
an operating system's worth of supporting files. Docker and Podman produce this
shape, and OCI registries distribute it. The image is run by a container runtime
that isolates the program from the host.

**A bundled interpreter tree** packages the program together with a private copy
of its language runtime, extracted to disk before the program starts.
PyInstaller, pkg, and the Ruby packagers in the `traveling-ruby` lineage produce
this shape. The program runs directly, without a container runtime, and the
runtime's files exist as ordinary files on the host.

**A mounted image** packages the program and the runtime as filesystem images
that are read *without extraction*. The loader maps the images into the process
and the interpreter reads its standard library through the mounted tree. Tebako
produces this shape, and so does AppImage for the narrower case of a single
executable with its shared libraries.

The three shapes are not interchangeable, and the differences that matter to a
user follow from the shape rather than from the quality of any implementation.

## Where each shape wins

**A container image wins when the program needs the operating system.** A
program that shells out to system tools, links against host libraries, or
expects a service manager is a program whose contract with the host is large.
A container states that contract explicitly and enforces it, and no
single-process packager can offer the same guarantee.

**A bundled interpreter tree wins on familiarity.** The files are ordinary
files; a user can inspect them, patch one, and run the program under a debugger
without learning a new mechanism. The cost is that every installation extracts
the runtime again, and that nothing in the tree is verified after it lands.

**A mounted image wins when the same runtime serves many programs.** The
runtime is stored once, content-addressed, and mounted per process; each
additional program pays for its own bytes and not for a second copy of the
interpreter. Nothing is extracted, so nothing can be half-extracted, and the
integrity check happens once at fetch rather than on every run.

## The structural differences, stated plainly

| property | container image | bundled tree | tebako |
|---|---|---|---|
| what runs the program | a container runtime | the host kernel | the host kernel |
| runtime files on disk | inside the image layer store | extracted per install | never extracted |
| second program on the same runtime | shares layers if the base is shared | extracts a second copy | mounts the same image |
| verification | image digest, often signed | none after install | sha256 anchor at fetch, per-group digests for lazy mounts |
| composition unit | a layer | none | a payload image with a manifest |
| the program's own dependencies | resolved at build time into layers | resolved at build time into the tree | declared in a manifest, resolved by a solver (spec 42) |
| platform coverage | the daemon's platforms | the packager's platforms | per-triplet artifacts, `universal` where the payload allows |

Two rows deserve a sentence each, because they are where the shapes differ for
reasons a user can feel.

*Runtime files on disk* decides whether a read of a standard-library file is a
read of a host file or a read through a filesystem layer. Tebako's answer is
that the interpreter reads through the mounted tree, which is why its start-up
cost is a mount and not an extraction; it is also why a tebako package cannot be
inspected with `cat` without going through `tfs cat`.

*Composition unit* decides what can be reused. A container layer is reused when
the base image matches; tebako's payload image is reused when its digest
matches, at any mount point, and payloads can mount further payloads (spec 23).
The recursive case is proven at five levels by the product's own suites and by a
standing fixture in the reference suite.

## Numbers that were measured here

These are from this repository's own runs, and they are the numbers this
document is allowed to cite. Anything else would be an anecdote.

| quantity | value | where it came from |
|---|---|---|
| re-press loop, unchanged project | 4.6–5.0 s (9.7–10.5 s before the deploy-driver cache) | tebako#864, release build, macOS arm64 |
| cold press including the runtime download | 31.7 s, of which 7.2 s is a 28.4 MB runtime fetch | the stranger walkthrough |
| a five-level nested composition, pressed and run | 5 levels served from their own mounts | `hello-runtimes` fixture, run `39907456088` |
| delta payload for a minor runtime bump | 0 % of bytes saved on today's image framing | tebako#868's measurement; lazy seeding is the optimisation that pays |

The last row is deliberately included as a negative result. A comparison that
only reports favourable numbers is not a comparison.

## What tebako does not do

It does not isolate a program from the host. A tebako package runs as the user
who starts it, with the host filesystem visible according to the jail policy the
package declares and the user permits (spec 08). This is a deliberate
limitation: isolation is the container's job, and a packager that pretended
otherwise would be worse at both jobs.

It does not manage services. A tebako package starts one process, the way a
native program does.

It does not replace a package manager. Tebako distributes programs and their
runtimes; it does not install system libraries, and it does not resolve
dependencies outside the payloads a registry publishes.

## Choosing

A reader with a service to run, or a program that expects an operating system,
should use a container. A reader with a single program to hand to a colleague
who has no Ruby, Python or Java installed should use the packager that keeps the
runtime out of the extraction path, because that is the property that makes the
second program cheap and the integrity check meaningful.

Tebako is that second choice, and the rest of this documentation describes how
it works rather than why it is better.
