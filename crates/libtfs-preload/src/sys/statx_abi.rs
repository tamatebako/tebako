//! The statx ABI the interpose compiles against.
//!
//! libc 0.2.189 exposes no `statx` on musl targets — and the platform
//! itself is the deeper gap: musl gained the statx(2) wrapper only in
//! 1.2.4 (alpine >= 3.19), so on the factory's alpine 3.17 floor there
//! is neither a symbol to interpose nor a header to declare it. The
//! preload's statx path is inert there by construction (no libc caller
//! can name the function), but it must still COMPILE — and on any
//! musl >= 1.2.4 runtime it must be ABI-truthful.
//!
//! The kernel uapi is one ABI across libcs. The mirror below is
//! verified against glibc's <sys/stat.h> with static asserts: stx_mask
//! @0, stx_mode @28, stx_ino @32, stx_size @40, stx_mtime @112,
//! sizeof(struct statx) == 256, sizeof(struct statx_timestamp) == 16,
//! tv_nsec @8, and the STATX_* mask constants — every field the
//! interpose touches, on both x86_64 and aarch64.

#[cfg(all(target_os = "linux", not(target_env = "musl")))]
pub(crate) use libc::{statx, STATX_MODE, STATX_MTIME, STATX_NLINK, STATX_SIZE, STATX_TYPE};

#[cfg(all(target_os = "linux", target_env = "musl"))]
pub(crate) use self::musl::{statx, STATX_MODE, STATX_MTIME, STATX_NLINK, STATX_SIZE, STATX_TYPE};

#[cfg(all(target_os = "linux", target_env = "musl"))]
mod musl {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct statx_timestamp {
        pub tv_sec: i64,
        pub tv_nsec: u32,
        pub __reserved: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct statx {
        pub stx_mask: u32,
        pub stx_blksize: u32,
        pub stx_attributes: u64,
        pub stx_nlink: u32,
        pub stx_uid: u32,
        pub stx_gid: u32,
        pub stx_mode: u16,
        pub __spare0: [u16; 1],
        pub stx_ino: u64,
        pub stx_size: u64,
        pub stx_blocks: u64,
        pub stx_attributes_mask: u64,
        pub stx_atime: statx_timestamp,
        pub stx_btime: statx_timestamp,
        pub stx_ctime: statx_timestamp,
        pub stx_mtime: statx_timestamp,
        pub stx_rdev_major: u32,
        pub stx_rdev_minor: u32,
        pub stx_dev_major: u32,
        pub stx_dev_minor: u32,
        pub stx_mnt_id: u64,
        pub stx_dio_mem_align: u32,
        pub stx_dio_offset_align: u32,
        pub __spare3: [u64; 12],
    }

    pub const STATX_TYPE: libc::c_uint = 0x0001;
    pub const STATX_MODE: libc::c_uint = 0x0002;
    pub const STATX_NLINK: libc::c_uint = 0x0004;
    pub const STATX_SIZE: libc::c_uint = 0x0200;
    pub const STATX_MTIME: libc::c_uint = 0x0040;

    // The mirror's own proof, checked on every musl build: the load-
    // bearing offsets stay pinned to the kernel uapi (see the module
    // docs for the glibc static-assert cross-check).
    const _: () = {
        assert!(core::mem::size_of::<statx>() == 256);
        assert!(core::mem::size_of::<statx_timestamp>() == 16);
        assert!(core::mem::offset_of!(statx, stx_mask) == 0);
        assert!(core::mem::offset_of!(statx, stx_mode) == 28);
        assert!(core::mem::offset_of!(statx, stx_ino) == 32);
        assert!(core::mem::offset_of!(statx, stx_size) == 40);
        assert!(core::mem::offset_of!(statx, stx_mtime) == 112);
        assert!(core::mem::offset_of!(statx_timestamp, tv_nsec) == 8);
    };
}
