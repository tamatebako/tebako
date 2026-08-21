/* win-subject: the spec 25 windows libc-layer dogfood subject (built by
 * CI, never shipped). print-data's ucrt spelling minus the _stat prelude:
 * fopen + fread + fwrite per argv file — the functions retrace's windows
 * ucrt inline hooks interpose (docs/windows.md). The _stat→fopen pair
 * trips CodeQL's TOCTOU rule and the leg's gates key on the fopen-driven
 * read path, so the stat stays out.
 * POSIX-fixture reuse does not port here (print-data.c rides unistd.h);
 * this is the same shape against the CRT the preload-mingw backend hooks.
 *
 * Exit 0 when every file printed; the first failing errno otherwise. */
#include <stdio.h>
#include <errno.h>
#include <string.h>

static int print_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "fopen: %s\n", strerror(errno));
        return errno;
    }
    char buf[4096];
    size_t n;
    while ((n = fread(buf, 1, sizeof buf, f)) > 0) {
        if (fwrite(buf, 1, n, stdout) != n) {
            fclose(f);
            return 74;
        }
    }
    fclose(f);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: win-subject FILE...\n");
        return 64;
    }
    int rc = 0;
    for (int i = 1; i < argc; i++) {
        int r = print_file(argv[i]);
        if (r != 0 && rc == 0)
            rc = r;
    }
    return rc;
}
