//! The synthetic Golden Century configuration runs and has a sane shape.
//!
//! This checks only that the invented numbers produce a *playable* game: it
//! validates, pays on lines, reaches both the scatter and the Hold & Spin
//! feature, and returns somewhere between a black hole and free money. It
//! makes no claim about Dragon Link — every figure here is a property of a
//! synthetic fixture, not of any original game.

use dragonlink_core::{Command, DevRng, Game};
use dragonlink_math::sim::{simulate, simulate_game};
use dragonlink_model::JackpotTier;
use dragonlink_model::fixtures::golden_century_synthetic;

#[test]
fn the_synthetic_golden_century_is_playable() {
    let model = golden_century_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    assert_eq!(
        model.bet.line_options,
        vec![1, 2, 3, 4, 5, 10, 20, 25, 30, 50]
    );
    assert_eq!(model.symbols.len(), 13);

    let report = simulate(&model, 1, 400_000).expect("simulate");
    eprintln!(
        "synthetic GC: RTP {:.1}% (line {:.1}, scatter {:.1}, feature {:.1}); hit {:.1}%; feature about 1 in {:.0}",
        report.rtp.estimate * 100.0,
        report.line_rtp * 100.0,
        report.scatter_rtp * 100.0,
        report.feature_rtp * 100.0,
        report.hit_frequency.estimate * 100.0,
        1.0 / report.feature_frequency.estimate.max(1e-9),
    );

    // A playable return, not free money and not a black hole. The exact value
    // is synthetic and not a target.
    assert!(
        (0.5..1.5).contains(&report.rtp.estimate),
        "synthetic RTP {:.3} is outside a playable range",
        report.rtp.estimate
    );
    // Lines, scatters and the feature are all reached.
    assert!(report.line_rtp > 0.0, "lines never paid");
    assert!(report.scatter_rtp > 0.0, "scatters never paid");
    assert!(
        report.feature_frequency.estimate > 0.0,
        "Hold & Spin never triggered"
    );
    // The feature is neither so common it dominates nor so rare it never shows.
    assert!(
        (1.0 / 1000.0..1.0 / 40.0).contains(&report.feature_frequency.estimate),
        "feature frequency {:.5} is outside a sensible range",
        report.feature_frequency.estimate
    );
    assert!(report.hit_frequency.estimate > 0.05 && report.hit_frequency.estimate < 0.95);
}

/// The configurations a flat return is measured over: (label, credit size
/// in cents, bet option index, lines).
/// Every one is a bet the club or casino deck allows (cabinet::line_keys,
/// lines_for and the bet keys).
const STAKES: [(&str, u32, usize, u16); 8] = [
    ("1c 1x1", 1, 0, 1),
    ("1c 1x50", 1, 0, 50),
    ("1c 10x50", 1, 5, 50),
    ("2c 2x50", 2, 1, 50),
    ("10c 1x25", 10, 0, 25),
    ("10c 10x25", 10, 5, 25),
    ("$1 1x1", 100, 0, 1),
    ("$1 10x5", 100, 5, 5),
];

