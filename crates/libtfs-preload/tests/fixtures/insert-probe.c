/* insert-probe: the tebako#448 exec-guard child probe. Reports whether
 * the insertion variable reached the process image (INSERT:set/unset) and
 * exits 42 — the guard e2e asserts that pair per arch leg. Built per-arch
 * by the test (cc -arch …), never by the fixtures builder. */
#include <stdio.h>
#include <stdlib.h>

int main(void) {
#ifdef __APPLE__
    const char *var = "DYLD_INSERT_LIBRARIES";
#else
    const char *var = "LD_PRELOAD";
#endif
    printf("INSERT:%s\n", getenv(var) ? "set" : "unset");
    return 42;
}
