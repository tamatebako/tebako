/* spawn-helper: spawn an executable with one data-path argument and
 * report the child's exit code (roadmap 39: exec/spawn of memfs paths).
 *
 * usage:
 *   spawn-helper --spawn <prog> <data>   posix_spawn  + waitpid
 *   spawn-helper --spawnp <prog> <data>  posix_spawnp + waitpid
 *   spawn-helper --execve <prog> <data>  fork + execve(child) + waitpid
 * prints "SPAWN-RC:<n>"; on spawn failure "SPAWN-ERR:<errno>" and exits
 * with the errno value. The child inherits environ (the preload env
 * propagates — the grandchild stays in the VFS). */
#include <sys/types.h>
#include <sys/wait.h>
#include <spawn.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>

extern char **environ;

static int report(pid_t pid) {
    int status;
    if (waitpid(pid, &status, 0) < 0) {
        int e = errno;
        dprintf(2, "waitpid: %s\n", strerror(e));
        return e;
    }
    dprintf(1, "SPAWN-RC:%d\n", WIFEXITED(status) ? WEXITSTATUS(status) : -1);
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 4) {
        dprintf(2, "usage: spawn-helper --spawn|--spawnp|--execve <prog> <data>\n");
        return 64;
    }
    char *child_argv[] = {argv[2], argv[3], NULL};
    pid_t pid;
    if (strcmp(argv[1], "--execve") == 0) {
        pid = fork();
        if (pid < 0) {
            int e = errno;
            dprintf(2, "fork: %s\n", strerror(e));
            return e;
        }
        if (pid == 0) {
            execve(argv[2], child_argv, environ);
            _exit(errno);
        }
        return report(pid);
    }
    int rc;
    if (strcmp(argv[1], "--spawn") == 0)
        rc = posix_spawn(&pid, argv[2], NULL, NULL, child_argv, environ);
    else if (strcmp(argv[1], "--spawnp") == 0)
        rc = posix_spawnp(&pid, argv[2], NULL, NULL, child_argv, environ);
    else {
        dprintf(2, "unknown mode %s\n", argv[1]);
        return 64;
    }
    if (rc != 0) {
        dprintf(1, "SPAWN-ERR:%d\n", rc);
        return rc;
    }
    return report(pid);
}
