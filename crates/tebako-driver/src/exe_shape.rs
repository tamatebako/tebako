//! The EXE shape's arm (spec 22 §3.6): hand the boot to the file-IO family
//! when the binary carries one.
//!
//! The problem the arm answers. The runtime exe defines class L for the whole
//! process, so a loader call routes through the mounts. It defines nothing of
//! the file-IO family, and the interpreter statically links a C library whose
//! own `fopen64`/`openat64` reads (OpenSSL's `BIO_new_file`, `RAND_load_file`,
//! the config) therefore reach the host unchecked under a `deny` jail — the
//! hole tebako#439 fixed for the injected shim, one shape over.
//!
//! The delivery. `crates/libtfs-preload` builds a `staticlib` with the
//! `exe-shape` feature, which compiles its library constructor out and
//! exposes `tebako_preload_init()` instead. A runtime that links that
//! archive (the factory's link unit change) gets the whole family in its own
//! image: the routing, the `dup`/`fcntl`/`realpath` semantics, the jail, the
//! fork guards.
//!
//! Why the symbol is looked up rather than linked. A hard reference would
//! couple every tebako release's link unit to a factory that links the archive
//! in the same step — two release chains in lockstep, or neither builds. The
//! lookup makes the arm OPT-IN BY LINK: a runtime without the archive has no
//! symbol, [`arm`] returns, and its boot is byte-identical to before the
//! change. A runtime with the archive arms the family at the one moment it can
//! work — after the mounts and the jail exist, before the interpreter makes
//! its first call.
//!
//! The lookup's one requirement. `dlsym(RTLD_DEFAULT)` searches the process's
//! DYNAMIC symbol table, so the factory's link line must add
//! `-Wl,--export-dynamic` (`--rdynamic`) beside the `--whole-archive` the
//! archive needs. Without it the family links but stays invisible, which is
//! why a missing family under a bound jail is NAMED on stderr rather than
//! passed over: the degraded coverage is stated, never inferred.
//!
//! What the family reads. The driver has already published `TEBAKO_TFS_MOUNTS`
//! (spec 17 §2.1, `crate::injection`) and carries `TEBAKO_JAIL` when a policy
//! was handed, so the arm passes nothing: the family's own entry parses
//! exactly the environment a child would have parsed. The family owns its
//! engine instance, so its mount pass builds its own view of the same images
//! — the cost §3.6 records for the link-the-crate decision.
//!
//! macOS is not this delivery. dyld honours `__interpose` tuples only from a
//! dylib image, so the exe shape there is the driver's micro interpose-dylib
//! (§2's phase-1 delivery) gaining the family's tuples; the archive's
//! definitions are not preemptive in a Mach-O executable.

#![allow(unsafe_code)]
// The crate root denies `unsafe_code`; `dlsym` is a libc call and needs the
// one exemption, exactly as `ffi.rs` takes it for the C ABI's own seam.

#[cfg(unix)]
use std::ffi::c_void;
use std::ffi::{c_char, c_int};

/// The entry point the `exe-shape` archive exports.
const INIT_SYMBOL: &[u8] = b"tebako_preload_init\0";

/// `EX_CONFIG` — the family's verdict when the environment is malformed or an
/// image will not mount (spec 18 §7's named configuration error).
const EX_CONFIG: c_int = 78;

/// The family's entry: no arguments, a verdict back (0 or `EX_CONFIG`).
type InitFn = unsafe extern "C" fn() -> c_int;

/// Resolve the family's entry from the process's own symbol table.
///
/// `RTLD_DEFAULT` searches the whole loaded image, which for a linked-in
/// family is the executable itself. No `dlopen` of a file, no path, no
/// environment: the symbol is either in this binary's dynamic table or it is
/// not.
#[cfg(unix)]
fn resolve() -> Option<InitFn> {
    // SAFETY: `RTLD_DEFAULT` is the documented pseudo-handle for the global
    // scope; the name is a NUL-terminated literal with static lifetime. A
    // null return means "not present", which is the shipped case for every
    // runtime that does not link the archive.
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, INIT_SYMBOL.as_ptr().cast::<c_char>()) };
    if sym.is_null() {
        return None;
    }
    // SAFETY: the symbol, when present, is `tebako_preload_init`'s
    // `extern "C" fn() -> c_int` — the archive's only exported entry point
    // and the one this module looks up by name.
    Some(unsafe { std::mem::transmute::<*mut c_void, InitFn>(sym) })
}

/// `true` when `TEBAKO_JAIL` carries a policy for this boot.
///
/// The diagnostic below is scoped to the jailed case because an unjailed run
/// loses nothing when the family is absent: there is no policy for the
/// in-exe reads to evade.
#[cfg(unix)]
fn jailed() -> bool {
    std::env::var("TEBAKO_JAIL")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

/// Arm the exe shape's family when this binary carries it.
///
/// Returns `true` when a family was found and initialized successfully, and
/// `false` in every other case — no archive (the shipped default today), or a
/// named configuration error, which is printed here and leaves the boot to
/// continue on the host filesystem as it would have before.
///
/// A missing family under a bound jail prints the coverage gap on stderr: the
/// jail still covers the loader surface, and the in-exe C library's own reads
/// stay on the host. That line is the operator's signal that this runtime was
/// built without the archive (or without `--export-dynamic`).
#[cfg(unix)]
pub(crate) fn arm() -> bool {
    let Some(init) = resolve() else {
        if jailed() {
            eprintln!(
                "tebako-driver: TEBAKO_JAIL is bound but this runtime defines no file-IO \
                 family (spec 22 §3.6) — the jail covers loader calls, not the in-exe C \
                 library's own file reads. A runtime linked against the exe-shape archive \
                 closes this."
            );
        }
        return false;
    };
    // SAFETY: `init` was resolved from this process's symbol table under the
    // name the archive exports; the call takes no arguments and returns the
    // family's verdict. It touches the mount table and the host policy, which
    // is exactly what the driver has finished establishing at the call site.
    let rc = unsafe { init() };
    if rc == 0 {
        return true;
    }
    let detail = if rc == EX_CONFIG {
        "the mount configuration is malformed (the named error above says which variable and token)"
    } else {
        "unexpected verdict"
    };
    eprintln!(
        "tebako-driver: the exe-shape file-IO family did not arm (exit {rc}). {detail} \
         — host reads are NOT covered by the mounts for this run."
    );
    false
}

/// Windows has no family to arm: its file-IO coverage is Class L's alias
/// table and the materialize tier (spec 22 §2.1), and the archive's routings
/// are `cfg(unix)` in the first place. The call site stays unconditional so
/// both boot shapes read the same.
#[cfg(not(unix))]
pub(crate) fn arm() -> bool {
    false
}

#[cfg(all(test, unix))]
mod tests {
    /// The shipped case: nothing links the archive, so the lookup misses and
    /// the arm is a no-op — every existing boot stays byte-identical.
    #[test]
    fn an_absent_family_is_a_silent_no_op() {
        assert!(
            !super::arm(),
            "the driver's test binary links no exe-shape archive"
        );
    }
}
