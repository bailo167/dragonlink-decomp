//! Sources of outcomes.
//!
//! **None of this models Aristocrat's random number generator.** Nothing is
//! known about the original's generator or about how it maps numbers to reel
//! stops (`claims/family.toml`, `math.rng`). The types here exist so that
//! *our* simulation can be tested:
//!
//! - [`DevRng`]: a small seeded generator (SplitMix64) for deterministic
//!   tests and statistical runs of our own code.
//! - [`RecordingRng`]: wraps any source and logs every draw.
//! - [`ScriptedRng`]: replays a log, so a recorded game can be re-run
//!   exactly and a hand-written scenario can force specific outcomes.
//!
//! If the original's generator is ever established from evidence, it becomes
//! another implementation of [`OutcomeRng`], with its provenance stated.

use serde::{Deserialize, Serialize};

/// Why a draw failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RngError {
    #[error("a draw below 0 was requested")]
    ZeroBound,
    #[error("the scripted draws ran out at draw {position}")]
    ScriptExhausted { position: usize },
    #[error(
        "scripted draw {position} was recorded with bound {recorded} but bound {requested} was requested"
    )]
    ScriptBoundMismatch {
        position: usize,
        recorded: u32,
        requested: u32,
    },
    #[error("scripted draw {position} is {value}, which is not below {bound}")]
    ScriptOutOfRange {
        position: usize,
        value: u32,
        bound: u32,
    },
}

/// One draw: a uniform integer `value` in `0..bound`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Draw {
    pub bound: u32,
    pub value: u32,
}

/// A source of outcomes. The simulation asks only for uniform integers below
/// a bound, in a documented order (see [`crate::game`]).
pub trait OutcomeRng {
    /// A uniform integer in `0..bound`.
    fn below(&mut self, bound: u32) -> Result<u32, RngError>;

    /// A short name written into trace headers.
    fn name(&self) -> &'static str;
}

/// Seeded test generator (SplitMix64 with unbiased bounded sampling).
/// Ours, for testing our implementation. Not the original's generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevRng {
    state: u64,
}

impl DevRng {
    /// Name written into trace headers.
    pub const NAME: &'static str = "dev-splitmix64";

