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
//! ## Surface (milestone 2)
//!
//! ABI: `tebako_fs_abi_version` (== 1, byte-for-byte with libtfs main).
//! Lifecycle: `tebako_fs_init_from_file`, `tebako_fs_init_from_file_at`,
//! `tebako_fs_init`, `tebako_fs_unmount`, `tebako_is_initialized`.
//! Multi-mount: `tebako_fs_mount_from_file`, `..._from_file_at`,
//! `..._from_memory`, `tebako_fs_unmount_handle`. Files: `tebako_fs_open`,
//! `read`, `pread`, `lseek`, `close`, `stat`, `fstat`. Directories:
//! `tebako_fs_opendir`, `readdir`, `closedir`, `rewinddir`, `telldir`,
//! `seekdir`, `tebako_fs_dir_is_embedded`. Extraction/dlopen:
//! `tebako_fs_extract_all`, `tebako_fs_dlmap2file`. Utility:
//! `tebako_get_errno`, `tebako_strerror`, `tebako_get_mount_point`,
//! `tebako_get_archive_path`, `tebako_get_backend_name`,
//! `tebako_path_is_embedded`, `tebako_fd_is_embedded`. Backends: ZIP
//! (pure-Rust `zip` crate) and DwarFS (external `dwarfs-rs` crate, feature
//! `vendored-dwarfs`, default on); squashfs mounts fail cleanly with
//! ENOTSUP.
//!
//! ## PLANNED (next milestones)
//!
//! Squashfs backend; the full 493-test C++ contract import; then
//! crates/tebako-pkg, tfs-cli, tebako-cli, tebako-bootstrap per the locked
//! repo strategy.

pub mod backend;
#[cfg(feature = "vendored-dwarfs")]
pub mod backends_dwarfs;
#[cfg(feature = "vendored-squashfs")]
pub mod backends_squashfs;
pub mod backends_zip;
pub mod c_api;
pub mod context;
pub mod errno;
pub mod mount;

pub use backend::{Backend, EntryType, RawDirEntry, RawStat};
pub use context::{TebakoCDirent, DT_DIR, DT_REG, TEBAKO_FD_FLAG, TEBAKO_FD_MAX};
