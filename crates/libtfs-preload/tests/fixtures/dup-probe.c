/* tebako#534: libxml2's xmlInputFromFd unconditionally dup()s its fd —
 * on a shim-managed (flag-bit) memfs fd the real dup answered EBADF and
 * every VFS-fed XML parse died (the xml2rfc feedstock's lxml shim). The
 * dup class is now the engine's: dup/dup2/fcntl(F_DUPFD/F_DUPFD_CLOEXEC)
 * clone the open-file description — the offset is SHARED, exactly like
 * POSIX — and the clone is itself a flagged fd the shim owns. A memfs
 * source with a HOST-numbered dup2 target is ENOTSUP (the fd routing
 * keys on the flag bit, so a host number can never name a memfs
 * description — the named error, never EBADF-by-default). The file
 * content is "VFS-SECRET-E2E\n" (15 bytes). Cross-platform: the dup
 * family is plain POSIX. */
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>

static int expect_read(int fd, const char *want, int rc, const char *what) {
    char buf[8];
    ssize_t n = read(fd, buf, strlen(want));
    if (n != (ssize_t)strlen(want) || memcmp(buf, want, strlen(want)) != 0) {
        fprintf(stderr, "%s: got %.8s (n=%zd), want %s\n", what, buf, n, want);
        return rc;
    }
    return 0;
}

int main(int argc, char **argv) {
    int fd, c1, c2, c3, fd2, hfd, r;
    if (argc < 2) return 64;
    /* O_CLOEXEC at open: the dup clones must carry the bit OFF. */
    fd = open(argv[1], O_RDONLY | O_CLOEXEC);
    if (fd < 0) { perror("open"); return 65; }

    /* THE #534 PIN: dup of a memfs fd must NOT die EBADF. */
    c1 = dup(fd);
    if (c1 < 0) { perror("dup"); return 66; }
    /* The clone shares the open-file description: read "VFS-" through
     * the original, the clone continues at "SECR". */
    if ((r = expect_read(fd, "VFS-", 67, "read orig")) != 0) return r;
    if ((r = expect_read(c1, "SECR", 68, "read clone (shared offset)")) != 0) return r;
    /* dup cleared FD_CLOEXEC on the clone (POSIX), even though the
     * original was opened O_CLOEXEC. */
    if (fcntl(c1, F_GETFD) != 0) { fprintf(stderr, "dup clone kept FD_CLOEXEC\n"); return 69; }
    /* An lseek through the clone moves the original's position. */
    if (lseek(c1, 0, SEEK_SET) != 0) { perror("lseek clone"); return 70; }
    if ((r = expect_read(fd, "VFS-", 71, "read orig after clone lseek")) != 0) return r;

    /* fcntl(F_DUPFD, min): another shared clone; the kernel returns the
     * NEW fd from this command. */
    c2 = fcntl(fd, F_DUPFD, 0);
    if (c2 < 0) { perror("fcntl(F_DUPFD)"); return 72; }
    if ((r = expect_read(c2, "SECR", 73, "read F_DUPFD clone")) != 0) return r;
    if (fcntl(c2, F_GETFD) != 0) { fprintf(stderr, "F_DUPFD clone kept FD_CLOEXEC\n"); return 74; }

    /* F_DUPFD_CLOEXEC: the clone with the bit SET. */
    c3 = fcntl(fd, F_DUPFD_CLOEXEC, 0);
    if (c3 < 0) { perror("fcntl(F_DUPFD_CLOEXEC)"); return 75; }
    if ((fcntl(c3, F_GETFD) & FD_CLOEXEC) == 0) {
        fprintf(stderr, "F_DUPFD_CLOEXEC clone lost FD_CLOEXEC\n");
        return 76;
    }

    /* THE REGRESSION PIN: dup2 of a memfs fd onto a SECOND live memfs
     * fd (an independent open of the same file) — the target's own
     * description dies and the number rebinds to the source's: reading
     * the target continues the SOURCE's position. */
    fd2 = open(argv[1], O_RDONLY);
    if (fd2 < 0) { perror("open 2"); return 77; }
    if (lseek(fd, 0, SEEK_SET) != 0) { perror("lseek"); return 78; }
    if ((r = expect_read(fd, "VFS-", 79, "read source pre-dup2")) != 0) return r;
    if (dup2(fd, fd2) != fd2) { perror("dup2 onto memfs target"); return 80; }
    if ((r = expect_read(fd2, "SECR", 81, "dup2 target reads source's pos")) != 0) return r;
    if (close(fd2) != 0) { perror("close rebound target"); return 82; }
    /* …and the source reads on (the alias close kept the description). */
    if ((r = expect_read(fd, "ET-E", 83, "source after alias close")) != 0) return r;

    /* dup2(fd, fd): the POSIX no-op — returns fd, position untouched. */
    if (dup2(fd, fd) != fd) { perror("dup2 no-op"); return 84; }
    if ((r = expect_read(fd, "2E\n", 85, "read after dup2 no-op")) != 0) return r;

    /* The named error: a memfs source with a HOST-numbered dup2 target
     * is ENOTSUP (a host number cannot carry the flag bit), never a
     * silent EBADF — and the host fd survives untouched. */
    hfd = open("/dev/null", O_RDONLY);
    if (hfd < 0) { perror("open /dev/null"); return 86; }
    errno = 0;
    if (dup2(fd, hfd) != -1 || errno != ENOTSUP) {
        fprintf(stderr, "dup2 onto host fd: rc/errno wrong (errno=%d)\n", errno);
        return 87;
    }
    if (close(hfd) != 0) { perror("host fd survived"); return 88; }

    /* dup of a dead memfs fd: EBADF, exactly the kernel's answer. */
    if (close(fd) != 0) { perror("close"); return 89; }
    errno = 0;
    if (dup(fd) != -1 || errno != EBADF) {
        fprintf(stderr, "dup of closed fd: rc/errno wrong (errno=%d)\n", errno);
        return 90;
    }
    /* Host fds pass through to the real dup untouched. */
    hfd = open("/dev/null", O_RDONLY);
    if (hfd < 0) { perror("open /dev/null 2"); return 91; }
    c1 = dup(hfd);
    if (c1 < 0) { perror("dup host"); return 92; }
    if (close(c1) != 0 || close(hfd) != 0) { perror("close host pair"); return 93; }

    puts("dup-probe:ok");
    return 0;
}
