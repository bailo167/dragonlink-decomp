//! Exact analysis of a math model.
//!
//! Everything here is computed from the model's distributions, not sampled.
//! The derivations, with their assumptions, are in
//! `docs/reverse-engineering/math/METHODS.md`. In brief:
//!
//! - **Line pays.** Every stop of a reel is equally likely, so the symbol
//!   showing at any one row of a reel is distributed as the strip's symbol
//!   frequencies, independently between reels. The expected pay of a line is
//!   therefore a sum over symbol combinations weighted by frequency products:
//!   an exact rational number. Every line has the same expectation.
//! - **Scatter pays and orb counts.** The number of a symbol in one reel's
//!   window is a distribution over that reel's stops; the window total is the
//!   convolution over reels.
//! - **Hold & Spin.** From `e` empty positions with `r` respins left, the
//!   number of positions that land is binomial. The state only moves to fewer
//!   empties or fewer respins, so the distribution of the final count follows
//!   from a finite recursion.
//! - **Hit frequency and variance** need the joint behaviour of all lines
//!   and are found by enumerating every stop combination through the
//!   simulation's own evaluator, when the number of combinations allows.
//!
//! These are properties of the model that is passed in. The rules themselves
//! (what a wild does, how the feature runs) are the generic rules implemented
//! in `dragonlink-core`; whether the original games follow them is a separate
//! question, tracked in the claims.

use std::collections::BTreeMap;

use dragonlink_core::{Grid, evaluate, line_run};
use dragonlink_model::{JackpotTier, MathModel, ModelError, OrbValue, SymbolId, SymbolKind};
use serde::Serialize;

/// Default ceiling on stop combinations for full enumeration.
pub const DEFAULT_ENUMERATION_LIMIT: u128 = 50_000_000;

/// Ceiling on symbol combinations for the line-pay sum.
const SYMBOL_COMBINATION_LIMIT: u128 = 200_000_000;

/// Why a model could not be analysed.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("the math model is not runnable: {0}")]
    Model(#[from] ModelError),
    #[error("{0} symbol combinations per line is too many to sum")]
    TooManySymbolCombinations(u128),
    #[error("a count does not fit in 128 bits")]
    Overflow,
}

/// One way a single line can pay.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LineOutcome {
    pub symbol: String,
    pub count: u8,
    /// Pay per credit bet on the line.
    pub pay: u32,
    /// Probability that a given line shows this outcome on one spin.
    pub probability: f64,
    /// This outcome's share of the total bet returned.
    pub rtp: f64,
}

/// Frequency and value of one jackpot level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JackpotStat {
    pub tier: JackpotTier,
    /// Expected number of awards of this level per spin.
    pub awards_per_spin: f64,
    /// Value in the model, as a multiple of the total bet.
    pub total_bet_multiple: u32,
    /// Share of the total bet returned through this level at that value.
    pub rtp: f64,
}

impl JackpotStat {
    /// Average number of spins between awards, when the level can be won.
    pub fn one_in(&self) -> Option<f64> {
        (self.awards_per_spin > 0.0).then(|| 1.0 / self.awards_per_spin)
    }
}

/// The Hold & Spin feature.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureAnalysis {
    /// Probability that a spin starts the feature.
    pub trigger_probability: f64,
    /// `(orbs held at the start, probability per spin)`.
    pub start_distribution: Vec<(u8, f64)>,
    /// `(orbs held at the end, probability given the feature started)`.
    pub final_distribution: Vec<(u8, f64)>,
    /// Expected orbs held at the end, given the feature started.
    pub expected_final_orbs: f64,
    /// Expected respins played, given the feature started.
    pub expected_respins: f64,
    /// Expected value of one orb, as a multiple of the total bet.
    pub mean_orb_value: f64,
    /// Probability per spin that the feature ends with every position held.
    pub full_grid_per_spin: f64,
    /// Expected award of the feature given it started, in multiples of the total bet.
    pub value_given_trigger: f64,
    /// Share of the total bet returned through credit-value orbs.
    pub rtp_credit_orbs: f64,
    pub jackpots: Vec<JackpotStat>,
    /// Share of the total bet returned through the whole feature.
    pub rtp: f64,
    /// Mean and second moment of the feature award for each starting count.
    pub award_moments: Vec<StartMoments>,
    /// Standard deviation of the feature award given it started, in
    /// multiples of the total bet.
    pub std_dev_given_trigger: f64,
}

