//! LimniFS backend contract cases (spec 20 §8): the backend-pair golden
//! class — the SAME logical tree the dwarfs fixture holds is imaged with
//! limnifs-write IN-PROCESS (no `limni` binary) and mounted through the
//! `tebako_fs_*` ABI, and the two mounts must answer identically
//! (readdir sets, stat types/sizes, file contents) — byte-identity is
//! per-backend, semantics are shared. Plus the region/memory mount
//! constructors and the named-error surfaces.
//!
//! Windows ships a dwarfs-only tfs (TODO.v2-1/02): the limnifs cases are
//! POSIX-only.
#![cfg(not(windows))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tebako_contract_tests::TempDir;

static LOCK: Mutex<()> = Mutex::new(());

const JUNK_SIZE: u64 = 1000;

fn fixture_image() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/simple.dwarfs")
        .canonicalize()
        .expect("simple.dwarfs fixture must exist")
}

fn c(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

unsafe fn errno() -> i32 {
    unsafe { tfs::c_api::tebako_get_errno() }
}

struct F {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    /// The limnifs single-file image (manifest + appended slabs).
    image: Vec<u8>,
    image_path: PathBuf,
    combined_path: PathBuf,
    mount_point: String,
}

/// Snapshot of one mount's logical answers: path → (is_dir, size,
/// content for regular files). THE parity surface (spec 20 §8).
type Tree = BTreeMap<String, (bool, i64, Vec<u8>)>;

/// Mount `path` at `mp` through the C ABI and snapshot every answer the
/// VFS gives for the whole tree, then unmount.
unsafe fn snapshot(path: &std::ffi::CStr, mp: &str) -> Tree {
    let rc = unsafe { tfs::c_api::tebako_fs_init_from_file(path.as_ptr(), c(mp).as_ptr()) };
    assert_eq!(rc, 0, "mount of {} must succeed", path.to_string_lossy());
    let mut tree = Tree::new();
    unsafe { walk_into(mp, &mut tree) };
    unsafe { tfs::c_api::tebako_fs_unmount() };
    tree
}

unsafe fn walk_into(dir: &str, tree: &mut Tree) {
    let d = unsafe { tfs::c_api::tebako_fs_opendir(c(dir).as_ptr()) };
    assert!(!d.is_null(), "opendir {dir}");
    loop {
        let entry = unsafe { tfs::c_api::tebako_fs_readdir(d) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        if name == "." || name == ".." {
            continue;
        }
        let path = format!("{dir}/{name}");
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { tfs::c_api::tebako_fs_stat(c(&path).as_ptr(), &mut st) },
            0
        );
        let is_dir = (st.st_mode & libc::S_IFMT) == libc::S_IFDIR as _;
        if is_dir {
            tree.insert(path.clone(), (true, 0, Vec::new()));
            unsafe { walk_into(&path, tree) };
        } else {
            let mut content = vec![0u8; st.st_size as usize];
            let fd = unsafe { tfs::c_api::tebako_fs_open(c(&path).as_ptr(), libc::O_RDONLY) };
            assert!(fd > 0, "open {path}");
            let n = unsafe {
                tfs::c_api::tebako_fs_read(fd, content.as_mut_ptr().cast(), content.len())
            };
            assert_eq!(n as i64, st.st_size, "short read on {path}");
            assert_eq!(unsafe { tfs::c_api::tebako_fs_close(fd) }, 0);
            tree.insert(path, (false, st.st_size, content));
        }
    }
    assert_eq!(unsafe { tfs::c_api::tebako_fs_closedir(d) }, 0);
}

/// Materialize a snapshot onto the host (the limnifs writer's input).
fn materialize(tree: &Tree, mp: &str, dest: &Path) {
    for (path, (is_dir, _, content)) in tree {
        let rel = path
            .strip_prefix(&format!("{mp}/"))
            .expect("snapshot paths are mount-relative");
        let host = dest.join(rel);
        if *is_dir {
            std::fs::create_dir_all(&host).unwrap();
        } else {
            std::fs::write(&host, content).unwrap();
        }
    }
}

/// The tebako single-file limnifs layout (spec 20 §4): the writer's
/// manifest bytes verbatim + every slab appended in slab-ordinal order.
/// Dictionaries off — the v1 backend resolves the fixed section order.
fn build_limnifs_image(src: &Path) -> Vec<u8> {
    let mut config = limnifs_write::WriteConfig::default_v0_1();
    config.dictionaries.enabled = false;
    let artifact = limnifs_write::write_directory_with_config(src, &config).expect("limnifs write");
    assert!(artifact.metadata_sidecar.is_none());
    let mut image = artifact.bytes;
    for slab in &artifact.slabs {
        image.extend_from_slice(&slab.bytes);
    }
    image
}

fn setup() -> (F, Tree) {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { tfs::c_api::tebako_fs_unmount() };

    let tmp = TempDir::new("limnifs");
    let mp = "/__limnifs_contract__";

    // The parity oracle: snapshot the dwarfs fixture's tree, materialize
    // it, and image it with limnifs-write — same tree in.
    let dwarfs_tree = unsafe { snapshot(&c(&fixture_image().to_string_lossy()), mp) };
    assert!(!dwarfs_tree.is_empty(), "the dwarfs fixture is non-empty");
    let src = tmp.0.join("src");
    std::fs::create_dir_all(&src).unwrap();
    materialize(&dwarfs_tree, mp, &src);

    let image = build_limnifs_image(&src);
    let image_path = tmp.0.join("fs.tfs");
    std::fs::write(&image_path, &image).unwrap();

    // junk prefix (deliberately not a valid archive magic) + image.
    let combined_path = tmp.0.join("combined.bin");
    let mut combined = Vec::new();
    for i in 0..JUNK_SIZE {
        combined.push(b'A' + ((i * 7) % 26) as u8);
    }
    combined.extend_from_slice(&image);
    std::fs::write(&combined_path, &combined).unwrap();

    (
        F {
            _guard: guard,
            _tmp: tmp,
            image,
            image_path,
            combined_path,
            mount_point: mp.to_string(),
        },
        dwarfs_tree,
    )
}

// ===================================================================

/// The backend-pair golden class (spec 20 §8): same tree in → same
/// logical VFS answers out.
#[test]
fn backend_pair_golden_parity_vs_dwarfs() {
    let (f, dwarfs_tree) = setup();
    let limnifs_tree = unsafe { snapshot(&c(&f.image_path.to_string_lossy()), &f.mount_point) };
    assert_eq!(
        limnifs_tree, dwarfs_tree,
        "the limnifs mount must answer identically to the dwarfs mount"
    );

    // The backend names itself for the detection-derived label (spec 15).
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file(
            c(f.image_path.to_str().unwrap()).as_ptr(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert!(!bn.is_null());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"LimniFS"
    );
    unsafe { tfs::c_api::tebako_fs_unmount() };
}

/// Region and memory mounts (spec 11 §5's mount-source kinds — one
/// `&[u8]` core serves them all).
#[test]
fn offset_and_memory_mounts() {
    let (f, dwarfs_tree) = setup();

    // Region: explicit offset+length into the combined file.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init_from_file_at(
            c(f.combined_path.to_str().unwrap()).as_ptr(),
            JUNK_SIZE,
            f.image.len() as u64,
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    let bn = unsafe { tfs::c_api::tebako_get_backend_name() };
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(bn) }.to_bytes(),
        b"LimniFS"
    );
    let mut tree = Tree::new();
    unsafe { walk_into(&f.mount_point, &mut tree) };
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(tree, dwarfs_tree);

    // Memory.
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            f.image.as_ptr().cast(),
            f.image.len(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, 0);
    assert!(unsafe { tfs::c_api::tebako_get_archive_path() }.is_null());
    let mut tree = Tree::new();
    unsafe { walk_into(&f.mount_point, &mut tree) };
    unsafe { tfs::c_api::tebako_fs_unmount() };
    assert_eq!(tree, dwarfs_tree);
}

/// A truncated limnifs image fails the mount with the NAMED EINVAL —
/// never a silent re-route (spec 20 §3/§5).
#[test]
fn corrupt_image_mount_fails_named() {
    let (f, _) = setup();
    let truncated = f.image[..f.image.len() / 2].to_vec();
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            truncated.as_ptr().cast(),
            truncated.len(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::EINVAL);
    assert_eq!(unsafe { tfs::c_api::tebako_is_initialized() }, 0);

    // Garbage with the LMFS magic but a zeroed section behind it: the
    // feature-flags section version is unsupported → the NAMED ENOTSUP
    // (spec 20 §4's UnsupportedFeature row), never a crash.
    let mut garbage = b"LMFS".to_vec();
    garbage.resize(512, 0);
    let rc = unsafe {
        tfs::c_api::tebako_fs_init(
            garbage.as_ptr().cast(),
            garbage.len(),
            c(&f.mount_point).as_ptr(),
        )
    };
    assert_eq!(rc, -1);
    assert_eq!(unsafe { errno() }, libc::ENOTSUP);
}
