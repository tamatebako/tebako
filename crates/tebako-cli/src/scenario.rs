//! Port of the gem's scenario machinery (lib/tebako/scenario_manager.rb,
//! lib/tebako/ruby_version.rb): root/entry validation, scenario detection,
//! Ruby version selection (including the Gemfile `ruby` directive) and the
//! bundler version resolution driven by Gemfile.lock / the Gemfile's
//! bundler dependency.

use std::path::{Path, PathBuf};

use crate::error::{packaging_error, plain_error, TebakoError};

/// Minimal bundler version providing linux-gnu / linux-musl
/// differentiation (gem's Tebako::BUNDLER_VERSION).
pub const BUNDLER_MIN_VERSION: &str = "2.4.22";

pub const DEFAULT_RUBY_VERSION: &str = "3.3.7";
pub const MIN_RUBY_VERSION_WINDOWS: &str = "3.1.6";

/// Ruby versions the CLI can press packages for (the prebuilt runtime
/// packages published by tebako-runtime-ruby are the operative constraint).
pub const RUBY_VERSIONS: &[&str] = &[
    "2.7.8", "3.0.7", "3.1.6", "3.2.4", "3.2.5", "3.2.6", "3.2.7", "3.3.3", "3.3.4", "3.3.5",
    "3.3.6", "3.3.7", "3.4.1", "3.4.2", "4.0.0", "4.0.1", "4.0.2", "4.0.3", "4.0.4", "4.0.5",
    "4.0.6",
];

// ---------------------------------------------------------------------
// Ruby version + Gem::Requirement-style matching
// ---------------------------------------------------------------------

/// Compare two dotted versions segment by segment (numeric segments;
/// a missing segment counts as 0, like Gem::Version).
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<&str> = a.split('.').collect();
    let pb: Vec<&str> = b.split('.').collect();
    for i in 0..pa.len().max(pb.len()) {
        let sa = pa.get(i).copied().unwrap_or("0");
        let sb = pb.get(i).copied().unwrap_or("0");
        let na = sa.parse::<u64>();
        let nb = sb.parse::<u64>();
        let ord = match (na, nb) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            // Non-numeric segments compare lexically (prerelease spellings);
            // numeric sorts before non-numeric, matching Gem::Version.
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => sa.cmp(sb),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Eq,
    NotEq,
    Gt,
    Lt,
    Ge,
    Le,
    Pessimistic,
}

#[derive(Debug, Clone)]
pub struct Requirement {
    constraints: Vec<(Op, String)>,
}

