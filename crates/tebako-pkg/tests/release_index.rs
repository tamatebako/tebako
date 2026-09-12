//! `release-index` (roadmap 85 §5): the shard→monolith derivation is a
//! golden byte-exact render, the CLI surfaces (`--out`, stdout, `--check`)
//! behave, and drift/malformed/selector cases are named errors.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tebako_contract_tests::TempDir;

const EXE_A: &str = "tebako-runtime-0.16.0-3.3.12-linux-x86_64";
const EXE_B: &str = "tebako-runtime-0.16.0-4.0.6-macos-arm64";

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako-pkg"))
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The raw shard body (the factory's 2-space-indented bare object, with a
/// trailing newline — the render rstrips it).
fn shard_body(ruby: &str, platform: &str, exe_sha: &str, image_sha: &str) -> String {
    format!(
        r#"{{
  "tebako_version": "0.16.0",
  "contract_era": 2,
  "contract_version": 2,
  "ruby_version": "{ruby}",
  "platform": "{platform}",
  "filename": "tebako-runtime-0.16.0-{ruby}-{platform}",
  "sha256": "{exe_sha}",
  "size_bytes": 1111111,
  "mount_root": "/__tfs__",
  "image_layout": "v2",
  "image": {{
    "filename": "tebako-runtime-0.16.0-{ruby}-{platform}.tfs",
    "sha256": "{image_sha}",
    "size_bytes": 2222222
  }}
}}
"#
    )
}

/// The golden expectation for one manifest entry: the shard body with
/// every line re-indented by 2 spaces, written out literally.
fn entry_block(ruby: &str, platform: &str, exe_sha: &str, image_sha: &str) -> String {
    format!(
        r#"  {{
    "tebako_version": "0.16.0",
    "contract_era": 2,
    "contract_version": 2,
    "ruby_version": "{ruby}",
    "platform": "{platform}",
    "filename": "tebako-runtime-0.16.0-{ruby}-{platform}",
    "sha256": "{exe_sha}",
    "size_bytes": 1111111,
    "mount_root": "/__tfs__",
    "image_layout": "v2",
    "image": {{
      "filename": "tebako-runtime-0.16.0-{ruby}-{platform}.tfs",
      "sha256": "{image_sha}",
      "size_bytes": 2222222
    }}
  }}"#
    )
}

struct Fixture {
    sha_a: String,
    sha_b: String,
    sha_c: String,
    sha_d: String,
    sha_e: String,
}

impl Fixture {
    fn new() -> Fixture {
        Fixture {
            sha_a: "a".repeat(64),
            sha_b: "b".repeat(64),
            sha_c: "c".repeat(64),
            sha_d: "d".repeat(64),
            // The sidecar-declared sha for A's image (wins over sha_b).
            sha_e: "e".repeat(64),
        }
    }

    fn expected_manifest(&self) -> String {
        format!(
            "[\n{},\n{}\n]\n",
            entry_block("3.3.12", "linux-x86_64", &self.sha_a, &self.sha_b),
            entry_block("4.0.6", "macos-arm64", &self.sha_c, &self.sha_d)
        )
    }

    fn expected_sums(&self) -> String {
        format!(
            "{}  {EXE_A}\n{}  {EXE_A}.tfs\n{}  {EXE_B}\n{}  {EXE_B}.tfs\n",
            self.sha_a, self.sha_e, self.sha_c, self.sha_d
        )
    }

    /// The release directory: two shards, ONE `.sha256` sidecar (A's
    /// image — the sidecar-wins path), no payload binaries needed.
    fn write(&self, dir: &Path) {
        std::fs::write(
            dir.join(format!("{EXE_A}.manifest.json")),
            shard_body("3.3.12", "linux-x86_64", &self.sha_a, &self.sha_b),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{EXE_B}.manifest.json")),
            shard_body("4.0.6", "macos-arm64", &self.sha_c, &self.sha_d),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{EXE_A}.tfs.sha256")),
            format!("{}  {EXE_A}.tfs\n", self.sha_e),
        )
        .unwrap();
    }

    fn url(dir: &Path) -> String {
        tebako_http::file_url(dir)
    }
}

#[test]
fn the_library_derives_the_monoliths_from_a_shard_space() {
    let w = TempDir::new("release-index-derive");
    let f = Fixture::new();
    f.write(&w.0);

    let space = tebako_pkg::release_index::open_release(&Fixture::url(&w.0)).unwrap();
    let derived = tebako_pkg::release_index::derive(space.as_ref()).unwrap();
    assert_eq!(derived.entries, 2);
    assert_eq!(derived.manifest_json, f.expected_manifest());
    assert_eq!(derived.sha256sums_txt, f.expected_sums());
}

