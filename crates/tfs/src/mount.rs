//! Mount construction: open an image (file / file region / memory), sniff
//! its format, build the backend. Mirrors the C++ BackendFactory dispatch:
//! DwarFS (vendored-dwarfs) and SquashFS (vendored-squashfs) are real with
//! their default features on, ENOTSUP stubs without.

use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
#[cfg(feature = "vendored-dwarfs")]
use std::path::Path;

use crate::backend::{detect_format, Backend, ImageFormat};
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
    Ok(make_mount(mount_point, Some(archive_path), backend))
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
    Ok(make_mount(mount_point, None, backend))
}
