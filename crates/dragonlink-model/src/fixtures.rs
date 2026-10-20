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
    BetModel, BigReel, FixedPrize, FreeGamesModel, HoldAndSpinModel, JackpotDef, JackpotTier,
    LinePay, MathModel, MathProvenance, OrbValue, Ratio, ScatterPay, WeightedOrbValue,
    WeightedSymbol,
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

/// Lay a strip out from its composition (symbol, count) in a fixed,
/// repeatable way. Symbols in `blocks` stay as the runs written, spread
/// evenly round the strip; every other symbol is dealt out one at a time,
/// the most numerous first, never beside itself and, where the counts allow,
/// never one apart from itself, so no three-row window shows it twice. The
/// strip has the same symbols in the same numbers as the composition.
fn spread(parts: &[(u16, usize)], blocks: &[u16]) -> Vec<SymbolId> {
    let runs: Vec<(u16, usize)> = parts
        .iter()
        .copied()
        .filter(|(s, _)| blocks.contains(s))
        .collect();
    let mut left: Vec<(u16, usize)> = Vec::new();
    for &(s, n) in parts.iter().filter(|(s, _)| !blocks.contains(s)) {
        match left.iter_mut().find(|(t, _)| *t == s) {
            Some((_, m)) => *m += n,
            None => left.push((s, n)),
        }
    }
    let total: usize = parts.iter().map(|(_, n)| n).sum();
    let mut out: Vec<u16> = Vec::with_capacity(total);
    let mut next_run = 0;
    while out.len() < total {
        // The runs fall at even spacing round the strip.
        let due = next_run < runs.len() && out.len() * runs.len() >= next_run * total;
        let singles_left = left.iter().any(|(_, n)| *n > 0);
        if (due || !singles_left)
            && let Some(&(s, n)) = runs.get(next_run)
        {
            out.extend(std::iter::repeat_n(s, n));
            next_run += 1;
            continue;
        }
        // The most numerous single that is clear of its own kind: three
        // stops apart if possible, else two, else anywhere.
        let clear = |s: u16, gap: usize| !out.iter().rev().take(gap).any(|&t| t == s);
        let pick = [2, 1, 0].iter().find_map(|&gap| {
            left.iter()
                .enumerate()
                .filter(|(_, (s, n))| *n > 0 && clear(*s, gap))
                .max_by_key(|(i, (_, n))| (*n, usize::MAX - i))
                .map(|(i, _)| i)
        });
        let Some(i) = pick else { break };
        out.push(left[i].0);
        left[i].1 -= 1;
    }
    strip(&out)
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
    .map(|(tier, total_bet_multiple)| JackpotDef::multiple(tier, total_bet_multiple))
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
pub const GOLDEN_CENTURY_SYNTHETIC_ID: &str = "golden-century-synthetic-3";

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
/// the scatter and orb value tables, the land chance, the jackpots' bet
/// multiples (which the exact analysis uses), the Major's restart value,
/// the stakes from which each jackpot has its full chance, and paylines 6
/// to 50. The Mini and Minor prizes by denomination are the club footage's
/// values (claims/family.toml, jackpots.minor-mini-au), the Major and Grand
/// are progressive (jackpots.meters-au), and the Grand restarts at $10,000
/// as filmed (claims/golden-century.toml, jackpots.grand-restart). Pays matching a reading of a real machine are marked in
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
    const EMPEROR: u16 = 9; // wild, reels 2 to 5 (help screen); reel 1 only through the doors
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
    // and pictures, with one scatter. The orbs, wilds and doors stay as the
    // runs written; every royal and picture is dealt out singly, apart from
    // its own kind, so a reel's window shows a mix of cards as the real game's
    // does (its columns show the same card twice only rarely), not two or
    // three of one card stacked. The counts, and so the return, are the same
    // either way: a payline sees each symbol as often wherever it stands.
    let reel = |parts: &[(u16, usize)]| spread(parts, &[ORB, EMPEROR, DOORS, PENNANT]);

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
            (EMPEROR, 2),
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
        // A run led by emperors pays the emperor's own line. Synthetic: the
        // values the mobile help screen shows, not cabinet evidence (D-017).
        // Only doors put an emperor on reel 1.
        (EMPEROR, [5, 25, 100, 500]),
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
        (220, OrbValue::BetMultiple(1)),
        (220, OrbValue::BetMultiple(2)),
        (160, OrbValue::BetMultiple(3)),
        (120, OrbValue::BetMultiple(4)),
        (90, OrbValue::BetMultiple(5)),
        (100, OrbValue::BetMultiple(10)),
        (70, OrbValue::BetMultiple(15)),
        (45, OrbValue::BetMultiple(20)),
        (20, OrbValue::BetMultiple(50)),
        (6, OrbValue::BetMultiple(100)),
        (3, OrbValue::BetMultiple(200)),
        (2, OrbValue::BetMultiple(250)),
        (100, OrbValue::Jackpot(JackpotTier::Mini)),
        (30, OrbValue::Jackpot(JackpotTier::Minor)),
        (10, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();

    // The bet multiples are synthetic: the exact analysis values every tier
    // by them, and a progressive pays one until its meter is set. In play
    // the Mini and Minor pay the footage's fixed prizes and the Major and
    // Grand their meters. Each jackpot's chance is the stake over its own
    // prize (its restart value for the Major and Grand), so its share of
    // the return is the same at every stake and denomination, as the
    // national standard requires a jackpot's chance be proportional to the
    // bet (synthetic; the real chances are unknown).
    let jackpots = [
        (JackpotTier::Mini, 15),
        (JackpotTier::Minor, 60),
        (JackpotTier::Major, 300),
        (JackpotTier::Grand, 1000),
    ]
    .into_iter()
    .map(|(tier, total_bet_multiple)| {
        let mut def = JackpotDef::multiple(tier, total_bet_multiple);
        def.full_chance_share = Some(Ratio { num: 1, den: 1 });
        // Australian club footage: the Mini is 1,000 credits and the Minor
        // 5,000 up to 10c, 250 and 1,250 from $1 ($10 / $50 at 1c, $250 /
        // $1,250 at $1); the Major and Grand are running meters. The Grand
        // restarts at $10,000, as filmed (a Golden Century machine read
        // $10,000.20 under WINNER after its Grand and climbed on from
        // $10,000.21: OL6Pu1AI-cQ 86-163 s, VERIFIED there; STRONG for
        // Australian Golden Century). The Major's $500 is synthetic
        // (HYPOTHESIS: the lowest filmed is $512.94).
        match tier {
            JackpotTier::Mini => {
                def.fixed_credits = vec![
                    FixedPrize {
                        from_unit: 1,
                        credits: 1_000,
                    },
                    FixedPrize {
                        from_unit: 100,
                        credits: 250,
                    },
                ];
            }
            JackpotTier::Minor => {
                def.fixed_credits = vec![
                    FixedPrize {
                        from_unit: 1,
                        credits: 5_000,
                    },
                    FixedPrize {
                        from_unit: 100,
                        credits: 1_250,
                    },
                ];
            }
            JackpotTier::Major => def.progressive_reset = Some(50_000),
            JackpotTier::Grand => def.progressive_reset = Some(1_000_000),
        }
        def
    })
    .collect();

    // Free-game reels: like the base reels with a mystery symbol (`doors`),
    // in long stacks so whole reels of doors are common (cabinet footage),
    // that reveals to one drawn symbol across all its positions, as Golden
    // Century's free games do. Coins land here too (public recording
    // y8c8lEdPW8Y, a Queensland club: 19 of 23 free-game stills show some,
    // and six started Hold & Spin inside the free games), in pairs. The scatter stays, so free games can
    // retrigger. Strips, weights and the reveal table are synthetic.
    let free_reel_strips = vec![
        reel(&[
            (NINE, 6),
            (DOORS, 9),
            (ORB, 2),
            (TEN, 5),
            (WARRIORS, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (PENNANT, 1),
            (DOORS, 6),
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
            (DOORS, 9),
            (ORB, 2),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (DOORS, 6),
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
            (DOORS, 9),
            (ORB, 2),
            (NINE, 6),
            (EMPEROR, 2),
            (TEN, 5),
            (WARRIORS, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (DOORS, 6),
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
            (DOORS, 9),
            (ORB, 2),
            (TEN, 5),
            (EMPEROR, 2),
            (JACK, 5),
            (CHEST, 1),
            (QUEEN, 4),
            (VASE, 2),
            (PENNANT, 1),
            (DOORS, 6),
            (KING, 4),
            (WARRIORS, 2),
            (NINE, 5),
            (WOMEN, 1),
            (TEN, 6),
            (JACK, 5),
        ]),
        reel(&[
            (TEN, 6),
            (DOORS, 9),
            (ORB, 2),
            (NINE, 6),
            (EMPEROR, 2),
            (WARRIORS, 2),
            (JACK, 5),
            (VASE, 2),
            (QUEEN, 4),
            (PENNANT, 1),
            (DOORS, 6),
            (KING, 4),
            (CHEST, 1),
            (TEN, 5),
            (WOMEN, 1),
            (NINE, 5),
            (JACK, 5),
        ]),
    ];
    // What the doors reveal to: the game's help screen lists the emperor,
    // the picture symbols, K to 9 and the coin. The weights are synthetic,
    // towards the lower symbols, with the emperor (wild) about one reveal in
    // five and the coin one in eight, near a tally of 60 filmed reveals
    // (emperor 12, coin 7; a small sample leaning to big wins, so not a
    // measurement); revealed coins each draw a prize, and six or more coins
    // start Hold & Spin.
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
        (30, EMPEROR),
        (20, ORB),
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
                      tiers, denominations and line selection) with INVENTED strips, weights, most \
                      pays, the jackpots' bet multiples, the Major's restart value and stake \
                      scaling; the Mini and Minor by denomination and the Grand's $10,000 restart \
                      follow club footage. Not recovered from any machine; never quote its numbers \
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
            // Every line count on the Australian club decks' ladders: 1/5/10/
            // 30/50 at 1c and 2c, 1/5/10/20/25 at 10c, 1-5 at $1 (cabinet
            // footage, docs/reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md).
            line_options: vec![1, 2, 3, 4, 5, 10, 20, 25, 30, 50],
        },
        hold_and_spin: Some(HoldAndSpinModel {
            orb: SymbolId(ORB),
            trigger_count: 6,
            initial_respins: 3,
            reset_respins_on_new_orb: true,
            orb_land_chance: Ratio { num: 1, den: 11 },
            orb_values,
            full_grid_award: Some(JackpotTier::Grand),
            jackpots,
        }),
        free_games: Some(FreeGamesModel {
            trigger_count: 3,  // three scatters (read)
            initial_games: 6,  // six free games (official)
            retrigger_count: 3,
            retrigger_games: 3, // the owner: three more free games
            free_reel_strips,
            mystery: Some(SymbolId(DOORS)),
            reveal_values,
            line_pays: Vec::new(),
            wild_multiplier: None,
            expanding_wilds: false,
            big_reel: None,
            no_hold_and_spin: false,
        }),
    }
}

