//! The symbolic path atoms of the needs/mounts grammar (spec 23 §2) — the
//! ONE home of the atom set and its expansion rule (spec 00 invariant 10:
//! the grammar is a contract value; consumers FLOW it from here).
//!
//! Manifests and composition documents write host paths as `$HOME/…`,
//! `$TMPDIR`, `$CWD`, `$TEBAKO_HOME` (windows spellings `%USERPROFILE%`,
//! `%TEMP%`); atoms resolve at bind time, per invocation, per user —
//! never baked. The VALUES come from the caller's environment (the lookup
//! closure); this module owns the GRAMMAR: which atoms exist, where in a
//! path they may appear (a leading prefix), and the named errors.

/// The atom names the grammar speaks (spec 23 §2), sigil-agnostic: the
/// lookup is tried with the bare name (`HOME`, `TEMP`, …). `$HOME` ≡
/// `%USERPROFILE%` and `$TMPDIR` ≡ `%TEMP%` are the same axis spelled per
/// platform family; the caller binds whichever its environment carries.
pub const SYMBOLIC_ATOMS: &[&str] = &[
    "HOME",
    "USERPROFILE",
    "TMPDIR",
    "TEMP",
    "CWD",
    "TEBAKO_HOME",
];

/// Expand a leading symbolic atom using `lookup`. Atoms are path
/// prefixes: bare `$HOME` or `$HOME/…`; `%USERPROFILE%`,
/// `%USERPROFILE%/…`, `%USERPROFILE%\…`. A path without a leading atom is
/// returned unchanged; a lone `%` without its closing sigil is not an
/// atom (literal passthrough).
///
/// Errors are named and distinguish the two failure classes: an atom the
/// grammar does not speak (a typo the author fixes) vs an atom the
/// grammar speaks that does not resolve in this environment (spec 23 §2:
/// "an atom that does not resolve fails the bind only when the need is
/// otherwise in force" — resolution is the caller's moment, the naming is
/// the grammar's).
pub fn expand_symbolic_atoms(
    path: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    // `$ATOM`, bare or `/`-tailed.
    if let Some(rest) = path.strip_prefix('$') {
        let (atom, tail) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        return bind_atom(atom, tail, path, lookup);
    }
    // `%ATOM%`, bare or `/`- / `\`-tailed (the windows spellings).
    if let Some(rest) = path.strip_prefix('%') {
        if let Some(i) = rest.find('%') {
            let atom = &rest[..i];
            let tail = &rest[i + 1..];
            return bind_atom(atom, tail, path, lookup);
        }
    }
    Ok(path.to_string())
}

fn bind_atom(
    atom: &str,
    tail: &str,
    path: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    if !SYMBOLIC_ATOMS.contains(&atom) {
        return Err(format!(
            "unknown atom in {path:?} (the needs/mounts grammar speaks {})",
            SYMBOLIC_ATOMS
                .iter()
                .map(|a| format!("${a}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    match lookup(atom) {
        Some(base) if !base.is_empty() => Ok(format!("{base}{tail}")),
        _ => Err(format!(
            "atom ${atom} in {path:?} does not resolve in this environment"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(atom: &str) -> Option<String> {
        match atom {
            "HOME" => Some("/Users/u".to_string()),
            "TMPDIR" => Some("/tmp/x".to_string()),
            "CWD" => Some("/work/dir".to_string()),
            _ => None,
        }
    }

    #[test]
    fn expands_the_dollar_spellings() {
        let e = |p: &str| expand_symbolic_atoms(p, &lookup);
        assert_eq!(e("$HOME").unwrap(), "/Users/u");
        assert_eq!(e("$HOME/.config/app").unwrap(), "/Users/u/.config/app");
        assert_eq!(e("$TMPDIR/scratch").unwrap(), "/tmp/x/scratch");
        assert_eq!(e("$CWD").unwrap(), "/work/dir");
    }

    #[test]
    fn expands_the_percent_spellings() {
        let e = |p: &str| expand_symbolic_atoms(p, &lookup);
        assert_eq!(e("%TMPDIR%").unwrap(), "/tmp/x");
        assert_eq!(e("%TMPDIR%/scratch").unwrap(), "/tmp/x/scratch");
        assert_eq!(e("%TMPDIR%\\scratch").unwrap(), "/tmp/x\\scratch");
    }

    #[test]
    fn paths_without_atoms_pass_through() {
        let e = |p: &str| expand_symbolic_atoms(p, &lookup);
        assert_eq!(e("/opt/vendor").unwrap(), "/opt/vendor");
        assert_eq!(e("relative/dir").unwrap(), "relative/dir");
        // A lone `%` without the closing sigil is not an atom.
        assert_eq!(e("%per_cent").unwrap(), "%per_cent");
        // An atom mid-path is not a prefix — never rewritten.
        assert_eq!(e("/opt/$HOME").unwrap(), "/opt/$HOME");
    }

    #[test]
    fn unknown_and_unresolvable_atoms_are_named_errors() {
        let unknown = expand_symbolic_atoms("$QUX/x", &lookup).unwrap_err();
        assert!(unknown.contains("$QUX"), "{unknown}");
        assert!(unknown.contains("unknown atom"), "{unknown}");
        assert!(unknown.contains("$TEBAKO_HOME"), "{unknown}");

        let unresolved = expand_symbolic_atoms("$TEBAKO_HOME/x", &lookup).unwrap_err();
        assert!(unresolved.contains("$TEBAKO_HOME"), "{unresolved}");
        assert!(unresolved.contains("does not resolve"), "{unresolved}");

        let empty = expand_symbolic_atoms("$/x", &lookup).unwrap_err();
        assert!(empty.contains("unknown atom"), "{empty}");
    }
}
