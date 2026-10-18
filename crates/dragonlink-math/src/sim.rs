//! Seeded Monte Carlo over the real state machine.
//!
//! Used for two things: to cross-check the exact analysis by an independent
//! route (sampling `dragonlink-core` rather than summing distributions), and
//! to estimate what the exact analysis does not give, such as the spread of
//! a whole spin's award including the feature.
//!
//! A run is reproducible from its seed and spin count. Every estimate comes
//! with an interval. The generator is the project's test generator; it says
//! nothing about any original machine, and a simulated figure is never to be
//! presented without its interval and the configuration's provenance.

use std::collections::BTreeMap;

use dragonlink_core::{Command, DevRng, Event, Game, GameError};
use dragonlink_model::{JackpotTier, MathModel};
use serde::Serialize;

/// An estimate with a 95% interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Interval {
    pub estimate: f64,
    pub low: f64,
    pub high: f64,
}

impl Interval {
    pub fn contains(&self, x: f64) -> bool {
        self.low <= x && x <= self.high
    }
}

const Z95: f64 = 1.959_963_984_540_054;

/// Wilson score interval for a proportion.
pub(crate) fn wilson(successes: u64, n: u64) -> Interval {
    if n == 0 {
        return Interval {
            estimate: 0.0,
            low: 0.0,
            high: 1.0,
        };
    }
    let n = n as f64;
    let p = successes as f64 / n;
    let z2 = Z95 * Z95;
    let centre = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let half = Z95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / (1.0 + z2 / n);
    Interval {
        estimate: p,
        low: (centre - half).max(0.0),
        high: (centre + half).min(1.0),
    }
}

/// What a simulation run measured. Awards are in multiples of the total bet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimReport {
    pub seed: u64,
    pub spins: u64,
    pub rng: &'static str,
    /// Mean award per spin, with a normal-approximation interval. Heavy
    /// tails (rare large jackpots) make this interval optimistic for short runs.
    pub rtp: Interval,
    /// Standard deviation of one spin's award.
    pub std_dev: f64,
    pub standard_error: f64,
    /// Proportion of spins that paid anything.
    pub hit_frequency: Interval,
    /// Proportion of spins that paid on a line or scatter.
    pub base_hit_frequency: Interval,
    /// Proportion of spins that started Hold & Spin.
    pub feature_frequency: Interval,
    pub line_rtp: f64,
    pub scatter_rtp: f64,
    pub feature_rtp: f64,
    pub jackpot_awards: BTreeMap<JackpotTier, u64>,
    /// Largest single-spin award seen.
    pub max_award: f64,
}

/// Play `spins` spins of `model` at its first bet option from `seed`. A
/// progressive jackpot pays its bet multiple here, its meter never being set.
pub fn simulate(model: &MathModel, seed: u64, spins: u64) -> Result<SimReport, GameError> {
    simulate_game(Game::new(model.clone(), u64::MAX / 2)?, &[], seed, spins)
}

