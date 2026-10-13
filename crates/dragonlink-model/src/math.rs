//! Runnable math models.
//!
//! A [`MathModel`] is everything `dragonlink-core` needs to resolve a spin:
//! window shape, symbols, reel strips, paylines, pays, bet options and the
//! Hold & Spin parameters. It is a *complete* configuration, so it can only
//! be built from evidence that is itself complete, or be synthetic. Its
//! [`MathProvenance`] says which, and travels into every trace header.
//!
//! The structure follows the generic shape of a line game with a
//! hold-and-respin feature. Which of these rules the original Dragon Link
//! titles actually follow is recorded as claims
//! (`crates/dragonlink-model/claims/`), not assumed here.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::layout::{GridShape, Payline};
use crate::symbol::{SymbolDef, SymbolId, SymbolKind};

/// Where a math model's numbers came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathProvenance {
    /// Invented by this project so the engine can be developed and tested.
    /// Not recovered from, measured on or fitted to any Aristocrat product.
    SyntheticDevFixture,
    /// Recovered from an official social-casino implementation. Valid as a
    /// description of that implementation only.
    OfficialMobile,
    /// Taken from regulatory documentation.
    Regulatory,
    /// Recovered from gaming-machine software.
    Cabinet,
}

impl MathProvenance {
    /// Whether results produced with this model may be compared with the
    /// Australian cabinet game as a statement about its mathematics.
    pub fn is_cabinet_math_evidence(self) -> bool {
        matches!(self, MathProvenance::Regulatory | MathProvenance::Cabinet)
    }

    /// The banner shown wherever a model of this provenance is used.
    pub fn banner(self) -> &'static str {
        match self {
            MathProvenance::SyntheticDevFixture => {
                "SYNTHETIC DEVELOPMENT MATH — not recovered from any original game"
            }
            MathProvenance::OfficialMobile => {
                "MOBILE-DERIVED MATH — not evidence of cabinet mathematics"
            }
            MathProvenance::Regulatory => "MATH FROM REGULATORY DOCUMENTATION",
            MathProvenance::Cabinet => "MATH RECOVERED FROM CABINET SOFTWARE",
        }
    }
}

/// A probability as an exact fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ratio {
    pub num: u32,
    pub den: u32,
}

/// Pay for a run of `count` of `symbol` on a line, in credits per credit bet on that line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinePay {
    pub symbol: SymbolId,
    pub count: u8,
    pub pay: u32,
}

/// Pay for `count` of `symbol` anywhere in the window, as a multiple of the total bet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterPay {
    pub symbol: SymbolId,
    pub count: u8,
    pub total_bet_multiple: u32,
}

/// Bet options. The player chooses credits per line and, where the game
/// offers it, how many paylines to play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetModel {
    pub credits_per_line_options: Vec<u32>,
    /// The numbers of paylines the player may play, ascending. Playing `n`
    /// lines plays the first `n` paylines of the model. Empty means every
    /// payline is always played.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_options: Vec<u16>,
}

/// Jackpot levels by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JackpotTier {
    Mini,
    Minor,
    Major,
    Grand,
}

/// A jackpot level and what it pays. Only fixed amounts are modelled so far;
/// progressive meters are not (see `progress/progress.toml`, `runtime.jackpots`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JackpotDef {
    pub tier: JackpotTier,
    /// Award as a multiple of the total bet of the triggering spin.
    pub total_bet_multiple: u32,
}

/// What a Hold & Spin prize symbol carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbValue {
    /// A credit prize, as a multiple of the total bet of the triggering spin.
    BetMultiple(u32),
    /// A jackpot level.
    Jackpot(JackpotTier),
}

/// One entry of the table an orb's value is drawn from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedOrbValue {
    pub weight: u32,
    pub value: OrbValue,
}