impl Requirement {
    /// Parse one constraint string like ">= 3.1", "~> 3.3.7", "3.3.7".
    pub fn parse(s: &str) -> Result<Requirement, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty requirement".to_string());
        }
        let (op, rest) = if let Some(r) = s.strip_prefix("~>") {
            (Op::Pessimistic, r)
        } else if let Some(r) = s.strip_prefix(">=") {
            (Op::Ge, r)
        } else if let Some(r) = s.strip_prefix("<=") {
            (Op::Le, r)
        } else if let Some(r) = s.strip_prefix("!=") {
            (Op::NotEq, r)
        } else if let Some(r) = s.strip_prefix('>') {
            (Op::Gt, r)
        } else if let Some(r) = s.strip_prefix('<') {
            (Op::Lt, r)
        } else if let Some(r) = s.strip_prefix('=') {
            (Op::Eq, r)
        } else {
            (Op::Eq, s)
        };
        let version = rest.trim().to_string();
        if version.is_empty() {
            return Err(format!("invalid requirement '{s}'"));
        }
        Ok(Requirement {
            constraints: vec![(op, version)],
        })
    }

    /// Combine several constraint strings (Gem::Requirement.create).
    pub fn create(parts: &[String]) -> Result<Requirement, String> {
        let mut constraints = Vec::new();
        for p in parts {
            constraints.extend(Requirement::parse(p)?.constraints);
        }
        if constraints.is_empty() {
            constraints.push((Op::Ge, "0".to_string()));
        }
        Ok(Requirement { constraints })
    }

    pub fn satisfied_by(&self, version: &str) -> bool {
        self.constraints.iter().all(|(op, v)| {
            let ord = version_cmp(version, v);
            match op {
                Op::Eq => ord == std::cmp::Ordering::Equal,
                Op::NotEq => ord != std::cmp::Ordering::Equal,
                Op::Gt => ord == std::cmp::Ordering::Greater,
                Op::Lt => ord == std::cmp::Ordering::Less,
                Op::Ge => ord != std::cmp::Ordering::Less,
                Op::Le => ord != std::cmp::Ordering::Greater,
                Op::Pessimistic => {
                    // ~> x.y   means >= x.y,  < (x+1)
                    // ~> x.y.z means >= x.y.z, < x.(y+1)
                    if ord == std::cmp::Ordering::Less {
                        return false;
                    }
                    let segs: Vec<&str> = v.split('.').collect();
                    let mut upper: Vec<u64> = segs.iter().map(|s| s.parse().unwrap_or(0)).collect();
                    if upper.len() < 2 {
                        upper.resize(2, 0);
                    }
                    let bump = upper.len() - 2;
                    upper.truncate(bump + 1);
                    upper[bump] += 1;
                    let upper_s = upper
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    version_cmp(version, &upper_s) == std::cmp::Ordering::Less
                }
            }
        })
    }

    pub fn describe(&self) -> String {
        self.constraints
            .iter()
            .map(|(op, v)| {
                let sym = match op {
                    Op::Eq => "=",
                    Op::NotEq => "!=",
                    Op::Gt => ">",
                    Op::Lt => "<",
                    Op::Ge => ">=",
                    Op::Le => "<=",
                    Op::Pessimistic => "~>",
                };
                format!("{sym} {v}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Tebako::RubyVersion: format/support checks (errors 109/110/111).
pub fn check_ruby_version(version: &str) -> Result<(), TebakoError> {
    let parts: Vec<&str> = version.split('.').collect();
    let well_formed = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if !well_formed {
        return Err(packaging_error(
            109,
            Some(&format!("'{version}'. Expected format: x.y.z")),
        ));
    }
    if !RUBY_VERSIONS.contains(&version) {
        return Err(TebakoError::new(
            format!("Ruby version {version} is not supported"),
            110,
        ));
    }
    if cfg!(windows) && version_cmp(version, MIN_RUBY_VERSION_WINDOWS) == std::cmp::Ordering::Less {
        return Err(TebakoError::new(
            format!("Ruby version {version} is not supported on Windows"),
            111,
        ));
    }
    Ok(())
}

/// `x.y.z` -> `x.y.0` (RubyVersion#api_version).
pub fn api_version(ruby_version: &str) -> String {
    let parts: Vec<&str> = ruby_version.split('.').collect();
    format!(
        "{}.{}.0",
        parts.first().unwrap_or(&""),
        parts.get(1).unwrap_or(&"")
    )
}

/// Tebako::RubyVersionWithGemfile: the Gemfile `ruby` directive constrains
/// the selectable Ruby version. `requested` is the --Ruby value (if any).
/// Returns the effective version (errors 115/116 on failure).
pub fn ruby_version_with_gemfile(
    requested: Option<&str>,
    gemfile_path: &Path,
) -> Result<String, TebakoError> {
    let content = std::fs::read_to_string(gemfile_path).map_err(|e| {
        packaging_error(
            115,
            Some(&format!("{e} reading {}", gemfile_path.display())),
        )
    })?;
    let constraints = match parse_ruby_directive(&content) {
        Ok(c) => c,
        Err(e) => return Err(packaging_error(115, Some(&e))),
    };
    if constraints.is_empty() {
        let v = requested.unwrap_or(DEFAULT_RUBY_VERSION).to_string();
        check_ruby_version(&v)?;
        return Ok(v);
    }
    println!(
        "-- Found Gemfile with Ruby requirements [{}]",
        constraints.join(", ")
    );
    let requirement =
        Requirement::create(&constraints).map_err(|e| packaging_error(115, Some(&e)))?;
    match requested {
        Some(v) => {
            if !requirement.satisfied_by(v) {
                return Err(TebakoError::new(
                    format!(
                        "Ruby version {v} does not satisfy requirement '{}'",
                        requirement.describe()
                    ),
                    116,
                ));
            }
            check_ruby_version(v)?;
            Ok(v.to_string())
        }
        None => {
            let matching = RUBY_VERSIONS.iter().find(|v| requirement.satisfied_by(v));
            if let Some(v) = matching {
                println!("-- Found matching Ruby version {v}");
                Ok(v.to_string())
            } else {
                Err(TebakoError::new(
                    format!(
                        "No available Ruby version satisfies requirement {}",
                        requirement.describe()
                    ),
                    116,
                ))
            }
        }
    }
}

/// Extract the version constraints of the Gemfile's `ruby` directive:
/// the quoted strings of a top-level `ruby "..."` line (the engine
/// options Bundler accepts are out of scope).
fn parse_ruby_directive(content: &str) -> Result<Vec<String>, String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("ruby") else {
            continue;
        };
        let rest = rest.trim_start();
        if !rest.starts_with('"') && !rest.starts_with('\'') {
            // `ruby` method call in some other shape (ruby_version etc.)
            continue;
        }
        // Collect every quoted string on the line, stopping at a comment.
        let mut out = Vec::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '#' => break,
                '"' | '\'' => {
                    let quote = c;
                    let mut s = String::new();
                    for c2 in chars.by_ref() {
                        if c2 == quote {
                            break;
                        }
                        s.push(c2);
                    }
                    out.push(s);
                }
                _ => {}
            }
        }
        return Ok(out);
    }
    Ok(Vec::new())
}

