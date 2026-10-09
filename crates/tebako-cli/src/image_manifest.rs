//! Read the embedded payload manifest (spec 03 §1,
//! `/__tpkg__/manifest.yaml`) from an image through the tfs C ABI — the
//! install path's tier-1 consumption (the rich, authoritative layer; the
//! registry mirrors only resolution-relevant fields).
//!
//! This module is the install path's ONLY FFI seam (like packager.rs's
//! runtime-image extraction): every `unsafe` lives here, and the
//! process-global VFS mount is serialized behind one mutex.

use std::path::Path;

use crate::error::{plain_error, TebakoError};

/// The tfs VFS mount table is process-global; concurrent installs in one
/// process (tests) must not interleave mount → read → unmount.
static MOUNT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The manifest bytes of the image at `path`. `Ok(None)` means the image
/// mounted cleanly and carries nothing at the well-known path — the caller
/// falls back to the registry's tier-3 mirror (loudly). An image the reader
/// cannot OPEN is a hard error naming the format mismatch (tebako#744):
/// never a silent `None`, which would degrade into a synthesized manifest
/// whose entrypoint paths the embedded manifest never declared.
pub fn read_embedded_manifest(path: &Path) -> Result<Option<String>, TebakoError> {
    use tfs::c_api::*;

    let _guard = MOUNT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let c_path = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| plain_error(format!("invalid image path: {}", path.display())))?;
    let c_mount = std::ffi::CString::new("/__tebako_install__").unwrap();
    let rc = unsafe { tebako_fs_init_from_file(c_path.as_ptr(), c_mount.as_ptr()) };
    if rc != 0 {
        // The reader refused the image: no backend recognizes these bytes —
        // an unsupported format, a newer format revision than this toolchain
        // speaks, or not an image at all. Distinguishable from "the image
        // carries no manifest" and must never degrade into the synthesized
        // mirror (tebako#744): the fallback would hand the dispatcher
        // entrypoint paths the embedded manifest never declared.
        let errno = tfs::errno::get_errno();
        let reason = tfs::errno::strerror_text(errno);
        return Err(plain_error(format!(
            "this tebako cannot read {} ({} {}) — the image is not in a format this toolchain supports; if the payload was pressed by a newer tebako, upgrade tebako and retry",
            path.display(),
            errno,
            reason
        )));
    }
    struct Unmount;
    impl Drop for Unmount {
        fn drop(&mut self) {
            unsafe { tebako_fs_unmount() };
        }
    }
    let _unmount = Unmount;

    let c_manifest = std::ffi::CString::new(format!(
        "/__tebako_install__{}",
        tpkg::PAYLOAD_MANIFEST_PATH
    ))
    .unwrap();
    let fd = unsafe { tebako_fs_open(c_manifest.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Ok(None); // the image carries no manifest
    }
    let mut data = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = unsafe { tebako_fs_read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            unsafe { tebako_fs_close(fd) };
            return Err(plain_error(format!(
                "cannot read the embedded manifest of {}",
                path.display()
            )));
        }
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { tebako_fs_close(fd) };
    String::from_utf8(data).map(Some).map_err(|e| {
        plain_error(format!(
            "the embedded manifest of {} is not UTF-8: {e}",
            path.display()
        ))
    })
}

/// Mount the image at `path` whole at `/` (covered-but-not-held paths
/// still pass through to the host), run `f`, and unmount — the
/// process-global VFS serialized behind the same lock as the manifest
/// read. The zero-runtime materialization's mount seam: the in-image
/// paths `f` sees are the payload's own (`/bin/hello`), so the
/// extracted store tree mirrors the payload layout exactly.
pub fn with_image_mounted<T>(
    path: &Path,
    f: impl FnOnce() -> Result<T, TebakoError>,
) -> Result<T, TebakoError> {
    use tfs::c_api::*;

    let _guard = MOUNT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let c_path = std::ffi::CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| plain_error(format!("invalid image path: {}", path.display())))?;
    let c_mount = std::ffi::CString::new("/").unwrap();
    let rc = unsafe { tebako_fs_init_from_file(c_path.as_ptr(), c_mount.as_ptr()) };
    if rc != 0 {
        return Err(plain_error(format!(
            "cannot mount the payload image {}",
            path.display()
        )));
    }
    struct Unmount;
    impl Drop for Unmount {
        fn drop(&mut self) {
            unsafe { tebako_fs_unmount() };
        }
    }
    let _unmount = Unmount;
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tebako#744: a reader-refused image (no backend recognizes the
    /// bytes) must surface as a NAMED format error at the read — never
    /// as "no embedded manifest", which would degrade into a
    /// synthesized mirror whose entrypoint paths the image never
    /// declared. The genuinely manifest-less image keeps its designed
    /// loud-synthesis path (covered in tests/install.rs).
    #[test]
    fn an_unreadable_image_is_a_named_error_not_a_missing_manifest() {
        let path = std::env::temp_dir().join(format!(
            "tebako-i744-garbage-{}-{}.tfs",
            std::process::id(),
            line!()
        ));
        std::fs::write(
            &path,
            b"not an image: no backend magic matches these bytes\n",
        )
        .unwrap();
        let err = read_embedded_manifest(&path)
            .expect_err("an unreadable image must fail, not synthesize");
        let text = err.to_string();
        assert!(text.contains("cannot read"), "{text}");
        assert!(text.contains("upgrade tebako"), "{text}");
        assert!(
            !text.to_lowercase().contains("no embedded manifest"),
            "{text}"
        );
        std::fs::remove_file(&path).ok();
    }
}
