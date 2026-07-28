/* at-probe: the *at family (roadmap 39) — fstatat/fstatat64/openat
 * everywhere, statx/getdents64/__xstat on linux. Each command prints a
 * deterministic line or exits with the errno. */
#ifdef __linux__
#define _GNU_SOURCE /* statx, getdents64, STATX_* */
#endif
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#ifdef __linux__
#include <dirent.h> /* getdents64 (bits/dirent_ext.h) */
#endif

static int fail(const char *what, const char *path, int e) {
    dprintf(2, "%s %s: %s\n", what, path, strerror(e));
    return e;
}

static int cat_fd(int fd) {
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof buf)) > 0)
        write(1, buf, (size_t)n);
    close(fd);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        dprintf(2, "usage: at-probe <cmd> <path> [base]\n");
        return 64;
    }
    const char *cmd = argv[1];
    const char *path = argv[2];
    struct stat st;

    if (strcmp(cmd, "fstatat") == 0) {
        if (fstatat(AT_FDCWD, path, &st, 0) < 0)
            return fail("fstatat", path, errno);
        dprintf(1, "SIZE:%lld\n", (long long)st.st_size);
        return 0;
    }
    if (strcmp(cmd, "fstatat-rel") == 0) {
        /* dirfd-relative: the base dir is argv[3], path is the basename. */
        int dfd = open(argv[3], O_RDONLY | O_DIRECTORY);
        if (dfd < 0)
            return fail("open-dir", argv[3], errno);
        int rc = fstatat(dfd, path, &st, 0) < 0 ? fail("fstatat-rel", path, errno) : 0;
        close(dfd);
        if (rc)
            return rc;
        dprintf(1, "SIZE:%lld\n", (long long)st.st_size);
        return 0;
    }
    if (strcmp(cmd, "openat") == 0) {
        int dfd = open(argv[3], O_RDONLY | O_DIRECTORY);
        if (dfd < 0)
            return fail("open-dir", argv[3], errno);
        int fd = openat(dfd, path, O_RDONLY);
        if (fd < 0)
            return fail("openat", path, errno);
        close(dfd);
        return cat_fd(fd);
    }
#ifdef __linux__
    if (strcmp(cmd, "fstatat64") == 0) {
        /* stat/stat64 are distinct C types on aarch64 (layout-identical);
         * the probe names the wrapper's own type. */
        struct stat64 st64;
        if (fstatat64(AT_FDCWD, path, &st64, 0) < 0)
            return fail("fstatat64", path, errno);
        dprintf(1, "SIZE:%lld\n", (long long)st64.st_size);
        return 0;
    }
    if (strcmp(cmd, "statx") == 0) {
        struct statx stx;
        if (statx(AT_FDCWD, path, 0, STATX_BASIC_STATS, &stx) < 0)
            return fail("statx", path, errno);
        dprintf(1, "SIZE:%lld MASK:%x\n", (long long)stx.stx_size, stx.stx_mask);
        return 0;
    }
    if (strcmp(cmd, "getdents64") == 0) {
        /* path names a directory to enumerate via the raw syscall form. */
        int fd = open(path, O_RDONLY | O_DIRECTORY);
        if (fd < 0)
            return fail("open-dir", path, errno);
        char buf[1024];
        int n = getdents64(fd, buf, sizeof buf);
        int e = errno;
        close(fd);
        if (n < 0)
            return fail("getdents64", path, e);
        dprintf(1, "BYTES:%d\n", n);
        return 0;
    }
    if (strcmp(cmd, "getdents64-file") == 0) {
        /* path names a memfs REGULAR file: ENOTDIR is the honest answer. */
        int fd = open(path, O_RDONLY);
        if (fd < 0)
            return fail("open", path, errno);
        char buf[1024];
        int n = getdents64(fd, buf, sizeof buf);
        int e = errno;
        close(fd);
        if (n >= 0) {
            dprintf(2, "getdents64 on a file succeeded?!\n");
            return 1;
        }
        dprintf(1, "ERRNO:%d\n", e);
        return 0;
    }
    if (strcmp(cmd, "__xstat") == 0) {
        extern int __xstat(int ver, const char *path, struct stat *st);
        if (__xstat(1, path, &st) < 0)
            return fail("__xstat", path, errno);
        dprintf(1, "SIZE:%lld\n", (long long)st.st_size);
        return 0;
    }
    if (strcmp(cmd, "__fxstatat") == 0) {
        extern int __fxstatat(int ver, int dirfd, const char *path, struct stat *st, int flags);
        if (__fxstatat(1, AT_FDCWD, path, &st, 0) < 0)
            return fail("__fxstatat", path, errno);
        dprintf(1, "SIZE:%lld\n", (long long)st.st_size);
        return 0;
    }
#endif
    dprintf(2, "at-probe: unknown command %s\n", cmd);
    return 64;
}
