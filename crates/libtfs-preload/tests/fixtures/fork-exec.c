/* fork-exec: the 2026-08-22 preload fork/exec deadlock regression pin
 * (runtime 0.16.4: a payload mounted at `/` spawning `git clone` wedged
 * git's pre-exec helper child). The parent forks; the CHILD execve's a
 * HOST binary whose path is covered by the shim's root mount — the exec
 * materialization probe reads the in-image manifest through the image
 * backend, whose worker pool (the dwarfs-t block cache) did not survive
 * the fork. Without the shim's fork-child guard the child wedges in the
 * backend dispatch; with it, the child's engine entries pass through to
 * the real execve and the exec completes.
 *
 * The parent is the watchdog: a wedged child is SIGKILLed after a grace
 * window and reported as rc 124, so a regression FAILS the test instead
 * of hanging the suite. */
#include <sys/types.h>
#include <sys/wait.h>
#include <signal.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

extern char **environ;

#define GRACE_MS 15000

int main(int argc, char **argv) {
    if (argc != 3) {
        dprintf(2, "usage: fork-exec <host-tool> <tool-arg>\n");
        return 64;
    }
    pid_t pid = fork();
    if (pid < 0) {
        dprintf(2, "fork: %s\n", strerror(errno));
        return errno;
    }
    if (pid == 0) {
        /* Child: only async-signal-safe work before the exec. */
        char *const child_argv[] = {argv[1], argv[2], NULL};
        execve(argv[1], child_argv, environ);
        _exit(errno ? errno : 1);
    }
    /* Parent watchdog: poll the child; kill + rc 124 on timeout. */
    int status;
    for (int waited = 0;; waited += 100) {
        pid_t r = waitpid(pid, &status, WNOHANG);
        if (r == pid) {
            if (WIFEXITED(status)) return WEXITSTATUS(status);
            if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
        } else if (r < 0) {
            dprintf(2, "waitpid: %s\n", strerror(errno));
            return errno;
        } else if (waited >= GRACE_MS) {
            kill(pid, SIGKILL);
            waitpid(pid, &status, 0);
            dprintf(2, "fork-exec: child wedged — killed after %d ms\n", GRACE_MS);
            return 124;
        } else {
            usleep(100000); /* 100 ms */
        }
    }
}
