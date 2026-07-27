//! The encryption verbs end-to-end (spec 10): encrypt → mount → decrypt
//! roundtrip, wrong-key EKEY, selective disclosure, rewrap rotation
//! without touching the bulk, and the plaintext-never-touches-disk scan
//! over the staging area.
//!
//! Recipient keys are minted in-process via rnp (Ed25519 primary +
//! X25519 encryption subkey — the SUITE-1 shape); images are built with
//! `tfs mkimage` from a source tree carrying the `data.yaml` manifest
//! fixture.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tebako_contract_tests::TempDir;
use tfs::Backend as _;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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
        let cand = target.join(profile).join("tfs");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tfs binary not built")
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env(
            "TEBAKO_HOME",
            cwd.join(format!("home-{}", COUNTER.fetch_add(1, Ordering::Relaxed))),
        )
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A minimal data manifest with `encryption: {state: none}` (the
/// data.yaml fixture declares an encrypted state as a spec example —
/// encrypt refuses an already-encrypted source).
const TEST_MANIFEST: &str = "identity:\n  schema_version: 1\n  kind: data\n  name: fixture\n  version: 1.0.0\n  producer: {tool: test, tool_version: 1}\n  created: 2026-07-27T00:00:00Z\n  digest: {tree_hash: \"sha256:0000000000000000000000000000000000000000000000000000000000000000\", blob_sha256: 0000000000000000000000000000000000000000000000000000000000000000}\n  signing: {state: unsigned}\n  encryption: {state: none}\nprovides:\n  mount_semantics: {suggested: /usr/share/fixture}\n  capabilities: {exec: false, read: true}\n";

/// Mint a recipient key pair (files on disk, as the CLI consumes them).
fn mint_recipient(dir: &Path, userid: &str) -> (PathBuf, PathBuf) {
    let ctx = rnp::Context::new().unwrap();
    let primary = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
        .hash(rnp::Hash::Sha256)
        .userid(userid)
        .add_usage(rnp::KeyUsage::Sign)
        .build(&ctx)
        .unwrap();
    rnp::SubkeyBuilder::new(rnp::Algorithm::Ecdh)
        .curve(rnp::Curve::Curve25519)
        .hash(rnp::Hash::Sha256)
        .add_usage(rnp::KeyUsage::EncryptComms)
        .build(&ctx, &primary)
        .unwrap();
    let public = primary
        .export(rnp::ExportFlags::ARMORED | rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS)
        .unwrap();
    let secret = primary
        .export(rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS)
        .unwrap();
    let tag = userid.split(' ').next().unwrap();
    let pub_path = dir.join(format!("{tag}.pub"));
    let sec_path = dir.join(format!("{tag}.key"));
    std::fs::write(&pub_path, public).unwrap();
    std::fs::write(&sec_path, secret).unwrap();
    (pub_path, sec_path)
}

/// The payload files of the test source tree (path, content).
fn payload_files() -> Vec<(&'static str, Vec<u8>)> {
    let mut big = b"PLAINTEXT-CANARY-7f3a9d51: ".to_vec();
    big.extend_from_slice(&[b'A'; 100_000]);
    big.extend_from_slice(b" end of the canary block.");
    vec![
        ("docs/readme.txt", b"the quick brown fox\n".to_vec()),
        (
            "a/secret/contract.txt",
            b"PLAINTEXT-CANARY-contract: for legal eyes only\n".to_vec(),
        ),
        ("a/secret/deep/more.txt", b"nested secret notes\n".to_vec()),
        ("a/other/public.txt", b"public documentation\n".to_vec()),
        ("big.bin", big),
    ]
}

/// Build the source image: the payload files plus the data manifest.
fn mk_source_image(w: &TempDir, name: &str) -> PathBuf {
    let src = w.0.join(format!("src-{name}"));
    std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
    std::fs::write(src.join("__tpkg__/manifest.yaml"), TEST_MANIFEST).unwrap();
    for (rel, content) in payload_files() {
        let dest = src.join(rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(dest, content).unwrap();
    }
    let img = w.0.join(name);
    let (rc, _, err) = run(
        &[
            "mkimage",
            "--format",
            "dwarfs",
            src.to_str().unwrap(),
            "-o",
            img.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mkimage must succeed");
    img
}

/// The plaintext contents of a tar file as (path → bytes).
fn tar_contents(tar_path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut archive = tar::Archive::new(std::fs::File::open(tar_path).unwrap());
    let mut out = Vec::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        if entry.header().entry_type().is_file() {
            use std::io::Read as _;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).unwrap();
            out.push((path, buf));
        }
    }
    out.sort();
    out
}

/// Raw (ciphertext) bytes of every non-`__tpkg__` file of an image.
fn bulk_bytes(image: &Path) -> Vec<(String, Vec<u8>)> {
    let mount = tfs::mount::build_from_file(&image.to_string_lossy(), "/mnt").unwrap();
    let backend = &*mount.backend;
    let mut out = Vec::new();
    fn walk(b: &dyn tfs::Backend, dir: &str, out: &mut Vec<(String, Vec<u8>)>) {
        for e in b.read_dir(dir).unwrap() {
            let path = if dir.is_empty() {
                e.name.clone()
            } else {
                format!("{dir}/{}", e.name)
            };
            if path == "__tpkg__" || path.starts_with("__tpkg__/") {
                continue;
            }
            let st = b.stat(&path).unwrap();
            match st.entry_type {
                tfs::EntryType::Directory => walk(b, &path, out),
                tfs::EntryType::File => {
                    let mut buf = vec![0u8; st.size as usize];
                    let mut off = 0u64;
                    while off < st.size as u64 {
                        let n = b.pread(&path, &mut buf[off as usize..], off).unwrap();
                        assert!(n > 0, "short read on {path}");
                        off += n as u64;
                    }
                    out.push((path, buf));
                }
                _ => {}
            }
        }
    }
    walk(backend, "", &mut out);
    out.sort();
    out
}

// ---------------------------------------------------------------------
// Roundtrip
// ---------------------------------------------------------------------

#[test]
fn encrypt_mount_decrypt_roundtrip() {
    let w = TempDir::new("encrt");
    let src = mk_source_image(&w, "plain.tfs");
    let (pub_a, sec_a) = mint_recipient(&w.0, "alice <a@example.com>");
    let enc_img = w.0.join("enc.tfs");

    let (rc, _, err) = run(
        &[
            "encrypt",
            src.to_str().unwrap(),
            "-o",
            enc_img.to_str().unwrap(),
            "--recipient",
            pub_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "encrypt must succeed");

    // mount reports the opened grant (the unlock surface).
    let (rc, out, err) = run(
        &[
            "mount",
            enc_img.to_str().unwrap(),
            "--key",
            sec_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "mount must succeed: {err}");
    assert!(out.contains("stack: ENC over"), "{out}");
    assert!(out.contains("grant: / → /"), "{out}");
    assert!(out.contains("suite: SUITE-1"), "{out}");
    assert!(
        out.contains("tree_hash (plaintext identity): sha256:"),
        "{out}"
    );

    // decrypt streams the plaintext to a tar; contents round-trip.
    let tar_out = w.0.join("plain.tar");
    let (rc, _, err) = run(
        &[
            "decrypt",
            enc_img.to_str().unwrap(),
            "-o",
            tar_out.to_str().unwrap(),
            "--key",
            sec_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "decrypt must succeed: {err}");
    let contents = tar_contents(&tar_out);
    let mut expected: Vec<(String, Vec<u8>)> = payload_files()
        .into_iter()
        .map(|(p, c)| (p.to_string(), c))
        .collect();
    expected.sort();
    let got: Vec<(String, Vec<u8>)> = contents
        .iter()
        .filter(|(p, _)| !p.starts_with("__tpkg__/"))
        .cloned()
        .collect();
    assert_eq!(got, expected, "decrypted contents must round-trip");

    // The manifest declares encryption, keeps the plaintext tree_hash
    // (spec 10 §2), and carries envelope refs — NEVER keys.
    let manifest_text = contents
        .iter()
        .find(|(p, _)| p == "__tpkg__/manifest.yaml")
        .map(|(_, c)| String::from_utf8(c.clone()).unwrap())
        .unwrap();
    let manifest = tpkg::PayloadManifest::from_yaml(&manifest_text).unwrap();
    assert_eq!(
        manifest.identity.encryption.state,
        tpkg::EncryptionState::Encrypted
    );
    assert_eq!(manifest.identity.encryption.parts.len(), 1);
    assert_eq!(manifest.identity.encryption.parts[0].paths, vec!["/"]);
    assert_eq!(
        manifest.identity.encryption.parts[0].algorithm,
        "aes-256-gcm"
    );
    assert!(
        !manifest_text.contains("BEGIN PGP"),
        "the payload manifest carries no key material"
    );

    // The plaintext tree hash matches the source image's manifest.
    let src_manifest_text = {
        let mount = tfs::mount::build_from_file(&src.to_string_lossy(), "/mnt").unwrap();
        let st = mount.backend.stat("__tpkg__/manifest.yaml").unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = mount
            .backend
            .pread("__tpkg__/manifest.yaml", &mut buf, 0)
            .unwrap();
        String::from_utf8(buf[..n].to_vec()).unwrap()
    };
    let src_manifest = tpkg::PayloadManifest::from_yaml(&src_manifest_text).unwrap();
    assert_eq!(
        manifest.identity.digest.tree_hash, src_manifest.identity.digest.tree_hash,
        "encryption is a per-audience transform: the plaintext identity is preserved"
    );
}

// ---------------------------------------------------------------------
// Wrong key
// ---------------------------------------------------------------------

#[test]
fn mount_and_decrypt_with_the_wrong_key_is_the_named_ekey_error() {
    let w = TempDir::new("encwk");
    let src = mk_source_image(&w, "plain.tfs");
    let (pub_a, _sec_a) = mint_recipient(&w.0, "alice <a@example.com>");
    let (_pub_b, sec_b) = mint_recipient(&w.0, "bob <b@example.com>");
    let enc_img = w.0.join("enc.tfs");
    let (rc, _, err) = run(
        &[
            "encrypt",
            src.to_str().unwrap(),
            "-o",
            enc_img.to_str().unwrap(),
            "--recipient",
            pub_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""));

    let (rc, _, err) = run(
        &[
            "mount",
            enc_img.to_str().unwrap(),
            "--key",
            sec_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert!(
        err.contains("EKEY: no envelope recipient slot opens"),
        "{err}"
    );

    let (rc, _, err) = run(
        &[
            "decrypt",
            enc_img.to_str().unwrap(),
            "-o",
            w.0.join("nope.tar").to_str().unwrap(),
            "--key",
            sec_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert!(
        err.contains("EKEY: no envelope recipient slot opens"),
        "{err}"
    );
}

// ---------------------------------------------------------------------
// Selective disclosure
// ---------------------------------------------------------------------

#[test]
fn selective_disclosure_a_subtree_recipient_opens_only_their_slice() {
    let w = TempDir::new("encsd");
    let src = mk_source_image(&w, "plain.tfs");
    let (pub_a, sec_a) = mint_recipient(&w.0, "alice <a@example.com>");
    let (pub_b, sec_b) = mint_recipient(&w.0, "bob <b@example.com>");
    let enc_img = w.0.join("enc.tfs");

    let (rc, _, err) = run(
        &[
            "encrypt",
            src.to_str().unwrap(),
            "-o",
            enc_img.to_str().unwrap(),
            "--recipient",
            pub_a.to_str().unwrap(),
            "--subtree",
            &format!("/a/secret={}", pub_b.display()),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "encrypt must succeed: {err}");

    // Bob's mount opens exactly the /a/secret grant; the root grant is
    // reported as sealed to him.
    let (rc, out, err) = run(
        &[
            "mount",
            enc_img.to_str().unwrap(),
            "--key",
            sec_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "bob must mount: {err}");
    assert!(out.contains("grant: /a/secret → /a/secret"), "{out}");
    assert!(out.contains("other grants (sealed): / → /"), "{out}");

    // Bob CANNOT decrypt the whole image (reads outside /a/secret are
    // EKEY); Alice can, and her plaintext round-trips.
    let (rc, _, err) = run(
        &[
            "decrypt",
            enc_img.to_str().unwrap(),
            "-o",
            w.0.join("bob.tar").to_str().unwrap(),
            "--key",
            sec_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1, "bob's decrypt must fail outside his subtree");
    assert!(err.contains("errno 126") || err.contains("EKEY"), "{err}");

    let alice_tar = w.0.join("alice.tar");
    let (rc, _, err) = run(
        &[
            "decrypt",
            enc_img.to_str().unwrap(),
            "-o",
            alice_tar.to_str().unwrap(),
            "--key",
            sec_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "alice must decrypt: {err}");
    let contents = tar_contents(&alice_tar);
    let contract = contents
        .iter()
        .find(|(p, _)| p == "a/secret/contract.txt")
        .map(|(_, c)| c.clone())
        .unwrap();
    assert_eq!(
        contract,
        b"PLAINTEXT-CANARY-contract: for legal eyes only\n"
    );

    // In-process proof of the one-way property: Bob's opened backend
    // reads his subtree and nothing else.
    let mount = tfs::mount::build_from_file(&enc_img.to_string_lossy(), "/mnt").unwrap();
    let bob = tfs::EncBackend::new(
        mount.backend,
        tfs::KeySource::Recipient {
            secret_key: std::fs::read(&sec_b).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(bob.grant_path(), "/a/secret");
    let st = bob.stat("a/secret/contract.txt").unwrap();
    let mut buf = vec![0u8; st.size as usize];
    let n = bob.pread("a/secret/contract.txt", &mut buf, 0).unwrap();
    assert_eq!(
        &buf[..n],
        b"PLAINTEXT-CANARY-contract: for legal eyes only\n"
    );
    assert_eq!(
        bob.pread("a/other/public.txt", &mut [0u8; 4], 0)
            .unwrap_err(),
        tfs::ENOKEY
    );
    assert_eq!(
        bob.pread("docs/readme.txt", &mut [0u8; 4], 0).unwrap_err(),
        tfs::ENOKEY
    );
}

// ---------------------------------------------------------------------
// Rewrap rotation
// ---------------------------------------------------------------------

#[test]
fn rewrap_rotates_recipients_without_touching_the_bulk() {
    let w = TempDir::new("encrw");
    let src = mk_source_image(&w, "plain.tfs");
    let (pub_a, sec_a) = mint_recipient(&w.0, "alice <a@example.com>");
    let (pub_b, sec_b) = mint_recipient(&w.0, "bob <b@example.com>");
    let enc_a = w.0.join("enc-a.tfs");
    let enc_b = w.0.join("enc-b.tfs");

    let (rc, _, err) = run(
        &[
            "encrypt",
            src.to_str().unwrap(),
            "-o",
            enc_a.to_str().unwrap(),
            "--recipient",
            pub_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""));

    // Rotate: Alice's key unwraps; the new envelope is Bob's alone.
    let (rc, _, err) = run(
        &[
            "encrypt",
            enc_a.to_str().unwrap(),
            "-o",
            enc_b.to_str().unwrap(),
            "--rewrap",
            "--key",
            sec_a.to_str().unwrap(),
            "--recipient",
            pub_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "rewrap must succeed: {err}");

    // The bulk ciphertext is BYTE-IDENTICAL (never re-encrypted)...
    assert_eq!(
        bulk_bytes(&enc_a),
        bulk_bytes(&enc_b),
        "rewrap must not touch the bulk ciphertext"
    );

    // ...Alice is out (prospective revocation), Bob is in, and his
    // plaintext is the original.
    let (rc, _, err) = run(
        &[
            "mount",
            enc_b.to_str().unwrap(),
            "--key",
            sec_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1, "alice must not open the rotated image");
    assert!(err.contains("EKEY"), "{err}");

    let bob_tar = w.0.join("bob.tar");
    let (rc, _, err) = run(
        &[
            "decrypt",
            enc_b.to_str().unwrap(),
            "-o",
            bob_tar.to_str().unwrap(),
            "--key",
            sec_b.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!((rc, err.as_str()), (0, ""), "bob must decrypt: {err}");
    let contents = tar_contents(&bob_tar);
    let readme = contents
        .iter()
        .find(|(p, _)| p == "docs/readme.txt")
        .map(|(_, c)| c.clone())
        .unwrap();
    assert_eq!(readme, b"the quick brown fox\n");

    // Rewrap with a key that opens nothing is the named EKEY error.
    let (_pub_c, sec_c) = mint_recipient(&w.0, "carol <c@example.com>");
    let (rc, _, err) = run(
        &[
            "encrypt",
            enc_b.to_str().unwrap(),
            "-o",
            w.0.join("enc-c.tfs").to_str().unwrap(),
            "--rewrap",
            "--key",
            sec_c.to_str().unwrap(),
            "--recipient",
            pub_a.to_str().unwrap(),
        ],
        &w.0,
    );
    assert_eq!(rc, 1);
    assert!(err.contains("EKEY"), "{err}");
}

// ---------------------------------------------------------------------
// The plaintext-never-touches-disk scan (staging area)
// ---------------------------------------------------------------------

#[test]
fn staging_holds_no_plaintext() {
    let w = TempDir::new("encscan");
    // A source tree straight on the host (the transform's write side
    // takes any Backend — HostDir needs no image).
    let src = w.0.join("src");
    std::fs::create_dir_all(src.join("__tpkg__")).unwrap();
    std::fs::write(src.join("__tpkg__/manifest.yaml"), TEST_MANIFEST).unwrap();
    let canaries: Vec<(&str, Vec<u8>)> = payload_files();
    for (rel, content) in &canaries {
        let dest = src.join(rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(dest, content).unwrap();
    }

    let backend = tfs::backends_hostdir::HostDirBackend::new(&src).unwrap();
    let dek = tfs::backends_enc::generate_dek().unwrap();
    let staging = w.0.join("staging");
    tfs_cli::enc::stage_encrypted(
        &backend,
        &staging,
        &dek,
        TEST_MANIFEST,
        "schema_version: 1\nsuite: SUITE-1\ngrants: []\n",
    )
    .unwrap();

    // Scan every staged byte: no source file's plaintext appears
    // anywhere (the metadata manifest/envelopes are the only plaintext
    // files, and they carry no content).
    let canary = b"PLAINTEXT-CANARY";
    fn scan(dir: &Path, canary: &[u8], canaries: &[(&str, Vec<u8>)]) -> Vec<String> {
        let mut hits = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                hits.extend(scan(&path, canary, canaries));
            } else {
                let bytes = std::fs::read(&path).unwrap();
                let rel = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if bytes.windows(canary.len()).any(|w| w == canary) {
                    hits.push(format!("canary found in {rel}"));
                }
                if !rel.starts_with("__tpkg__") {
                    for (_, content) in canaries {
                        if content.len() >= 64 && bytes == *content {
                            hits.push(format!("whole plaintext file found in {rel}"));
                        }
                    }
                }
            }
        }
        hits
    }
    let hits = scan(&staging, canary, &canaries);
    assert!(
        hits.is_empty(),
        "plaintext reached the staging area: {hits:?}"
    );

    // And the staged tree decrypts back to the source (the scan is not
    // vacuous — the transform is real).
    let enc_backend = tfs::backends_hostdir::HostDirBackend::new(&staging).unwrap();
    let enc = tfs::EncBackend::new(
        Box::new(enc_backend),
        tfs::KeySource::SubtreeKey {
            path: "/".to_string(),
            key: dek,
        },
    )
    .unwrap();
    for (rel, content) in &canaries {
        let st = enc.stat(rel).unwrap();
        assert_eq!(st.size as usize, content.len(), "{rel}");
        let mut buf = vec![0u8; st.size as usize];
        let n = enc.pread(rel, &mut buf, 0).unwrap();
        assert_eq!(&buf[..n], content.as_slice(), "{rel}");
    }
}
