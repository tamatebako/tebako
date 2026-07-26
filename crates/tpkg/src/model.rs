//! Manifest model types, mirroring the C `tpkg_manifest`/`tpkg_slot`.

use crate::error::TpkgError;
use crate::{
    TPKG_FLAG_SIGNED_V2, TPKG_FORMAT_RUNTIME, TPKG_KEYID_LEN, TPKG_MAX_SLOTS, TPKG_MOUNT_POINT_LEN,
    TPKG_RUNTIME_REF_LEN, TPKG_SHA256_LEN, TPKG_SIG_MAX, TPKG_VERSION,
};

/// One payload slot (the in-Rust form of C `tpkg_slot`).
///
/// `mount_point` is the full fixed-width field (NUL-padded on the wire);
/// use [`Slot::mount_point`] / [`Slot::set_mount_point`] for string access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// Image start (absolute file offset).
    pub offset: u64,
    /// Image length in bytes.
    pub size: u64,
    /// `TPKG_FORMAT_*` id.
    pub format_id: u32,
    /// Slot flags (format-specific; 0 for now).
    pub flags: u32,
    /// Fixed-width mount point field (NUL-padded).
    pub mount_point: [u8; TPKG_MOUNT_POINT_LEN],
}

impl Default for Slot {
    fn default() -> Self {
        Slot {
            offset: 0,
            size: 0,
            format_id: 0,
            flags: 0,
            mount_point: [0; TPKG_MOUNT_POINT_LEN],
        }
    }
}

impl Slot {
    /// Create a slot from string parts (convenience).
    pub fn new(offset: u64, size: u64, format_id: u32, mount_point: &str) -> Slot {
        let mut slot = Slot {
            offset,
            size,
            format_id,
            flags: 0,
            ..Default::default()
        };
        slot.set_mount_point(mount_point.as_bytes());
        slot
    }

    /// The mount point as bytes, up to (not including) the first NUL.
    pub fn mount_point(&self) -> &[u8] {
        let len = strnlen(&self.mount_point);
        &self.mount_point[..len]
    }

    /// The mount point as `&str`, if it is valid UTF-8.
    pub fn mount_point_str(&self) -> Option<&str> {
        std::str::from_utf8(self.mount_point()).ok()
    }

    /// Set the mount point from bytes; NUL-pads the remainder.
    /// Bytes at or beyond the field width are ignored.
    pub fn set_mount_point(&mut self, s: &[u8]) {
        put_str(&mut self.mount_point, s);
    }
}

/// The v2 chain-of-trust extension (item 29): per-slot SHA-256 digests,
/// the signer keyid and the OpenPGP detached signature over the canonical
/// trailer bytes. Present iff `package_flags` has `TPKG_FLAG_SIGNED_V2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2Extension {
    /// One SHA-256 digest per possible slot; slot i's digest at index i,
    /// entries beyond the slot count zeroed.
    pub slot_digests: [[u8; TPKG_SHA256_LEN]; TPKG_MAX_SLOTS as usize],
    /// Signer key id: the low 64 bits of the OpenPGP fingerprint (BE on
    /// the wire; kept as raw bytes).
    pub signer_keyid: [u8; TPKG_KEYID_LEN],
    /// The OpenPGP detached signature (binary packets) over the canonical
    /// trailer bytes.
    pub signature: Vec<u8>,
}

impl V2Extension {
    /// The digest of slot `i` (`None` when out of range).
    pub fn slot_digest(&self, i: usize) -> Option<&[u8; TPKG_SHA256_LEN]> {
        self.slot_digests.get(i)
    }

    /// The signer keyid as a 16-character lowercase hex string (the
    /// usual OpenPGP keyid rendering).
    pub fn signer_keyid_hex(&self) -> String {
        crate::codec::hex_lower(&self.signer_keyid)
    }
}

impl Default for V2Extension {
    fn default() -> Self {
        V2Extension {
            slot_digests: [[0; TPKG_SHA256_LEN]; TPKG_MAX_SLOTS as usize],
            signer_keyid: [0; TPKG_KEYID_LEN],
            signature: Vec::new(),
        }
    }
}

