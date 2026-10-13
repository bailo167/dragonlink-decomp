//! Shannon entropy of byte streams.
//!
//! Entropy near 8 bits per byte means compressed or encrypted data; text and
//! most structured data sit well below. It is a measurement to record, not a
//! classification: compressed and encrypted data cannot be told apart by it.

/// Counts of each byte value seen, fed incrementally.
#[derive(Debug, Clone)]
pub struct ByteHistogram {
    counts: [u64; 256],
    total: u64,
}

impl Default for ByteHistogram {
    fn default() -> Self {
        ByteHistogram {
            counts: [0; 256],
            total: 0,
        }
    }
}

impl ByteHistogram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.counts[*b as usize] += 1;
        }
        self.total += bytes.len() as u64;
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    /// Entropy in bits per byte, 0.0 to 8.0. Zero for no data.
    pub fn entropy(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let n = self.total as f64;
        self.counts
            .iter()
            .filter(|c| **c > 0)
            .map(|c| {
                let p = *c as f64 / n;
                -p * p.log2()
            })
            .sum()
    }

    /// Fraction of bytes that are printable ASCII or common whitespace.
    pub fn printable_fraction(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let printable: u64 = (0x20..0x7Fusize)
            .chain([0x09, 0x0A, 0x0D])
            .map(|b| self.counts[b])
            .sum();
        printable as f64 / self.total as f64
    }
}

/// Entropy of `bytes` in bits per byte.
pub fn shannon_entropy(bytes: &[u8]) -> f64 {
    let mut h = ByteHistogram::new();
    h.update(bytes);
    h.entropy()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_distributions_have_known_entropy() {
        assert_eq!(shannon_entropy(b""), 0.0);
        assert_eq!(shannon_entropy(&[7u8; 1000]), 0.0);
        // Two values equally often: one bit.
        assert!((shannon_entropy(&[0, 1, 0, 1, 1, 0, 0, 1]) - 1.0).abs() < 1e-12);
        // Every byte value equally often: eight bits.
        let all: Vec<u8> = (0..=255u8).cycle().take(256 * 16).collect();
        assert!((shannon_entropy(&all) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn incremental_updates_match_one_pass() {
        let data: Vec<u8> = (0..5000u32).map(|i| (i * 31 % 251) as u8).collect();
        let mut h = ByteHistogram::new();
        for chunk in data.chunks(77) {
            h.update(chunk);
        }
        assert_eq!(h.total(), 5000);
        assert!((h.entropy() - shannon_entropy(&data)).abs() < 1e-12);
    }

    #[test]
    fn printable_fraction_separates_text_from_binary() {
        let mut text = ByteHistogram::new();
        text.update(b"[workspace]\nmembers = [\"a\"]\r\n\t# comment\n");
        assert_eq!(text.printable_fraction(), 1.0);
        let mut binary = ByteHistogram::new();
        binary.update(&[0u8, 1, 2, 3, 0xFF, 0xFE, b'A', b'B']);
        assert_eq!(binary.printable_fraction(), 0.25);
        assert_eq!(ByteHistogram::new().printable_fraction(), 0.0);
    }
}
