# Tebako building blocks

Tebako packages an application and everything it needs into a single
file that runs directly. This section describes the system's building
blocks, one page each: what it is, what it is for, and how it works.

## Contents

| Block | Function | Page |
|---|---|---|
| package | the single runnable file | [package.md](package.md) |
| bootstrap | the loader embedded in every package | [bootstrap.md](bootstrap.md) |
| runtime slice | a downloadable, shareable interpreter | [runtime-slice.md](runtime-slice.md) |
| executable slice | an application packaged as a mountable image | [executable-slice.md](executable-slice.md) |
| data slice | versioned content mounted alongside an application | [data-slice.md](data-slice.md) |
| toolkit slice | an optional capability downloaded on demand | [toolkit-slice.md](toolkit-slice.md) |
| shim system | how packaged commands reach PATH | [shim-system.md](shim-system.md) |
| install & local register | how slices are stored and tracked on a machine | [install-register.md](install-register.md) |
| remote registry | how authors publish slices and users find them | [remote-registry.md](remote-registry.md) |

## Terminology

Seven terms are used with fixed meanings throughout the documentation.

- **bootstrap** — the loader program. Not a slice.
- **slice** — one filesystem image file (`.tfs`). Runtimes,
  applications, data, and toolkits are all delivered as slices.
- **slot** — a slice's numbered position inside a package.
- **package** — the complete runnable file: bootstrap, slices in slots,
  and a table of contents at the end of the file.
- **stack** — physically assembling slices into a package.
- **resolve / compose** — determining which slices a run requires and
  combining them.
- **mount** — making a slice's contents visible at a directory path
  during execution.

## How the parts fit together

An author builds slices from source code and publishes them. A user
downloads a package. When the package runs, its bootstrap locates or
downloads the required runtime, mounts the slices, and starts the
program. The program then runs against ordinary-looking files; the fact
that those files came from images is invisible to it.

```
 author                          distribution                     user machine
──────────────────────────────────────────────────────────────────────────────────
 source code ──press──► slices ──publish──► registry ──download──► local store
                 │                                   │
                 └──stack──► package ◄────────────────┘
                              │
                              ▼  user runs it
                    bootstrap → runtime → mounted slices → the program runs
```
