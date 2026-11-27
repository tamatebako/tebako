//! tebako-sbom — the CycloneDX 1.6 document model and deterministic
//! emitter for the tebako evidence plane. One owner of the document
//! shape (the house's SSOT invariant): the spec 15 §8 verb
//! (`tebako sbom`, the payload inventory and the spec 15 §8a runtime
//! artifacts) and the spec 13 §5a release generator (`sbom-cargo`, the
//! workspace's own crate inventories) both emit through this crate.
//!
//! The crate also owns the build-time crate-inventory capture
//! ([`capture`] — `crate-inventory.json`, schema 1): the intermediate
//! that carries one cargo resolution's crate set from the build that
//! produced it (the link-unit tarball) to the runtime exe's SBOM
//! (`tebako sbom --runtime-exe`), so writer and reader can never
//! diverge on the grammar.
//!
//! The determinism law (spec 15 §8): components sort by purl, the
//! `serialNumber` is a name-based UUID (v5 shape) derived from the
//! document's identity content, and the timestamp reads
//! `SOURCE_DATE_EPOCH` when the operator pins it (the press's
//! reproducibility convention — the same variable the image writer
//! reads), the wall clock otherwise. Two runs on unchanged inputs are
//! byte-identical; [`SerialMode::Random`] is the explicit opt-out.

#![forbid(unsafe_code)]

pub mod capture;

use sha2::Digest as _;

/// The CycloneDX spec version this emitter writes.
pub const SPEC_VERSION: &str = "1.6";

/// The named failure of the emission's environment reads. Nothing in the
/// document-building paths can fail — only the operator's
/// `SOURCE_DATE_EPOCH` parse is fallible, and it fails named (spec 00
/// invariant 9), never silently to the wall clock.
#[derive(Debug)]
pub enum SbomError {
    /// `SOURCE_DATE_EPOCH` is set but is not an integer number of
    /// seconds.
    EpochNotInteger(String),
}

impl std::fmt::Display for SbomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EpochNotInteger(value) => write!(
                f,
                "SOURCE_DATE_EPOCH='{value}' is not an integer number of seconds"
            ),
        }
    }
}

impl std::error::Error for SbomError {}

impl SbomError {
    /// The spec 15 §8 exit class (manifest/grammar malformed → 65).
    pub fn exit_code(&self) -> i32 {
        65
    }
}

/// The CycloneDX component `type` (the 1.6 vocabulary, only the values
/// the evidence plane emits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Things you run (exes, app payloads, toolkits, generic native
    /// tools).
    Application,
    /// Language packages and libraries (crates, gems, wheels, preload
    /// shims).
    Library,
    /// The platform things run on (runtime/language payloads).
    Platform,
    /// Data payloads.
    Data,
    /// File containers (the link-unit tarballs).
    File,
}

impl ComponentType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Library => "library",
            Self::Platform => "platform",
            Self::Data => "data",
            Self::File => "file",
        }
    }
}

/// One component (or the document's `metadata.component`). The purl is
/// mandatory: every component carries an identity, and the bom-ref
/// derives from it (disambiguated on collision — the schema's uniqueness
/// law). All evidence fields are optional BY DECLARATION: a component
/// without a hash or license says so by omission, never by a guess.
#[derive(Debug, Clone)]
pub struct Component {
    pub ctype: ComponentType,
    pub name: String,
    pub version: String,
    pub purl: String,
    /// sha256 of the artifact, lowercase hex (CycloneDX `hashes` —
    /// the "is this the exact byte?" answer).
    pub sha256: Option<String>,
    /// The SPDX license expression, when the source declares one.
    pub license_expression: Option<String>,
    /// CycloneDX `properties` (ordered; the caller owns the order).
    pub properties: Vec<(String, String)>,
    /// Component-level `externalReferences` as (type, url) pairs (the
    /// provenance links — `attestation` for the `.asc`, `distribution`
    /// for an origin or index).
    pub external_references: Vec<(String, String)>,
}

