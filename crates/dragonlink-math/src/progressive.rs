//! Expected value as a function of jackpot meter values.
//!
//! For one spin at a given bet:
//!
//! ```text
//! return = return from fixed pays
//!        + Σ over jackpot levels ( expected awards per spin × meter value ) / total bet
//! ```
//!
//! The break-even value of one level is the meter value at which the return
//! reaches 100% with the other levels held where they are.
//!
//! What this assumes, so that a result is not over-read:
//!
//! - the award probabilities come from the configuration and do not depend
//!   on the meters (if evidence shows otherwise, that is a state dependence
//!   to model, not to ignore);
//! - a meter pays its displayed value;
//! - it is the value of one spin now. It says nothing about the variance: a
//!   level won once in millions of spins contributes its share of return
//!   only in the long run, to whoever happens to win it.
//!
//! On the synthetic fixture the levels are fixed multiples of the bet, so any
//! meter value passed in is a what-if on invented numbers.

use dragonlink_model::{JackpotTier, MathModel};
use serde::Serialize;

use crate::exact::Analysis;

/// One level's part in the expected value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TierEv {
    pub tier: JackpotTier,
    pub awards_per_spin: f64,
    /// The value used, in credits.
    pub meter_credits: f64,
    /// Whether `meter_credits` was supplied (true) or taken from the model (false).
    pub supplied: bool,
    /// Share of the total bet returned through this level at that value.
    pub rtp: f64,
    /// Meter value, in credits, at which the total return is 100% with the
    /// other levels unchanged. `None` when the level cannot be won.
    pub break_even_credits: Option<f64>,
}

/// Expected value of one spin given meter values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProgressiveEv {
    pub credits_per_line: u32,
    pub total_bet: u64,
    pub rtp_excluding_jackpots: f64,
    pub tiers: Vec<TierEv>,
    /// Total return per unit bet at these meter values.
    pub rtp: f64,
    /// Expected result of one spin in credits (negative is a cost).
    pub expected_credits_per_spin: f64,
}

/// Why the calculation could not be done.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProgressiveError {
    #[error("the model has no bet option {0}")]
    NoSuchBet(usize),
    #[error("the model has no Hold & Spin feature, so no jackpot levels")]
    NoJackpots,
    #[error("jackpot level {0:?} is not defined in the model")]
    UnknownTier(JackpotTier),
    #[error("meter value for {0:?} is not a non-negative number")]
    BadMeter(JackpotTier),
}

