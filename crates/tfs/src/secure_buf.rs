//! Locked, zeroed buffers for keys and plaintext blocks (spec 10 §3 —
//! the memory discipline).
//!
//! Decrypted blocks and key material live in heap buffers that are
//! `mlock`'d (never swapped) and zeroized on free. `mlock` is attempted
//! best-effort: a failed lock (rlimits, platforms without it) is
//! recorded in [`SecureBuf::locked`] and never fails the operation —
//! the honest statement is "mlock + zeroize is the baseline, not a
//! panacea" (spec 10 §6), and refusing to run under a strict rlimit
//! would break exactly the CI lanes that gate this.
//!
//! The `unsafe` in this module is the FFI-adjacent kind (libc
//! `mlock`/`munlock` and the manual drop), contained here and nowhere
//! else in the ENC transform.

use zeroize::Zeroize;

/// A heap buffer that is `mlock`'d on creation (best effort) and
/// zeroized (then `munlock`'d) on drop.
pub struct SecureBuf {
    buf: std::mem::ManuallyDrop<Vec<u8>>,
    locked: bool,
}

impl SecureBuf {
    /// Allocate `len` zeroed bytes, attempting to lock them.
    pub fn new(len: usize) -> SecureBuf {
        let mut buf = vec![0u8; len];
        // SAFETY: `buf` is a live allocation of exactly `len` bytes;
        // mlock only pins it. Failure is recorded, not fatal.
        let locked = !buf.is_empty() && unsafe { libc::mlock(buf.as_mut_ptr().cast(), len) } == 0;
        SecureBuf {
            buf: std::mem::ManuallyDrop::new(buf),
            locked,
        }
    }

    /// Allocate a locked copy of `data`.
    pub fn from_slice(data: &[u8]) -> SecureBuf {
        let mut out = SecureBuf::new(data.len());
        out.as_mut_slice().copy_from_slice(data);
        out
    }

    /// The buffer contents.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// The buffer contents, mutably.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    /// Buffer length.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// True for a zero-length buffer.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Whether the `mlock` attempt succeeded (informational — best
    /// effort, see the module docs).
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Zero the contents in place (the same routine `Drop` runs).
    pub fn wipe(&mut self) {
        // Zeroize the SLICE: `Vec::zeroize` would also truncate to
        // length 0; the buffer keeps its (zeroed) allocation.
        self.buf.as_mut_slice().zeroize();
    }
}

impl Drop for SecureBuf {
    fn drop(&mut self) {
        self.wipe();
        if self.locked {
            // SAFETY: the buffer was successfully mlock'd at creation
            // and is still live.
            unsafe {
                libc::munlock(self.buf.as_mut_ptr().cast(), self.buf.len());
            }
        }
        // SAFETY: `buf` is dropped exactly once, here.
        unsafe { std::mem::ManuallyDrop::drop(&mut self.buf) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_slice_roundtrips_and_wipe_zeroes() {
        let mut buf = SecureBuf::from_slice(&[0xAA; 64]);
        assert_eq!(buf.as_slice(), &[0xAA; 64]);
        assert_eq!(buf.len(), 64);
        assert!(!buf.is_empty());
        // The routine Drop runs: contents are gone, the allocation isn't.
        buf.wipe();
        assert_eq!(buf.as_slice(), &[0u8; 64]);
    }

    #[test]
    fn new_allocates_zeroed_and_lock_attempt_is_safe() {
        let buf = SecureBuf::new(4096);
        assert!(buf.as_slice().iter().all(|&b| b == 0));
        // locked() is a hint (rlimits differ per machine); the call must
        // simply be safe either way.
        let _ = buf.locked();
        let empty = SecureBuf::new(0);
        assert!(empty.is_empty());
        assert!(!empty.locked());
    }

    #[test]
    fn drop_zeroizes_via_the_same_routine() {
        // Drop itself cannot be observed without reading freed memory
        // (UB); what CAN be asserted is that the drop path runs the
        // wipe routine — proven by construction (Drop::drop calls
        // wipe()) and by the wipe test above.
        let buf = SecureBuf::from_slice(b"key material");
        drop(buf);
    }
}
