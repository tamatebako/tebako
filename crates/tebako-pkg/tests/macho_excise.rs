//! Mach-O signature excision at the press surface (spec 31 §1.2): a
//! signed bootstrap goes in, a cleanly-unsigned Mach-O + slots + trailer
//! comes out — signable downstream, runnable on arm64.

use std::path::{Path, PathBuf};

use tebako_contract_tests::TempDir;
use tebako_pkg::{bundle, macho, parse_image_spec, PackageOptions};

/// A 64-bit little-endian Mach-O carrying LC_CODE_SIGNATURE: header +
/// LC_SEGMENT_64("__LINKEDIT", fileoff=0x100, covering to the superblob
/// end) + the LC, filler to `dataoff`, then the superblob.
fn signed_macho(dataoff: u32, datasize: u32) -> Vec<u8> {
    let le32 = |v: u32| v.to_le_bytes();
    let le64 = |v: u64| v.to_le_bytes();
    let filesize = dataoff as u64 + datasize as u64 - 0x100;
    let mut v = Vec::new();
    v.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]); // MH_MAGIC_64
    v.extend_from_slice(&le32(0x0100_000c));
    v.extend_from_slice(&le32(0));
    v.extend_from_slice(&le32(2)); // MH_EXECUTE
    v.extend_from_slice(&le32(2)); // ncmds
    v.extend_from_slice(&le32(72 + 16)); // sizeofcmds
    v.extend_from_slice(&le32(0));
    v.extend_from_slice(&le32(0));
    v.extend_from_slice(&le32(0x19)); // LC_SEGMENT_64
    v.extend_from_slice(&le32(72));
    v.extend_from_slice(b"__LINKEDIT\0\0\0\0\0\0");
    v.extend_from_slice(&le64(0x1_0000_0000)); // vmaddr
    v.extend_from_slice(&le64(filesize.div_ceil(0x4000) * 0x4000)); // vmsize
    v.extend_from_slice(&le64(0x100)); // fileoff
    v.extend_from_slice(&le64(filesize));
    v.extend_from_slice(&[0u8; 16]); // maxprot/initprot/nsects/flags
    v.extend_from_slice(&le32(0x1d)); // LC_CODE_SIGNATURE
    v.extend_from_slice(&le32(16));
    v.extend_from_slice(&le32(dataoff));
    v.extend_from_slice(&le32(datasize));
    v.resize(dataoff as usize, 0xaa);
    v.extend_from_slice(&vec![0xbb; datasize as usize]);
    v
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn le_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

