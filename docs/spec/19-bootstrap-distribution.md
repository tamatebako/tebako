# Spec 19 — The Bootstrap, Distributed: How Tebako Reaches Your Machine Without a Compiler

**Status:** normative · user- and developer-facing
**Audience:** everyone who runs, builds, or distributes tebako packages
**Related:** spec 00 (invariants), spec 17 (the runtime driver contract),
spec 18 (the contract graph)

---

## 1. What this document is about

A tebako package is a single executable file you can copy to a machine and
run. It is made of three parts:

```
┌────────────────────────────────────────────────────────────┐
│ A · the bootstrap (a tiny Rust loader, < 3 MB)             │
│ B · the runtime (the interpreter, e.g. Ruby)               │
│ C · the payload slice(s) (your app and its data, as images)│
└────────────────────────────────────────────────────────────┘
```

This document answers one question completely: **where does part A come
from, and why will it run on your machine?**

The short answer: **we build it, you never do.** The bootstrap is a
precompiled artifact we publish for every supported platform, alongside
the rest of the tebako release. A press stitches it onto your payloads —
today from a **local** Rust bootstrap (`--bootstrap`, `$TEBAKO_BOOTSTRAP`,
or the `tebako-bootstrap` binary next to the `tebako` executable); the
download-and-cache flow of §4 (PLANNED) makes that automatic. There is no
C compiler, no CMake, no Rust toolchain, no vcpkg anywhere in that path —
and there never will be.

> **Shipped vs planned.** The CLI today sources the bootstrap from local
> binaries only and **fails closed** when none is found (named error,
> exit 136). The retired v1 C++ `tebako-bootstrap` release download —
> whose argv0-verbatim handoff the image-era runtime driver rejects — is
> gone for good: no silent network fetch remains on the press path. The
> per-triplet Rust bootstrap assets this document describes ARE published
> with every tebako release (§3.3); teaching `tebako press` to resolve
> them into the store (once, verified, cached) is the PLANNED part.

## 2. The audience rule (who needs what)

| Who you are | What you need on your machine |
|---|---|
| **You run tebako packages** | Nothing. Download, run, done. No compiler, no libraries. |
| **You build data-only payloads** (docs, fonts, datasets, archives) | Nothing. `tfs mkimage` works in-process; no toolchain. |
| **You build runnable payloads** (your app, a gem with native extensions) | A compiler **for your own code only**. The entire tebako stack — bootstrap, runtime, imager — arrives as binaries we built. |
| **You work on tebako itself** | The full toolchain (the factories, the Rust workspace, vcpkg). This is the only audience that ever compiles tebako. |

Prebuilt artifacts flow downward; compilation never flows outward. If
your use of tebako ever asks you to compile part of tebako, that is a
bug in our distribution, not a requirement.

## 3. Why "just build it anywhere" does not work

### 3.1 The glibc trap (Linux)

You might assume a Linux binary built on a recent distro runs everywhere
Linux runs. It does not:

```
$ ./tebako-bootstrap            # built on Ubuntu 22.04 (glibc 2.35)
./tebako-bootstrap: /lib/x86_64-linux-gnu/libc.so.6:
    version `GLIBC_2.34' not found (required by ./tebako-bootstrap)
```

**Why:** glibc versions every public symbol. A binary built against
glibc 2.35 records *symbol versions* like `GLIBC_2.34` in its dynamic
section. The dynamic loader on Ubuntu 20.04 (glibc 2.31) looks for those
versions, doesn't find them, and refuses to start the program.

glibc's compatibility promise is **one-directional**: binaries built on
*old* glibc run on *newer* glibc, never the reverse. So the build host's
glibc is the floor below which the binary cannot go.

### 3.2 What each platform's "floor" means

| Platform | The floor question | Our answer |
|---|---|---|
| **linux-musl** (x86_64, arm64) | None. musl is fully static — the binary carries everything and runs on any Linux kernel ≥ the baseline. | **The universal Linux artifact.** When in doubt, this is the one to ship and to use. |
| **linux-gnu** (x86_64, arm64) | Oldest glibc we promise to support. | The floor decision is **Ubuntu 20.04 (glibc 2.31)**: built there, the binary runs on 20.04 *and everything newer*. We never build the shipping artifact on a newer glibc. (Honesty note: the linux-gnu release legs are being **moved** to the 20.04 container to make this true; the musl legs already carry the universal promise.) |
| **macOS** (arm64, x86_64) | `MACOSX_DEPLOYMENT_TARGET` — the oldest macOS the binary may load on. | Pinned per arch (currently 11.0); the linker stamps it and the OS enforces it. |
| **Windows** (x86_64, ucrt64) | The Universal CRT is part of Windows 10+. | ucrt64 with the C/C++ runtimes statically linked (no mingw DLLs to find); floor is Windows 10 / Server 2016. |

### 3.3 Yes, that means the bootstrap comes in triplets

There is no single "Linux binary." Every tebako binary artifact — the
bootstrap first among them — is published **per platform triplet**:

```
linux-gnu-x86_64    linux-gnu-arm64
linux-musl-x86_64   linux-musl-arm64
macos-arm64         macos-x86_64
windows-x86_64
```

The matrix is deliberate and small: each triplet is a real CI leg with
real tests, not a combinatorial explosion. The loader picks exactly one
for your machine; you never pick anything.

## 4. How a press gets its bootstrap (no compiler involved)

**Shipped behavior:** `tebako press` sources the bootstrap from local
Rust binaries, in this order:

1. `--bootstrap <path>` — an explicit binary;
2. `$TEBAKO_BOOTSTRAP` — the environment override;
3. the `tebako-bootstrap` executable next to the `tebako` binary (the
   dogfooding/installed-pair layout).

With none of these present the press **fails closed** with the named
error "Press requires a local Rust tebako-bootstrap binary" (exit 136) —
no network fetch is attempted. (The historical fallback downloaded the
v1 C++ bootstrap release; its handoff predates the spec 17 driver
contract and produced packages that fail at run time, so it is retired,
not merely refused.)

**PLANNED (not yet shipped):** the release-store flow below — resolving
the published per-triplet Rust bootstrap into `~/.tebako/bootstraps/`
automatically, with the same verification discipline the runtime cache
already has:

```
tebako press app.rb -o myapp
        │
        ▼
  1. resolve the bootstrap for the target triplet
        ├── store hit: ~/.tebako/bootstraps/<version>-<triplet> → use it
        └── miss: download from the tebako release page
                 ├── sha256-verified against the release's SHA256SUMS
                 └── tmp + rename into the store (a partial download
                     is invisible — never used)
        │
        ▼
  2. read the bootstrap's embedded contract card
        (artifact-info.yaml: era, version, launcher_abi, contract_version)
        ├── era mismatch → named refusal BEFORE stitching
        └── ok → proceed
        │
        ▼
  3. image your payload(s) with tfs (in-process — no compiler,
        no external tools of any kind)
        │
        ▼
  4. stitch: bootstrap + payload slots + trailer → `myapp`