impl Component {
    /// The minimal constructor: identity only, evidence empty.
    pub fn new(ctype: ComponentType, name: &str, version: &str, purl: &str) -> Self {
        Component {
            ctype,
            name: name.to_string(),
            version: version.to_string(),
            purl: purl.to_string(),
            sha256: None,
            license_expression: None,
            properties: Vec::new(),
            external_references: Vec::new(),
        }
    }
}

/// One edge of the CycloneDX `dependencies` graph.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub refr: String,
    pub depends_on: Vec<String>,
}

/// The document's dependency graph.
#[derive(Debug, Clone)]
pub enum DependencyGraph {
    /// The flat form (spec 15 §8): the metadata component depends on
    /// every component, every component on nothing — intra-ecosystem
    /// edges are the lockfile's knowledge, not the closure's.
    Flat,
    /// The explicit form (spec 13 §5a): every edge given, including the
    /// root edge whose `refr` is the metadata component's bom-ref. The
    /// edges render sorted by ref with each `dependsOn` sorted — the
    /// input order never leaks into the document.
    Explicit(Vec<Dependency>),
}

/// A CycloneDX `annotations[]` entry — the degraded-mode completeness
/// note is its one use today (spec 15 §8: JSON carries no comments, so
/// the note is an annotation against the subject's bom-ref).
#[derive(Debug, Clone)]
pub struct Annotation {
    pub subjects: Vec<String>,
    pub text: String,
}

/// The serial derivation.
#[derive(Debug, Clone, Copy)]
pub enum SerialMode {
    /// A name-based UUID (v5 shape) over the document's identity
    /// content — unchanged inputs reproduce the same BOM byte for byte.
    ContentDerived,
    /// A per-run v4 UUID. The entropy is the process's own sources
    /// (time, a RandomState-seeded hash of the content) — adequate for
    /// a document serial, which is an identifier, never a token.
    Random,
}

/// The full document. Build the parts, then [`Document::emit`].
#[derive(Debug, Clone)]
pub struct Document {
    /// The subject (the payload, the released asset).
    pub metadata_component: Component,
    /// `metadata.properties` (ordered).
    pub metadata_properties: Vec<(String, String)>,
    /// `metadata.tools`: the emitting tool's (name, version).
    pub tool: (String, String),
    pub components: Vec<Component>,
    pub graph: DependencyGraph,
    pub annotations: Vec<Annotation>,
    pub serial_mode: SerialMode,
    /// The timestamp pin: an explicit epoch for deterministic tests and
    /// CI (the release pipeline pins the tag commit's epoch through the
    /// env), `None` to read the environment (`SOURCE_DATE_EPOCH`, else
    /// the wall clock).
    pub epoch_override: Option<u64>,
}

// ---------------------------------------------------------------------
// Serial + timestamp (the determinism law)
// ---------------------------------------------------------------------

/// Format 16 digest bytes as a UUID string with the given version nibble
/// (4 = random, 5 = name-based) and the RFC 4122 variant bits.
fn format_uuid(mut bytes: [u8; 16], version: u8) -> String {
    bytes[6] = (bytes[6] & 0x0f) | (version << 4);
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The identity content the serial derives from: the subject's bom-ref
/// and metadata properties plus every component's bom-ref/hash, in the
/// document's own (sorted) order.
fn content_serial(
    metadata: &Component,
    properties: &[(String, String)],
    refs: &[(&str, Option<&str>)],
) -> String {
    let mut canonical = String::new();
    canonical.push_str(&metadata.purl);
    canonical.push('\n');
    for (key, value) in properties {
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        canonical.push('\n');
    }
    for (refr, sha) in refs {
        canonical.push_str(refr);
        canonical.push(' ');
        canonical.push_str(sha.unwrap_or("-"));
        canonical.push('\n');
    }
    let digest = sha2::Sha256::digest(canonical.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    format!("urn:uuid:{}", format_uuid(bytes, 5))
}

fn random_serial(seed: &str) -> String {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write(seed.as_bytes());
    let mixed = hasher.finish();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);
    bytes[8..].copy_from_slice(&mixed.to_le_bytes());
    let digest = sha2::Sha256::digest(bytes);
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    format!("urn:uuid:{}", format_uuid(out, 4))
}

/// RFC 3339 UTC rendering of a Unix epoch (no chrono in the tree — the
/// civil-from-days algorithm is Howard Hinnant's, the same algorithm the
/// CLI's press stamps render).
fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// The document timestamp: the explicit pin when given,
/// `SOURCE_DATE_EPOCH` when the operator set it, the wall clock
/// otherwise. A malformed env value is the named error — a
/// reproducibility pin that cannot parse must never silently become
/// the wall clock.
fn resolve_epoch(epoch_override: Option<u64>) -> Result<u64, SbomError> {
    match epoch_override {
        Some(secs) => Ok(secs),
        None => match std::env::var("SOURCE_DATE_EPOCH") {
            Ok(value) => value
                .trim()
                .parse::<u64>()
                .map_err(|_| SbomError::EpochNotInteger(value)),
            Err(_) => Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)),
        },
    }
}