/// A title's slug and a constructor for a fresh model of it.
type Title = (&'static str, fn() -> dragonlink_model::MathModel);

/// Every title the app plays, by its slug.
fn all_titles() -> [Title; 8] {
    use dragonlink_model::fixtures::{
        autumn_moon_synthetic, genghis_khan_synthetic, golden_gong_synthetic,
        happy_prosperous_synthetic, panda_magic_synthetic, peace_long_life_synthetic,
        spring_festival_synthetic,
    };
    [
        ("golden-century", golden_century_synthetic),
        ("autumn-moon", autumn_moon_synthetic),
        ("happy-prosperous", happy_prosperous_synthetic),
        ("panda-magic", panda_magic_synthetic),
        ("genghis-khan", genghis_khan_synthetic),
        ("golden-gong", golden_gong_synthetic),
        ("peace-long-life", peace_long_life_synthetic),
        ("spring-festival", spring_festival_synthetic),
    ]
}

/// The played return of each configuration over `spins`, the meters held at
/// `meters` (Major, Grand), as a list of (label, return).
#[cfg(test)]
fn returns_at(
    model: fn() -> dragonlink_model::MathModel,
    spins: u64,
    meters: (u64, u64),
    seed: u64,
) -> Vec<(&'static str, f64)> {
    STAKES
        .iter()
        .map(|&(label, unit, index, lines)| {
            let mut game = Game::new(model(), u64::MAX / 4).unwrap();
            let mut rng = DevRng::new(0);
            if unit != 1 {
                game.apply(Command::ChangeDenomination { from: 1, to: unit }, &mut rng)
                    .unwrap();
            }
            game.apply(Command::SetStake { index, lines }, &mut rng)
                .unwrap();
            let held = [
                (JackpotTier::Major, meters.0),
                (JackpotTier::Grand, meters.1),
            ];
            let report = simulate_game(game, &held, seed, spins).unwrap();
            eprintln!(
                "{label}: RTP {:.2}% (feature {:.1}); jackpots {:?}",
                report.rtp.estimate * 100.0,
                report.feature_rtp * 100.0,
                report.jackpot_awards
            );
            (label, report.rtp.estimate)
        })
        .collect()
}

/// The return the app plays is the same at every bet the decks allow, as a
/// real machine's is (the standard allows 0.20 points between bets): each
/// jackpot's chance follows the stake over its prize. With the meters at
/// their restarts and near the top, every configuration returns about 90%
/// (synthetic, not Dragon Link's) and they agree within the run's noise.
#[test]
fn the_return_is_flat_across_stakes_and_denominations() {
    for (title, meters) in all_titles()
        .iter()
        .flat_map(|(_, t)| [(50_000, 500_000), (120_000, 1_200_000)].map(|m| (*t, m)))
    {
        let r = returns_at(title, 150_000, meters, 7);
        for (label, x) in &r {
            assert!((0.80..1.0).contains(x), "{label}: RTP {x:.3}");
        }
        let (lo, hi) = r.iter().fold((f64::MAX, f64::MIN), |(lo, hi), (_, x)| {
            (lo.min(*x), hi.max(*x))
        });
        assert!(hi - lo < 0.08, "spread {:.3}", hi - lo);
    }
}

/// A long measurement of the return's flatness, run by hand:
/// `cargo test --release -p dragonlink-math --test gc_synthetic_plays
/// measure_the_return_across_stakes -- --ignored --nocapture`.
#[test]
#[ignore = "a long measurement, run by hand"]
fn measure_the_return_across_stakes() {
    for meters in [(50_000, 500_000), (120_000, 1_200_000)] {
        eprintln!("meters {meters:?}");
        let spins = std::env::var("GC_RTP_SPINS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(2_000_000);
        // RTP_TITLE picks another title by its slug.
        let title = std::env::var("RTP_TITLE").unwrap_or_default();
        let model = all_titles()
            .into_iter()
            .find(|(slug, _)| *slug == title)
            .map_or(golden_century_synthetic as fn() -> _, |(_, m)| m);
        let r = returns_at(model, spins, meters, 11);
        let (lo, hi) = r.iter().fold((f64::MAX, f64::MIN), |(lo, hi), (_, x)| {
            (lo.min(*x), hi.max(*x))
        });
        eprintln!("spread {:.2} points", (hi - lo) * 100.0);
    }
}

#[test]
fn the_free_games_feature_triggers_plays_and_returns_cleanly() {
    use dragonlink_core::{DevRng, Event, Game};
    let model = golden_century_synthetic();
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(4);
    let mut free_rounds = 0usize;
    let mut reveals = 0usize;
    let mut retriggers = 0usize;
    let (mut wild_reveals, mut coin_reveals, mut nested) = (0usize, 0usize, 0usize);
    for _ in 0..60_000 {
        let events = game.play_spin(&mut rng).unwrap();
        // A Hold & Spin a free game's coins start sits inside the free
        // games: it ends before they do, and they carry on or end with it.
        let fg_start = events
            .iter()
            .position(|e| matches!(e, Event::FreeGamesStarted { .. }));
        for (i, e) in events.iter().enumerate() {
            if let (Event::HoldAndSpinStarted { .. }, Some(start)) = (e, fg_start)
                && i > start
            {
                nested += 1;
                let end = events[i..]
                    .iter()
                    .position(|e| matches!(e, Event::HoldAndSpinCompleted { .. }))
                    .map(|j| i + j)
                    .expect("the Hold & Spin ends");
                assert!(
                    matches!(
                        events.get(end + 1..end + 3),
                        Some([Event::CreditsChanged { .. }, Event::FreeGamesResumed { .. }])
                            | Some([
                                Event::CreditsChanged { .. },
                                Event::FreeGamesCompleted { .. }
                            ])
                            | Some([Event::FreeGamesResumed { .. }, _])
                            | Some([Event::FreeGamesCompleted { .. }, _])
                    ),
                    "the free games carry on or end after it: {:?}",
                    events.get(end..end + 3)
                );
            }
        }
        for e in &events {
            match e {
                Event::FreeGamesStarted { games } => {
                    assert_eq!(*games, 6, "six free games");
                    free_rounds += 1;
                }
                Event::MysteryRevealed { count, symbol } => {
                    assert!(*count >= 1);
                    reveals += 1;
                    wild_reveals += usize::from(symbol == "emperor");
                    coin_reveals += usize::from(symbol == "orb");
                }
                Event::FreeGamesRetriggered { added, .. } => {
                    assert_eq!(*added, 3, "three more free games");
                    retriggers += 1;
                }
                _ => {}
            }
        }
        // play_spin plays features to the end, so we are always back to idle.
        assert!(game.is_idle(), "did not return to the base game");
    }
    eprintln!(
        "free games: {free_rounds} rounds, {reveals} reveals ({wild_reveals} wild, {coin_reveals} coins), \
         {retriggers} retriggers, {nested} Hold & Spin inside in 60k spins"
    );
    assert!(free_rounds > 0, "free games never triggered");
    assert!(reveals > 0, "the reveal never happened");
    // The doors reveal the wild and the coin (the game's help screen), and
    // revealed coins start Hold & Spin (the manufacturer's page).
    assert!(wild_reveals > 0, "the doors never revealed the wild");
    assert!(coin_reveals > 0, "the doors never revealed coins");
    assert!(nested > 0, "revealed coins never started Hold & Spin");
}

#[test]
fn the_synthetic_autumn_moon_is_playable() {
    use dragonlink_core::{DevRng, Event, Game};
    use dragonlink_model::fixtures::autumn_moon_synthetic;
    let model = autumn_moon_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    assert_eq!(model.symbols.len(), 13);

    let report = simulate(&model, 1, 400_000).expect("simulate");
    eprintln!(
        "synthetic Autumn Moon: RTP {:.1}% (line {:.1}, scatter {:.1}, feature {:.1}); hit {:.1}%; feature about 1 in {:.0}",
        report.rtp.estimate * 100.0,
        report.line_rtp * 100.0,
        report.scatter_rtp * 100.0,
        report.feature_rtp * 100.0,
        report.hit_frequency.estimate * 100.0,
        1.0 / report.feature_frequency.estimate.max(1e-9),
    );
    assert!(
        (0.5..1.5).contains(&report.rtp.estimate),
        "RTP {:.3}",
        report.rtp.estimate
    );
    assert!(report.line_rtp > 0.0 && report.scatter_rtp > 0.0);
    assert!(report.feature_frequency.estimate > 0.0);

    // The feature set plays and returns cleanly.
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(4);
    let (mut free, mut reveal) = (0, 0);
    for _ in 0..60_000 {
        for e in &game.play_spin(&mut rng).unwrap() {
            match e {
                Event::FreeGamesStarted { .. } => free += 1,
                Event::MysteryRevealed { .. } => reveal += 1,
                _ => {}
            }
        }
        assert!(game.is_idle());
    }
    assert!(
        free > 0 && reveal > 0,
        "free games {free}, reveals {reveal}"
    );
}

#[test]
fn the_synthetic_happy_prosperous_is_playable() {
    use dragonlink_core::{DevRng, Event, Game};
    use dragonlink_model::fixtures::happy_prosperous_synthetic;
    let model = happy_prosperous_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    // No mystery symbol in this title: twelve symbols, not thirteen.
    assert_eq!(model.symbols.len(), 12);
    assert!(
        model
            .free_games
            .as_ref()
            .expect("free games")
            .mystery
            .is_none(),
        "Happy & Prosperous has no free-games mystery reveal"
    );

    let report = simulate(&model, 1, 400_000).expect("simulate");
    eprintln!(
        "synthetic Happy & Prosperous: RTP {:.1}% (line {:.1}, scatter {:.1}, feature {:.1}); hit {:.1}%; feature about 1 in {:.0}",
        report.rtp.estimate * 100.0,
        report.line_rtp * 100.0,
        report.scatter_rtp * 100.0,
        report.feature_rtp * 100.0,
        report.hit_frequency.estimate * 100.0,
        1.0 / report.feature_frequency.estimate.max(1e-9),
    );
    assert!(
        (0.5..1.5).contains(&report.rtp.estimate),
        "RTP {:.3}",
        report.rtp.estimate
    );
    assert!(report.line_rtp > 0.0 && report.scatter_rtp > 0.0);
    assert!(report.feature_frequency.estimate > 0.0);

    // The feature set plays and returns cleanly; the royals never appear inside
    // the free games (the one rule that sets this title apart), and no reveal
    // ever fires because this title has no mystery symbol.
    let royals: Vec<String> = ["9", "10", "J", "Q", "K"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(4);
    let (mut free, mut retrig, mut reveal) = (0, 0, 0);
    let mut in_free = false;
    for _ in 0..60_000 {
        for e in &game.play_spin(&mut rng).unwrap() {
            match e {
                Event::FreeGamesStarted { games } => {
                    assert_eq!(*games, 6, "six free games");
                    free += 1;
                    in_free = true;
                }
                Event::FreeGamesRetriggered { added, .. } => {
                    assert_eq!(*added, 3, "three extra games on a retrigger");
                    retrig += 1;
                }
                Event::FreeGamesCompleted { .. } => in_free = false,
                Event::MysteryRevealed { .. } => reveal += 1,
                Event::ReelStopped { symbols, .. } if in_free => {
                    for key in symbols {
                        assert!(
                            !royals.contains(key),
                            "royal {key} appeared inside the free games"
                        );
                    }
                }
                _ => {}
            }
        }
        assert!(game.is_idle());
    }
    eprintln!("Happy & Prosperous: {free} free rounds, {retrig} retriggers in 60k spins");
    assert!(free > 0, "free games never triggered");
    assert_eq!(reveal, 0, "there is no mystery reveal in this title");
}

#[test]
fn the_synthetic_panda_magic_is_playable() {
    use dragonlink_core::{DevRng, Event, Game};
    use dragonlink_model::fixtures::panda_magic_synthetic;
    let model = panda_magic_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    // Same family as Happy & Prosperous: twelve symbols, no mystery reveal.
    assert_eq!(model.symbols.len(), 12);
    assert!(
        model
            .free_games
            .as_ref()
            .expect("free games")
            .mystery
            .is_none(),
        "Panda Magic has no free-games mystery reveal"
    );

    let report = simulate(&model, 1, 400_000).expect("simulate");
    eprintln!(
        "synthetic Panda Magic: RTP {:.1}% (line {:.1}, scatter {:.1}, feature {:.1}); hit {:.1}%; feature about 1 in {:.0}",
        report.rtp.estimate * 100.0,
        report.line_rtp * 100.0,
        report.scatter_rtp * 100.0,
        report.feature_rtp * 100.0,
        report.hit_frequency.estimate * 100.0,
        1.0 / report.feature_frequency.estimate.max(1e-9),
    );
    assert!(
        (0.5..1.5).contains(&report.rtp.estimate),
        "RTP {:.3}",
        report.rtp.estimate
    );
    assert!(report.line_rtp > 0.0 && report.scatter_rtp > 0.0);
    assert!(report.feature_frequency.estimate > 0.0);

    // The feature set plays and returns cleanly; the royals never appear inside
    // the free games, and no reveal ever fires (no mystery symbol).
    let royals: Vec<String> = ["9", "10", "J", "Q", "K"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(4);
    let (mut free, mut retrig, mut reveal) = (0, 0, 0);
    let mut in_free = false;
    for _ in 0..60_000 {
        for e in &game.play_spin(&mut rng).unwrap() {
            match e {
                Event::FreeGamesStarted { games } => {
                    assert_eq!(*games, 6, "six free games");
                    free += 1;
                    in_free = true;
                }
                Event::FreeGamesRetriggered { added, .. } => {
                    assert_eq!(*added, 3, "three extra games on a retrigger");
                    retrig += 1;
                }
                Event::FreeGamesCompleted { .. } => in_free = false,
                Event::MysteryRevealed { .. } => reveal += 1,
                Event::ReelStopped { symbols, .. } if in_free => {
                    for key in symbols {
                        assert!(
                            !royals.contains(key),
                            "royal {key} appeared inside the free games"
                        );
                    }
                }
                _ => {}
            }
        }
        assert!(game.is_idle());
    }
    eprintln!("Panda Magic: {free} free rounds, {retrig} retriggers in 60k spins");
    assert!(free > 0, "free games never triggered");
    assert_eq!(reveal, 0, "there is no mystery reveal in this title");
}

/// A spin that wins both features plays Hold & Spin first, then its free
/// games, and completes once, after both (the owner's account of the
/// cabinet). Found by playing a model padded with orbs and scatters.
#[test]
fn both_features_from_one_spin_play_hold_and_spin_then_the_free_games() {
    use dragonlink_core::{DevRng, Event, Game};
    let mut model = golden_century_synthetic();
    let id = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .map(|i| dragonlink_model::SymbolId(i as u16))
            .unwrap()
    };
    let (orb, scatter) = (id("orb"), id("pennant"));
    for strip in &mut model.reel_strips {
        let mut padded = Vec::new();
        for (i, &s) in strip.iter().enumerate() {
            padded.push(s);
            if i % 2 == 0 {
                padded.push(orb);
            }
            if i % 5 == 0 {
                padded.push(scatter);
            }
        }
        *strip = padded;
    }
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(11);
    let mut both = 0;
    for _ in 0..4000 {
        let events = game.play_spin(&mut rng).unwrap();
        let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
        let hs_start = at(&|e| matches!(e, Event::HoldAndSpinStarted { .. }));
        let hs_end = at(&|e| matches!(e, Event::HoldAndSpinCompleted { .. }));
        let fg_start = at(&|e| matches!(e, Event::FreeGamesStarted { .. }));
        let fg_end = at(&|e| matches!(e, Event::FreeGamesCompleted { .. }));
        let completed: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Event::SpinCompleted { .. }))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(completed.len(), 1, "one completion a spin");
        // The base window won both (a Hold & Spin inside the free games,
        // started by their coins, comes after them instead).
        if let (Some(a), Some(b), Some(c), Some(d)) = (hs_start, hs_end, fg_start, fg_end)
            && a < c
        {
            both += 1;
            assert!(a < b && b < c && c < d, "Hold & Spin, then the free games");
            assert!(completed[0] > d, "the spin completes after both");
        }
        assert!(game.is_idle());
    }
    assert!(both > 0, "no spin won both features");
}