/// Hold & Spin parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldAndSpinModel {
    /// The prize symbol. Must be of kind [`SymbolKind::Orb`].
    pub orb: SymbolId,
    /// Orbs in the window that start the feature.
    pub trigger_count: u8,
    /// Respins the feature starts with.
    pub initial_respins: u8,
    /// Whether landing a new orb puts the respin counter back to `initial_respins`.
    pub reset_respins_on_new_orb: bool,
    /// Chance that an empty position lands an orb on one respin.
    pub orb_land_chance: Ratio,
    /// Table an orb's value is drawn from, in the base game and in the feature.
    pub orb_values: Vec<WeightedOrbValue>,
    /// Jackpot awarded in addition when every position holds an orb.
    pub full_grid_award: Option<JackpotTier>,
    pub jackpots: Vec<JackpotDef>,
}

/// One weighted choice of a symbol, for the free-games reveal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedSymbol {
    pub weight: u32,
    pub symbol: SymbolId,
}

/// Free games: a number of spins played without a bet, on their own reels,
/// started by scatters in the base window, with an optional retrigger and an
/// optional mystery-symbol reveal (all mystery positions become one drawn
/// symbol, as Golden Century's free games do).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeGamesModel {
    /// Scatters in the base window that start the free games.
    pub trigger_count: u8,
    /// Free games awarded at the start.
    pub initial_games: u8,
    /// Scatters during a free game that add more games. 0 means no retrigger.
    pub retrigger_count: u8,
    /// Free games added on a retrigger.
    pub retrigger_games: u8,
    /// One strip per reel, played during the free games.
    pub free_reel_strips: Vec<Vec<SymbolId>>,
    /// A symbol in the free reels that, when it appears, reveals to one drawn
    /// symbol across all of its positions. `None` for no reveal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mystery: Option<SymbolId>,
    /// What the mystery reveals to, weighted. Required when `mystery` is set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reveal_values: Vec<WeightedSymbol>,
}

/// A complete, runnable game configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathModel {
    /// Stable identifier written into traces.
    pub id: String,
    pub provenance: MathProvenance,
    pub description: String,
    pub grid: GridShape,
    pub symbols: Vec<SymbolDef>,
    /// One strip per reel; the window shows `grid.rows` consecutive stops, wrapping.
    pub reel_strips: Vec<Vec<SymbolId>>,
    pub paylines: Vec<Payline>,
    pub line_pays: Vec<LinePay>,
    pub scatter_pays: Vec<ScatterPay>,
    pub bet: BetModel,
    pub hold_and_spin: Option<HoldAndSpinModel>,
    /// Free games (scatter triggered), when the game has them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_games: Option<FreeGamesModel>,
}

