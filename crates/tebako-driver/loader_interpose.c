/* -- tebako loader interposition (spec 22, class L, macOS) -- */
/* The micro interpose-dylib the driver embeds (build.rs compiles it,
   src/ffi/interpose.rs embeds its bytes) and self-inserts at the head of
   the boot: dyld honors __interpose tuples only from a DYLIB image —
   tuples in the main executable are silently ignored, and a dylib
   dlopen'd after launch stays inert (spec 22 §2, verified empirically) —
   so the macOS delivery is this dylib plus DYLD_INSERT_LIBRARIES +
   re-exec, where the ELF delivery is the ruby patch's exe-defined
   wrappers (tamatebako/ruby patches/*\/dln_c_loader_interpose.patch).

   The route glue below MIRRORS that patch's semantics exactly: the same
   coverage check (tebako_path_is_embedded), the same materialization
   call (tebako_fs_dlmap2file — the library plus its dependency closure
   to the exec cache), the same ENOENT covered-but-not-held pass-through
   (the dln_load precedent), the same verdict line carried on the dlerror
   channel, the same __thread verdict stash. The two shells differ in
   plumbing (dyld tuples vs exe-preempt), never in the contract — a
   change to the route semantics lands in BOTH or in neither.

   This dylib links NO tebako library: the tebako_fs_* symbols bind the
   exe's own exports (exports.txt) at run time via -undefined
   dynamic_lookup — one VFS context in the process, owned by the exe.
   Dependencies stay at dlfcn/stdio/stdlib/string/errno, so the
   authoritative declarations in include/tebako/fs/c_api.h are restated
   here by hand. */

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

/* The exe's tebako_fs_* exports (see include/tebako/fs/c_api.h — the
   single authority; the returned strings are malloc'd, the caller
   frees). */
extern int tebako_path_is_embedded(const char *path);
extern char *tebako_fs_mount_of(const char *path);
extern char *tebako_fs_dlmap2file(const char *path);

typedef void *(*tfs_dlopen_fn)(const char *, int);
typedef char *(*tfs_dlerror_fn)(void);

static __thread char tfs_dl_verdict[1024];
static __thread int tfs_dl_verdict_pending;

/* The materialization verdict becomes the dlerror() answer of the
   failed call, so ffi/fiddle/dln raise carrying this exact line. (The
   ELF sibling — the ruby dln_c_loader_interpose patch — keeps the same
   verdict wording: the two shells differ in plumbing, not in the
   contract.) */
static void
tfs_dl_remember(const char *path, int err)
{
    char *mount = tebako_fs_mount_of(path);
    snprintf(tfs_dl_verdict, sizeof(tfs_dl_verdict),
             "tebako: cannot materialize VFS-resident library '%s' (mount '%s'): %s",
             path, mount != NULL ? mount : "(unknown)", strerror(err));
    free(mount);
    tfs_dl_verdict_pending = 1;
}

static void *
tfs_dlopen_route(const char *path, int mode, tfs_dlopen_fn real_dlopen)
{
    void *handle;
    char *mapped;
    int err;

    tfs_dl_verdict_pending = 0;
    if (!tebako_path_is_embedded(path)) {
        return real_dlopen(path, mode);
    }
    mapped = tebako_fs_dlmap2file(path);
    if (mapped != NULL) {
        handle = real_dlopen(mapped, mode);
        free(mapped);
        return handle;
    }
    err = errno;
    if (err == ENOENT) {
        /* covered but not held: a host path the mount happens to
           cover -- the raw call serves it from the host */
        return real_dlopen(path, mode);
    }
    tfs_dl_remember(path, err);
    errno = err;  /* the failed call's own errno, for direct errno readers */
    return NULL;
}

static char *
tfs_dlerror_route(tfs_dlerror_fn real_dlerror)
{
    if (tfs_dl_verdict_pending) {
        tfs_dl_verdict_pending = 0;
        return tfs_dl_verdict;
    }
    return real_dlerror();
}

/* dyld interpose tuples. dyld rebinds EVERY reference to the replacee
   process-wide -- this translation unit's own included -- so the
   originals are reachable ONLY through the tuples' replacee fields (read
   volatile: dyld's load-time write is invisible to the compiler, and
   constant-folding the field back to the interposed symbol would
   recurse). The interposed bodies stay static: the tuples carry them. */
typedef struct {
    const void *replacement;
    const void *replacee;
} tfs_interpose_t;

static void *tfs_dlopen_interposed(const char *path, int mode);
static char *tfs_dlerror_interposed(void);

__attribute__((used, section("__DATA,__interpose")))
static tfs_interpose_t tfs_interpose_dlopen = {
    (const void *)tfs_dlopen_interposed,
    (const void *)dlopen
};
__attribute__((used, section("__DATA,__interpose")))
static tfs_interpose_t tfs_interpose_dlerror = {
    (const void *)tfs_dlerror_interposed,
    (const void *)dlerror
};

static void *
tfs_dlopen_interposed(const char *path, int mode)
{
    const void *volatile *original = &tfs_interpose_dlopen.replacee;
    return tfs_dlopen_route(path, mode, (tfs_dlopen_fn)*original);
}

static char *
tfs_dlerror_interposed(void)
{
    const void *volatile *original = &tfs_interpose_dlerror.replacee;
    return tfs_dlerror_route((tfs_dlerror_fn)*original);
}
/* -- End of tebako loader interposition -- */