/// Play `spins` spins of `game` as it stands (its stake, its denomination)
/// from `seed`, with each progressive meter in `meters` (in money units) held
/// at its value before every spin, as a link standing at those values would.
pub fn simulate_game(
    mut game: Game,
    meters: &[(JackpotTier, u64)],
    seed: u64,
    spins: u64,
) -> Result<SimReport, GameError> {
    let mut rng = DevRng::new(seed);
    let bet = game.total_bet() as f64;

    let (mut sum, mut sum_sq, mut max) = (0.0f64, 0.0f64, 0.0f64);
    let (mut line, mut scatter, mut feature) = (0.0f64, 0.0f64, 0.0f64);
    let (mut hits, mut base_hits, mut features) = (0u64, 0u64, 0u64);
    let mut jackpot_awards: BTreeMap<JackpotTier, u64> = BTreeMap::new();

    for _ in 0..spins {
        for &(tier, value) in meters {
            game.apply(Command::SetProgressive { tier, value }, &mut rng)?;
        }
        let mut base = 0u64;
        let mut total = 0u64;
        for event in game.play_spin(&mut rng)? {
            match event {
                Event::LineWin { award, .. } => {
                    line += award as f64 / bet;
                    base += award;
                }
                Event::ScatterWin { award, .. } => {
                    scatter += award as f64 / bet;
                    base += award;
                }
                Event::HoldAndSpinStarted { .. } => features += 1,
                Event::HoldAndSpinCompleted { total_award, .. } => {
                    feature += total_award as f64 / bet;
                }
                Event::JackpotAwarded { tier, .. } => {
                    *jackpot_awards.entry(tier).or_insert(0) += 1;
                }
                Event::SpinCompleted { total_award, .. } => total = total_award,
                _ => {}
            }
        }
        let x = total as f64 / bet;
        sum += x;
        sum_sq += x * x;
        max = max.max(x);
        hits += u64::from(total > 0);
        base_hits += u64::from(base > 0);
    }

    let n = spins.max(1) as f64;
    let mean = sum / n;
    let variance = if spins > 1 {
        ((sum_sq - n * mean * mean) / (n - 1.0)).max(0.0)
    } else {
        0.0
    };
    let standard_error = (variance / n).sqrt();
    Ok(SimReport {
        seed,
        spins,
        rng: DevRng::NAME,
        rtp: Interval {
            estimate: mean,
            low: mean - Z95 * standard_error,
            high: mean + Z95 * standard_error,
        },
        std_dev: variance.sqrt(),
        standard_error,
        hit_frequency: wilson(hits, spins),
        base_hit_frequency: wilson(base_hits, spins),
        feature_frequency: wilson(features, spins),
        line_rtp: line / n,
        scatter_rtp: scatter / n,
        feature_rtp: feature / n,
        jackpot_awards,
        max_award: max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::analyze;
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    #[test]
    fn a_run_is_reproducible_from_its_seed() {
        let model = synthetic_dev_fixture();
        let a = simulate(&model, 11, 20_000).unwrap();
        let b = simulate(&model, 11, 20_000).unwrap();
        assert_eq!(a, b);
        assert_ne!(
            a.rtp.estimate,
            simulate(&model, 12, 20_000).unwrap().rtp.estimate
        );
        assert_eq!(a.spins, 20_000);
        assert!(a.rtp.low <= a.rtp.estimate && a.rtp.estimate <= a.rtp.high);
    }

    #[test]
    fn components_add_up_to_the_total() {
        let r = simulate(&synthetic_dev_fixture(), 5, 50_000).unwrap();
        let parts = r.line_rtp + r.scatter_rtp + r.feature_rtp;
        assert!(
            (parts - r.rtp.estimate).abs() < 1e-9,
            "{parts} vs {}",
            r.rtp.estimate
        );
        assert!(r.hit_frequency.estimate >= r.base_hit_frequency.estimate);
    }

    #[test]
    fn wilson_intervals_behave_at_the_edges() {
        let none = wilson(0, 1000);
        assert_eq!(none.estimate, 0.0);
        // The lower bound is zero up to rounding.
        assert!(none.low < 1e-12 && none.high > 0.0 && none.high < 0.01);
        let half = wilson(500, 1000);
        assert!(half.contains(0.5) && half.low > 0.46 && half.high < 0.54);
        assert_eq!(wilson(0, 0).high, 1.0);
    }

    /// The exact analysis and the simulation reach the same numbers by
    /// different routes. This checks our analysis and our state machine
    /// against each other on the synthetic fixture; it says nothing about
    /// any original game.
    #[test]
    fn simulation_agrees_with_exact_analysis() {
        let model = synthetic_dev_fixture();
        let exact = analyze(&model, 0).unwrap();
        let feature = exact.feature.as_ref().unwrap();
        let spins = 400_000u64;
        let sim = simulate(&model, 20_261_013, spins).unwrap();
        let n = spins as f64;

        // Components that are light-tailed are compared tightly.
        let line_se = 0.02; // generous bound on the standard error of the line share
        assert!(
            (sim.line_rtp - exact.line_rtp).abs() < 4.0 * line_se,
            "line {} vs {}",
            sim.line_rtp,
            exact.line_rtp
        );
        let p = feature.trigger_probability;
        let trigger_se = (p * (1.0 - p) / n).sqrt();
        assert!(
            (sim.feature_frequency.estimate - p).abs() < 4.5 * trigger_se,
            "trigger {} vs {p}",
            sim.feature_frequency.estimate
        );
        // The whole return, within its own measured standard error.
        assert!(
            (sim.rtp.estimate - exact.total_rtp).abs() < 4.5 * sim.standard_error,
            "rtp {} ± {} vs {}",
            sim.rtp.estimate,
            sim.standard_error,
            exact.total_rtp
        );
        // Scatter share.
        assert!((sim.scatter_rtp - exact.scatter_rtp).abs() < 0.01);
    }
}