/// Identifier of [`autumn_moon_synthetic`], written into trace headers.
pub const AUTUMN_MOON_SYNTHETIC_ID: &str = "autumn-moon-synthetic-2";

/// A **synthetic** Autumn Moon configuration. In the official mobile build
/// Autumn Moon is Golden Century with its own art: the same five lines, the
/// same free-games and Hold & Spin rules and byte-identical feature scenes,
/// the medallion in the doors' place, the moon in the flag's and the woman
/// in the emperor's (research notes, 2026-10-19). So this is
/// [`golden_century_synthetic`] in Autumn Moon's symbols, with Autumn Moon's
/// own pay table (the mobile help screens; four Australian club wins fit it
/// and none fits the US table: STRONG) and a six-game retrigger (Australian
/// club footage, three recordings: VERIFIED). Every strip, weight, chance and
/// jackpot amount is Golden Century's synthetic one and never to be quoted as
/// Dragon Link's.
pub fn autumn_moon_synthetic() -> MathModel {
    let mut model = golden_century_synthetic();
    for s in &mut model.symbols {
        let key = match s.key.as_str() {
            "warriors" => "rabbit",
            "women" => "mooncake",
            "vase" => "fan",
            "chest" => "lute",
            "emperor" => "woman",
            "pennant" => "moon",
            "doors" => "medallion",
            other => other,
        };
        s.key = key.to_string();
    }
    let id = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .and_then(|i| u16::try_from(i).ok())
            .map(SymbolId)
    };
    // Autumn Moon's pays for 2, 3, 4 and 5 of a kind (mobile help screens).
    // The woman pays only in the free games there; she lands on reel 1 only
    // when the medallions reveal her, so her line here pays only then.
    let pays = [
        ("rabbit", [3, 10, 50, 250]),
        ("mooncake", [2, 5, 30, 150]),
        ("fan", [2, 5, 25, 100]),
        ("lute", [2, 5, 25, 100]),
        ("K", [0, 5, 20, 75]),
        ("Q", [0, 5, 10, 50]),
        ("J", [0, 5, 10, 50]),
        ("10", [0, 5, 10, 50]),
        ("9", [0, 5, 10, 50]),
        ("woman", [5, 25, 100, 500]),
    ];
    let mut line_pays = Vec::new();
    for (key, row) in pays {
        let Some(symbol) = id(key) else { continue };
        for (count, pay) in (2u8..=5).zip(row) {
            if pay > 0 {
                line_pays.push(LinePay { symbol, count, pay });
            }
        }
    }
    // The moon pays 2, 10 and 100 times the total bet for 3, 4 and 5 (mobile).
    let scatter_pays = id("moon")
        .map(|symbol| {
            [(3u8, 2u32), (4, 10), (5, 100)]
                .into_iter()
                .map(|(count, total_bet_multiple)| ScatterPay {
                    symbol,
                    count,
                    total_bet_multiple,
                })
                .collect()
        })
        .unwrap_or_default();
    model.line_pays = line_pays;
    model.scatter_pays = scatter_pays;
    if let Some(fg) = model.free_games.as_mut() {
        fg.retrigger_games = 6;
    }
    model.id = AUTUMN_MOON_SYNTHETIC_ID.to_string();
    model.description = "SYNTHETIC Autumn Moon: Golden Century's synthetic model in Autumn Moon's                          symbols (the mobile build gives both the same rules and scenes), with                          Autumn Moon's own pay table and a six-game retrigger. Strips, weights and                          jackpots INVENTED; never quote its numbers as Dragon Link's."
        .to_string();
    model
}

