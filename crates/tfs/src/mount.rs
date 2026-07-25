//! Mount construction: open an image (file / file region / memory), sniff
//! its format, build the backend. Mirrors the C++ BackendFactory dispatch;
//! the dwarfs and squashfs formats are clean ENOTSUP stubs in v1 (the
//! dwarfs backend lands via the external `dwarfs-rs` crate next milestone).

use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::backend::{detect_format, Backend, ImageFormat};
use crate::backends_zip::ZipBackend;
use crate::context::Mount;

const MAGIC_LEN: usize = 8;

/// Build a backend for a detected format, or a stubbed error.
fn backend_for(
    format: ImageFormat,
    open_zip: impl FnOnce() -> Result<ZipBackend, i32>,
) -> Result<Box<dyn Backend>, i32> {
    match format {
        ImageFormat::Zip => Ok(Box::new(open_zip()?)),
        ImageFormat::Dwarfs => Err(libc::ENOTSUP),
        ImageFormat::Squashfs => Err(libc::ENOTSUP),
        ImageFormat::Unknown => Err(libc::EINVAL),
    }
}

fn cstring(s: &str) -> Box<CString> {
    // Paths reaching this layer have already been NUL-validated.
    Box::new(CString::new(s).expect("path contains interior NUL"))
}

/// Mount an archive file (whole file, zero-copy path).
pub fn build_from_file(archive_path: &str, mount_point: &str) -> Result<Mount, i32> {
    let mut file =
        File::open(archive_path).map_err(|e| e.raw_os_error().unwrap_or(libc::ENOENT))?;
    let mut magic = [0u8; MAGIC_LEN];
    let n = file.read(&mut magic).map_err(|_| libc::EIO)?;
    file.seek(SeekFrom::Start(0)).map_err(|_| libc::EIO)?;

    let backend = backend_for(detect_format(&magic[..n]), || ZipBackend::from_file(file))?;
    Ok(Mount {
        handle: 0,
        mount_point: mount_point.to_string(),
        mount_point_c: cstring(mount_point),
        archive_path: Some(cstring(archive_path)),
        backend,
    })
}

/// Mount `length` bytes starting at `offset` of an archive file
/// (`offset == 0 && length == 0` mounts the whole file directly).
/// The region is read into memory owned by the mount.
pub fn build_from_file_at(
    archive_path: &str,
    offset: u64,
    length: u64,
    mount_point: &str,
) -> Result<Mount, i32> {
    if offset == 0 && length == 0 {
        return build_from_file(archive_path, mount_point);
    }
    let mut file =
        File::open(archive_path).map_err(|e| e.raw_os_error().unwrap_or(libc::ENOENT))?;
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
    let mut data = vec![0u8; length as usize];
    file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;
    file.read_exact(&mut data).map_err(|_| libc::EIO)?;
    build_from_memory_owned(data, archive_path, mount_point)
}

/// Mount an archive residing in memory. The image is COPIED (stronger than
/// the C contract, which only borrows until unmount) so no lifetime escapes
/// the FFI layer.
pub fn build_from_memory(data: &[u8], mount_point: &str) -> Result<Mount, i32> {
    build_from_memory_owned(data.to_vec(), "", mount_point)
}

fn build_from_memory_owned(
    data: Vec<u8>,
    archive_path: &str,
    mount_point: &str,
) -> Result<Mount, i32> {
    if data.is_empty() {
        return Err(libc::EINVAL);
    }
    let format = detect_format(&data[..data.len().min(MAGIC_LEN)]);
    let backend = backend_for(format, || ZipBackend::from_memory(data))?;
    Ok(Mount {
        handle: 0,
        mount_point: mount_point.to_string(),
        mount_point_c: cstring(mount_point),
        archive_path: if archive_path.is_empty() {
            None
        } else {
            Some(cstring(archive_path))
        },
        backend,
    })
}
