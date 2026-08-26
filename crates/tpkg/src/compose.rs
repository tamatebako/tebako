//! The D2 composition document (spec 23 §3/§13) — the press-time subset.
//!
//! `tebako press --compose <tebako.yaml>` reads this document: one runtime
//! requirement plus N payload slice references, the per-slice `carry`
//! verdicts (the composition spectrum, spec 23 §13), and the `platforms:`
//! coverage assertions (§13.3). Press resolves the full closure at build
//! time and bakes the lock into the L2 package manifest (§4).
//!
//! Out of this layer (Phase-R, spec 23's status line): the jail/policy
//! keys (`policy:`, `mounts:`, `needs:`). They are REFUSED by name here —
//! a compose document that looks jail-declaring but is pressed without
//! its policy would be a silent security downgrade (fail-closed, spec 00
//! §9). Truly unknown keys stay tolerated (forward compatibility).
//!
//! ```yaml
//! version: 1
//! preset: shared-runtime        # self-contained | shared-runtime
//!                               # (lean/fat: deprecated aliases, a named
//!                               # warning each — never silent)
//! runtime:                      # … or the shorthand: ref: "ruby@~> 3.3"
//!   name: ruby
//!   requirement: "~> 3.3"
//!   carry: false                # true = the two-slot carried pair (spec 19 §6.1)
//!   platforms: [macos-arm64]    # OPTIONAL coverage assertion (release-asset names)
//! slices:
//!   - name: metanorma           # … or ref: "metanorma@>= 2.1"
//!     requirement: ">= 2.1"
//!     carry: true
//!   - ref: "openjdk@21"
//!     platforms: universal      # assertion: fails the press when not universal
//! entrypoint: mnconvert         # the pointer-package form's entry selector
//! quiet_notices: true           # optional: bake TPKG_FLAG_QUIET_NOTICES
//!                               # (registry setting, spec 23 §14 — CLI/env override)
//! ```

use serde::Deserialize;
use std::fmt;

use crate::manifest::{Constraint, Platform, Platforms};

/// The only `version` this implementation reads.
pub const COMPOSE_SCHEMA_VERSION: u32 = 1;

/// The compose document keys that are declared in spec 23 §3 but belong to
/// the Phase-R jail/dispatch wiring — refused by name, never silently
/// dropped.
const PHASE_R_KEYS: [&str; 3] = ["policy", "mounts", "needs"];

/// Compose document errors.
#[derive(Debug)]
pub enum ComposeError {
    /// YAML structural failure (the document does not match the model).
    Yaml(serde_yml::Error),
    /// Semantic validation failure (the reason travels).
    Invalid(String),
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComposeError::Yaml(e) => write!(f, "compose document yaml error: {e}"),
            ComposeError::Invalid(reason) => write!(f, "invalid compose document: {reason}"),
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<serde_yml::Error> for ComposeError {
    fn from(e: serde_yml::Error) -> ComposeError {
        ComposeError::Yaml(e)
    }
}

fn invalid(reason: impl Into<String>) -> ComposeError {
    ComposeError::Invalid(reason.into())
}

/// The package preset (spec 23 §13.2). The deprecated aliases parse with a
/// named warning each — never silently (invariant 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposePreset {
    /// Carries the full closure: runtime exe + env image + every payload
    /// slice. Zero network, empty cache, one file. (The "fat" successor.)
    SelfContained,
    /// Carries the app payload(s), shares the runtime and any slice marked
    /// shared. The DEFAULT. (The "lean" successor.)
    SharedRuntime,
}

impl ComposePreset {
    /// Parse a preset spelling; the deprecated aliases map onto their
    /// successors and return the named warning the caller surfaces.
    pub fn parse(spelling: &str) -> Result<(ComposePreset, Option<String>), ComposeError> {
        match spelling {
            "self-contained" => Ok((ComposePreset::SelfContained, None)),
            "shared-runtime" => Ok((ComposePreset::SharedRuntime, None)),
            "fat" => Ok((
                ComposePreset::SelfContained,
                Some(
                    "the 'fat' preset is deprecated — spell it 'self-contained' (spec 23 §13.2)"
                        .to_string(),
                ),
            )),
            "lean" => Ok((
                ComposePreset::SharedRuntime,
                Some(
                    "the 'lean' preset is deprecated — spell it 'shared-runtime' (spec 23 §13.2)"
                        .to_string(),
                ),
            )),
            _ => Err(invalid(format!(
                "unknown preset '{spelling}' (self-contained | shared-runtime expected; lean/fat are deprecated aliases)"
            ))),
        }
    }

