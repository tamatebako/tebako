//! The macOS interpose-tuple self-reference gate (tebako#1063).
//!
//! Each tuple in sys/macos.rs is `(replacement, replacee)` with the
//! replacee spelled `libc::<name>`. That spelling resolves at LINK time:
//! if the dylib itself exports `_<name>` (an unconditional `#[no_mangle]`
//! on the shim), the linker binds the replacee to OUR OWN export and the
//! tuple becomes `(our_shim, our_shim)` — dyld's interpose then no-ops
//! and every foreign caller's libSystem-bound `<name>` bypasses the shim
//! silently. chdir/fchdir/getcwd shipped exactly that defect.
//!
//! The rule this test proves from the source: every tuple whose
//! replacement is `super::<name>` (a shim defined in sys/mod.rs) defines
//! `<name>` with NO unconditional `#[no_mangle]` — the Linux LD_PRELOAD
//! export (`#[cfg_attr(target_os = "linux", no_mangle)]`) is the only
//! permitted form, since Linux has no tuples. Replacements that are not
//! `super::<name>` (the arm64 variadic trampolines) export names that
//! never equal the victim's, so they cannot self-reference by
//! construction.
//!
//! Text-based like exe_shape_surface.rs: the proof runs on every
//! platform, no dylib build needed.

use std::collections::BTreeSet;

fn read(rel: &str) -> String {
    std::fs::read_to_string(format!("{}/{rel}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// The `super::<name>` replacements named by macos.rs's tuple invocations.
fn tuple_shims() -> BTreeSet<String> {
    let src = read("src/sys/macos.rs");
    // Join continuation lines so a multi-line invocation parses as one
    // statement, then walk `interpose!(` / `interpose_tuple!(` blocks.
    let joined = src
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = BTreeSet::new();
    let mut rest = joined.as_str();
    loop {
        let (at, skip) = match (rest.find("interpose_tuple!("), rest.find("interpose!(")) {
            (Some(a), Some(b)) if a < b => (a, "interpose_tuple!(".len()),
            (Some(_), Some(b)) => (b, "interpose!(".len()),
            (Some(a), None) => (a, "interpose_tuple!(".len()),
            (None, Some(b)) => (b, "interpose!(".len()),
            (None, None) => break,
        };
        rest = &rest[at + skip..];
        // The replacement argument is the 2nd (tuple) or 3rd (full) comma
        // field at the invocation's top nesting level.
        let args_end = rest.find(");").unwrap_or(rest.len());
        let args = &rest[..args_end];
        let fields: Vec<&str> = args.split(',').map(str::trim).collect();
        // interpose_tuple!(NAME, replacement, replacee):
        //   fields = [NAME, replacement, replacee]
        // interpose!(NAME, real_fn, replacement, replacee, ty):
        //   fields = [NAME, real_fn, replacement, replacee, ty...]
        let replacement = if skip == "interpose_tuple!(".len() {
            fields.get(1).copied()
        } else {
            fields.get(2).copied()
        };
        if let Some(r) = replacement {
            if let Some(name) = r.strip_prefix("super::") {
                out.insert(name.to_string());
            }
        }
        rest = &rest[args_end..];
    }
    out
}

/// The fns in sys/mod.rs carrying an UNCONDITIONAL `#[no_mangle]` (the
/// macOS-visible name export). A linux-gated cfg_attr is NOT collected.
fn unconditionally_exported() -> BTreeSet<String> {
    let src = read("src/sys/mod.rs");
    let lines: Vec<&str> = src.lines().collect();
    let mut out = BTreeSet::new();
    let mut window: Vec<&str> = Vec::new();
    for line in lines {
        window.push(line);
        if window.len() > 6 {
            window.remove(0);
        }
        let trimmed = line.trim();
        if !(trimmed.starts_with("pub")
            && trimmed.contains("fn ")
            && trimmed.contains("extern \"C\""))
        {
            continue;
        }
        let name = trimmed
            .split("fn ")
            .nth(1)
            .and_then(|rest| rest.split('(').next())
            .map(str::trim)
            .unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let bare = window.iter().any(|l| l.trim() == "#[no_mangle]");
        if bare {
            out.insert(name.to_string());
        }
    }
    out
}

#[test]
fn no_tuple_shim_is_name_exported_on_macos() {
    let shims = tuple_shims();
    // The family is large enough that an empty parse is a test bug, not a
    // pass: the 45-tuple surface names dozens of super:: shims.
    assert!(
        shims.len() >= 40,
        "tuple parse found only {} super:: shims — the parser, not the code, is broken",
        shims.len()
    );
    let exported = unconditionally_exported();
    let bad: Vec<_> = shims.intersection(&exported).collect();
    assert!(
        bad.is_empty(),
        "tebako#1063: these tuple-backed shims carry an unconditional \
         #[no_mangle], so their macOS interpose tuple self-resolves and the \
         interpose is dead for foreign callers: {bad:?}. Gate the export to \
         Linux (#[cfg_attr(target_os = \"linux\", no_mangle)]) like the rest \
         of the family."
    );
}
