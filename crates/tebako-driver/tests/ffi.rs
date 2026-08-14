//! The C entry surface (spec 17 §1): `tebako_driver_boot` rewrites argv
//! in place on success and returns a named exit code on failure;
//! `tebako_driver_contract_version` reports the compiled-in contract.
//! Serialized on LOCK — the boot mounts into the process-global context.

#![allow(unsafe_code)]

use std::ffi::{CStr, CString};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use libc::{c_char, c_int};
use tfs::context::context;

static LOCK: Mutex<()> = Mutex::new(());

struct Guard {
    _guard: MutexGuard<'static, ()>,
    tmp: TempDir,
}

impl Guard {
    fn path(&self) -> &Path {
        self.tmp.path()
    }
}

fn guard(tag: &str) -> Guard {
    let g = LOCK.lock().unwrap();
    let tmp = TempDir::new(tag);
    // The in-process boots are the re-exec'd child by definition (spec 22
    // §2 "Phase 1 delivery"): the sentinel skips the macOS boot-head
    // self-insertion — a test process must never execv itself away. The
    // functional re-exec proof lives in tests/interpose.rs.
    std::env::set_var("TEBAKO_LOADER_INTERPOSED", "1");
    context().write().unwrap().unmount();
    context()
        .write()
        .unwrap()
        .set_host_policy(tfs::policy::HostPolicy::open(), None);
    Guard { _guard: g, tmp }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-driver-ffi-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_payload_image(dir: &Path) -> PathBuf {
    let p = dir.join("payload.tfs");
    let file = std::fs::File::create(&p).expect("create zip");
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    w.add_directory("bin/", opts).unwrap();
    w.start_file("bin/app", opts).unwrap();
    w.write_all(b"#!/usr/bin/env ruby\nputs 'hi'\n").unwrap();
    w.finish().unwrap();
    p
}

struct CArgv {
    _strings: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl CArgv {
    fn new(args: &[&str]) -> CArgv {
        let strings: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
        let mut ptrs: Vec<*mut c_char> =
            strings.iter().map(|c| c.as_ptr() as *mut c_char).collect();
        ptrs.push(std::ptr::null_mut());
        CArgv {
            _strings: strings,
            ptrs,
        }
    }
}

#[test]
fn contract_version_is_2() {
    assert_eq!(
        unsafe { tebako_driver::ffi::tebako_driver_contract_version() },
        2
    );
}

#[test]
fn tebako_main_miniruby_passes_through_and_flags_it() {
    let _g = guard("miniruby");
    std::env::remove_var("TEBAKO_CONTRACT_VERSION");
    let mut cargv = CArgv::new(&["/build/ruby/miniruby", "-v"]);
    let mut argc: c_int = 2;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
    let rc = unsafe { tebako_driver::ffi::tebako_main(&mut argc, &mut argvp) };
    assert_eq!(rc, 0);
    assert_eq!(argc, 2, "argv untouched for miniruby");
    assert_eq!(
        unsafe { tebako_driver::ffi::tebako_is_running_miniruby() },
        -1
    );
    assert!(
        std::env::var("TEBAKO_CONTRACT_VERSION").is_err(),
        "miniruby exports nothing"
    );
    assert!(
        !context().read().unwrap().is_mounted(),
        "miniruby mounts nothing"
    );
}

#[test]
fn tebako_main_boots_with_the_ruby_root_and_exports_the_contract() {
    let g = guard("tebako-main");
    std::env::remove_var("TEBAKO_CONTRACT_VERSION");
    let payload = write_payload_image(g.path());
    let triple = format!("{}:0:/", payload.display());
    let mut cargv = CArgv::new(&[
        "ruby",
        "--tebako-image",
        &triple,
        "--tebako-entry",
        "/bin/app",
    ]);
    let mut argc: c_int = 5;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
    let rc = unsafe { tebako_driver::ffi::tebako_main(&mut argc, &mut argvp) };
    assert_eq!(rc, 0);
    assert_eq!(argc, 2, "argv0 (the program name) + the resolved entry");
    let program = unsafe { CStr::from_ptr(*argvp) }.to_string_lossy();
    assert_eq!(program, "ruby", "argv0 stays the interpreter's name");
    let entry = unsafe { CStr::from_ptr(*argvp.offset(1)) }.to_string_lossy();
    assert_eq!(entry, "/bin/app");
    assert_eq!(
        std::env::var("TEBAKO_CONTRACT_VERSION").as_deref(),
        Ok("2"),
        "the runtime exports its contract (roadmap 45)"
    );
    assert_eq!(
        unsafe { tebako_driver::ffi::tebako_is_running_miniruby() },
        0
    );
    let mp = unsafe { CStr::from_ptr(tebako_driver::ffi::tebako_mount_point()) }.to_string_lossy();
    // The ruby runtime root convention: the memfs is its own drive on
    // windows, a root-level dir elsewhere (the msys 13/21 boot-smoke
    // class was this constant disagreeing with the factory convention).
    #[cfg(not(windows))]
    assert_eq!(mp, "/__tfs__");
    #[cfg(windows)]
    assert_eq!(mp, "A:/t");
    let pwd =
        unsafe { CStr::from_ptr(tebako_driver::ffi::tebako_original_pwd()) }.to_string_lossy();
    assert!(!pwd.is_empty(), "the original cwd is recorded");
    assert!(context().read().unwrap().is_mounted());
}

#[test]
fn tebako_run_is_a_named_v1_migration_error() {
    let _g = guard("tebako-run");
    let mut cargv = CArgv::new(&["ruby", "--tebako-run", "app.tfs"]);
    let mut argc: c_int = 3;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();
    let rc = unsafe { tebako_driver::ffi::tebako_main(&mut argc, &mut argvp) };
    assert_eq!(rc, 65);
    assert!(!context().read().unwrap().is_mounted());
}

#[test]
fn boot_rewrites_argv_in_place() {
    let g = guard("rewrite");
    let payload = write_payload_image(g.path());
    let triple = format!("{}:0:/", payload.display());
    let mut cargv = CArgv::new(&[
        "ruby",
        "--tebako-image",
        &triple,
        "--tebako-entry",
        "/bin/app",
        "--version",
    ]);
    let root = CString::new("/__tfs__").unwrap();
    let mut argc: c_int = 6;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();

    let rc =
        unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
    assert_eq!(rc, 0);
    assert_eq!(argc, 3);
    let rewritten: Vec<String> = (0..argc as isize)
        .map(|i| {
            unsafe { CStr::from_ptr(*argvp.offset(i)) }
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(rewritten, vec!["ruby", "/bin/app", "--version"]);
    // The mount happened (the entry resolved inside it).
    assert!(context().read().unwrap().is_mounted());
}

#[test]
fn boot_failure_returns_the_named_code_and_leaves_argv_alone() {
    let g = guard("fail");
    let triple = format!("{}:0:/", g.path().join("nope.tfs").display());
    let mut cargv = CArgv::new(&[
        "ruby",
        "--tebako-image",
        &triple,
        "--tebako-entry",
        "/bin/app",
    ]);
    let root = CString::new("/__tfs__").unwrap();
    let mut argc: c_int = 5;
    let mut argvp: *mut *mut c_char = cargv.ptrs.as_mut_ptr();

    let rc =
        unsafe { tebako_driver::ffi::tebako_driver_boot(&mut argc, &mut argvp, root.as_ptr()) };
    assert_eq!(rc, 69);
    assert_eq!(argc, 5, "argc untouched on failure");
    assert!(!context().read().unwrap().is_mounted());
}