    /// The spelling this preset serializes as (the canonical, never the
    /// deprecated alias).
    pub fn name(self) -> &'static str {
        match self {
            ComposePreset::SelfContained => "self-contained",
            ComposePreset::SharedRuntime => "shared-runtime",
        }
    }

    /// The preset's default carry verdict (spec 23 §13.2): self-contained
    /// carries everything; shared-runtime shares the runtime and carries
    /// the payload slices. A slice's authored `carry:` overrides this.
    pub fn default_carry(self, is_runtime: bool) -> bool {
        match self {
            ComposePreset::SelfContained => true,
            ComposePreset::SharedRuntime => !is_runtime,
        }
    }
}

/// One slice reference of the document (the runtime or a payload slice):
/// `name:` + `requirement:`, or the `ref: "name@constraint"` shorthand —
/// one semantics, two spellings (a conflict is a named validation error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeSliceRef {
    pub name: String,
    pub requirement: Option<Constraint>,
    /// The authored carry verdict; `None` = the preset decides.
    pub carry: Option<bool>,
    /// The coverage assertion (spec 23 §13.3); `None` = the payload's own
    /// declaration rules.
    pub platforms: Option<Platforms>,
}

#[derive(Clone, Deserialize)]
struct RawSlice {
    name: Option<String>,
    #[serde(rename = "ref")]
    ref_: Option<String>,
    requirement: Option<String>,
    carry: Option<bool>,
    platforms: Option<RawPlatforms>,
}

/// The assertion's YAML shape: `universal` or a list of release-asset
/// platform names (the spec 23 §3 example's spelling).
#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum RawPlatforms {
    Universal(String),
    List(Vec<String>),
}

impl RawPlatforms {
    fn to_platforms(&self, what: &str) -> Result<Platforms, ComposeError> {
        match self {
            RawPlatforms::Universal(s) if s == "universal" => Ok(Platforms::Universal),
            RawPlatforms::Universal(s) => Err(invalid(format!(
                "{what}: platforms must be 'universal' or a list of platform names, got '{s}'"
            ))),
            RawPlatforms::List(names) => {
                let mut out = Vec::new();
                for name in names {
                    let Some(p) = Platform::from_release_asset_name(name) else {
                        return Err(invalid(format!(
                            "{what}: unknown platform '{name}' (release-asset names: macos-arm64, linux-gnu-x86_64, …)"
                        )));
                    };
                    out.push(p);
                }
                if out.is_empty() {
                    return Err(invalid(format!(
                        "{what}: an empty platforms list asserts nothing (spell 'universal')"
                    )));
                }
                Ok(Platforms::Triplets(out))
            }
        }
    }
}

/// The shorthand: `name@constraint` — the name is everything before the
/// FIRST '@', the constraint the rest (the requirement grammar).
fn parse_ref_shorthand(ref_: &str, what: &str) -> Result<(String, Constraint), ComposeError> {
    let Some(at) = ref_.find('@') else {
        return Err(invalid(format!(
            "{what}: ref '{ref_}' is not 'name@constraint' (the shorthand needs both)"
        )));
    };
    let (name, constraint) = (&ref_[..at], &ref_[at + 1..]);
    if name.is_empty() || constraint.is_empty() {
        return Err(invalid(format!(
            "{what}: ref '{ref_}' is not 'name@constraint' (empty name or constraint)"
        )));
    }
    let constraint = Constraint::new(constraint).map_err(|e| {
        invalid(format!(
            "{what}: ref '{ref_}' carries an unparseable constraint: {e}"
        ))
    })?;
    Ok((name.to_string(), constraint))
}

