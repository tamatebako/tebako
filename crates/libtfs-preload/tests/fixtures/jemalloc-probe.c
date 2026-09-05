/* jemalloc-probe: the tebako#527 wedge in one binary.
 *
 * Linked with `-Wl,--export-dynamic` and a STATIC jemalloc (see the
 * e2e.rs test for the exact flags), so the exe's own dynamic table
 * exports malloc/free/calloc and every allocation in the process —
 * including the ones glibc's dlsym makes — enters THIS jemalloc. The
 * shim's constructor runs before main; its first allocation triggers
 * jemalloc's lazy malloc_init_hard, whose arena-base pages_map mmap
 * re-enters the shim's interposed mmap. Pre-fix that mmap resolved the
 * real symbol with a lazy dlsym, whose allocation re-entered jemalloc
 * and self-deadlocked the non-recursive init_lock: the process parked
 * forever before main. Post-fix the shim's mm family answers through
 * the raw syscall and main runs. A plain malloc/free pair is the whole
 * probe — reaching `return 0` at all IS the assertion.
 */
#include <stdlib.h>

int main(void) {
    void *p = malloc(1);
    free(p);
    return p == NULL;
}
