//! Slot → byte-region resolution (spec 17 §2.1): given a seekable package
//! file and a numeric slot, answer the region to mount. The rules mirror
//! the driver's `resolve_image` exactly — the driver's traced copy stays
//! the boot path's owner (it needs the trailer for mount modes and the
//! trace events); this helper serves the preload shim's
//! `TEBAKO_TFS_MOUNTS` consumption, where slot references arrive by env
//! and no trace bus exists yet.

use std::fmt;
use std::io::{Read, Seek};

use crate::error::TpkgError;
use crate::TPKG_FORMAT_RUNTIME;

/// What a slot reference resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRegion {
    /// Mount the whole file: a bare image (slot 0 on a trailer-less
    /// file — the same answer `build_from_file_at`'s
    /// `offset == 0 && len == 0` convention gives).
    Whole,
    /// Mount `len` bytes at `offset` (a package slot).
    Region {
        /// Absolute file offset of the slot's image.
        offset: u64,
        /// Image length in bytes.
        len: u64,
    },
}

/// A named slot-resolution failure (spec 17 §2.1's named errors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRegionError {
    /// The file could not be read (seek/read failure).
    Io,
    /// The file is a bare image (no slot table) but a non-zero slot was
    /// requested.
    NoSlotTable(u32),
    /// A trailer is present but does not parse.
    CorruptTrailer(TpkgError),
    /// The slot index is beyond the manifest's slot count.
    SlotOutOfRange {
        /// The requested slot.
        slot: u32,
        /// The manifest's slot count.
        slots: usize,
    },
    /// The slot is a runtime payload slot (`TPKG_FORMAT_RUNTIME`) —
    /// runtime payload slots are never mounted.
    RuntimeSlot(u32),
}

impl fmt::Display for SlotRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlotRegionError::Io => write!(f, "cannot read the package file"),
            SlotRegionError::NoSlotTable(slot) => write!(
                f,
                "slot {slot} is out of range (a bare image file — no slot table; use slot 0 or -)"
            ),
            SlotRegionError::CorruptTrailer(e) => {
                write!(f, "corrupt tpkg manifest trailer ({e})")
            }
            SlotRegionError::SlotOutOfRange { slot, slots } => {
                write!(
                    f,
                    "slot {slot} is out of range ({slots} slot(s) in its manifest)"
                )
            }
            SlotRegionError::RuntimeSlot(slot) => write!(
                f,
                "slot {slot} is a runtime payload slot — payload slots are never mounted"
            ),
        }
    }
}

impl std::error::Error for SlotRegionError {}

/// Resolve a numeric slot against the file's tpkg trailer (spec 17 §2.1):
/// a bare file answers [`SlotRegion::Whole`] for slot 0 only; a packaged
/// file answers its slot's region, with out-of-range and runtime-role
/// slots as named errors.
pub fn resolve_slot_region<R: Read + Seek>(
    r: &mut R,
    slot: u32,
) -> Result<SlotRegion, SlotRegionError> {
    let manifest = match crate::io::read_from(r) {
        Ok(m) => m,
        Err(TpkgError::NoTrailer) => {
            return if slot == 0 {
                Ok(SlotRegion::Whole)
            } else {
                Err(SlotRegionError::NoSlotTable(slot))
            };
        }
        Err(TpkgError::Io) => return Err(SlotRegionError::Io),
        Err(e) => return Err(SlotRegionError::CorruptTrailer(e)),
    };
    let Some(s) = manifest.slots.get(slot as usize) else {
        return Err(SlotRegionError::SlotOutOfRange {
            slot,
            slots: manifest.slots.len(),
        });
    };
    if s.format_id == TPKG_FORMAT_RUNTIME {
        return Err(SlotRegionError::RuntimeSlot(slot));
    }
    Ok(SlotRegion::Region {
        offset: s.offset,
        len: s.size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Manifest, Slot};
    use std::io::Cursor;

    /// A 100-byte payload region carrying two slots ([0,40) and [40,100))
    /// plus the appended trailer.
    fn packaged(slots: Vec<Slot>) -> Cursor<Vec<u8>> {
        let mut cur = Cursor::new(vec![0u8; 100]);
        let manifest = Manifest {
            slots,
            ..Default::default()
        };
        crate::io::write_to(&mut cur, &manifest).unwrap();
        cur
    }

    #[test]
    fn resolves_slot_regions() {
        let mut cur = packaged(vec![
            Slot::new(0, 40, crate::TPKG_FORMAT_ZIP, "/a"),
            Slot::new(40, 60, crate::TPKG_FORMAT_DWARFS, "/b"),
        ]);
        assert_eq!(
            resolve_slot_region(&mut cur, 0).unwrap(),
            SlotRegion::Region { offset: 0, len: 40 }
        );
        assert_eq!(
            resolve_slot_region(&mut cur, 1).unwrap(),
            SlotRegion::Region {
                offset: 40,
                len: 60
            }
        );
    }

    #[test]
    fn out_of_range_is_named() {
        let mut cur = packaged(vec![Slot::new(0, 40, crate::TPKG_FORMAT_ZIP, "/a")]);
        assert_eq!(
            resolve_slot_region(&mut cur, 1).unwrap_err(),
            SlotRegionError::SlotOutOfRange { slot: 1, slots: 1 }
        );
    }

    #[test]
    fn runtime_slot_is_never_mounted() {
        let mut cur = packaged(vec![Slot::new(0, 40, crate::TPKG_FORMAT_RUNTIME, "/r")]);
        assert_eq!(
            resolve_slot_region(&mut cur, 0).unwrap_err(),
            SlotRegionError::RuntimeSlot(0)
        );
    }

    #[test]
    fn bare_file_answers_whole_for_slot_zero_only() {
        let mut cur = Cursor::new(vec![0u8; 100]);
        assert_eq!(resolve_slot_region(&mut cur, 0).unwrap(), SlotRegion::Whole);
        assert_eq!(
            resolve_slot_region(&mut cur, 3).unwrap_err(),
            SlotRegionError::NoSlotTable(3)
        );
    }

    #[test]
    fn corrupt_trailer_is_named() {
        // The magic PREFIX present but the full magic wrong: corrupt,
        // never absent (a bare zero/garbage file is legitimately
        // trailer-less — NoTrailer is not corruption).
        let mut bytes = vec![0xAAu8; 2048];
        let trailer_at = bytes.len() - crate::TPKG_HEADER_SIZE;
        bytes[trailer_at..trailer_at + crate::TPKG_MAGIC_PREFIX_LEN]
            .copy_from_slice(&crate::TPKG_MAGIC[..crate::TPKG_MAGIC_PREFIX_LEN]);
        let mut cur = Cursor::new(bytes);
        let err = resolve_slot_region(&mut cur, 0).unwrap_err();
        assert!(
            matches!(err, SlotRegionError::CorruptTrailer(_)),
            "unexpected error: {err:?}"
        );
    }
}
