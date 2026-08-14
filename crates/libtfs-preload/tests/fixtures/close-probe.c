/* spec 22 class-E oracle (darwin): the JVM's jar lifecycle ends with a
 * PLAIN `close` on the flagged memfs fd (libjava's FileDescriptor.close0
 * — ZipFile$Source.close after the CEN/manifest reads). On x86_64 darwin
 * the libc crate maps `libc::close` to `close$NOCANCEL`, so the shim's
 * close tuple covered only the NOCANCEL spelling; a plain close of the
 * flagged fd fell through to the kernel — EBADF — and every `java -jar`
 * against a VFS jar died in LauncherHelper with jar.error1 ("An
 * unexpected error occurred while trying to open file"). This fixture
 * CHECKS close's return: before the plain-close tuple it fails on
 * x86_64 (kernel EBADF); it passes on both arches after. arm64 never
 * regressed (there `libc::close` IS plain close). Darwin-only body (the
 * $NOCANCEL variant family is a mach-o concept). */
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#ifdef __APPLE__

int main(int argc, char **argv) {
    char buf[8];
    int fd;
    int round;
    if (argc < 2) return 64;
    /* Three open/read/close rounds — the JVM opens the jar once for the
     * libjli manifest walk and again for ZipFile$Source. */
    for (round = 0; round < 3; round++) {
        fd = open(argv[1], O_RDONLY);
        if (fd < 0) { perror("open"); return 65; }
        if (lseek(fd, 0, SEEK_SET) < 0) { perror("lseek"); return 65; }
        if (read(fd, buf, 4) != 4) { perror("read"); return 65; }
        /* THE PIN: plain close on a flagged fd must be served by the
         * shim (rc 0), never reach the kernel (EBADF). */
        if (close(fd) != 0) { perror("close"); return 66; }
    }
    puts("close-probe:ok");
    return 0;
}
#else
int main(void) { return 0; }
#endif
