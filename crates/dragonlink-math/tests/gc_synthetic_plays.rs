//! The synthetic Golden Century configuration runs and has a sane shape.
//!
//! This checks only that the invented numbers produce a *playable* game: it
//! validates, pays on lines, reaches both the scatter and the Hold & Spin
//! feature, and returns somewhere between a black hole and free money. It
//! makes no claim about Dragon Link — every figure here is a property of a
//! synthetic fixture, not of any original game.

use dragonlink_math::sim::simulate;
use dragonlink_model::fixtures::golden_century_synthetic;

#[test]
fn the_synthetic_golden_century_is_playable() {
    let model = golden_century_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    assert_eq!(model.bet.line_options, vec![5, 25, 50]);
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

#[test]
fn the_free_games_feature_triggers_plays_and_returns_cleanly() {
    use dragonlink_core::{DevRng, Event, Game};
    let model = golden_century_synthetic();
    let mut game = Game::new(model, 1_000_000_000).unwrap();
    let mut rng = DevRng::new(4);
    let mut free_rounds = 0usize;
    let mut reveals = 0usize;
    let mut retriggers = 0usize;
    for _ in 0..60_000 {
        let events = game.play_spin(&mut rng).unwrap();
        for e in &events {
            match e {
                Event::FreeGamesStarted { games } => {
                    assert_eq!(*games, 6, "six free games");
                    free_rounds += 1;
                }
                Event::MysteryRevealed { count, .. } => {
                    assert!(*count >= 1);
                    reveals += 1;
                }
                Event::FreeGamesRetriggered { added, .. } => {
                    assert_eq!(*added, 6);
                    retriggers += 1;
                }
                _ => {}
            }
        }
        // play_spin plays features to the end, so we are always back to idle.
        assert!(game.is_idle(), "did not return to the base game");
    }
    eprintln!(
        "free games: {free_rounds} rounds, {reveals} reveals, {retriggers} retriggers in 60k spins"
    );
    assert!(free_rounds > 0, "free games never triggered");
    assert!(reveals > 0, "the reveal never happened");
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