/// The royal-free Dragon Link titles: in the official mobile build Happy &
/// Prosperous and Panda Magic are Golden Century's package with their own art,
/// less two features: no free-games mystery symbol and no tall stacked wild
/// (research notes, 2026-10-19). Their free games are six, with the royals
/// removed from the reels and three more on a retrigger (mobile help; club
/// footage). So each is [`golden_century_synthetic`] in the title's symbols,
/// its doors taken out, its free reels rebuilt without royals (synthetic), and
/// the mobile pay table, which matches Golden Century's mobile one rank for
/// rank; which table Australian club machines pay is not established (no club
/// rules page found), so that choice is ours.
fn golden_century_royal_free(
    id: &str,
    description: &str,
    names: [(&str, &str); 6],
    pays: [(&str, [u32; 4]); 9],
) -> MathModel {
    let mut model = golden_century_synthetic();
    let gc = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .and_then(|i| u16::try_from(i).ok())
            .unwrap_or(0)
    };
    let (p1, p2, p3, p4) = (gc("warriors"), gc("women"), gc("vase"), gc("chest"));
    let (wild, scatter, orb, doors) = (gc("emperor"), gc("pennant"), gc("orb"), gc("doors"));
    // Royal-free free reels: the four pictures, the wild in stacks on reels
    // 2 to 5, one scatter and a pair of coins on each. Synthetic.
    let free_reel_strips = (0..5u8)
        .map(|reel| {
            let mut parts = vec![(p1, 3), (p2, 4), (p3, 5), (p4, 6), (scatter, 1), (orb, 2)];
            if reel > 0 {
                parts.push((wild, 3));
            }
            spread(&parts, &[orb, wild, doors, scatter])
        })
        .collect();
    // The doors go: they are the last symbol and appear on no reel now.
    if usize::from(doors) + 1 == model.symbols.len() {
        model.symbols.pop();
    }
    if let Some(fg) = model.free_games.as_mut() {
        fg.free_reel_strips = free_reel_strips;
        fg.mystery = None;
        fg.reveal_values = Vec::new();
        fg.retrigger_games = 3;
    }
    for s in &mut model.symbols {
        if let Some((_, to)) = names.iter().find(|(from, _)| *from == s.key) {
            s.key = (*to).to_string();
        }
    }
    let title = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .and_then(|i| u16::try_from(i).ok())
            .map(SymbolId)
    };
    let mut line_pays = Vec::new();
    for (key, row) in pays {
        let Some(symbol) = title(key) else { continue };
        for (count, pay) in (2u8..=5).zip(row) {
            if pay > 0 {
                line_pays.push(LinePay { symbol, count, pay });
            }
        }
    }
    // The scatter pays 2, 10 and 100 times the total bet (mobile; three pay
    // twice the bet on Australian machines).
    let scatter_key = names[5].1;
    let scatter_pays = title(scatter_key)
        .map(|symbol| {
            [(3u8, 2u32), (4, 10), (5, 100)]
                .into_iter()
                .map(|(count, total_bet_multiple)| ScatterPay {
                    symbol,
                    count,
                    total_bet_multiple,
                })
                .collect()
        })
        .unwrap_or_default();
    model.line_pays = line_pays;
    model.scatter_pays = scatter_pays;
    model.id = id.to_string();
    model.description = description.to_string();
    model
}

