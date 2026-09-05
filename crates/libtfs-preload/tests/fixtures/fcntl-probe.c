/* CPython boot-path oracle (the python runtime factory dogfood,
 * TODO.python/02): io.FileIO.__init__ runs _Py_set_inheritable →
 * get_inheritable → fcntl(F_GETFD) on the freshly opened fd with
 * raise=1 (Modules/_io/fileio.c:451 → Python/fileutils.c). The shim's
 * memfs fds are flag-bit integers with no kernel state, so the real
 * fcntl answered EBADF and EVERY source open of the unpatched
 * interpreter died importing `encodings`, before any user code. The
 * shim now serves the descriptor commands from its own fd table:
 * F_GETFD/F_SETFD track the close-on-exec bit (O_CLOEXEC at open is
 * the seed), F_GETFL answers the truthful read-only status word,
 * F_SETFL is an accepted no-op, and a flagged-but-closed fd is EBADF —
 * the kernel's own answer. A host fd must pass through untouched.
 * The dup class (F_DUPFD and kin) is EINVAL by design (duplicating a
 * memfs fd is the engine's to own) and is deliberately not pinned
 * here. Cross-platform: fcntl's fd commands are plain POSIX. */
#include <errno.h>
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>

#ifdef __linux__
/* glibc's headers redirect fcntl→fcntl64 under _FILE_OFFSET_BITS=64 —
 * which CPython's pyconfig.h sets on every gnu build — so a gnu
 * interpreter's get_inheritable probe names fcntl64, never fcntl
 * (tebako#529: the unexported alias bound glibc's real fcntl64 and the
 * synthetic memfs fd died EBADF at init_fs_encoding). Declared by hand
 * so no feature-test macro is needed. */
extern int fcntl64(int fd, int cmd, ...);
#endif

int main(int argc, char **argv) {
    char buf[16];
    int fd, fl, hfd;
    if (argc < 2) return 64;
    /* CPython opens sources with O_CLOEXEC (PEP 446: non-inheritable
     * by default) — the open seeds the shim's fd-table bit. */
    fd = open(argv[1], O_RDONLY | O_CLOEXEC);
    if (fd < 0) { perror("open"); return 65; }
    /* THE PIN: F_GETFD on the flagged fd must be served by the shim
     * (FD_CLOEXEC set), never reach the kernel (EBADF). */
    fl = fcntl(fd, F_GETFD);
    if (fl < 0 || !(fl & FD_CLOEXEC)) { perror("fcntl(F_GETFD)"); return 66; }
    /* F_SETFD clears, F_GETFD reads back clear… */
    if (fcntl(fd, F_SETFD, 0) != 0) { perror("fcntl(F_SETFD,0)"); return 67; }
    fl = fcntl(fd, F_GETFD);
    if (fl != 0) { fprintf(stderr, "F_GETFD after clear: %#x\n", fl); return 68; }
    /* …F_SETFD sets again. */
    if (fcntl(fd, F_SETFD, FD_CLOEXEC) != 0) { perror("fcntl(F_SETFD,CLOEXEC)"); return 69; }
    /* F_GETFL: the truthful read-only status word (payloads are ro). */
    fl = fcntl(fd, F_GETFL);
    if (fl < 0) { perror("fcntl(F_GETFL)"); return 70; }
    if ((fl & O_ACCMODE) != O_RDONLY) { fprintf(stderr, "F_GETFL accmode: %#x\n", fl); return 71; }
    /* F_SETFL is an accepted no-op (a memfs read never blocks). */
    if (fcntl(fd, F_SETFL, O_NONBLOCK) != 0) { perror("fcntl(F_SETFL)"); return 72; }
#ifdef __linux__
    /* tebako#529: the same descriptor dance through the LFS alias —
     * the gnu CPython binary's actual entry point. */
    fl = fcntl64(fd, F_GETFD);
    if (fl < 0 || !(fl & FD_CLOEXEC)) { perror("fcntl64(F_GETFD)"); return 79; }
    if (fcntl64(fd, F_SETFD, 0) != 0) { perror("fcntl64(F_SETFD,0)"); return 80; }
    fl = fcntl64(fd, F_GETFD);
    if (fl != 0) { fprintf(stderr, "fcntl64 F_GETFD after clear: %#x\n", fl); return 81; }
    fl = fcntl64(fd, F_GETFL);
    if (fl < 0 || (fl & O_ACCMODE) != O_RDONLY) { fprintf(stderr, "fcntl64 F_GETFL: %#x\n", fl); return 82; }
    if (fcntl64(fd, F_SETFD, FD_CLOEXEC) != 0) { perror("fcntl64(F_SETFD,CLOEXEC)"); return 83; }
#endif
    /* The fd still reads after the flag dances. */
    if (read(fd, buf, sizeof(buf)) <= 0) { perror("read"); return 73; }
    if (close(fd) != 0) { perror("close"); return 74; }
    /* A flagged-but-closed fd: EBADF, exactly the kernel's answer. */
    errno = 0;
    if (fcntl(fd, F_GETFD) != -1 || errno != EBADF) {
        fprintf(stderr, "fcntl on closed fd: rc/errno wrong\n");
        return 75;
    }
    /* Host fds pass through to the real fcntl untouched. */
    hfd = open("/dev/null", O_RDONLY);
    if (hfd < 0) { perror("open /dev/null"); return 76; }
    if (fcntl(hfd, F_GETFD) < 0) { perror("fcntl host F_GETFD"); return 77; }
    if (close(hfd) != 0) { perror("close /dev/null"); return 78; }
    puts("fcntl-probe:ok");
    return 0;
}