/// The feature award, in multiples of the total bet, given how many orbs it
/// started with.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StartMoments {
    pub start: u8,
    pub mean: f64,
    /// Expected square of the award.
    pub second_moment: f64,
}

/// Results of enumerating every stop combination.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Enumeration {
    pub combinations: u128,
    /// Probability that a spin pays on a line or scatter.
    pub base_hit_frequency: f64,
    /// Probability that a spin pays on a line or scatter, or starts the feature.
    pub any_hit_frequency: f64,
    /// Mean base-game award in multiples of the total bet (lines and scatters).
    pub base_mean: f64,
    /// Variance of the base-game award in multiples of the total bet.
    pub base_variance: f64,
    /// Largest base-game award found, in multiples of the total bet.
    pub base_max: f64,
    /// Variance of a whole spin's award, feature included, in multiples of
    /// the total bet. Exact: the base award and the feature are linked only
    /// through the number of orbs in the window, which the enumeration sees.
    pub total_variance: f64,
}

/// The quantitative breakdown of a model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Analysis {
    /// Number of equally likely reel-stop combinations.
    pub stop_combinations: u128,
    /// Expected line pay per credit bet on a line, as an exact fraction
    /// `(numerator, denominator)`. Equal to the line share of RTP.
    pub line_rtp_exact: (u128, u128),
    pub line_rtp: f64,
    /// Probability that a given line pays.
    pub line_hit_probability: f64,
    pub line_outcomes: Vec<LineOutcome>,
    pub scatter_rtp: f64,
    /// Per scatter symbol: probability of each count in the window.
    pub scatter_distributions: Vec<(String, Vec<f64>)>,
    /// Probability of each number of orbs in the window after a spin.
    pub orb_distribution: Vec<f64>,
    pub feature: Option<FeatureAnalysis>,
    /// Line, scatter and feature shares together.
    pub total_rtp: f64,
    /// Present when the stop combinations were few enough to enumerate.
    pub enumeration: Option<Enumeration>,
}

impl Analysis {
    /// Share of the total bet returned through fixed pays and credit orbs,
    /// that is, everything except jackpot levels.
    pub fn rtp_excluding_jackpots(&self) -> f64 {
        self.line_rtp + self.scatter_rtp + self.feature.as_ref().map_or(0.0, |f| f.rtp_credit_orbs)
    }
}

/// Distribution of the number of symbols satisfying `wanted` in the whole
/// window: element `k` is the probability of exactly `k`.
fn window_count_distribution(model: &MathModel, wanted: &dyn Fn(SymbolId) -> bool) -> Vec<f64> {
    let rows = model.grid.rows as usize;
    let mut total = vec![1.0f64];
    for strip in &model.reel_strips {
        let mut reel = vec![0.0f64; rows + 1];
        for stop in 0..strip.len() {
            let count = (0..rows)
                .filter(|row| wanted(strip[(stop + row) % strip.len()]))
                .count();
            reel[count] += 1.0 / strip.len() as f64;
        }
        let mut next = vec![0.0f64; total.len() + rows];
        for (a, pa) in total.iter().enumerate() {
            for (b, pb) in reel.iter().enumerate() {
                next[a + b] += pa * pb;
            }
        }
        total = next;
    }
    total
}

fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// For every number of empty positions `e` and respins left `r`:
/// the distribution of the number of positions still empty when the feature
/// ends (`dist[e][r][f]`), and the expected respins still to play (`respins[e][r]`).
fn feature_tables(
    cells: usize,
    initial: usize,
    reset: bool,
    p: f64,
) -> (Vec<Vec<Vec<f64>>>, Vec<Vec<f64>>) {
    let q = 1.0 - p;
    let mut dist: Vec<Vec<Vec<f64>>> = Vec::with_capacity(cells + 1);
    let mut respins: Vec<Vec<f64>> = Vec::with_capacity(cells + 1);
    for e in 0..=cells {
        let mut by_r: Vec<Vec<f64>> = Vec::with_capacity(initial + 1);
        let mut resp_by_r: Vec<f64> = Vec::with_capacity(initial + 1);
        for r in 0..=initial {
            let mut d = vec![0.0f64; e + 1];
            let mut resp = 0.0;
            if e == 0 || r == 0 {
                d[e] = 1.0;
            } else {
                resp = 1.0;
                let none = q.powi(e as i32);
                for (f, v) in by_r[r - 1].iter().enumerate() {
                    d[f] += none * v;
                }
                resp += none * resp_by_r[r - 1];
                let next_r = if reset { initial } else { r - 1 };
                for j in 1..=e {
                    let landed = binomial(e, j) * p.powi(j as i32) * q.powi((e - j) as i32);
                    for (f, v) in dist[e - j][next_r].iter().enumerate() {
                        d[f] += landed * v;
                    }
                    resp += landed * respins[e - j][next_r];
                }
            }
            by_r.push(d);
            resp_by_r.push(resp);
        }
        dist.push(by_r);
        respins.push(resp_by_r);
    }
    (dist, respins)
}

