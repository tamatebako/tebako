//! CLI-level tests: exit codes, error surfaces, and the verbose outputs
//! through the built `tebako-pkg` binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use tebako_contract_tests::TempDir;

fn bin() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tebako-pkg");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tebako-pkg binary not built (run `cargo build -p tebako-pkg`)")
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

fn patterned_bytes(n: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    let mut x = seed;
    for _ in 0..n {
        x = x.wrapping_mul(31).wrapping_add(17);
        v.push(x);
    }
    v
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn make_pkg(w: &TempDir) -> PathBuf {
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(1024, 0x21)).unwrap();
    let pkg = w.0.join("pkg");
    let (rc, _, err) = run(
        &[
            "bundle",
            "--bootstrap",
            boot.to_str().unwrap(),
            "--image",
            fixture("simple.dwarfs").to_str().unwrap(),
            "--image",
            &format!("{}:/data", fixture("simple.sqfs").display()),
            "-o",
            pkg.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "bundle must succeed");
    pkg
}

#[test]
fn help_and_unknown_command() {
    let w = TempDir::new("cli");
    let (rc, out, _) = run(&["help"], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("tebako-pkg - tebako package (tpkg) trailer surgery"));
    assert!(out.contains("insert-image"));

    let (rc, _, err) = run(&["frobnicate"], &w.0);
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        "Error: Unknown command: frobnicate\nUse 'tebako-pkg help' for usage information\n"
    );
}

