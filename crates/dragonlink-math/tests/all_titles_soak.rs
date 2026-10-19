//! Hardening: every playable synthetic title survives a long session.
//!
//! For each of the eight titles this plays tens of thousands of spins through
//! `dragonlink-core`, varying the stake, and checks the engine's contract
//! holds the whole way: a spin never fails, the game always returns to the
//! base game, both features are reached, and the run is deterministic and
//! replays event for event. It makes no claim about Dragon Link; the figures
//! are properties of synthetic fixtures (`CLAUDE.md`).

use dragonlink_core::{Command, DevRng, Event, Game, RecordingRng, ScriptedRng};
use dragonlink_model::MathModel;
use dragonlink_model::fixtures::{
    autumn_moon_synthetic, genghis_khan_synthetic, golden_century_synthetic, golden_gong_synthetic,
    happy_prosperous_synthetic, panda_magic_synthetic, peace_long_life_synthetic,
    spring_festival_synthetic,
};

/// A title's name and a constructor for a fresh model of it.
type Title = (&'static str, fn() -> MathModel);

/// The eight titles the launcher offers, each as a fresh model.
const TITLES: [Title; 8] = [
    ("golden-century", golden_century_synthetic),
    ("happy-prosperous", happy_prosperous_synthetic),
    ("panda-magic", panda_magic_synthetic),
    ("autumn-moon", autumn_moon_synthetic),
    ("genghis-khan", genghis_khan_synthetic),
    ("golden-gong", golden_gong_synthetic),
    ("peace-long-life", peace_long_life_synthetic),
    ("spring-festival", spring_festival_synthetic),
];

/// A whole session of one title: play `spins` games, varying the stake, and
/// return how many of each feature were reached. Panics (through the engine's
/// own `Result`) if any spin fails or the game is left mid-feature.
fn play_session(make: fn() -> MathModel, seed: u64, spins: usize) -> (u32, u32) {
    let model = make();
    let bet_options = model.bet.credits_per_line_options.len();
    let line_counts = model.line_counts();
    // Start rich enough that the stake is never refused over the whole run.
    let mut game = Game::new(model, 1_000_000_000_000).expect("a valid model starts a game");
    let mut rng = DevRng::new(seed);
    let (mut free, mut hold) = (0u32, 0u32);
    for i in 0..spins {
        // Exercise the stake-setting paths now and then; ignore a refusal (for
        // example setting the same stake), it is not what this test is about.
        if i % 250 == 0 && bet_options > 0 {
            let _ = game.apply(
                Command::SetBet {
                    index: (i / 250) % bet_options,
                },
                &mut rng,
            );
        }
        if i % 400 == 0 && !line_counts.is_empty() {
            let _ = game.apply(
                Command::SetLines {
                    lines: line_counts[(i / 400) % line_counts.len()],
                },
                &mut rng,
            );
        }
        let events = game
            .play_spin(&mut rng)
            .expect("a spin on a valid model never fails");
        for e in &events {
            match e {
                Event::FreeGamesStarted { .. } => free += 1,
                Event::HoldAndSpinStarted { .. } => hold += 1,
                _ => {}
            }
        }
        assert!(
            game.is_idle(),
            "spin {i} left the game mid-feature (play_spin must return to the base game)"
        );
    }
    (free, hold)
}

#[test]
fn every_title_survives_a_long_session_and_reaches_both_features() {
    const SPINS: usize = 25_000;
    for (name, make) in TITLES {
        for seed in [1u64, 20_241_014] {
            let (free, hold) = play_session(make, seed, SPINS);
            eprintln!(
                "{name} seed {seed}: {free} free-game rounds, {hold} Hold & Spin in {SPINS} spins"
            );
            assert!(
                free > 0,
                "{name} seed {seed}: free games never triggered in {SPINS} spins"
            );
            assert!(
                hold > 0,
                "{name} seed {seed}: Hold & Spin never triggered in {SPINS} spins"
            );
        }
    }
}

#[test]
fn every_title_is_deterministic_and_replays_event_for_event() {
    const SPINS: usize = 3_000;
    for (name, make) in TITLES {
        // Same seed, same game: identical event streams.
        let mut a = Game::new(make(), 1_000_000_000).unwrap();
        let mut b = Game::new(make(), 1_000_000_000).unwrap();
        let (mut ra, mut rb) = (DevRng::new(7), DevRng::new(7));
        // Record one run's drawn values (wrapping a seeded source) so it can
        // be replayed exactly.
        let mut c = Game::new(make(), 1_000_000_000).unwrap();
        let mut recorder = RecordingRng::new(DevRng::new(7));
        let mut recorded = Vec::new();
        for _ in 0..SPINS {
            let ea = a.play_spin(&mut ra).unwrap();
            let eb = b.play_spin(&mut rb).unwrap();
            assert_eq!(ea, eb, "{name}: same seed produced different events");
            recorded.extend(c.play_spin(&mut recorder).unwrap());
        }
        assert_eq!(a, b, "{name}: same seed produced different end state");

        // The recorded draws replay event for event, into the same end state.
        let mut replay_game = Game::new(make(), 1_000_000_000).unwrap();
        let mut replay = ScriptedRng::new(recorder.take_log());
        let mut replayed = Vec::new();
        for _ in 0..SPINS {
            replayed.extend(replay_game.play_spin(&mut replay).unwrap());
        }
        assert_eq!(recorded, replayed, "{name}: recorded game did not replay");
        assert_eq!(replay.remaining(), 0, "{name}: replay left unused draws");
        assert_eq!(replay_game, c, "{name}: replay reached a different state");
    }
}