fn analyze_feature(model: &MathModel, orb_distribution: &[f64]) -> Option<FeatureAnalysis> {
    let hs = model.hold_and_spin.as_ref()?;
    let cells = model.grid.cells();
    let trigger = hs.trigger_count as usize;
    let initial = hs.initial_respins as usize;
    let p = f64::from(hs.orb_land_chance.num) / f64::from(hs.orb_land_chance.den);
    let (dist, respins) = feature_tables(cells, initial, hs.reset_respins_on_new_orb, p);

    let multiple_of = |tier: JackpotTier| {
        hs.jackpots
            .iter()
            .find(|j| j.tier == tier)
            .map_or(0, |j| j.total_bet_multiple)
    };
    let weight: f64 = hs.orb_values.iter().map(|v| f64::from(v.weight)).sum();
    let mut credit_mean = 0.0;
    let mut tier_share: BTreeMap<JackpotTier, f64> = BTreeMap::new();
    for entry in &hs.orb_values {
        let share = f64::from(entry.weight) / weight;
        match entry.value {
            OrbValue::BetMultiple(m) => credit_mean += share * f64::from(m),
            OrbValue::Jackpot(tier) => *tier_share.entry(tier).or_insert(0.0) += share,
        }
    }
    let mean_orb_value = credit_mean
        + tier_share
            .iter()
            .map(|(tier, share)| share * f64::from(multiple_of(*tier)))
            .sum::<f64>();

    // Second moment of one orb's value, and the award for a full grid.
    let orb_second_moment: f64 = hs
        .orb_values
        .iter()
        .map(|entry| {
            let value = match entry.value {
                OrbValue::BetMultiple(m) => f64::from(m),
                OrbValue::Jackpot(tier) => f64::from(multiple_of(tier)),
            };
            f64::from(entry.weight) / weight * value * value
        })
        .sum();
    let orb_variance = (orb_second_moment - mean_orb_value * mean_orb_value).max(0.0);
    let full_grid_value = hs
        .full_grid_award
        .map_or(0.0, |tier| f64::from(multiple_of(tier)));

    let mut trigger_probability = 0.0;
    let mut start_distribution = Vec::new();
    let mut final_per_spin = vec![0.0f64; cells + 1];
    let mut respins_per_spin = 0.0;
    let mut award_moments = Vec::new();
    let mut second_moment_per_spin = 0.0;
    for (k, pk) in orb_distribution.iter().enumerate().skip(trigger) {
        if *pk == 0.0 || k > cells {
            continue;
        }
        trigger_probability += pk;
        start_distribution.push((k as u8, *pk));
        let empty = cells - k;
        // The award is the sum of N independent orb values, plus the full-grid
        // award when N is every position. With N = cells - f:
        //   E[award]   = E[N] m + G P(full)
        //   E[award^2] = E[N] v + E[N^2] m^2 + 2 G cells m P(full) + G^2 P(full)
        let (mut n_mean, mut n_second, mut full) = (0.0, 0.0, 0.0);
        for (f, v) in dist[empty][initial].iter().enumerate() {
            final_per_spin[cells - f] += pk * v;
            let n = (cells - f) as f64;
            n_mean += v * n;
            n_second += v * n * n;
            if f == 0 {
                full = *v;
            }
        }
        let mean = n_mean * mean_orb_value + full_grid_value * full;
        let second_moment = n_mean * orb_variance
            + n_second * mean_orb_value * mean_orb_value
            + 2.0 * full_grid_value * cells as f64 * mean_orb_value * full
            + full_grid_value * full_grid_value * full;
        award_moments.push(StartMoments {
            start: k as u8,
            mean,
            second_moment,
        });
        second_moment_per_spin += pk * second_moment;
        respins_per_spin += pk * respins[empty][initial];
    }

    let final_orbs_per_spin: f64 = final_per_spin
        .iter()
        .enumerate()
        .map(|(n, pr)| n as f64 * pr)
        .sum();
    let full_grid_per_spin = final_per_spin[cells];
    let conditional = |x: f64| {
        if trigger_probability > 0.0 {
            x / trigger_probability
        } else {
            0.0
        }
    };

    let rtp_credit_orbs = final_orbs_per_spin * credit_mean;
    let mut tiers: Vec<JackpotTier> = hs.jackpots.iter().map(|j| j.tier).collect();
    tiers.sort();
    let jackpots: Vec<JackpotStat> = tiers
        .into_iter()
        .map(|tier| {
            let from_orbs = final_orbs_per_spin * tier_share.get(&tier).copied().unwrap_or(0.0);
            let from_full_grid = if hs.full_grid_award == Some(tier) {
                full_grid_per_spin
            } else {
                0.0
            };
            let awards_per_spin = from_orbs + from_full_grid;
            let total_bet_multiple = multiple_of(tier);
            JackpotStat {
                tier,
                awards_per_spin,
                total_bet_multiple,
                rtp: awards_per_spin * f64::from(total_bet_multiple),
            }
        })
        .collect();
    let rtp = rtp_credit_orbs + jackpots.iter().map(|j| j.rtp).sum::<f64>();

    Some(FeatureAnalysis {
        trigger_probability,
        start_distribution,
        final_distribution: final_per_spin
            .iter()
            .enumerate()
            .filter(|(_, pr)| **pr > 0.0)
            .map(|(n, pr)| (n as u8, conditional(*pr)))
            .collect(),
        expected_final_orbs: conditional(final_orbs_per_spin),
        expected_respins: conditional(respins_per_spin),
        mean_orb_value,
        full_grid_per_spin,
        value_given_trigger: conditional(rtp),
        rtp_credit_orbs,
        jackpots,
        rtp,
        award_moments,
        std_dev_given_trigger: (conditional(second_moment_per_spin)
            - conditional(rtp) * conditional(rtp))
        .max(0.0)
        .sqrt(),
    })
}

