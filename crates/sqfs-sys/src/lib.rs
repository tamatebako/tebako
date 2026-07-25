//! Raw FFI bindings to libsquashfs (squashfs-tools-ng), the SquashFS
//! reader C library.
//!
//! Hand-written on purpose (same discipline as dwarfs-t-sys): the used
//! surface is small (~15 functions, 6 structs) and the ABI is pinned at
//! build time by `abi_check.c` (`_Static_assert`s over every struct size,
//! field offset, and constant the Rust side relies on). `shim.c` keeps the
//! variable-layout `sqfs_compressor_config_t` entirely on the C side.
//!
//! Everything in this crate is `unsafe` to call. The safe consumer is the
//! `tfs` crate's SquashFS backend.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

pub mod memory_file;

pub use memory_file::sqfs_memory_file_create;

use core::ffi::{c_char, c_int, c_void};

/// libsquashfs `sqfs_u8`.
pub type sqfs_u8 = u8;
/// libsquashfs `sqfs_u16`.
pub type sqfs_u16 = u16;
/// libsquashfs `sqfs_u32`.
pub type sqfs_u32 = u32;
/// libsquashfs `sqfs_u64`.
pub type sqfs_u64 = u64;
/// libsquashfs `sqfs_s16`.
pub type sqfs_s16 = i16;
/// libsquashfs `sqfs_s32`.
pub type sqfs_s32 = i32;

/// `SQFS_FILE_OPEN_READ_ONLY`.
pub const SQFS_FILE_OPEN_READ_ONLY: c_int = 0x01;
/// `SQFS_COMP_FLAG_UNCOMPRESS` (reader mode).
pub const SQFS_COMP_FLAG_UNCOMPRESS: sqfs_u32 = 0x8000;

/// `SQFS_INODE_DIR`.
pub const SQFS_INODE_DIR: sqfs_u16 = 1;
/// `SQFS_INODE_FILE`.
pub const SQFS_INODE_FILE: sqfs_u16 = 2;
/// `SQFS_INODE_SLINK`.
pub const SQFS_INODE_SLINK: sqfs_u16 = 3;
/// `SQFS_INODE_EXT_DIR`.
pub const SQFS_INODE_EXT_DIR: sqfs_u16 = 8;
/// `SQFS_INODE_EXT_FILE`.
pub const SQFS_INODE_EXT_FILE: sqfs_u16 = 9;
/// `SQFS_INODE_EXT_SLINK`.
pub const SQFS_INODE_EXT_SLINK: sqfs_u16 = 10;

/// `SQFS_ERROR_IO` — generic i/o failure (also used by the memory file).
pub const SQFS_ERROR_IO: c_int = -2;
/// `SQFS_ERROR_NOT_FILE` — sqfs_inode_get_file_size on a non-file.
pub const SQFS_ERROR_NOT_FILE: c_int = -15;
/// `SQFS_ERROR_UNSUPPORTED` — operation not supported (memory write_at).
pub const SQFS_ERROR_UNSUPPORTED: c_int = -6;

/// `sqfs_super_t` — the on-disk superblock (96 bytes; pinned by abi_check).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sqfs_super_t {
    pub magic: sqfs_u32,
    pub inode_count: sqfs_u32,
    pub modification_time: sqfs_u32,
    pub block_size: sqfs_u32,
    pub fragment_entry_count: sqfs_u32,
    pub compression_id: sqfs_u16,
    pub block_log: sqfs_u16,
    pub flags: sqfs_u16,
    pub id_count: sqfs_u16,
    pub version_major: sqfs_u16,
    pub version_minor: sqfs_u16,
    pub root_inode_ref: sqfs_u64,
    pub bytes_used: sqfs_u64,
    pub id_table_start: sqfs_u64,
    pub xattr_id_table_start: sqfs_u64,
    pub inode_table_start: sqfs_u64,
    pub directory_table_start: sqfs_u64,
    pub fragment_table_start: sqfs_u64,
    pub export_table_start: sqfs_u64,
}

/// `sqfs_inode_t` — the common inode header; also the FIRST member of
/// `sqfs_inode_generic_t`, so a generic inode pointer reads as this
/// (abi_check asserts `offsetof(sqfs_inode_generic_t, base) == 0`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sqfs_inode_t {
    /// `SQFS_INODE_*` type.
    pub r#type: sqfs_u16,
    /// POSIX mode bits.
    pub mode: sqfs_u16,
    pub uid_idx: sqfs_u16,
    pub gid_idx: sqfs_u16,
    /// Modification time, seconds since the epoch.
    pub mod_time: sqfs_u32,
    pub inode_number: sqfs_u32,
}

/// `sqfs_dir_entry_t` — directory entry header; the name follows inline
/// (length is `size + 1`, stored off-by-one).
#[repr(C)]
pub struct sqfs_dir_entry_t {
    pub offset: sqfs_u16,
    pub inode_diff: sqfs_s16,
    /// `SQFS_INODE_*` type of the referenced inode.
    pub r#type: sqfs_u16,
    /// Name length MINUS ONE (off-by-one on the wire).
    pub size: sqfs_u16,
    /// Start of the entry name (NOT NUL-terminated).
    pub name: [u8; 0],
}

/// `sqfs_object_t` — libsquashfs refcounted-object vtable header.
#[repr(C)]
pub struct sqfs_object_t {
    pub destroy: Option<unsafe extern "C" fn(*mut sqfs_object_t)>,
    pub copy: Option<unsafe extern "C" fn(*const sqfs_object_t) -> *mut sqfs_object_t>,
}