/// Expected value at bet option `bet_index` with the given meter values in
/// credits. Levels without a supplied value use the model's own value.
pub fn progressive_ev(
    model: &MathModel,
    analysis: &Analysis,
    bet_index: usize,
    meters: &[(JackpotTier, f64)],
) -> Result<ProgressiveEv, ProgressiveError> {
    let credits_per_line = *model
        .bet
        .credits_per_line_options
        .get(bet_index)
        .ok_or(ProgressiveError::NoSuchBet(bet_index))?;
    let total_bet = u64::from(credits_per_line) * model.paylines.len() as u64;
    let feature = analysis
        .feature
        .as_ref()
        .ok_or(ProgressiveError::NoJackpots)?;
    for (tier, value) in meters {
        if !feature.jackpots.iter().any(|j| j.tier == *tier) {
            return Err(ProgressiveError::UnknownTier(*tier));
        }
        if !value.is_finite() || *value < 0.0 {
            return Err(ProgressiveError::BadMeter(*tier));
        }
    }

    let bet = total_bet as f64;
    let rtp_excluding_jackpots = analysis.rtp_excluding_jackpots();
    let mut tiers: Vec<TierEv> = feature
        .jackpots
        .iter()
        .map(|j| {
            let supplied = meters.iter().find(|(t, _)| *t == j.tier).map(|(_, v)| *v);
            let meter_credits = supplied.unwrap_or(f64::from(j.total_bet_multiple) * bet);
            TierEv {
                tier: j.tier,
                awards_per_spin: j.awards_per_spin,
                meter_credits,
                supplied: supplied.is_some(),
                rtp: j.awards_per_spin * meter_credits / bet,
                break_even_credits: None,
            }
        })
        .collect();
    let rtp = rtp_excluding_jackpots + tiers.iter().map(|t| t.rtp).sum::<f64>();
    for tier in &mut tiers {
        if tier.awards_per_spin > 0.0 {
            let others = rtp - tier.rtp;
            tier.break_even_credits = Some(((1.0 - others) * bet / tier.awards_per_spin).max(0.0));
        }
    }
    Ok(ProgressiveEv {
        credits_per_line,
        total_bet,
        rtp_excluding_jackpots,
        tiers,
        rtp,
        expected_credits_per_spin: (rtp - 1.0) * bet,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::analyze;
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    fn setup() -> (MathModel, Analysis) {
        let model = synthetic_dev_fixture();
        let analysis = analyze(&model, 0).unwrap();
        (model, analysis)
    }

    #[test]
    fn with_no_meters_supplied_the_return_is_the_models_own() {
        let (model, analysis) = setup();
        let ev = progressive_ev(&model, &analysis, 0, &[]).unwrap();
        assert_eq!(ev.total_bet, 5);
        assert!((ev.rtp - analysis.total_rtp).abs() < 1e-12);
        assert!(ev.tiers.iter().all(|t| !t.supplied));
        // The same at a larger bet: the fixture's levels scale with the bet.
        let big = progressive_ev(&model, &analysis, 3, &[]).unwrap();
        assert_eq!(big.total_bet, 50);
        assert!((big.rtp - ev.rtp).abs() < 1e-12);
    }

    #[test]
    fn a_larger_meter_raises_the_return_in_proportion_to_its_frequency() {
        let (model, analysis) = setup();
        let base = progressive_ev(&model, &analysis, 0, &[]).unwrap();
        let grand = base
            .tiers
            .iter()
            .find(|t| t.tier == JackpotTier::Grand)
            .unwrap();
        let doubled = grand.meter_credits * 2.0;
        let ev = progressive_ev(&model, &analysis, 0, &[(JackpotTier::Grand, doubled)]).unwrap();
        let expected = base.rtp + grand.awards_per_spin * grand.meter_credits / 5.0;
        assert!((ev.rtp - expected).abs() < 1e-12);
        assert!(
            ev.tiers
                .iter()
                .find(|t| t.tier == JackpotTier::Grand)
                .unwrap()
                .supplied
        );
    }

    #[test]
    fn at_the_break_even_meter_the_return_is_exactly_one() {
        let (model, analysis) = setup();
        let base = progressive_ev(&model, &analysis, 0, &[]).unwrap();
        for tier in &base.tiers {
            let Some(value) = tier.break_even_credits else {
                continue;
            };
            if value == 0.0 {
                continue;
            }
            let ev = progressive_ev(&model, &analysis, 0, &[(tier.tier, value)]).unwrap();
            assert!((ev.rtp - 1.0).abs() < 1e-9, "{:?}: {}", tier.tier, ev.rtp);
            assert!(ev.expected_credits_per_spin.abs() < 1e-6);
        }
    }

    #[test]
    fn bad_input_is_refused() {
        let (model, analysis) = setup();
        assert_eq!(
            progressive_ev(&model, &analysis, 9, &[]),
            Err(ProgressiveError::NoSuchBet(9))
        );
        assert_eq!(
            progressive_ev(&model, &analysis, 0, &[(JackpotTier::Grand, -1.0)]),
            Err(ProgressiveError::BadMeter(JackpotTier::Grand))
        );
        assert_eq!(
            progressive_ev(&model, &analysis, 0, &[(JackpotTier::Major, f64::NAN)]),
            Err(ProgressiveError::BadMeter(JackpotTier::Major))
        );
    }
}
