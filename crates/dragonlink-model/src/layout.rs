//! Reel window layout.

use serde::{Deserialize, Serialize};

/// Size of the visible reel window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridShape {
    pub reels: u8,
    pub rows: u8,
}

impl GridShape {
    /// Number of visible positions.
    pub fn cells(self) -> usize {
        self.reels as usize * self.rows as usize
    }

    /// Index of a position in reel-major order (all rows of reel 0, then reel 1, …).
    pub fn index(self, reel: u8, row: u8) -> Option<usize> {
        (reel < self.reels && row < self.rows)
            .then(|| reel as usize * self.rows as usize + row as usize)
    }
}

/// A payline: the row it crosses on each reel, left to right.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Payline(pub Vec<u8>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_reel_major_and_bounded() {
        let g = GridShape { reels: 5, rows: 3 };
        assert_eq!(g.cells(), 15);
        assert_eq!(g.index(0, 0), Some(0));
        assert_eq!(g.index(0, 2), Some(2));
        assert_eq!(g.index(1, 0), Some(3));
        assert_eq!(g.index(4, 2), Some(14));
        assert_eq!(g.index(5, 0), None);
        assert_eq!(g.index(0, 3), None);
    }
}
