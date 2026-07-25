//! Mount construction: open an image (file / file region / memory), sniff
//! its format, build the backend. Mirrors the C++ BackendFactory dispatch.
//! Squashfs stays an ENOTSUP stub; DwarFS is real when the
//! `vendored-dwarfs` feature is on (default) and an ENOTSUP stub without it.

use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
#[cfg(feature = "vendored-dwarfs")]
use std::path::Path;

use crate::backend::{detect_format, Backend, ImageFormat};
use crate::backends_zip::ZipBackend;
use crate::context::Mount;

#[cfg(feature = "vendored-dwarfs")]
use crate::backends_dwarfs::DwarfsBackend;

const MAGIC_LEN: usize = 8;

fn cstring(s: &str) -> Box<CString> {
    // Paths reaching this layer have already been NUL-validated.
    Box::new(CString::new(s).expect("path contains interior NUL"))
}

fn make_mount(mount_point: &str, archive_path: Option<&str>, backend: Box<dyn Backend>) -> Mount {
    Mount {
        handle: 0,
        mount_point: mount_point.to_string(),
        mount_point_c: cstring(mount_point),
        archive_path: archive_path.map(cstring),
        backend,
    }
}

fn open_error(e: std::io::Error) -> i32 {
    e.raw_os_error().unwrap_or(libc::ENOENT)
}

/// Mount an archive file (whole file, zero-copy path where the backend
/// supports it).
pub fn build_from_file(archive_path: &str, mount_point: &str) -> Result<Mount, i32> {
    let mut file = File::open(archive_path).map_err(open_error)?;
    let mut magic = [0u8; MAGIC_LEN];
    let n = file.read(&mut magic).map_err(|_| libc::EIO)?;
    file.seek(SeekFrom::Start(0)).map_err(|_| libc::EIO)?;
    let format = detect_format(&magic[..n]);

    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => Box::new(ZipBackend::from_file(file)?),
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_file(Path::new(archive_path))?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    Ok(make_mount(mount_point, Some(archive_path), backend))
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
    if offset == 0 && length == 0 {
        return build_from_file(archive_path, mount_point);
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
    let mut magic = [0u8; MAGIC_LEN];
    file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;
    let n = file.read(&mut magic).map_err(|_| libc::EIO)?;
    let format = detect_format(&magic[..n]);

    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => {
            let mut data = vec![0u8; length as usize];
            file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;
            file.read_exact(&mut data).map_err(|_| libc::EIO)?;
            Box::new(ZipBackend::from_memory(data)?)
        }
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_file_at(
            Path::new(archive_path),
            offset as i64,
            length,
        )?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    Ok(make_mount(mount_point, Some(archive_path), backend))
}

/// Mount an archive residing in memory. The image is COPIED (stronger than
/// the C contract, which only borrows until unmount) so no lifetime escapes
/// the FFI layer.
pub fn build_from_memory(data: &[u8], mount_point: &str) -> Result<Mount, i32> {
    if data.is_empty() {
        return Err(libc::EINVAL);
    }
    let format = detect_format(&data[..data.len().min(MAGIC_LEN)]);
    let backend: Box<dyn Backend> = match format {
        ImageFormat::Zip => Box::new(ZipBackend::from_memory(data.to_vec())?),
        #[cfg(feature = "vendored-dwarfs")]
        ImageFormat::Dwarfs => Box::new(DwarfsBackend::from_memory(data)?),
        #[cfg(not(feature = "vendored-dwarfs"))]
        ImageFormat::Dwarfs => return Err(libc::ENOTSUP),
        ImageFormat::Squashfs => return Err(libc::ENOTSUP),
        ImageFormat::Unknown => return Err(libc::EINVAL),
    };
    Ok(make_mount(mount_point, None, backend))
}
