//! rust-at-tool: a rust-built dynamic tool proving the preload shim's
//! *at coverage end-to-end (roadmap 39). `std::fs` exercises the base
//! surface (open/read/stat — what `std::fs::Metadata` lowers to on
//! glibc); the direct `libc::fstatat` / `libc::statx` calls exercise the
//! *at family itself. The in-image path does not exist on the host, so
//! every successful answer can only have come through the VFS.
//!
//! usage: rust-at-tool <memfs-path> <host-relative-path>
//! (run with cwd = the host dir the relative path resolves against)

use std::ffi::CString;

fn main() {
    let mut args = std::env::args().skip(1);
    let memfs = CString::new(args.next().expect("usage: rust-at-tool <memfs-path> <host-rel>"))
        .expect("memfs path is not NUL-free");
    let host_rel = CString::new(args.next().expect("usage: rust-at-tool <memfs-path> <host-rel>"))
        .expect("host path is not NUL-free");
    let memfs_str = memfs.to_str().unwrap();

    // The base surface from Rust (open/read/stat).
    let data = std::fs::read_to_string(memfs_str).expect("read in-image data");
    print!("{data}");
    let md = std::fs::metadata(memfs_str).expect("metadata of in-image data");
    println!("META:{}", md.len());

    // SAFETY: all output buffers are valid, initialized (zeroed) locals;
    // the CStrings outlive the calls.
    unsafe {
        // fstatat(AT_FDCWD, <memfs path>) — served by the shim (the path
        // is absent on the host; ENOENT would fail the assert).
        let mut st: libc::stat = std::mem::zeroed();
        let rc = libc::fstatat(libc::AT_FDCWD, memfs.as_ptr(), &mut st, 0);
        assert_eq!(rc, 0, "fstatat: {}", std::io::Error::last_os_error());
        println!("FSTATAT:{}", st.st_size);

        // statx(AT_FDCWD, <memfs path>) — the modern linux *at stat call.
        #[cfg(target_os = "linux")]
        {
            let mut sx: libc::statx = std::mem::zeroed();
            let rc = libc::statx(
                libc::AT_FDCWD,
                memfs.as_ptr(),
                0,
                libc::STATX_BASIC_STATS,
                &mut sx,
            );
            assert_eq!(rc, 0, "statx: {}", std::io::Error::last_os_error());
            println!("STATX:{}", sx.stx_size);
        }

        // The AT_FDCWD regression pin (the 4.0 lesson): a cwd-relative
        // HOST path through fstatat must pass through to the host — an
        // ungated is_memfs_fd(AT_FDCWD) branch answers ENOTDIR instead.
        let mut st: libc::stat = std::mem::zeroed();
        let rc = libc::fstatat(libc::AT_FDCWD, host_rel.as_ptr(), &mut st, 0);
        assert_eq!(
            rc,
            0,
            "fstatat(AT_FDCWD, host-relative): {}",
            std::io::Error::last_os_error()
        );
        println!("REL:{}", st.st_size);
    }
}
