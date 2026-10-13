//! Pay evaluation.
//!
//! Generic line-game rules, stated here so they can be checked against
//! evidence one by one (none has been yet):
//!
//! - Lines pay left to right, starting on the first reel.
//! - A wild substitutes for regular symbols on a line. It does not
//!   substitute for scatters or orbs, which end a line run.
//! - One win per line: the highest of the run counted as the first regular
//!   symbol (with wilds) and the run of leading wilds counted as wilds.
//! - A line win is `pay × credits bet on the line`.
//! - A scatter win counts the symbol anywhere in the window and is
//!   `multiple × total bet`.

use dragonlink_model::{MathModel, SymbolId, SymbolKind};

use crate::grid::Grid;

/// One award found in a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Win {
    Line {
        /// Index into the model's paylines.
        line: u16,
        symbol: SymbolId,
        count: u8,
        award: u64,
    },
    Scatter {
        symbol: SymbolId,
        count: u8,
        award: u64,
    },
}

impl Win {
    pub fn award(&self) -> u64 {
        match self {
            Win::Line { award, .. } | Win::Scatter { award, .. } => *award,
        }
    }
}

/// The paying run for the symbols along one line, left to right:
/// `(symbol, count, pay per credit bet on the line)`, or `None` when the line
/// pays nothing. Public so that exact analysis applies the same rules.
pub fn line_run(model: &MathModel, symbols: &[SymbolId]) -> Option<(SymbolId, u8, u32)> {
    let is_wild = |s: &SymbolId| model.kind(*s) == Some(SymbolKind::Wild);

    let wild_run = symbols.iter().take_while(|s| is_wild(s)).count() as u8;
    let as_wild = symbols
        .first()
        .filter(|_| wild_run > 0)
        .map(|w| (*w, wild_run, model.line_pay(*w, wild_run)));

    let as_regular = symbols
        .iter()
        .find(|s| !is_wild(s))
        .filter(|s| model.kind(**s) == Some(SymbolKind::Regular))
        .map(|first| {
            let run = symbols
                .iter()
                .take_while(|s| *s == first || is_wild(s))
                .count() as u8;
            (*first, run, model.line_pay(*first, run))
        });

    // On equal pay the regular symbol is reported.
    let best = match (as_regular, as_wild) {
        (Some(r), Some(w)) if w.2 > r.2 => Some(w),
        (Some(r), _) => Some(r),
        (None, w) => w,
    };
    best.filter(|(_, _, pay)| *pay > 0)
}