/// The mobile pay table of the Golden Century family for 2, 3, 4 and 5 of a
/// kind, the pictures highest first (help screens); the wild pays nothing.
const FAMILY_PAYS: [[u32; 4]; 9] = [
    [3, 10, 50, 250],
    [2, 5, 30, 150],
    [2, 5, 25, 100],
    [2, 5, 25, 100],
    [0, 5, 20, 75],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
];

/// The family pays with the title's four pictures, highest first.
fn family_pays(pictures: [&str; 4]) -> [(&str, [u32; 4]); 9] {
    let keys = [
        pictures[0],
        pictures[1],
        pictures[2],
        pictures[3],
        "K",
        "Q",
        "J",
        "10",
        "9",
    ];
    let mut out = [("", [0; 4]); 9];
    for (i, key) in keys.into_iter().enumerate() {
        out[i] = (key, FAMILY_PAYS[i]);
    }
    out
}

/// Identifier of [`happy_prosperous_synthetic`], written into trace headers.
pub const HAPPY_PROSPEROUS_SYNTHETIC_ID: &str = "happy-prosperous-synthetic-2";

/// A **synthetic** Happy & Prosperous configuration (see
/// [`golden_century_royal_free`]): the bell, jade bangle, coins and lotus as
/// the pictures, the laughing Buddha as the wild (reels 2 to 5, one cell),
/// the pot of gold as the scatter.
pub fn happy_prosperous_synthetic() -> MathModel {
    golden_century_royal_free(
        HAPPY_PROSPEROUS_SYNTHETIC_ID,
        "SYNTHETIC Happy & Prosperous: Golden Century's synthetic model in Happy & Prosperous's \
         symbols, without the mystery reveal, with royal-free free games (three more on a \
         retrigger) and the family's mobile pay table. Strips, weights and jackpots INVENTED; \
         never quote its numbers as Dragon Link's.",
        [
            ("warriors", "bell"),
            ("women", "jade"),
            ("vase", "coins"),
            ("chest", "lotus"),
            ("emperor", "buddha"),
            ("pennant", "pot"),
        ],
        family_pays(["bell", "jade", "coins", "lotus"]),
    )
}

/// Identifier of [`panda_magic_synthetic`], written into trace headers.
pub const PANDA_MAGIC_SYNTHETIC_ID: &str = "panda-magic-synthetic-2";

/// A **synthetic** Panda Magic configuration (see
/// [`golden_century_royal_free`]): the tiger, rooster, junk and lotus as the
/// pictures, the panda as the wild (reels 2 to 5, one cell, in stacks), the
/// lantern as the scatter.
pub fn panda_magic_synthetic() -> MathModel {
    golden_century_royal_free(
        PANDA_MAGIC_SYNTHETIC_ID,
        "SYNTHETIC Panda Magic: Golden Century's synthetic model in Panda Magic's symbols, \
         without the mystery reveal, with royal-free free games (three more on a retrigger) and \
         the family's mobile pay table. Strips, weights and jackpots INVENTED; never quote its \
         numbers as Dragon Link's.",
        [
            ("warriors", "tiger"),
            ("women", "rooster"),
            ("vase", "junk"),
            ("chest", "lotus"),
            ("emperor", "panda"),
            ("pennant", "lantern"),
        ],
        family_pays(["tiger", "rooster", "junk", "lotus"]),
    )
}

/// The fifty win lines on Genghis Khan's and Golden Gong's help pages, read
/// from each and identical (rows 0 top to 2 bottom, reels 1 to 5). Their
/// first 25 are Peace & Long Life's and Spring Festival's 25 lines and their
/// first five Golden Century's five. VERIFIED (official mobile) for those
/// titles; that a cabinet playing more lines than its mobile build has uses
/// these is HYPOTHESIS.
const FAMILY_LINES: [[u8; 5]; 50] = [
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0],
    [2, 2, 2, 2, 2],
    [0, 1, 2, 1, 0],
    [2, 1, 0, 1, 2],
    [0, 0, 1, 0, 0],
    [2, 2, 1, 2, 2],
    [1, 2, 2, 2, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
    [2, 1, 1, 1, 2],
    [0, 1, 0, 1, 0],
    [2, 1, 2, 1, 2],
    [1, 0, 1, 0, 1],
    [1, 2, 1, 2, 1],
    [1, 1, 0, 1, 1],
    [1, 1, 2, 1, 1],
    [0, 2, 0, 2, 0],
    [2, 0, 2, 0, 2],
    [1, 0, 2, 0, 1],
    [1, 2, 0, 2, 1],
    [0, 0, 2, 0, 0],
    [2, 2, 0, 2, 2],
    [0, 2, 2, 2, 0],
    [2, 0, 0, 0, 2],
    [0, 2, 1, 2, 0],
    [2, 0, 1, 0, 2],
    [1, 1, 1, 1, 2],
    [0, 0, 1, 2, 2],
    [2, 2, 1, 0, 0],
    [0, 1, 1, 1, 2],
    [2, 1, 1, 1, 0],
    [0, 1, 2, 1, 2],
    [2, 1, 0, 1, 0],
    [0, 0, 0, 0, 1],
    [2, 2, 2, 2, 1],
    [0, 1, 0, 1, 2],
    [2, 1, 2, 1, 0],
    [1, 0, 1, 2, 1],
    [1, 2, 1, 0, 1],
    [1, 1, 0, 0, 0],
    [1, 1, 2, 2, 2],
    [1, 0, 0, 1, 2],
    [1, 2, 2, 1, 0],
    [1, 0, 1, 2, 2],
    [1, 2, 1, 0, 0],
    [2, 1, 0, 0, 1],
    [0, 1, 2, 2, 1],
    [0, 0, 1, 2, 1],
    [2, 2, 1, 0, 1],
];

