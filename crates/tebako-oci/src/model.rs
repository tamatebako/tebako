//! The spec 38 §3 artifact model: the media types, the manifest shape,
//! the annotation map, and the tag derivation — the single owner of all
//! four. Every served file is ONE OCI image-manifest artifact: the empty
//! config (`application/vnd.oci.empty.v1+json`, the canonical `{}` bytes)
//! and EXACTLY ONE layer — the file's raw bytes, so the layer digest IS
//! the artifact's sha256 (the `.sha256` sidecar's exact equivalence). A
//! manifest violating the shape (≠1 layer, a foreign `artifactType`, a
//! missing required annotation) is never read best-effort — the client
//! refuses it by name (`OciArtifactMalformed`).

use tebako_json::{parse as json_parse, Value as JsonValue};

/// The OCI image manifest media type (distribution-spec v1.1 form).
pub const MANIFEST_MT: &str = "application/vnd.oci.image.manifest.v1+json";
/// The empty config's media type.
pub const EMPTY_CONFIG_MT: &str = "application/vnd.oci.empty.v1+json";
/// The empty config's canonical digest (sha256 of `{}`).
pub const EMPTY_CONFIG_DIGEST: &str =
    "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fcd02fe2d1a42cb2d57b9b8b37d";

// The annotation keys (spec 38 §3 — annotations mirror resolution
// fields only; the in-image L1 manifest stays authoritative).
pub const ANNOTATION_TITLE: &str = "org.opencontainers.image.title";
pub const ANNOTATION_NAME: &str = "org.tebako.name";
pub const ANNOTATION_VERSION: &str = "org.tebako.version";
pub const ANNOTATION_KIND: &str = "org.tebako.kind";
pub const ANNOTATION_TRIPLET: &str = "org.tebako.triplet";
pub const ANNOTATION_ENTRYPOINTS: &str = "org.tebako.entrypoints";
pub const ANNOTATION_RUNTIME_REQUIREMENT: &str = "org.tebako.runtime-requirement";
pub const ANNOTATION_RUNTIME_SHARD: &str = "org.tebako.runtime.shard";
pub const ANNOTATION_SIGNATURE_KEYID: &str = "org.tebako.signature.keyid";
pub const ANNOTATION_SIGNATURE_SUBJECT: &str = "org.tebako.signature.subject";

/// The four artifact classes of spec 38 §3 — one artifact per served
/// file, the class self-describing via `artifactType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactClass {
    /// A payload image (`.tfs`).
    Payload,
    /// A spec-36 runtime bundle.
    RuntimeBundle,
    /// A `tpkg-registry.yaml` index.
    RegistryIndex,
    /// A detached OpenPGP signature.
    Signature,
}

impl ArtifactClass {
    /// The `artifactType` this class publishes under.
    pub fn artifact_type(self) -> &'static str {
        match self {
            ArtifactClass::Payload => "application/vnd.tebako.tfs.v1",
            ArtifactClass::RuntimeBundle => "application/vnd.tebako.runtime-bundle.v1",
            ArtifactClass::RegistryIndex => "application/vnd.tebako.registry.v1",
            ArtifactClass::Signature => "application/vnd.tebako.signature.v1",
        }
    }

    /// The layer media type of this class's single layer.
    pub fn layer_media_type(self) -> &'static str {
        match self {
            ArtifactClass::Payload => "application/vnd.tebako.tfs.v1+layer",
            ArtifactClass::RuntimeBundle => "application/vnd.tebako.runtime-bundle.v1+tar.gz",
            ArtifactClass::RegistryIndex => "application/vnd.tebako.registry.v1+yaml",
            ArtifactClass::Signature => "application/vnd.tebako.signature.v1+asc",
        }
    }

    /// The class an `artifactType` names, if it is one of the four.
    pub fn from_artifact_type(media_type: &str) -> Option<ArtifactClass> {
        [
            ArtifactClass::Payload,
            ArtifactClass::RuntimeBundle,
            ArtifactClass::RegistryIndex,
            ArtifactClass::Signature,
        ]
        .into_iter()
        .find(|c| c.artifact_type() == media_type)
    }
}

/// What a manifest read expects (spec 38 §3's shape law): a specific
/// class where the caller knows it (a registry fetch refuses a payload
/// manifest served as the registry), or the union of the four tebako
/// classes for the generic small-file fetch (the `.asc` rider).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeExpectation {
    /// Exactly this class.
    Class(ArtifactClass),
    /// Any of the four spec 38 §3 classes.
    AnyTebako,
}

