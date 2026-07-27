/* helper: the in-image spawned tool (roadmap 39). Packed into the test
 * image and exec'd via the shim's dlmap2file materialization; it reads
 * the data path given in argv[1] to prove the grandchild's shim mounted
 * the image (the preload env propagates). Prints HELPER:ok, then the
 * file's bytes. Exit 0 on success; the errno as rc on failure. */
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>

int main(int argc, char **argv) {
    dprintf(1, "HELPER:ok\n");
    if (argc < 2) {
        dprintf(2, "usage: helper <file>\n");
        return 64;
    }
    int fd = open(argv[1], O_RDONLY);
    if (fd < 0) {
        int e = errno;
        dprintf(2, "open %s: %s\n", argv[1], strerror(e));
        return e;
    }
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof buf)) > 0)
        if (write(1, buf, (size_t)n) <= 0)
            return 74;
    if (n < 0) {
        int e = errno;
        dprintf(2, "read: %s\n", strerror(e));
        return e;
    }
    close(fd);
    return 0;
}
