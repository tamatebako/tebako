//! CRC-32 (zlib polynomial `0xEDB88320`, init/xorout `0xFFFFFFFF`),
//! identical to the C implementation's `tpkg_crc32()`.

/// Compute the CRC-32 of `data` (zlib polynomial).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = Crc32::new();
    crc.update(data);
    crc.finish()
}

/// Streaming CRC-32 (same polynomial/result as [`crc32`]): feed chunks with
/// [`Crc32::update`], finalize with [`Crc32::finish`]. Table-driven
/// implementation producing byte-identical results to the bit-wise loop in
/// the C implementation's `tpkg_crc32()` and in `crc32()`.
pub struct Crc32 {
    state: u32,
}

impl Crc32 {
    const fn make_table() -> [u32; 256] {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = (c >> 1) ^ (0xEDB8_8320u32 & 0u32.wrapping_sub(c & 1));
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    }

    /// A fresh CRC-32 state.
    pub fn new() -> Self {
        Crc32 { state: 0xFFFF_FFFF }
    }

    /// Feed a chunk.
    pub fn update(&mut self, data: &[u8]) {
        const TABLE: [u32; 256] = Crc32::make_table();
        for &b in data {
            self.state = TABLE[((self.state ^ u32::from(b)) & 0xFF) as usize] ^ (self.state >> 8);
        }
    }

    /// Finalize and return the CRC-32 value.
    pub fn finish(self) -> u32 {
        self.state ^ 0xFFFF_FFFF
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}
