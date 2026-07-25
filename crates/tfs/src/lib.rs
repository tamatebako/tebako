//! # tfs — libtfs-rs
//!
//! The Rust implementation of libtfs: a read-only VFS for embedded archive
//! images, exporting the **`tebako_fs_*` C ABI** byte-for-byte per
//! `include/tebako/fs/c_api.h` in [libtfs](https://github.com/tamatebako/libtfs).
//! Built as `cdylib` (`libtfs.so`/`.dylib`/`.dll`) and `staticlib`
//! (`libtfs.a`/`.lib`), it is a drop-in: consumers cannot tell which
//! implementation they linked.
//!
//! ## Architecture
//!
//! ```text
//! C consumer ──► c_api.rs (extern "C", the ONLY unsafe module)
//!                   │  arg validation, thread-local errno channel
//!                   ▼
//!                context.rs (FsContext: mount table, fd/dir tables,
//!                   │         longest-prefix dispatch — safe Rust)
//!                   ▼
//!                backend.rs (Backend trait)
//!                   ├── backends_zip.rs  (ZIP via the pure-Rust `zip` crate)
//!                   ├── dwarfs           (PLANNED: external dwarfs-rs crate)
//!                   └── squashfs         (PLANNED: squashfs-tools-ng FFI)
//! ```
//!
//! ## v1 surface (SHIPPED)
//!
//! Lifecycle: `tebako_fs_init_from_file`, `tebako_fs_init_from_file_at`,
//! `tebako_fs_init`, `tebako_fs_unmount`, `tebako_is_initialized`.
//! Files: `tebako_fs_open`, `read`, `pread`, `lseek`, `close`, `stat`,
//! `fstat`. Directories: `tebako_fs_opendir`, `readdir`, `closedir`,
//! `tebako_fs_dir_is_embedded`. Utility: `tebako_get_errno`,
//! `tebako_strerror`, `tebako_get_mount_point`, `tebako_get_archive_path`,
//! `tebako_get_backend_name`, `tebako_path_is_embedded`,
//! `tebako_fd_is_embedded`. Backend: ZIP (dwarfs/squashfs mounts fail
//! cleanly with ENOTSUP).
//!
//! ## PLANNED (next milestones)
//!
//! Multi-mount (`tebako_fs_mount_*`/`tebako_fs_unmount_handle`), the dwarfs
//! backend via the external [`dwarfs`](https://github.com/tamatebako/dwarfs-rs)
//! crate, squashfs backend, `tebako_fs_rewinddir`/`telldir`/`seekdir`,
//! `tebako_fs_extract_all`, `tebako_fs_dlmap2file`.

pub mod backend;
pub mod backends_zip;
pub mod c_api;
pub mod context;
pub mod errno;
pub mod mount;

pub use backend::{Backend, EntryType, RawDirEntry, RawStat};
pub use context::{TebakoCDirent, DT_DIR, DT_REG, TEBAKO_FD_FLAG, TEBAKO_FD_MAX};
