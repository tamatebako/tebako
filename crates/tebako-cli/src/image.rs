//! In-process image creation (spec 20 §6): DwarFS via the dwarfs-t
//! `Writer` (the safe binding of dwarfs-t-rs), LimniFS via
//! `limnifs-write` (pure Rust). No mkdwarfs/limni binary, no PATH
//! lookup, no provisioning — in-process writers only.
//!
//! DwarFS images produced here carry dwarfs-t-native (FlatBuffers)
//! metadata — upstream DwarFS cannot read them — so they are named
//! `.tfs` (the `.dwarfs` extension stays for upstream-compatible
//! images). LimniFS images are the tebako single-file layout (spec 20
//! §4): the writer's manifest bytes verbatim plus every slab appended
//! in slab-ordinal order; they are `.tfs`-named too (the store's rule:
//! payload artifacts keep one extension regardless of format).

use std::path::Path;

use dwarfs_t::{Writer, WriterOptions};

use crate::error::{plain_error, TebakoError};
use crate::options::PressImageFormat;

/// Build the application image of `src_dir` at `out` in the chosen
/// format (spec 20 §6: the flag routes the packager's image build and
/// nothing else). An existing output is replaced (the mkdwarfs
/// `--force` parity).
///
/// When the assembled tree carries `__tpkg__/manifest.yaml`, its
/// `identity.digest.tree_hash` is filled with the payload tree hash
/// (spec 03 §7 fixed-point rule: the hash excludes `/__tpkg__/`, so the
/// stamp is a fixed point) via a hardlink staging mirror — the assembled
/// tree itself is never mutated. The stamp is format-neutral: it rides
/// ahead of writer selection.
pub fn build_image(
    out: &Path,
    src_dir: &Path,
    format: PressImageFormat,
) -> Result<(), TebakoError> {
    println!("-- Building {} image {}", format.name(), out.display());
    let staged = stamp_tree_hash(src_dir)?;
    let staged_tree;
    let source = match &staged {
        Some((tmp, tree_hash)) => {
            println!("-- Payload tree hash: {tree_hash}");
            staged_tree = tmp.path().join("tree");
            staged_tree.as_path()
        }
        None => src_dir,
    };
    match format {
        PressImageFormat::Dwarfs => write_dwarfs_image(out, source),
        PressImageFormat::Limnifs => write_limnifs_image(out, source),
    }
}

/// The DwarFS writer (the mkdwarfs `-o <out> -i <src_dir>` equivalent,
/// in-process). The Writer never overwrites; the packaging environment
/// is recreated per press, so `out` never exists at this point.
fn write_dwarfs_image(out: &Path, source: &Path) -> Result<(), TebakoError> {
    let mut writer = Writer::new(WriterOptions::default())
        .map_err(|e| plain_error(format!("dwarfs writer: {e}")))?;
    writer
        .add_tree(source, "/")
        .map_err(|e| plain_error(format!("dwarfs writer: scanning {}: {e}", source.display())))?;
    writer
        .write(out)
        .map_err(|e| plain_error(format!("dwarfs writer: {}: {e}", out.display())))
}

/// The LimniFS writer (spec 20 §6): manifest bytes verbatim + every
/// slab appended in slab-ordinal order (the mount-open walk relies on
/// exactly this shape). Dictionaries are disabled: a dictionary section
/// would sit between the history section and the slab region (and tag
/// drops with dictionary ids), neither of which the v1 backend
/// resolves. Content drops ride lz4-or-store: the runtime-floor
/// readers (every published runtime of the v0.16.x era) reject brotli
/// streams beyond the small-buffer case, and the omnizip zstd decoder
/// shipping in limnifs-core ≤ 0.2.54 mis-decodes some valid zstd frames
/// (omnizip-rs#315, still open; frame checksum mismatch on bytes libzstd
/// itself accepts). The
/// metadata blob rides lz4-HC (codec 0x13): every floor reader decodes
/// it through the SAME fast-lz4 decoder (limnifs-core's
/// `Lz4HcCodec::decompress` delegates), and the HC match finder keeps a
/// realistic tree's blob under the inline ceiling — the
/// native-extension e2e tree: 830 KiB lz4-hc vs 1049 KiB fast lz4,
/// which overshoots the writer's 1000 KiB threshold; `store` (2.5 MB)
/// overshoots the readers' 1 MiB hard ceiling outright. The
/// shared-inline table stays off (`defaults.shared_inline = false`):
/// every floor reader (limnifs-core < 0.2.53) rejects its inode flag
/// 0x08 via its own reserved mask (limnifs#186; the knob is stock
/// since limnifs 0.2.57 — limnifs#189). Spec 20 §5's
/// floor rule pins the full recipe. The metadata is inlined up to the
/// readers' 1 MiB ceiling.
fn write_limnifs_image(out: &Path, source: &Path) -> Result<(), TebakoError> {
    let mut config = limnifs_write::WriteConfig::default_v0_1();
    config.dictionaries.enabled = false;
    config.defaults.metadata_codec = "lz4-hc".to_string();
    config.defaults.text_codec = "lz4".to_string();
    config.defaults.binary_codec = "lz4".to_string();
    config.defaults.shared_inline = false;
    config.tournament.codecs = vec!["store".to_string(), "lz4".to_string()];
    let artifact = limnifs_write::write_directory_with_config(source, &config).map_err(|e| {
        plain_error(format!(
            "limnifs writer: scanning {}: {e}",
            source.display()
        ))
    })?;
    if let Some(sidecar) = &artifact.metadata_sidecar {
        return Err(plain_error(format!(
            "limnifs writer: the tree's metadata externalized ({} bytes to '{}') — a self-contained tebako image inlines the metadata; the tree is too large for this format today (press with --format dwarfs for trees this size)",
            sidecar.bytes.len(),
            sidecar.locator
        )));
    }
    let mut image = artifact.bytes;
    for slab in &artifact.slabs {
        image.extend_from_slice(&slab.bytes);
    }
    std::fs::write(out, &image)
        .map_err(|e| plain_error(format!("limnifs writer: {}: {e}", out.display())))
}