#[test]
fn the_cli_out_form_writes_byte_exact_monoliths() {
    let w = TempDir::new("release-index-out");
    let f = Fixture::new();
    f.write(&w.0);
    let out = w.0.join("derived");

    let (rc, stdout, stderr) = run(
        &[
            "release-index",
            &Fixture::url(&w.0),
            "--out",
            out.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, stderr.as_str()), (0, ""));
    assert_eq!(
        stdout,
        format!(
            "wrote manifest.json + SHA256SUMS.txt (2 entries) to {}\n",
            out.display()
        )
    );
    assert_eq!(
        std::fs::read_to_string(out.join("manifest.json")).unwrap(),
        f.expected_manifest()
    );
    assert_eq!(
        std::fs::read_to_string(out.join("SHA256SUMS.txt")).unwrap(),
        f.expected_sums()
    );
}

#[test]
fn the_cli_default_form_prints_the_labeled_documents() {
    let w = TempDir::new("release-index-stdout");
    let f = Fixture::new();
    f.write(&w.0);

    let (rc, stdout, stderr) = run(&["release-index", &Fixture::url(&w.0)], &w.0);
    assert_eq!((rc, stderr.as_str()), (0, ""));
    let expected = format!(
        "# manifest.json\n{}# SHA256SUMS.txt\n{}",
        f.expected_manifest(),
        f.expected_sums()
    );
    assert_eq!(stdout, expected);
}

#[test]
fn check_byte_matches_when_the_published_monoliths_agree() {
    let w = TempDir::new("release-index-check-ok");
    let f = Fixture::new();
    f.write(&w.0);
    std::fs::write(w.0.join("manifest.json"), f.expected_manifest()).unwrap();
    std::fs::write(w.0.join("SHA256SUMS.txt"), f.expected_sums()).unwrap();

    let (rc, stdout, stderr) = run(&["release-index", &Fixture::url(&w.0), "--check"], &w.0);
    assert_eq!((rc, stderr.as_str()), (0, ""));
    assert!(stdout.contains("manifest.json: byte-identical with the derived index"));
    assert!(stdout.contains("SHA256SUMS.txt: byte-identical with the derived index"));
}

#[test]
fn check_names_the_first_missing_entry_on_drift() {
    let w = TempDir::new("release-index-check-drift");
    let f = Fixture::new();
    f.write(&w.0);
    // The stale monolith: only the first entry (B's shard is newer).
    let stale = format!(
        "[\n{}\n]\n",
        entry_block("3.3.12", "linux-x86_64", &f.sha_a, &f.sha_b)
    );
    std::fs::write(w.0.join("manifest.json"), stale).unwrap();

    let (rc, _, stderr) = run(&["release-index", &Fixture::url(&w.0), "--check"], &w.0);
    assert_eq!(rc, 1);
    assert!(
        stderr.contains(&format!(
            "drift: manifest.json is stale — entry \"{EXE_B}\" is in the shards but not in the published manifest.json"
        )),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn check_succeeds_when_no_monoliths_exist() {
    let w = TempDir::new("release-index-check-none");
    let f = Fixture::new();
    f.write(&w.0);

    let (rc, stdout, stderr) = run(&["release-index", &Fixture::url(&w.0), "--check"], &w.0);
    assert_eq!((rc, stderr.as_str()), (0, ""));
    assert_eq!(stdout, "no monoliths to check against (post-85 line)\n");
}

#[test]
fn a_malformed_shard_is_a_named_error() {
    let w = TempDir::new("release-index-malformed");
    let f = Fixture::new();
    f.write(&w.0);
    std::fs::write(
        w.0.join("zz-bad.manifest.json"),
        "{\n  \"filename\": \"zz-bad\"\n}\n",
    )
    .unwrap();

    let (rc, _, stderr) = run(&["release-index", &Fixture::url(&w.0)], &w.0);
    assert_eq!(rc, 1);
    assert!(
        stderr.contains("malformed shard zz-bad.manifest.json: no \"sha256\""),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn an_artifact_selector_is_a_named_error_without_network() {
    let w = TempDir::new("release-index-artifact");
    let (rc, _, stderr) = run(
        &["release-index", "tfs:github:owner/repo:1.0#thing.tfs"],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert!(
        stderr.contains("drop the #thing.tfs artifact selector"),
        "unexpected stderr: {stderr}"
    );
}
