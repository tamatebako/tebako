//! Would dyld honor `DYLD_INSERT_LIBRARIES` for a spawn of this Mach-O
//! image on this host? (packed-metanorma#305; spec 22 §3.1's macOS row,
//! refined.)
//!
//! The preload tier's injection rides `DYLD_INSERT_LIBRARIES` into every
//! spawned child — but on darwin 23 dyld SILENTLY DROPS the variable when
//! the target image carries the hardened runtime
//! (`CS_RUNTIME`) without the
//! `com.apple.security.cs.allow-dyld-environment-variables` entitlement
//! (spec 22 §2's "macOS 14 insertion drop", verified there for the
//! interpreter's own boot; the same plane governs children). On darwin
//! 24+ the variable is honored — fatally, for Apple platform binaries,
//! which is the spawn hook's restricted scrub, a different boundary.
//!
//! A child whose image drops the insertion runs WITHOUT the preload
//! shim: every in-image (VFS) path handed to it — argv operands like
//! sass-embedded's snapshot — is dead, and the child answers with its
//! own honest-but-misleading failure (dart's "not an AOT snapshot"
//! after the VFS `File::Open` comes back ENOENT). The interpreter's
//! spawn hook asks this module per spawn: `false` means bridge the
//! embedded operand tokens to their materialized host twins instead of
//! relying on the injection.
//!
//! The probe is a pure byte parse of the image's code signature —
//! `unsafe`-free, no loader involvement, never fails closed on an
//! unreadable or unparseable file: any doubt answers `true` (the
//! pre-probe behavior — attempt the injection; the child's own error
//! stands if the file never execs at all).
//!
//! The darwin-23/24 law itself is owned by spec 22 §2/§3.1 (and mirrored
//! by the driver's `ffi/rebind.rs` header); this module owns only the
//! per-image PROBE. The Mach-O container grammar constants are
//! `exec_closure`'s — one spelling of `<mach-o/loader.h>` in the crate.

use std::path::Path;

use crate::exec_closure::{
    CPU_TYPE_ARM64, CPU_TYPE_X86_64, FAT_MAGIC, FAT_MAGIC_64, MH_MAGIC, MH_MAGIC_64,
};

/// `LC_CODE_SIGNATURE` — the load command naming the signature region.
const LC_CODE_SIGNATURE: u32 = 0x1d;
/// `CSMAGIC_EMBEDDED_SIGNATURE` — the superblob wrapping the slots.
const CSMAGIC_SUPERBLOB: u32 = 0xFADE_0CC0;
/// `CSMAGIC_CODEDIRECTORY` — the slot-0 code directory.
const CSMAGIC_CODEDIRECTORY: u32 = 0xFADE_0C02;
/// `CSMAGIC_ENTITLEMENTS` — the slot-5 XML entitlement blob.
const CSMAGIC_ENTITLEMENTS: u32 = 0xFADE_7171;
/// `CSMAGIC_DER_ENTITLEMENTS` — the slot-7 DER entitlement blob.
const CSMAGIC_DER_ENTITLEMENTS: u32 = 0xFADE_7172;
/// The code-directory flag for the hardened runtime.
const CS_RUNTIME: u32 = 0x0001_0000;
/// The superblob index types the probe reads.
const CSSLOT_CODE_DIRECTORY: u32 = 0;
const CSSLOT_ENTITLEMENTS: u32 = 5;
const CSSLOT_DER_ENTITLEMENTS: u32 = 7;

/// The entitlement whose presence unblocks the environment plane.
const DYLD_ENTITLEMENT_KEY: &[u8] = b"com.apple.security.cs.allow-dyld-environment-variables";

/// What the image's signature says about the hardened runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hardening {
    /// No `LC_CODE_SIGNATURE`, no superblob, or no code directory — an
    /// unsigned image; the insertion is honored.
    Unsigned,
    /// A code directory WITHOUT `CS_RUNTIME` (the adhoc shape, entitled
    /// or not): the insertion is honored.
    NotHardened,
    /// `CS_RUNTIME` set: honored on darwin 24+, honored everywhere when
    /// the dyld entitlement reads true, dropped on darwin 23 otherwise.
    Hardened { entitled: bool },
}

fn u32be(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}