/// Fill the payload manifest's tree hash and stage the stamped tree
/// (see [`build_image`]); `Ok(None)` for a manifest-less tree.
fn stamp_tree_hash(src_dir: &Path) -> Result<Option<(tempfile::TempDir, String)>, TebakoError> {
    let manifest_path = src_dir
        .join(tpkg::merkle::MANIFEST_DIR)
        .join("manifest.yaml");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let digest = tpkg::tree_digest(&tpkg::merkle_host::HostTree::new(src_dir))
        .map_err(|e| plain_error(format!("cannot hash the payload tree: {e}")))?;
    let authored = std::fs::read_to_string(&manifest_path)
        .map_err(|e| plain_error(format!("cannot read {}: {e}", manifest_path.display())))?;
    let Ok(filled) = tpkg::merkle_host::fill_tree_hash(&authored, &digest) else {
        // A malformed authored manifest goes in unstamped (mkimage is
        // the stamper, not the validator; `tfs info --verify` grades it).
        return Ok(None);
    };
    let tmp = tempfile::tempdir()
        .map_err(|e| plain_error(format!("cannot create a staging dir: {e}")))?;
    tpkg::merkle_host::stage_tree(src_dir, &tmp.path().join("tree"), &filled)
        .map_err(|e| plain_error(format!("cannot stage the payload tree: {e}")))?;
    Ok(Some((tmp, tpkg::render_tree_hash(&digest))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_tree(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        // Unique per test: the tests run concurrently in one process.
        let dir =
            std::env::temp_dir().join(format!("tebako-cli-image-{tag}-{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(src.join("local")).unwrap();
        std::fs::write(src.join("local").join("hello.txt"), b"hi").unwrap();
        (dir, src)
    }

    #[test]
    fn dwarfs_image_roundtrip_through_the_reader() {
        let (dir, src) = fixture_tree("dwarfs");
        let out = dir.join("fs.tfs");
        build_image(&out, &src, PressImageFormat::Dwarfs).unwrap();
        let fs = dwarfs_t::Filesystem::open(&out).unwrap();
        let meta = fs.stat("local/hello.txt").unwrap();
        let mut buf = vec![0u8; meta.size as usize];
        let n = fs.pread("local/hello.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"hi");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The limnifs press path (spec 20 §6): the image detects as
    /// limnifs and mounts through the tfs backend — windows included
    /// (the windows tfs ships dwarfs+limnifs).
    #[test]
    fn limnifs_image_roundtrip_through_the_backend() {
        let (dir, src) = fixture_tree("limnifs");
        let out = dir.join("fs.tfs");
        build_image(&out, &src, PressImageFormat::Limnifs).unwrap();
        let mount = tfs::mount::build_from_file(&out.to_string_lossy(), "/mnt")
            .expect("the pressed image mounts");
        assert_eq!(mount.backend.name().to_str().unwrap(), "LimniFS");
        let st = mount.backend.stat("local/hello.txt").unwrap();
        assert_eq!(st.size, 2);
        let mut buf = [0u8; 2];
        let n = mount.backend.pread("local/hello.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"hi");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn press_image_format_ids_match_the_trailer_vocabulary() {
        assert_eq!(
            PressImageFormat::Dwarfs.tpkg_format_id(),
            tpkg::TPKG_FORMAT_DWARFS
        );
        assert_eq!(
            PressImageFormat::Limnifs.tpkg_format_id(),
            tpkg::TPKG_FORMAT_LIMNIFS
        );
        assert!(PressImageFormat::parse("zip").is_err());
    }
}
