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
//! Hand-rolled (no semver crate): the dispatcher keeps bootstrap size
//! discipline. Versions are dot-separated components; numeric components
//! compare numerically, anything else lexicographically, missing
//! components are zero.

use std::cmp::Ordering;

use crate::{fail, ShimError, EX_TEBAKO_MANIFEST};

fn components(v: &str) -> Vec<&str> {
    v.split('.').collect()
}

fn compare_component(a: &str, b: &str) -> Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// Compare two dotted versions (`1.2` == `1.2.0`, `1.10` > `1.9`).
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

/// Constraint versions must be plain dotted numerics (they are authored
/// in manifests / registries; anything fancier is a named error, never a
/// silent fallback).
fn check_constraint_version(v: &str) -> Result<(), ShimError> {
    if v.is_empty()
        || !v
            .split('.')
            .all(|c| !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit()))
    {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "malformed version \"{v}\" in a requirement constraint — expected dotted numerics (e.g. \">= 3.3, < 5.0\" or \"~> 3.3.0\")"
            ),
        );
    }
    Ok(())
}

pub fn parse_constraint(source: &str) -> Result<Constraint, ShimError> {
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
        let version = rest.trim().to_string();
        check_constraint_version(&version)?;
        clauses.push(Clause { op, version });
    }
    if clauses.is_empty() {
        return fail(
            EX_TEBAKO_MANIFEST,
            "empty requirement constraint — expected e.g. \">= 3.3, < 5.0\"",
        );
    }
    Ok(Constraint {
        source: source.to_string(),
        clauses,
    })
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
        assert_eq!(compare("3.3.5", "3.4.0"), Ordering::Less);
        assert_eq!(compare("4.0.6", "4.0.6"), Ordering::Equal);
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
}
