//! The platforms document model (spec 27 §3) — authored YAML, the
//! triplet → runner + asset mapping the workflow's matrix is generated
//! from. No JSON Schema artifact in this revision: this serde model is the
//! structural gate, and `schema_version` reserves the versioning slot.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformFile {
    pub schema_version: u32,
    pub packed_mn: PackedMn,
    /// Keyed by the release workflow's triplet spelling (linux-gnu-x86_64,
    /// …, windows-ucrt64) so a benchmark row joins a release row directly.
    pub triplets: BTreeMap<String, Triplet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedMn {
    /// GitHub `owner/repo` of the packed-mn releases.
    pub repo: String,
    /// The release tag. packed-mn tags track metanorma-cli: this tag IS the
    /// v1 arm's metanorma version (the accepted, documented skew).
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Triplet {
    /// The CI runner label (the matrix's runs-on).
    pub runner: String,
    /// musl legs: the image the leg builds and runs inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// The packed-mn asset for the v1-exe arm. `null` is not a skip: one
    /// explicit `unavailable` row per workload (named gaps, invariant 9).
    pub v1_asset: Option<String>,
    /// Whether the suite's payload is published for this triplet. `false`
    /// gates both v2 arms to named-gap rows.
    pub v2_payload: bool,
    /// e.g. "aibika-packed" for the Windows old world — surfaced in reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v1_note: Option<String>,
}

impl PlatformFile {
    pub fn from_yaml(text: &str) -> Result<Self, serde_yml::Error> {
        serde_yml::from_str(text)
    }

    pub fn semantic_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.schema_version != 1 {
            violations.push(format!(
                "schema_version: expected 1, got {}",
                self.schema_version
            ));
        }
        violations
    }
}
