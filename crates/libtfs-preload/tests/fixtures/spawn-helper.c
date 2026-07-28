/* spawn-helper: execve/posix_spawn of a MEMFS path (roadmap 39). The
 * helper lives inside the image; the shim must materialize it through the
 * dlmap2file host cache so it runs with no extraction, and the preload
 * env must reach it (grandchildren stay in the VFS). */
#include <sys/wait.h>
#include <spawn.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

extern char **environ;

int main(int argc, char **argv) {
    if (argc != 4) {
        dprintf(2, "usage: spawn-helper <execve|posix_spawn> <helper> <arg>\n");
        return 64;
    }
    char *const child_argv[] = {argv[2], argv[3], NULL};
    if (strcmp(argv[1], "execve") == 0) {
        execve(argv[2], child_argv, environ);
        int e = errno;
        dprintf(2, "execve %s: %s\n", argv[2], strerror(e));
        return e;
    }
    if (strcmp(argv[1], "posix_spawn") == 0) {
        pid_t pid;
        int s = posix_spawn(&pid, argv[2], NULL, NULL, child_argv, environ);
        if (s != 0) {
            dprintf(2, "posix_spawn %s: %s\n", argv[2], strerror(s));
            return s;
        }
        int status;
        waitpid(pid, &status, 0);
        return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    }
    dprintf(2, "spawn-helper: unknown mode %s\n", argv[1]);
    return 64;
}
