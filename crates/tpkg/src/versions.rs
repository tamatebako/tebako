//! Dotted-version comparison and requirement-constraint matching
//! (spec 05 §5 compatibility model).
//!
//! Two constraint forms, one matcher:
//! - **range** (`>= 3.3, < 5.0`) — pure-language payloads; any newer
//!   runtime within the range works;
//! - **abi-line** (`~> 3.3.0`) — native-extension payloads lock to the
//!   ABI line they were built against (pessimistic / rubygems semantics:
//!   `~> 3.3.0` means `>= 3.3.0, < 3.4`; `~> 3.3` means `>= 3.3, < 4`).
//!
//! Hand-rolled (no semver crate): the loaders keep bootstrap size
//! discipline. Versions are dot-separated components; missing components
//! are zero. Within one component (spec 05 §5's ordering rule): a leading
//! numeric prefix compares numerically (`1.10 > 1.9`); at an equal prefix
//! a PLAIN component ranks ABOVE a suffixed one (`3.13.15 > 3.13.15-jit`
//! — semver's release > prerelease rule, so a factory's build variant
//! never silently outranks its plain twin in a newest-satisfying pick);
//! two suffixes order lexicographically; a component without a numeric
//! prefix falls back to whole-component string order. Constraint MATCHING
//! is unaffected by the ordering rule: a variant version still satisfies
//! an open constraint (a platform where ONLY the variant exists still
//! resolves) — it just never wins a max pick against the plain twin. A
//! variant is SELECTED through the pin surface (config `version:`, the
//! registry default), which names versions exactly; the constraint
//! grammar itself (spec 03) admits only plain dot-decimal clauses.
//!
//! Constraint GRAMMAR is not re-implemented here: [`Constraint`] (the
//! manifest model) validates at parse (spec 03 — the unified model), and
//! [`from_validated`] only clause-splits that validated string into the
//! evaluable form. [`parse_constraint`] (for the few raw-string callers)
//! is validate-then-split.
//!
//! **Placement (spec 00 §10 SSOT):** the spec 05 §5 evaluation semantics
//! have exactly ONE owner — this module. tebako-shim (dispatch),
//! tebako-cli (install/compose/check) and tebako-driver (spec 30 §2's
//! spawn-time runtime-edge resolution) all consume it from here; the
//! shim's pre-move private copy is retired (spec 30).

use std::cmp::Ordering;

use crate::ManifestError;

fn components(v: &str) -> Vec<&str> {
    v.split('.').collect()
}

/// Split a component into its leading numeric prefix and the optional
/// non-numeric suffix (`15-jit` → `(15, Some("-jit"))`, `15` →
/// `(15, None)`). A component with no leading digit (or a prefix too big
/// for u64) has no numeric prefix and falls back to string order.
fn numeric_prefix(c: &str) -> Option<(u64, Option<&str>)> {
    let digits = c.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let (num, rest) = c.split_at(digits);
    let n = num.parse::<u64>().ok()?;
    Some((n, if rest.is_empty() { None } else { Some(rest) }))
}

fn compare_component(a: &str, b: &str) -> Ordering {
    match (numeric_prefix(a), numeric_prefix(b)) {
        (Some((x, sx)), Some((y, sy))) => x.cmp(&y).then_with(|| match (sx, sy) {
            (None, None) => Ordering::Equal,
            // The plain-wins rule (spec 05 §5): release > prerelease.
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(s), Some(t)) => s.cmp(t),
        }),
        _ => a.cmp(b),
    }
}