/// Number of stop combinations giving each paying line outcome
/// `(symbol, count, pay)`.
type OutcomeWeights = BTreeMap<(SymbolId, u8, u32), u128>;

/// Sum over symbol combinations along one line. Returns the numerator of the
/// expected pay (the denominator is the number of stop combinations) and, per
/// paying outcome, its weight.
fn line_sum(model: &MathModel) -> Result<(u128, OutcomeWeights), AnalysisError> {
    let symbols = model.symbols.len();
    let reels = model.reel_strips.len();
    let combos = (symbols as u128)
        .checked_pow(reels as u32)
        .ok_or(AnalysisError::Overflow)?;
    if combos > SYMBOL_COMBINATION_LIMIT {
        return Err(AnalysisError::TooManySymbolCombinations(combos));
    }
    let counts: Vec<Vec<u128>> = model
        .reel_strips
        .iter()
        .map(|strip| {
            let mut c = vec![0u128; symbols];
            for id in strip {
                c[id.0 as usize] += 1;
            }
            c
        })
        .collect();

    let mut numerator = 0u128;
    let mut outcomes = OutcomeWeights::new();
    let mut choice = vec![0usize; reels];
    let mut line = vec![SymbolId(0); reels];
    'combinations: loop {
        let mut weight = 1u128;
        for (reel, symbol) in choice.iter().enumerate() {
            weight = weight
                .checked_mul(counts[reel][*symbol])
                .ok_or(AnalysisError::Overflow)?;
            line[reel] = SymbolId(*symbol as u16);
        }
        if weight > 0
            && let Some((symbol, count, pay)) = line_run(model, &line)
        {
            numerator = weight
                .checked_mul(u128::from(pay))
                .and_then(|w| numerator.checked_add(w))
                .ok_or(AnalysisError::Overflow)?;
            *outcomes.entry((symbol, count, pay)).or_insert(0) += weight;
        }
        // Next combination (odometer).
        for digit in choice.iter_mut() {
            *digit += 1;
            if *digit < symbols {
                continue 'combinations;
            }
            *digit = 0;
        }
        break;
    }
    Ok((numerator, outcomes))
}

