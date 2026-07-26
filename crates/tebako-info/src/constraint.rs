//! Evaluation of the spec-03 version-constraint grammar against concrete
//! versions (the derived runtime-compatibility fact needs it; the tpkg
//! model is deliberately parse-only).
//!
//! Grammar (already guaranteed by [`tpkg::Constraint`]):
//!
//! ```text
//! constraint := clause ("," clause)*
//! clause     := op? version
//! op         := ">=" | "<=" | "~>" | ">" | "<" | "!=" | "="
//! version    := num ("." num){0,3}
//! ```
//!
//! `~>` is the ruby pessimistic operator: `~> a.b` means `>= a.b, < a+1`
//! and `~> a.b.c` means `>= a.b.c, < a.(b+1)` (drop the last component,
//! bump the new last one).

use tpkg::Constraint;

/// One parsed clause (operator + version components).
struct Clause {
    op: Op,
    version: Vec<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Ge,
    Le,
    Tilde,
    Gt,
    Lt,
    Ne,
    Eq,
}

fn parse_version(s: &str) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    for part in s.split('.') {
        let n: u64 = part
            .parse()
            .map_err(|_| format!("version component {part:?} is not a decimal number"))?;
        out.push(n);
    }
    Ok(out)
}

fn parse_clause(s: &str) -> Result<Clause, String> {
    const OPS: [(&str, Op); 7] = [
        (">=", Op::Ge),
        ("<=", Op::Le),
        ("~>", Op::Tilde),
        (">", Op::Gt),
        ("<", Op::Lt),
        ("!=", Op::Ne),
        ("=", Op::Eq),
    ];
    let s = s.trim();
    let (op, rest) = OPS
        .iter()
        .find_map(|(text, op)| s.strip_prefix(text).map(|r| (*op, r)))
        .unwrap_or((Op::Eq, s));
    Ok(Clause {
        op,
        version: parse_version(rest.trim())?,
    })
}

/// Component-wise compare; missing components read as 0
/// (`3.3` == `3.3.0.0`).
fn compare(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}

/// The pessimistic upper bound: drop the last component, bump the new
/// last (`~> 3.3.0` → `< 3.4`, `~> 4.0` → `< 5`).
fn tilde_upper(version: &[u64]) -> Vec<u64> {
    let mut upper = version.to_vec();
    upper.pop();
    match upper.last_mut() {
        Some(last) => *last += 1,
        None => upper.push(u64::MAX), // "~> 3" — everything above 3
    }
    upper
}

/// Does `version` satisfy the constraint?
pub fn satisfies(constraint: &Constraint, version: &str) -> Result<bool, String> {
    let version = parse_version(version.trim())?;
    for raw in constraint.as_str().split(',') {
        let clause = parse_clause(raw)?;
        let ok = match clause.op {
            Op::Ge => compare(&version, &clause.version) != std::cmp::Ordering::Less,
            Op::Le => compare(&version, &clause.version) != std::cmp::Ordering::Greater,
            Op::Gt => compare(&version, &clause.version) == std::cmp::Ordering::Greater,
            Op::Lt => compare(&version, &clause.version) == std::cmp::Ordering::Less,
            Op::Ne => compare(&version, &clause.version) != std::cmp::Ordering::Equal,
            Op::Eq => compare(&version, &clause.version) == std::cmp::Ordering::Equal,
            Op::Tilde => {
                compare(&version, &clause.version) != std::cmp::Ordering::Less
                    && compare(&version, &tilde_upper(&clause.version)) == std::cmp::Ordering::Less
            }
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Constraint {
        Constraint::new(s).unwrap()
    }

    #[test]
    fn ranges() {
        assert!(satisfies(&c(">= 3.3, < 5.0"), "3.4.2").unwrap());
        assert!(satisfies(&c(">= 3.3, < 5.0"), "3.3").unwrap());
        assert!(!satisfies(&c(">= 3.3, < 5.0"), "3.2.9").unwrap());
        assert!(!satisfies(&c(">= 3.3, < 5.0"), "5.0").unwrap());
    }

    #[test]
    fn pessimistic() {
        assert!(satisfies(&c("~> 3.3.0"), "3.3.7").unwrap());
        assert!(!satisfies(&c("~> 3.3.0"), "3.4.0").unwrap());
        assert!(!satisfies(&c("~> 3.3.0"), "3.2.9").unwrap());
        assert!(satisfies(&c("~> 4.0"), "4.0.6").unwrap());
        assert!(!satisfies(&c("~> 4.0"), "5.0").unwrap());
    }

    #[test]
    fn exact_and_negated_and_datever() {
        assert!(satisfies(&c("4.0.6"), "4.0.6").unwrap());
        assert!(satisfies(&c("= 4.0.6"), "4.0.6.0").unwrap()); // missing = 0
        assert!(!satisfies(&c("4.0.6"), "4.0.7").unwrap());
        assert!(satisfies(&c("!= 3.0.0"), "3.0.1").unwrap());
        assert!(!satisfies(&c("!= 3.0.0"), "3.0.0").unwrap());
        assert!(satisfies(&c(">= 2024.1"), "2024.11").unwrap());
        assert!(!satisfies(&c(">= 2024.1"), "2023.12").unwrap());
    }

    #[test]
    fn malformed_versions_are_named_errors() {
        assert!(satisfies(&c(">= 3.3"), "three-three").is_err());
        assert!(satisfies(&c(">= 3.3"), "").is_err());
    }
}
