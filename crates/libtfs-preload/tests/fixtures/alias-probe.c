/* alias-probe: the LFS64/fortify/versioned alias surface (tebako#439) —
 * the DISTINCT dynamic symbols a consumer binds instead of the plain
 * names when built with _FILE_OFFSET_BITS=64 / _FORTIFY_SOURCE=2 /
 * against pre-glibc-2.33 headers:
 *
 *   fopen64       — OpenSSL 3.6's openssl_fopen (crypto/o_fopen.c defines
 *                   _FILE_OFFSET_BITS=64 itself on linux): every
 *                   BIO_new_file / X509_LOOKUP_load_file (ruby's
 *                   X509::Store#add_file/set_default_paths) binds this,
 *                   never fopen. Also FLAC's *_init_file and libstdc++'s
 *                   __basic_file::open.
 *   openat64      — Rust std's openat spelling (the runtime's own
 *                   dir-walk), any LFS64 C caller of openat.
 *   __openat_2    — the _FORTIFY_SOURCE=2 three-arg openat (vendored
 *                   C++ in the runtime exe binds it).
 *   __fxstatat64  — the versioned LFS64 fstatat entry.
 *
 * All four are imported by the 0.16.6 linux-gnu-arm64 runtime exe (nm
 * -D proof in the issue). The legs name the symbols EXPLICITLY (the
 * at-probe __xstat idiom) so the probe pins exactly the alias,
 * independent of the build's macro/redirect whims — a redirect that
 * silently fell back to the plain (already interposed) name would make
 * the deny assertions false-green.
 *
 * Each command cats the file to stdout (or prints SIZE for the stat
 * leg) and exits 0; on failure it prints "<op>: <strerror>" to stderr
 * and exits with the errno (EPERM=1, ...). Linux/glibc only. */
#ifdef __linux__
#define _GNU_SOURCE /* struct stat64, fopen64/openat64 declarations */
#endif
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

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
#ifdef __linux__
    if (argc != 3) {
        dprintf(2, "usage: alias-probe <cmd> <path>\n");
        return 64;
    }
    const char *cmd = argv[1];
    const char *path = argv[2];

    if (strcmp(cmd, "fopen64") == 0) {
        FILE *f = fopen64(path, "r");
        if (!f)
            return fail("fopen64", path, errno);
        char buf[4096];
        size_t n;
        while ((n = fread(buf, 1, sizeof buf, f)) > 0)
            write(1, buf, n);
        fclose(f);
        return 0;
    }
    if (strcmp(cmd, "openat64") == 0) {
        int fd = openat64(AT_FDCWD, path, O_RDONLY);
        if (fd < 0)
            return fail("openat64", path, errno);
        return cat_fd(fd);
    }
    if (strcmp(cmd, "__openat_2") == 0) {
        /* glibc's fortified three-arg openat (bits/fcntl2.h): the mode
         * check is compile-time, the runtime symbol takes no mode. */
        extern int __openat_2(int dirfd, const char *path, int flags);
        int fd = __openat_2(AT_FDCWD, path, O_RDONLY);
        if (fd < 0)
            return fail("__openat_2", path, errno);
        return cat_fd(fd);
    }
    if (strcmp(cmd, "__fxstatat64") == 0) {
        /* stat/stat64 are distinct C types on aarch64 (layout-identical);
         * the probe names the wrapper's own type (the at-probe fstatat64
         * idiom). The version argument is glibc-INTERNAL (no public
         * macro): 1 on x86_64 (_STAT_VER_LINUX), 0 on the 64-bit new
         * ports — aarch64 proven twice: the 0.16.6 runtime's own
         * fstatat64 wrapper passes 0 to __fxstatat64, and 1 EINVALs on
         * aarch64 glibc 2.41. */
        extern int __fxstatat64(int ver, int dirfd, const char *path,
                                struct stat64 *st, int flags);
#if defined(__x86_64__)
        const int ver = 1;
#else
        const int ver = 0;
#endif
        struct stat64 st64;
        if (__fxstatat64(ver, AT_FDCWD, path, &st64, 0) < 0)
            return fail("__fxstatat64", path, errno);
        dprintf(1, "SIZE:%lld\n", (long long)st64.st_size);
        return 0;
    }
    dprintf(2, "alias-probe: unknown command %s\n", cmd);
    return 64;
#else
    (void)argc; (void)argv;
    dprintf(2, "alias-probe: linux/glibc only\n");
    return 64;
#endif
}
