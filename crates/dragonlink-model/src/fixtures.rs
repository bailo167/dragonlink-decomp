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
    BetModel, FreeGamesModel, HoldAndSpinModel, JackpotDef, JackpotTier, LinePay, MathModel,
    MathProvenance, OrbValue, Ratio, ScatterPay, WeightedOrbValue, WeightedSymbol,
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
            line_options: Vec::new(),
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
        free_games: None,
    }
}

/// Identifier of [`golden_century_synthetic`], written into trace headers.
pub const GOLDEN_CENTURY_SYNTHETIC_ID: &str = "golden-century-synthetic-1";

/// A **synthetic** Golden Century configuration: the game's *evidenced
/// structure* with *invented* numbers.
///
/// What is shaped from public evidence
/// (`docs/reverse-engineering/GOLDEN_CENTURY_SESSION.md` and `FOOTAGE.md`): a
/// 5x3 window; the symbol set and roles (nine, ten, jack, queen, king; four
/// pictures; a wild that appears on reels 2 to 4 only and substitutes for the
/// paying symbols; a pennant scatter; an orb prize symbol); the shapes of
/// paylines 1 to 5; three scatters starting the free games; six orbs starting
/// Hold & Spin with three respins that reset on a new orb and fifteen filling
/// the grid; the Grand/Major/Minor/Mini jackpot tiers; denominations 1c to $1
/// and player-selected line counts.
///
/// What is **invented** by this project, and must never be quoted as Dragon
/// Link's: every reel strip, every weight, every pay not read from a machine,
/// the scatter and orb value tables, the land chance, the jackpot amounts
/// (fixed multiples here; the real levels are progressive and unknown), and
/// paylines 6 to 50. Pays matching a reading of a real machine are marked in
/// the source; the rest are plausible fillers chosen only so the game plays.
///
/// This exists so Golden Century can be played end to end now. When authentic
/// values are recovered they replace this configuration without the game
/// architecture changing (`docs/DECISIONS.md`, D-017).
pub fn golden_century_synthetic() -> MathModel {
    // Roles from evidence; indices are this fixture's own.
    const NINE: u16 = 0;
    const TEN: u16 = 1;
    const JACK: u16 = 2;
    const QUEEN: u16 = 3;
    const KING: u16 = 4;
    const WARRIORS: u16 = 5;
    const VASE: u16 = 6;
    const CHEST: u16 = 7;
    const WOMEN: u16 = 8;
    const EMPEROR: u16 = 9; // wild, reels 2 to 4 only
    const PENNANT: u16 = 10; // scatter
    const ORB: u16 = 11; // Hold & Spin prize symbol
    const DOORS: u16 = 12; // free-games mystery symbol

    let symbols = [
        ("9", SymbolKind::Regular),
        ("10", SymbolKind::Regular),
        ("J", SymbolKind::Regular),
        ("Q", SymbolKind::Regular),
        ("K", SymbolKind::Regular),
        ("warriors", SymbolKind::Regular),
        ("vase", SymbolKind::Regular),
        ("chest", SymbolKind::Regular),
        ("women", SymbolKind::Regular),
        ("emperor", SymbolKind::Wild),
        ("pennant", SymbolKind::Scatter),
        ("orb", SymbolKind::Orb),
        ("doors", SymbolKind::Regular),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (key, kind))| SymbolDef {
        id: SymbolId(i as u16),
        key: key.to_string(),
        kind,
    })
    .collect::<Vec<_>>();

    // One strip per reel, built from a composition so the counts are plain.
    // An orb block and (on reels 2 to 4) a wild block sit among filler royals
    // and pictures, with one scatter. `rep` is a run of one symbol.
    let reel = |parts: &[(u16, usize)]| -> Vec<SymbolId> {
        parts
            .iter()
            .flat_map(|(sym, n)| std::iter::repeat_n(*sym, *n))
            .map(SymbolId)
            .collect()
    };

    // Filler shaped so royals are common and pictures rarer, the top picture
    // (women) rarest. Orbs come in a block of three so a window can show two
    // or three; six across the reels start the feature.
    let reel_strips = vec![
        reel(&[
            (NINE, 6),
            (ORB, 3),
            (TEN, 5),
            (WARRIORS, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (VASE, 2),
            (NINE, 6),
            (WOMEN, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 5),
            (EMPEROR, 2),
            (NINE, 6),
            (ORB, 3),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (CHEST, 1),
            (WARRIORS, 2),
            (TEN, 6),
            (WOMEN, 1),
            (NINE, 6),
            (JACK, 5),
        ]),
        reel(&[
            (JACK, 6),
            (ORB, 3),
            (NINE, 6),
            (EMPEROR, 2),
            (TEN, 5),
            (WARRIORS, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (VASE, 2),
            (KING, 4),
            (CHEST, 1),
            (NINE, 6),
            (WOMEN, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (NINE, 6),
            (ORB, 3),
            (TEN, 5),
            (EMPEROR, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (VASE, 2),
            (PENNANT, 1),
            (KING, 4),
            (WARRIORS, 2),
            (NINE, 6),
            (WOMEN, 1),
            (TEN, 6),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 6),
            (ORB, 3),
            (NINE, 6),
            (WARRIORS, 2),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (CHEST, 1),
            (TEN, 6),
            (WOMEN, 1),
            (NINE, 6),
            (JACK, 5),
        ]),
    ];

    // Paylines 1 to 5 are the evidenced shapes; 6 to 50 are synthetic,
    // generated as distinct row patterns so there is a full set to play 5, 25
    // or 50 lines.
    let known = [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [2, 2, 2, 2, 2],
        [0, 1, 2, 1, 0],
        [2, 1, 0, 1, 2],
    ];
    let mut paylines: Vec<Payline> = known.iter().map(|rows| Payline(rows.to_vec())).collect();
    // Patterns in base 3, skipping the five above and every all-equal row,
    // until there are fifty lines.
    let mut pattern = 0u32;
    while paylines.len() < 50 {
        let rows: Vec<u8> = (0..5)
            .map(|r| ((pattern / 3u32.pow(r)) % 3) as u8)
            .collect();
        let all_same = rows.iter().all(|r| *r == rows[0]);
        if !all_same && !known.iter().any(|k| k[..] == rows[..]) {
            paylines.push(Payline(rows));
        }
        pattern += 1;
    }

    // Line pays, per credit bet on the line. Entries marked "read" match a
    // reading of a real Australian machine (GOLDEN_CENTURY_SESSION.md); the
    // rest are synthetic fillers, descending and plausible, never authentic.
    // Count 2 is only for the four pictures; a pay of 0 means no win for that
    // count.
    let mut line_pays = Vec::new();
    for (symbol, pays) in [
        //               2   3   4    5
        (WOMEN, [2, 5, 30, 150]),     // 2,3,4,5 all read
        (CHEST, [2, 5, 25, 100]),     // 2,3,5 read; 4 synthetic
        (VASE, [2, 5, 25, 100]),      // 2,3 read; 4,5 synthetic
        (WARRIORS, [3, 15, 40, 100]), // 2 read ($1); rest synthetic
        (KING, [0, 5, 20, 75]),       // 3 read; rest synthetic
        (QUEEN, [0, 5, 15, 60]),      // 3 read; rest synthetic
        (JACK, [0, 5, 10, 50]),       // 3,4 read; 5 synthetic
        (TEN, [0, 5, 10, 50]),        // 3,4 read; 5 synthetic
        (NINE, [0, 5, 10, 40]),       // synthetic
    ] {
        for (count, pay) in (2u8..=5).zip(pays) {
            if pay > 0 {
                line_pays.push(LinePay {
                    symbol: SymbolId(symbol),
                    count,
                    pay,
                });
            }
        }
    }

    // Scatter: three pennants pay twice the total bet (read); four and five
    // synthetic.
    let scatter_pays = vec![
        ScatterPay {
            symbol: SymbolId(PENNANT),
            count: 3,
            total_bet_multiple: 2,
        },
        ScatterPay {
            symbol: SymbolId(PENNANT),
            count: 4,
            total_bet_multiple: 10,
        },
        ScatterPay {
            symbol: SymbolId(PENNANT),
            count: 5,
            total_bet_multiple: 50,
        },
    ];

    // Orb value table. The twelve prize multiples a player sees
    // (1,2,3,4,5,10,15,20,50,100,200,250 times the total bet) and the four
    // levels are the official visible values; only the weights are synthetic
    // (small prizes common, big ones rare).
    let orb_values = [
        (260, OrbValue::BetMultiple(1)),
        (220, OrbValue::BetMultiple(2)),
        (160, OrbValue::BetMultiple(3)),
        (120, OrbValue::BetMultiple(4)),
        (90, OrbValue::BetMultiple(5)),
        (70, OrbValue::BetMultiple(10)),
        (46, OrbValue::BetMultiple(15)),
        (28, OrbValue::BetMultiple(20)),
        (12, OrbValue::BetMultiple(50)),
        (6, OrbValue::BetMultiple(100)),
        (3, OrbValue::BetMultiple(200)),
        (2, OrbValue::BetMultiple(250)),
        (45, OrbValue::Jackpot(JackpotTier::Mini)),
        (14, OrbValue::Jackpot(JackpotTier::Minor)),
        (3, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();

    // Jackpot amounts are synthetic fixed multiples. The real Grand and Major
    // are progressive and their values are unknown; the Minor and Mini are
    // fixed per denomination (GOLDEN_CENTURY_SESSION.md) but not these numbers.
    let jackpots = [
        (JackpotTier::Mini, 15),
        (JackpotTier::Minor, 60),
        (JackpotTier::Major, 300),
        (JackpotTier::Grand, 1000),
    ]
    .into_iter()
    .map(|(tier, total_bet_multiple)| JackpotDef {
        tier,
        total_bet_multiple,
    })
    .collect();

    // Free-game reels: like the base reels but with no orbs (so the Hold &
    // Spin feature does not nest here) and a mystery symbol (`doors`) that,
    // when it appears, reveals to one drawn symbol across all its positions,
    // as Golden Century's free games do. The scatter stays, so free games can
    // retrigger. Strips, weights and the reveal table are synthetic.
    let free_reel_strips = vec![
        reel(&[
            (NINE, 6),
            (DOORS, 2),
            (TEN, 5),
            (WARRIORS, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (VASE, 2),
            (NINE, 5),
            (WOMEN, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 5),
            (EMPEROR, 2),
            (NINE, 6),
            (DOORS, 2),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (CHEST, 1),
            (WARRIORS, 2),
            (TEN, 6),
            (WOMEN, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
        reel(&[
            (JACK, 6),
            (DOORS, 2),
            (NINE, 6),
            (EMPEROR, 2),
            (TEN, 5),
            (WARRIORS, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (VASE, 2),
            (KING, 4),
            (CHEST, 1),
            (NINE, 5),
            (WOMEN, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (NINE, 6),
            (DOORS, 2),
            (TEN, 5),
            (EMPEROR, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (VASE, 2),
            (PENNANT, 1),
            (KING, 4),
            (WARRIORS, 2),
            (NINE, 5),
            (WOMEN, 1),
            (TEN, 6),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 6),
            (DOORS, 2),
            (NINE, 6),
            (WARRIORS, 2),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (KING, 4),
            (CHEST, 1),
            (TEN, 5),
            (WOMEN, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
    ];
    // What the mystery reveals to, weighted towards the lower symbols. Synthetic.
    let reveal_values = [
        (28, NINE),
        (24, TEN),
        (20, JACK),
        (14, QUEEN),
        (10, KING),
        (7, WARRIORS),
        (5, VASE),
        (4, CHEST),
        (2, WOMEN),
    ]
    .into_iter()
    .map(|(weight, symbol)| WeightedSymbol {
        weight,
        symbol: SymbolId(symbol),
    })
    .collect();

    MathModel {
        id: GOLDEN_CENTURY_SYNTHETIC_ID.to_string(),
        provenance: MathProvenance::SyntheticDevFixture,
        description: "SYNTHETIC Golden Century: the game's evidenced structure (5x3, symbol roles, \
                      paylines 1-5, scatter-triggered free games, six-orb Hold & Spin, four jackpot \
                      tiers, denominations and line selection) with INVENTED strips, weights, pays \
                      and jackpot amounts. Not recovered from any machine; never quote its numbers \
                      as Dragon Link's (docs/reverse-engineering/GOLDEN_CENTURY_SESSION.md, \
                      docs/DECISIONS.md D-017)."
            .to_string(),
        grid: GridShape { reels: 5, rows: 3 },
        symbols,
        reel_strips,
        paylines,
        line_pays,
        scatter_pays,
        bet: BetModel {
            credits_per_line_options: vec![1, 2, 3, 4, 5, 10],
            line_options: vec![5, 25, 50],
        },
        hold_and_spin: Some(HoldAndSpinModel {
            orb: SymbolId(ORB),
            trigger_count: 6,
            initial_respins: 3,
            reset_respins_on_new_orb: true,
            orb_land_chance: Ratio { num: 1, den: 14 },
            orb_values,
            full_grid_award: Some(JackpotTier::Grand),
            jackpots,
        }),
        free_games: Some(FreeGamesModel {
            trigger_count: 3,  // three scatters (read)
            initial_games: 6,  // six free games (official)
            retrigger_count: 3,
            retrigger_games: 6, // synthetic
            free_reel_strips,
            mystery: Some(SymbolId(DOORS)),
            reveal_values,
        }),
    }
}

/// Identifier of [`autumn_moon_synthetic`], written into trace headers.
pub const AUTUMN_MOON_SYNTHETIC_ID: &str = "autumn-moon-synthetic-1";

/// A **synthetic** Autumn Moon configuration: the game's *evidenced structure*
/// with *invented* numbers. Autumn Moon runs on the same engine family as
/// Golden Century, so it shares this project's Dragon Link simulation; only
/// the symbols, the pay table and the free-games reveal differ
/// (`docs/reverse-engineering/FOOTAGE.md`).
///
/// Official visible values used here: the US family pay table (top picture
/// 150/50/10 and 2 for two; others 3 to 5), the scatter 100/15/2 times the
/// total bet, six free games, and the twelve orb prize multiples. Invented and
/// never to be quoted as Dragon Link's: every reel strip, weight, the reveal
/// table, the land chance and the jackpot amounts. Autumn Moon's free-games
/// wild line pay is not yet modelled (the wild substitutes only); noted as a
/// fidelity gap.
pub fn autumn_moon_synthetic() -> MathModel {
    const NINE: u16 = 0;
    const TEN: u16 = 1;
    const JACK: u16 = 2;
    const QUEEN: u16 = 3;
    const KING: u16 = 4;
    const LUTE: u16 = 5; // fourth picture
    const FAN: u16 = 6; // third picture
    const MOONCAKE: u16 = 7; // second picture
    const RABBIT: u16 = 8; // top picture
    const WOMAN: u16 = 9; // wild, reels 2 to 4 only
    const MOON: u16 = 10; // scatter
    const ORB: u16 = 11; // Hold & Spin prize symbol
    const MEDALLION: u16 = 12; // free-games mystery symbol

    let symbols = [
        ("9", SymbolKind::Regular),
        ("10", SymbolKind::Regular),
        ("J", SymbolKind::Regular),
        ("Q", SymbolKind::Regular),
        ("K", SymbolKind::Regular),
        ("lute", SymbolKind::Regular),
        ("fan", SymbolKind::Regular),
        ("mooncake", SymbolKind::Regular),
        ("rabbit", SymbolKind::Regular),
        ("woman", SymbolKind::Wild),
        ("moon", SymbolKind::Scatter),
        ("orb", SymbolKind::Orb),
        ("medallion", SymbolKind::Regular),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (key, kind))| SymbolDef {
        id: SymbolId(i as u16),
        key: key.to_string(),
        kind,
    })
    .collect::<Vec<_>>();

    let reel = |parts: &[(u16, usize)]| -> Vec<SymbolId> {
        parts
            .iter()
            .flat_map(|(sym, n)| std::iter::repeat_n(*sym, *n))
            .map(SymbolId)
            .collect()
    };

    // The base reels: royals common, pictures rarer, the top picture (rabbit)
    // rarest; an orb block of three per reel, a wild block on reels 2 to 4, one
    // scatter. Synthetic.
    let reel_strips = vec![
        reel(&[
            (NINE, 6),
            (ORB, 3),
            (TEN, 5),
            (LUTE, 2),
            (JACK, 5),
            (MOONCAKE, 1),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (FAN, 2),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 5),
            (WOMAN, 2),
            (NINE, 6),
            (ORB, 3),
            (JACK, 5),
            (FAN, 2),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (MOONCAKE, 1),
            (LUTE, 2),
            (TEN, 6),
            (RABBIT, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
        reel(&[
            (JACK, 6),
            (ORB, 3),
            (NINE, 6),
            (WOMAN, 2),
            (TEN, 5),
            (LUTE, 2),
            (QUEEN, 4),
            (MOON, 1),
            (FAN, 2),
            (KING, 4),
            (MOONCAKE, 1),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (NINE, 6),
            (ORB, 3),
            (TEN, 5),
            (WOMAN, 2),
            (JACK, 5),
            (MOONCAKE, 1),
            (QUEEN, 4),
            (FAN, 2),
            (MOON, 1),
            (KING, 4),
            (LUTE, 2),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 6),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 6),
            (ORB, 3),
            (NINE, 6),
            (LUTE, 2),
            (JACK, 5),
            (FAN, 2),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (MOONCAKE, 1),
            (TEN, 5),
            (RABBIT, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
    ];

    // Paylines 1 to 5 are the evidenced shapes; 6 to 50 synthetic.
    let known = [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [2, 2, 2, 2, 2],
        [0, 1, 2, 1, 0],
        [2, 1, 0, 1, 2],
    ];
    let mut paylines: Vec<Payline> = known.iter().map(|rows| Payline(rows.to_vec())).collect();
    let mut pattern = 0u32;
    while paylines.len() < 50 {
        let rows: Vec<u8> = (0..5)
            .map(|r| ((pattern / 3u32.pow(r)) % 3) as u8)
            .collect();
        let all_same = rows.iter().all(|r| *r == rows[0]);
        if !all_same && !known.iter().any(|k| k[..] == rows[..]) {
            paylines.push(Payline(rows));
        }
        pattern += 1;
    }

    // Line pays, per credit on the line. The US family pay table (official
    // visible values); only the top picture pays for two of a kind.
    let mut line_pays = Vec::new();
    for (symbol, pays) in [
        //                2   3   4    5
        (RABBIT, [2, 10, 50, 150]),
        (MOONCAKE, [0, 5, 30, 100]),
        (FAN, [0, 5, 25, 100]),
        (LUTE, [0, 5, 25, 100]),
        (KING, [0, 5, 20, 75]),
        (QUEEN, [0, 5, 10, 50]),
        (JACK, [0, 5, 10, 50]),
        (TEN, [0, 5, 10, 50]),
        (NINE, [0, 5, 10, 50]),
    ] {
        for (count, pay) in (2u8..=5).zip(pays) {
            if pay > 0 {
                line_pays.push(LinePay {
                    symbol: SymbolId(symbol),
                    count,
                    pay,
                });
            }
        }
    }

    // Scatter: the family's 100/15/2 times the total bet (official visible).
    let scatter_pays = vec![
        ScatterPay {
            symbol: SymbolId(MOON),
            count: 3,
            total_bet_multiple: 2,
        },
        ScatterPay {
            symbol: SymbolId(MOON),
            count: 4,
            total_bet_multiple: 15,
        },
        ScatterPay {
            symbol: SymbolId(MOON),
            count: 5,
            total_bet_multiple: 100,
        },
    ];

    // Orb values: the twelve official multiples and the four levels; synthetic
    // weights (as Golden Century).
    let orb_values = [
        (260, OrbValue::BetMultiple(1)),
        (220, OrbValue::BetMultiple(2)),
        (160, OrbValue::BetMultiple(3)),
        (120, OrbValue::BetMultiple(4)),
        (90, OrbValue::BetMultiple(5)),
        (70, OrbValue::BetMultiple(10)),
        (46, OrbValue::BetMultiple(15)),
        (28, OrbValue::BetMultiple(20)),
        (12, OrbValue::BetMultiple(50)),
        (6, OrbValue::BetMultiple(100)),
        (3, OrbValue::BetMultiple(200)),
        (2, OrbValue::BetMultiple(250)),
        (45, OrbValue::Jackpot(JackpotTier::Mini)),
        (14, OrbValue::Jackpot(JackpotTier::Minor)),
        (3, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();

    let jackpots = [
        (JackpotTier::Mini, 15),
        (JackpotTier::Minor, 60),
        (JackpotTier::Major, 300),
        (JackpotTier::Grand, 1000),
    ]
    .into_iter()
    .map(|(tier, total_bet_multiple)| JackpotDef {
        tier,
        total_bet_multiple,
    })
    .collect();

    // Free-game reels: orb-free, with the medallion mystery and the scatter.
    let free_reel_strips = vec![
        reel(&[
            (NINE, 6),
            (MEDALLION, 2),
            (TEN, 5),
            (LUTE, 2),
            (JACK, 5),
            (MOONCAKE, 1),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (FAN, 2),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 5),
            (WOMAN, 2),
            (NINE, 6),
            (MEDALLION, 2),
            (JACK, 5),
            (FAN, 2),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (MOONCAKE, 1),
            (LUTE, 2),
            (TEN, 6),
            (RABBIT, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
        reel(&[
            (JACK, 6),
            (MEDALLION, 2),
            (NINE, 6),
            (WOMAN, 2),
            (TEN, 5),
            (LUTE, 2),
            (QUEEN, 4),
            (MOON, 1),
            (FAN, 2),
            (KING, 4),
            (MOONCAKE, 1),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 5),
            (JACK, 5),
        ]),
        reel(&[
            (NINE, 6),
            (MEDALLION, 2),
            (TEN, 5),
            (WOMAN, 2),
            (JACK, 5),
            (MOONCAKE, 1),
            (QUEEN, 4),
            (FAN, 2),
            (MOON, 1),
            (KING, 4),
            (LUTE, 2),
            (NINE, 5),
            (RABBIT, 1),
            (TEN, 6),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 6),
            (MEDALLION, 2),
            (NINE, 6),
            (LUTE, 2),
            (JACK, 5),
            (FAN, 2),
            (QUEEN, 4),
            (MOON, 1),
            (KING, 4),
            (MOONCAKE, 1),
            (TEN, 5),
            (RABBIT, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
    ];
    let reveal_values = [
        (28, NINE),
        (24, TEN),
        (20, JACK),
        (14, QUEEN),
        (10, KING),
        (7, LUTE),
        (5, FAN),
        (4, MOONCAKE),
        (2, RABBIT),
    ]
    .into_iter()
    .map(|(weight, symbol)| WeightedSymbol {
        weight,
        symbol: SymbolId(symbol),
    })
    .collect();

    MathModel {
        id: AUTUMN_MOON_SYNTHETIC_ID.to_string(),
        provenance: MathProvenance::SyntheticDevFixture,
        description: "SYNTHETIC Autumn Moon: the game's evidenced structure (the Golden Century \
                      engine family, with its own symbols and the US family pay table) with \
                      INVENTED strips, weights and jackpot amounts. Not recovered from any \
                      machine; never quote its numbers as Dragon Link's \
                      (docs/reverse-engineering/FOOTAGE.md, docs/DECISIONS.md D-017)."
            .to_string(),
        grid: GridShape { reels: 5, rows: 3 },
        symbols,
        reel_strips,
        paylines,
        line_pays,
        scatter_pays,
        bet: BetModel {
            credits_per_line_options: vec![1, 2, 3, 4, 5, 10],
            line_options: vec![5, 25, 50],
        },
        hold_and_spin: Some(HoldAndSpinModel {
            orb: SymbolId(ORB),
            trigger_count: 6,
            initial_respins: 3,
            reset_respins_on_new_orb: true,
            orb_land_chance: Ratio { num: 1, den: 14 },
            orb_values,
            full_grid_award: Some(JackpotTier::Grand),
            jackpots,
        }),
        free_games: Some(FreeGamesModel {
            trigger_count: 3,
            initial_games: 6,
            retrigger_count: 3,
            retrigger_games: 6,
            free_reel_strips,
            mystery: Some(SymbolId(MEDALLION)),
            reveal_values,
        }),
    }
}

/// Identifier of [`happy_prosperous_synthetic`], written into trace headers.
pub const HAPPY_PROSPEROUS_SYNTHETIC_ID: &str = "happy-prosperous-synthetic-1";

/// A **synthetic** Happy & Prosperous configuration: the game's *evidenced
/// structure* with *invented* numbers. Happy & Prosperous runs on the same
/// engine family as Golden Century, so it shares this project's Dragon Link
/// simulation; only the symbols, the pay table and the free-games rule differ
/// (`docs/reverse-engineering/FOOTAGE.md`).
///
/// Official visible values used here: the US family pay table read from Happy
/// & Prosperous's own rules screens (top picture 150/50/10 and 2 for two;
/// second to fourth 100 down to 5; royals 75/50 down to 5), the scatter
/// 100/15/2 times the total bet, six free games with three more on a
/// retrigger, the royals removed inside the feature, and the twelve orb prize
/// multiples. Invented and never to be quoted as Dragon Link's: every reel
/// strip, weight, the land chance and the jackpot amounts. Unlike Autumn Moon
/// this title has no free-games mystery reveal; the feature simply plays
/// royal-free reels.
pub fn happy_prosperous_synthetic() -> MathModel {
    const NINE: u16 = 0;
    const TEN: u16 = 1;
    const JACK: u16 = 2;
    const QUEEN: u16 = 3;
    const KING: u16 = 4;
    const LOTUS: u16 = 5; // fourth picture
    const COINS: u16 = 6; // third picture
    const JADE: u16 = 7; // second picture (jade ring)
    const BELL: u16 = 8; // top picture
    const BUDDHA: u16 = 9; // wild (laughing Buddha), reels 2 to 5 only
    const POT: u16 = 10; // scatter (pot of gold)
    const ORB: u16 = 11; // Hold & Spin prize symbol

    let symbols = [
        ("9", SymbolKind::Regular),
        ("10", SymbolKind::Regular),
        ("J", SymbolKind::Regular),
        ("Q", SymbolKind::Regular),
        ("K", SymbolKind::Regular),
        ("lotus", SymbolKind::Regular),
        ("coins", SymbolKind::Regular),
        ("jade", SymbolKind::Regular),
        ("bell", SymbolKind::Regular),
        ("buddha", SymbolKind::Wild),
        ("pot", SymbolKind::Scatter),
        ("orb", SymbolKind::Orb),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (key, kind))| SymbolDef {
        id: SymbolId(i as u16),
        key: key.to_string(),
        kind,
    })
    .collect::<Vec<_>>();

    let reel = |parts: &[(u16, usize)]| -> Vec<SymbolId> {
        parts
            .iter()
            .flat_map(|(sym, n)| std::iter::repeat_n(*sym, *n))
            .map(SymbolId)
            .collect()
    };

    // The base reels: royals common, pictures rarer, the top picture (bell)
    // rarest; an orb block of three per reel, a wild block on reels 2 to 5, one
    // scatter. Synthetic.
    let reel_strips = vec![
        reel(&[
            (NINE, 7),
            (ORB, 3),
            (TEN, 6),
            (LOTUS, 2),
            (JACK, 6),
            (JADE, 1),
            (QUEEN, 4),
            (POT, 1),
            (KING, 4),
            (COINS, 2),
            (NINE, 6),
            (BELL, 1),
            (TEN, 6),
            (JACK, 6),
        ]),
        reel(&[
            (TEN, 6),
            (BUDDHA, 2),
            (NINE, 7),
            (ORB, 3),
            (JACK, 6),
            (COINS, 2),
            (QUEEN, 4),
            (POT, 1),
            (KING, 4),
            (JADE, 1),
            (LOTUS, 2),
            (TEN, 6),
            (BELL, 1),
            (NINE, 6),
            (JACK, 6),
        ]),
        reel(&[
            (JACK, 7),
            (ORB, 3),
            (NINE, 7),
            (BUDDHA, 2),
            (TEN, 6),
            (LOTUS, 2),
            (QUEEN, 4),
            (POT, 1),
            (COINS, 2),
            (KING, 4),
            (JADE, 1),
            (NINE, 6),
            (BELL, 1),
            (TEN, 6),
            (JACK, 6),
        ]),
        reel(&[
            (NINE, 7),
            (ORB, 3),
            (TEN, 6),
            (BUDDHA, 2),
            (JACK, 6),
            (JADE, 1),
            (QUEEN, 4),
            (COINS, 2),
            (POT, 1),
            (KING, 4),
            (LOTUS, 2),
            (NINE, 6),
            (BELL, 1),
            (TEN, 6),
            (JACK, 6),
        ]),
        reel(&[
            (TEN, 7),
            (ORB, 3),
            (NINE, 7),
            (BUDDHA, 2),
            (JACK, 6),
            (COINS, 2),
            (QUEEN, 4),
            (POT, 1),
            (KING, 4),
            (JADE, 1),
            (LOTUS, 2),
            (TEN, 6),
            (BELL, 1),
            (NINE, 6),
            (JACK, 6),
        ]),
    ];

    // Paylines 1 to 5 are the evidenced shapes; 6 to 50 synthetic.
    let known = [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [2, 2, 2, 2, 2],
        [0, 1, 2, 1, 0],
        [2, 1, 0, 1, 2],
    ];
    let mut paylines: Vec<Payline> = known.iter().map(|rows| Payline(rows.to_vec())).collect();
    let mut pattern = 0u32;
    while paylines.len() < 50 {
        let rows: Vec<u8> = (0..5)
            .map(|r| ((pattern / 3u32.pow(r)) % 3) as u8)
            .collect();
        let all_same = rows.iter().all(|r| *r == rows[0]);
        if !all_same && !known.iter().any(|k| k[..] == rows[..]) {
            paylines.push(Payline(rows));
        }
        pattern += 1;
    }

    // Line pays, per credit on the line. The US family pay table (official
    // visible values); only the top picture pays for two of a kind.
    let mut line_pays = Vec::new();
    for (symbol, pays) in [
        //              2   3   4    5
        (BELL, [2, 10, 50, 150]),
        (JADE, [0, 5, 30, 100]),
        (COINS, [0, 5, 25, 100]),
        (LOTUS, [0, 5, 25, 100]),
        (KING, [0, 5, 20, 75]),
        (QUEEN, [0, 5, 10, 50]),
        (JACK, [0, 5, 10, 50]),
        (TEN, [0, 5, 10, 50]),
        (NINE, [0, 5, 10, 50]),
    ] {
        for (count, pay) in (2u8..=5).zip(pays) {
            if pay > 0 {
                line_pays.push(LinePay {
                    symbol: SymbolId(symbol),
                    count,
                    pay,
                });
            }
        }
    }

    // Scatter: the family's 100/15/2 times the total bet (official visible).
    let scatter_pays = vec![
        ScatterPay {
            symbol: SymbolId(POT),
            count: 3,
            total_bet_multiple: 2,
        },
        ScatterPay {
            symbol: SymbolId(POT),
            count: 4,
            total_bet_multiple: 15,
        },
        ScatterPay {
            symbol: SymbolId(POT),
            count: 5,
            total_bet_multiple: 100,
        },
    ];

    // Orb values: the twelve official multiples and the four levels; synthetic
    // weights (as Golden Century and Autumn Moon).
    let orb_values = [
        (260, OrbValue::BetMultiple(1)),
        (220, OrbValue::BetMultiple(2)),
        (160, OrbValue::BetMultiple(3)),
        (120, OrbValue::BetMultiple(4)),
        (90, OrbValue::BetMultiple(5)),
        (70, OrbValue::BetMultiple(10)),
        (46, OrbValue::BetMultiple(15)),
        (28, OrbValue::BetMultiple(20)),
        (12, OrbValue::BetMultiple(50)),
        (6, OrbValue::BetMultiple(100)),
        (3, OrbValue::BetMultiple(200)),
        (2, OrbValue::BetMultiple(250)),
        (45, OrbValue::Jackpot(JackpotTier::Mini)),
        (14, OrbValue::Jackpot(JackpotTier::Minor)),
        (3, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();

    let jackpots = [
        (JackpotTier::Mini, 15),
        (JackpotTier::Minor, 60),
        (JackpotTier::Major, 300),
        (JackpotTier::Grand, 1000),
    ]
    .into_iter()
    .map(|(tier, total_bet_multiple)| JackpotDef {
        tier,
        total_bet_multiple,
    })
    .collect();

    // Free-game reels: the royals removed (official US rule), so only the four
    // pictures, the wild (reels 2 to 5) and the scatter appear; no orb and no
    // mystery reveal. Synthetic composition.
    let free_reel_strips = vec![
        reel(&[(LOTUS, 8), (COINS, 6), (JADE, 5), (BELL, 2), (POT, 1)]),
        reel(&[
            (LOTUS, 6),
            (BUDDHA, 3),
            (COINS, 6),
            (JADE, 5),
            (BELL, 2),
            (POT, 1),
        ]),
        reel(&[
            (COINS, 6),
            (BUDDHA, 3),
            (LOTUS, 6),
            (JADE, 5),
            (BELL, 2),
            (POT, 1),
        ]),
        reel(&[
            (LOTUS, 6),
            (BUDDHA, 3),
            (JADE, 5),
            (COINS, 6),
            (BELL, 2),
            (POT, 1),
        ]),
        reel(&[
            (JADE, 6),
            (BUDDHA, 3),
            (LOTUS, 6),
            (COINS, 5),
            (BELL, 2),
            (POT, 1),
        ]),
    ];

    MathModel {
        id: HAPPY_PROSPEROUS_SYNTHETIC_ID.to_string(),
        provenance: MathProvenance::SyntheticDevFixture,
        description: "SYNTHETIC Happy & Prosperous: the game's evidenced structure (the Golden \
                      Century engine family, with its own symbols, the US family pay table and \
                      royals removed inside the free games) with INVENTED strips, weights and \
                      jackpot amounts. Not recovered from any machine; never quote its numbers \
                      as Dragon Link's (docs/reverse-engineering/FOOTAGE.md, docs/DECISIONS.md \
                      D-017)."
            .to_string(),
        grid: GridShape { reels: 5, rows: 3 },
        symbols,
        reel_strips,
        paylines,
        line_pays,
        scatter_pays,
        bet: BetModel {
            credits_per_line_options: vec![1, 2, 3, 4, 5, 10],
            line_options: vec![5, 25, 50],
        },
        hold_and_spin: Some(HoldAndSpinModel {
            orb: SymbolId(ORB),
            trigger_count: 6,
            initial_respins: 3,
            reset_respins_on_new_orb: true,
            orb_land_chance: Ratio { num: 1, den: 14 },
            orb_values,
            full_grid_award: Some(JackpotTier::Grand),
            jackpots,
        }),
        free_games: Some(FreeGamesModel {
            trigger_count: 3,
            initial_games: 6,
            retrigger_count: 3,
            retrigger_games: 3,
            free_reel_strips,
            mystery: None,
            reveal_values: Vec::new(),
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
