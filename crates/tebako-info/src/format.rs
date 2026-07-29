//! Image-format reporting for the info surface (spec 15 §2/§3): the tfs
//! detection chain answers how the bytes are read; this module names the
//! answer — distinguishing dwarfs-t (FlatBuffers metadata) from upstream
//! dwarfs (Thrift metadata) — and keeps the trailer's `format_id` in its
//! place (a hint; `auto` means detect).
//!
//! The FlatBuffers/Thrift distinction: the dwarfs-t writer emits a 4-byte
//! all-zeros `METADATA_V2_SCHEMA` section for FlatBuffers metadata (the
//! schema travels inside the flatbuffer), while a Thrift-frozen image
//! carries a real schema section (dwarfs-t
//! `src/writer/internal/metadata_freezer.cpp`). The backend's
//! `image_info_json` already exposes the per-section uncompressed sizes,
//! so the flavor is read off the schema-section size.

use tpkg::{
    TPKG_FORMAT_AUTO, TPKG_FORMAT_DWARFS, TPKG_FORMAT_RUNTIME, TPKG_FORMAT_SQUASHFS,
    TPKG_FORMAT_ZIP,
};

/// The schema-section marker size emitted for FlatBuffers images.
const FLATBUFFERS_SCHEMA_MARKER: u64 = 4;

/// How an image's bytes are read (the detection chain's answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatInfo {
    /// The mounted backend's name (`DwarFS`, `SquashFS`, `ZIP`, `TAR`, …).
    pub backend: String,
    /// Short lower-case label for slot tables (`dwarfs`, `squashfs`,
    /// `zip`, `tar`, `tar.gz`, `tar.zst`).
    pub short: String,
    /// Long label for the human header
    /// (`dwarfs-t (flatbuffers metadata)`, `dwarfs (thrift metadata)`, …).
    pub label: String,
    /// Backend-level metadata JSON (dwarfs only), when the backend
    /// exposes it — the `--backend-json` payload.
    pub backend_json: Option<String>,
}

impl FormatInfo {
    /// Build from the mounted backend's name and its metadata JSON.
    pub fn detect(backend_name: &str, backend_json: Option<&str>) -> FormatInfo {
        let (short, label) = match backend_name {
            "DwarFS" => {
                let flavor = dwarfs_flavor(backend_json);
                (
                    "dwarfs".to_string(),
                    match flavor {
                        DwarfsFlavor::FlatBuffers => "dwarfs-t (flatbuffers metadata)".to_string(),
                        DwarfsFlavor::Thrift => "dwarfs (thrift metadata)".to_string(),
                    },
                )
            }
            "SquashFS" => ("squashfs".to_string(), "squashfs".to_string()),
            "ZIP" => ("zip".to_string(), "zip".to_string()),
            "TAR" => ("tar".to_string(), "tar".to_string()),
            "TAR.GZ" => ("tar.gz".to_string(), "tar.gz".to_string()),
            "TAR.ZST" => ("tar.zst".to_string(), "tar.zst".to_string()),
            other => (other.to_lowercase(), other.to_string()),
        };
        FormatInfo {
            backend: backend_name.to_string(),
            short,
            label,
            backend_json: backend_json.map(str::to_string),
        }
    }
}

/// The dwarfs metadata flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DwarfsFlavor {
    FlatBuffers,
    Thrift,
}

/// Read the flavor off the backend metadata JSON: a schema section no
/// larger than the 4-byte zero marker means FlatBuffers; a real schema
/// section means Thrift; no schema section at all means the modern
/// (FlatBuffers) layout.
fn dwarfs_flavor(backend_json: Option<&str>) -> DwarfsFlavor {
    let Some(json) = backend_json else {
        return DwarfsFlavor::FlatBuffers;
    };
    let Ok(doc) = tebako_json::parse(json) else {
        return DwarfsFlavor::FlatBuffers;
    };
    let Some(tebako_json::Value::Array(sections)) = doc.find("sections") else {
        return DwarfsFlavor::FlatBuffers;
    };
    for section in sections {
        if section.find("type").and_then(|t| t.as_string()).as_deref() == Some("METADATA_V2_SCHEMA")
        {
            let size = section
                .find("size")
                .and_then(|s| s.as_u64())
                .or_else(|| section.find("compressed_size").and_then(|s| s.as_u64()));
            return match size {
                Some(n) if n > FLATBUFFERS_SCHEMA_MARKER => DwarfsFlavor::Thrift,
                _ => DwarfsFlavor::FlatBuffers,
            };
        }
    }
    DwarfsFlavor::FlatBuffers
}

/// The `format_id` hint rendered for humans (spec 02 §6: 4 is a legacy
/// ROLE riding in the format field, reported as such).
pub fn hint_name(format_id: u32) -> &'static str {
    match format_id {
        TPKG_FORMAT_AUTO => "auto",
        TPKG_FORMAT_DWARFS => "dwarfs",
        TPKG_FORMAT_SQUASHFS => "squashfs",
        TPKG_FORMAT_ZIP => "zip",
        TPKG_FORMAT_RUNTIME => "runtime (legacy role)",
        _ => "unknown",
    }
}

/// The `format_id` hint rendered for JSON (`runtime (legacy role)` keeps
/// its role wording there too — it is never a detected format).
pub fn hint_json_name(format_id: u32) -> &'static str {
    hint_name(format_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwarfs_flavor_from_schema_section() {
        let flat = FormatInfo::detect(
            "DwarFS",
            Some(
                r#"{"sections":[{"type":"BLOCK","size":45},
                {"type":"METADATA_V2_SCHEMA","size":4,"compressed_size":4},
                {"type":"METADATA_V2","size":792}]}"#,
            ),
        );
        assert_eq!(flat.label, "dwarfs-t (flatbuffers metadata)");
        assert_eq!(flat.short, "dwarfs");

        let thrift = FormatInfo::detect(
            "DwarFS",
            Some(
                r#"{"sections":[{"type":"METADATA_V2_SCHEMA","size":1480},
                {"type":"METADATA_V2","size":792}]}"#,
            ),
        );
        assert_eq!(thrift.label, "dwarfs (thrift metadata)");

        // No sections array / no schema section / no JSON: modern default.
        for json in [
            Some(r#"{"sections":[{"type":"METADATA_V2","size":1}]}"#),
            Some(r#"{"version":2}"#),
            Some("not json"),
            None,
        ] {
            assert_eq!(
                FormatInfo::detect("DwarFS", json).label,
                "dwarfs-t (flatbuffers metadata)"
            );
        }
    }

    #[test]
    fn backend_labels() {
        assert_eq!(FormatInfo::detect("ZIP", None).short, "zip");
        assert_eq!(FormatInfo::detect("SquashFS", None).short, "squashfs");
        assert_eq!(FormatInfo::detect("TAR", None).label, "tar");
        assert_eq!(FormatInfo::detect("TAR.GZ", None).label, "tar.gz");
        assert_eq!(FormatInfo::detect("TAR.ZST", None).label, "tar.zst");
    }

    #[test]
    fn format_id_hints() {
        assert_eq!(hint_name(TPKG_FORMAT_AUTO), "auto");
        assert_eq!(hint_name(TPKG_FORMAT_DWARFS), "dwarfs");
        assert_eq!(hint_name(TPKG_FORMAT_SQUASHFS), "squashfs");
        assert_eq!(hint_name(TPKG_FORMAT_ZIP), "zip");
        assert_eq!(hint_name(TPKG_FORMAT_RUNTIME), "runtime (legacy role)");
        assert_eq!(hint_name(9), "unknown");
    }
}