/// Compare two dotted versions (`1.2` == `1.2.0`, `1.10` > `1.9`,
/// `3.13.15` > `3.13.15-jit` — the plain-wins rule, spec 05 §5).
pub fn compare(a: &str, b: &str) -> Ordering {
    let (ca, cb) = (components(a), components(b));
    for i in 0..ca.len().max(cb.len()) {
        let (x, y) = (
            ca.get(i).copied().unwrap_or("0"),
            cb.get(i).copied().unwrap_or("0"),
        );
        let ord = compare_component(x, y);
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Pessimistic,
}

#[derive(Debug, Clone)]
struct Clause {
    op: Op,
    version: String,
}

/// A parsed requirement constraint: a comma-separated conjunction of
/// clauses (`>= 3.3, < 5.0`), a single abi-line clause (`~> 3.3.0`), or a
/// bare version (exact match).
#[derive(Debug, Clone)]
pub struct Constraint {
    source: String,
    clauses: Vec<Clause>,
}

/// Validate a raw constraint string and build the evaluable form. The
/// grammar is the manifest model's (the unified grammar); anything
/// fancier than the spec 03 grammar is a named error, never a silent
/// fallback.
pub fn parse_constraint(source: &str) -> Result<Constraint, ManifestError> {
    let validated = crate::Constraint::new(source).map_err(|e| {
        ManifestError::InvalidOwned(
            format!(
                "malformed requirement constraint \"{source}\" ({e}) — expected e.g. \">= 3.3, < 5.0\" or \"~> 3.3.0\""
            ),
        )
    })?;
    Ok(from_validated(&validated))
}

/// Clause-split an already-validated constraint into the evaluable form.
/// The grammar was checked when the [`crate::Constraint`] was built (at
/// manifest parse), so this never fails and never re-validates.
pub fn from_validated(validated: &crate::Constraint) -> Constraint {
    let source = validated.as_str();
    let mut clauses = Vec::new();
    for raw in source.split(',') {
        let part = raw.trim();
        let (op, rest) = if let Some(r) = part.strip_prefix("~>") {
            (Op::Pessimistic, r)
        } else if let Some(r) = part.strip_prefix(">=") {
            (Op::Ge, r)
        } else if let Some(r) = part.strip_prefix("<=") {
            (Op::Le, r)
        } else if let Some(r) = part.strip_prefix("!=") {
            (Op::Ne, r)
        } else if let Some(r) = part.strip_prefix('=') {
            (Op::Eq, r.strip_prefix('=').unwrap_or(r))
        } else if let Some(r) = part.strip_prefix('>') {
            (Op::Gt, r)
        } else if let Some(r) = part.strip_prefix('<') {
            (Op::Lt, r)
        } else {
            (Op::Eq, part)
        };
        clauses.push(Clause {
            op,
            version: rest.trim().to_string(),
        });
    }
    Constraint {
        source: source.to_string(),
        clauses,
    }
}

/// The pessimistic upper bound: drop the last component, increment the
/// new last (`~> 3.3.0` → `< 3.4`; `~> 3.3` → `< 4`; `~> 3` → `< 4`).
fn pessimistic_upper(version: &str) -> String {
    let mut parts: Vec<u64> = version
        .split('.')
        .map(|c| c.parse::<u64>().unwrap_or(0))
        .collect();
    if parts.len() > 1 {
        parts.pop();
    }
    let last = parts.len() - 1;
    parts[last] += 1;
    parts
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

impl Constraint {
    pub fn matches(&self, version: &str) -> bool {
        self.clauses.iter().all(|clause| {
            let ord = compare(version, &clause.version);
            match clause.op {
                Op::Eq => ord == Ordering::Equal,
                Op::Ne => ord != Ordering::Equal,
                Op::Gt => ord == Ordering::Greater,
                Op::Ge => ord != Ordering::Less,
                Op::Lt => ord == Ordering::Less,
                Op::Le => ord != Ordering::Greater,
                Op::Pessimistic => {
                    ord != Ordering::Less
                        && compare(version, &pessimistic_upper(&clause.version)) == Ordering::Less
                }
            }
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// The newest version string from an iterator, by [`compare`].
pub fn newest<'a, I>(versions: I) -> Option<String>
where
    I: IntoIterator<Item = &'a String>,
{
    versions.into_iter().max_by(|a, b| compare(a, b)).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_dotted() {
        assert_eq!(compare("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare("1.10", "1.9"), Ordering::Greater);
        assert!(compare("3.3.5", "3.4.0") == Ordering::Less);
        assert_eq!(compare("4.0.6", "4.0.6"), Ordering::Equal);
    }

    #[test]
    fn plain_wins_over_a_variant_suffix() {
        // spec 05 §5: at an equal numeric prefix, plain ranks ABOVE
        // suffixed (semver's release > prerelease rule) — a factory's
        // build variant never outranks its plain twin in a max pick.
        assert_eq!(compare("3.13.15", "3.13.15-jit"), Ordering::Greater);
        assert_eq!(compare("3.13.15-jit", "3.13.15"), Ordering::Less);
        assert_eq!(compare("3.13.15-jit", "3.13.15-jit"), Ordering::Equal);
        // two suffixes order lexicographically
        assert_eq!(compare("3.13.15-a", "3.13.15-jit"), Ordering::Less);
        // numeric dominance is unchanged across the suffix boundary
        assert_eq!(compare("3.13.16-jit", "3.13.15"), Ordering::Greater);
        assert_eq!(compare("3.13.15-jit", "3.13.16"), Ordering::Less);
        assert_eq!(compare("1.10", "1.9"), Ordering::Greater);
        // a component with no numeric prefix keeps whole-string order
        assert_eq!(compare("1.2.rc1", "1.2.rc2"), Ordering::Less);
        // and the max pick over twins lands on the plain
        let vs = vec![
            "3.13.15-jit".to_string(),
            "3.13.15".to_string(),
            "3.13.9".to_string(),
        ];
        assert_eq!(newest(&vs).as_deref(), Some("3.13.15"));
    }

    #[test]
    fn variant_matching_is_unchanged_only_the_pick_changes() {
        // An open constraint still MATCHES the variant (a platform where
        // only the variant exists still resolves)…
        let c = parse_constraint("~> 3.13.0").unwrap();
        assert!(c.matches("3.13.15"));
        assert!(c.matches("3.13.15-jit"));
        assert!(!c.matches("3.14.0"));
        // …but the max pick among satisfiers is the plain twin.
        let vs = vec!["3.13.15-jit".to_string(), "3.13.15".to_string()];
        let pick = vs
            .iter()
            .filter(|v| c.matches(v))
            .max_by(|a, b| compare(a, b));
        assert_eq!(pick.map(String::as_str), Some("3.13.15"));
        // The constraint grammar (spec 03) admits only plain dot-decimal
        // clauses — a suffixed pin is a named parse error there; variant
        // selection rides the pin surface (config `version:`, registry
        // default), which names versions exactly and never compares.
        assert!(parse_constraint("= 3.13.15-jit").is_err());
    }

    #[test]
    fn range_form() {
        let c = parse_constraint(">= 3.3, < 5.0").unwrap();
        assert!(c.matches("3.3.0"));
        assert!(c.matches("4.0.6"));
        assert!(!c.matches("3.2.9"));
        assert!(!c.matches("5.0.0"));
    }

    #[test]
    fn abi_line_form() {
        let c = parse_constraint("~> 3.3.0").unwrap();
        assert!(c.matches("3.3.0"));
        assert!(c.matches("3.3.9"));
        assert!(!c.matches("3.4.0"));
        assert!(!c.matches("3.2.9"));

        let c = parse_constraint("~> 3.3").unwrap();
        assert!(c.matches("3.4.2"));
        assert!(!c.matches("4.0.0"));
    }

    #[test]
    fn exact_and_negated() {
        assert!(parse_constraint("3.3.5").unwrap().matches("3.3.5"));
        assert!(!parse_constraint("3.3.5").unwrap().matches("3.3.6"));
        assert!(parse_constraint("= 3.3.5").unwrap().matches("3.3.5"));
        assert!(parse_constraint("!= 3.3.5, >= 3.3")
            .unwrap()
            .matches("3.3.6"));
        assert!(!parse_constraint("!= 3.3.5, >= 3.3")
            .unwrap()
            .matches("3.3.5"));
    }

    #[test]
    fn malformed_is_a_named_error() {
        assert!(parse_constraint(">= ").is_err());
        assert!(parse_constraint("~> 3.x").is_err());
        assert!(parse_constraint("").is_err());
    }

    #[test]
    fn from_validated_clause_splits_without_revalidating() {
        let validated = crate::Constraint::new(">= 3.3, < 5.0").unwrap();
        let c = from_validated(&validated);
        assert_eq!(c.source(), ">= 3.3, < 5.0");
        assert!(c.matches("4.0.6"));
        assert!(!c.matches("5.0.0"));
    }
}
