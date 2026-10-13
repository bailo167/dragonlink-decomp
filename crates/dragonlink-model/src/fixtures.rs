//! Synthetic development fixtures.
//!
//! **Nothing in this file was recovered from, measured on, fitted to or
//! otherwise derived from any Aristocrat or Product Madness product.** The
//! reel strips, paylines, pays, chances, value tables and jackpot amounts
//! below were typed in by this project so that the engine, the trace tooling
//! and the renderer have something deterministic to run. The symbols are
//! named `A`…`H`, `WILD`, `SCATTER` and `ORB` on purpose: they are not the
//! symbols of any title.
//!
//! Any result produced with this fixture (hit rates, return, feature
//! frequency, a screenshot) is a property of the fixture. It is never to be
//! quoted as a property of Dragon Link.

use crate::layout::{GridShape, Payline};
use crate::math::{
    BetModel, HoldAndSpinModel, JackpotDef, JackpotTier, LinePay, MathModel, MathProvenance,
    OrbValue, Ratio, ScatterPay, WeightedOrbValue,
};
use crate::symbol::{SymbolDef, SymbolId, SymbolKind};

/// Identifier of [`synthetic_dev_fixture`], written into trace headers.
pub const SYNTHETIC_DEV_FIXTURE_ID: &str = "synthetic-dev-2";

const A: u16 = 0;
const B: u16 = 1;
const C: u16 = 2;
const D: u16 = 3;
const E: u16 = 4;
const F: u16 = 5;
const G: u16 = 6;
const H: u16 = 7;
const WILD: u16 = 8;
const SCATTER: u16 = 9;
const ORB: u16 = 10;

fn strip(ids: &[u16]) -> Vec<SymbolId> {
    ids.iter().copied().map(SymbolId).collect()
}

