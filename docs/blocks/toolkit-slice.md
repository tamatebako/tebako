# The toolkit slice

A toolkit slice packages an optional capability as a downloadable
image: cryptography is the first example, UI toolkits are the expected
next. It is neither an interpreter nor an end-user application — it is
machinery other parts of the system call on when they need it.

## Why it exists

Some capabilities are too large or too specialized to build into every
binary. Cryptography is the case in point: a full OpenPGP library with
post-quantum algorithms adds megabytes and a long dependency chain. The
tebako bootstrap stays under 3 MB precisely by *not* containing it.

The toolkit slice moves that cost out of the core. The bootstrap ships
without cryptographic verification built in and says so plainly when a
signature cannot be checked. When verification is actually required,
the crypto toolkit is downloaded once, verified itself by the
bootstrap's small built-in check, and used from then on. The core stays
small; the capability is available exactly when needed; and when it is
absent, the error says so by name instead of pretending otherwise.

## The first toolkit: tebako-crypto

The crypto toolkit is built in exactly one place — its own feedstock
repository — from source, with the complete algorithm suite including
the post-quantum families, and published as a signed slice per
platform. Nothing else in the ecosystem compiles that library; everyone
else consumes the slice. One build to audit, one place to fix.

## How capabilities are declared

A toolkit slice's manifest names what it provides — a capability
identifier and its ABI version — and the platforms it covers. Consumers
declare the capability, not a file path: a package or a policy says it
requires cryptography, and the resolver finds a suitable toolkit slice,
fetches it if missing, and mounts it. If none can be found, the failure
names the missing capability.

## Lifecycle

1. **Build** — the feedstock compiles and tests the toolkit per
   platform, then releases it.
2. **Declare** — packages and policies reference the capability by
   name.
3. **Resolve** — the local store is consulted first; a miss downloads
   and verifies the slice.
4. **Mount** — the toolkit is available to the runtime for the duration
   of the run.
5. **Upgrade** — a new toolkit version ships independently of every
   consumer, with its own version and ABI markers.

## Implementation

The crypto toolkit lives in `tebako-packages/tebako-crypto`. Capability
resolution follows the same store and registry machinery as every other
slice; see [install & local register](install-register.md) and
[remote registry](remote-registry.md).