fn resolve_slice_ref(raw: &RawSlice, what: &str) -> Result<ComposeSliceRef, ComposeError> {
    let shorthand = raw
        .ref_
        .as_deref()
        .map(|r| parse_ref_shorthand(r, what))
        .transpose()?;
    let expanded_name = raw.name.clone().filter(|n| !n.trim().is_empty());
    let expanded_req = raw
        .requirement
        .as_deref()
        .map(|r| {
            Constraint::new(r)
                .map_err(|e| invalid(format!("{what}: unparseable requirement '{r}': {e}")))
        })
        .transpose()?;
    let (name, requirement) = match (shorthand, expanded_name, expanded_req) {
        (Some((ref_name, ref_req)), Some(name), req) => {
            // Both spellings present: they must AGREE — a conflict is a
            // named error (spec 23 §3); agreement = the expanded form.
            let conflict = ref_name != name || req.as_ref().is_some_and(|r| *r != ref_req);
            if conflict {
                return Err(invalid(format!(
                    "{what}: ref '{ref_name}@{}' conflicts with the expanded name/requirement — one semantics, two spellings, never two meanings",
                    ref_req.as_str()
                )));
            }
            (name, Some(req.unwrap_or(ref_req)))
        }
        (Some((ref_name, ref_req)), None, None) => (ref_name, Some(ref_req)),
        (Some((ref_name, ref_req)), None, Some(_)) => {
            // A requirement alongside the shorthand is a conflict waiting
            // to happen — the shorthand already carries one.
            return Err(invalid(format!(
                "{what}: ref '{ref_name}@{}' and a separate requirement: key — use one spelling",
                ref_req.as_str()
            )));
        }
        (None, Some(name), req) => (name, req),
        (None, None, _) => {
            return Err(invalid(format!(
                "{what}: a slice needs a name (or the ref: 'name@constraint' shorthand)"
            )));
        }
    };
    let platforms = raw
        .platforms
        .as_ref()
        .map(|p| p.to_platforms(what))
        .transpose()?;
    Ok(ComposeSliceRef {
        name,
        requirement,
        carry: raw.carry,
        platforms,
    })
}

/// The parsed + validated composition document, plus the deprecation
/// warnings the caller surfaces (the aliases' named warnings ride
/// out-of-band — they are not errors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeDoc {
    pub preset: ComposePreset,
    /// The runtime slice reference. The name is the engine (`ruby`); the
    /// requirement the version constraint.
    pub runtime: ComposeSliceRef,
    /// The payload slices, in document order.
    pub slices: Vec<ComposeSliceRef>,
    /// The pointer-package form's entry selector (a PROVIDES entrypoint
    /// name of one of the slices). Absent when the press packages a local
    /// root (the app's own entry governs).
    pub entrypoint: Option<String>,
    /// The config channel of the `quiet_notices` registry setting
    /// (spec 23 §14): `Some(v)` declares the package's notice policy in
    /// the repo-carried document — environment-independent, git-
    /// shareable. CLI and env channels override it at press; the
    /// resolved value bakes `TPKG_FLAG_QUIET_NOTICES`.
    pub quiet_notices: Option<bool>,
}

impl ComposeDoc {
    /// The carry verdict of one slice: authored `carry:` wins, else the
    /// preset's default (spec 23 §13.2).
    pub fn carry_of(&self, slice: &ComposeSliceRef, is_runtime: bool) -> bool {
        slice
            .carry
            .unwrap_or_else(|| self.preset.default_carry(is_runtime))
    }
}

#[derive(Deserialize)]
struct RawDoc {
    version: Option<u32>,
    preset: Option<String>,
    runtime: Option<RawSlice>,
    #[serde(default)]
    slices: Vec<RawSlice>,
    entrypoint: Option<String>,
    // The config channel of the `quiet_notices` registry setting
    // (spec 23 §14) — tri-state; absent lets the CLI/env channels rule.
    quiet_notices: Option<bool>,
    // Phase-R keys — present ⇒ the named refusal, never a silent drop.
    policy: Option<serde_yml::Value>,
    mounts: Option<serde_yml::Value>,
    needs: Option<serde_yml::Value>,
}

