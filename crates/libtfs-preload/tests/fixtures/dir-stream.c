/* dir-stream: rewinddir/telldir/seekdir/readdir_r (roadmap 39). Prints a
 * deterministic transcript of one stream's navigation; the e2e asserts it
 * line for line. */
#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>

static void show(const char *tag, struct dirent *e) {
    dprintf(1, "%s:%s\n", tag, e ? e->d_name : "<end>");
}

int main(int argc, char **argv) {
    if (argc != 2) {
        dprintf(2, "usage: dir-stream <dir>\n");
        return 64;
    }
    DIR *d = opendir(argv[1]);
    if (!d) {
        dprintf(2, "opendir %s: %s\n", argv[1], strerror(errno));
        return errno;
    }
    show("r1", readdir(d));
    dprintf(1, "tell:%ld\n", telldir(d));
    show("r2", readdir(d));
    show("r3", readdir(d)); /* end of stream */
    rewinddir(d);
    show("after-rewind", readdir(d));
    seekdir(d, 1);
    struct dirent entry, *result;
    int rc = readdir_r(d, &entry, &result);
    dprintf(1, "readdir_r:%d:%s\n", rc, result ? entry.d_name : "<end>");
    rc = readdir_r(d, &entry, &result);
    dprintf(1, "readdir_r:%d:%s\n", rc, result ? entry.d_name : "<end>");
    closedir(d);
    return 0;
}