/// The later titles' pay table for 2, 3, 4 and 5 of a kind, the pictures
/// highest first, then K, Q, J, 10 and 9: the same on Genghis Khan's, Golden
/// Gong's, Peace & Long Life's and Spring Festival's help pages (official
/// mobile, VERIFIED); only the top picture pays two of a kind and the wild
/// pays nothing of its own.
const LATER_PAYS: [[u32; 4]; 9] = [
    [2, 10, 50, 150],
    [0, 5, 30, 100],
    [0, 5, 25, 100],
    [0, 5, 25, 100],
    [0, 5, 20, 75],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
    [0, 5, 10, 50],
];

/// A symbol's id in `model` by its key.
fn id_of(model: &MathModel, key: &str) -> SymbolId {
    model
        .symbols
        .iter()
        .position(|s| s.key == key)
        .and_then(|i| u16::try_from(i).ok())
        .map(SymbolId)
        .unwrap_or(SymbolId(0))
}

/// Line pays for the title's pictures (highest first) and the royals, from
/// rows of 2, 3, 4 and 5 of a kind.
fn pays_for(model: &MathModel, pictures: [&str; 4], rows: &[[u32; 4]; 9]) -> Vec<LinePay> {
    let keys = [
        pictures[0],
        pictures[1],
        pictures[2],
        pictures[3],
        "K",
        "Q",
        "J",
        "10",
        "9",
    ];
    let mut out = Vec::new();
    for (key, row) in keys.into_iter().zip(rows) {
        let symbol = id_of(model, key);
        for (count, pay) in (2u8..=5).zip(row) {
            if *pay > 0 {
                out.push(LinePay {
                    symbol,
                    count,
                    pay: *pay,
                });
            }
        }
    }
    out
}

/// The base game the later Dragon Link titles share (Genghis Khan, Golden
/// Gong, Peace & Long Life, Spring Festival; research notes 2026-10-19, each
/// title's help pages): Golden Century's 5x3 window, wild on reels 2 to 5,
/// scattered trigger and Hold & Spin, with the family's fifty lines, a wild
/// that pays nothing itself, the later pay table and a scatter paying 2, 15
/// and 100 times the total bet. Their free games are six and none has a
/// mystery symbol; three of them say Hold & Spin starts in bought games only
/// (STRONG), so coins landing in their free games start none here. Built on
/// [`golden_century_synthetic`]'s strips, renamed (women, chest, vase and
/// warriors as the pictures highest first, emperor as the wild, the flag as
/// the scatter), the doors taken out; plain free reels (the base reels' make-
/// up with a pair of coins) for each title to give its own feature, and three
/// more games on three scatters (HYPOTHESIS: the help allows a retrigger
/// without a number; three is the family's count elsewhere and one
/// third-party lead's for Genghis Khan). Strips and weights synthetic.
fn golden_century_fifty(
    id: &str,
    description: &str,
    pictures: [&str; 4],
    wild: &str,
    scatter: &str,
) -> MathModel {
    let mut model = golden_century_synthetic();
    let gc = |key: &str| id_of(&model, key).0;
    let (nine, ten, jack, queen, king) = (gc("9"), gc("10"), gc("J"), gc("Q"), gc("K"));
    let (p1, p2, p3, p4) = (gc("women"), gc("chest"), gc("vase"), gc("warriors"));
    let (w, sc, orb, doors) = (gc("emperor"), gc("pennant"), gc("orb"), gc("doors"));
    let free_reel_strips = (0..5u8)
        .map(|reel| {
            let mut parts = vec![
                (nine, 6),
                (ten, 5),
                (jack, 5),
                (queen, 4),
                (king, 4),
                (p1, 1),
                (p2, 1),
                (p3, 2),
                (p4, 2),
                (sc, 1),
                (orb, 2),
            ];
            if reel > 0 {
                parts.push((w, 2));
            }
            spread(&parts, &[orb, w, sc])
        })
        .collect();
    if usize::from(doors) + 1 == model.symbols.len() {
        model.symbols.pop();
    }
    for (from, to) in [
        ("women", pictures[0]),
        ("chest", pictures[1]),
        ("vase", pictures[2]),
        ("warriors", pictures[3]),
        ("emperor", wild),
        ("pennant", scatter),
    ] {
        if let Some(s) = model.symbols.iter_mut().find(|s| s.key == from) {
            s.key = to.to_string();
        }
    }
    model.paylines = FAMILY_LINES
        .iter()
        .map(|rows| Payline(rows.to_vec()))
        .collect();
    model.line_pays = pays_for(&model, pictures, &LATER_PAYS);
    model.scatter_pays = [(3u8, 2u32), (4, 15), (5, 100)]
        .into_iter()
        .map(|(count, total_bet_multiple)| ScatterPay {
            symbol: SymbolId(sc),
            count,
            total_bet_multiple,
        })
        .collect();
    if let Some(fg) = model.free_games.as_mut() {
        fg.free_reel_strips = free_reel_strips;
        fg.mystery = None;
        fg.reveal_values = Vec::new();
        fg.retrigger_count = 3;
        fg.retrigger_games = 3;
        fg.no_hold_and_spin = true;
    }
    model.id = id.to_string();
    model.description = description.to_string();
    model
}

