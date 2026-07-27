/* at-probe: exercises the *at stat family through the shim (roadmap 39).
 *
 * usage:
 *   at-probe <path>               fstatat(AT_FDCWD, path) + statx (linux)
 *                                 -> "FSTATAT:<size>" / "STATX:<size>"
 *   at-probe --rel <relpath>      fstatat(AT_FDCWD, relpath) against the
 *                                 inherited cwd -> "REL:<size>"
 *                                 (the AT_FDCWD regression pin: an ungated
 *                                 is_memfs_fd(AT_FDCWD) branch answers
 *                                 ENOTDIR here — the 4.0 bug)
 *   at-probe --dirfd <dir> <rel>  open(dir) + fstatat(fd, rel) + openat
 *                                 -> "FSTATAT:<size>" + the file's bytes
 * Exit 0 on success; "<op>: <strerror>" to stderr and the errno as rc. */
#define _GNU_SOURCE
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>

static int fail(const char *op, int e) {
    dprintf(2, "%s: %s\n", op, strerror(e));
    return e;
}

static int do_fstatat(int dirfd, const char *path, const char *tag) {
    struct stat st;
    if (fstatat(dirfd, path, &st, 0) != 0)
        return fail("fstatat", errno);
    dprintf(1, "%s:%lld\n", tag, (long long)st.st_size);
#ifdef __linux__
    struct statx sx;
    if (statx(dirfd, path, 0, STATX_BASIC_STATS, &sx) != 0)
        return fail("statx", errno);
    dprintf(1, "STATX:%lld\n", (long long)sx.stx_size);
#endif
    return 0;
}

static int cat_fd(int fd) {
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof buf)) > 0)
        if (write(1, buf, (size_t)n) <= 0)
            return 74;
    if (n < 0)
        return fail("read", errno);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 3 && strcmp(argv[1], "--rel") == 0) {
        struct stat st;
        if (fstatat(AT_FDCWD, argv[2], &st, 0) != 0)
            return fail("fstatat(AT_FDCWD, rel)", errno);
        dprintf(1, "REL:%lld\n", (long long)st.st_size);
        return 0;
    }
    if (argc == 4 && strcmp(argv[1], "--dirfd") == 0) {
        int dfd = open(argv[2], O_RDONLY | O_DIRECTORY);
        if (dfd < 0)
            return fail("open(dir)", errno);
        int rc = do_fstatat(dfd, argv[3], "FSTATAT");
        if (rc)
            return rc;
        int fd = openat(dfd, argv[3], O_RDONLY);
        if (fd < 0)
            return fail("openat", errno);
        rc = cat_fd(fd);
        close(fd);
        close(dfd);
        return rc;
    }
    if (argc == 2)
        return do_fstatat(AT_FDCWD, argv[1], "FSTATAT");
    dprintf(2, "usage: at-probe <path> | --rel <rel> | --dirfd <dir> <rel>\n");
    return 64;
}