// ---------------------------------------------------------------------
// Scenario manager
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    SimpleScript,
    Gemfile,
    Gem,
    Gemspec,
    GemspecAndGemfile,
}

#[derive(Debug)]
pub struct ScenarioManager {
    pub fs_root: String,
    pub fs_entrance: String,
    pub fs_entry_point: String,
    pub fs_mount_point: String,
    pub exe_suffix: String,
    pub scenario: Scenario,
    pub with_gemfile: bool,
    pub gemfile_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub needs_bundler: bool,
    pub bundler_version: String,
}

impl ScenarioManager {
    /// root must already be absolute (the options layer joins relative
    /// roots onto the current directory, like OptionsManager#root).
    pub fn new(root: &str, entrance: &str) -> Result<ScenarioManager, TebakoError> {
        let root_path = Path::new(root);
        if !root_path.is_dir() {
            return Err(packaging_error(107, None));
        }
        let cleaned = cleanpath(root);
        if !Path::new(&cleaned).is_absolute() {
            return Err(packaging_error(113, None));
        }
        let fs_root = std::fs::canonicalize(&cleaned)
            .unwrap_or_else(|_| PathBuf::from(&cleaned))
            .to_string_lossy()
            .into_owned();
        // canonicalize yields the \\?\ verbatim prefix on windows; the
        // entry comparison/reduction below works in cleanpath's drive
        // form, so strip the prefix (\\?\UNC\srv\share keeps its UNC
        // root as \\srv\share).
        #[cfg(windows)]
        let fs_root = {
            let stripped =
                fs_root
                    .strip_prefix("\\\\?\\")
                    .map(|rest| match rest.strip_prefix("UNC\\") {
                        Some(unc) => format!("\\\\{unc}"),
                        None => rest.to_string(),
                    });
            stripped.unwrap_or(fs_root)
        };

        let mut ent = cleanpath(entrance);
        if absolute_path(&ent) {
            if !path_starts_with(&ent, &fs_root) {
                return Err(packaging_error(114, None));
            }
            let original = ent.clone();
            let reduced = Path::new(&ent)
                .strip_prefix(&fs_root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or(ent);
            println!(
                "-- Absolute path to entry point '{original}' will be reduced to '{reduced}' relative to '{fs_root}'"
            );
            ent = reduced;
        }

        let msys = cfg!(windows);
        Ok(ScenarioManager {
            fs_root,
            fs_entrance: ent.clone(),
            fs_entry_point: format!("/bin/{ent}"),
            fs_mount_point: if msys {
                "A:/__tfs__".to_string()
            } else {
                "/__tfs__".to_string()
            },
            exe_suffix: if msys {
                ".exe".to_string()
            } else {
                String::new()
            },
            scenario: Scenario::SimpleScript,
            with_gemfile: false,
            gemfile_path: PathBuf::new(),
            lockfile_path: PathBuf::new(),
            needs_bundler: false,
            bundler_version: BUNDLER_MIN_VERSION.to_string(),
        })
    }

    /// Scenario detection (gemspec/gem/Gemfile counts); bundler
    /// resolution itself happens lazily in `resolve_bundler` during
    /// deploy, like the gem's DeployHelper (a ScenarioManagerWithBundler).
    pub fn configure_scenario(&mut self) -> Result<(), TebakoError> {
        let root = Path::new(&self.fs_root);
        let gs_length = count_glob(root, "gemspec");
        let g_length = count_glob(root, "gem");
        self.with_gemfile = root.join("Gemfile").is_file();
        self.gemfile_path = root.join("Gemfile");
        self.lockfile_path = root.join("Gemfile.lock");

        match gs_length {
            0 => {
                if self.with_gemfile || g_length == 0 {
                    self.fs_entry_point = format!("/local/{}", self.fs_entrance);
                }
                self.scenario = if self.with_gemfile {
                    Scenario::Gemfile
                } else if g_length > 0 {
                    Scenario::Gem
                } else {
                    Scenario::SimpleScript
                };
                Ok(())
            }
            1 => {
                self.scenario = if self.with_gemfile {
                    Scenario::GemspecAndGemfile
                } else {
                    Scenario::Gemspec
                };
                Ok(())
            }
            _ => Err(plain_error(format!(
                "Multiple Ruby gemspecs found in {}",
                self.fs_root
            ))),
        }
    }

    /// ScenarioManagerWithBundler#lookup_files: pin the bundler version
    /// from Gemfile.lock, or resolve the Gemfile's bundler dependency
    /// against the latest published bundler (the gem's SpecFetcher).
    pub fn resolve_bundler(&mut self) -> Result<(), TebakoError> {
        let with_lockfile = self.lockfile_path.is_file();
        if with_lockfile {
            self.update_bundler_version_from_lockfile()
        } else if self.with_gemfile {
            self.update_bundler_version_from_gemfile()
        } else {
            Ok(())
        }
    }

    fn update_bundler_version_from_lockfile(&mut self) -> Result<(), TebakoError> {
        println!("   ... using lockfile at {}", self.lockfile_path.display());
        let content =
            std::fs::read_to_string(&self.lockfile_path).map_err(|_| packaging_error(117, None))?;
        let Some(version) = parse_bundled_with(&content) else {
            return Err(packaging_error(117, None));
        };
        self.bundler_version = version.clone();
        self.needs_bundler = true;
        if version_cmp(&version, BUNDLER_MIN_VERSION) == std::cmp::Ordering::Less {
            return Err(packaging_error(
                118,
                Some(&format!(
                    " : {version} requested, {BUNDLER_MIN_VERSION} minimum required"
                )),
            ));
        }
        Ok(())
    }

    fn update_bundler_version_from_gemfile(&mut self) -> Result<(), TebakoError> {
        let content = std::fs::read_to_string(&self.gemfile_path)
            .map_err(|e| packaging_error(115, Some(&format!("{e}"))))?;
        let constraints = parse_bundler_dependency(&content);
        if constraints.is_empty() {
            return Ok(());
        }
        self.needs_bundler = true;
        // Gem::SpecFetcher.detect(:released): the latest published bundler
        // satisfying the Gemfile's requirement and the tebako minimum.
        // The rubygems 'latest' endpoint names exactly that candidate
        // unless the requirement excludes it (then no version fits the
        // gem's detect either, unless an older one does — see README).
        let mut all = constraints;
        all.push(format!(">= {BUNDLER_MIN_VERSION}"));
        let requirement = Requirement::create(&all).map_err(|_| packaging_error(119, None))?;
        let latest = latest_bundler_version().ok_or_else(|| packaging_error(119, None))?;
        if !requirement.satisfied_by(&latest) {
            return Err(packaging_error(119, None));
        }
        self.bundler_version = latest;
        Ok(())
    }
}

/// `BUNDLED WITH\n   x.y.z` block of a Gemfile.lock.
fn parse_bundled_with(content: &str) -> Option<String> {
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() == "BUNDLED WITH" {
            let next = lines.next()?;
            let v = next.trim();
            if !v.is_empty() && v.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return Some(v.to_string());
            }
            return None;
        }
    }
    None
}