/// Give a title's free games a wild of their own, `key`, in place of the
/// base wild, each one doubling any win it is in (two make x4; the help's
/// "every x2 wild doubles", HYPOTHESIS for three or more).
fn with_x2_wild(mut model: MathModel, key: &str) -> MathModel {
    let base_wild = model
        .symbols
        .iter()
        .position(|s| s.kind == SymbolKind::Wild)
        .and_then(|i| u16::try_from(i).ok())
        .map(SymbolId);
    let x2 = SymbolId(model.symbols.len() as u16);
    model.symbols.push(SymbolDef {
        id: x2,
        key: key.to_string(),
        kind: SymbolKind::Wild,
    });
    if let Some(fg) = model.free_games.as_mut() {
        for strip in &mut fg.free_reel_strips {
            for s in strip.iter_mut() {
                if Some(*s) == base_wild {
                    *s = x2;
                }
            }
        }
        fg.wild_multiplier = Some(2);
    }
    model
}

/// Identifier of [`genghis_khan_synthetic`], written into trace headers.
pub const GENGHIS_KHAN_SYNTHETIC_ID: &str = "genghis-khan-synthetic-1";

/// A **synthetic** Genghis Khan configuration (see [`golden_century_fifty`]):
/// the woman, fortress, pot and shield as the pictures, Khan as the wild, the
/// archer as the scatter; in the free games Khan wears an x2 badge and every
/// one in a win doubles it (help page, VERIFIED; two making x4 seen on club
/// footage, STRONG). The royals stay in the free games (footage).
pub fn genghis_khan_synthetic() -> MathModel {
    with_x2_wild(
        golden_century_fifty(
            GENGHIS_KHAN_SYNTHETIC_ID,
            "SYNTHETIC Genghis Khan: the family's evidenced structure (50 lines, pay table, \
             scatter pays, six free games with an x2 wild, six-coin Hold & Spin, four jackpot \
             tiers) on Golden Century's synthetic strips. Strips, weights, the retrigger count \
             and jackpots INVENTED; never quote its numbers as Dragon Link's.",
            ["woman", "fortress", "pot", "shield"],
            "khan",
            "archer",
        ),
        "khan_x2",
    )
}

/// Identifier of [`peace_long_life_synthetic`], written into trace headers.
pub const PEACE_LONG_LIFE_SYNTHETIC_ID: &str = "peace-long-life-synthetic-2";

/// A **synthetic** Peace & Long Life configuration (see
/// [`golden_century_fifty`]): the tortoise, deer, birdcage and bamboo as the
/// pictures, the sage as the wild, the peach as the scatter; Genghis Khan's
/// sibling in the mobile build, with the same x2 wild in the free games (help
/// page, VERIFIED). Whether its free reels keep the royals is unknown; they
/// do here.
pub fn peace_long_life_synthetic() -> MathModel {
    let mut model = with_x2_wild(
        golden_century_fifty(
            PEACE_LONG_LIFE_SYNTHETIC_ID,
            "SYNTHETIC Peace & Long Life: the family's evidenced structure (25 lines on mobile, \
             up to 50 on club machines, pay table, scatter pays, six free games with an x2 wild \
             and a six-game retrigger, six-coin Hold & Spin, four jackpot tiers) on Golden \
             Century's synthetic strips. Strips, weights and jackpots INVENTED; never quote its \
             numbers as Dragon Link's.",
            ["tortoise", "deer", "birdcage", "bamboo"],
            "sage",
            "peach",
        ),
        "sage_x2",
    );
    // A retrigger adds six games: a club machine's banner counts "1 OF 6" up
    // to "1 OF 12" (jM1NDlBHZCs 1:01-1:03, the only retrigger in 18 filmed
    // features; VERIFIED as displayed, one case).
    if let Some(fg) = model.free_games.as_mut() {
        fg.retrigger_games = 6;
    }
    model
}

/// Identifier of [`golden_gong_synthetic`], written into trace headers.
pub const GOLDEN_GONG_SYNTHETIC_ID: &str = "golden-gong-synthetic-1";

/// A **synthetic** Golden Gong configuration (see [`golden_century_fifty`]):
/// the four disciples, rooster, scroll and bamboo as the pictures, the monk
/// as the wild, the gong as the scatter. In the free games a monk fills its
/// whole reel when it lands, and the free games pay their own table, the
/// pictures more and the royals less (help pages, VERIFIED; the expansion on
/// club footage, STRONG).
pub fn golden_gong_synthetic() -> MathModel {
    let mut model = golden_century_fifty(
        GOLDEN_GONG_SYNTHETIC_ID,
        "SYNTHETIC Golden Gong: the family's evidenced structure (50 lines, pay tables, scatter \
         pays, six free games with an expanding wild and their own pay table, six-coin Hold & \
         Spin, four jackpot tiers) on Golden Century's synthetic strips. Strips, weights, the \
         retrigger count and jackpots INVENTED; never quote its numbers as Dragon Link's.",
        ["disciples", "rooster", "scroll", "bamboo"],
        "monk",
        "gong",
    );
    let pictures = ["disciples", "rooster", "scroll", "bamboo"];
    let free_pays = [
        [2, 10, 50, 250],
        [0, 5, 30, 150],
        [0, 5, 25, 150],
        [0, 5, 25, 150],
        [0, 2, 10, 50],
        [0, 2, 5, 25],
        [0, 2, 5, 25],
        [0, 2, 5, 25],
        [0, 2, 5, 25],
    ];
    let line_pays = pays_for(&model, pictures, &free_pays);
    if let Some(fg) = model.free_games.as_mut() {
        fg.expanding_wilds = true;
        fg.line_pays = line_pays;
    }
    model
}