fn enumerate(
    model: &MathModel,
    combinations: u128,
    feature: Option<&FeatureAnalysis>,
) -> Option<Enumeration> {
    let rows = model.grid.rows as usize;
    // Sum of the base award over the windows holding each number of orbs.
    let mut base_by_orbs = vec![0.0f64; model.grid.cells() + 1];
    let lines = model.paylines.len() as u64;
    let trigger = model
        .hold_and_spin
        .as_ref()
        .map(|hs| hs.trigger_count as usize);
    let mut stops = vec![0usize; model.reel_strips.len()];
    let mut cells = Vec::with_capacity(model.grid.cells());
    let (mut sum, mut sum_sq, mut max) = (0.0f64, 0.0f64, 0.0f64);
    let (mut base_hits, mut any_hits) = (0u128, 0u128);
    'stops: loop {
        cells.clear();
        for (reel, strip) in model.reel_strips.iter().enumerate() {
            for row in 0..rows {
                cells.push(strip[(stops[reel] + row) % strip.len()]);
            }
        }
        let orbs = cells
            .iter()
            .filter(|s| model.kind(**s) == Some(SymbolKind::Orb))
            .count();
        let grid = Grid::new(model.grid, cells.clone())?;
        // One credit per line, so the total bet is the number of lines.
        let award: u64 = evaluate(model, &grid, 1, lines)?
            .iter()
            .map(|w| w.award())
            .sum();
        let x = award as f64 / lines as f64;
        sum += x;
        sum_sq += x * x;
        max = max.max(x);
        base_by_orbs[orbs] += x;
        let triggered = trigger.is_some_and(|t| orbs >= t);
        if award > 0 {
            base_hits += 1;
        }
        if award > 0 || triggered {
            any_hits += 1;
        }
        for (reel, stop) in stops.iter_mut().enumerate() {
            *stop += 1;
            if *stop < model.reel_strips[reel].len() {
                continue 'stops;
            }
            *stop = 0;
        }
        break;
    }
    let n = combinations as f64;
    let mean = sum / n;
    // Whole spin: X = B + F. Given the window, F depends only on its orb count k, so
    //   E[X^2] = E[B^2] + 2 * sum_k E[B; K = k] E[F | k] + sum_k P(k) E[F^2 | k].
    let (mut cross, mut feature_second, mut feature_mean) = (0.0, 0.0, 0.0);
    if let Some(f) = feature {
        for (moments, (_, pk)) in f.award_moments.iter().zip(&f.start_distribution) {
            cross += base_by_orbs[moments.start as usize] / n * moments.mean;
            feature_second += pk * moments.second_moment;
            feature_mean += pk * moments.mean;
        }
    }
    let total_mean = mean + feature_mean;
    let total_second = sum_sq / n + 2.0 * cross + feature_second;
    Some(Enumeration {
        combinations,
        base_hit_frequency: base_hits as f64 / n,
        any_hit_frequency: any_hits as f64 / n,
        base_mean: mean,
        base_variance: (sum_sq / n - mean * mean).max(0.0),
        base_max: max,
        total_variance: (total_second - total_mean * total_mean).max(0.0),
    })
}