/// Constraints of a `gem "bundler", ...` dependency in a Gemfile.
fn parse_bundler_dependency(content: &str) -> Vec<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("gem") else {
            continue;
        };
        let rest = rest.trim_start();
        let Ok(first) = first_quoted(rest) else {
            continue;
        };
        if first != "bundler" {
            continue;
        }
        // Remaining quoted strings on the line are version constraints.
        let mut out = Vec::new();
        let mut seen_first = false;
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '#' {
                break;
            }
            if c == '"' || c == '\'' {
                let mut s = String::new();
                for c2 in chars.by_ref() {
                    if c2 == c {
                        break;
                    }
                    s.push(c2);
                }
                if seen_first {
                    out.push(s);
                } else {
                    seen_first = true;
                }
            }
        }
        return out;
    }
    Vec::new()
}

fn first_quoted(s: &str) -> Result<String, ()> {
    let mut chars = s.chars();
    let quote = match chars.next() {
        Some('"') => '"',
        Some('\'') => '\'',
        _ => return Err(()),
    };
    let mut out = String::new();
    for c in chars {
        if c == quote {
            return Ok(out);
        }
        out.push(c);
    }
    Err(())
}

/// Latest released bundler version per rubygems.org (the SpecFetcher
/// stand-in); None when the endpoint cannot be read.
fn latest_bundler_version() -> Option<String> {
    let body = crate::fetch::fetch_text("https://rubygems.org/api/v1/versions/bundler/latest.json")
        .ok()?;
    let json = tebako_pkg::json_parse(&body).ok()?;
    json.find("version").and_then(|v| v.as_string())
}