/// Identifier of [`spring_festival_synthetic`], written into trace headers.
pub const SPRING_FESTIVAL_SYNTHETIC_ID: &str = "spring-festival-synthetic-1";

/// A **synthetic** Spring Festival configuration (see
/// [`golden_century_fifty`]): the drum, lantern, envelopes and firecrackers
/// as the pictures, the God of Wealth as the wild, the gold ingot as the
/// scatter. In the free games reels 2 to 4 spin as one BIG reel whose every
/// stop is a 3x3 symbol: a BIG ingot pays the total bet and adds three games,
/// a BIG coin starts Hold & Spin with its nine positions held, and the free
/// games pay their own table, four and five of a kind only (help pages,
/// VERIFIED). The ingot and coin come only BIG there; the wild also lands on
/// reel 5, never reel 1 (help, VERIFIED). The BIG coin's prizes are the help
/// page's list; their weights, and the BIG reel, are synthetic.
pub fn spring_festival_synthetic() -> MathModel {
    let pictures = ["drum", "lantern", "envelopes", "firecrackers"];
    let mut model = golden_century_fifty(
        SPRING_FESTIVAL_SYNTHETIC_ID,
        "SYNTHETIC Spring Festival: the family's evidenced structure (25 lines on mobile, up to \
         50 on club machines, pay tables, scatter pays, six free games on a BIG 3x3 reel with \
         BIG ingot retriggers and BIG coin Hold & Spin, six-coin Hold & Spin, four jackpot \
         tiers) on Golden Century's synthetic strips. Strips, weights, the BIG reel and jackpots \
         INVENTED; never quote its numbers as Dragon Link's.",
        pictures,
        "fortune_god",
        "ingot",
    );
    let at = |key: &str| id_of(&model, key).0;
    let (nine, ten, jack, queen, king) = (at("9"), at("10"), at("J"), at("Q"), at("K"));
    let (p1, p2, p3, p4) = (
        at(pictures[0]),
        at(pictures[1]),
        at(pictures[2]),
        at(pictures[3]),
    );
    let (wild, ingot, orb) = (at("fortune_god"), at("ingot"), at("orb"));
    let cards = [
        (nine, 6),
        (ten, 5),
        (jack, 5),
        (queen, 4),
        (king, 4),
        (p1, 1),
        (p2, 1),
        (p3, 2),
        (p4, 2),
    ];
    // Reel 1 has no wild, ingot or coin in the free games; reel 5 has the
    // wild; reels 2 to 4 are the BIG reel's (their own strips go unused).
    let mut with_wild = cards.to_vec();
    with_wild.push((wild, 2));
    let free_reel_strips = vec![
        spread(&cards, &[]),
        spread(&with_wild, &[wild]),
        spread(&with_wild, &[wild]),
        spread(&with_wild, &[wild]),
        spread(&with_wild, &[wild]),
    ];
    let mut big = cards.to_vec();
    big.extend([(wild, 2), (ingot, 1), (orb, 1)]);
    let big_strip = spread(&big, &[]);
    let free_pays = [
        [0, 0, 10, 100],
        [0, 0, 10, 50],
        [0, 0, 10, 25],
        [0, 0, 10, 25],
        [0, 0, 5, 20],
        [0, 0, 5, 15],
        [0, 0, 5, 15],
        [0, 0, 5, 15],
        [0, 0, 5, 15],
    ];
    let line_pays = pays_for(&model, pictures, &free_pays);
    let big_orb_values = [
        (200, OrbValue::BetMultiple(5)),
        (160, OrbValue::BetMultiple(10)),
        (120, OrbValue::BetMultiple(15)),
        (100, OrbValue::BetMultiple(20)),
        (80, OrbValue::BetMultiple(25)),
        (60, OrbValue::BetMultiple(30)),
        (40, OrbValue::BetMultiple(40)),
        (30, OrbValue::BetMultiple(50)),
        (20, OrbValue::BetMultiple(60)),
        (12, OrbValue::BetMultiple(80)),
        (8, OrbValue::BetMultiple(100)),
        (3, OrbValue::BetMultiple(200)),
        (2, OrbValue::BetMultiple(250)),
        (60, OrbValue::Jackpot(JackpotTier::Mini)),
        (20, OrbValue::Jackpot(JackpotTier::Minor)),
        (6, OrbValue::Jackpot(JackpotTier::Major)),
    ]
    .into_iter()
    .map(|(weight, value)| WeightedOrbValue { weight, value })
    .collect();
    if let Some(fg) = model.free_games.as_mut() {
        fg.free_reel_strips = free_reel_strips;
        fg.line_pays = line_pays;
        fg.retrigger_count = 0;
        fg.retrigger_games = 0;
        fg.big_reel = Some(BigReel {
            first: 1,
            width: 3,
            strip: big_strip,
            scatter_pay: 1,
            scatter_games: 3,
            orb_values: big_orb_values,
        });
    }
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two scatters never share a reel's window: on every strip, base and
    /// free, of every title, scatters stand at least a window's height apart
    /// (the owner: two flags in one column cannot happen on the machine).
    #[test]
    fn no_window_shows_two_scatters_on_one_reel() {
        for model in [
            synthetic_dev_fixture(),
            golden_century_synthetic(),
            autumn_moon_synthetic(),
            happy_prosperous_synthetic(),
            panda_magic_synthetic(),
            genghis_khan_synthetic(),
            golden_gong_synthetic(),
            peace_long_life_synthetic(),
            spring_festival_synthetic(),
        ] {
            let rows = model.grid.rows as usize;
            let free = model
                .free_games
                .as_ref()
                .map(|fg| fg.free_reel_strips.clone())
                .unwrap_or_default();
            for strip in model.reel_strips.iter().chain(free.iter()) {
                let at: Vec<usize> = strip
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| model.kind(**s) == Some(SymbolKind::Scatter))
                    .map(|(i, _)| i)
                    .collect();
                for (n, &a) in at.iter().enumerate() {
                    let b = at[(n + 1) % at.len()];
                    let gap = (b + strip.len() - a) % strip.len();
                    assert!(
                        at.len() == 1 || gap >= rows,
                        "{}: scatters {a} and {b} share a window",
                        model.id
                    );
                }
            }
        }
    }

    /// The later titles: valid, labelled synthetic, on the family's fifty
    /// lines and pay table, no mystery symbol, no Hold & Spin from free-game
    /// coins, and each its own free games.
    #[test]
    fn the_later_titles_have_their_own_free_games() {
        for model in [
            genghis_khan_synthetic(),
            golden_gong_synthetic(),
            peace_long_life_synthetic(),
            spring_festival_synthetic(),
        ] {
            assert_eq!(model.validate(), Ok(()), "{}", model.id);
            assert_eq!(model.provenance, MathProvenance::SyntheticDevFixture);
            assert!(model.description.contains("SYNTHETIC"));
            assert_eq!(model.paylines.len(), 50);
            assert!(
                model
                    .paylines
                    .iter()
                    .zip(FAMILY_LINES)
                    .all(|(p, l)| p.0 == l)
            );
            let fg = model.free_games.as_ref().unwrap();
            assert_eq!(fg.initial_games, 6);
            assert!(fg.mystery.is_none() && fg.no_hold_and_spin);
            // The wild pays nothing of its own.
            assert!(
                model
                    .line_pays
                    .iter()
                    .all(|p| model.kind(p.symbol) == Some(SymbolKind::Regular))
            );
        }
        // The x2 wilds only in the free games, never on reel 1.
        for (model, x2) in [
            (genghis_khan_synthetic(), "khan_x2"),
            (peace_long_life_synthetic(), "sage_x2"),
        ] {
            let x2 = id_of(&model, x2);
            let fg = model.free_games.as_ref().unwrap();
            assert_eq!(fg.wild_multiplier, Some(2));
            assert!(model.reel_strips.iter().flatten().all(|s| *s != x2));
            assert!(!fg.free_reel_strips[0].contains(&x2));
            assert!(fg.free_reel_strips[1..].iter().all(|r| r.contains(&x2)));
        }
        let gg = golden_gong_synthetic();
        let fg = gg.free_games.as_ref().unwrap();
        assert!(fg.expanding_wilds && !fg.line_pays.is_empty());
        // Spring Festival: reels 2 to 4 one BIG reel, the ingot and coin
        // only on it in the free games, the wild on it and reel 5.
        let sf = spring_festival_synthetic();
        let fg = sf.free_games.as_ref().unwrap();
        let big = fg.big_reel.as_ref().unwrap();
        assert_eq!((big.first, big.width), (1, 3));
        let (ingot, orb, wild) = (
            id_of(&sf, "ingot"),
            id_of(&sf, "orb"),
            id_of(&sf, "fortune_god"),
        );
        assert!(
            big.strip.contains(&ingot) && big.strip.contains(&orb) && big.strip.contains(&wild)
        );
        for (reel, strip) in fg.free_reel_strips.iter().enumerate() {
            assert!(!strip.contains(&ingot) && !strip.contains(&orb));
            assert_eq!(strip.contains(&wild), reel > 0, "reel {reel}");
        }
        assert!(fg.line_pays.iter().all(|p| p.count >= 4));
    }

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

    /// Golden Century's strips deal the royals and pictures out: a three-row
    /// window shows a royal or picture twice only rarely, never beside itself;
    /// the orb, wild and door runs are kept; the symbol counts are those of
    /// the composition.
    #[test]
    fn golden_century_columns_mix_their_cards() {
        let m = golden_century_synthetic();
        assert_eq!(m.validate(), Ok(()));
        let key = |s: SymbolId| m.symbols[s.0 as usize].key.as_str();
        let dealt = |k: &str| {
            [
                "9", "10", "J", "Q", "K", "warriors", "vase", "chest", "women",
            ]
            .contains(&k)
        };
        let fg = m.free_games.as_ref().expect("free games");
        let (mut windows, mut doubled) = (0, 0);
        for strip in m.reel_strips.iter().chain(&fg.free_reel_strips) {
            let n = strip.len();
            for i in 0..n {
                let w = [strip[i], strip[(i + 1) % n], strip[(i + 2) % n]];
                // Never beside itself.
                assert!(
                    !(dealt(key(w[0])) && w[0] == w[1]),
                    "{} beside itself",
                    key(w[0])
                );
                windows += 1;
                if (dealt(key(w[0])) && w[0] == w[2]) || (dealt(key(w[1])) && w[1] == w[2]) {
                    doubled += 1;
                }
            }
        }
        assert!(
            doubled * 20 < windows,
            "{doubled} of {windows} windows show a card twice"
        );
        // The orbs still come three together, and the counts are unchanged.
        let orb = m.symbols.iter().position(|s| s.key == "orb").unwrap();
        let orbs: usize = m
            .reel_strips
            .iter()
            .map(|s| s.iter().filter(|x| x.0 as usize == orb).count())
            .sum();
        assert_eq!(orbs, 15);
        // 261 stops, and the emperor block on reel 5 (help screen).
        assert_eq!(m.reel_strips.iter().map(Vec::len).sum::<usize>(), 263);
    }
}
