/* fopen-probe: the fopen write-mode jail gate (tebako#444) + the
 * audit's adjacent-write surface (creat).
 *
 * Before the fix the fopen interpose passed every non-'r' mode straight
 * to the real fopen with no policy gate: under TEBAKO_JAIL=deny a
 * payload could still fopen("w"/"a"/"r+"/…) its way onto the host, and
 * "r+" — led by 'r' — was misclassified as a read (the materialized
 * copy absorbed the writes silently). creat(2) was never interposed at
 * all (glibc builds it over libc-internal open aliases; libSystem gives
 * it its own stub — both invisible to the open/openat interpose).
 *
 * Commands:
 *   fopen-write <mode> <path> — fopen(path, mode); on success write one
 *       byte + fclose and print "OK:<mode>"; on NULL print
 *       "fopen <mode>: <strerror>" and exit with the errno (EPERM=1,
 *       EROFS=30, ...).
 *   fopen-read <path>         — fopen(path, "r") and cat to stdout (the
 *       read-side control); on NULL print + exit the errno.
 *   open-write <path>         — open(path, O_WRONLY|O_CREAT|O_TRUNC, 0644):
 *       the engine-side audit pin (was already gated; must not regress).
 *   open-rdwr <path>          — open(path, O_RDWR): the "r+"-class pin.
 *   creat-write <path>        — creat(path, 0644): the audit finding.
 * All legs are plain POSIX — they link and run on glibc, musl, and
 * libSystem alike (the musl CI smoke rides them verbatim). */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static int fail(const char *what, const char *path, int e) {
    dprintf(2, "%s %s: %s\n", what, path, strerror(e));
    return e;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        dprintf(2, "usage: fopen-probe <cmd> [mode] <path>\n");
        return 64;
    }
    const char *cmd = argv[1];

    if (strcmp(cmd, "fopen-write") == 0) {
        if (argc != 4)
            return 64;
        const char *mode = argv[2];
        const char *path = argv[3];
        FILE *f = fopen(path, mode);
        if (!f)
            return fail("fopen", mode, errno);
        if (fwrite("Z", 1, 1, f) != 1) {
            int e = errno;
            fclose(f);
            return fail("fwrite", path, e ? e : 5);
        }
        if (fclose(f) != 0)
            return fail("fclose", path, errno);
        dprintf(1, "OK:%s\n", mode);
        return 0;
    }
    if (strcmp(cmd, "fopen-read") == 0) {
        FILE *f = fopen(argv[2], "r");
        if (!f)
            return fail("fopen", argv[2], errno);
        char buf[4096];
        size_t n;
        while ((n = fread(buf, 1, sizeof buf, f)) > 0)
            write(1, buf, n);
        fclose(f);
        return 0;
    }
    if (strcmp(cmd, "open-write") == 0) {
        int fd = open(argv[2], O_WRONLY | O_CREAT | O_TRUNC, 0644);
        if (fd < 0)
            return fail("open-write", argv[2], errno);
        if (write(fd, "Z", 1) != 1)
            return fail("write", argv[2], errno);
        close(fd);
        dprintf(1, "OK:open-write\n");
        return 0;
    }
    if (strcmp(cmd, "open-rdwr") == 0) {
        int fd = open(argv[2], O_RDWR);
        if (fd < 0)
            return fail("open-rdwr", argv[2], errno);
        close(fd);
        dprintf(1, "OK:open-rdwr\n");
        return 0;
    }
    if (strcmp(cmd, "creat-write") == 0) {
        int fd = creat(argv[2], 0644);
        if (fd < 0)
            return fail("creat", argv[2], errno);
        close(fd);
        dprintf(1, "OK:creat\n");
        return 0;
    }
    dprintf(2, "fopen-probe: unknown command %s\n", cmd);
    return 64;
}