/// Lexical path cleanup (Pathname.cleanpath for the shapes tebako sees:
/// duplicate slashes, ".", ".." with an absolute base). The windows
/// shapes keep their prefix: a drive-letter path ("D:\…") keeps the
/// drive — absolutizing with a leading slash would LOSE it ("/D:/…" is
/// rooted but NOT absolute to Rust's windows is_absolute, which is how
/// the root check misjudged drive roots as relative); a UNC path
/// ("\\srv\share\…") comes out as "//srv/share/…".
pub fn cleanpath(p: &str) -> String {
    let drive = if cfg!(windows) && p.chars().nth(1) == Some(':') {
        Some(&p[..2])
    } else {
        None
    };
    let unc = cfg!(windows) && drive.is_none() && p.starts_with("\\\\");
    let absolute = p.starts_with('/') || drive.is_some() || unc;
    let mut out: Vec<&str> = Vec::new();
    for part in p.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|last| *last != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            // The drive component goes back verbatim (see above).
            _ if Some(part) == drive => {}
            _ => out.push(part),
        }
    }
    let joined = out.join("/");
    if let Some(d) = drive {
        format!("{d}/{joined}")
    } else if unc {
        format!("//{joined}")
    } else if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// Ruby `Pathname#absolute?` semantics: Rust's `Path::is_absolute` (the
/// drive-letter/UNC forms on windows), plus the rooted form (`/x`, `\x`)
/// — Ruby and the MS CRT count it as absolute while Rust's windows
/// is_absolute calls it rooted-but-not-absolute (no drive). An entry
/// like `/etc/passwd` is absolute on every platform tebako runs on.
fn absolute_path(p: &str) -> bool {
    Path::new(p).is_absolute() || (cfg!(windows) && (p.starts_with('/') || p.starts_with('\\')))
}

fn path_starts_with(path: &str, prefix: &str) -> bool {
    Path::new(path).starts_with(Path::new(prefix))
}

