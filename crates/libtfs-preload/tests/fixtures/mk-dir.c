/* mk-dir: mkdir a path (the write-class proof). Exit 0 on success;
 * on failure print "mkdir: <strerror>" and exit with the errno value
 * (EROFS=30, EPERM=1). */
#include <sys/stat.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: mk-dir <path>\n");
        return 64;
    }
    if (mkdir(argv[1], 0755) != 0) {
        int e = errno;
        fprintf(stderr, "mkdir: %s\n", strerror(e));
        return e;
    }
    return 0;
}
