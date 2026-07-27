/* list-dir: opendir + readdir + closedir, one "name type" per line. */
#include <dirent.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: list-dir <dir>\n");
        return 64;
    }
    DIR *d = opendir(argv[1]);
    if (!d) {
        fprintf(stderr, "opendir: %s\n", strerror(errno));
        return errno;
    }
    struct dirent *e;
    while ((e = readdir(d)) != NULL)
        printf("%s %d\n", e->d_name, (int)e->d_type);
    closedir(d);
    return 0;
}
