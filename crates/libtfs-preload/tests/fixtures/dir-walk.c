/* dir-walk: readdir_r + telldir/seekdir/rewinddir through the shim
 * (roadmap 39). "." and ".." are skipped so output is deterministic for
 * both memfs dirs (no dot entries) and host dirs (dot entries).
 *
 * usage: dir-walk <dir>
 * prints:
 *   R1:<name>|R1:eod     first entry via readdir_r
 *   TELL:<pos>           telldir after the first entry
 *   R2:<name>|R2:eod     second entry
 *   BACK:<name>|BACK:eod seekdir(saved pos of entry 2) + readdir_r
 *   REW:<name>|REW:eod   rewinddir + readdir_r
 *   END:eod              the stream ends cleanly (result NULL, rc 0)
 * Exit 0 on success; "<op>: <strerror>" to stderr and the errno as rc. */
#include <sys/types.h>
#include <dirent.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>

static int fail(const char *op, int e) {
    dprintf(2, "%s: %s\n", op, strerror(e));
    return e;
}

/* next non-dot entry via readdir_r; 1 = entry, 0 = end, < 0 = -errno */
static int next(DIR *d, struct dirent *e) {
    for (;;) {
        struct dirent *res;
        int rc = readdir_r(d, e, &res);
        if (rc)
            return -rc;
        if (!res)
            return 0;
        if (strcmp(e->d_name, ".") && strcmp(e->d_name, ".."))
            return 1;
    }
}

static void print_entry(const char *tag, int has, struct dirent *e) {
    if (has)
        dprintf(1, "%s:%s\n", tag, e->d_name);
    else
        dprintf(1, "%s:eod\n", tag);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        dprintf(2, "usage: dir-walk <dir>\n");
        return 64;
    }
    DIR *d = opendir(argv[1]);
    if (!d)
        return fail("opendir", errno);
    struct dirent e;
    int has = next(d, &e);
    if (has < 0)
        return fail("readdir_r", -has);
    print_entry("R1", has, &e);
    dprintf(1, "TELL:%ld\n", telldir(d));
    long pos2 = telldir(d);
    has = next(d, &e);
    if (has < 0)
        return fail("readdir_r", -has);
    print_entry("R2", has, &e);
    seekdir(d, pos2);
    has = next(d, &e);
    if (has < 0)
        return fail("readdir_r", -has);
    print_entry("BACK", has, &e);
    rewinddir(d);
    has = next(d, &e);
    if (has < 0)
        return fail("readdir_r", -has);
    print_entry("REW", has, &e);
    while ((has = next(d, &e)) > 0)
        ;
    if (has < 0)
        return fail("readdir_r", -has);
    dprintf(1, "END:eod\n");
    if (closedir(d))
        return fail("closedir", errno);
    return 0;
}
