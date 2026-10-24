/* tebako#920 oracle: `getcwd` from inside a preloaded process — once from
 * a CONSTRUCTOR (where the crash report showed 506 frames of the same
 * libtfs_preload:getcwd image offset) and once from main. The shim's
 * readback must answer the HOST directory when no virtual cwd is set; an
 * answer that re-enters the replacement kills the stack, and an answer of
 * the wrong directory is just as wrong. */
#include <stdio.h>
#include <unistd.h>

static char ctor_cwd[4096];

__attribute__((constructor)) static void at_load(void) {
    if (getcwd(ctor_cwd, sizeof ctor_cwd) == NULL) {
        fprintf(stderr, "ctor-getcwd:failed\n");
        _exit(70);
    }
}

int main(void) {
    char cwd[4096];
    if (getcwd(cwd, sizeof cwd) == NULL) {
        perror("getcwd");
        return 71;
    }
    printf("ctor:%s\n", ctor_cwd);
    printf("main:%s\n", cwd);
    return 0;
}
