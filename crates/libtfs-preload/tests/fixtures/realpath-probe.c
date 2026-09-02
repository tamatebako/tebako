/* realpath-probe: print realpath(argv[1]) on stdout, or "ERR <errno>"
 * (rc 1) when it fails. argv[2] == "null" selects the NULL-buffer arm
 * (malloc'd result — glibc's canonicalize_file_name semantics; the JDK's
 * canonicalize_md.c uses the caller-buffer arm).
 *
 * The 2026-09-03 dogfood linux-gnu regression pin: glibc realpath(3)
 * walks the path with libc-INTERNAL stat/readlink aliases that PLT
 * interposition never sees, so before the shim interposed the realpath
 * family, a path under a VFS mount fell through to the host resolver —
 * which canonicalized host symlinks in the mount spelling (usrmerge
 * /lib -> usr/lib) and then ENOENT'd on the in-image file. The JDK's
 * URLClassPath built the classpath URL from that leaked spelling and
 * dropped the jar: ClassNotFoundException on a byte-perfect jar. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <limits.h>
#include <unistd.h>

#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

int main(int argc, char **argv) {
    if (argc < 2 || argc > 3) {
        dprintf(2, "usage: realpath-probe <path> [null]\n");
        return 64;
    }
    char *buf = NULL;
    char *resolved;
    errno = 0;
    if (argc == 3 && strcmp(argv[2], "null") == 0) {
        resolved = realpath(argv[1], NULL);
    } else {
        buf = malloc(PATH_MAX);
        if (!buf) {
            dprintf(2, "malloc: %s\n", strerror(errno));
            return 70;
        }
        resolved = realpath(argv[1], buf);
    }
    if (!resolved) {
        int e = errno;
        printf("ERR %d\n", e);
        free(buf);
        return 1;
    }
    printf("%s\n", resolved);
    if (resolved != buf) free(resolved);
    free(buf);
    return 0;
}
