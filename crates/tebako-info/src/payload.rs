//! Inspection of one payload image, whole-file (`tfs info`) or a region
//! of a package (`tebako-pkg info --slot` / `--full`): mount in place via
//! the tfs detection chain (nothing is extracted), read
//! `/__tpkg__/manifest.yaml` when present, parse it (leniently — display
//! paths never fail on an invalid manifest; `--verify` grades it), and
//! compute the derived facts.

use std::path::Path;

use tpkg::PayloadManifest;

use crate::derived::{derive, Derived};
use crate::format::FormatInfo;
use crate::{err, InfoError};

/// The in-image manifest path, backend-relative (no leading slash).
const MANIFEST_BACKEND_PATH: &str = "__tpkg__/manifest.yaml";

/// Sanity bound on the in-image manifest size.
const MANIFEST_MAX: u64 = 1 << 20;

/// The inspection of one payload image. Every field is a named state —
/// nothing here panics on malformed input.
#[derive(Debug)]
pub struct PayloadInspection {
    /// How the artifact is named in output (path, or `path[slot N]`).
    pub path_display: String,
    /// Image size in bytes (the region size for slots).
    pub size_bytes: u64,
    /// Format detection (None when the image would not mount).
    pub format: Option<FormatInfo>,
    /// Named mount failure (unreadable/unsupported image).
    pub mount_error: Option<String>,
    /// The raw manifest text (when a manifest file exists).
    pub manifest_text: Option<String>,
    /// The parsed model (structure ok; validation state is separate).
    pub manifest: Option<PayloadManifest>,
    /// validate() failure for a structurally-parsed manifest, or the YAML
    /// structural error when the document does not match the model.
    pub manifest_validation: Option<String>,
    /// Why there is no manifest (plain image, unreadable file, …).
    pub manifest_note: Option<String>,
    /// Derived facts (only for a fully valid manifest).
    pub derived: Option<Derived>,
}

/// Read the whole manifest file from the mounted backend.
fn read_manifest_file(backend: &dyn tfs::Backend) -> Result<Option<String>, InfoError> {
    let st = match backend.stat(MANIFEST_BACKEND_PATH) {
        Ok(st) => st,
        Err(_) => return Ok(None), // ENOENT and friends: absent
    };
    if st.entry_type != tfs::EntryType::File {
        return Err(err(format!(
            "payload manifest {MANIFEST_BACKEND_PATH} is not a regular file"
        )));
    }
    let size = u64::try_from(st.size).map_err(|_| err("payload manifest has a negative size"))?;
    if size > MANIFEST_MAX {
        return Err(err(format!(
            "payload manifest exceeds {} bytes",
            MANIFEST_MAX
        )));
    }
    let mut buf = vec![0u8; size as usize];
    let mut off = 0u64;
    while off < size {
        let n = backend
            .pread(MANIFEST_BACKEND_PATH, &mut buf[off as usize..], off)
            .map_err(|e| err(format!("cannot read the payload manifest (errno {e})")))?;
        if n == 0 {
            return Err(err("short read on the payload manifest"));
        }
        off += n as u64;
    }
    let text = String::from_utf8(buf)
        .map_err(|_| err("payload manifest is not valid UTF-8".to_string()))?;
    Ok(Some(text))
}

/// Parse the manifest leniently: structure first, then validate(). The
/// display paths report the state; `--verify` grades it.
fn parse_manifest(text: &str, p: &mut PayloadInspection) {
    match serde_yml::from_str::<PayloadManifest>(text) {
        Ok(m) => {
            if let Err(e) = m.validate() {
                p.manifest_validation = Some(e.to_string());
            } else {
                p.derived = Some(derive(&m));
            }
            p.manifest = Some(m);
        }
        Err(e) => {
            p.manifest_validation = Some(format!("payload manifest yaml error: {e}"));
        }
    }
}

