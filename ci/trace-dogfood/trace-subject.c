/* trace-subject: the spec 25 §7 dogfood subject for the linux coverage
 * legs. BUILT BY CI, NEVER SHIPPED (§7's raw-syscall fixture rule). It
 * extends the libtfs-preload e2e print-data.c shape with the two knobs
 * the coverage legs need:
 *
 *   --wait DIR   write DIR/ready (the pid), then poll for DIR/go —
 *                the launch-then-attach handshake the ptrace (kernel
 *                layer) leg needs: retrace attach(1) needs a live pid
 *                BEFORE the traced work happens.
 *   --raw PATH   one raw syscall(2) touch of PATH (SYS_openat, no libc
 *                wrapper) — the sub-libc probe of spec 25 §6.1: invisible
 *                to libc-boundary hooks by construction, visible to a
 *                kernel-layer tracer. The touch's own result (ENOENT —
 *                the prefix does not exist on the host) is REPORTED on
 *                stdout, never fatal: this binary observes, the CI leg
 *                asserts.
 *   FILE ...     open + read + write-to-stdout, print-data's
 *                libc-routed shape minus the stat(2) prelude: retrace's
 *                linux hook set carries no stat wrapper (the event would
 *                never reach the outside capture), and the stat→open
 *                pair trips CodeQL's TOCTOU rule. Served by the VFS when
 *                under the mount, a host passthrough otherwise.
 *
 * Exit 0 when the work completed; 64 on usage; 70 if the go-file never
 * arrived (a stuck attach must fail the leg loudly, not hang it).
 */
#include <sys/stat.h>
#include <sys/syscall.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>

static int print_file(const char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) {
        dprintf(2, "open: %s\n", strerror(errno));
        return errno;
    }
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof buf)) > 0) {
        off_t off = 0;
        while (off < n) {
            ssize_t w = write(1, buf + off, (size_t)(n - off));
            if (w <= 0)
                return 74;
            off += w;
        }
    }
    if (n < 0) {
        dprintf(2, "read: %s\n", strerror(errno));
        return errno;
    }
    close(fd);
    return 0;
}

int main(int argc, char **argv) {
    const char *wait_dir = NULL;
    const char *raw = NULL;
    int i = 1;
    for (; i < argc; i++) {
        if (strcmp(argv[i], "--wait") == 0 && i + 1 < argc) {
            wait_dir = argv[++i];
        } else if (strcmp(argv[i], "--raw") == 0 && i + 1 < argc) {
            raw = argv[++i];
        } else {
            break;
        }
    }
    if (i >= argc) {
        dprintf(2, "usage: trace-subject [--wait DIR] [--raw PATH] FILE...\n");
        return 64;
    }

    if (wait_dir) {
        char ready[4096], go[4096];
        snprintf(ready, sizeof ready, "%s/ready", wait_dir);
        snprintf(go, sizeof go, "%s/go", wait_dir);
        FILE *f = fopen(ready, "w");
        if (!f) {
            dprintf(2, "ready: %s\n", strerror(errno));
            return 74;
        }
        fprintf(f, "%d\n", getpid());
        fclose(f);
        struct stat st;
        int spins = 0;
        while (stat(go, &st) != 0) {
            if (++spins > 600) { /* ~30 s at 50 ms */
                dprintf(2, "go-file never arrived — the attach is stuck\n");
                return 70;
            }
            usleep(50 * 1000);
        }
    }

    if (raw) {
        /* The sub-libc touch: the syscall directly, no libc wrapper —
         * libc-boundary observation (retrace preload) never sees it. */
        long fd = syscall(SYS_openat, AT_FDCWD, raw, O_RDONLY, 0);
        if (fd >= 0) {
            dprintf(1, "raw:%s:ok\n", raw);
            close((int)fd);
        } else {
            dprintf(1, "raw:%s:%d\n", raw, errno);
        }
    }

    int rc = 0;
    for (; i < argc; i++) {
        int r = print_file(argv[i]);
        if (r != 0 && rc == 0)
            rc = r;
    }
    return rc;
}
