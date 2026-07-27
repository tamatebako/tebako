//! In-process DwarFS image creation via the dwarfs-t `Writer` (the safe
//! binding of dwarfs-t-rs). No mkdwarfs binary, no PATH lookup, no
//! provisioning — the same stable C ABI the reader uses produces the
//! image.
//!
//! Images produced here carry dwarfs-t-native (FlatBuffers) metadata —
//! upstream DwarFS cannot read them — so they are named `.tfs` (the
//! `.dwarfs` extension stays for upstream-compatible images).

use std::path::Path;

use dwarfs_t::{Writer, WriterOptions};

use crate::error::{plain_error, TebakoError};

/// Build a DwarFS image of `src_dir` at `out` (the mkdwarfs
/// `-o <out> -i <src_dir>` equivalent, in-process). The Writer never
/// overwrites; the packaging environment is recreated per press, so
/// `out` never exists at this point.
///
/// When the assembled tree carries `__tpkg__/manifest.yaml`, its
/// `identity.digest.tree_hash` is filled with the payload tree hash
/// (spec 03 §7 fixed-point rule: the hash excludes `/__tpkg__/`, so the
/// stamp is a fixed point) via a hardlink staging mirror — the assembled
/// tree itself is never mutated.
pub fn build_image(out: &Path, src_dir: &Path) -> Result<(), TebakoError> {
    println!("-- Building DwarFS image {}", out.display());
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
    let mut writer = Writer::new(WriterOptions::default())
        .map_err(|e| plain_error(format!("dwarfs writer: {e}")))?;
    writer
        .add_tree(source, "/")
        .map_err(|e| plain_error(format!("dwarfs writer: scanning {}: {e}", source.display())))?;
    writer
        .write(out)
        .map_err(|e| plain_error(format!("dwarfs writer: {}: {e}", out.display())))
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

    #[test]
    fn image_roundtrip_through_the_reader() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-image-{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(src.join("local")).unwrap();
        std::fs::write(src.join("local").join("hello.txt"), b"hi").unwrap();
        let out = dir.join("fs.tfs");
        build_image(&out, &src).unwrap();
        let fs = dwarfs_t::Filesystem::open(&out).unwrap();
        let meta = fs.stat("local/hello.txt").unwrap();
        let mut buf = vec![0u8; meta.size as usize];
        let n = fs.pread("local/hello.txt", &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], b"hi");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
