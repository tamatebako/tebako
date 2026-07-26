/* print-data: stat + open + read + close a file, write its bytes to
 * stdout. Exit 0 on success; on failure print "<op>: <strerror>" to
 * stderr and exit with the errno value (EPERM=1, ENOENT=2, ...). */
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        dprintf(2, "usage: print-data <file>\n");
        return 64;
    }
    struct stat st;
    if (stat(argv[1], &st) != 0) {
        int e = errno;
        dprintf(2, "stat: %s\n", strerror(e));
        return e;
    }
    int fd = open(argv[1], O_RDONLY);
    if (fd < 0) {
        int e = errno;
        dprintf(2, "open: %s\n", strerror(e));
        return e;
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
        int e = errno;
        dprintf(2, "read: %s\n", strerror(e));
        return e;
    }
    close(fd);
    return 0;
}
