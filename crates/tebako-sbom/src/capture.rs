//! The crate-inventory capture (`crate-inventory.json`, schema 1) — the
//! build-time artifact that carries one cargo resolution's crate set from
//! the build that produced it to the SBOM that describes it (spec 15
//! §8a, tebako#1121).
//!
//! The capture exists because a native exe's vendored crates are not
//! recoverable from its bytes: the only honest inventory is the Cargo
//! resolution AT BUILD TIME. The writer is `sbom-cargo
//! --inventory-only` (the product release's link-unit legs emit it
//! before the tar, so the capture rides INSIDE the link-unit tarball —
//! one hash covers the staticlibs and the inventory they vendored, and
//! a factory release can never silently drift from it); the reader is
//! `tebako sbom --runtime-exe`, which re-emits the capture as the exe's
//! CycloneDX document. One crate owns both directions — the writer and
//! the reader can never disagree about the grammar.
//!
//! The `source` member carries the crate's cargo source classified:
//! `workspace` (cargo's null), `registry` (registry+/sparse+
//! normalized), or the verbatim `git+…` / `path+…` string. The same
//! classification feeds the CycloneDX `tebako:crate:source` property
//! mapping, so the capture and the documents never diverge on what a
//! source means.

use tebako_json::Value as Json;

/// The capture grammar version. Readers fail closed on any other value
/// (an additive change bumps nothing — extra members are tolerated;
/// a grammar change is a new schema).
pub const CAPTURE_SCHEMA: u64 = 1;

/// The named failure of capture parsing/classification (spec 15 §8a's
/// exit grammar: malformed → 65).
#[derive(Debug)]
pub enum CaptureError {
    /// The JSON itself does not parse.
    NotJson(String),
    /// `capture_schema` is missing or names an unsupported version.
    UnsupportedSchema(Option<u64>),
    /// A required member is missing or has the wrong shape.
    Malformed(String),
    /// A cargo source string no classification covers (writer side —
    /// fail closed: a new cargo source kind is a review event, never a
    /// silently property-less crate).
    UnknownSource(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson(e) => write!(f, "the crate capture is not JSON: {e}"),
            Self::UnsupportedSchema(found) => write!(
                f,
                "the crate capture's capture_schema is {} — this reader supports {CAPTURE_SCHEMA}",
                found
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "missing".to_string())
            ),
            Self::Malformed(detail) => write!(f, "the crate capture is malformed: {detail}"),
            Self::UnknownSource(source) => write!(
                f,
                "cargo source '{source}' has no capture classification (workspace | registry+ | git+ | path+)"
            ),
        }
    }
}

impl std::error::Error for CaptureError {}

impl CaptureError {
    /// The spec 15 §8a exit class (malformed → 65).
    pub fn exit_code(&self) -> i32 {
        65
    }
}

/// Classify one crate's cargo `source` string into the capture's
/// vocabulary: cargo's null (workspace member / path-in-workspace dep)
/// → `workspace`; the registry spellings → `registry`; `git+…` and
/// `path+…` carry their URLs verbatim (evidence). Anything else fails
/// closed — a silently mis-classified source would lose the property
/// the documents rely on.
pub fn classify_cargo_source(source: Option<&str>) -> Result<String, CaptureError> {
    match source {
        None => Ok("workspace".to_string()),
        Some(s) if s.starts_with("git+") => Ok(s.to_string()),
        Some(s) if s.starts_with("path+") => Ok(s.to_string()),
        Some(s) if s.starts_with("registry+") || s.starts_with("sparse+") => {
            Ok("registry".to_string())
        }
        Some(other) => Err(CaptureError::UnknownSource(other.to_string())),
    }
}

/// The CycloneDX `tebako:crate:source` properties for one capture
/// source string (the reader's mapping — the exact inverse of
/// [`classify_cargo_source`], shared by every document that describes
/// crates). Registry crates carry NO property: registry is the default
/// provenance and its absence of a claim reads correctly.
pub fn source_properties(source: &str) -> Result<Vec<(String, String)>, CaptureError> {
    const KEY: &str = "tebako:crate:source";
    match source {
        "workspace" => Ok(vec![(KEY.to_string(), "workspace".to_string())]),
        "registry" => Ok(Vec::new()),
        s if s.starts_with("git+") => Ok(vec![(KEY.to_string(), s.to_string())]),
        s if s.starts_with("path+") => Ok(vec![
            (KEY.to_string(), "path".to_string()),
            ("tebako:crate:source:url".to_string(), s.to_string()),
        ]),
        other => Err(CaptureError::UnknownSource(other.to_string())),
    }
}