fn inspect_mounted(
    path: &Path,
    region: Option<(u64, u64)>,
    size_bytes: u64,
    display: String,
) -> Result<PayloadInspection, InfoError> {
    let mut p = PayloadInspection {
        path_display: display,
        size_bytes,
        format: None,
        mount_error: None,
        manifest_text: None,
        manifest: None,
        manifest_validation: None,
        manifest_note: None,
        derived: None,
    };

    let mounted = match region {
        None => tfs::mount::build_from_file(&path.to_string_lossy(), "/mnt"),
        Some((offset, size)) => {
            tfs::mount::build_from_file_at(&path.to_string_lossy(), offset, size, "/mnt")
        }
    };
    let mount = match mounted {
        Ok(m) => m,
        Err(errno) => {
            let message = std::ffi::CStr::from_bytes_until_nul(tfs::errno::strerror(errno))
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "unknown error".to_string());
            p.mount_error = Some(format!("cannot mount the image (errno {errno}): {message}"));
            p.manifest_note = Some("unreadable (image does not mount)".to_string());
            return Ok(p);
        }
    };

    let backend = mount.backend;
    let backend_json = backend.image_info_json();
    p.format = Some(FormatInfo::detect(
        backend.name().to_string_lossy().as_ref(),
        backend_json.as_deref(),
    ));

    match read_manifest_file(&*backend)? {
        Some(text) => {
            p.manifest_text = Some(text.clone());
            parse_manifest(&text, &mut p);
        }
        None => {
            p.manifest_note = Some("none (no /__tpkg__/manifest.yaml — plain image)".to_string());
        }
    }
    Ok(p)
}

/// Inspect a standalone image file.
pub fn inspect_image(path: &Path) -> Result<PayloadInspection, InfoError> {
    let meta = std::fs::metadata(path)
        .map_err(|e| err(format!("{}: cannot read file ({e})", path.display())))?;
    let size = meta.len();
    inspect_mounted(path, None, size, path.display().to_string())
}

/// Inspect `size` bytes at `offset` of `path` (a package slot) — the tfs
/// mount-from-region; nothing is extracted.
pub fn inspect_region(
    path: &Path,
    offset: u64,
    size: u64,
    display: String,
) -> Result<PayloadInspection, InfoError> {
    inspect_mounted(path, Some((offset, size)), size, display)
}

// ---------------------------------------------------------------------
// JSON document (spec 15 §6)
// ---------------------------------------------------------------------

fn derived_json(d: &Derived) -> tebako_json::Value {
    use tebako_json::Value as Json;
    let s = |v: &str| Json::String(v.to_string());
    let compat = d
        .runtime_compat
        .iter()
        .map(|c| match c {
            crate::RuntimeCompat::SatisfiedBy { entry } => Json::Object(vec![
                ("state".to_string(), s("satisfied-by")),
                ("entry".to_string(), s(entry)),
            ]),
            crate::RuntimeCompat::RequiresDownload { requirement } => Json::Object(vec![
                ("state".to_string(), s("requires-download")),
                ("requirement".to_string(), s(requirement)),
            ]),
            crate::RuntimeCompat::Incompatible { reason } => Json::Object(vec![
                ("state".to_string(), s("incompatible")),
                ("reason".to_string(), s(reason)),
            ]),
        })
        .collect();
    Json::Object(vec![
        (
            "shims".to_string(),
            Json::Array(d.shims.iter().map(|x| s(x)).collect()),
        ),
        ("runtime_compat".to_string(), Json::Array(compat)),
        (
            "dependency_names".to_string(),
            Json::Array(d.dependency_names.iter().map(|x| s(x)).collect()),
        ),
    ])
}