/// `sqfs_file_t` — the archive file interface (vtable).
#[repr(C)]
pub struct sqfs_file_t {
    pub base: sqfs_object_t,
    pub read_at:
        Option<unsafe extern "C" fn(*mut sqfs_file_t, sqfs_u64, *mut c_void, usize) -> c_int>,
    pub write_at:
        Option<unsafe extern "C" fn(*mut sqfs_file_t, sqfs_u64, *const c_void, usize) -> c_int>,
    pub get_size: Option<unsafe extern "C" fn(*const sqfs_file_t) -> sqfs_u64>,
    pub truncate: Option<unsafe extern "C" fn(*mut sqfs_file_t, sqfs_u64) -> c_int>,
}

/// Opaque `sqfs_compressor_t`.
pub enum sqfs_compressor_t {}
/// Opaque `sqfs_dir_reader_t`.
pub enum sqfs_dir_reader_t {}
/// Opaque `sqfs_data_reader_t`.
pub enum sqfs_data_reader_t {}
/// Opaque `sqfs_inode_generic_t` (readable as [`sqfs_inode_t`] via its
/// leading `base` member).
pub enum sqfs_inode_generic_t {}

extern "C" {
    /// Open an archive file on disk.
    pub fn sqfs_open_file(filename: *const c_char, flags: c_int) -> *mut sqfs_file_t;

    /// Read the superblock from an archive file.
    pub fn sqfs_super_read(super_: *mut sqfs_super_t, file: *mut sqfs_file_t) -> c_int;

    /// Create a directory reader.
    pub fn sqfs_dir_reader_create(
        super_: *const sqfs_super_t,
        cmp: *mut sqfs_compressor_t,
        file: *mut sqfs_file_t,
        flags: sqfs_u32,
    ) -> *mut sqfs_dir_reader_t;

    /// Resolve the root directory inode (caller frees with sqfs_free).
    pub fn sqfs_dir_reader_get_root_inode(
        rd: *mut sqfs_dir_reader_t,
        inode: *mut *mut sqfs_inode_generic_t,
    ) -> c_int;

    /// Resolve an inode by path (`start` NULL = from root; caller frees the
    /// result with sqfs_free).
    pub fn sqfs_dir_reader_find_by_path(
        rd: *mut sqfs_dir_reader_t,
        start: *const sqfs_inode_generic_t,
        path: *const c_char,
        inode: *mut *mut sqfs_inode_generic_t,
    ) -> c_int;

    /// Open a directory for reading (after open, repeated
    /// sqfs_dir_reader_read yields entries).
    pub fn sqfs_dir_reader_open_dir(
        rd: *mut sqfs_dir_reader_t,
        inode: *const sqfs_inode_generic_t,
        flags: sqfs_u32,
    ) -> c_int;

    /// Read the next directory entry (caller frees each with sqfs_free).
    /// Returns 0 while entries remain, non-zero at end/error.
    pub fn sqfs_dir_reader_read(
        rd: *mut sqfs_dir_reader_t,
        out: *mut *mut sqfs_dir_entry_t,
    ) -> c_int;

    /// Get the inode of the most recently read directory entry (caller
    /// frees with sqfs_free).
    pub fn sqfs_dir_reader_get_inode(
        rd: *mut sqfs_dir_reader_t,
        inode: *mut *mut sqfs_inode_generic_t,
    ) -> c_int;

    /// File size of an inode (SQFS_ERROR_NOT_FILE for non-files).
    pub fn sqfs_inode_get_file_size(
        inode: *const sqfs_inode_generic_t,
        size: *mut sqfs_u64,
    ) -> c_int;

    /// Create a data reader.
    pub fn sqfs_data_reader_create(
        file: *mut sqfs_file_t,
        block_size: usize,
        cmp: *mut sqfs_compressor_t,
        flags: sqfs_u32,
    ) -> *mut sqfs_data_reader_t;

    /// Load the fragment table (required to read fragment-packed files).
    pub fn sqfs_data_reader_load_fragment_table(
        data: *mut sqfs_data_reader_t,
        super_: *const sqfs_super_t,
    ) -> c_int;

    /// Read file data at an offset (returns bytes read, or SQFS_ERROR_*).
    pub fn sqfs_data_reader_read(
        data: *mut sqfs_data_reader_t,
        inode: *const sqfs_inode_generic_t,
        offset: sqfs_u64,
        buffer: *mut c_void,
        size: sqfs_u32,
    ) -> sqfs_s32;

    /// Free an object allocated by libsquashfs (inodes, dir entries).
    pub fn sqfs_free(ptr: *mut c_void);

    /// shim.c: compressor-config init + create in one call, keeping the
    /// variable-layout `sqfs_compressor_config_t` on the C side.
    pub fn sqfs_shim_compressor_create(
        compression_id: sqfs_u16,
        block_size: sqfs_u32,
        out: *mut *mut sqfs_compressor_t,
    ) -> c_int;
}

/// `sqfs_destroy` is a `static SQFS_INLINE` in sqfs/predef.h (not an
/// exported symbol): it invokes the object's own vtable destroy hook.
/// Re-implemented here with the identical semantics.
///
/// # Safety
/// `ptr` must be NULL or a valid libsquashfs object whose destroy hook
/// releases it exactly once.
pub unsafe fn sqfs_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let obj = ptr.cast::<sqfs_object_t>();
        // SAFETY: per the caller contract above.
        unsafe {
            if let Some(destroy) = (*obj).destroy {
                destroy(obj);
            }
        }
    }
}
