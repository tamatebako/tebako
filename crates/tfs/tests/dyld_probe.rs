//! The `tebako_fs_exec_dyld_insertion_honored` FFI surface (packed-
//! metanorma#305): argument validation, the errno channel, and the
//! end-to-end wiring from the C ABI to the Mach-O probe. The probe's
//! verdict matrix itself is unit-tested in `macho_dyld`; this file pins
//! the surface contract the interpreter's spawn hook consumes.

#![allow(unsafe_code)]

use std::ffi::CString;
use tfs::c_api;
use tfs::errno::get_errno;

fn write_fixture(dir: &std::path::Path, name: &str, bytes: &[u8]) -> CString {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    CString::new(path.to_string_lossy().as_ref()).unwrap()
}

/// A thin 64-bit Mach-O with one LC_CODE_SIGNATURE whose superblob holds
/// exactly one slot-0 code directory carrying `cd_flags` (no entitlement
/// blobs — the unentitled shape).
fn mach_o_hardened_no_entitlement() -> Vec<u8> {
    let superblob_off = 48usize;
    let mut b = Vec::new();
    b.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]); // MH_MAGIC_64 (stored LE)
    b.extend_from_slice(&[0u8; 12]); // cputype / cpusubtype / filetype
    b.extend_from_slice(&1u32.to_le_bytes()); // ncmds
    b.extend_from_slice(&16u32.to_le_bytes()); // sizeofcmds
    b.extend_from_slice(&[0u8; 8]); // flags + reserved
    b.extend_from_slice(&0x1du32.to_le_bytes()); // LC_CODE_SIGNATURE
    b.extend_from_slice(&16u32.to_le_bytes()); // cmdsize
    b.extend_from_slice(&(superblob_off as u32).to_le_bytes()); // dataoff
    b.extend_from_slice(&[0u8; 4]); // datasize (never read)
    assert_eq!(b.len(), superblob_off);
    // Superblob: magic, length, count=1, (type 0, offset). The index
    // offset is relative to the superblob start; the blob follows the
    // 12-byte header + one 8-byte index entry at relative 20.
    b.extend_from_slice(&0xFADE_0CC0u32.to_be_bytes());
    b.extend_from_slice(&40u32.to_be_bytes()); // total length
    b.extend_from_slice(&1u32.to_be_bytes());
    b.extend_from_slice(&0u32.to_be_bytes()); // CSSLOT_CODE_DIRECTORY
    b.extend_from_slice(&20u32.to_be_bytes()); // blob offset (relative)
    // Code directory: magic, length, version, flags (0x10000 CS_RUNTIME).
    b.extend_from_slice(&0xFADE_0C02u32.to_be_bytes());
    b.extend_from_slice(&20u32.to_be_bytes());
    b.extend_from_slice(&0x20500u32.to_be_bytes());
    b.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    b.extend_from_slice(&[0u8; 4]);
    b
}

#[test]
fn null_and_non_utf8_arguments_answer_attempt_with_einval() {
    // NULL: the pre-probe default (attempt the injection), errno EINVAL.
    assert_eq!(unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(std::ptr::null()) }, 1);
    assert_eq!(get_errno(), libc::EINVAL);
    // Non-UTF-8: same contract.
    let bad = CString::new(vec![0x2f, 0xff, 0xfe]).unwrap();
    assert_eq!(unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(bad.as_ptr()) }, 1);
    assert_eq!(get_errno(), libc::EINVAL);
}

#[test]
fn an_unreadable_path_answers_attempt() {
    let missing = CString::new("/definitely/not/here/tee").unwrap();
    assert_eq!(unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(missing.as_ptr()) }, 1);
}

#[test]
fn a_non_macho_file_answers_honored() {
    let dir = std::env::temp_dir().join(format!("tfs-dyld-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = write_fixture(&dir, "script.sh", b"#!/bin/sh\necho hi\n");
    assert_eq!(unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(script.as_ptr()) }, 1);
    assert_eq!(get_errno(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hardened_unentitled_image_verdicts_by_host_os() {
    let dir = std::env::temp_dir().join(format!("tfs-dyld-probe-h-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = write_fixture(&dir, "hardened", &mach_o_hardened_no_entitlement());
    let got = unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(exe.as_ptr()) };
    assert_eq!(get_errno(), 0);
    #[cfg(target_os = "macos")]
    {
        let major = {
            // kern.osproductversion's major — the same sysctl the probe
            // reads (the test asserts the contract, so it asks the OS
            // itself rather than restating a number).
            let mut buf = [0u8; 32];
            let mut size = buf.len();
            let rc = unsafe {
                libc::sysctlbyname(
                    c"kern.osproductversion".as_ptr(),
                    buf.as_mut_ptr().cast(),
                    &mut size,
                    std::ptr::null_mut(),
                    0,
                )
            };
            assert_eq!(rc, 0);
            let end = buf.iter().position(|b| *b == 0).unwrap_or(size);
            std::str::from_utf8(&buf[..end])
                .unwrap()
                .split('.')
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        assert_eq!(got, i32::from(major >= 15), "darwin {major} law");
    }
    #[cfg(not(target_os = "macos"))]
    {
        assert_eq!(got, 1, "ELF has no entitlement plane");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(target_os = "macos")]
#[test]
fn the_test_binary_itself_is_adhoc_and_honored() {
    let exe = std::env::current_exe().unwrap();
    let c = CString::new(exe.to_string_lossy().as_ref()).unwrap();
    assert_eq!(unsafe { c_api::tebako_fs_exec_dyld_insertion_honored(c.as_ptr()) }, 1);
    assert_eq!(get_errno(), 0);
}