/// Analyse `model`. Stop combinations are enumerated in full when there are
/// at most `enumeration_limit` of them.
pub fn analyze(model: &MathModel, enumeration_limit: u128) -> Result<Analysis, AnalysisError> {
    model.validate()?;
    let stop_combinations = model
        .reel_strips
        .iter()
        .try_fold(1u128, |acc, strip| acc.checked_mul(strip.len() as u128))
        .ok_or(AnalysisError::Overflow)?;
    let key = |id: SymbolId| {
        model
            .symbol(id)
            .map_or_else(|| format!("#{}", id.0), |s| s.key.clone())
    };

    let (numerator, outcome_weights) = line_sum(model)?;
    let denominator = stop_combinations as f64;
    let line_rtp = numerator as f64 / denominator;
    let mut line_outcomes: Vec<LineOutcome> = outcome_weights
        .iter()
        .map(|((symbol, count, pay), weight)| {
            let probability = *weight as f64 / denominator;
            LineOutcome {
                symbol: key(*symbol),
                count: *count,
                pay: *pay,
                probability,
                rtp: probability * f64::from(*pay),
            }
        })
        .collect();
    line_outcomes.sort_by(|a, b| b.rtp.total_cmp(&a.rtp));
    let line_hit_probability = outcome_weights.values().sum::<u128>() as f64 / denominator;

    let mut scatter_rtp = 0.0;
    let mut scatter_distributions = Vec::new();
    for def in model
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Scatter)
    {
        let id = def.id;
        let dist = window_count_distribution(model, &|s| s == id);
        for pay in model.scatter_pays.iter().filter(|p| p.symbol == id) {
            scatter_rtp += dist.get(pay.count as usize).copied().unwrap_or(0.0)
                * f64::from(pay.total_bet_multiple);
        }
        scatter_distributions.push((def.key.clone(), dist));
    }

    let orb_distribution =
        window_count_distribution(model, &|s| model.kind(s) == Some(SymbolKind::Orb));
    let feature = analyze_feature(model, &orb_distribution);
    let total_rtp = line_rtp + scatter_rtp + feature.as_ref().map_or(0.0, |f| f.rtp);
    let enumeration = (stop_combinations <= enumeration_limit)
        .then(|| enumerate(model, stop_combinations, feature.as_ref()))
        .flatten();

    Ok(Analysis {
        stop_combinations,
        line_rtp_exact: (numerator, stop_combinations),
        line_rtp,
        line_hit_probability,
        line_outcomes,
        scatter_rtp,
        scatter_distributions,
        orb_distribution,
        feature,
        total_rtp,
        enumeration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_model::{
        BetModel, GridShape, HoldAndSpinModel, JackpotDef, LinePay, MathProvenance, Payline, Ratio,
        ScatterPay, SymbolDef, WeightedOrbValue,
    };

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    fn symbols(defs: &[(&str, SymbolKind)]) -> Vec<SymbolDef> {
        defs.iter()
            .enumerate()
            .map(|(i, (key, kind))| SymbolDef {
                id: SymbolId(i as u16),
                key: (*key).to_string(),
                kind: *kind,
            })
            .collect()
    }

    fn ids(v: &[u16]) -> Vec<SymbolId> {
        v.iter().copied().map(SymbolId).collect()
    }

    /// Three reels, three rows, three lines; A, B regular, W wild, S scatter, O orb.
    /// Small enough to enumerate (6 × 7 × 8 = 336 stop combinations) and to
    /// reach the feature.
    fn small() -> MathModel {
        const A: u16 = 0;
        const B: u16 = 1;
        const W: u16 = 2;
        const S: u16 = 3;
        const O: u16 = 4;
        MathModel {
            id: "test-small".into(),
            provenance: MathProvenance::SyntheticDevFixture,
            description: "test".into(),
            grid: GridShape { reels: 3, rows: 3 },
            symbols: symbols(&[
                ("A", SymbolKind::Regular),
                ("B", SymbolKind::Regular),
                ("W", SymbolKind::Wild),
                ("S", SymbolKind::Scatter),
                ("O", SymbolKind::Orb),
            ]),
            reel_strips: vec![
                ids(&[A, B, O, O, S, B]),
                ids(&[A, W, B, O, O, S, B]),
                ids(&[A, B, W, O, O, O, S, B]),
            ],
            paylines: vec![
                Payline(vec![0, 0, 0]),
                Payline(vec![1, 1, 1]),
                Payline(vec![0, 1, 2]),
            ],
            line_pays: vec![
                LinePay {
                    symbol: SymbolId(A),
                    count: 2,
                    pay: 2,
                },
                LinePay {
                    symbol: SymbolId(A),
                    count: 3,
                    pay: 10,
                },
                LinePay {
                    symbol: SymbolId(B),
                    count: 3,
                    pay: 4,
                },
                LinePay {
                    symbol: SymbolId(W),
                    count: 2,
                    pay: 7,
                },
            ],
            scatter_pays: vec![
                ScatterPay {
                    symbol: SymbolId(S),
                    count: 2,
                    total_bet_multiple: 1,
                },
                ScatterPay {
                    symbol: SymbolId(S),
                    count: 3,
                    total_bet_multiple: 5,
                },
            ],
            bet: BetModel {
                credits_per_line_options: vec![1, 3],
            },
            hold_and_spin: Some(HoldAndSpinModel {
                orb: SymbolId(O),
                trigger_count: 4,
                initial_respins: 3,
                reset_respins_on_new_orb: true,
                orb_land_chance: Ratio { num: 1, den: 5 },
                orb_values: vec![
                    WeightedOrbValue {
                        weight: 6,
                        value: OrbValue::BetMultiple(1),
                    },
                    WeightedOrbValue {
                        weight: 3,
                        value: OrbValue::BetMultiple(4),
                    },
                    WeightedOrbValue {
                        weight: 1,
                        value: OrbValue::Jackpot(JackpotTier::Mini),
                    },
                ],
                full_grid_award: Some(JackpotTier::Grand),
                jackpots: vec![
                    JackpotDef {
                        tier: JackpotTier::Mini,
                        total_bet_multiple: 8,
                    },
                    JackpotDef {
                        tier: JackpotTier::Grand,
                        total_bet_multiple: 200,
                    },
                ],
            }),
        }
    }

    #[test]
    fn a_one_reel_game_matches_hand_arithmetic() {
        // One reel, one row: A shows on 2 of 4 stops and pays 4, so the return is 2.
        let model = MathModel {
            id: "test-one".into(),
            provenance: MathProvenance::SyntheticDevFixture,
            description: "test".into(),
            grid: GridShape { reels: 1, rows: 1 },
            symbols: symbols(&[("A", SymbolKind::Regular), ("X", SymbolKind::Regular)]),
            reel_strips: vec![ids(&[0, 0, 1, 1])],
            paylines: vec![Payline(vec![0])],
            line_pays: vec![LinePay {
                symbol: SymbolId(0),
                count: 1,
                pay: 4,
            }],
            scatter_pays: vec![],
            bet: BetModel {
                credits_per_line_options: vec![1],
            },
            hold_and_spin: None,
        };
        let a = analyze(&model, DEFAULT_ENUMERATION_LIMIT).unwrap();
        assert_eq!(a.stop_combinations, 4);
        assert_eq!(a.line_rtp_exact, (8, 4));
        assert!(close(a.line_rtp, 2.0));
        assert!(close(a.line_hit_probability, 0.5));
        assert!(close(a.total_rtp, 2.0));
        assert!(a.feature.is_none());
        let e = a.enumeration.unwrap();
        assert!(close(e.base_hit_frequency, 0.5));
        assert!(close(e.base_mean, 2.0));
        // Award is 0 or 4 with equal probability: variance 4.
        assert!(close(e.base_variance, 4.0));
        assert!(close(e.base_max, 4.0));
        // Without a feature the whole spin is the base game.
        assert!(close(e.total_variance, e.base_variance));
    }

    #[test]
    fn the_frequency_method_agrees_with_full_enumeration() {
        // Two independent routes to the base-game return: symbol frequencies per
        // line, and every stop combination through the simulation's evaluator.
        let model = small();
        let a = analyze(&model, DEFAULT_ENUMERATION_LIMIT).unwrap();
        assert_eq!(a.stop_combinations, 336);
        let e = a.enumeration.as_ref().unwrap();
        assert!(
            close(e.base_mean, a.line_rtp + a.scatter_rtp),
            "enumerated {} vs summed {}",
            e.base_mean,
            a.line_rtp + a.scatter_rtp
        );
        assert!(a.line_rtp > 0.0 && a.scatter_rtp > 0.0);
        assert!(e.any_hit_frequency >= e.base_hit_frequency);
        // Outcome shares add up to the line return.
        let from_outcomes: f64 = a.line_outcomes.iter().map(|o| o.rtp).sum();
        assert!(close(from_outcomes, a.line_rtp));
    }

    #[test]
    fn count_distributions_are_probability_distributions() {
        let a = analyze(&small(), DEFAULT_ENUMERATION_LIMIT).unwrap();
        assert!(close(a.orb_distribution.iter().sum::<f64>(), 1.0));
        for (_, d) in &a.scatter_distributions {
            assert!(close(d.iter().sum::<f64>(), 1.0));
        }
        let f = a.feature.unwrap();
        assert!(close(
            f.final_distribution.iter().map(|(_, p)| p).sum::<f64>(),
            1.0
        ));
        let start: f64 = f.start_distribution.iter().map(|(_, p)| p).sum();
        assert!(close(start, f.trigger_probability));
        assert!(f.trigger_probability > 0.0 && f.trigger_probability < 1.0);
    }

    #[test]
    fn the_feature_recursion_matches_hand_arithmetic() {
        // One empty position, chance 1/2. With two respins the position stays
        // empty only if both miss: full with probability 3/4, and the expected
        // number of respins is 1 + 1/2 = 1.5.
        let (dist, respins) = feature_tables(2, 2, true, 0.5);
        assert!(close(dist[1][2][0], 0.75));
        assert!(close(dist[1][2][1], 0.25));
        assert!(close(respins[1][2], 1.5));
        assert!(close(respins[1][1], 1.0));

        // Two empty positions, one respin, chance 1/2.
        // With reset: full = 1/4 (both land) + 1/2 × 1/2 (one lands, then the other) = 1/2.
        let (with_reset, _) = feature_tables(2, 1, true, 0.5);
        assert!(close(with_reset[2][1][0], 0.5));
        assert!(close(with_reset[2][1][1], 0.25));
        assert!(close(with_reset[2][1][2], 0.25));
        // Without reset a single landing ends the feature one short: full = 1/4.
        let (no_reset, _) = feature_tables(2, 1, false, 0.5);
        assert!(close(no_reset[2][1][0], 0.25));
        assert!(close(no_reset[2][1][1], 0.5));
        assert!(close(no_reset[2][1][2], 0.25));
    }

    #[test]
    fn feature_value_is_composed_of_its_parts() {
        let model = small();
        let a = analyze(&model, DEFAULT_ENUMERATION_LIMIT).unwrap();
        let f = a.feature.as_ref().unwrap();
        // Mean orb value: (6×1 + 3×4 + 1×8) / 10 = 2.6 total bets.
        assert!(close(f.mean_orb_value, 2.6));
        let jackpot_rtp: f64 = f.jackpots.iter().map(|j| j.rtp).sum();
        assert!(close(f.rtp, f.rtp_credit_orbs + jackpot_rtp));
        assert!(close(f.value_given_trigger * f.trigger_probability, f.rtp));
        assert!(close(a.total_rtp, a.line_rtp + a.scatter_rtp + f.rtp));
        assert!(close(a.rtp_excluding_jackpots() + jackpot_rtp, a.total_rtp));
        // The Grand is won only on a full grid.
        let grand = f
            .jackpots
            .iter()
            .find(|j| j.tier == JackpotTier::Grand)
            .unwrap();
        assert!(close(grand.awards_per_spin, f.full_grid_per_spin));
        assert!(grand.one_in().unwrap() > 1.0);
        // At least the trigger count is held at the end, at most every position.
        assert!(f.expected_final_orbs >= 4.0 && f.expected_final_orbs <= 9.0);
    }

    #[test]
    fn feature_award_moments_match_hand_arithmetic() {
        // With a land chance of zero the feature ends holding exactly what it
        // started with, so the award is a sum of that many independent values.
        // One value: mean 2.6, second moment 0.6×1 + 0.3×16 + 0.1×64 = 11.8,
        // variance 5.04. Four orbs: mean 10.4, second moment 4×5.04 + 16×6.76.
        let mut model = small();
        if let Some(hs) = &mut model.hold_and_spin {
            hs.orb_land_chance = Ratio { num: 0, den: 5 };
        }
        let a = analyze(&model, DEFAULT_ENUMERATION_LIMIT).unwrap();
        let f = a.feature.unwrap();
        let four = f.award_moments.iter().find(|m| m.start == 4).unwrap();
        assert!(close(four.mean, 10.4));
        assert!((four.second_moment - 128.32).abs() < 1e-9);
        // The moments are listed for the same starting counts as the distribution.
        let starts: Vec<u8> = f.award_moments.iter().map(|m| m.start).collect();
        let listed: Vec<u8> = f.start_distribution.iter().map(|(k, _)| *k).collect();
        assert_eq!(starts, listed);
    }

    #[test]
    fn exact_whole_spin_deviation_agrees_with_simulation() {
        // The exact figure comes from distributions; the simulated one from
        // playing the state machine. Our mathematics against our implementation.
        let model = small();
        let a = analyze(&model, DEFAULT_ENUMERATION_LIMIT).unwrap();
        let e = a.enumeration.as_ref().unwrap();
        assert!(e.total_variance > e.base_variance);
        let exact = e.total_variance.sqrt();
        let sim = crate::sim::simulate(&model, 17, 400_000).unwrap();
        assert!(
            (sim.std_dev - exact).abs() / exact < 0.05,
            "simulated {} vs exact {exact}",
            sim.std_dev
        );
        assert!((sim.rtp.estimate - a.total_rtp).abs() < 4.5 * sim.standard_error);
        let f = a.feature.as_ref().unwrap();
        assert!(f.std_dev_given_trigger > 0.0);
    }

    #[test]
    fn enumeration_is_skipped_above_the_limit() {
        let a = analyze(&small(), 10).unwrap();
        assert!(a.enumeration.is_none());
        assert!(a.total_rtp > 0.0);
    }

    #[test]
    fn an_unrunnable_model_is_refused() {
        let mut model = small();
        model.paylines.clear();
        assert!(matches!(analyze(&model, 10), Err(AnalysisError::Model(_))));
    }
}
