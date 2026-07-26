/* spawn-self: the grandchild proof. Reads a file itself, then
 * posix_spawns ITSELF (argv[0]) with --child; the child re-reads the
 * file and reports whether the preload env propagated. */
#include <sys/stat.h>
#include <sys/wait.h>
#include <spawn.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>

extern char **environ;

static int cat(const char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) {
        int e = errno;
        dprintf(2, "open %s: %s\n", path, strerror(e));
        return e;
    }
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof buf)) > 0)
        write(1, buf, (size_t)n);
    close(fd);
    return 0;
}

int main(int argc, char **argv) {
    if (argc >= 3 && strcmp(argv[1], "--child") == 0) {
        dprintf(1, "CHILD-ENV:%s\n", getenv("TEBAKO_TFS_MOUNTS") ? "ok" : "missing");
        return cat(argv[2]);
    }
    if (argc != 2) {
        dprintf(2, "usage: spawn-self <file>\n");
        return 64;
    }
    int rc = cat(argv[1]);
    if (rc)
        return rc;
    char *child_argv[] = {argv[0], (char *)"--child", argv[1], NULL};
    pid_t pid;
    int s = posix_spawn(&pid, argv[0], NULL, NULL, child_argv, environ);
    if (s != 0) {
        dprintf(2, "posix_spawn: %s\n", strerror(s));
        return s;
    }
    int status;
    waitpid(pid, &status, 0);
    dprintf(1, "SPAWN-RC:%d\n", WIFEXITED(status) ? WEXITSTATUS(status) : -1);
    return 0;
}