/// One crate in the capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedCrate {
    pub name: String,
    pub version: String,
    /// `pkg:cargo/<name>@<version>` — captured, never re-derived by the
    /// reader (the purl mapping has exactly one author: the writer).
    pub purl: String,
    /// The Cargo.lock checksum (registry crates), lowercase hex.
    pub sha256: Option<String>,
    /// The declared SPDX license expression, when the crate declares one.
    pub license: Option<String>,
    /// The classified source (see [`classify_cargo_source`]).
    pub source: String,
}

/// One dependency edge of the resolve graph (purl space, dev edges
/// excluded at capture time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureEdge {
    pub refr: String,
    pub depends_on: Vec<String>,
}

/// The whole capture.
#[derive(Debug, Clone)]
pub struct CrateCapture {
    /// The emitting tool's (name, version) — provenance.
    pub tool: (String, String),
    /// The cargo package whose resolve closure this capture is.
    pub package: String,
    /// The rust target triple the resolution was filtered for.
    pub target: String,
    /// The root crate's purl (the document roots its graph here).
    pub root: String,
    pub components: Vec<CapturedCrate>,
    pub edges: Vec<CaptureEdge>,
}

fn opt_string(v: &Json) -> Option<String> {
    match v {
        Json::String(s) => Some(s.clone()),
        Json::Null => None,
        _ => None,
    }
}

fn required_string<'a>(v: &'a Json, key: &str, where_: &str) -> Result<&'a str, CaptureError> {
    match v.find(key) {
        Some(Json::String(s)) => Ok(s),
        _ => Err(CaptureError::Malformed(format!(
            "{where_}: missing the string member '{key}'"
        ))),
    }
}

fn required_array<'a>(v: &'a Json, key: &str, where_: &str) -> Result<&'a Vec<Json>, CaptureError> {
    match v.find(key) {
        Some(Json::Array(items)) => Ok(items),
        _ => Err(CaptureError::Malformed(format!(
            "{where_}: missing the array member '{key}'"
        ))),
    }
}

