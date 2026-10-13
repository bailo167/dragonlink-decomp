//! Game mathematics analysis.
//!
//! Given a [`config::GameConfiguration`] (a complete math model plus the
//! provenance of every constant in it) this crate answers quantitative
//! questions about the game it describes:
//!
//! - [`exact`]: return to player and its composition, feature trigger
//!   probability, the Hold & Spin outcome distribution and jackpot
//!   frequencies, computed from the configuration's distributions, and hit
//!   frequency and variance by full enumeration where that is tractable;
//! - [`sim`]: a seeded Monte Carlo run of the real state machine with
//!   confidence intervals, to cross-check the exact results and to cover what
//!   cannot be enumerated;
//! - [`progressive`]: expected value as a function of jackpot meter values;
//! - [`compare`]: a structural and quantitative diff of two configurations;
//! - [`reconstruct`]: a reel rebuilt from the windows it was observed to stop on;
//! - [`report`]: the per-game technical report.
//!
//! **What a result means depends entirely on the configuration.** The only
//! configuration that exists today is the synthetic development fixture, and
//! every number computed from it is a property of that fixture, not of any
//! Dragon Link game. Reports say so on their first line. See
//! `docs/reverse-engineering/math/`.
//!
//! Exact analysis and simulation are independent paths to the same
//! quantities: one works from distributions, the other samples the state
//! machine in `dragonlink-core`. The tests require them to agree.

pub mod compare;
pub mod config;
pub mod exact;
pub mod overlap;
pub mod progressive;
pub mod reconstruct;
pub mod report;
pub mod sim;

pub use config::{
    Behaviour, ConfigError, ConfigIdentity, ConstantProvenance, ConstantRecord, GameConfiguration,
};
pub use exact::{Analysis, analyze};
