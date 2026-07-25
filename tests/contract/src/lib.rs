//! Contract-test package root (tests live in `tests/`).

/// Build a zip from an in-memory entry list. `dirs` are explicit directory
/// entries; `files` are (path, content) pairs. Mirrors the trees built by
/// the C++ multi-mount fixture (`CApiMultiMountTest::SetUp`).
pub fn build_zip(path: &std::path::Path, dirs: &[&str], files: &[(&str, &[u8])]) {
    use std::io::Write as _;

    let file = std::fs::File::create(path).expect("create zip");
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for d in dirs {
        w.add_directory(*d, opts).unwrap();
    }
    for (name, content) in files {
        w.start_file(name, opts).unwrap();
        w.write_all(content).unwrap();
    }
    w.finish().unwrap();
}

/// Build the standard zip fixture used by the C++ `CApiTest` suite
/// (libtfs `tests/test_c_api.cpp`, `create_test_archive()`), so both
/// implementations are exercised against the SAME tree:
///
/// ```text
/// content/hello.txt           "Hello, World!"
/// content/data.bin            1024 x 'X'
/// content/subdir/nested.txt   "Nested file content"
/// content/empty.txt           ""
/// ```
pub fn build_fixture_zip(path: &std::path::Path) {
    build_zip(
        path,
        &["content/", "content/subdir/"],
        &[
            ("content/hello.txt", b"Hello, World!".as_slice()),
            ("content/data.bin", vec![b'X'; 1024].as_slice()),
            (
                "content/subdir/nested.txt",
                b"Nested file content".as_slice(),
            ),
            ("content/empty.txt", b"".as_slice()),
        ],
    );
}

/// A temporary directory that cleans itself up. Paths are unique per
/// instance (panic-safe under the test harness's parallelism).
pub struct TempDir(pub std::path::PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-rs-contract-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