fn count_glob(dir: &Path, ext: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|x| x == ext))
                .count()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert_eq!(version_cmp("2.4.22", "2.4.22"), std::cmp::Ordering::Equal);
        assert_eq!(version_cmp("2.4.9", "2.4.22"), std::cmp::Ordering::Less);
        assert_eq!(version_cmp("2.5.0", "2.4.22"), std::cmp::Ordering::Greater);
        // Gem::Version pads the shorter version with zero segments
        assert_eq!(version_cmp("3.3.7", "3.3"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("3.3.0", "3.3"), std::cmp::Ordering::Equal);
        assert_eq!(version_cmp("10.0.0", "9.9.9"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn requirement_matching() {
        let ge = Requirement::parse(">= 2.4.22").unwrap();
        assert!(ge.satisfied_by("2.4.22"));
        assert!(ge.satisfied_by("2.6.1"));
        assert!(!ge.satisfied_by("2.4.21"));

        // ~> 3.3 means >= 3.3, < 4
        let twiddle = Requirement::parse("~> 3.3").unwrap();
        assert!(twiddle.satisfied_by("3.3.7"));
        assert!(twiddle.satisfied_by("3.4.1"));
        assert!(!twiddle.satisfied_by("4.0.0"));
        assert!(!twiddle.satisfied_by("3.2.9"));

        // ~> 3.3.0 means >= 3.3.0, < 3.4
        let twiddle3 = Requirement::parse("~> 3.3.0").unwrap();
        assert!(twiddle3.satisfied_by("3.3.7"));
        assert!(!twiddle3.satisfied_by("3.4.0"));

        let combined = Requirement::create(&[">= 3.1".to_string(), "< 3.4".to_string()]).unwrap();
        assert!(combined.satisfied_by("3.3.7"));
        assert!(!combined.satisfied_by("3.4.1"));
        assert!(!combined.satisfied_by("3.0.7"));
    }

    #[test]
    fn ruby_directive_parsing() {
        assert_eq!(
            parse_ruby_directive("source \"https://rubygems.org\"\nruby \"3.3.7\"\n").unwrap(),
            vec!["3.3.7".to_string()]
        );
        assert_eq!(
            parse_ruby_directive("ruby '>= 3.1', '< 3.4'\n").unwrap(),
            vec![">= 3.1".to_string(), "< 3.4".to_string()]
        );
        assert!(parse_ruby_directive("source \"x\"\n").unwrap().is_empty());
        assert!(parse_ruby_directive("# ruby \"9.9.9\"\n")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn bundled_with_parsing() {
        let lock = "GEM\n  specs:\n\nBUNDLED WITH\n   2.5.6\n";
        assert_eq!(parse_bundled_with(lock), Some("2.5.6".to_string()));
        assert_eq!(parse_bundled_with("GEM\n"), None);
    }

    #[test]
    fn bundler_dep_parsing() {
        assert_eq!(
            parse_bundler_dependency("gem \"bundler\", \">= 2.4\"\n"),
            vec![">= 2.4".to_string()]
        );
        assert!(parse_bundler_dependency("gem \"rake\"\n").is_empty());
        assert!(parse_bundler_dependency("source \"x\"\n").is_empty());
    }

    #[test]
    fn cleanpath_lexical() {
        assert_eq!(cleanpath("/a/b/"), "/a/b");
        assert_eq!(cleanpath("/a/./b/../c"), "/a/c");
        assert_eq!(cleanpath("a/b"), "a/b");
        assert_eq!(cleanpath("/a//b"), "/a/b");
    }

    #[test]
    #[cfg(windows)]
    fn cleanpath_windows_shapes() {
        // Drive-letter paths keep the drive — never "/D:/…" (the 113
        // misjudgment: rooted-but-not-absolute).
        assert_eq!(cleanpath("D:\\a\\b\\"), "D:/a/b");
        assert_eq!(cleanpath("D:/a/./b/../c"), "D:/a/c");
        // UNC paths absolutize as //srv/share.
        assert_eq!(cleanpath("\\\\srv\\share\\x"), "//srv/share/x");
        // Rooted paths stay POSIX-shaped.
        assert_eq!(cleanpath("/a/b"), "/a/b");
    }

    #[test]
    #[cfg(windows)]
    fn absolute_path_counts_the_rooted_form() {
        // Ruby Pathname#absolute?: the rooted form counts on windows too.
        assert!(absolute_path("/etc/passwd"));
        assert!(absolute_path("D:/a/b"));
        assert!(absolute_path("\\\\srv\\share\\x"));
        assert!(!absolute_path("rel/x"));
    }

    #[test]
    fn api_version_mapping() {
        assert_eq!(api_version("3.3.7"), "3.3.0");
        assert_eq!(api_version("4.0.6"), "4.0.0");
    }
}
