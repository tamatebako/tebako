//! Contract-test package root (tests live in `tests/`).

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
    use std::io::Write as _;

    let file = std::fs::File::create(path).expect("create fixture zip");
    let mut w = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();

    w.add_directory("content/", opts).unwrap();
    w.start_file("content/hello.txt", opts).unwrap();
    w.write_all(b"Hello, World!").unwrap();
    w.start_file("content/data.bin", opts).unwrap();
    w.write_all(&vec![b'X'; 1024]).unwrap();
    w.add_directory("content/subdir/", opts).unwrap();
    w.start_file("content/subdir/nested.txt", opts).unwrap();
    w.write_all(b"Nested file content").unwrap();
    w.start_file("content/empty.txt", opts).unwrap();
    w.write_all(b"").unwrap();
    w.finish().unwrap();
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