#[test]
fn press_excises_signed_macho_bootstrap() {
    let w = TempDir::new("macho-press");
    let boot = w.0.join("boot.bin");
    let input = signed_macho(0x400, 0x80);
    std::fs::write(&boot, &input).unwrap();
    let img = w.0.join("a.dwarfs");
    std::fs::copy(fixture("simple.dwarfs"), &img).unwrap();
    let pkg = w.0.join("pkg");

    let images = vec![parse_image_spec(img.to_str().unwrap())];
    bundle(
        &boot,
        &images,
        &pkg,
        &PackageOptions {
            runtime_ref: "rt-1.0".into(),
            ..Default::default()
        },
    )
    .expect("bundle");

    let out = std::fs::read(&pkg).unwrap();
    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).expect("trailer parses");
    // The bootstrap region was truncated at the superblob's dataoff.
    assert_eq!(m.slots.len(), 1);
    assert_eq!(m.slots[0].offset, 0x400);
    // The LC_CODE_SIGNATURE command was REMOVED from the table (the
    // codesign --remove-signature shape): ncmds 2→1, sizeofcmds 88→72,
    // the vacated bytes zeroed.
    assert_eq!(&out[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
    assert_eq!(&out[16..20], &1u32.to_le_bytes());
    assert_eq!(&out[20..24], &72u32.to_le_bytes());
    let vacated = 32 + 72;
    assert_eq!(&out[vacated..vacated + 16], &[0; 16]);
    // __LINKEDIT was extended over the whole package (bootstrap + slot +
    // trailer) so the result stays codesign-able.
    let filesize = le_u64(&out, 32 + 48);
    assert_eq!(filesize, out.len() as u64 - 0x100);
    let vmsize = le_u64(&out, 32 + 32);
    assert_eq!(vmsize, filesize.div_ceil(0x4000) * 0x4000);
    // The superblob is gone; the slot bytes follow the excised prefix.
    let slot_len = m.slots[0].size as usize;
    assert_eq!(
        &out[0x400..0x400 + slot_len],
        &std::fs::read(&img).unwrap()[..]
    );
}

#[test]
fn press_of_unsigned_macho_is_byte_identical() {
    let w = TempDir::new("macho-golden");
    let boot = w.0.join("boot.bin");
    // A Mach-O without LC_CODE_SIGNATURE: the golden path must be a no-op.
    let mut input = signed_macho(0x400, 0x80);
    input[16..20].copy_from_slice(&1u32.to_le_bytes()); // ncmds=1
    input[20..24].copy_from_slice(&72u32.to_le_bytes()); // sizeofcmds
    input.truncate(32 + 72); // drop the LC; keep header + segment only
    input.resize(0x400, 0xaa);
    std::fs::write(&boot, &input).unwrap();
    let img = w.0.join("a.dwarfs");
    std::fs::copy(fixture("simple.dwarfs"), &img).unwrap();
    let pkg = w.0.join("pkg");

    let images = vec![parse_image_spec(img.to_str().unwrap())];
    bundle(&boot, &images, &pkg, &PackageOptions::default()).expect("bundle");

    let out = std::fs::read(&pkg).unwrap();
    assert_eq!(&out[..input.len()], &input[..]);
}

/// The acceptance round-trip: excise a real linker-signed Mach-O (the
/// test binary itself), then — in the press's ordering — append the
/// trailer-stand-in junk and apply the __LINKEDIT fixup, and prove
/// codesign accepts the result. The pre-fix failure was
/// "main executable failed strict validation".
#[cfg(target_os = "macos")]
#[test]
fn excised_macho_is_signable_and_verifiable() {
    use std::process::Command;

    let exe = std::env::current_exe().expect("current exe");
    let input = std::fs::read(&exe).expect("read current exe");
    assert!(macho::is_mach_o(&input[..4]), "test binary is a Mach-O");

    let e = match macho::excise_code_signature(&input).expect("excise") {
        macho::Excision::Excised(e) => e,
        macho::Excision::Unchanged => panic!("test binary carries no code signature"),
    };
    // Linker output carries the superblob at the file tail: the excise
    // truncates it away.
    assert!(
        e.bytes.len() < input.len(),
        "superblob at the tail must be truncated ({} !< {})",
        e.bytes.len(),
        input.len()
    );
    assert_eq!(e.fixups.len(), 1);

    // The press's ordering: excised bootstrap bytes, then the appended
    // regions, then the fixup over the final length.
    let mut staged = e.bytes;
    staged.extend_from_slice(b"tpkg-trailer-stand-in-bytes");
    let total = staged.len() as u64;
    let mut cursor = std::io::Cursor::new(&mut staged);
    e.fixups[0].apply(&mut cursor, total).expect("fixup");

    let w = TempDir::new("macho-codesign");
    let pressed = w.0.join("pressed");
    std::fs::write(&pressed, cursor.into_inner()).unwrap();

    let run = |args: &[&str]| {
        let out = Command::new("codesign")
            .args(args)
            .arg(&pressed)
            .output()
            .expect("spawn codesign");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };

    let (code, _, stderr) = run(&["--force", "--sign", "-"]);
    assert_eq!(code, 0, "codesign sign failed: {stderr}");
    let (code, _, stderr) = run(&["--verify", "--strict"]);
    assert_eq!(code, 0, "codesign verify failed: {stderr}");
}

/// The full installer flow: press a real package from a signed Mach-O
/// bootstrap, codesign it post-press, and prove the trailer still reads
/// (codesign appends the superblob AFTER the trailer; the reader locates
/// the trailer before it) — the two ends of spec 31 §1.2's pipeline.
#[cfg(target_os = "macos")]
#[test]
fn pressed_signed_package_keeps_a_readable_trailer() {
    use std::process::Command;

    let w = TempDir::new("macho-pkg-sign");
    let exe = std::env::current_exe().expect("current exe");
    let boot = w.0.join("boot.bin");
    std::fs::copy(&exe, &boot).unwrap();
    let img = w.0.join("a.dwarfs");
    std::fs::copy(fixture("simple.dwarfs"), &img).unwrap();
    let pkg = w.0.join("pkg");

    let images = vec![parse_image_spec(img.to_str().unwrap())];
    bundle(&boot, &images, &pkg, &PackageOptions::default()).expect("bundle");

    let out = Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(&pkg)
        .output()
        .expect("spawn codesign");
    assert!(
        out.status.success(),
        "codesign sign failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new("codesign")
        .args(["--verify", "--strict"])
        .arg(&pkg)
        .output()
        .expect("spawn codesign");
    assert!(
        out.status.success(),
        "codesign verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The superblob now trails the trailer; the trailer still parses.
    let mut f = std::fs::File::open(&pkg).unwrap();
    let m = tpkg::read_from(&mut f).expect("trailer parses past the superblob");
    assert_eq!(m.slots.len(), 1);
    let slot_len = m.slots[0].size as usize;
    let off = m.slots[0].offset as usize;
    let bytes = std::fs::read(&pkg).unwrap();
    assert_eq!(
        &bytes[off..off + slot_len],
        &std::fs::read(&img).unwrap()[..]
    );
}
