//! `tebako blksum` — the ONE producer of an image's lazy-mount digest
//! sidecar (spec 39 §3) outside `tebako publish`.
//!
//! The sidecar anchors group-wise fetching: the loader verifies it
//! against the release row's pin BEFORE the first range read, and
//! `tpkg::lazy::Blksum` is the model of record for its contents. A
//! factory that presses an image and publishes it through its own tooling
//! must therefore derive the sidecar from the SERVED bytes through this
//! module — never through a second implementation of the group scheme.

use std::path::{Path, PathBuf};

use crate::error::TebakoError;

const EX_USAGE: i32 = 64;
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_IO: i32 = 74;

/// Derive the sidecar for `image` and write it (to `out` when given, else
/// beside the image), mapping the shared emitter's named errors onto this
/// CLI's exit classes.
pub fn write_sidecar(image: &Path, out: Option<&Path>) -> Result<tpkg::lazy::Sidecar, TebakoError> {
    tpkg::lazy::write_sidecar(image, out).map_err(|e| {
        let code = match e {
            tpkg::lazy::LazyError::BlksumInvalid(_) => EX_TEBAKO_MANIFEST,
            _ => EX_TEBAKO_IO,
        };
        TebakoError::new(e.to_string(), code)
    })
}

/// `tebako blksum <image> [--out <path>] [--json]`.
///
/// The progress line is the `.sha256` convention so a factory can append
/// it to its own `SHA256SUMS`-shaped metadata; `--json` emits the full
/// record for a shard renderer.
pub fn run(args: &[String]) -> Result<(), TebakoError> {
    let mut image: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| TebakoError::new("--out needs a path", EX_USAGE))?;
                out = Some(PathBuf::from(value));
                i += 2;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            flag if flag.starts_with('-') => {
                return Err(TebakoError::new(
                    format!("unknown option '{flag}' (usage: tebako blksum <image> [--out <path>] [--json])"),
                    EX_USAGE,
                ));
            }
            value => {
                if image.is_some() {
                    return Err(TebakoError::new(
                        "tebako blksum takes exactly one image",
                        EX_USAGE,
                    ));
                }
                image = Some(PathBuf::from(value));
                i += 1;
            }
        }
    }
    let image = image.ok_or_else(|| {
        TebakoError::new(
            "usage: tebako blksum <image> [--out <path>] [--json]",
            EX_USAGE,
        )
    })?;
    let report = write_sidecar(&image, out.as_deref())?;
    if json {
        println!(
            "{{\"filename\":\"{}\",\"sha256\":\"{}\",\"size_bytes\":{},\"group_count\":{},\"path\":\"{}\"}}",
            tebako_json::escape(&report.filename),
            report.sha256,
            report.size_bytes,
            report.group_count,
            tebako_json::escape(&report.path.display().to_string()),
        );
    } else {
        println!("{}  {}", report.sha256, report.filename);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tebako-cli-blksum-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_sidecar_is_derived_from_the_image_and_named_beside_it() {
        let dir = scratch("derive");
        let image = dir.join("app-1.0-linux-gnu-x86_64.tfs");
        let bytes = vec![0x5au8; 70 * 1024 * 1024 + 7];
        std::fs::write(&image, &bytes).unwrap();

        let report = write_sidecar(&image, None).unwrap();
        assert_eq!(
            report.path,
            dir.join("app-1.0-linux-gnu-x86_64.tfs.blksum.json")
        );
        assert_eq!(report.filename, "app-1.0-linux-gnu-x86_64.tfs.blksum.json");
        assert_eq!(report.size_bytes, bytes.len() as u64);
        // 4 MiB groups: 70 MiB + 7 bytes is 18 groups, the last short.
        assert_eq!(report.group_count, 18);

        // The served sidecar is the model's own render, anchored by its
        // own digest — the value a registry row pins.
        let written = std::fs::read(&report.path).unwrap();
        assert_eq!(report.sha256, tebako_resolve::sha256_hex(&written));
        let parsed = tpkg::lazy::Blksum::parse(std::str::from_utf8(&written).unwrap()).unwrap();
        assert_eq!(parsed.sha256, tebako_resolve::sha256_hex(&bytes));
        assert_eq!(parsed.group_count(), 18);
    }

    #[test]
    fn an_empty_image_is_the_named_refusal() {
        let dir = scratch("empty");
        let image = dir.join("empty.tfs");
        std::fs::write(&image, b"").unwrap();
        let err = write_sidecar(&image, None).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
        assert!(
            err.message.contains("a 0-byte image has no blksum"),
            "{err}"
        );
    }

    #[test]
    fn out_places_the_sidecar_under_its_own_name() {
        let dir = scratch("out");
        let image = dir.join("image.tfs");
        std::fs::write(&image, b"x").unwrap();
        let out = dir.join("published.tfs.blksum.json");
        let report = write_sidecar(&image, Some(&out)).unwrap();
        assert_eq!(report.path, out);
        assert_eq!(report.filename, "published.tfs.blksum.json");
        assert!(out.is_file());
    }
}