fn u32le(b: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

/// The slice dyld would load: the host CPU's fat slice, or the whole
/// thin image. `None` for a non-Mach-O buffer or a fat image with no
/// host slice (execve fails honestly on it — the caller's probe answers
/// "honored" and never reaches a verdict about the child).
fn host_slice(bytes: &[u8]) -> Option<&[u8]> {
    match u32be(bytes.get(..4)?)? {
        FAT_MAGIC | FAT_MAGIC_64 => {
            let fat64 = u32be(bytes.get(..4)?)? == FAT_MAGIC_64;
            let nfat = u32be(bytes.get(4..8)?)? as usize;
            let want = if cfg!(target_arch = "aarch64") {
                CPU_TYPE_ARM64
            } else {
                CPU_TYPE_X86_64
            };
            for i in 0..nfat {
                // fat_arch (20 bytes) / fat_arch_64 (32 bytes):
                // cputype, cpusubtype, offset, size, align (, reserved).
                let base = 8 + i * if fat64 { 32 } else { 20 };
                let cputype = u32be(bytes.get(base..base + 4)?)?;
                let (offset, size) = if fat64 {
                    (
                        u64::from(u32be(bytes.get(base + 8..base + 12)?)?) << 32
                            | u64::from(u32be(bytes.get(base + 12..base + 16)?)?),
                        u64::from(u32be(bytes.get(base + 16..base + 20)?)?) << 32
                            | u64::from(u32be(bytes.get(base + 20..base + 24)?)?),
                    )
                } else {
                    (
                        u64::from(u32be(bytes.get(base + 8..base + 12)?)?),
                        u64::from(u32be(bytes.get(base + 12..base + 16)?)?),
                    )
                };
                if cputype == want {
                    return bytes
                        .get(offset as usize..(offset as usize).checked_add(size as usize)?);
                }
            }
            None
        }
        _ => {
            let magic = u32le(bytes.get(..4)?)?;
            matches!(magic, MH_MAGIC_64 | MH_MAGIC).then_some(bytes)
        }
    }
}

/// The image's code-signature region: `(superblob offset)` found by
/// walking the load commands of a thin slice.
fn code_signature_offset(slice: &[u8]) -> Option<usize> {
    let is64 = match u32le(slice.get(..4)?)? {
        MH_MAGIC_64 => true,
        MH_MAGIC => false,
        _ => return None,
    };
    let ncmds = u32le(slice.get(16..20)?)? as usize;
    let mut at: usize = if is64 { 32 } else { 28 };
    for _ in 0..ncmds {
        let cmd = u32le(slice.get(at..at + 4)?)?;
        let cmdsize = u32le(slice.get(at + 4..at + 8)?)? as usize;
        if cmdsize < 8 {
            return None;
        }
        if cmd == LC_CODE_SIGNATURE {
            let dataoff = u32le(slice.get(at + 8..at + 12)?)?;
            return Some(dataoff as usize);
        }
        at = at.checked_add(cmdsize)?;
    }
    None
}

/// One slot's blob bytes from the superblob.
fn slot_blob(superblob: &[u8], want_type: u32) -> Option<&[u8]> {
    if u32be(superblob.get(..4)?)? != CSMAGIC_SUPERBLOB {
        return None;
    }
    let count = u32be(superblob.get(8..12)?)? as usize;
    for i in 0..count {
        let entry = 12 + i * 8;
        let slot_type = u32be(superblob.get(entry..entry + 4)?)?;
        let offset = u32be(superblob.get(entry + 4..entry + 8)?)? as usize;
        if slot_type == want_type {
            return superblob.get(offset..);
        }
    }
    None
}

/// Does the XML entitlement blob assert the dyld key? The plist form is
/// `<key>KEY</key>` followed (whitespace aside) by `<true/>` or
/// `<false/>` — the first of the two tokens after the key decides.
fn xml_entitlement_true(blob: &[u8]) -> Option<bool> {
    let at = blob
        .windows(DYLD_ENTITLEMENT_KEY.len())
        .position(|w| w == DYLD_ENTITLEMENT_KEY)?;
    let rest = &blob[at + DYLD_ENTITLEMENT_KEY.len()..];
    let t = rest.windows(5).position(|w| w == b"<true");
    let f = rest.windows(6).position(|w| w == b"<false");
    match (t, f) {
        (Some(t), Some(f)) => Some(t < f),
        (Some(_), None) => Some(true),
        (None, Some(_)) => Some(false),
        (None, None) => None,
    }
}

/// Does the DER entitlement blob assert the dyld key? Apple's DER form
/// encodes the pair as UTF8String key (`0x0C len bytes`) immediately
/// followed by BOOLEAN (`0x01 0x01 0xFF` true / `0x01 0x01 0x00`
/// false) — the three bytes right after the key decide.
fn der_entitlement_true(blob: &[u8]) -> Option<bool> {
    let at = blob
        .windows(DYLD_ENTITLEMENT_KEY.len())
        .position(|w| w == DYLD_ENTITLEMENT_KEY)?;
    let rest = &blob[at + DYLD_ENTITLEMENT_KEY.len()..];
    match rest.get(..3)? {
        [0x01, 0x01, v] => Some(*v != 0),
        _ => None,
    }
}

/// The hardening verdict from the superblob: read the slot-0 code
/// directory's flags, then (hardened only) the entitlement slots — DER
/// first (authoritative when present: codesign emits both forms from
/// one source, and where they could diverge the DER reading is the
/// system's), XML as the legacy fallback (either may be the only one
/// present; an unreadable verdict is an UNENTITLED reading, matching
/// the adhoc-hardened shapes that carry neither).
fn hardening_of(super_blob: &[u8]) -> Hardening {
    let Some(cd) = slot_blob(super_blob, CSSLOT_CODE_DIRECTORY) else {
        return Hardening::Unsigned;
    };
    if cd.get(..4).and_then(u32be) != Some(CSMAGIC_CODEDIRECTORY) {
        return Hardening::Unsigned;
    }
    // code directory: magic, length, version, flags — flags at +12.
    let Some(flags) = cd.get(12..16).and_then(u32be) else {
        return Hardening::Unsigned;
    };
    if flags & CS_RUNTIME == 0 {
        return Hardening::NotHardened;
    }
    let entitled = slot_blob(super_blob, CSSLOT_DER_ENTITLEMENTS)
        .filter(|b| b.get(..4).and_then(u32be) == Some(CSMAGIC_DER_ENTITLEMENTS))
        .and_then(der_entitlement_true)
        .or_else(|| {
            slot_blob(super_blob, CSSLOT_ENTITLEMENTS)
                .filter(|b| b.get(..4).and_then(u32be) == Some(CSMAGIC_ENTITLEMENTS))
                .and_then(xml_entitlement_true)
        })
        .unwrap_or(false);
    Hardening::Hardened { entitled }
}

/// The running OS's major version (`kern.osproductversion` — 15 for the
/// insertion-honoring generation). Mirrors the driver's
/// `ffi/interpose.rs` helper (layering forbids the import — the
/// darwin-23/24 law it serves is spec 22 §2's, the single owner).
#[cfg(target_os = "macos")]
fn macos_major() -> Option<u64> {
    let mut buf = [0u8; 32];
    let mut size = buf.len();
    // SAFETY: the named sysctl is a string; buf is sized for it, size is
    // in/out, and no new value is written (the null oldp form).
    let rc = unsafe {
        libc::sysctlbyname(
            c"kern.osproductversion".as_ptr(),
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(size);
    let s = std::str::from_utf8(&buf[..end]).ok()?;
    s.split('.').next()?.parse().ok()
}

/// The pure verdict: given the image's hardening and the host's macOS
/// major, would dyld honor `DYLD_INSERT_LIBRARIES`? Darwin 23 drops it
/// for a hardened image without the entitlement; everything else —
/// unsigned, adhoc, entitled, any ELF target, and darwin 24+ — honors.
fn honored_verdict(hardening: Hardening, os_major: Option<u64>) -> bool {
    match hardening {
        Hardening::Hardened { entitled: false } => os_major.is_none_or(|v| v >= 15),
        _ => true,
    }
}

/// The probe the C export wraps (any read/parse doubt answers `true` —
/// attempt the injection; the spawn's own error surface stands).
pub(crate) fn dyld_insertion_honored(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return true;
    };
    let Some(slice) = host_slice(&bytes) else {
        return true;
    };
    let Some(sig_off) = code_signature_offset(slice) else {
        return true;
    };
    let Some(super_blob) = slice.get(sig_off..) else {
        return true;
    };
    #[cfg(target_os = "macos")]
    let os_major = macos_major();
    #[cfg(not(target_os = "macos"))]
    let os_major = None;
    honored_verdict(hardening_of(super_blob), os_major)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thin 64-bit LE Mach-O with one LC_CODE_SIGNATURE whose region
    /// carries a superblob indexing `(code directory with `cd_flags`,
    /// optional XML entitlement blob, optional DER entitlement blob)`.
    fn mach_o_with_signature(
        cd_flags: u32,
        xml_entitlements: Option<&[u8]>,
        der_entitlements: Option<&[u8]>,
    ) -> Vec<u8> {
        // The superblob index: slot 0 always; 5 / 7 per argument.
        let mut slots: Vec<(u32, Vec<u8>)> = Vec::new();
        // Code directory blob: magic + length + version + flags (+ pad).
        let mut cd = Vec::new();
        cd.extend_from_slice(&CSMAGIC_CODEDIRECTORY.to_be_bytes());
        cd.extend_from_slice(&20u32.to_be_bytes());
        cd.extend_from_slice(&0x20500u32.to_be_bytes()); // version
        cd.extend_from_slice(&cd_flags.to_be_bytes());
        cd.extend_from_slice(&[0u8; 4]);
        slots.push((CSSLOT_CODE_DIRECTORY, cd));
        if let Some(x) = xml_entitlements {
            let mut b = Vec::new();
            b.extend_from_slice(&CSMAGIC_ENTITLEMENTS.to_be_bytes());
            b.extend_from_slice(&(8 + x.len() as u32).to_be_bytes());
            b.extend_from_slice(x);
            slots.push((CSSLOT_ENTITLEMENTS, b));
        }
        if let Some(d) = der_entitlements {
            let mut b = Vec::new();
            b.extend_from_slice(&CSMAGIC_DER_ENTITLEMENTS.to_be_bytes());
            b.extend_from_slice(&(8 + d.len() as u32).to_be_bytes());
            b.extend_from_slice(d);
            slots.push((CSSLOT_DER_ENTITLEMENTS, b));
        }
        let superblob_off = 48usize;
        // Superblob index offsets are relative to the superblob start
        // (the probe reads them that way — Apple's grammar).
        let mut blob_off = 12 + slots.len() * 8;
        let mut b = Vec::new();
        b.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]); // MH_MAGIC_64 (stored LE)
        b.extend_from_slice(&[0u8; 12]); // cputype / cpusubtype / filetype
        b.extend_from_slice(&1u32.to_le_bytes()); // ncmds
        b.extend_from_slice(&16u32.to_le_bytes()); // sizeofcmds
        b.extend_from_slice(&[0u8; 8]); // flags + reserved
        b.extend_from_slice(&LC_CODE_SIGNATURE.to_le_bytes());
        b.extend_from_slice(&16u32.to_le_bytes()); // cmdsize
        b.extend_from_slice(&(superblob_off as u32).to_le_bytes()); // dataoff
        b.extend_from_slice(&[0u8; 4]); // datasize (never read)
        assert_eq!(b.len(), superblob_off);
        // Superblob header + index.
        let total_len = 12 + slots.len() * 8 + slots.iter().map(|(_, s)| s.len()).sum::<usize>();
        b.extend_from_slice(&CSMAGIC_SUPERBLOB.to_be_bytes());
        b.extend_from_slice(&(total_len as u32).to_be_bytes());
        b.extend_from_slice(&(slots.len() as u32).to_be_bytes());
        let mut bodies = Vec::new();
        for (slot_type, body) in &slots {
            b.extend_from_slice(&slot_type.to_be_bytes());
            b.extend_from_slice(&(blob_off as u32).to_be_bytes());
            bodies.extend_from_slice(body);
            blob_off += body.len();
        }
        b.extend_from_slice(&bodies);
        b
    }

    const XML_TRUE: &[u8] = b"<?xml version=\"1.0\"?><plist><dict><key>com.apple.security.cs.allow-dyld-environment-variables</key>\n\t<true/></dict></plist>";
    const XML_FALSE: &[u8] = b"<?xml version=\"1.0\"?><plist><dict><key>com.apple.security.cs.allow-dyld-environment-variables</key>\n\t<false/></dict></plist>";
    const XML_OTHER_ONLY: &[u8] = b"<?xml version=\"1.0\"?><plist><dict><key>com.apple.security.cs.allow-jit</key>\n\t<true/></dict></plist>";

    fn der_pair(key: &[u8], value: bool) -> Vec<u8> {
        let mut d = vec![0x30, 0x81, 0x00]; // SEQUENCE, length (unused by the probe)
        d.push(0x0c);
        d.push(key.len() as u8);
        d.extend_from_slice(key);
        d.extend_from_slice(&[0x01, 0x01, if value { 0xff } else { 0x00 }]);
        d
    }

    #[test]
    fn unsigned_and_adhoc_images_are_always_honored() {
        // No LC_CODE_SIGNATURE at all: a bare header.
        let plain = {
            let mut b = Vec::new();
            b.extend_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
            b.extend_from_slice(&[0u8; 12]);
            b.extend_from_slice(&0u32.to_le_bytes()); // ncmds = 0
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&[0u8; 8]);
            b
        };
        assert_eq!(hardening_of_on(&plain), None); // no signature → Unsigned
        assert!(honored_verdict(Hardening::Unsigned, Some(14)));
        assert!(honored_verdict(Hardening::Unsigned, Some(15)));
        // Adhoc: code directory present, CS_RUNTIME clear (flags 0x2).
        let adhoc = mach_o_with_signature(0x2, None, None);
        assert!(matches!(
            hardening_of(&adhoc[code_signature_offset(&adhoc).unwrap()..]),
            Hardening::NotHardened
        ));
        assert!(honored_verdict(Hardening::NotHardened, Some(14)));
        // Adhoc + hardened flag BOTH set (a `codesign -s - -o runtime`
        // shape) is HARDENED for dyld's purposes — flags carry 0x10000.
    }

    /// Parse a whole file buffer's signature when one exists (None = the
    /// probe's "no signature" early-out).
    fn hardening_of_on(bytes: &[u8]) -> Option<Hardening> {
        let slice = host_slice(bytes)?;
        let off = code_signature_offset(slice)?;
        Some(hardening_of(slice.get(off..)?))
    }

    #[test]
    fn hardened_without_the_entitlement_drops_only_on_darwin23() {
        let img = mach_o_with_signature(CS_RUNTIME, None, None);
        let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
        assert_eq!(h, Hardening::Hardened { entitled: false });
        assert!(!honored_verdict(h, Some(14)), "darwin 23: dropped");
        assert!(honored_verdict(h, Some(15)), "darwin 24: honored");
        assert!(honored_verdict(h, None), "an unknown OS never fails closed");
    }

    #[test]
    fn hardened_with_the_entitlement_is_honored_everywhere() {
        for (xml, der, note) in [
            (Some(XML_TRUE), None, "XML true"),
            (
                None,
                Some(&der_pair(DYLD_ENTITLEMENT_KEY, true)[..]),
                "DER true",
            ),
            (
                Some(XML_FALSE),
                Some(&der_pair(DYLD_ENTITLEMENT_KEY, true)[..]),
                "DER wins when XML says false and DER says true",
            ),
        ] {
            let img = mach_o_with_signature(CS_RUNTIME, xml, der);
            let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
            assert_eq!(h, Hardening::Hardened { entitled: true }, "{note}");
            assert!(honored_verdict(h, Some(14)), "{note}");
        }
    }

    #[test]
    fn entitlement_false_and_absent_read_unentitled() {
        let img = mach_o_with_signature(CS_RUNTIME, Some(XML_FALSE), None);
        let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
        assert_eq!(h, Hardening::Hardened { entitled: false });
        let img = mach_o_with_signature(CS_RUNTIME, Some(XML_OTHER_ONLY), None);
        let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
        assert_eq!(h, Hardening::Hardened { entitled: false });
        let img = mach_o_with_signature(
            CS_RUNTIME,
            None,
            Some(&der_pair(DYLD_ENTITLEMENT_KEY, false)[..]),
        );
        let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
        assert_eq!(h, Hardening::Hardened { entitled: false });
        // The reverse divergence pins the same precedence: DER false
        // overrides XML true.
        let img = mach_o_with_signature(
            CS_RUNTIME,
            Some(XML_TRUE),
            Some(&der_pair(DYLD_ENTITLEMENT_KEY, false)[..]),
        );
        let h = hardening_of(&img[code_signature_offset(&img).unwrap()..]);
        assert_eq!(h, Hardening::Hardened { entitled: false });
    }

    #[test]
    fn the_fat_wrapper_reads_the_host_slice() {
        let thin = mach_o_with_signature(CS_RUNTIME, Some(XML_TRUE), None);
        // A fat wrapper: one arm64 slice holding `thin`.
        let mut fat = Vec::new();
        fat.extend_from_slice(&FAT_MAGIC.to_be_bytes());
        fat.extend_from_slice(&1u32.to_be_bytes()); // nfat
        let want = if cfg!(target_arch = "aarch64") {
            CPU_TYPE_ARM64
        } else {
            CPU_TYPE_X86_64
        };
        fat.extend_from_slice(&want.to_be_bytes());
        fat.extend_from_slice(&3u32.to_be_bytes()); // cpusubtype
        fat.extend_from_slice(&28u32.to_be_bytes()); // offset (right after the arch record)
        fat.extend_from_slice(&(thin.len() as u32).to_be_bytes());
        fat.extend_from_slice(&0u32.to_be_bytes()); // align
        fat.extend_from_slice(&thin);
        let slice = host_slice(&fat).unwrap();
        assert_eq!(slice, &thin[..]);
        // A fat image WITHOUT a host slice is no verdict (execve fails honestly).
        let mut fat2 = Vec::new();
        fat2.extend_from_slice(&FAT_MAGIC.to_be_bytes());
        fat2.extend_from_slice(&1u32.to_be_bytes());
        let other = if cfg!(target_arch = "aarch64") {
            CPU_TYPE_X86_64
        } else {
            CPU_TYPE_ARM64
        };
        fat2.extend_from_slice(&other.to_be_bytes());
        fat2.extend_from_slice(&[0u8; 16]);
        assert!(host_slice(&fat2).is_none());
    }

    #[test]
    fn the_xml_scan_reads_the_first_token_after_the_key() {
        assert_eq!(xml_entitlement_true(XML_TRUE), Some(true));
        assert_eq!(xml_entitlement_true(XML_FALSE), Some(false));
        assert_eq!(xml_entitlement_true(XML_OTHER_ONLY), None);
        // A `<false` mention BEFORE the key (a comment) does not confuse
        // the after-key scan.
        let mut tricky = b"<!-- <false/> -->".to_vec();
        tricky.extend_from_slice(XML_TRUE);
        assert_eq!(xml_entitlement_true(&tricky), Some(true));
    }

    #[test]
    fn the_der_scan_reads_the_boolean_tail() {
        assert_eq!(
            der_entitlement_true(&der_pair(DYLD_ENTITLEMENT_KEY, true)),
            Some(true)
        );
        assert_eq!(
            der_entitlement_true(&der_pair(DYLD_ENTITLEMENT_KEY, false)),
            Some(false)
        );
        assert_eq!(der_entitlement_true(b"no key here"), None);
    }

    #[test]
    fn non_macho_and_short_buffers_are_no_verdict() {
        assert!(host_slice(b"#!/bin/sh\necho hi").is_none());
        assert!(host_slice(b"").is_none());
        assert!(host_slice(&[0xcf, 0xfa]).is_none());
        assert!(dyld_insertion_honored(Path::new("/definitely/not/here")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_test_binary_itself_is_adhoc_and_honored() {
        // This test binary is adhoc-signed (cargo link): the probe reads
        // a real on-disk code signature and takes the insertion. Apple
        // platform binaries (/bin/sh &c.) are NOT a fixture here: their
        // code directories carry no CS_RUNTIME (the SIP/platform plane,
        // owned by the spawn hook's restricted scrub — the probe's
        // contract covers only third-party materialized targets), so
        // they would assert nothing about the hardened reading.
        let exe = std::env::current_exe().unwrap();
        assert!(
            dyld_insertion_honored(&exe),
            "an adhoc test binary takes the insertion"
        );
    }
}
