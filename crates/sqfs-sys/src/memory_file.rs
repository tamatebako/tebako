//! Read-only in-memory implementation of the libsquashfs `sqfs_file_t`
//! interface (mirrors the C++ backend's `sqfs_memory_file_t`).
//!
//! The buffer is BORROWED: the caller must keep it valid until the file
//! (and every reader built on it) is destroyed. Copies (`sqfs_copy`)
//! reference the same buffer; only the wrapper is freed on destroy.

use crate::{sqfs_file_t, sqfs_object_t, sqfs_u64, sqfs_u8, SQFS_ERROR_IO, SQFS_ERROR_UNSUPPORTED};
use core::ffi::{c_int, c_void};

/// `sqfs_memory_file_t`: the vtable struct plus the borrowed-buffer state.
#[repr(C)]
pub struct sqfs_memory_file_t {
    pub base: crate::sqfs_file_t,
    pub data: *const sqfs_u8,
    pub size: sqfs_u64,
}

unsafe extern "C" fn memory_read_at(
    file: *mut sqfs_file_t,
    offset: sqfs_u64,
    buffer: *mut c_void,
    size: usize,
) -> c_int {
    // SAFETY: the object is a live sqfs_memory_file_t; the caller passes a
    // valid buffer of `size` bytes.
    unsafe {
        let mf = file.cast::<sqfs_memory_file_t>();
        if offset > (*mf).size || (size as sqfs_u64) > (*mf).size - offset {
            return SQFS_ERROR_IO;
        }
        std::ptr::copy_nonoverlapping((*mf).data.add(offset as usize), buffer.cast::<u8>(), size);
    }
    0
}

unsafe extern "C" fn memory_write_at(
    _file: *mut sqfs_file_t,
    _offset: sqfs_u64,
    _buffer: *const c_void,
    _size: usize,
) -> c_int {
    SQFS_ERROR_UNSUPPORTED
}

unsafe extern "C" fn memory_get_size(file: *const sqfs_file_t) -> sqfs_u64 {
    // SAFETY: the object is a live sqfs_memory_file_t.
    unsafe { (*file.cast::<sqfs_memory_file_t>()).size }
}

unsafe extern "C" fn memory_truncate(_file: *mut sqfs_file_t, _size: sqfs_u64) -> c_int {
    SQFS_ERROR_UNSUPPORTED
}

unsafe extern "C" fn memory_destroy(obj: *mut sqfs_object_t) {
    // SAFETY: the object was created by sqfs_memory_file_create (Box).
    unsafe {
        drop(Box::from_raw(obj.cast::<sqfs_memory_file_t>()));
    }
}

unsafe extern "C" fn memory_copy(orig: *const sqfs_object_t) -> *mut sqfs_object_t {
    // SAFETY: the object is a live sqfs_memory_file_t; the copy is a fresh
    // shallow clone referencing the SAME borrowed buffer (like the C++ side).
    unsafe {
        let src = &*(orig as *const sqfs_memory_file_t);
        let dup = Box::new(sqfs_memory_file_t {
            base: crate::sqfs_file_t {
                base: sqfs_object_t {
                    destroy: Some(memory_destroy),
                    copy: Some(memory_copy),
                },
                read_at: Some(memory_read_at),
                write_at: Some(memory_write_at),
                get_size: Some(memory_get_size),
                truncate: Some(memory_truncate),
            },
            data: src.data,
            size: src.size,
        });
        Box::into_raw(dup).cast::<sqfs_object_t>()
    }
}

/// Create a memory-backed `sqfs_file_t` over `data` (borrowed, NOT copied;
/// `data` must outlive the returned file and everything built on it).
/// NULL-sized or NULL buffers return null.
pub unsafe fn sqfs_memory_file_create(data: *const c_void, size: usize) -> *mut sqfs_file_t {
    if data.is_null() || size == 0 {
        return std::ptr::null_mut();
    }
    let mf = Box::new(sqfs_memory_file_t {
        base: crate::sqfs_file_t {
            base: sqfs_object_t {
                destroy: Some(memory_destroy),
                copy: Some(memory_copy),
            },
            read_at: Some(memory_read_at),
            write_at: Some(memory_write_at),
            get_size: Some(memory_get_size),
            truncate: Some(memory_truncate),
        },
        data: data.cast::<sqfs_u8>(),
        size: size as sqfs_u64,
    });
    Box::into_raw(mf).cast::<sqfs_file_t>()
}
