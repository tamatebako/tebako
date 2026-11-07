/* connect-probe <host> <port> [4|6] — the network axis's e2e instrument
 * (spec 08 §2). Resolves <host> with getaddrinfo (observed, never gated
 * — the DNS carve-out), connects to the FIRST answer, prints the
 * verdict:
 *
 *   CONNECTED            the peer was allowed and accepted
 *   REFUSED <errno>      connect failed; <errno> names the cause
 *                        (EPERM = the jail's refusal)
 *   GAI-FAIL <text>      the resolve itself failed
 *
 * The optional third argument pins the resolver's address family
 * (`4` = AF_INET, `6` = AF_INET6) — a test whose listener lives on one
 * family keeps "localhost" deterministic through it.
 *
 * Exit 0 on CONNECTED, 3 on the jail's EPERM, 4 otherwise.
 */
#include <errno.h>
#include <netdb.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc < 3 || argc > 4) {
        fprintf(stderr, "usage: connect-probe <host> <port> [4|6]\n");
        return 2;
    }
    struct addrinfo hints;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    if (argc == 4) {
        hints.ai_family = argv[3][0] == '4' ? AF_INET : AF_INET6;
    }
    struct addrinfo *res = NULL;
    int rc = getaddrinfo(argv[1], argv[2], &hints, &res);
    if (rc != 0) {
        printf("GAI-FAIL %s\n", gai_strerror(rc));
        return 4;
    }
    int fd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (fd < 0) {
        printf("SOCK-FAIL %d\n", errno);
        freeaddrinfo(res);
        return 4;
    }
    if (connect(fd, res->ai_addr, res->ai_addrlen) == 0) {
        printf("CONNECTED\n");
        close(fd);
        freeaddrinfo(res);
        return 0;
    }
    int e = errno;
    printf("REFUSED %d %s\n", e, strerror(e));
    close(fd);
    freeaddrinfo(res);
    return e == EPERM ? 3 : 4;
}
