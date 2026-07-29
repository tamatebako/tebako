//! Mount construction: open an image (file / file region / memory), sniff
//! its format, build the backend. Mirrors the C++ BackendFactory dispatch:
//! DwarFS (vendored-dwarfs) and SquashFS (vendored-squashfs) are real with
//! their default features on, ENOTSUP stubs without.
//!
//! Mounts carry a mode (spec 11 §3): RO (default), COW (a HostDir overlay
//! stacked over the image backend — spec 11 §4), RW (in-place; no in-tree
//! format backend offers it → ENOTSUP).

use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
#[cfg(feature = "vendored-dwarfs")]
use std::path::Path;

use crate::backend::{detect_format, Backend, ImageFormat};
use crate::backends_cow::CowBackend;
use crate::backends_hostdir::{io_errno, HostDirBackend};
use crate::backends_tar::{TarBackend, TarCompression};
use crate::backends_zip::ZipBackend;
use crate::context::Mount;

#[cfg(feature = "vendored-dwarfs")]
use crate::backends_dwarfs::DwarfsBackend;
#[cfg(feature = "vendored-squashfs")]
use crate::backends_squashfs::SquashfsBackend;

/// Sniff length: one full tar block, so the tar header-checksum heuristic
/// (weak, last in the chain — spec 11 §3) has its 512 bytes.
const SNIFF_LEN: usize = 512;

/// Mount-mode flag: read-only (the default).
pub const TEBAKO_MOUNT_RO: u32 = 0;
/// Mount-mode flag: copy-on-write (HostDir overlay over the image).
pub const TEBAKO_MOUNT_COW: u32 = 1;
/// Mount-mode flag: read-write in place (no in-tree backend offers it).
pub const TEBAKO_MOUNT_RW: u32 = 2;

/// The mount mode (spec 11 §3); writes on RO mounts fail with EROFS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountMode {
    /// Read-only (default; `TEBAKO_MOUNT_RO`).
    ReadOnly,
    /// Copy-on-write composite (`TEBAKO_MOUNT_COW`).
    Cow,
    /// Read-write in place (`TEBAKO_MOUNT_RW`; ENOTSUP in-tree).
    ReadWrite,
}

/// The compression envelope matching a detected tar-family format.
fn tar_compression(format: ImageFormat) -> TarCompression {
    match format {
        ImageFormat::Tar => TarCompression::None,
        ImageFormat::TarGz => TarCompression::Gzip,
        ImageFormat::TarZst => TarCompression::Zstd,
        _ => unreachable!("tar_compression called for a non-tar format"),
    }
}

fn cstring(s: &str) -> Box<CString> {
    // Paths reaching this layer have already been NUL-validated.
    Box::new(CString::new(s).expect("path contains interior NUL"))
}

fn make_mount(
    mount_point: &str,
    archive_path: Option<&str>,
    backend: Box<dyn Backend>,
    mode: MountMode,
) -> Mount {
    Mount {
        handle: 0,
        mount_point: mount_point.to_string(),
        mount_point_c: cstring(mount_point),
        archive_path: archive_path.map(cstring),
        backend,
        mode,
    }
}

fn open_error(e: std::io::Error) -> i32 {
    e.raw_os_error().unwrap_or(libc::ENOENT)
}

/// Wrap the image backend for the mount mode (spec 11 §3/§4). COW creates
/// the overlay directory when missing — it is a scratch area, disposable
/// by deletion.
fn apply_mode(
    backend: Box<dyn Backend>,
    mode: MountMode,
    overlay_dir: Option<&str>,
) -> Result<Box<dyn Backend>, i32> {
    match mode {
        MountMode::ReadOnly => {
            if overlay_dir.is_some() {
                return Err(libc::EINVAL); // an overlay only makes sense for COW
            }
            Ok(backend)
        }
        MountMode::Cow => {
            let dir = overlay_dir.ok_or(libc::EINVAL)?;
            std::fs::create_dir_all(dir).map_err(|e| io_errno(&e))?;
            let overlay = HostDirBackend::new(std::path::Path::new(dir))?;
            Ok(Box::new(CowBackend::new(backend, overlay)?))
        }
        // Honest capability model (spec 11 §3): no in-tree format backend
        // writes in place.
        MountMode::ReadWrite => Err(libc::ENOTSUP),
    }
}

/// Mount an archive file (whole file, zero-copy path where the backend
/// supports it).
pub fn build_from_file(archive_path: &str, mount_point: &str) -> Result<Mount, i32> {
    build_from_file_with_mode(archive_path, mount_point, MountMode::ReadOnly, None)
}

/// [`build_from_file`] with an explicit mount mode (spec 11 §3). COW
/// requires `overlay_dir` (created when missing); RW is ENOTSUP.
pub fn build_from_file_with_mode(
    archive_path: &str,
    mount_point: &str,
    mode: MountMode,
    overlay_dir: Option<&str>,
) -> Result<Mount, i32> {
    let mut file = File::open(archive_path).map_err(open_error)?;
    let mut magic = [0u8; SNIFF_LEN];
    let n = file.read(&mut magic).map_err(|_| libc::EIO)?;
    file.seek(SeekFrom::Start(0)).map_err(|_| libc::EIO)?;
    let format = detect_format(&magic[..n]);

    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => Box::new(ZipBackend::from_file(file)?),
        ImageFormat::Tar | ImageFormat::TarGz | ImageFormat::TarZst => {
            Box::new(TarBackend::from_file(file, tar_compression(format))?)
        }
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_file(Path::new(archive_path))?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        #[cfg(feature = "vendored-squashfs")]
        ImageFormat::Squashfs => Box::new(SquashfsBackend::from_file(archive_path)?),
        #[cfg(not(feature = "vendored-squashfs"))]
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    let backend = apply_mode(backend, mode, overlay_dir)?;
    Ok(make_mount(mount_point, Some(archive_path), backend, mode))
}

