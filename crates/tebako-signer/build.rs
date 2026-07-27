// bzip2 1.0.8 never DEFINES bz_internal_error (see bz_internal_shim.c):
// dynamic libbz2 tolerates the dangling AssertH reference, static links
// fail full symbol resolution (rust-lld on linux-gnu; tebako-pkg /
// tebako-cli / tfs-cli). Compile the one-function shim into every final
// binary that links the static libbz2 (via bzip2-sys, declared in this
// crate). Harmless where the system dylib is used instead (macOS): a
// static definition shadows, never conflicts.

fn main() {
    cc::Build::new()
        .file("src/bz_internal_shim.c")
        .compile("bz-internal-shim");
}
