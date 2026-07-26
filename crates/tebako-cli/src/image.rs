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
pub fn build_image(out: &Path, src_dir: &Path) -> Result<(), TebakoError> {
    println!("-- Building DwarFS image {}", out.display());
    let mut writer = Writer::new(WriterOptions::default())
        .map_err(|e| plain_error(format!("dwarfs writer: {e}")))?;
    writer.add_tree(src_dir, "/").map_err(|e| {
        plain_error(format!(
            "dwarfs writer: scanning {}: {e}",
            src_dir.display()
        ))
    })?;
    writer
        .write(out)
        .map_err(|e| plain_error(format!("dwarfs writer: {}: {e}", out.display())))
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