impl CrateCapture {
    /// Render the capture: the deterministic pretty JSON text (components
    /// sorted by purl, edges sorted by ref with `depends_on` sorted —
    /// the input order never leaks into the artifact), ending with a
    /// newline.
    pub fn emit(&self) -> String {
        fn s(v: &str) -> Json {
            Json::String(v.to_string())
        }
        let mut components = self.components.clone();
        components.sort_by(|a, b| a.purl.cmp(&b.purl));
        let mut edges = self.edges.clone();
        edges.sort_by(|a, b| a.refr.cmp(&b.refr));
        let document = Json::Object(vec![
            (
                "capture_schema".to_string(),
                Json::Number(CAPTURE_SCHEMA.to_string()),
            ),
            ("tool".to_string(), s(&self.tool.0)),
            ("tool_version".to_string(), s(&self.tool.1)),
            ("package".to_string(), s(&self.package)),
            ("target".to_string(), s(&self.target)),
            ("root".to_string(), s(&self.root)),
            (
                "components".to_string(),
                Json::Array(
                    components
                        .iter()
                        .map(|c| {
                            Json::Object(vec![
                                ("name".to_string(), s(&c.name)),
                                ("version".to_string(), s(&c.version)),
                                ("purl".to_string(), s(&c.purl)),
                                (
                                    "sha256".to_string(),
                                    c.sha256.as_deref().map_or(Json::Null, s),
                                ),
                                (
                                    "license".to_string(),
                                    c.license.as_deref().map_or(Json::Null, s),
                                ),
                                ("source".to_string(), s(&c.source)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "edges".to_string(),
                Json::Array(
                    edges
                        .iter()
                        .map(|e| {
                            let mut depends_on = e.depends_on.clone();
                            depends_on.sort();
                            depends_on.dedup();
                            Json::Object(vec![
                                ("ref".to_string(), s(&e.refr)),
                                (
                                    "depends_on".to_string(),
                                    Json::Array(depends_on.iter().map(|d| s(d)).collect()),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]);
        format!("{}\n", tebako_json::to_string(&document))
    }

    /// Parse a capture text. The schema version gates fail-closed; extra
    /// members are tolerated (the evolution law — an additive producer
    /// never breaks a shipped reader).
    pub fn parse(text: &str) -> Result<CrateCapture, CaptureError> {
        let document = tebako_json::parse(text).map_err(CaptureError::NotJson)?;
        let schema = document.find("capture_schema").and_then(Json::as_u64);
        if schema != Some(CAPTURE_SCHEMA) {
            return Err(CaptureError::UnsupportedSchema(schema));
        }
        let where_ = "the crate capture";
        let tool = required_string(&document, "tool", where_)?.to_string();
        let tool_version = required_string(&document, "tool_version", where_)?.to_string();
        let package = required_string(&document, "package", where_)?.to_string();
        let target = required_string(&document, "target", where_)?.to_string();
        let root = required_string(&document, "root", where_)?.to_string();
        let mut components = Vec::new();
        for (index, member) in required_array(&document, "components", where_)?
            .iter()
            .enumerate()
        {
            let at = format!("components[{index}]");
            let name = required_string(member, "name", &at)?.to_string();
            let version = required_string(member, "version", &at)?.to_string();
            let purl = required_string(member, "purl", &at)?.to_string();
            let sha256 = member.find("sha256").and_then(opt_string);
            let license = member.find("license").and_then(opt_string);
            let source = required_string(member, "source", &at)?.to_string();
            // The source string is validated HERE (fail closed at parse,
            // never at property time) — an unknown classification is a
            // broken capture, not a property-less crate.
            source_properties(&source)?;
            components.push(CapturedCrate {
                name,
                version,
                purl,
                sha256,
                license,
                source,
            });
        }
        let mut edges = Vec::new();
        for (index, member) in required_array(&document, "edges", where_)?
            .iter()
            .enumerate()
        {
            let at = format!("edges[{index}]");
            let refr = required_string(member, "ref", &at)?.to_string();
            let mut depends_on = Vec::new();
            for dep in required_array(member, "depends_on", &at)? {
                match dep {
                    Json::String(d) => depends_on.push(d.clone()),
                    _ => {
                        return Err(CaptureError::Malformed(format!(
                            "{at}: 'depends_on' carries a non-string entry"
                        )))
                    }
                }
            }
            edges.push(CaptureEdge { refr, depends_on });
        }
        if components.is_empty() {
            return Err(CaptureError::Malformed(
                "the capture carries no crate components".to_string(),
            ));
        }
        if !components.iter().any(|c| c.purl == root) {
            return Err(CaptureError::Malformed(format!(
                "the root '{root}' is not one of the captured crates"
            )));
        }
        Ok(CrateCapture {
            tool: (tool, tool_version),
            package,
            target,
            root,
            components,
            edges,
        })
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CrateCapture {
        CrateCapture {
            tool: ("sbom-cargo".to_string(), "9.9.9".to_string()),
            package: "probe".to_string(),
            target: "aarch64-testos".to_string(),
            root: "pkg:cargo/probe@2.0.0".to_string(),
            components: vec![
                CapturedCrate {
                    name: "probe".to_string(),
                    version: "2.0.0".to_string(),
                    purl: "pkg:cargo/probe@2.0.0".to_string(),
                    sha256: None,
                    license: Some("BSD-2-Clause".to_string()),
                    source: "workspace".to_string(),
                },
                CapturedCrate {
                    name: "dep".to_string(),
                    version: "1.0.0".to_string(),
                    purl: "pkg:cargo/dep@1.0.0".to_string(),
                    sha256: Some("aa".repeat(32)),
                    license: Some("MIT OR Apache-2.0".to_string()),
                    source: "registry".to_string(),
                },
                CapturedCrate {
                    name: "thing".to_string(),
                    version: "0.4.0".to_string(),
                    purl: "pkg:cargo/thing@0.4.0".to_string(),
                    sha256: None,
                    license: None,
                    source: "git+https://example.com/thing?rev=abc123".to_string(),
                },
            ],
            edges: vec![
                CaptureEdge {
                    refr: "pkg:cargo/probe@2.0.0".to_string(),
                    depends_on: vec![
                        "pkg:cargo/thing@0.4.0".to_string(),
                        "pkg:cargo/dep@1.0.0".to_string(),
                    ],
                },
                CaptureEdge {
                    refr: "pkg:cargo/dep@1.0.0".to_string(),
                    depends_on: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn emit_is_deterministic_and_sorted() {
        let capture = fixture();
        let first = capture.emit();
        assert_eq!(first, fixture().emit(), "two emits are byte-identical");
        let dep = first.find("\"purl\": \"pkg:cargo/dep@1.0.0\"").unwrap();
        let probe = first.find("\"purl\": \"pkg:cargo/probe@2.0.0\"").unwrap();
        let thing = first.find("\"purl\": \"pkg:cargo/thing@0.4.0\"").unwrap();
        assert!(dep < probe && probe < thing, "components sort by purl");
        // depends_on sorts (the fixture hands it in unsorted).
        assert!(first.contains(
            "\"depends_on\": [\n        \"pkg:cargo/dep@1.0.0\",\n        \"pkg:cargo/thing@0.4.0\"\n      ]"
        ));
        assert!(first.ends_with('\n'));
        assert!(first.contains("\"capture_schema\": 1"));
    }

    #[test]
    fn round_trip_preserves_the_model() {
        let capture = fixture();
        let parsed = CrateCapture::parse(&capture.emit()).unwrap();
        assert_eq!(parsed.tool, capture.tool);
        assert_eq!(parsed.package, capture.package);
        assert_eq!(parsed.target, capture.target);
        assert_eq!(parsed.root, capture.root);
        assert_eq!(parsed.components.len(), 3);
        assert_eq!(parsed.edges.len(), 2);
        let dep = parsed.components.iter().find(|c| c.name == "dep").unwrap();
        assert_eq!(dep.sha256.as_deref(), Some("aa".repeat(32).as_str()));
        assert_eq!(dep.source, "registry");
    }

    #[test]
    fn parse_refuses_the_wrong_schema_named() {
        let text = fixture()
            .emit()
            .replace("\"capture_schema\": 1", "\"capture_schema\": 2");
        let err = CrateCapture::parse(&text).unwrap_err();
        assert!(matches!(err, CaptureError::UnsupportedSchema(Some(2))));
        assert_eq!(err.exit_code(), 65);
        // A missing schema names the supported version too.
        let text = fixture().emit().replace("  \"capture_schema\": 1,\n", "");
        let err = CrateCapture::parse(&text).unwrap_err();
        assert!(matches!(err, CaptureError::UnsupportedSchema(None)));
    }

    #[test]
    fn parse_refuses_malformed_members_and_a_dangling_root() {
        // A component with a non-string name.
        let text = fixture()
            .emit()
            .replace("\"name\": \"probe\"", "\"name\": 7");
        assert!(matches!(
            CrateCapture::parse(&text),
            Err(CaptureError::Malformed(_))
        ));
        // A root that is none of the captured crates.
        let mut capture = fixture();
        capture.root = "pkg:cargo/absent@0.0.0".to_string();
        let err = CrateCapture::parse(&capture.emit()).unwrap_err();
        assert!(err.to_string().contains("the root"));
        // An empty component set.
        let mut capture = fixture();
        capture.components.clear();
        capture.root.clear();
        let err = CrateCapture::parse(&capture.emit()).unwrap_err();
        assert!(err.to_string().contains("no crate components"));
        // A source string with no classification fails at parse.
        let mut capture = fixture();
        capture.components[1].source = "svn+https://example.com".to_string();
        assert!(matches!(
            CrateCapture::parse(&capture.emit()),
            Err(CaptureError::UnknownSource(_))
        ));
        // Extra members are tolerated (the evolution law).
        let text = fixture().emit().replace(
            "\"capture_schema\": 1,",
            "\"capture_schema\": 1,\n  \"future_member\": {\"nested\": true},",
        );
        assert!(CrateCapture::parse(&text).is_ok());
    }

    #[test]
    fn source_classification_round_trips_the_vocabulary() {
        assert_eq!(classify_cargo_source(None).unwrap(), "workspace");
        assert_eq!(
            classify_cargo_source(Some(
                "registry+https://github.com/rust-lang/crates.io-index"
            ))
            .unwrap(),
            "registry"
        );
        assert_eq!(
            classify_cargo_source(Some("sparse+https://index.crates.io/")).unwrap(),
            "registry"
        );
        assert_eq!(
            classify_cargo_source(Some("git+https://example.com/t?rev=a")).unwrap(),
            "git+https://example.com/t?rev=a"
        );
        assert_eq!(
            classify_cargo_source(Some("path+file:///w/dep")).unwrap(),
            "path+file:///w/dep"
        );
        assert!(classify_cargo_source(Some("svn+https://example.com")).is_err());

        assert_eq!(
            source_properties("workspace").unwrap(),
            vec![("tebako:crate:source".to_string(), "workspace".to_string())]
        );
        assert!(source_properties("registry").unwrap().is_empty());
        assert_eq!(
            source_properties("git+https://example.com/t?rev=a").unwrap(),
            vec![(
                "tebako:crate:source".to_string(),
                "git+https://example.com/t?rev=a".to_string()
            )]
        );
        assert_eq!(
            source_properties("path+file:///w/dep").unwrap(),
            vec![
                ("tebako:crate:source".to_string(), "path".to_string()),
                (
                    "tebako:crate:source:url".to_string(),
                    "path+file:///w/dep".to_string()
                )
            ]
        );
        assert!(source_properties("svn+x").is_err());
    }
}
