//! The visible reel window.

use dragonlink_model::{GridShape, SymbolId};
use serde::{Deserialize, Serialize};

/// The symbols showing in the window, in reel-major order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    shape: GridShape,
    cells: Vec<SymbolId>,
}

impl Grid {
    /// A window of `shape` filled from `cells` (reel-major). `None` when the
    /// number of cells does not match the shape.
    pub fn new(shape: GridShape, cells: Vec<SymbolId>) -> Option<Grid> {
        (cells.len() == shape.cells()).then_some(Grid { shape, cells })
    }

    pub fn shape(&self) -> GridShape {
        self.shape
    }

    /// The symbol at `reel`, `row` (row 0 is the top).
    pub fn get(&self, reel: u8, row: u8) -> Option<SymbolId> {
        self.shape
            .index(reel, row)
            .and_then(|i| self.cells.get(i).copied())
    }

    /// The symbols of one reel, top to bottom.
    pub fn reel(&self, reel: u8) -> &[SymbolId] {
        let rows = self.shape.rows as usize;
        let start = reel as usize * rows;
        self.cells.get(start..start + rows).unwrap_or(&[])
    }

    /// All symbols with their positions, in reel-major order.
    pub fn iter(&self) -> impl Iterator<Item = (u8, u8, SymbolId)> + '_ {
        let rows = self.shape.rows.max(1) as usize;
        self.cells
            .iter()
            .enumerate()
            .map(move |(i, s)| ((i / rows) as u8, (i % rows) as u8, *s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_reel_major() {
        let shape = GridShape { reels: 2, rows: 3 };
        let g = Grid::new(shape, (0..6).map(SymbolId).collect()).unwrap();
        assert_eq!(g.get(0, 0), Some(SymbolId(0)));
        assert_eq!(g.get(0, 2), Some(SymbolId(2)));
        assert_eq!(g.get(1, 0), Some(SymbolId(3)));
        assert_eq!(g.get(2, 0), None);
        assert_eq!(g.reel(1), &[SymbolId(3), SymbolId(4), SymbolId(5)]);
        assert_eq!(g.reel(2), &[] as &[SymbolId]);
        let all: Vec<_> = g.iter().collect();
        assert_eq!(all[4], (1, 1, SymbolId(4)));
        assert!(Grid::new(shape, vec![SymbolId(0)]).is_none());
    }
}