/// An OCI descriptor (media type + `sha256:<hex>` digest + size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    pub media_type: String,
    /// The full `sha256:<64 hex>` digest string.
    pub digest: String,
    pub size: u64,
}

impl Descriptor {
    /// The 64-hex half of [`Descriptor::digest`].
    pub fn digest_hex(&self) -> Option<&str> {
        self.digest.strip_prefix("sha256:")
    }
}

/// The annotation map (the L3 mirror subset, spec 38 §3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Annotations {
    /// `org.opencontainers.image.title` — the served file name. The one
    /// REQUIRED annotation (every class carries it).
    pub title: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub kind: Option<String>,
    pub triplet: Option<String>,
    /// Comma-separated entrypoint names (payloads).
    pub entrypoints: Option<String>,
    /// The requirement object, JSON-encoded (payloads).
    pub runtime_requirement: Option<String>,
    /// The per-package shard JSON verbatim (runtime-bundle artifacts).
    pub runtime_shard: Option<String>,
    /// The signer's PRIMARY keyid (signature artifacts).
    pub signature_keyid: Option<String>,
    /// The digest of the signed blob (signature artifacts).
    pub signature_subject: Option<String>,
}

/// A parsed image manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub artifact_type: Option<String>,
    pub config: Descriptor,
    pub layers: Vec<Descriptor>,
    pub annotations: Annotations,
}

fn jstr<'a>(v: &'a JsonValue, key: &str) -> Option<&'a str> {
    match v.find(key) {
        Some(JsonValue::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn descriptor(v: &JsonValue, what: &str) -> Result<Descriptor, String> {
    let media_type = jstr(v, "mediaType")
        .ok_or_else(|| format!("{what} carries no mediaType"))?
        .to_string();
    let digest = jstr(v, "digest")
        .ok_or_else(|| format!("{what} carries no digest"))?
        .to_string();
    let size = v
        .find("size")
        .and_then(|s| s.as_u64())
        .ok_or_else(|| format!("{what} carries no size"))?;
    Ok(Descriptor {
        media_type,
        digest,
        size,
    })
}

impl Manifest {
    /// Parse a manifest body (structural JSON read — the §3 SHAPE law is
    /// [`Manifest::validate_shape`]).
    pub fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| format!("{e} decoding the manifest"))?;
        let doc = json_parse(text).map_err(|e| format!("invalid manifest JSON: {e}"))?;
        let mt = jstr(&doc, "mediaType");
        if mt != Some(MANIFEST_MT) {
            return Err(format!(
                "mediaType is {}, not {MANIFEST_MT}",
                mt.unwrap_or("(absent)")
            ));
        }
        let config = descriptor(
            doc.find("config").ok_or("the manifest carries no config")?,
            "the config descriptor",
        )?;
        let layers = match doc.find("layers") {
            Some(JsonValue::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, item)| descriptor(item, &format!("layer {i}")))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("the manifest carries no layers array".to_string()),
        };
        let mut annotations = Annotations::default();
        if let Some(JsonValue::Object(entries)) = doc.find("annotations") {
            for (key, value) in entries {
                let Some(value) = value.as_string() else {
                    continue;
                };
                match key.as_str() {
                    ANNOTATION_TITLE => annotations.title = Some(value),
                    ANNOTATION_NAME => annotations.name = Some(value),
                    ANNOTATION_VERSION => annotations.version = Some(value),
                    ANNOTATION_KIND => annotations.kind = Some(value),
                    ANNOTATION_TRIPLET => annotations.triplet = Some(value),
                    ANNOTATION_ENTRYPOINTS => annotations.entrypoints = Some(value),
                    ANNOTATION_RUNTIME_REQUIREMENT => annotations.runtime_requirement = Some(value),
                    ANNOTATION_RUNTIME_SHARD => annotations.runtime_shard = Some(value),
                    ANNOTATION_SIGNATURE_KEYID => annotations.signature_keyid = Some(value),
                    ANNOTATION_SIGNATURE_SUBJECT => annotations.signature_subject = Some(value),
                    _ => {}
                }
            }
        }
        Ok(Manifest {
            artifact_type: jstr(&doc, "artifactType").map(str::to_string),
            config,
            layers,
            annotations,
        })
    }

    /// The §3 shape law: the empty config, EXACTLY ONE layer, a known
    /// (or the expected) `artifactType`, the matching layer media type,
    /// and the required title annotation. Returns the layer descriptor —
    /// whose digest IS the artifact's sha256 (the trust anchor).
    pub fn validate_shape(&self, expect: ShapeExpectation) -> Result<&Descriptor, String> {
        if self.config.media_type != EMPTY_CONFIG_MT || self.config.digest != EMPTY_CONFIG_DIGEST {
            return Err(format!(
                "the config is not the canonical empty config ({EMPTY_CONFIG_MT} at {EMPTY_CONFIG_DIGEST})"
            ));
        }
        let [layer] = self.layers.as_slice() else {
            return Err(format!(
                "expected exactly one layer (the file's raw bytes), found {}",
                self.layers.len()
            ));
        };
        let artifact_type = self
            .artifact_type
            .as_deref()
            .ok_or("the manifest carries no artifactType")?;
        let class = ArtifactClass::from_artifact_type(artifact_type)
            .ok_or_else(|| format!("foreign artifactType '{artifact_type}'"))?;
        match expect {
            ShapeExpectation::Class(want) if want != class => {
                return Err(format!(
                    "expected a {} artifact ({}), the registry served {}",
                    want.artifact_type(),
                    match want {
                        ArtifactClass::Payload => "payload",
                        ArtifactClass::RuntimeBundle => "runtime-bundle",
                        ArtifactClass::RegistryIndex => "registry",
                        ArtifactClass::Signature => "signature",
                    },
                    artifact_type,
                ));
            }
            _ => {}
        }
        if layer.media_type != class.layer_media_type() {
            return Err(format!(
                "the layer media type is '{}', not {}",
                layer.media_type,
                class.layer_media_type()
            ));
        }
        match &self.annotations.title {
            Some(title) if !title.is_empty() => {}
            _ => {
                return Err(format!(
                    "the required {ANNOTATION_TITLE} annotation is missing (the served file name)"
                ))
            }
        }
        Ok(layer)
    }
}