// ---------------------------------------------------------------------
// Emission
// ---------------------------------------------------------------------

fn properties_json(pairs: &[(String, String)]) -> tebako_json::Value {
    use tebako_json::Value as Json;
    Json::Array(
        pairs
            .iter()
            .map(|(name, value)| {
                Json::Object(vec![
                    ("name".to_string(), Json::String(name.clone())),
                    ("value".to_string(), Json::String(value.clone())),
                ])
            })
            .collect(),
    )
}

fn hashes_json(sha256: &str) -> tebako_json::Value {
    use tebako_json::Value as Json;
    Json::Array(vec![Json::Object(vec![
        ("alg".to_string(), Json::String("SHA-256".to_string())),
        ("content".to_string(), Json::String(sha256.to_string())),
    ])])
}

fn component_json(c: &Component, bom_ref: &str) -> tebako_json::Value {
    use tebako_json::Value as Json;
    let s = |v: &str| Json::String(v.to_string());
    let mut obj = vec![
        ("type".to_string(), s(c.ctype.as_str())),
        ("bom-ref".to_string(), s(bom_ref)),
        ("name".to_string(), s(&c.name)),
        ("version".to_string(), s(&c.version)),
        ("purl".to_string(), s(&c.purl)),
    ];
    if let Some(sha) = &c.sha256 {
        obj.push(("hashes".to_string(), hashes_json(sha)));
    }
    if let Some(expression) = &c.license_expression {
        obj.push((
            "licenses".to_string(),
            Json::Array(vec![Json::Object(vec![(
                "expression".to_string(),
                s(expression),
            )])]),
        ));
    }
    if !c.properties.is_empty() {
        obj.push(("properties".to_string(), properties_json(&c.properties)));
    }
    if !c.external_references.is_empty() {
        obj.push((
            "externalReferences".to_string(),
            Json::Array(
                c.external_references
                    .iter()
                    .map(|(ty, url)| {
                        Json::Object(vec![
                            ("type".to_string(), s(ty)),
                            ("url".to_string(), s(url)),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    Json::Object(obj)
}

impl Document {
    /// Render the document: the deterministic pretty JSON text, ending
    /// with a newline.
    pub fn emit(&self) -> Result<String, SbomError> {
        use tebako_json::Value as Json;
        let s = |v: &str| Json::String(v.to_string());

        // -- bom-refs: the purl itself, disambiguated on collision by a
        //    deterministic `#artifact-N` suffix (two artifacts of one
        //    identity — different platform wheels, say — keep canonical
        //    purls; only the ref disambiguates).
        let mut components = self.components.clone();
        components.sort_by(|a, b| a.purl.cmp(&b.purl).then(a.sha256.cmp(&b.sha256)));
        let mut refs: Vec<String> = Vec::with_capacity(components.len());
        {
            let mut seen: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for c in &components {
                let count = seen.entry(&c.purl).or_insert(0);
                *count += 1;
                refs.push(if *count == 1 {
                    c.purl.clone()
                } else {
                    format!("{}#artifact-{}", c.purl, count)
                });
            }
        }

        let timestamp = rfc3339_utc(resolve_epoch(self.epoch_override)?);
        let seed_content: Vec<(&str, Option<&str>)> = components
            .iter()
            .zip(&refs)
            .map(|(c, r)| (r.as_str(), c.sha256.as_deref()))
            .collect();
        let serial = match self.serial_mode {
            SerialMode::ContentDerived => content_serial(
                &self.metadata_component,
                &self.metadata_properties,
                &seed_content,
            ),
            SerialMode::Random => {
                let seed = format!("{}\n{}", self.metadata_component.purl, timestamp);
                random_serial(&seed)
            }
        };

        // -- metadata
        let mut metadata = vec![
            ("timestamp".to_string(), s(&timestamp)),
            (
                "tools".to_string(),
                Json::Object(vec![(
                    "components".to_string(),
                    Json::Array(vec![Json::Object(vec![
                        ("type".to_string(), s("application")),
                        ("name".to_string(), s(&self.tool.0)),
                        ("version".to_string(), s(&self.tool.1)),
                    ])]),
                )]),
            ),
            (
                "component".to_string(),
                component_json(&self.metadata_component, &self.metadata_component.purl),
            ),
        ];
        if !self.metadata_properties.is_empty() {
            metadata.push((
                "properties".to_string(),
                properties_json(&self.metadata_properties),
            ));
        }

        // -- dependencies (both forms render sorted: ref order and
        //    dependsOn order never leak input order into the document)
        let mut edges: Vec<Dependency> = match &self.graph {
            DependencyGraph::Flat => std::iter::once(Dependency {
                refr: self.metadata_component.purl.clone(),
                depends_on: refs.clone(),
            })
            .chain(refs.iter().map(|r| Dependency {
                refr: r.clone(),
                depends_on: Vec::new(),
            }))
            .collect(),
            DependencyGraph::Explicit(given) => given.clone(),
        };
        edges.sort_by(|a, b| a.refr.cmp(&b.refr));
        for edge in &mut edges {
            edge.depends_on.sort();
            edge.depends_on.dedup();
        }

        // -- the document
        let mut document = vec![
            ("bomFormat".to_string(), s("CycloneDX")),
            ("specVersion".to_string(), s(SPEC_VERSION)),
            ("serialNumber".to_string(), s(&serial)),
            ("version".to_string(), Json::Number("1".to_string())),
            ("metadata".to_string(), Json::Object(metadata)),
            (
                "components".to_string(),
                Json::Array(
                    components
                        .iter()
                        .zip(&refs)
                        .map(|(c, r)| component_json(c, r))
                        .collect(),
                ),
            ),
            (
                "dependencies".to_string(),
                Json::Array(
                    edges
                        .iter()
                        .map(|edge| {
                            Json::Object(vec![
                                ("ref".to_string(), s(&edge.refr)),
                                (
                                    "dependsOn".to_string(),
                                    Json::Array(edge.depends_on.iter().map(|d| s(d)).collect()),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ];
        if !self.annotations.is_empty() {
            document.push((
                "annotations".to_string(),
                Json::Array(
                    self.annotations
                        .iter()
                        .map(|a| {
                            Json::Object(vec![
                                (
                                    "subjects".to_string(),
                                    Json::Array(
                                        a.subjects.iter().map(|subject| s(subject)).collect(),
                                    ),
                                ),
                                (
                                    "annotator".to_string(),
                                    Json::Object(vec![(
                                        "component".to_string(),
                                        Json::Object(vec![
                                            ("type".to_string(), s("application")),
                                            ("name".to_string(), s(&self.tool.0)),
                                            ("version".to_string(), s(&self.tool.1)),
                                        ]),
                                    )]),
                                ),
                                ("timestamp".to_string(), s(&timestamp)),
                                ("text".to_string(), s(&a.text)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }

        Ok(format!(
            "{}\n",
            tebako_json::to_string(&Json::Object(document))
        ))
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_document() -> Document {
        let mut subject = Component::new(
            ComponentType::Application,
            "probe-app",
            "1.2.3",
            "pkg:generic/probe-app@1.2.3",
        );
        subject.sha256 = Some("00".repeat(32));
        subject.properties = vec![("tebako:payload:kind".to_string(), "app".to_string())];
        let mut dep = Component::new(
            ComponentType::Library,
            "rack",
            "3.1.0",
            "pkg:gem/rack@3.1.0",
        );
        dep.sha256 = Some("11".repeat(32));
        dep.license_expression = Some("MIT".to_string());
        let mut dep2 = Component::new(
            ComponentType::Library,
            "rake",
            "13.2.1",
            "pkg:gem/rake@13.2.1",
        );
        dep2.sha256 = Some("22".repeat(32));
        Document {
            metadata_component: subject,
            metadata_properties: vec![("tebako:closure:complete".to_string(), "true".to_string())],
            tool: ("tebako".to_string(), "9.9.9".to_string()),
            components: vec![dep2, dep], // unsorted on purpose — the emit sorts
            graph: DependencyGraph::Flat,
            annotations: Vec::new(),
            serial_mode: SerialMode::ContentDerived,
            epoch_override: Some(1_700_000_000),
        }
    }

    #[test]
    fn emit_is_deterministic_byte_for_byte() {
        let doc = tiny_document();
        let first = doc.emit().unwrap();
        let second = doc.emit().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn emit_golden_minimal_document() {
        let json = tiny_document().emit().unwrap();
        let golden = r#"{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "serialNumber": "SERIAL",
  "version": 1,
  "metadata": {
    "timestamp": "2023-11-14T22:13:20Z",
    "tools": {
      "components": [
        {
          "type": "application",
          "name": "tebako",
          "version": "9.9.9"
        }
      ]
    },
    "component": {
      "type": "application",
      "bom-ref": "pkg:generic/probe-app@1.2.3",
      "name": "probe-app",
      "version": "1.2.3",
      "purl": "pkg:generic/probe-app@1.2.3",
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "0000000000000000000000000000000000000000000000000000000000000000"
        }
      ],
      "properties": [
        {
          "name": "tebako:payload:kind",
          "value": "app"
        }
      ]
    },
    "properties": [
      {
        "name": "tebako:closure:complete",
        "value": "true"
      }
    ]
  },
  "components": [
    {
      "type": "library",
      "bom-ref": "pkg:gem/rack@3.1.0",
      "name": "rack",
      "version": "3.1.0",
      "purl": "pkg:gem/rack@3.1.0",
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "1111111111111111111111111111111111111111111111111111111111111111"
        }
      ],
      "licenses": [
        {
          "expression": "MIT"
        }
      ]
    },
    {
      "type": "library",
      "bom-ref": "pkg:gem/rake@13.2.1",
      "name": "rake",
      "version": "13.2.1",
      "purl": "pkg:gem/rake@13.2.1",
      "hashes": [
        {
          "alg": "SHA-256",
          "content": "2222222222222222222222222222222222222222222222222222222222222222"
        }
      ]
    }
  ],
  "dependencies": [
    {
      "ref": "pkg:gem/rack@3.1.0",
      "dependsOn": []
    },
    {
      "ref": "pkg:gem/rake@13.2.1",
      "dependsOn": []
    },
    {
      "ref": "pkg:generic/probe-app@1.2.3",
      "dependsOn": [
        "pkg:gem/rack@3.1.0",
        "pkg:gem/rake@13.2.1"
      ]
    }
  ]
}
"#;
        // The serial is content-derived — assert the whole document
        // modulo the serial line, then pin the serial's own stability
        // below.
        let normalized: String = json
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("\"serialNumber\"") {
                    "  \"serialNumber\": \"SERIAL\","
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(normalized, golden);
    }

    #[test]
    fn serial_is_content_derived_and_stable() {
        let doc = tiny_document();
        let serial = |d: &Document| {
            let json = d.emit().unwrap();
            json.lines()
                .find(|l| l.contains("serialNumber"))
                .unwrap()
                .to_string()
        };
        let first = serial(&doc);
        assert_eq!(first, serial(&doc));
        // A content change (one component's hash) changes the serial.
        let mut changed = tiny_document();
        changed.components[0].sha256 = Some("ff".repeat(32));
        assert_ne!(first, serial(&changed));
        // The random opt-out differs run to run (two calls) and never
        // reproduces the content serial.
        let mut random = tiny_document();
        random.serial_mode = SerialMode::Random;
        let r1 = serial(&random);
        let r2 = serial(&random);
        assert_ne!(r1, r2);
        assert_ne!(r1, first);
    }

    #[test]
    fn colliding_purls_get_disambiguated_refs() {
        let mut doc = tiny_document();
        // Duplicate the rake component (components[0] is the
        // intentionally-unsorted rake row) — identical purl AND hash, so
        // the second gets the disambiguating ref.
        let dup = doc.components[0].clone();
        doc.components.push(dup);
        let json = doc.emit().unwrap();
        assert!(json.contains("\"bom-ref\": \"pkg:gem/rake@13.2.1\""));
        assert!(json.contains("\"bom-ref\": \"pkg:gem/rake@13.2.1#artifact-2\""));
    }

    #[test]
    fn explicit_graph_sorts_edges_and_depends_on() {
        let mut doc = tiny_document();
        doc.graph = DependencyGraph::Explicit(vec![
            Dependency {
                refr: "pkg:gem/rake@13.2.1".to_string(),
                depends_on: vec![
                    "pkg:gem/rack@3.1.0".to_string(),
                    "pkg:gem/rack@3.1.0".to_string(),
                ],
            },
            Dependency {
                refr: "pkg:generic/probe-app@1.2.3".to_string(),
                depends_on: vec![
                    "pkg:gem/rake@13.2.1".to_string(),
                    "pkg:gem/rack@3.1.0".to_string(),
                ],
            },
        ]);
        let json = doc.emit().unwrap();
        let deps = json.split("\"dependencies\"").nth(1).unwrap();
        let rake = deps.find("\"ref\": \"pkg:gem/rake@13.2.1\"").unwrap();
        let root = deps
            .find("\"ref\": \"pkg:generic/probe-app@1.2.3\"")
            .unwrap();
        assert!(rake < root, "edges sort by ref");
        let rake_edge = deps
            .split("\"ref\": \"pkg:gem/rake@13.2.1\"")
            .nth(1)
            .unwrap();
        assert!(rake_edge.contains("\"dependsOn\": [\n        \"pkg:gem/rack@3.1.0\"\n      ]"));
    }

    #[test]
    fn epoch_override_and_env_shape() {
        // The override pins the timestamp exactly.
        let json = tiny_document().emit().unwrap();
        assert!(json.contains("\"timestamp\": \"2023-11-14T22:13:20Z\""));
        // A malformed env value is the named error, never a silent
        // wall-clock fallthrough.
        std::env::set_var("SOURCE_DATE_EPOCH", "not-an-integer");
        let mut doc = tiny_document();
        doc.epoch_override = None;
        let err = doc.emit().unwrap_err();
        assert!(matches!(err, SbomError::EpochNotInteger(_)));
        assert_eq!(err.exit_code(), 65);
        std::env::remove_var("SOURCE_DATE_EPOCH");
        assert!(doc.emit().is_ok());
    }

    #[test]
    fn annotations_render_against_the_subject() {
        let mut doc = tiny_document();
        doc.annotations = vec![Annotation {
            subjects: vec!["pkg:generic/probe-app@1.2.3".to_string()],
            text: "degraded inventory".to_string(),
        }];
        let json = doc.emit().unwrap();
        assert!(json.contains("\"annotations\""));
        assert!(json.contains("\"text\": \"degraded inventory\""));
        assert!(json.contains("\"subjects\": [\n        \"pkg:generic/probe-app@1.2.3\"\n      ]"));
    }
}