/// Every award in `grid`, lines first (in payline order), then scatters (in
/// symbol order). `None` when an award does not fit in 64 bits.
pub fn evaluate(
    model: &MathModel,
    grid: &Grid,
    credits_per_line: u32,
    total_bet: u64,
) -> Option<Vec<Win>> {
    let mut wins = Vec::new();
    let mut on_line = Vec::with_capacity(model.grid.reels as usize);
    for (index, line) in model.paylines.iter().enumerate() {
        on_line.clear();
        for (reel, row) in line.0.iter().enumerate() {
            on_line.push(grid.get(reel as u8, *row)?);
        }
        if let Some((symbol, count, pay)) = line_run(model, &on_line) {
            wins.push(Win::Line {
                line: index as u16,
                symbol,
                count,
                award: u64::from(pay).checked_mul(u64::from(credits_per_line))?,
            });
        }
    }
    for def in &model.symbols {
        if def.kind != SymbolKind::Scatter {
            continue;
        }
        let count = grid.iter().filter(|(_, _, s)| *s == def.id).count() as u8;
        let multiple = model
            .scatter_pays
            .iter()
            .find(|p| p.symbol == def.id && p.count == count)
            .map_or(0, |p| p.total_bet_multiple);
        if multiple > 0 {
            wins.push(Win::Scatter {
                symbol: def.id,
                count,
                award: u64::from(multiple).checked_mul(total_bet)?,
            });
        }
    }
    Some(wins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    // Symbol ids of the synthetic fixture.
    const A: u16 = 0;
    const B: u16 = 1;
    const E: u16 = 4;
    const H: u16 = 7;
    const WILD: u16 = 8;
    const SCATTER: u16 = 9;
    const ORB: u16 = 10;

    /// A window given as rows (top, middle, bottom), each left to right.
    fn window(rows: [[u16; 5]; 3]) -> Grid {
        let model = synthetic_dev_fixture();
        let mut cells = Vec::new();
        for reel in 0..5 {
            for row in rows {
                cells.push(SymbolId(row[reel]));
            }
        }
        Grid::new(model.grid, cells).unwrap()
    }

    fn line_wins(grid: &Grid, cpl: u32) -> Vec<(u16, u16, u8, u64)> {
        let model = synthetic_dev_fixture();
        evaluate(&model, grid, cpl, 5 * u64::from(cpl))
            .unwrap()
            .into_iter()
            .filter_map(|w| match w {
                Win::Line {
                    line,
                    symbol,
                    count,
                    award,
                } => Some((line, symbol.0, count, award)),
                Win::Scatter { .. } => None,
            })
            .collect()
    }

    #[test]
    fn a_run_from_the_left_pays_per_credit_on_the_line() {
        // Middle row (line 0): A A A then others. Fixture: A×3 pays 20.
        let g = window([[H, E, H, E, H], [A, A, A, E, H], [E, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![(0, A, 3, 20)]);
        assert_eq!(line_wins(&g, 5), vec![(0, A, 3, 100)]);
    }

    #[test]
    fn a_run_must_start_on_the_first_reel() {
        let g = window([[H, E, H, E, H], [E, A, A, A, A], [B, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![]);
    }

    #[test]
    fn wilds_substitute_for_regular_symbols() {
        // A WILD A WILD A on the middle row: A×5 pays 500.
        let g = window([[H, E, H, E, H], [A, WILD, A, WILD, A], [E, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![(0, A, 5, 500)]);
        // Leading wilds take the first regular symbol: WILD WILD B → B×3 pays 15.
        let g = window([[H, E, H, E, H], [WILD, WILD, B, E, H], [E, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![(0, B, 3, 15)]);
    }

    #[test]
    fn scatters_and_orbs_end_a_run_and_are_not_substituted() {
        let g = window([[H, E, H, E, H], [A, A, SCATTER, A, A], [E, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![]);
        let g = window([[H, E, H, E, H], [A, A, ORB, A, A], [E, H, E, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![]);
        let g = window([
            [H, E, H, E, H],
            [WILD, WILD, SCATTER, A, A],
            [E, H, E, H, E],
        ]);
        assert_eq!(line_wins(&g, 1), vec![]);
    }

    #[test]
    fn each_line_is_evaluated_on_its_own_rows() {
        // Top row all A (line 1) and the V line 3 (rows 0,1,2,1,0) with A at its positions.
        let g = window([[A, A, A, A, A], [H, A, E, A, H], [E, H, A, H, E]]);
        assert_eq!(line_wins(&g, 1), vec![(1, A, 5, 500), (3, A, 5, 500)]);
    }

    #[test]
    fn scatters_pay_anywhere_on_the_total_bet() {
        let model = synthetic_dev_fixture();
        let g = window([
            [SCATTER, E, H, E, H],
            [H, E, SCATTER, H, E],
            [E, H, E, H, SCATTER],
        ]);
        let wins = evaluate(&model, &g, 2, 10).unwrap();
        assert_eq!(
            wins,
            vec![Win::Scatter {
                symbol: SymbolId(SCATTER),
                count: 3,
                award: 20
            }]
        );
        // Two scatters pay nothing in the fixture.
        let g = window([
            [SCATTER, E, H, E, H],
            [H, E, SCATTER, H, E],
            [E, H, E, H, E],
        ]);
        assert_eq!(evaluate(&model, &g, 2, 10).unwrap(), vec![]);
    }

    #[test]
    fn an_award_that_does_not_fit_is_reported_not_wrapped() {
        let model = synthetic_dev_fixture();
        let g = window([
            [SCATTER, E, H, E, H],
            [H, E, SCATTER, H, E],
            [E, H, E, H, SCATTER],
        ]);
        assert_eq!(evaluate(&model, &g, 1, u64::MAX), None);
    }
}
