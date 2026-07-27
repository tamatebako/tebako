/* dents-probe (linux only): getdents64 + the pre-glibc-2.33 __xstat
 * entry point through the shim (roadmap 39).
 *
 * usage: dents-probe <memfs-file> <host-dir>
 *  - open(memfs-file) + getdents64(fd) -> a memfs fd is a regular file:
 *    the honest answer is ENOTDIR ("DENTS-MEMFS:ENOTDIR")
 *  - open(host-dir, O_DIRECTORY) + getdents64 -> passthrough works
 *    ("DENTS-HOST:ok")
 *  - dlsym(RTLD_DEFAULT, "__xstat")(1, memfs-file, &st) -> "XSTAT:<size>"
 *    (under LD_PRELOAD the shim's export is found first — the same
 *     binding order an old binary's versioned __xstat reference resolves
 *     through; _STAT_VER == 1 is the x86-64 layout)
 * Exit 0 on success; "<op>: <strerror>" to stderr and the errno as rc. */
#define _GNU_SOURCE
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>
#include <dlfcn.h>

/* glibc 2.30+ declares it in dirent.h; declare it ourselves so the build
 * does not depend on the header vintage. */
extern ssize_t getdents64(int fd, void *buf, size_t nbytes);

static int fail(const char *op, int e) {
    dprintf(2, "%s: %s\n", op, strerror(e));
    return e;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        dprintf(2, "usage: dents-probe <memfs-file> <host-dir>\n");
        return 64;
    }
    int fd = open(argv[1], O_RDONLY);
    if (fd < 0)
        return fail("open(memfs)", errno);
    char buf[4096];
    errno = 0;
    ssize_t n = getdents64(fd, buf, sizeof buf);
    if (n >= 0 || errno != ENOTDIR) {
        dprintf(2, "getdents64(memfs fd): expected ENOTDIR, got n=%zd errno=%d\n", n, errno);
        return 70;
    }
    dprintf(1, "DENTS-MEMFS:ENOTDIR\n");
    close(fd);

    int dfd = open(argv[2], O_RDONLY | O_DIRECTORY);
    if (dfd < 0)
        return fail("open(hostdir)", errno);
    n = getdents64(dfd, buf, sizeof buf);
    if (n <= 0)
        return fail("getdents64(hostdir)", errno);
    dprintf(1, "DENTS-HOST:ok\n");
    close(dfd);

#if defined(__x86_64__) && defined(__GLIBC__)
    int (*xstat_fn)(int, const char *, struct stat *) =
        (int (*)(int, const char *, struct stat *))dlsym(RTLD_DEFAULT, "__xstat");
    if (!xstat_fn) {
        dprintf(2, "dlsym(__xstat): %s\n", dlerror());
        return 69;
    }
    struct stat st;
    if (xstat_fn(1, argv[1], &st) != 0)
        return fail("__xstat", errno);
    dprintf(1, "XSTAT:%lld\n", (long long)st.st_size);
#else
    dprintf(1, "XSTAT:skipped\n");
#endif
    return 0;
}
