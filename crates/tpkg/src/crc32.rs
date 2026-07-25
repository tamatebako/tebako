//! CRC-32 (zlib polynomial `0xEDB88320`, init/xorout `0xFFFFFFFF`),
//! identical to the C implementation's `tpkg_crc32()`.

/// Compute the CRC-32 of `data` (zlib polynomial).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320u32 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc ^ 0xFFFF_FFFF
}
