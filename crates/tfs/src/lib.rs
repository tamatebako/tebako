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
//!                backend.rs (Backend trait; WritableBackend for the
//!                   │         composite write seam — spec 11 §4)
//!                   ├── backends_zip.rs  (ZIP via the pure-Rust `zip` crate)
//!                   ├── backends_tar.rs  (tar/tar.gz/tar.zst, offset index)
//!                   ├── backends_cow.rs  (COW composite + whiteout journal)
//!                   ├── backends_hostdir.rs (host directory, the COW overlay)
//!                   ├── dwarfs           (external dwarfs-rs crate)
//!                   └── squashfs         (squashfs-tools-ng FFI)
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
//! `tebako_path_is_embedded`, `tebako_fd_is_embedded`. Mount modes (spec
//! 11 §3, additive): `tebako_fs_mount_from_file_with_mode`,
//! `..._from_file_at_with_mode`, `..._from_memory_with_mode` taking
//! `TEBAKO_MOUNT_RO` (0, default), `_COW` (1, HostDir overlay + whiteout
//! journal) or `_RW` (2, ENOTSUP — no in-tree backend writes in place).
//! Backends: ZIP (pure-Rust `zip` crate), tar/tar.gz/tar.zst (pure-Rust
//! offset index — `backends_tar`), COW composite over any image
//! (`backends_cow` + `backends_hostdir`), and DwarFS (external `dwarfs-rs`
//! crate, feature `vendored-dwarfs`, default on); SquashFS via
//! `crates/sqfs-sys` (feature `vendored-squashfs`, default on). Writes on
//! RO mounts fail EROFS; path-level writes on COW mounts route through
//! the context (`pwrite_path`/`truncate_path`/`mkdir_path`/`remove_path`);
//! the fd-based write family (spec 11 §7) is a later additive milestone.
//! Jails (spec 08): `tebako_fs_host_policy`
//! installs the host-access policy gating every host-passthrough path
//! decision (`crates/tfs/src/policy.rs`); denied paths fail EPERM, writes
//! against an ro grant EROFS, allowed paths keep today's ENOENT
//! pass-through.
//!
//! ## PLANNED (next milestones)
//!
//! The fd-based write family gated by mount mode; then the remaining
//! roadmap-13 format backends (iso9660, ext, FAT — pure Rust, detection
//! slots already ordered after the strong magics).

pub mod backend;
pub mod backends_cow;
#[cfg(feature = "vendored-dwarfs")]
pub mod backends_dwarfs;
pub mod backends_enc;
pub mod backends_hostdir;
#[cfg(feature = "vendored-squashfs")]
pub mod backends_squashfs;
pub mod backends_tar;
pub mod backends_zip;
pub mod c_api;
pub mod context;
pub mod errno;
pub mod mount;
pub mod mount_spec;
pub mod policy;
pub mod secure_buf;
pub mod tree_walk;

pub use backend::{Backend, EntryType, RawDirEntry, RawStat, WritableBackend};
pub use backends_enc::{EncBackend, KeySource, ENOKEY};
pub use context::{TebakoCDirent, DT_DIR, DT_REG, TEBAKO_FD_FLAG, TEBAKO_FD_MAX};
pub use mount::{MountMode, TEBAKO_MOUNT_COW, TEBAKO_MOUNT_RO, TEBAKO_MOUNT_RW};
pub use policy::{HostAccess, HostMount, HostMountSpec, HostPolicy, JailSpec, JailSpecError};

/// Image-level metadata as JSON for an image file (item 24's
/// `image_info_json`), built straight from the backend — outside the
/// mount-table C ABI. `Err(ENOTSUP)` for backends without a metadata
/// surface, `Err(errno)` on open failures.
pub fn image_info_json(path: &str) -> Result<String, i32> {
    let mount = mount::build_from_file(path, "/mnt")?;
    mount.backend.image_info_json().ok_or(libc::ENOTSUP)
}
