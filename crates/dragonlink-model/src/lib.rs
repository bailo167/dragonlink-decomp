//! Dragon Link title data model.
//!
//! Two kinds of data live here and are deliberately kept apart:
//!
//! - **Claims** ([`claims`]): what is known, suspected or unknown about the
//!   original games, one statement at a time, each with a confidence label,
//!   a source class and a citation ([`evidence`]). Claims are documentation
//!   in machine-readable form. An unknown stays unknown.
//! - **Math models** ([`math`]): complete, runnable configurations for the
//!   simulation in `dragonlink-core`. Every model carries a
//!   [`math::MathProvenance`]. The only model that exists today is
//!   [`fixtures::synthetic_dev_fixture`], which is ours, invented for engine
//!   development, and is not evidence of anything about the original games.
//!
//! Nothing in this crate fills a gap in the claims with a plausible value so
//! that a model can run. A model is either recovered from evidence or
//! labelled synthetic.

pub mod claims;
pub mod evidence;
pub mod fixtures;
pub mod layout;
pub mod math;
pub mod symbol;
pub mod title;

pub use evidence::{Confidence, SourceClass, Sourced};
pub use layout::{GridShape, Payline};
pub use math::{
    BetModel, BigReel, FixedPrize, FreeGamesModel, HoldAndSpinModel, JackpotDef, JackpotTier,
    LinePay, MathModel, MathProvenance, ModelError, OrbValue, Ratio, ScatterPay, WeightedOrbValue,
    WeightedSymbol,
};
pub use symbol::{SymbolDef, SymbolId, SymbolKind};
pub use title::TitleId;