/// The synthetic development fixture: a 5×3 window, five paylines, eight
/// regular symbols, a wild, a scatter and an orb, with a Hold & Spin feature.
///
/// The five paylines and 24-stop strips are deliberately unlike a real
/// machine, so that output from this fixture is not mistaken for one.
pub fn synthetic_dev_fixture() -> MathModel {
    let symbols = [
        ("A", SymbolKind::Regular),
        ("B", SymbolKind::Regular),
        ("C", SymbolKind::Regular),
        ("D", SymbolKind::Regular),
        ("E", SymbolKind::Regular),
        ("F", SymbolKind::Regular),
        ("G", SymbolKind::Regular),
        ("H", SymbolKind::Regular),
        ("WILD", SymbolKind::Wild),
        ("SCATTER", SymbolKind::Scatter),
        ("ORB", SymbolKind::Orb),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (key, kind))| SymbolDef {
        id: SymbolId(i as u16),
        key: key.to_string(),
        kind,
    })
    .collect();

    // Each strip has one block of two orbs and one single orb, so that the
    // window can hold enough orbs to start the feature. The strips, chances and
    // values were adjusted with `dragonlink-analyze` until the fixture had a
    // sane shape (a return a little under 100%, the feature about once in 78
    // spins). They were tuned for usability as a development fixture and
    // towards no real game's figures, which are unknown.
    let reel_strips = vec![
        strip(&[
            A, E, F, A, ORB, ORB, G, H, B, E, F, G, SCATTER, H, C, F, ORB, D, E, G, H, A, F, H,
        ]),
        strip(&[
            B, F, G, H, WILD, E, C, G, B, ORB, ORB, H, A, F, E, SCATTER, H, D, F, ORB, G, H, B, E,
        ]),
        strip(&[
            C, G, H, E, F, A, H, G, WILD, F, D, C, ORB, ORB, E, H, G, B, SCATTER, F, ORB, H, E, G,
        ]),
        strip(&[
            D, H, E, F, G, WILD, B, H, E, A, G, F, H, SCATTER, E, C, D, ORB, ORB, G, F, ORB, H, E,
        ]),
        strip(&[
            A, G, F, E, H, C, G, E, H, B, F, SCATTER, G, D, A, ORB, ORB, H, E, F, ORB, A, H, E,
        ]),
    ];

    let paylines = [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [2, 2, 2, 2, 2],
        [0, 1, 2, 1, 0],
        [2, 1, 0, 1, 2],
    ]
    .into_iter()
    .map(|rows| Payline(rows.to_vec()))
    .collect();

    let mut line_pays = Vec::new();
    for (symbol, pays) in [
        (A, [20, 100, 500]),
        (B, [15, 75, 300]),
        (C, [10, 50, 200]),
        (D, [10, 40, 150]),
        (E, [5, 20, 100]),
        (F, [5, 20, 100]),
        (G, [5, 15, 75]),
        (H, [5, 15, 75]),
    ] {
        for (count, pay) in (3u8..=5).zip(pays) {
            line_pays.push(LinePay {
                symbol: SymbolId(symbol),
                count,
                pay,
            });
        }
    }

    let scatter_pays = vec![
        ScatterPay {
            symbol: SymbolId(SCATTER),
            count: 3,
            total_bet_multiple: 2,
        },
        ScatterPay {
            symbol: SymbolId(SCATTER),
            count: 4,
            total_bet_multiple: 10,
        },
        ScatterPay {
            symbol: SymbolId(SCATTER),
            count: 5,
            total_bet_multiple: 50,
        },
    ];

    let orb_values = [
        (400, OrbValue::BetMultiple(2)),
        (250, OrbValue::BetMultiple(4)),
        (150, OrbValue::BetMultiple(6)),
        (100, OrbValue::BetMultiple(10)),
        (50, OrbValue::BetMultiple(20)),
        (30, OrbValue::Jackpot(JackpotTier::Mini)),
        (19, OrbValue::Jackpot(JackpotTier::Minor)),
        (1, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();

    let jackpots = [
        (JackpotTier::Mini, 10),
        (JackpotTier::Minor, 25),
        (JackpotTier::Major, 250),
        (JackpotTier::Grand, 2500),
    ]
    .into_iter()
    .map(|(tier, total_bet_multiple)| JackpotDef {
        tier,
        total_bet_multiple,
    })
    .collect();

    MathModel {
        id: SYNTHETIC_DEV_FIXTURE_ID.to_string(),
        provenance: MathProvenance::SyntheticDevFixture,
        description: "Synthetic development fixture invented by dragonlink-decomp. Not recovered \
                      from any original game; do not quote its statistics as Dragon Link's."
            .to_string(),
        grid: GridShape { reels: 5, rows: 3 },
        symbols,
        reel_strips,
        paylines,
        line_pays,
        scatter_pays,
        bet: BetModel {
            credits_per_line_options: vec![1, 2, 5, 10],
        },
        hold_and_spin: Some(HoldAndSpinModel {
            orb: SymbolId(ORB),
            trigger_count: 6,
            initial_respins: 3,
            reset_respins_on_new_orb: true,
            orb_land_chance: Ratio { num: 1, den: 20 },
            orb_values,
            full_grid_award: Some(JackpotTier::Grand),
            jackpots,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_is_valid_and_labelled_synthetic() {
        let m = synthetic_dev_fixture();
        assert_eq!(m.validate(), Ok(()));
        assert_eq!(m.provenance, MathProvenance::SyntheticDevFixture);
        assert_eq!(m.id, SYNTHETIC_DEV_FIXTURE_ID);
        assert!(m.description.contains("Synthetic"));
    }

    #[test]
    fn the_fixture_does_not_use_title_symbol_names() {
        // Generic keys only: a trace from this fixture must not read like a title's.
        let m = synthetic_dev_fixture();
        let keys: Vec<&str> = m.symbols.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "A", "B", "C", "D", "E", "F", "G", "H", "WILD", "SCATTER", "ORB"
            ]
        );
    }
}
