//! Opt-in signing tests (owner directive: "signing/encryption is optional").
//!
//! - unsigned bundle: byte-identical across runs, no key material created,
//!   v1 trailer, accepted by the bootstrap legacy path
//! - --sign (press-local): signed v2, verifies strictly, tamper -> named exit
//! - --sign=<keyid>: selects a secret key from $TEBAKO_HOME/keys
//! - rewrite operations preserve the input's signing state

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use tebako_bootstrap::{verify_chain_with_home, EX_TEBAKO_SHA};
use tebako_pkg::{bundle, insert_image, PackageImage, PackageOptions, SignRequest};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-pkg-optin-{}-{name}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_temp_home<R>(f: impl FnOnce(&Path) -> R) -> R {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let home = std::env::temp_dir().join(format!(
        "tebako-pkg-optin-home-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&home).unwrap();
    let r = f(&home);
    std::env::remove_var("TEBAKO_HOME");
    std::fs::remove_dir_all(&home).ok();
    r
}

fn parts(dir: &Path) -> (PathBuf, PathBuf, Vec<PackageImage>) {
    std::fs::create_dir_all(dir).unwrap();
    let bootstrap = dir.join("bootstrap.bin");
    let image = dir.join("app.img");
    std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
    std::fs::write(&image, b"the app image payload").unwrap();
    let images = vec![PackageImage {
        path: image.clone(),
        mount_point: "/app".to_string(),
        format_id: tpkg::TPKG_FORMAT_ZIP,
    }];
    (bootstrap, image, images)
}

fn opts(sign: SignRequest) -> PackageOptions {
    PackageOptions {
        runtime_ref: String::new(),
        package_flags: 0,
        launcher_abi: 1,
        sign,
    }
}

#[test]
fn unsigned_bundle_is_deterministic_and_creates_no_keys() {
    let dir = scratch("unsigned");
    let (bootstrap, _, images) = parts(&dir);
    let out_a = dir.join("a.pkg");
    let out_b = dir.join("b.pkg");

    with_temp_home(|home| {
        std::env::set_var("TEBAKO_HOME", home);
        bundle(&bootstrap, &images, &out_a, &opts(SignRequest::None)).expect("bundle a");
        bundle(&bootstrap, &images, &out_b, &opts(SignRequest::None)).expect("bundle b");

        // byte-identical to pre-signing behavior (and run to run)
        assert_eq!(
            std::fs::read(&out_a).unwrap(),
            std::fs::read(&out_b).unwrap(),
            "unsigned bundles must be byte-identical"
        );
        // no key material, no keyring, no ceremony
        assert!(!home.join("keys").exists(), "no keys dir may be created");
        assert!(
            !home.join("keyring/trusted.pgp").exists(),
            "no keyring may be created"
        );

        // plain v1 trailer: no flag, no extension
        let m = tpkg::read_from(&mut std::fs::File::open(&out_a).unwrap()).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_SIGNED_V2, 0);
        assert!(m.v2.is_none());

        // the bootstrap accepts it via the v1-legacy path
        verify_chain_with_home(&out_a, &m, home).expect("legacy acceptance");
        assert!(
            home.join("journal.log").exists(),
            "legacy acceptance must be journaled"
        );
        let j = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(j.contains("legacy-v1-accepted"), "{j}");
    });
}

#[test]
fn sign_with_press_key_verifies_and_tamper_fails_closed() {
    let dir = scratch("press");
    let (bootstrap, _, images) = parts(&dir);
    let out = dir.join("signed.pkg");

    with_temp_home(|home| {
        std::env::set_var("TEBAKO_HOME", home);
        bundle(&bootstrap, &images, &out, &opts(SignRequest::PressLocal)).expect("signed bundle");
        assert!(home.join("keys/press-local.key").exists());

        let m = tpkg::read_from(&mut std::fs::File::open(&out).unwrap()).unwrap();
        assert!(m.v2.is_some());
        verify_chain_with_home(&out, &m, home).expect("chain verifies");

        // tamper the slot -> named EX_TEBAKO_SHA. The trusted-cache
        // marker is keyed on (size, mtime seconds): force the mtime to
        // advance past the marker's second so the tamper cannot be
        // mistaken for the verified file (an attacker-forgeable mtime is
        // a documented phase-1 caveat of the marker).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let mut bytes = std::fs::read(&out).unwrap();
        bytes[16] ^= 0xFF;
        std::fs::write(&out, &bytes).unwrap();
        let err = verify_chain_with_home(&out, &m, home).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_SHA);
    });
}

