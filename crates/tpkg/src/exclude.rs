//! Imaging-time path exclusion (`tebako press --exclude` / `tfs mkimage
//! --exclude`; the feedstock Tebakofile's `exclude:` list hands the same
//! patterns to the imager verbatim): development-time trees (tmp/, test
//! directories, scratch data) stay in the source tree without landing in
//! the built image. The exclusion list never lands in the payload
//! manifest — the image simply lacks the paths.
//!
//! The grammar (documented in the imaging spec's production section): a
//! pattern matches the payload-ROOT-RELATIVE path (slash-separated, no
//! leading `/`) — `*`, `?`, and `[...]` classes follow fnmatch flag-0
//! semantics (a `*` run spans `/`), `\` quotes the next character, and a
//! matched directory prunes its whole subtree. A trailing `/` on the
//! pattern is accepted and stripped (it names the directory). The
//! matcher is pure Rust and platform-independent by construction — the
//! deterministic-imaging rule (a rebuilt tree must emit byte-identical
//! bytes on every host) forbids the libc fnmatch whose classes and
//! escapes differ per platform.

use std::fmt;

/// A malformed exclusion pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludeError(pub String);

impl fmt::Display for ExcludeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExcludeError {}

/// One validated pattern (the parse-time checks ran; matching never
/// fails).
#[derive(Debug, Clone)]
struct Pattern {
    /// The normalized form: trimmed, trailing slashes stripped.
    text: String,
}

/// The parsed exclusion set. An empty set excludes nothing and lets the
/// imaging callers keep their unstaged fast path.
#[derive(Debug, Clone, Default)]
pub struct ExcludeSet {
    patterns: Vec<Pattern>,
}

impl ExcludeSet {
    /// Parse and validate the authored patterns (one `--exclude` repeat
    /// or one Tebakofile `exclude:` entry each). Malformed patterns are a
    /// named error, never a silent drop: empty after normalization, an
    /// absolute spelling (leading `/`), or a `.`/`..` component (the
    /// match root is the payload root — there is nothing above it).
    pub fn parse(patterns: &[String]) -> Result<ExcludeSet, ExcludeError> {
        let mut out = Vec::with_capacity(patterns.len());
        for raw in patterns {
            let trimmed = raw.trim();
            let stripped = trimmed.trim_end_matches('/');
            if stripped.is_empty() {
                return Err(ExcludeError(format!(
                    "invalid exclude pattern {raw:?} — empty (a pattern names a payload-root-relative path)"
                )));
            }
            if stripped.starts_with('/') {
                return Err(ExcludeError(format!(
                    "invalid exclude pattern {raw:?} — patterns are relative to the payload root (drop the leading '/')"
                )));
            }
            if stripped.split('/').any(|c| c == "." || c == "..") {
                return Err(ExcludeError(format!(
                    "invalid exclude pattern {raw:?} — '.' and '..' components are meaningless against the payload root"
                )));
            }
            out.push(Pattern {
                text: stripped.to_string(),
            });
        }
        Ok(ExcludeSet { patterns: out })
    }