/// Why a [`MathModel`] cannot be run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    #[error("the window has no positions")]
    EmptyGrid,
    #[error("symbol at position {index} has id {id}; ids must equal their position")]
    SymbolIdMismatch { index: usize, id: u16 },
    #[error("symbol key {0:?} is used twice")]
    DuplicateSymbolKey(String),
    #[error(
        "line options must be ascending numbers of paylines, each at least 1 and no more than the model has"
    )]
    BadLineOptions,
    #[error("{reels} reels but {strips} reel strips")]
    StripCount { reels: u8, strips: usize },
    #[error("reel strip {reel} has {len} stops; at least the {rows} visible rows are needed")]
    StripTooShort { reel: usize, len: usize, rows: u8 },
    #[error("{what} refers to symbol {id}, which is not defined")]
    UnknownSymbol { what: &'static str, id: u16 },
    #[error("payline {line} has {len} positions for {reels} reels")]
    PaylineLength { line: usize, len: usize, reels: u8 },
    #[error("payline {line} uses row {row}; the window has {rows} rows")]
    PaylineRow { line: usize, row: u8, rows: u8 },
    #[error("no paylines")]
    NoPaylines,
    #[error("line pay for symbol {id} needs a count between 1 and {reels}, got {count}")]
    LinePayCount { id: u16, count: u8, reels: u8 },
    #[error("line pay for symbol {id} × {count} is given twice")]
    DuplicateLinePay { id: u16, count: u8 },
    #[error("symbol {id} is a {kind:?} and cannot have line pays")]
    LinePayOnNonLineSymbol { id: u16, kind: SymbolKind },
    #[error("scatter pay refers to symbol {id}, which is a {kind:?}")]
    ScatterPayOnNonScatter { id: u16, kind: SymbolKind },
    #[error("no bet options")]
    NoBetOptions,
    #[error("a bet option of 0 credits per line")]
    ZeroBetOption,
    #[error("Hold & Spin symbol {id} is a {kind:?}, not an orb")]
    HoldAndSpinSymbolNotOrb { id: u16, kind: SymbolKind },
    #[error("Hold & Spin trigger count {count} is outside 1..={cells}")]
    TriggerCount { count: u8, cells: usize },
    #[error("Hold & Spin starts with 0 respins")]
    NoRespins,
    #[error("free games need a scatter symbol to be triggered by, and the model has none")]
    FreeGamesWithoutScatter,
    #[error("free games: {0}")]
    BadFreeGames(&'static str),
    #[error("orb land chance {num}/{den} is not a probability")]
    BadChance { num: u32, den: u32 },
    #[error("the orb value table is empty or has no weight")]
    NoOrbValues,
    #[error("jackpot {0:?} is awarded but not defined")]
    UndefinedJackpot(JackpotTier),
    #[error("jackpot {0:?} is defined twice")]
    DuplicateJackpot(JackpotTier),
}

impl MathModel {
    /// The definition of `id`.
    pub fn symbol(&self, id: SymbolId) -> Option<&SymbolDef> {
        self.symbols.get(id.0 as usize)
    }

    /// The kind of `id`, if defined.
    pub fn kind(&self, id: SymbolId) -> Option<SymbolKind> {
        self.symbol(id).map(|s| s.kind)
    }

    /// Line pay for a run of `count` of `symbol`, 0 when there is none.
    pub fn line_pay(&self, symbol: SymbolId, count: u8) -> u32 {
        self.line_pays
            .iter()
            .find(|p| p.symbol == symbol && p.count == count)
            .map_or(0, |p| p.pay)
    }

    /// The first scatter symbol, if any. Scatters start the free games.
    pub fn scatter_symbol(&self) -> Option<SymbolId> {
        self.symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Scatter)
            .map(|s| s.id)
    }

    /// The numbers of paylines the player may play, ascending: the model's
    /// line options, or only "all of them" when it has none.
    pub fn line_counts(&self) -> Vec<u16> {
        if self.bet.line_options.is_empty() {
            vec![u16::try_from(self.paylines.len()).unwrap_or(u16::MAX)]
        } else {
            self.bet.line_options.clone()
        }
    }

    /// Check that the model is internally consistent and can be run.
    /// This checks *our* configuration; it says nothing about the original.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.grid.cells() == 0 {
            return Err(ModelError::EmptyGrid);
        }
        let mut keys = BTreeSet::new();
        for (index, s) in self.symbols.iter().enumerate() {
            if s.id.0 as usize != index {
                return Err(ModelError::SymbolIdMismatch { index, id: s.id.0 });
            }
            if !keys.insert(s.key.as_str()) {
                return Err(ModelError::DuplicateSymbolKey(s.key.clone()));
            }
        }
        let defined = |what: &'static str, id: SymbolId| {
            self.kind(id)
                .ok_or(ModelError::UnknownSymbol { what, id: id.0 })
        };

        if self.reel_strips.len() != self.grid.reels as usize {
            return Err(ModelError::StripCount {
                reels: self.grid.reels,
                strips: self.reel_strips.len(),
            });
        }
        for (reel, strip) in self.reel_strips.iter().enumerate() {
            if strip.len() < self.grid.rows as usize {
                return Err(ModelError::StripTooShort {
                    reel,
                    len: strip.len(),
                    rows: self.grid.rows,
                });
            }
            for id in strip {
                defined("a reel strip", *id)?;
            }
        }

        if self.paylines.is_empty() {
            return Err(ModelError::NoPaylines);
        }
        for (line, p) in self.paylines.iter().enumerate() {
            if p.0.len() != self.grid.reels as usize {
                return Err(ModelError::PaylineLength {
                    line,
                    len: p.0.len(),
                    reels: self.grid.reels,
                });
            }
            if let Some(row) = p.0.iter().copied().find(|r| *r >= self.grid.rows) {
                return Err(ModelError::PaylineRow {
                    line,
                    row,
                    rows: self.grid.rows,
                });
            }
        }

        let mut seen = BTreeSet::new();
        for p in &self.line_pays {
            let kind = defined("a line pay", p.symbol)?;
            if !matches!(kind, SymbolKind::Regular | SymbolKind::Wild) {
                return Err(ModelError::LinePayOnNonLineSymbol {
                    id: p.symbol.0,
                    kind,
                });
            }
            if p.count == 0 || p.count > self.grid.reels {
                return Err(ModelError::LinePayCount {
                    id: p.symbol.0,
                    count: p.count,
                    reels: self.grid.reels,
                });
            }
            if !seen.insert((p.symbol, p.count)) {
                return Err(ModelError::DuplicateLinePay {
                    id: p.symbol.0,
                    count: p.count,
                });
            }
        }
        for p in &self.scatter_pays {
            let kind = defined("a scatter pay", p.symbol)?;
            if kind != SymbolKind::Scatter {
                return Err(ModelError::ScatterPayOnNonScatter {
                    id: p.symbol.0,
                    kind,
                });
            }
        }

        if self.bet.credits_per_line_options.is_empty() {
            return Err(ModelError::NoBetOptions);
        }
        if self.bet.credits_per_line_options.contains(&0) {
            return Err(ModelError::ZeroBetOption);
        }
        let lines = &self.bet.line_options;
        if lines.first() == Some(&0)
            || lines
                .last()
                .is_some_and(|n| *n as usize > self.paylines.len())
            || lines.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ModelError::BadLineOptions);
        }

        if let Some(hs) = &self.hold_and_spin {
            let kind = defined("Hold & Spin", hs.orb)?;
            if kind != SymbolKind::Orb {
                return Err(ModelError::HoldAndSpinSymbolNotOrb { id: hs.orb.0, kind });
            }
            if hs.trigger_count == 0 || hs.trigger_count as usize > self.grid.cells() {
                return Err(ModelError::TriggerCount {
                    count: hs.trigger_count,
                    cells: self.grid.cells(),
                });
            }
            if hs.initial_respins == 0 {
                return Err(ModelError::NoRespins);
            }
            let c = hs.orb_land_chance;
            if c.den == 0 || c.num > c.den {
                return Err(ModelError::BadChance {
                    num: c.num,
                    den: c.den,
                });
            }
            if hs.orb_values.iter().map(|v| v.weight as u64).sum::<u64>() == 0 {
                return Err(ModelError::NoOrbValues);
            }
            let mut tiers = BTreeSet::new();
            for j in &hs.jackpots {
                if !tiers.insert(j.tier) {
                    return Err(ModelError::DuplicateJackpot(j.tier));
                }
            }
            let awarded = hs
                .orb_values
                .iter()
                .filter_map(|v| match v.value {
                    OrbValue::Jackpot(t) => Some(t),
                    OrbValue::BetMultiple(_) => None,
                })
                .chain(hs.full_grid_award);
            for t in awarded {
                if !tiers.contains(&t) {
                    return Err(ModelError::UndefinedJackpot(t));
                }
            }
        }

        if let Some(fg) = &self.free_games {
            if self.scatter_symbol().is_none() {
                return Err(ModelError::FreeGamesWithoutScatter);
            }
            if fg.trigger_count == 0 || fg.initial_games == 0 {
                return Err(ModelError::BadFreeGames(
                    "the trigger count and the initial games must be at least 1",
                ));
            }
            if fg.retrigger_count == 0 && fg.retrigger_games != 0 {
                return Err(ModelError::BadFreeGames(
                    "games are added on a retrigger but no scatter count triggers one",
                ));
            }
            if fg.free_reel_strips.len() != self.grid.reels as usize {
                return Err(ModelError::StripCount {
                    reels: self.grid.reels,
                    strips: fg.free_reel_strips.len(),
                });
            }
            for (reel, strip) in fg.free_reel_strips.iter().enumerate() {
                if strip.len() < self.grid.rows as usize {
                    return Err(ModelError::StripTooShort {
                        reel,
                        len: strip.len(),
                        rows: self.grid.rows,
                    });
                }
                for id in strip {
                    defined("a free-game strip", *id)?;
                }
            }
            match (fg.mystery, fg.reveal_values.is_empty()) {
                (Some(m), _) => {
                    defined("the mystery symbol", m)?;
                    if fg
                        .reveal_values
                        .iter()
                        .map(|v| v.weight as u64)
                        .sum::<u64>()
                        == 0
                    {
                        return Err(ModelError::BadFreeGames(
                            "a mystery symbol is set but nothing is weighted for it to reveal",
                        ));
                    }
                    for v in &fg.reveal_values {
                        defined("a reveal value", v.symbol)?;
                    }
                }
                (None, false) => {
                    return Err(ModelError::BadFreeGames(
                        "reveal values are given but no mystery symbol uses them",
                    ));
                }
                (None, true) => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::synthetic_dev_fixture;

    #[test]
    fn only_cabinet_and_regulatory_math_is_cabinet_evidence() {
        assert!(!MathProvenance::SyntheticDevFixture.is_cabinet_math_evidence());
        assert!(!MathProvenance::OfficialMobile.is_cabinet_math_evidence());
        assert!(MathProvenance::Regulatory.is_cabinet_math_evidence());
        assert!(MathProvenance::Cabinet.is_cabinet_math_evidence());
        assert!(
            MathProvenance::SyntheticDevFixture
                .banner()
                .contains("SYNTHETIC")
        );
    }

    #[test]
    fn validation_rejects_broken_models() {
        let good = synthetic_dev_fixture();
        assert_eq!(good.validate(), Ok(()));

        let mut m = good.clone();
        m.reel_strips.pop();
        assert!(matches!(m.validate(), Err(ModelError::StripCount { .. })));

        let mut m = good.clone();
        m.reel_strips[0] = vec![SymbolId(0)];
        assert!(matches!(
            m.validate(),
            Err(ModelError::StripTooShort { .. })
        ));

        let mut m = good.clone();
        m.reel_strips[2][0] = SymbolId(999);
        assert!(matches!(
            m.validate(),
            Err(ModelError::UnknownSymbol { .. })
        ));

        let mut m = good.clone();
        m.paylines[0].0[1] = 9;
        assert!(matches!(m.validate(), Err(ModelError::PaylineRow { .. })));

        let mut m = good.clone();
        m.paylines[0].0.pop();
        assert!(matches!(
            m.validate(),
            Err(ModelError::PaylineLength { .. })
        ));

        let mut m = good.clone();
        m.bet.credits_per_line_options = vec![1, 0];
        assert_eq!(m.validate(), Err(ModelError::ZeroBetOption));

        let mut m = good.clone();
        let dup = m.line_pays[0].clone();
        m.line_pays.push(dup);
        assert!(matches!(
            m.validate(),
            Err(ModelError::DuplicateLinePay { .. })
        ));

        let mut m = good.clone();
        if let Some(hs) = &mut m.hold_and_spin {
            hs.orb_land_chance = Ratio { num: 3, den: 2 };
        }
        assert!(matches!(m.validate(), Err(ModelError::BadChance { .. })));

        let mut m = good.clone();
        if let Some(hs) = &mut m.hold_and_spin {
            hs.jackpots.clear();
        }
        assert!(matches!(m.validate(), Err(ModelError::UndefinedJackpot(_))));

        let mut m = good.clone();
        if let Some(hs) = &mut m.hold_and_spin {
            hs.trigger_count = 16;
        }
        assert!(matches!(m.validate(), Err(ModelError::TriggerCount { .. })));

        let mut m = good;
        if let Some(hs) = &mut m.hold_and_spin {
            hs.orb = SymbolId(0);
        }
        assert!(matches!(
            m.validate(),
            Err(ModelError::HoldAndSpinSymbolNotOrb { .. })
        ));
    }
}