/// The payload image as one JSON document (`"info_schema": 1`; spec 15
/// §6 keys `artifact`, `manifest`, `derived`). `with_backend` folds the
/// backend metadata JSON in (`--backend-json` combined with `--json`).
pub fn payload_json(p: &PayloadInspection, with_backend: bool) -> tebako_json::Value {
    use tebako_json::Value as Json;
    let s = |v: &str| Json::String(v.to_string());
    let mut out: Vec<(String, Json)> = vec![
        (
            "info_schema".to_string(),
            Json::Number(crate::INFO_SCHEMA.to_string()),
        ),
        (
            "artifact".to_string(),
            Json::Object(vec![
                ("path".to_string(), s(&p.path_display)),
                ("kind".to_string(), s("image")),
                ("size".to_string(), Json::Number(p.size_bytes.to_string())),
            ]),
        ),
    ];
    if let Some(f) = &p.format {
        out.push((
            "format".to_string(),
            Json::Object(vec![
                ("backend".to_string(), s(&f.backend)),
                ("short".to_string(), s(&f.short)),
                ("label".to_string(), s(&f.label)),
            ]),
        ));
    }
    if let Some(err) = &p.mount_error {
        out.push(("mount_error".to_string(), s(err)));
    }
    if let Some(m) = &p.manifest {
        out.push((
            "manifest".to_string(),
            crate::manifest_json::manifest_to_json(m),
        ));
        if let Some(err) = &p.manifest_validation {
            out.push(("manifest_validation".to_string(), s(err)));
        }
    } else if let Some(err) = &p.manifest_validation {
        out.push(("manifest_error".to_string(), s(err)));
    } else if let Some(note) = &p.manifest_note {
        out.push(("manifest_note".to_string(), s(note)));
    }
    if let Some(d) = &p.derived {
        out.push(("derived".to_string(), derived_json(d)));
    }
    if with_backend {
        if let Some(f) = &p.format {
            if let Some(json) = &f.backend_json {
                if let Ok(parsed) = tebako_json::parse(json) {
                    out.push(("backend".to_string(), parsed));
                }
            }
        }
    }
    Json::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a dwarfs-t image carrying `files` (path, content) via the
    /// in-process writer (the same path `tfs mkimage` takes).
    fn mkimage(dir: &Path, files: &[(&str, &[u8])], out: &Path) {
        for (name, content) in files {
            let dest = dir.join(name);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(dest, content).unwrap();
        }
        let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default()).unwrap();
        writer.add_tree(dir, "/").unwrap();
        writer.write(out).unwrap();
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-info-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    use std::path::PathBuf;

    #[test]
    fn payload_json_shape() {
        let dir = scratch("json");
        let img = dir.join("app.tfs");
        mkimage(
            &dir.join("src"),
            &[(
                "__tpkg__/manifest.yaml",
                include_bytes!("../../tpkg/tests/fixtures/manifests/app-suite.yaml").as_slice(),
            )],
            &img,
        );
        let p = inspect_image(&img).unwrap();
        let j = payload_json(&p, false);
        assert_eq!(
            j.find("info_schema").unwrap().as_u64(),
            Some(crate::INFO_SCHEMA as u64)
        );
        assert_eq!(
            j.find("artifact")
                .unwrap()
                .find("kind")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("image")
        );
        assert!(j.find("manifest").is_some(), "manifest must be present");
        let d = j.find("derived").unwrap();
        let tebako_json::Value::Array(shims) = d.find("shims").unwrap() else {
            panic!("shims must be an array");
        };
        assert_eq!(shims.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plain_image_reports_the_named_note() {
        let dir = scratch("plain");
        let img = dir.join("plain.tfs");
        mkimage(&dir.join("src"), &[("hello.txt", b"hi")], &img);
        let p = inspect_image(&img).unwrap();
        assert!(p.mount_error.is_none());
        assert_eq!(
            p.format.as_ref().map(|f| f.label.as_str()),
            Some("dwarfs-t (flatbuffers metadata)")
        );
        assert!(p.manifest.is_none());
        assert!(p.manifest_note.as_deref().unwrap().contains("plain image"));
        assert!(p.derived.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_image_parses_and_derives() {
        let dir = scratch("manifest");
        let img = dir.join("app.tfs");
        mkimage(
            &dir.join("src"),
            &[(
                "__tpkg__/manifest.yaml",
                include_bytes!("../../tpkg/tests/fixtures/manifests/app-suite.yaml").as_slice(),
            )],
            &img,
        );
        let p = inspect_image(&img).unwrap();
        let m = p.manifest.as_ref().expect("manifest must parse");
        assert_eq!(m.identity.name, "metanorma");
        assert!(p.manifest_validation.is_none());
        let d = p.derived.as_ref().unwrap();
        assert_eq!(d.shims, vec!["metanorma", "metanorma-nokogiri"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_manifest_is_a_named_state_not_a_panic() {
        let dir = scratch("bad");
        let img = dir.join("bad.tfs");
        mkimage(
            &dir.join("src"),
            &[("__tpkg__/manifest.yaml", b"not: [valid: yaml".as_slice())],
            &img,
        );
        let p = inspect_image(&img).unwrap();
        assert!(p.manifest.is_none());
        assert!(p.manifest_validation.as_deref().unwrap().contains("yaml"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unmountable_image_is_a_named_state() {
        let dir = scratch("junk");
        let img = dir.join("junk.tfs");
        std::fs::write(&img, b"not an image at all").unwrap();
        let p = inspect_image(&img).unwrap();
        assert!(p.format.is_none());
        assert!(p.mount_error.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
