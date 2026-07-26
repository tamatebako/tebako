//! tebako-pkg bundle signing tests: a bundled package carries a signed v2
//! trailer with correct per-slot digests, and `info` reports the signature
//! status. Uses a temp TEBAECO_HOME so the real user keyring is never
//! touched (the env is process-global, so these tests serialize on a lock).

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use tebako_pkg::{bundle, info, PackageImage, PackageOptions};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn with_temp_home<R>(f: impl FnOnce(&Path) -> R) -> R {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let home = std::env::temp_dir().join(format!("tebako-pkg-test-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("TEBAKO_HOME", &home);
    let r = f(&home);
    std::env::remove_var("TEBAKO_HOME");
    std::fs::remove_dir_all(&home).ok();
    r
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    h.finalize().into()
}

#[test]
fn bundle_produces_signed_v2_package() {
    with_temp_home(|home| {
        let dir = home.join("work");
        std::fs::create_dir_all(&dir).unwrap();
        let bootstrap = dir.join("bootstrap.bin");
        let image = dir.join("app.img");
        let out = dir.join("out.pkg");
        std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
        std::fs::write(&image, b"the app image payload").unwrap();

        let images = [PackageImage {
            path: image.clone(),
            mount_point: "/app".to_string(),
            format_id: tpkg::TPKG_FORMAT_ZIP,
        }];
        let options = PackageOptions {
            runtime_ref: String::new(),
            package_flags: 0,
            launcher_abi: 1,
        };
        bundle(&bootstrap, &images, &out, &options).expect("bundle");

        let mut f = std::fs::File::open(&out).unwrap();
        let m = tpkg::read_from(&mut f).expect("parse trailer");
        assert_ne!(m.package_flags & tpkg::TPKG_FLAG_SIGNED_V2, 0);
        assert_eq!(m.slots.len(), 1);

        let v2 = m.v2.as_ref().expect("v2 extension");
        assert_ne!(v2.signer_keyid, [0; 8]);
        assert!(!v2.signature.is_empty());
        // slot digest covers the image bytes streamed into the package
        assert_eq!(v2.slot_digests[0], sha256(b"the app image payload"));
        // unused digest entries stay zeroed
        assert!(v2.slot_digests[1..].iter().all(|d| *d == [0; 32]));

        // the press key was auto-registered in the temp home's keyring
        assert!(home.join("keyring/trusted.pgp").exists());

        let text = info(&out).expect("info");
        assert!(text.contains("Signature: OpenPGP v2"), "info: {text}");
        assert!(text.contains("trusted"), "info: {text}");
    });
}

#[test]
fn tampered_slot_is_named_by_the_digest() {
    with_temp_home(|home| {
        let dir = home.join("work2");
        std::fs::create_dir_all(&dir).unwrap();
        let bootstrap = dir.join("bootstrap.bin");
        let image = dir.join("app.img");
        let out = dir.join("out.pkg");
        std::fs::write(&bootstrap, b"BOOTSTRAP-BYTES").unwrap();
        std::fs::write(&image, b"the app image payload").unwrap();

        let images = [PackageImage {
            path: image,
            mount_point: "/app".to_string(),
            format_id: tpkg::TPKG_FORMAT_ZIP,
        }];
        bundle(
            &bootstrap,
            &images,
            &out,
            &PackageOptions {
                runtime_ref: String::new(),
                package_flags: 0,
                launcher_abi: 1,
            },
        )
        .unwrap();

        // tamper one byte inside the slot region (slot 0 starts right
        // after the 15 bootstrap bytes)
        let mut bytes = std::fs::read(&out).unwrap();
        bytes[16] ^= 0xFF;
        std::fs::write(&out, &bytes).unwrap();

        // the trailer's digest no longer matches the slot content
        let mut f = std::fs::File::open(&out).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        let v2 = m.v2.as_ref().unwrap();
        let slot = &m.slots[0];
        let region = &bytes[slot.offset as usize..(slot.offset + slot.size) as usize];
        assert_ne!(sha256(region), v2.slot_digests[0]);
    });
}