    pub fn new(seed: u64) -> Self {
        DevRng { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

impl OutcomeRng for DevRng {
    fn below(&mut self, bound: u32) -> Result<u32, RngError> {
        if bound == 0 {
            return Err(RngError::ZeroBound);
        }
        // Multiply-shift with rejection (Lemire), so every value is equally likely.
        let mut m = u64::from(self.next_u32()) * u64::from(bound);
        if (m as u32) < bound {
            let threshold = bound.wrapping_neg() % bound;
            while (m as u32) < threshold {
                m = u64::from(self.next_u32()) * u64::from(bound);
            }
        }
        Ok((m >> 32) as u32)
    }

    fn name(&self) -> &'static str {
        DevRng::NAME
    }
}

/// Wraps a source and records every draw it hands out.
#[derive(Debug, Clone)]
pub struct RecordingRng<R> {
    inner: R,
    log: Vec<Draw>,
}

impl<R: OutcomeRng> RecordingRng<R> {
    pub fn new(inner: R) -> Self {
        RecordingRng {
            inner,
            log: Vec::new(),
        }
    }

    /// The draws made so far, in order.
    pub fn log(&self) -> &[Draw] {
        &self.log
    }

    /// The draws made so far, leaving the log empty.
    pub fn take_log(&mut self) -> Vec<Draw> {
        std::mem::take(&mut self.log)
    }
}

impl<R: OutcomeRng> OutcomeRng for RecordingRng<R> {
    fn below(&mut self, bound: u32) -> Result<u32, RngError> {
        let value = self.inner.below(bound)?;
        self.log.push(Draw { bound, value });
        Ok(value)
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// Replays recorded draws. A request whose bound differs from the recorded
/// one is an error: it means the replay has diverged from the recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedRng {
    draws: Vec<Draw>,
    position: usize,
}

impl ScriptedRng {
    /// Name written into trace headers.
    pub const NAME: &'static str = "scripted";

    pub fn new(draws: Vec<Draw>) -> Self {
        ScriptedRng { draws, position: 0 }
    }

    /// Draws not yet consumed.
    pub fn remaining(&self) -> usize {
        self.draws.len() - self.position
    }
}

impl OutcomeRng for ScriptedRng {
    fn below(&mut self, bound: u32) -> Result<u32, RngError> {
        if bound == 0 {
            return Err(RngError::ZeroBound);
        }
        let position = self.position;
        let draw = self
            .draws
            .get(position)
            .copied()
            .ok_or(RngError::ScriptExhausted { position })?;
        if draw.bound != bound {
            return Err(RngError::ScriptBoundMismatch {
                position,
                recorded: draw.bound,
                requested: bound,
            });
        }
        if draw.value >= bound {
            return Err(RngError::ScriptOutOfRange {
                position,
                value: draw.value,
                bound,
            });
        }
        self.position += 1;
        Ok(draw.value)
    }

    fn name(&self) -> &'static str {
        ScriptedRng::NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_rng_is_deterministic_and_in_range() {
        let mut a = DevRng::new(42);
        let mut b = DevRng::new(42);
        for bound in [1u32, 2, 3, 7, 24, 100, 1 << 31, u32::MAX] {
            for _ in 0..200 {
                let x = a.below(bound).unwrap();
                assert_eq!(x, b.below(bound).unwrap());
                assert!(x < bound);
            }
        }
        assert_ne!(
            DevRng::new(1).below(u32::MAX).unwrap(),
            DevRng::new(2).below(u32::MAX).unwrap()
        );
    }

    #[test]
    fn dev_rng_matches_the_published_splitmix64_sequence() {
        // First outputs of SplitMix64 from seed 0 (reference implementation by Vigna).
        let mut r = DevRng::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn dev_rng_covers_a_small_range_evenly() {
        // A sanity check of our sampling, not a statement about any real machine.
        let mut r = DevRng::new(7);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[r.below(6).unwrap() as usize] += 1;
        }
        for c in counts {
            assert!((9_500..=10_500).contains(&c), "{counts:?}");
        }
    }

    #[test]
    fn zero_bound_is_an_error() {
        assert_eq!(DevRng::new(1).below(0), Err(RngError::ZeroBound));
        assert_eq!(ScriptedRng::new(vec![]).below(0), Err(RngError::ZeroBound));
    }

    #[test]
    fn a_recording_replays_exactly() {
        let mut rec = RecordingRng::new(DevRng::new(99));
        let bounds = [24u32, 24, 24, 24, 24, 100, 12, 12, 100];
        let values: Vec<u32> = bounds.iter().map(|b| rec.below(*b).unwrap()).collect();
        assert_eq!(rec.name(), DevRng::NAME);
        let mut replay = ScriptedRng::new(rec.log().to_vec());
        let replayed: Vec<u32> = bounds.iter().map(|b| replay.below(*b).unwrap()).collect();
        assert_eq!(values, replayed);
        assert_eq!(replay.remaining(), 0);
        assert_eq!(
            replay.below(5),
            Err(RngError::ScriptExhausted { position: 9 })
        );
    }

    #[test]
    fn a_diverging_replay_is_detected() {
        let mut s = ScriptedRng::new(vec![Draw {
            bound: 24,
            value: 3,
        }]);
        assert_eq!(
            s.below(12),
            Err(RngError::ScriptBoundMismatch {
                position: 0,
                recorded: 24,
                requested: 12
            })
        );
        // The failed request consumed nothing.
        assert_eq!(s.below(24), Ok(3));
        let mut bad = ScriptedRng::new(vec![Draw { bound: 4, value: 9 }]);
        assert_eq!(
            bad.below(4),
            Err(RngError::ScriptOutOfRange {
                position: 0,
                value: 9,
                bound: 4
            })
        );
    }
}