/// The package manifest (the in-Rust form of C `tpkg_manifest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Format version (`TPKG_VERSION`; the v2 extension is flagged via
    /// `TPKG_FLAG_SIGNED_V2`, not a version bump, so v1-era readers keep
    /// working).
    pub version: u32,
    /// `TPKG_FLAG_*` bits.
    pub package_flags: u32,
    /// Launcher ABI version understood by the bootstrap.
    pub launcher_abi: u32,
    /// Fixed-width runtime reference field (NUL-padded; empty = classic bundle).
    pub runtime_ref: [u8; TPKG_RUNTIME_REF_LEN],
    /// Payload slots (`1..=TPKG_MAX_SLOTS` entries in a valid manifest).
    pub slots: Vec<Slot>,
    /// Chain-of-trust extension (present iff `package_flags` has
    /// `TPKG_FLAG_SIGNED_V2`).
    pub v2: Option<V2Extension>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            version: TPKG_VERSION,
            package_flags: 0,
            launcher_abi: 0,
            runtime_ref: [0; TPKG_RUNTIME_REF_LEN],
            slots: Vec::new(),
            v2: None,
        }
    }
}

impl Manifest {
    /// The runtime reference as bytes, up to (not including) the first NUL.
    pub fn runtime_ref(&self) -> &[u8] {
        let len = strnlen(&self.runtime_ref);
        &self.runtime_ref[..len]
    }

    /// The runtime reference as `&str`, if it is valid UTF-8.
    pub fn runtime_ref_str(&self) -> Option<&str> {
        std::str::from_utf8(self.runtime_ref()).ok()
    }

    /// Set the runtime reference from bytes; NUL-pads the remainder.
    pub fn set_runtime_ref(&mut self, s: &[u8]) {
        put_str(&mut self.runtime_ref, s);
    }

    /// True when `TPKG_FLAG_LEAN` is set.
    pub fn is_lean(&self) -> bool {
        self.package_flags & crate::TPKG_FLAG_LEAN != 0
    }

    /// Magic-independent structural checks, mirroring the C `tpkg_validate()`:
    /// version supported, `1..=TPKG_MAX_SLOTS` slots, `offset+size`
    /// non-overflowing, `format_id <= TPKG_FORMAT_RUNTIME`, `runtime_ref` and
    /// mount points NUL-terminated within their fixed fields. The v2
    /// extension requires the `TPKG_FLAG_SIGNED_V2` flag and vice versa,
    /// plus zeroed trailing digests, a non-empty signature (bounded by
    /// `TPKG_SIG_MAX`) and a non-zero signer keyid.
    pub fn validate(&self) -> Result<(), TpkgError> {
        if self.version != TPKG_VERSION {
            return Err(TpkgError::Version);
        }
        if self.slots.is_empty() || self.slots.len() > TPKG_MAX_SLOTS as usize {
            return Err(TpkgError::Slots);
        }
        if strnlen(&self.runtime_ref) == TPKG_RUNTIME_REF_LEN {
            return Err(TpkgError::Invalid);
        }
        for slot in &self.slots {
            if slot.size > u64::MAX - slot.offset {
                return Err(TpkgError::Invalid);
            }
            if slot.format_id > TPKG_FORMAT_RUNTIME {
                return Err(TpkgError::Invalid);
            }
            if strnlen(&slot.mount_point) == TPKG_MOUNT_POINT_LEN {
                return Err(TpkgError::Invalid);
            }
        }
        let signed = self.package_flags & TPKG_FLAG_SIGNED_V2 != 0;
        match (signed, &self.v2) {
            (true, Some(v2)) => {
                if v2.slot_digests[self.slots.len()..]
                    .iter()
                    .any(|d| *d != [0; TPKG_SHA256_LEN])
                {
                    return Err(TpkgError::Invalid);
                }
                if v2.signature.is_empty() || v2.signature.len() > TPKG_SIG_MAX as usize {
                    return Err(TpkgError::Invalid);
                }
                if v2.signer_keyid == [0; TPKG_KEYID_LEN] {
                    return Err(TpkgError::Invalid);
                }
            }
            (true, None) | (false, Some(_)) => return Err(TpkgError::Invalid),
            (false, None) => {}
        }
        Ok(())
    }
}

/// Length up to the first NUL, capped at the field width (C `tpkg__strnlen`).
pub(crate) fn strnlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// Copy bytes into a fixed-width field, zero-padding the remainder
/// (C `tpkg__put_str`): content is truncated at the field width and at the
/// first interior NUL.
pub(crate) fn put_str(field: &mut [u8], s: &[u8]) {
    field.fill(0);
    let n = strnlen(s).min(field.len());
    field[..n].copy_from_slice(&s[..n]);
}