// ---------------------------------------------------------------------
// Tag derivation (spec 38 §3 — the locked rules, this module their
// single code owner)
// ---------------------------------------------------------------------

/// The payload tag: `<version>` for universal, `<version>-<triplet>`
/// per triplet.
pub fn payload_tag(version: &str, triplet: Option<&str>) -> String {
    match triplet {
        Some(t) => format!("{version}-{t}"),
        None => version.to_string(),
    }
}

/// The signature tag: `sha256-<64 hex of the SIGNED BLOB's digest>.asc`
/// — keyed by the signed bytes, derived deterministically from the
/// layer descriptor the fetcher already holds. No listing, no guessing.
pub fn signature_tag(signed_blob_sha256: &str) -> String {
    format!("sha256-{signed_blob_sha256}.asc")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_json(artifact_type: &str, layer_mt: &str, title: Option<&str>) -> String {
        let title = title
            .map(|t| format!(r#", "annotations": {{"{ANNOTATION_TITLE}": "{t}"}}"#))
            .unwrap_or_default();
        format!(
            r#"{{"schemaVersion": 2, "mediaType": "{MANIFEST_MT}",
  "artifactType": "{artifact_type}",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{layer_mt}", "digest": "sha256:{}" , "size": 7}}]{title}}}"#,
            "a".repeat(64)
        )
    }

    #[test]
    fn a_well_formed_manifest_parses_and_validates() {
        let m = Manifest::parse(
            manifest_json(
                ArtifactClass::Payload.artifact_type(),
                ArtifactClass::Payload.layer_media_type(),
                Some("tool-1.0.tfs"),
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(m.layers.len(), 1);
        assert_eq!(m.annotations.title.as_deref(), Some("tool-1.0.tfs"));
        let layer = m
            .validate_shape(ShapeExpectation::Class(ArtifactClass::Payload))
            .unwrap();
        assert_eq!(layer.digest_hex(), Some("a".repeat(64).as_str()));
    }

    #[test]
    fn every_shape_violation_is_named() {
        let payload_mt = ArtifactClass::Payload.artifact_type();
        let layer_mt = ArtifactClass::Payload.layer_media_type();
        for (json, needle) in [
            // two layers
            (
                manifest_json(payload_mt, layer_mt, Some("x.tfs")).replacen(
                    "}]",
                    &format!(
                        "}}, {{\"mediaType\": \"{layer_mt}\", \"digest\": \"sha256:{}\", \"size\": 1}}]",
                        "b".repeat(64)
                    ),
                    1,
                ),
                "exactly one layer",
            ),
            // foreign artifactType
            (
                manifest_json("application/vnd.foreign.thing", layer_mt, Some("x.tfs")),
                "foreign artifactType",
            ),
            // the layer media type disagrees with the class
            (
                manifest_json(payload_mt, "application/octet-stream", Some("x.tfs")),
                "layer media type",
            ),
            // the required title annotation is absent
            (manifest_json(payload_mt, layer_mt, None), ANNOTATION_TITLE),
            // a non-canonical config
            (
                manifest_json(payload_mt, layer_mt, Some("x.tfs")).replace(
                    EMPTY_CONFIG_DIGEST,
                    &format!("sha256:{}", "c".repeat(64)),
                ),
                "empty config",
            ),
        ] {
            let m = Manifest::parse(json.as_bytes()).unwrap();
            let err = m
                .validate_shape(ShapeExpectation::AnyTebako)
                .unwrap_err();
            assert!(err.contains(needle), "expected '{needle}' in: {err}");
        }
        // the class expectation refuses a mismatched artifact
        let m = Manifest::parse(
            manifest_json(
                ArtifactClass::RegistryIndex.artifact_type(),
                ArtifactClass::RegistryIndex.layer_media_type(),
                Some("tpkg-registry.yaml"),
            )
            .as_bytes(),
        )
        .unwrap();
        m.validate_shape(ShapeExpectation::Class(ArtifactClass::RegistryIndex))
            .unwrap();
        let err = m
            .validate_shape(ShapeExpectation::Class(ArtifactClass::Payload))
            .unwrap_err();
        assert!(err.contains("(payload)"), "{err}");
    }

    #[test]
    fn the_annotation_map_reads_the_l3_mirror() {
        let json = format!(
            r#"{{"mediaType": "{MANIFEST_MT}", "artifactType": "{}",
  "config": {{"mediaType": "{EMPTY_CONFIG_MT}", "digest": "{EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha256:{}", "size": 7}}],
  "annotations": {{
    "{ANNOTATION_TITLE}": "metanorma-1.2.3-linux-gnu-x86_64.tfs",
    "{ANNOTATION_NAME}": "metanorma",
    "{ANNOTATION_VERSION}": "1.2.3",
    "{ANNOTATION_KIND}": "app",
    "{ANNOTATION_TRIPLET}": "x86_64-linux-gnu",
    "{ANNOTATION_ENTRYPOINTS}": "metanorma,mn2pdf",
    "{ANNOTATION_RUNTIME_REQUIREMENT}": "{{\"engine\":\"ruby\",\"constraint\":\">= 3.3\"}}",
    "unrelated.annotation": "ignored"
  }}}}"#,
            ArtifactClass::Payload.artifact_type(),
            ArtifactClass::Payload.layer_media_type(),
            "a".repeat(64)
        );
        let m = Manifest::parse(json.as_bytes()).unwrap();
        let a = &m.annotations;
        assert_eq!(a.name.as_deref(), Some("metanorma"));
        assert_eq!(a.version.as_deref(), Some("1.2.3"));
        assert_eq!(a.kind.as_deref(), Some("app"));
        assert_eq!(a.triplet.as_deref(), Some("x86_64-linux-gnu"));
        assert_eq!(a.entrypoints.as_deref(), Some("metanorma,mn2pdf"));
        assert_eq!(
            a.runtime_requirement.as_deref(),
            Some("{\"engine\":\"ruby\",\"constraint\":\">= 3.3\"}")
        );
        assert!(a.runtime_shard.is_none());
        m.validate_shape(ShapeExpectation::Class(ArtifactClass::Payload))
            .unwrap();
    }

    #[test]
    fn tag_derivation_is_the_locked_rule() {
        assert_eq!(payload_tag("1.2.3", None), "1.2.3");
        assert_eq!(
            payload_tag("1.2.3", Some("linux-gnu-x86_64")),
            "1.2.3-linux-gnu-x86_64"
        );
        assert_eq!(
            signature_tag(&"e".repeat(64)),
            format!("sha256-{}.asc", "e".repeat(64))
        );
    }
}