/// Parse and validate a composition document. Returns the document and
/// the deprecation warnings to surface (preset aliases).
pub fn parse_compose(yaml: &str) -> Result<(ComposeDoc, Vec<String>), ComposeError> {
    let raw: RawDoc = serde_yml::from_str(yaml)?;
    for key in PHASE_R_KEYS {
        let present = match key {
            "policy" => raw.policy.is_some(),
            "mounts" => raw.mounts.is_some(),
            _ => raw.needs.is_some(),
        };
        if present {
            return Err(invalid(format!(
                "the '{key}:' key is the Phase-R jail wiring (spec 23 §5–§8) — not pressable today; press --jail owns a package's policy request"
            )));
        }
    }
    if raw.version != Some(COMPOSE_SCHEMA_VERSION) {
        return Err(invalid(format!(
            "version: {} is required (the compose document's schema version)",
            COMPOSE_SCHEMA_VERSION
        )));
    }
    let mut warnings = Vec::new();
    let (preset, warning) = match raw.preset.as_deref() {
        Some(spelling) => ComposePreset::parse(spelling)?,
        None => (ComposePreset::SharedRuntime, None),
    };
    if let Some(warning) = warning {
        warnings.push(warning);
    }
    let runtime = raw
        .runtime
        .as_ref()
        .map(|r| resolve_slice_ref(r, "runtime"))
        .transpose()?
        .ok_or_else(|| invalid("runtime: is required (the composition's engine slice)"))?;
    if runtime.requirement.is_none() {
        return Err(invalid(
            "runtime: needs a requirement (a version constraint — press always resolves by constraint)",
        ));
    }
    let mut slices = Vec::new();
    for (i, raw_slice) in raw.slices.iter().enumerate() {
        slices.push(resolve_slice_ref(raw_slice, &format!("slices[{}]", i + 1))?);
    }
    let mut names: Vec<&str> = slices.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    if names.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("duplicate slice name (one row per slice)"));
    }
    let entrypoint = raw.entrypoint.filter(|e| !e.trim().is_empty());
    Ok((
        ComposeDoc {
            preset,
            runtime,
            slices,
            entrypoint,
            quiet_notices: raw.quiet_notices,
        },
        warnings,
    ))
}

