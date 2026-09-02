/* vfork-probe: the vfork arm of the preload fork-guard regression pin
 * (the 2026-09-03 dogfood-repro deadlock). glibc runs NO pthread_atfork
 * handlers for vfork(2), so the atfork child guard alone never armed in
 * a vfork child — and a vfork child shares the parent's whole address
 * space with the parent's calling thread suspended in the kernel, so an
 * engine entry there raced live backend state in memory the parent still
 * owned (dash's execvp PATH search: parent in kernel_clone, child in
 * futex_wait — deterministic under Rosetta, latent corruption natively).
 * The shim's pid gate is the vfork backstop: every engine entry in a
 * vfork child passes through to the host libc, exactly like the fork
 * guard.
 *
 * Protocol: vfork; the CHILD stat()s argv[1] through the interposed
 * shim and writes one answer byte down the pipe — 'H' stat hit, 'E'
 * ENOENT, 'X' any other errno — then _exit(0). The parent prints the
 * byte. The test stats a path that exists IN THE IMAGE but not on the
 * host under a root mount: the gated child must answer the HOST's
 * ENOENT ('E'); ungated it either wedges (deadlock) or answers the
 * image's 'H' — both fail the test.
 *
 * Watchdog: a wedged vfork child leaves the PARENT suspended in the
 * kernel, so the parent cannot be its own watchdog (the fork-exec
 * fixture's pattern does not reach vfork). A forked SIBLING watchdog
 * SIGKILLs the probe's own process group after a grace window — the
 * probe setpgid's first so the kill can never reach the test harness's
 * group. A regression reports a signal death, not a hung suite.
 *
 * The parent warms the pass-through slots the gated child will need
 * (real_stat via a covered-but-MISSING path's host fallback, real_write
 * via a zero-byte pipe write): the slots resolve lazily through dlsym,
 * and a first-call dlsym inside the vfork child's shared address space
 * would itself be undefined. */
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <signal.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#define GRACE_S 15

int main(int argc, char **argv) {
    if (argc != 2) {
        dprintf(2, "usage: vfork-probe <path>\n");
        return 64;
    }
    /* Own process group: the watchdog's kill(0, …) reaches only this
     * probe and its children, never the spawning test harness. */
    (void)setpgid(0, 0);
    int p[2];
    if (pipe(p) != 0) {
        dprintf(2, "pipe: %s\n", strerror(errno));
        return 70;
    }
    pid_t wd = fork();
    if (wd < 0) {
        dprintf(2, "fork: %s\n", strerror(errno));
        return 70;
    }
    if (wd == 0) {
        /* Watchdog sibling. Only non-interposed calls here: a fork
         * child's first dlsym on an interposed symbol could take a
         * loader lock a dead sibling thread held. */
        struct timespec ts = {GRACE_S, 0};
        nanosleep(&ts, NULL);
        kill(0, SIGKILL);
        _exit(0);
    }
    /* Warm the gated child's pass-through slots (see the header). A
     * covered-but-missing path routes to the host fallback, resolving
     * real_stat; the zero-byte write resolves real_write. */
    struct stat st;
    (void)stat("/tebako-vfork-probe-warm-nonexistent", &st);
    (void)write(p[1], "", 0);

    pid_t pid = vfork();
    if (pid < 0) {
        dprintf(2, "vfork: %s\n", strerror(errno));
        return 70;
    }
    if (pid == 0) {
        /* vfork child: SHARED address space, parent thread suspended.
         * Nothing here may allocate, lock, or return from this frame. */
        char b;
        if (stat(argv[1], &st) == 0) {
            b = 'H';
        } else if (errno == ENOENT) {
            b = 'E';
        } else {
            b = 'X';
        }
        (void)write(p[1], &b, 1);
        _exit(0);
    }
    close(p[1]);
    char b = '?';
    ssize_t n = read(p[0], &b, 1);
    kill(wd, SIGKILL);
    waitpid(wd, NULL, 0);
    waitpid(pid, NULL, 0);
    if (n != 1) {
        dprintf(2, "vfork-probe: no answer from the child\n");
        return 71;
    }
    printf("%c\n", b);
    return 0;
}
