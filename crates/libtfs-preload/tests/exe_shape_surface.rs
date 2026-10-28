//! The exe-shape surface parity: `exe-shape-symbols.txt` is the link
//! unit's keep list (the scoper must not rename the family, or it
//! preempts nothing), and sys/mod.rs is the definition. This test proves
//! the two agree so the list can never drift from the code.

use std::collections::BTreeSet;

/// The names the exe-shape build exports on Linux: every `no_mangle`
/// (or linux-gated `no_mangle`) `extern "C"` fn in sys/mod.rs, minus the
/// exec class (gated OUT of the exe shape by its own cfg).
fn defined_surface() -> BTreeSet<String> {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/sys/mod.rs"))
        .expect("read sys/mod.rs");
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
        let no_mangle = window.iter().any(|l| l.contains("no_mangle"));
        let exe_shape_excluded = window
            .iter()
            .any(|l| l.contains("no_mangle") && l.contains("not(feature = \"exe-shape\")"));
        if no_mangle && !exe_shape_excluded {
            out.insert(name.to_string());
        }
    }
    out
}

fn listed_surface() -> BTreeSet<String> {
    let txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/exe-shape-symbols.txt"
    ))
    .expect("read exe-shape-symbols.txt");
    txt.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_keep_list_matches_the_definition() {
    let defined = defined_surface();
    let listed = listed_surface();
    let missing_from_list: Vec<_> = defined.difference(&listed).collect();
    let missing_from_code: Vec<_> = listed.difference(&defined).collect();
    assert!(
        missing_from_list.is_empty() && missing_from_code.is_empty(),
        "exe-shape surface drift — exported but unlisted: {missing_from_list:?}; listed but not exported: {missing_from_code:?}"
    );
}

#[test]
fn the_exec_class_is_not_listed() {
    let listed = listed_surface();
    for name in ["dlopen", "execve", "posix_spawn", "posix_spawnp"] {
        assert!(
            !listed.contains(name),
            "{name} is the exec class — the exe shape never exports it"
        );
    }
}
