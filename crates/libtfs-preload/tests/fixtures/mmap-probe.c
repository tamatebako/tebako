/* spec 22 class-E oracle: the JDK's jar-open syscall pattern against a
 * flagged memfs fd. The JDK launcher maps JLI_Lseek to lseek64 on glibc
 * and probes the zip END record with lseek64(SEEK_END)+read; libzip then
 * mmaps the central directory (USE_MMAP, usemmap=TRUE). The debian/temurin
 * builds add _FORTIFY_SOURCE=2 / _FILE_OFFSET_BITS=64, so the read lands
 * on __read_chk and fstat on __fxstat64 — this fixture is compiled with
 * the same flags (see e2e.rs) to pin those exact entries. Before the
 * shim interposed lseek64/mmap64/__read_chk the flagged (virtual) fd
 * reached the kernel — EBADF — and every `java -jar` against a VFS jar
 * failed. Linux-only body (the *64 entry points are glibc names). */
#ifdef __linux__
/* MUST precede every system header: glibc locks the feature set at the
 * first inclusion (features.h via stdio.h); defining it below the
 * includes leaves off64_t/mmap64 undeclared — ubuntu-24.04 CI proved it
 * (tebako run 31705342187). musl exposes the *64 names regardless. */
#define _LARGEFILE64_SOURCE
#endif
#include <stdio.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/mman.h>
#include <sys/stat.h>
#endif

int main(int argc, char **argv) {
#ifdef __linux__
    int fd;
    off64_t end;
    char tail[8];
    void *map;
    void *anon;
    size_t len;
    struct stat st;
    volatile size_t n = 4;
    /* Unbuffered: the stage markers must survive a crash — ubuntu-24.04
     * CI (run 31714704212) ate them to a SIGSEGV's block-buffered loss. */
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) return 64;
    /* The JVM's first allocation is an ANONYMOUS mmap (the PaX check)
     * with fd -1 — whose every bit, TEBAKO_FD_FLAG included, is set. The
     * shim must pass it to the host, not the memfs fd table. */
    anon = mmap64(NULL, 4096, PROT_READ | PROT_WRITE,
                  MAP_PRIVATE | MAP_ANONYMOUS, -1, (off64_t)0);
    if (anon == MAP_FAILED) { perror("mmap64-anon"); return 65; }
    memset(anon, 0x5a, 4096);
    munmap(anon, 4096);
    puts("anon-mmap:ok");
    fd = open(argv[1], O_RDONLY);
    if (fd < 0) { perror("open"); return 66; }
    /* libjava's open path stats the jar (with _FILE_OFFSET_BITS=64 this
     * is __fxstat64 on glibc < 2.33). */
    if (fstat(fd, &st) < 0) { perror("fstat"); return 65; }
    /* The launcher's END-record probe: seek to size-4, read the tail.
     * volatile n defeats constant folding so fortify emits __read_chk —
     * libjli's fortified read, exactly. */
    end = lseek64(fd, (off64_t)-4, SEEK_END);
    if (end < 0) { perror("lseek64"); return 65; }
    memset(tail, 0, sizeof tail);
    if (read(fd, tail, n) != (ssize_t) n) { perror("read-tail"); return 65; }
    /* libzip's central-directory window (PROT_READ|MAP_SHARED is its
     * exact request). */
    len = (size_t) lseek64(fd, (off64_t)0, SEEK_END);
    map = mmap64(NULL, len, PROT_READ, MAP_SHARED, fd, (off64_t)0);
    if (map == MAP_FAILED) { perror("mmap64"); return 65; }
    printf("lseek64-tail:%.*s\n", 4, tail);
    printf("mmap64-head:%.*s\n", (int)(len < 16 ? len : 16), (char *)map);
    munmap(map, len);
    close(fd);
    return 0;
#else
    (void)argc; (void)argv;
    puts("unsupported-platform");
    return 0;
#endif
}
