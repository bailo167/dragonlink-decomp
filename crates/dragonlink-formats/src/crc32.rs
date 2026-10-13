//! CRC-32 as ZIP, gzip and PNG use it (reflected polynomial `0xEDB88320`).
//!
//! Written here rather than taken from a crate: it is twenty lines, and the
//! archive reader needs nothing else from a checksum library.

/// One table entry per byte value, computed at compile time.
const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut value = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 != 0 {
                0xEDB8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[byte] = value;
        byte += 1;
    }
    table
};

/// A CRC-32 computed over data that arrives in pieces.
#[derive(Debug, Clone, Copy)]
pub struct Crc32 {
    state: u32,
}

impl Default for Crc32 {
    fn default() -> Self {
        Crc32::new()
    }
}

impl Crc32 {
    pub fn new() -> Self {
        Crc32 { state: u32::MAX }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            let index = (self.state ^ u32::from(*byte)) & 0xFF;
            self.state = TABLE[index as usize] ^ (self.state >> 8);
        }
    }

    /// The checksum of everything given to [`Crc32::update`] so far.
    pub fn finish(self) -> u32 {
        !self.state
    }
}

/// CRC-32 of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = Crc32::new();
    crc.update(bytes);
    crc.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_check_values_are_reproduced() {
        // The standard check value of this CRC, and two values any ZIP tool agrees with.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
    }

    #[test]
    fn data_given_in_pieces_has_the_same_checksum() {
        let data: Vec<u8> = (0..=255u8).cycle().take(5000).collect();
        for split in [0, 1, 255, 256, 4999, 5000] {
            let mut crc = Crc32::new();
            crc.update(&data[..split]);
            crc.update(&data[split..]);
            assert_eq!(crc.finish(), crc32(&data), "split at {split}");
        }
    }
}