    /// True when no patterns are authored (the callers' unstaged fast
    /// path stays byte-identical with the pre-exclusion behavior).
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Does `rel_path` (slash-separated, relative to the payload root, no
    /// leading slash) match any pattern? A matching directory's subtree
    /// is pruned by the caller's walk — the matcher answers for the path
    /// itself only.
    pub fn matches(&self, rel_path: &str) -> bool {
        self.patterns.iter().any(|p| glob_match(&p.text, rel_path))
    }
}

/// fnmatch(pattern, name, flags=0): `*` matches any run INCLUDING `/`,
/// `?` any single character, `[...]` character classes (ranges; a
/// leading `!` negates; a `]` first is a literal member), `\` quotes the
/// next character (a trailing `\` is a literal backslash), and an
/// unterminated `[` is a literal '['. The classic recursive matcher —
/// exclusion patterns are small.
fn glob_match(pattern: &str, name: &str) -> bool {
    /// One class character at `*i` (`\x` unescaped); None at the end.
    fn class_char(p: &[char], i: &mut usize) -> Option<char> {
        let c = *p.get(*i)?;
        *i += 1;
        if c == '\\' {
            let e = *p.get(*i)?;
            *i += 1;
            Some(e)
        } else {
            Some(c)
        }
    }

    /// The "[...]" at p[0] against `c`: (matched, consumed pattern
    /// length), or None when the class is unterminated.
    fn class_match(p: &[char], c: char) -> Option<(bool, usize)> {
        let mut i = 1;
        let negated = p.get(i) == Some(&'!');
        if negated {
            i += 1;
        }
        let mut matched = false;
        let mut first = true;
        loop {
            match p.get(i) {
                None => return None,
                Some(&']') if !first => return Some((matched != negated, i + 1)),
                _ => {}
            }
            first = false;
            let lo = class_char(p, &mut i)?;
            // A range: '-' followed by a character other than ']'.
            let hi = if p.get(i) == Some(&'-') && p.get(i + 1).is_some_and(|&h| h != ']') {
                i += 1;
                class_char(p, &mut i)?
            } else {
                lo
            };
            if lo <= c && c <= hi {
                matched = true;
            }
        }
    }

    fn mat(p: &[char], n: &[char]) -> bool {
        let (mut pi, mut ni) = (0, 0);
        while pi < p.len() {
            match p[pi] {
                // A star run: try every rest (a trailing star matches the
                // empty rest too).
                '*' => {
                    while p.get(pi + 1) == Some(&'*') {
                        pi += 1;
                    }
                    return (ni..=n.len()).any(|k| mat(&p[pi + 1..], &n[k..]));
                }
                '?' => {
                    if ni == n.len() {
                        return false;
                    }
                    pi += 1;
                    ni += 1;
                }
                '[' => {
                    if ni == n.len() {
                        return false;
                    }
                    match class_match(&p[pi..], n[ni]) {
                        Some((true, len)) => {
                            pi += len;
                            ni += 1;
                        }
                        Some((false, _)) => return false,
                        // Unterminated class: a literal '['.
                        None if n[ni] == '[' => {
                            pi += 1;
                            ni += 1;
                        }
                        None => return false,
                    }
                }
                // `\x` quotes x.
                '\\' if pi + 1 < p.len() => {
                    if n.get(ni) != Some(&p[pi + 1]) {
                        return false;
                    }
                    pi += 2;
                    ni += 1;
                }
                c => {
                    if n.get(ni) != Some(&c) {
                        return false;
                    }
                    pi += 1;
                    ni += 1;
                }
            }
        }
        ni == n.len()
    }

    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    mat(&p, &n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(patterns: &[&str]) -> ExcludeSet {
        ExcludeSet::parse(&patterns.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("the fixture patterns parse")
    }

    #[test]
    fn a_bare_name_excludes_the_root_entry_and_its_subtree() {
        let set = parsed(&["tmp"]);
        assert!(set.matches("tmp"));
        // The walk prunes at the directory, so subtree paths never reach
        // the matcher — but a direct query answers honestly anyway.
        assert!(!set.matches("tmp/inner"));
        assert!(!set.matches("vendor/tmp"));
    }

    #[test]
    fn a_trailing_slash_names_the_directory() {
        let set = parsed(&["tmp/"]);
        assert!(set.matches("tmp"));
        assert!(!set.matches("tmpfile"));
    }

    #[test]
    fn a_star_run_spans_slashes() {
        let set = parsed(&["tmp/*", "*.log"]);
        assert!(set.matches("tmp/a"));
        assert!(set.matches("tmp/a/b/c"));
        assert!(set.matches("debug.log"));
        assert!(set.matches("var/log/debug.log"));
        assert!(!set.matches("tmp"));
        assert!(!set.matches("debug.logs"));
    }

    #[test]
    fn classes_and_quotes_follow_fnmatch_flag_zero() {
        let set = parsed(&["test/unit-?.txt", "data/[0-9]", r"literal\*"]);
        assert!(set.matches("test/unit-7.txt"));
        assert!(!set.matches("test/unit-77.txt"));
        assert!(set.matches("data/5"));
        assert!(!set.matches("data/x"));
        assert!(set.matches("literal*"));
        assert!(!set.matches("literalX"));
        // a leading `!` negates the class
        let negated = parsed(&["data/[!0-9]"]);
        assert!(negated.matches("data/x"));
        assert!(!negated.matches("data/5"));
    }

    #[test]
    fn malformed_patterns_are_named_errors() {
        for bad in ["", "/", "/tmp", "..", "a/../b", "./tmp", "  "] {
            let raw = bad.to_string();
            let e = ExcludeSet::parse(&[raw]).unwrap_err();
            assert!(!e.0.is_empty(), "{bad:?} names its reason");
        }
        // a// strips to a — the trailing-slash rule, not an error
        assert!(ExcludeSet::parse(&["a/".to_string(), "a//".to_string()]).is_ok());
    }

    #[test]
    fn the_empty_set_matches_nothing() {
        let set = ExcludeSet::default();
        assert!(set.is_empty());
        assert!(!set.matches("anything"));
    }
}