/// The spec 23 §13.3 coverage check (fail-closed, press-side): the
/// document's per-slice `platforms:` assertion must be COVERED by the
/// payload's declared coverage (the assertion narrows, never extends), and
/// the host triplet must be covered by the (possibly narrowed) effective
/// coverage. The named error names the slice, the triplet, and the
/// declared coverage.
pub fn check_platforms_assertion(
    slice: &str,
    declared: &Platforms,
    assertion: Option<&Platforms>,
    host: Platform,
) -> Result<(), ComposeError> {
    let declared_list = |declared: &Platforms| match declared {
        Platforms::Universal => "universal".to_string(),
        Platforms::Triplets(ts) => ts
            .iter()
            .map(|p| p.as_triplet())
            .collect::<Vec<_>>()
            .join(", "),
    };
    match assertion {
        Some(Platforms::Universal) => {
            // A universal assertion over a triplet-listed payload EXTENDS
            // the declared coverage — refused by name.
            if let Platforms::Triplets(_) = declared {
                return Err(invalid(format!(
                    "slice '{slice}': the assertion is universal but the payload declares coverage {} — the assertion narrows, never extends (spec 23 §13.3)",
                    declared_list(declared),
                )));
            }
            Ok(())
        }
        Some(Platforms::Triplets(ts)) => {
            let uncovered = ts.iter().find(|p| !declared.covers(**p));
            if let Some(p) = uncovered {
                return Err(invalid(format!(
                    "slice '{slice}': the platforms assertion names {} but the payload declares coverage {} — the assertion narrows, never extends (spec 23 §13.3)",
                    p.as_triplet(),
                    declared_list(declared),
                )));
            }
            if !ts.contains(&host) {
                return Err(invalid(format!(
                    "slice '{slice}': the platforms assertion does not cover the host triplet {} (assertion: {}; declared coverage: {})",
                    host.as_triplet(),
                    ts.iter()
                        .map(|p| p.as_triplet())
                        .collect::<Vec<_>>()
                        .join(", "),
                    declared_list(declared),
                )));
            }
            Ok(())
        }
        None => {
            if !declared.covers(host) {
                return Err(invalid(format!(
                    "slice '{slice}': the payload's declared coverage ({}) does not cover the host triplet {} (spec 23 §13.3)",
                    declared_list(declared),
                    host.as_triplet(),
                )));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<(ComposeDoc, Vec<String>), ComposeError> {
        parse_compose(yaml)
    }

    #[test]
    fn the_spec_23_example_parses() {
        let yaml = "\
version: 1
preset: shared-runtime
runtime:
  name: ruby
  requirement: \"~> 3.3\"
  carry: false
  platforms: [macos-arm64, linux-gnu-x86_64]
slices:
  - name: metanorma
    requirement: \">= 2.1\"
    carry: true
  - name: openjdk
    requirement: \"21\"
    carry: true
  - ref: \"ourorg-templates@3\"
    carry: false
    platforms: universal
entrypoint: mnconvert
";
        let (doc, warnings) = parse(yaml).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(doc.preset, ComposePreset::SharedRuntime);
        assert_eq!(doc.runtime.name, "ruby");
        assert_eq!(doc.runtime.requirement.unwrap().as_str(), "~> 3.3");
        assert_eq!(doc.runtime.carry, Some(false));
        assert_eq!(
            doc.runtime.platforms,
            Some(Platforms::Triplets(vec![
                Platform::Aarch64Macos,
                Platform::X86_64LinuxGnu
            ]))
        );
        assert_eq!(doc.slices.len(), 3);
        assert_eq!(doc.slices[2].name, "ourorg-templates");
        assert_eq!(doc.slices[2].requirement.as_ref().unwrap().as_str(), "3");
        assert_eq!(doc.slices[2].carry, Some(false));
        assert_eq!(doc.slices[2].platforms, Some(Platforms::Universal));
        assert_eq!(doc.entrypoint.as_deref(), Some("mnconvert"));
    }

    #[test]
    fn quiet_notices_parses_tri_state() {
        // spec 23 §14: the compose key is the config channel of the
        // registry setting — present-true, present-false, and absent
        // (the CLI/env channels then rule) are three distinct states.
        let yes = parse("version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nquiet_notices: true\n")
            .unwrap()
            .0;
        assert_eq!(yes.quiet_notices, Some(true));
        let no = parse("version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nquiet_notices: false\n")
            .unwrap()
            .0;
        assert_eq!(no.quiet_notices, Some(false));
        let absent = parse("version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\n")
            .unwrap()
            .0;
        assert_eq!(absent.quiet_notices, None);
    }

    #[test]
    fn the_default_preset_is_shared_runtime() {
        let yaml = "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\n";
        let (doc, warnings) = parse(yaml).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(doc.preset, ComposePreset::SharedRuntime);
        assert_eq!(doc.runtime.name, "ruby");
        assert!(!doc.carry_of(&doc.runtime, true));
    }

    #[test]
    fn lean_and_fat_are_named_warnings_never_silent() {
        let (doc, warnings) =
            parse("version: 1\npreset: lean\nruntime: {ref: \"ruby@~> 3.3\"}\n").unwrap();
        assert_eq!(doc.preset, ComposePreset::SharedRuntime);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'lean' preset is deprecated"));
        assert!(warnings[0].contains("shared-runtime"));

        let (doc, warnings) =
            parse("version: 1\npreset: fat\nruntime: {ref: \"ruby@~> 3.3\"}\n").unwrap();
        assert_eq!(doc.preset, ComposePreset::SelfContained);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'fat' preset is deprecated"));
        assert!(warnings[0].contains("self-contained"));
        assert!(doc.carry_of(&doc.runtime, true));
    }

    #[test]
    fn validation_is_fail_closed() {
        let bad = |yaml: &str| parse(yaml).is_err();
        assert!(bad("runtime: {ref: \"ruby@~> 3.3\"}\n")); // no version
        assert!(bad("version: 2\nruntime: {ref: \"ruby@~> 3.3\"}\n"));
        assert!(bad("version: 1\n")); // no runtime
        assert!(bad(
            "version: 1\npreset: chunky\nruntime: {ref: \"ruby@~> 3.3\"}\n"
        ));
        assert!(bad("version: 1\nruntime: {name: ruby}\n")); // no requirement
        assert!(bad("version: 1\nruntime: {ref: \"ruby\"}\n")); // shorthand without constraint
        assert!(bad("version: 1\nruntime: {ref: \"@~> 3.3\"}\n")); // empty name
                                                                   // a ref/expanded conflict
        assert!(bad(
            "version: 1\nruntime: {name: ruby, requirement: \"~> 3.3\", ref: \"ruby@>= 3.4\"}\n"
        ));
        // a duplicate slice name
        assert!(bad(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nslices:\n  - {name: a, requirement: \"1\"}\n  - {ref: \"a@2\"}\n"
        ));
        // the Phase-R keys refuse by name
        assert!(bad(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\npolicy: deny\n"
        ));
        assert!(bad(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nmounts: []\n"
        ));
        assert!(bad(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nneeds: {}\n"
        ));
        // unknown platforms spellings
        assert!(bad(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nslices:\n  - {name: a, platforms: [plan9]}\n"
        ));
        // unknown keys stay tolerated (forward compatibility)
        let (doc, _) =
            parse("version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nfuture: {anything: goes}\n")
                .unwrap();
        assert!(doc.slices.is_empty());
    }

    #[test]
    fn agreeing_spellings_are_not_a_conflict() {
        let (doc, _) = parse(
            "version: 1\nruntime: {name: ruby, requirement: \"~> 3.3\", ref: \"ruby@~> 3.3\"}\n",
        )
        .unwrap();
        assert_eq!(doc.runtime.name, "ruby");
        assert_eq!(doc.runtime.requirement.unwrap().as_str(), "~> 3.3");
    }

    #[test]
    fn the_platforms_assertion_narrows_never_extends() {
        let declared = Platforms::Triplets(vec![Platform::Aarch64Macos, Platform::X86_64LinuxGnu]);
        // covered assertion
        check_platforms_assertion(
            "s",
            &declared,
            Some(&Platforms::Triplets(vec![Platform::Aarch64Macos])),
            Platform::Aarch64Macos,
        )
        .unwrap();
        // extending assertion
        let err = check_platforms_assertion(
            "my-slice",
            &declared,
            Some(&Platforms::Triplets(vec![Platform::X86_64WindowsUcrt])),
            Platform::Aarch64Macos,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("my-slice"), "{msg}");
        assert!(msg.contains("x86_64-windows-ucrt"), "{msg}");
        assert!(msg.contains("narrows, never extends"), "{msg}");
        // a universal assertion over a triplet-listed payload extends — refused
        let err = check_platforms_assertion(
            "my-slice",
            &declared,
            Some(&Platforms::Universal),
            Platform::Aarch64Macos,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("assertion is universal"), "{msg}");
        assert!(msg.contains("narrows, never extends"), "{msg}");
        // assertion not covering the host
        let err = check_platforms_assertion(
            "my-slice",
            &declared,
            Some(&Platforms::Triplets(vec![Platform::X86_64LinuxGnu])),
            Platform::Aarch64Macos,
        )
        .unwrap_err();
        assert!(err.to_string().contains("does not cover the host triplet"));
        // no assertion: the declaration rules — an uncovered host is named
        let err =
            check_platforms_assertion("my-slice", &declared, None, Platform::X86_64WindowsUcrt)
                .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("my-slice") && msg.contains("x86_64-windows-ucrt"),
            "{msg}"
        );
        // universal declares everything
        check_platforms_assertion(
            "s",
            &Platforms::Universal,
            Some(&Platforms::Triplets(vec![Platform::X86_64WindowsUcrt])),
            Platform::X86_64WindowsUcrt,
        )
        .unwrap();
    }
}