#[test]
fn sign_with_keyid_selects_the_named_key() {
    let dir = scratch("keyid");
    let (bootstrap, _, images) = parts(&dir);

    with_temp_home(|home| {
        std::env::set_var("TEBAKO_HOME", home);
        // two keys: the press-local default and a second named key
        let press = tebako_signer::press_local_key(home).expect("press key");
        let other_ctx = rnp::Context::new().unwrap();
        let other_key = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid("other-signing-key")
            .add_usage(rnp::KeyUsage::Sign)
            .build(&other_ctx)
            .unwrap();
        let other_fp = other_key.fingerprint().unwrap();
        let other_keyid = tebako_signer::keyid_bytes_from_fingerprint(&other_fp).unwrap();
        let other_secret = other_key
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        std::fs::write(home.join("keys/other.key"), &other_secret).unwrap();

        let out = dir.join("other-signed.pkg");
        let keyid_hex = tebako_signer::hex_lower(&other_keyid);
        bundle(
            &bootstrap,
            &images,
            &out,
            &opts(SignRequest::Keyid(keyid_hex.clone())),
        )
        .expect("bundle with named key");

        let m = tpkg::read_from(&mut std::fs::File::open(&out).unwrap()).unwrap();
        let v2 = m.v2.as_ref().unwrap();
        assert_eq!(v2.signer_keyid_hex(), keyid_hex);
        assert_ne!(v2.signer_keyid_hex(), press.keyid_hex());

        // unknown keyid -> named error, no package written
        let out2 = dir.join("nope.pkg");
        let e = bundle(
            &bootstrap,
            &images,
            &out2,
            &opts(SignRequest::Keyid("0000000000000000".into())),
        )
        .unwrap_err();
        assert!(e.contains("no secret key with keyid"), "{e}");
        assert!(!out2.exists());
    });
}

#[test]
fn rewrite_preserves_signing_state() {
    let dir = scratch("rewrite");
    let (bootstrap, image, images) = parts(&dir);

    with_temp_home(|home| {
        std::env::set_var("TEBAKO_HOME", home);

        // signed input -> insert-image -> still signed and verifying
        let signed_pkg = dir.join("signed.pkg");
        bundle(
            &bootstrap,
            &images,
            &signed_pkg,
            &opts(SignRequest::PressLocal),
        )
        .unwrap();
        insert_image(&signed_pkg, &image, "/extra").expect("insert into signed");
        let m = tpkg::read_from(&mut std::fs::File::open(&signed_pkg).unwrap()).unwrap();
        assert!(m.v2.is_some(), "signed input must stay signed");
        verify_chain_with_home(&signed_pkg, &m, home).expect("still verifies");

        // unsigned input -> insert-image -> stays unsigned
        let unsigned_pkg = dir.join("unsigned.pkg");
        bundle(&bootstrap, &images, &unsigned_pkg, &opts(SignRequest::None)).unwrap();
        insert_image(&unsigned_pkg, &image, "/extra").expect("insert into unsigned");
        let m = tpkg::read_from(&mut std::fs::File::open(&unsigned_pkg).unwrap()).unwrap();
        assert!(m.v2.is_none(), "unsigned input must stay unsigned");
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_SIGNED_V2, 0);
    });
}