#[test]
fn verbose_outputs_match_cpp() {
    let w = TempDir::new("cli2");
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(512, 0x77)).unwrap();
    let pkg = w.0.join("pkg");

    // bundle -v
    let (rc, out, _) = run(
        &[
            "bundle",
            "-v",
            "--bootstrap",
            boot.to_str().unwrap(),
            "--image",
            fixture("simple.dwarfs").to_str().unwrap(),
            "--image",
            fixture("simple.sqfs").to_str().unwrap(),
            "-o",
            pkg.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(
        out,
        format!("Wrote package: {} (2 image slot(s))\n", pkg.display())
    );

    // unbundle -v
    let parts = w.0.join("parts");
    let (rc, out, _) = run(
        &[
            "unbundle",
            "-v",
            pkg.to_str().unwrap(),
            "-o",
            parts.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(
        out,
        format!("Unbundled {} into: {}\n", pkg.display(), parts.display())
    );

    // reassemble -v
    let re = w.0.join("re");
    let (rc, out, _) = run(
        &[
            "reassemble",
            "-v",
            parts.to_str().unwrap(),
            "-o",
            re.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(
        out,
        format!("Reassembled {} into: {}\n", parts.display(), re.display())
    );

    // insert-image -v
    let extra = fixture("nested.sqfs");
    let (rc, out, _) = run(
        &[
            "insert-image",
            "-v",
            pkg.to_str().unwrap(),
            extra.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(
        out,
        format!("Inserted {} into: {}\n", extra.display(), pkg.display())
    );

    // remove-image -v
    let (rc, out, _) = run(&["remove-image", "-v", pkg.to_str().unwrap(), "2"], &w.0);
    assert_eq!(rc, 0);
    assert_eq!(out, format!("Removed slot 2 from: {}\n", pkg.display()));

    // set-runtime -v
    let boot2 = w.0.join("boot2.bin");
    std::fs::write(&boot2, patterned_bytes(256, 0x55)).unwrap();
    let (rc, out, _) = run(
        &[
            "set-runtime",
            "-v",
            pkg.to_str().unwrap(),
            boot2.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 0);
    assert_eq!(
        out,
        format!("Replaced the bootstrap portion of: {}\n", pkg.display())
    );
}

#[test]
fn error_exit_codes_and_messages() {
    let w = TempDir::new("cli3");
    let pkg = make_pkg(&w);

    // remove-image: last-slot guard (remove 1 then 0).
    let one = w.0.join("one");
    std::fs::copy(&pkg, &one).unwrap();
    assert_eq!(
        run(&["remove-image", one.to_str().unwrap(), "1"], &w.0).0,
        0
    );
    let (rc, _, err) = run(&["remove-image", one.to_str().unwrap(), "0"], &w.0);
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        format!(
            "Error: remove-image failed: {}: cannot remove the last image slot (a manifest requires at least one slot)\n",
            one.display()
        )
    );

    // insert-image into a non-package.
    let junk = w.0.join("junk.bin");
    std::fs::write(&junk, patterned_bytes(128, 0x99)).unwrap();
    let (rc, _, err) = run(
        &[
            "insert-image",
            junk.to_str().unwrap(),
            fixture("simple.sqfs").to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        format!(
            "Error: insert-image failed: {}: no tpkg manifest trailer present (not a three-part package)\n",
            junk.display()
        )
    );

    // reassemble without manifest.json.
    let empty = w.0.join("emptydir");
    std::fs::create_dir(&empty).unwrap();
    let (rc, _, err) = run(
        &[
            "reassemble",
            empty.to_str().unwrap(),
            "-o",
            w.0.join("x").to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        format!(
            "Error: reassemble failed: manifest.json not found in {} (not an unbundled package directory)\n",
            empty.display()
        )
    );

    // set-runtime with a missing runtime file.
    let (rc, _, err) = run(
        &["set-runtime", pkg.to_str().unwrap(), "/nonexistent"],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert_eq!(
        err,
        "Error: set-runtime failed: runtime file not found: /nonexistent\n"
    );

    // info on a package: the full golden shape.
    let (rc, out, _) = run(&["info", pkg.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(
        out.contains("Format: tebako three-part package (tpkg v1)"),
        "{out}"
    );
    assert!(
        out.contains("Runtime ref: (none — classic bundle)"),
        "{out}"
    );
    assert!(out.contains("Bootstrap size: 1024 bytes"), "{out}");
    assert!(out.contains("Slots: 2"), "{out}");
    assert!(
        out.contains("format=dwarfs flags=0 mount=/__tebako_memfs__"),
        "{out}"
    );
    assert!(out.contains("format=squashfs flags=0 mount=/data"), "{out}");
    assert!(out.contains("Trailer: valid (magic and crc32 ok)"), "{out}");
}

#[test]
fn flag_forms() {
    let w = TempDir::new("cli4");
    let boot = w.0.join("boot.bin");
    std::fs::write(&boot, patterned_bytes(100, 0x01)).unwrap();
    let pkg = w.0.join("pkg");

    // --flag=value and -o value forms must both work.
    let (rc, _, err) = run(
        &[
            "bundle",
            &format!("--bootstrap={}", boot.display()),
            &format!("--image={}", fixture("simple.dwarfs").display()),
            "--runtime-ref=rt",
            "--lean",
            "--launcher-abi=2",
            "-o",
            pkg.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "--flag=value form");

    let (rc, out, _) = run(&["info", pkg.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Runtime ref: rt"), "{out}");
    assert!(out.contains("Flags: 0x1 (LEAN)"), "{out}");
    assert!(out.contains("Launcher ABI: 2"), "{out}");
    // default bundle is unsigned (signing is opt-in)
    assert!(out.contains("Signature: none"), "{out}");

    // --sign opts into the press-local signature
    let signed_pkg = w.0.join("pkg-signed");
    let (rc, _, err) = run(
        &[
            "bundle",
            &format!("--bootstrap={}", boot.display()),
            &format!("--image={}", fixture("simple.dwarfs").display()),
            "--runtime-ref=rt",
            "--lean",
            "--launcher-abi=2",
            "--sign",
            "-o",
            signed_pkg.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "bundle --sign");
    let (rc, out, _) = run(&["info", signed_pkg.to_str().unwrap()], &w.0);
    assert_eq!(rc, 0);
    assert!(out.contains("Flags: 0x3 (LEAN|SIGNED_V2)"), "{out}");
    assert!(out.contains("Signature: OpenPGP v2"), "{out}");
}
