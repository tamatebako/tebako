/* dl-user: dlopen a library, dlsym plug_value, print its result. */
#include <dlfcn.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: dl-user <library>\n");
        return 64;
    }
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) {
        fprintf(stderr, "dlopen failed\n");
        return 1;
    }
    int (*pv)(void) = (int (*)(void))dlsym(h, "plug_value");
    if (!pv) {
        fprintf(stderr, "dlsym failed\n");
        return 1;
    }
    printf("%d\n", pv());
    return 0;
}