/// Mount `length` bytes starting at `offset` of an archive file
/// (`offset == 0 && length == 0` mounts the whole file directly).
///
/// DwarFS regions are opened in place (the reader handles image offsets
/// natively); ZIP regions are read into memory owned by the backend,
/// mirroring the C++ semantics.
pub fn build_from_file_at(
    archive_path: &str,
    offset: u64,
    length: u64,
    mount_point: &str,
) -> Result<Mount, i32> {
    build_from_file_at_with_mode(
        archive_path,
        offset,
        length,
        mount_point,
        MountMode::ReadOnly,
        None,
    )
}

/// [`build_from_file_at`] with an explicit mount mode (spec 11 §3).
pub fn build_from_file_at_with_mode(
    archive_path: &str,
    offset: u64,
    length: u64,
    mount_point: &str,
    mode: MountMode,
    overlay_dir: Option<&str>,
) -> Result<Mount, i32> {
    if offset == 0 && length == 0 {
        return build_from_file_with_mode(archive_path, mount_point, mode, overlay_dir);
    }
    let mut file = File::open(archive_path).map_err(open_error)?;
    let file_size = file.seek(SeekFrom::End(0)).map_err(|_| libc::EIO)?;
    if offset > file_size {
        return Err(libc::EINVAL);
    }
    let length = if length == 0 {
        file_size - offset
    } else {
        length
    };
    if length > file_size - offset {
        return Err(libc::EINVAL);
    }
    // An empty region is not a mountable image (matches C++ EINVAL).
    if length == 0 {
        return Err(libc::EINVAL);
    }

    // Sniff the format at the region start.
    let mut magic = [0u8; SNIFF_LEN];
    file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;
    let n = file.read(&mut magic).map_err(|_| libc::EIO)?;
    let format = detect_format(&magic[..n]);

    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => Box::new(ZipBackend::from_memory(read_region(
            &mut file, offset, length,
        )?)?),
        // Tar regions are read in place (positioned reads relative to the
        // region start; the index pass streams inside the region bounds).
        ImageFormat::Tar | ImageFormat::TarGz | ImageFormat::TarZst => Box::new(
            TarBackend::from_file_at(file, offset, length, tar_compression(format))?,
        ),
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_file_at(
            Path::new(archive_path),
            offset as i64,
            length,
        )?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        #[cfg(feature = "vendored-squashfs")]
        ImageFormat::Squashfs => Box::new(SquashfsBackend::from_memory(read_region(
            &mut file, offset, length,
        )?)?),
        #[cfg(not(feature = "vendored-squashfs"))]
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    let backend = apply_mode(backend, mode, overlay_dir)?;
    Ok(make_mount(mount_point, Some(archive_path), backend, mode))
}

/// Read `[offset, offset+length)` of a file into memory (region mounts of
/// seek-less backends, mirroring the C++ copy semantics).
fn read_region(file: &mut File, offset: u64, length: u64) -> Result<Vec<u8>, i32> {
    let mut data = vec![0u8; length as usize];
    file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;
    file.read_exact(&mut data).map_err(|_| libc::EIO)?;
    Ok(data)
}

/// Mount an archive residing in memory. The image is COPIED (stronger than
/// the C contract, which only borrows until unmount) so no lifetime escapes
/// the FFI layer.
pub fn build_from_memory(data: &[u8], mount_point: &str) -> Result<Mount, i32> {
    build_from_memory_with_mode(data, mount_point, MountMode::ReadOnly, None)
}

/// [`build_from_memory`] with an explicit mount mode (spec 11 §3).
pub fn build_from_memory_with_mode(
    data: &[u8],
    mount_point: &str,
    mode: MountMode,
    overlay_dir: Option<&str>,
) -> Result<Mount, i32> {
    if data.is_empty() {
        return Err(libc::EINVAL);
    }
    let format = detect_format(&data[..data.len().min(SNIFF_LEN)]);
    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => Box::new(ZipBackend::from_memory(data.to_vec())?),
        ImageFormat::Tar | ImageFormat::TarGz | ImageFormat::TarZst => Box::new(
            TarBackend::from_memory(data.to_vec(), tar_compression(format))?,
        ),
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_memory(data)?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        #[cfg(feature = "vendored-squashfs")]
        ImageFormat::Squashfs => Box::new(SquashfsBackend::from_memory(data.to_vec())?),
        #[cfg(not(feature = "vendored-squashfs"))]
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    let backend = apply_mode(backend, mode, overlay_dir)?;
    Ok(make_mount(mount_point, None, backend, mode))
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

/// A gated-off backend degrades to the NAMED error, never a crash or
/// a silent re-route (the Windows build ships dwarfs-only —
/// TODO.v2-1/02). Runs in the `--no-default-features` CI job; with
/// the feature on, the same magic would attempt a real SquashFS
/// mount instead. The whole module compiles out with the feature on
/// (an empty tests module would only borrow trouble).
#[cfg(all(test, not(feature = "vendored-squashfs")))]
mod tests {
    use super::*;

    #[test]
    fn squashfs_without_the_backend_is_a_named_enotsup() {
        let mut magic = Vec::with_capacity(SNIFF_LEN);
        magic.extend_from_slice(b"hsqs");
        magic.resize(SNIFF_LEN, 0);
        let path = std::env::temp_dir().join(format!(
            "tfs-enotsup-{}-{}.sqfs",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, &magic).unwrap();
        let result = build_from_file(&path.to_string_lossy(), "/x");
        let _ = std::fs::remove_file(&path);
        assert_eq!(result.err(), Some(libc::ENOTSUP));
    }
}
