// bzip2 1.0.8 declares bz_internal_error (bzlib_private.h, used by the
// AssertH macro on assertion failure) but NEVER DEFINES it — anywhere.
// Dynamic libbz2.so/.dylib tolerates the dangling reference (shared
// libraries allow undefined symbols; the assertion path never executes
// with valid data), so every distro ships a libbz2 with this hole
// (verified: Debian bookworm's libbz2.so.1.0, macOS libbz2.dylib).
// A STATIC link resolves everything and fails:
//   rust-lld: error: undefined symbol: bz_internal_error
//       >>> referenced by decompress.c:614
//       >>> ... in archive .../bzip2-sys-*/out/lib/libbz2.a
// Provide the trivial ABI-compatible definition ourselves rather than
// forking bzip2-sys. Behavior matches upstream bzip2 1.0.6 (report and
// abort — the function is reached only on a failed internal assertion).

#include <stdio.h>
#include <stdlib.h>

void bz_internal_error(int errcode)
{
    fprintf(stderr,
            "bzip2: internal assertion failed (error %d) — the compressed "
            "stream is inconsistent with this library build\n",
            errcode);
    exit(3);
}
