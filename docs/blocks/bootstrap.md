# The bootstrap

The bootstrap is the loader embedded at the start of every tebako
package. When a user runs a package, the operating system starts the
bootstrap; everything after that is its job.

It is a small, static Rust program with no dependencies of its own,
kept under 3 MB so that packages stay small. That budget is enforced in
CI.

## What it does on every run

The bootstrap performs the same fixed sequence:

1. **Reads the trailer** at the end of its own file. If it is missing
   or corrupt, it stops with a named error — it never guesses.
2. **Checks the launcher ABI** the package was pressed for. A package
   from an incompatible generation is refused with a named error.
3. **Applies the trust policy.** If the package is signed, the
   signature is verified (when verification support is compiled in).
   Otherwise the package runs with a clear warning and a line in the
   audit journal. If the user demands signed packages only
   (`TEBAKO_REQUIRE_SIGNED=1`), an unsigned package is refused outright
   rather than quietly accepted.
4. **Resolves the runtime.** If the required runtime is already in the
   machine cache, it is used directly. Otherwise the bootstrap downloads
   it with its own built-in HTTP and TLS — never curl, git, or any
   system tool — verifies the checksum against the release index, and
   installs it atomically under a lock. Before accepting any checksum,
   it checks the runtime's declared contract version: a runtime from an
   incompatible generation is refused with a named error, so an old
   bootstrap never silently mis-runs a new runtime.
5. **Stages the runtime image** — the interpreter's library files —
   into the shared cache as read-only files with their verification
   markers.
6. **Hands off to the runtime.** The payload slices are mounted and the
   entrypoint is started. On Linux and macOS the bootstrap process is
   replaced outright (`execve`), so signals and exit codes belong to
   the runtime. On Windows there is no `execve`, so the bootstrap spawns
   the runtime as a child, waits for it, and exits with the child's
   exit code.

## Error reporting

Every failure is a named exit code with a specific message: bad
trailer, ABI mismatch, runtime unavailable, checksum mismatch, bad
signature, untrusted signer, jail policy error, I/O failure, contract
mismatch, install refused. Scripts can test for each case; users get an
explanation, not a stack trace.

## Progress output

Downloads report progress on an interactive terminal — a single
updating line with transfer rate — and stay quiet in CI logs, where one
line marks the start and one the end. A cache hit prints one line:
`runtime ruby-3.4.2 (cached)`.

## Platform coverage

The same behavior is required on macOS, Linux (glibc and musl), and
Windows. The Windows implementation uses native file locking, atomic
rename-with-retry (for the sharing violations antivirus scanners cause),
and the spawn-and-wait handoff described above. All of it is exercised
by a dedicated Windows CI job.

## Implementation

`crates/tebako-bootstrap`, built on `crates/tebako-http` (downloads),
`crates/tebako-term` (progress output), and `crates/tpkg` (the trailer
format). The only unsafe code is the platform FFI, confined to one
module.