```

Every artifact in this flow is one we built and sha256-published. The
only bytes of yours are the payloads.

**A custom bootstrap** (`TEBAKO_BOOTSTRAP=/path/to/binary`) follows the
same contract card check — a press will refuse to stitch an era-1 or
mismatched bootstrap rather than produce a package that fails later on a
user's machine (spec 18, S38).

## 5. Why we don't ask users to compile the bootstrap

Four reasons, in descending order of how much they matter to you:

1. **It would put a compiler on every user's machine** — the exact
   complexity tebako exists to remove. "Install Xcode/Build Tools/a
   cross-gcc" is not an acceptable prerequisite for packaging a script.
2. **Determinism.** A bootstrap built on your machine is untraceable:
   your toolchain version, your flags, your libc. Ours is built in one
   pipeline, on a pinned floor, with the size gate enforced (currently
   2.50 MiB against the 3 MiB budget on every triplet, checked per
   release).
3. **Trust.** Every published bootstrap is sha256-anchored in the
   release and verified at download. There is one artifact to audit, not
   a million local variations.
4. **The size budget needs the pipeline.** The 3 MB gate is a
   release-time discipline (opt-level z, fat LTO, one codegen unit,
   panic=abort, stripped symbols). It is not something we can ask of a
   casual local build.

## 6. Keeping a package's bootstrap current

Packages are **immutable**: once stitched, a package's bytes don't
change. When a new bootstrap ships, old packages keep their old
bootstrap — that is by design (a package is a reproducible artifact, not
a living install).

If you want the new bootstrap on an existing package, there are two
honest paths:

- **Re-press** from the payload (the recommended path — the payload
  images are the durable part; stitching is cheap).
- **`tebako-pkg set-runtime`**, which swaps only the bootstrap region of
  the package in place. The swap **verifies the contract card first**: a
  new bootstrap that would refuse the package (era mismatch) is rejected
  at swap time, not at the user's run time.

Either way, the decision is explicit, and `tebako inspect <package>`
always tells you which bootstrap era and version a package carries.

## 7. The contract card inside every bootstrap

Every bootstrap binary we ship carries an embedded
`artifact-info.yaml` block (appended at build, marked and readable):

```yaml
schema: artifact_info
schema_version: 1
era: 2
name: tebako-bootstrap
version: 2.0.0
triplet: linux-musl-x86_64
launcher_abi: 1
contract_version: 2
sha256: <of the enclosing release entry>
```

This is what `tebako inspect` reads to answer "which tebako can open
this?" for the bootstrap itself, and what a press verifies before
stitching. (spec 18, S38 — the same declaration model as every other
artifact in the ecosystem.)

## 8. FAQ

**"I built tebako on Ubuntu 22.04 and it won't start on 20.04. Broken?"**
No — that's glibc working as designed (§3.1). Use our prebuilt artifacts:
they're built on the floor distro or musl, and they start everywhere we
promise.

**"Can I just use the musl bootstrap everywhere on Linux?"**
Yes. It's the universal Linux artifact. The linux-gnu variants exist for
environments that specifically want glibc-linked binaries (some
monitoring/injection tooling expects glibc behaviors), not because
musl is second-class.

**"Do I need Rust or Cargo installed to use tebako?"**
No. Only people hacking on tebako itself do.

**"I'm building a gem with a C extension. What do I compile?"**
Your extension, and only your extension. tebako's runtime (which your
extension links against) is a prebuilt artifact; you never rebuild it.

**"What happens if the download is interrupted mid-press?"**
Nothing bad. Downloads land in a tmp area and are renamed into the store
only when complete and verified; a partial file is invisible and is
simply re-fetched next time.

**"My company's policy requires building everything from source."**
You can — every repo is public and the factories are reproducible — but
it's a supported-you're-on-your-own path, not our recommendation. If you
rebuild, keep the era/contract fields intact so the artifacts still
verify against each other (spec 18).

---

## 9. Summary in one paragraph

The bootstrap is **ours, precompiled, per-triplet, verified, and
cached** — built once in our pipeline on deliberately old floors (or
musl, which has no floor), published with sha256 anchors, sourced by the
press from local binaries today (failing closed otherwise; the release
store download of §4 is PLANNED), and stitched onto your payloads by a
press that checks its contract card first. You never compile it, you
never pick it, and it always starts.
