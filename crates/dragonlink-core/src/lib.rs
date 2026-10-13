//! Deterministic, render-free game simulation.
//!
//! This crate turns commands ([`Command`]) into events ([`Event`]) under a
//! [`dragonlink_model::MathModel`] and an injected source of outcomes
//! ([`rng::OutcomeRng`]). It knows nothing about windows, textures, sound,
//! files or time: the renderer reads its state and events and never decides
//! an outcome. It has no platform dependencies and compiles for WASM.
//!
//! What it is not:
//!
//! - It is not a model of Aristocrat's random number generator. The
//!   generators in [`rng`] are test instruments (see that module).
//! - It is not evidence of the original's mathematics. Every result depends
//!   on the math model it is given, and the only model that exists today is
//!   a synthetic development fixture.
//!
//! The rules implemented here are the generic rules of a line game with a
//! hold-and-respin feature. Which of them the original titles follow is
//! tracked claim by claim in `crates/dragonlink-model/claims/`.

pub mod eval;
pub mod event;
pub mod game;
pub mod grid;
pub mod rng;

pub use eval::{Win, evaluate};
pub use event::{CreditReason, Event, Feature, LockedOrb};
pub use game::{Command, Game, GameError, GameState, HoldAndSpinState, Phase};
pub use grid::Grid;
pub use rng::{DevRng, Draw, OutcomeRng, RecordingRng, RngError, ScriptedRng};
