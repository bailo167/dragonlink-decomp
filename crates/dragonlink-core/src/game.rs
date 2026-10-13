//! The game state machine.
//!
//! ```text
//!            Spin                       Advance (repeat)
//!   Idle ───────────► Idle                    ┌───────┐
//!     │                                       ▼       │
//!     └── Spin, trigger met ──► HoldAndSpin ──┴──► Idle
//! ```
//!
//! A command either succeeds, changing the state and returning the events it
//! caused, or fails and leaves the state untouched.
//!
//! # Draw order
//!
//! The simulation asks its [`OutcomeRng`] for uniform integers in this
//! order, which is part of the contract a recorded game replays against:
//!
//! - **Spin**: one draw per reel, left to right, below the strip length (the
//!   top visible stop); then, for each orb in the window in reel-major
//!   order, one draw below the value table's total weight.
//! - **Advance**: for each empty position in reel-major order, one draw
//!   below the land chance's denominator (an orb lands when the draw is
//!   below the numerator), followed, if it landed, by one value draw.
//!
//! This order is ours. Nothing is known about the order, or the generator,
//! of the original.

use dragonlink_model::{
    HoldAndSpinModel, JackpotTier, MathModel, ModelError, OrbValue, SymbolId, SymbolKind,
};
use serde::{Deserialize, Serialize};

use crate::eval::{Win, evaluate};
use crate::event::{CreditReason, Event, Feature, LockedOrb};
use crate::grid::Grid;
use crate::rng::{OutcomeRng, RngError};

/// What the player can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Choose a bet option by index. Only while idle.
    SetBet { index: usize },
    /// Place the bet and spin. Only while idle.
    Spin,
    /// Play the next step of the feature in progress.
    Advance,
}

/// Why a command was refused. The state is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GameError {
    #[error("the math model is not runnable: {0}")]
    Model(#[from] ModelError),
    #[error("outcome source failed: {0}")]
    Rng(#[from] RngError),
    #[error("a feature is in progress")]
    FeatureInProgress,
    #[error("no feature is in progress")]
    NoFeatureInProgress,
    #[error("the bet is {needed} credits but only {available} are available")]
    InsufficientCredits { needed: u64, available: u64 },
    #[error("there is no bet option {index}")]
    NoSuchBet { index: usize },
    #[error("an amount does not fit in 64 bits")]
    Overflow,
    #[error("internal invariant broken: {0}")]
    Invariant(&'static str),
}

/// Hold & Spin in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldAndSpinState {
    /// Held orbs by position, reel-major. `None` is an empty position.
    pub cells: Vec<Option<OrbValue>>,
    pub respins_left: u8,
    pub respins_played: u32,
    /// Total bet of the triggering spin; orb values are multiples of it.
    pub total_bet: u64,
    /// What the triggering spin paid before the feature.
    pub base_award: u64,
}

impl HoldAndSpinState {
    pub fn locked(&self) -> usize {
        self.cells.iter().flatten().count()
    }

    pub fn is_full(&self) -> bool {
        self.cells.iter().all(Option::is_some)
    }
}

/// Where the game is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    HoldAndSpin(HoldAndSpinState),
}

/// Everything that changes while playing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    /// Play credits. Simulation credits only; they stand for no money.
    pub credits: u64,
    pub bet_index: usize,
    pub phase: Phase,
    /// Spins started so far.
    pub spins: u64,
    /// The window left by the last spin.
    pub last_grid: Option<Grid>,
}

/// A game: a math model and the state of play under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    model: MathModel,
    state: GameState,
}

fn add(a: u64, b: u64) -> Result<u64, GameError> {
    a.checked_add(b).ok_or(GameError::Overflow)
}

fn draw_orb_value(hs: &HoldAndSpinModel, rng: &mut dyn OutcomeRng) -> Result<OrbValue, GameError> {
    let total: u64 = hs.orb_values.iter().map(|v| u64::from(v.weight)).sum();
    let total = u32::try_from(total).map_err(|_| GameError::Overflow)?;
    let mut pick = rng.below(total)?;
    for entry in &hs.orb_values {
        if pick < entry.weight {
            return Ok(entry.value);
        }
        pick -= entry.weight;
    }
    Err(GameError::Invariant("orb value draw past the table"))
}

fn jackpot_award(
    hs: &HoldAndSpinModel,
    tier: JackpotTier,
    total_bet: u64,
) -> Result<u64, GameError> {
    let def = hs
        .jackpots
        .iter()
        .find(|j| j.tier == tier)
        .ok_or(GameError::Model(ModelError::UndefinedJackpot(tier)))?;
    u64::from(def.total_bet_multiple)
        .checked_mul(total_bet)
        .ok_or(GameError::Overflow)
}

impl Game {
    /// A new game under `model` with `credits` play credits, idle, on the first bet option.
    pub fn new(model: MathModel, credits: u64) -> Result<Game, GameError> {
        model.validate()?;
        Ok(Game {
            model,
            state: GameState {
                credits,
                bet_index: 0,
                phase: Phase::Idle,
                spins: 0,
                last_grid: None,
            },
        })
    }

    /// Resume a game from a saved state.
    pub fn resume(model: MathModel, state: GameState) -> Result<Game, GameError> {
        model.validate()?;
        if state.bet_index >= model.bet.credits_per_line_options.len() {
            return Err(GameError::NoSuchBet {
                index: state.bet_index,
            });
        }
        if let Phase::HoldAndSpin(hs) = &state.phase
            && hs.cells.len() != model.grid.cells()
        {
            return Err(GameError::Invariant(
                "saved feature does not fit the window",
            ));
        }
        Ok(Game { model, state })
    }

    pub fn model(&self) -> &MathModel {
        &self.model
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn credits(&self) -> u64 {
        self.state.credits
    }

    pub fn is_idle(&self) -> bool {
        self.state.phase == Phase::Idle
    }

    /// Credits bet on each line at the current bet option.
    pub fn credits_per_line(&self) -> u32 {
        self.model
            .bet
            .credits_per_line_options
            .get(self.state.bet_index)
            .copied()
            .unwrap_or(0)
    }

    /// Credits one spin costs at the current bet option.
    pub fn total_bet(&self) -> u64 {
        u64::from(self.credits_per_line()) * self.model.paylines.len() as u64
    }

    fn key(&self, id: SymbolId) -> String {
        self.model
            .symbol(id)
            .map_or_else(|| format!("#{}", id.0), |s| s.key.clone())
    }

    /// Carry out `command`. On success the state has changed and the events
    /// it caused are returned in order. On failure nothing has changed.
    pub fn apply(
        &mut self,
        command: Command,
        rng: &mut dyn OutcomeRng,
    ) -> Result<Vec<Event>, GameError> {
        // Work on a copy so a failure part-way (an exhausted scripted source,
        // an overflow) cannot leave a half-applied command behind.
        let mut next = self.state.clone();
        let mut events = Vec::new();
        match command {
            Command::SetBet { index } => self.set_bet(&mut next, index, &mut events)?,
            Command::Spin => self.spin(&mut next, rng, &mut events)?,
            Command::Advance => self.advance(&mut next, rng, &mut events)?,
        }
        self.state = next;
        Ok(events)
    }

    /// Spin, then play any feature to its end. For headless runs.
    pub fn play_spin(&mut self, rng: &mut dyn OutcomeRng) -> Result<Vec<Event>, GameError> {
        let mut events = self.apply(Command::Spin, rng)?;
        while !self.is_idle() {
            events.extend(self.apply(Command::Advance, rng)?);
        }
        Ok(events)
    }

    fn set_bet(
        &self,
        next: &mut GameState,
        index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), GameError> {
        if next.phase != Phase::Idle {
            return Err(GameError::FeatureInProgress);
        }
        let credits_per_line = *self
            .model
            .bet
            .credits_per_line_options
            .get(index)
            .ok_or(GameError::NoSuchBet { index })?;
        next.bet_index = index;
        events.push(Event::BetChanged {
            credits_per_line,
            total_bet: u64::from(credits_per_line) * self.model.paylines.len() as u64,
        });
        Ok(())
    }

    fn spin(
        &self,
        next: &mut GameState,
        rng: &mut dyn OutcomeRng,
        events: &mut Vec<Event>,
    ) -> Result<(), GameError> {
        if next.phase != Phase::Idle {
            return Err(GameError::FeatureInProgress);
        }
        let credits_per_line = self.credits_per_line();
        let total_bet = self.total_bet();
        if next.credits < total_bet {
            return Err(GameError::InsufficientCredits {
                needed: total_bet,
                available: next.credits,
            });
        }
        events.push(Event::BetPlaced { total_bet });
        events.push(Event::CreditsChanged {
            from: next.credits,
            to: next.credits - total_bet,
            reason: CreditReason::Bet,
        });
        next.credits -= total_bet;
        next.spins = add(next.spins, 1)?;
        events.push(Event::SpinStarted { spin: next.spins });

        // Reels.
        let rows = self.model.grid.rows as usize;
        let mut cells = Vec::with_capacity(self.model.grid.cells());
        for (reel, strip) in self.model.reel_strips.iter().enumerate() {
            let len = u32::try_from(strip.len()).map_err(|_| GameError::Overflow)?;
            let stop = rng.below(len)?;
            let visible: Vec<SymbolId> = (0..rows)
                .map(|row| strip[(stop as usize + row) % strip.len()])
                .collect();
            events.push(Event::ReelStopped {
                reel: reel as u8,
                stop: Some(stop),
                symbols: visible.iter().map(|s| self.key(*s)).collect(),
            });
            cells.extend(visible);
        }
        let grid = Grid::new(self.model.grid, cells)
            .ok_or(GameError::Invariant("window size does not match the model"))?;

        // Orb values.
        let mut orbs: Vec<Option<OrbValue>> = vec![None; self.model.grid.cells()];
        if let Some(hs) = &self.model.hold_and_spin {
            for (index, (reel, row, symbol)) in grid.iter().enumerate() {
                if self.model.kind(symbol) == Some(SymbolKind::Orb) {
                    let value = draw_orb_value(hs, rng)?;
                    orbs[index] = Some(value);
                    events.push(Event::OrbValueShown { reel, row, value });
                }
            }
        }

        // Pays.
        let wins =
            evaluate(&self.model, &grid, credits_per_line, total_bet).ok_or(GameError::Overflow)?;
        let mut base_award = 0u64;
        for win in &wins {
            base_award = add(base_award, win.award())?;
            events.push(match win {
                Win::Line {
                    line,
                    symbol,
                    count,
                    award,
                } => Event::LineWin {
                    line: *line,
                    symbol: self.key(*symbol),
                    count: *count,
                    award: *award,
                },
                Win::Scatter {
                    symbol,
                    count,
                    award,
                } => Event::ScatterWin {
                    symbol: self.key(*symbol),
                    count: *count,
                    award: *award,
                },
            });
        }
        if base_award > 0 {
            let to = add(next.credits, base_award)?;
            events.push(Event::CreditsChanged {
                from: next.credits,
                to,
                reason: CreditReason::Win,
            });
            next.credits = to;
        }

        // Feature.
        let orb_count = orbs.iter().flatten().count();
        let trigger = self
            .model
            .hold_and_spin
            .as_ref()
            .filter(|hs| orb_count >= hs.trigger_count as usize);
        next.last_grid = Some(grid);
        if let Some(hs) = trigger {
            events.push(Event::FeatureTriggered {
                feature: Feature::HoldAndSpin,
                count: orb_count as u8,
            });
            events.push(Event::HoldAndSpinStarted {
                respins: hs.initial_respins,
                locked: locked_orbs(&orbs, rows),
            });
            next.phase = Phase::HoldAndSpin(HoldAndSpinState {
                cells: orbs,
                respins_left: hs.initial_respins,
                respins_played: 0,
                total_bet,
                base_award,
            });
        } else {
            events.push(Event::SpinCompleted {
                spin: next.spins,
                total_award: base_award,
            });
        }
        Ok(())
    }

    fn advance(
        &self,
        next: &mut GameState,
        rng: &mut dyn OutcomeRng,
        events: &mut Vec<Event>,
    ) -> Result<(), GameError> {
        let Phase::HoldAndSpin(feature) = &mut next.phase else {
            return Err(GameError::NoFeatureInProgress);
        };
        let hs = self
            .model
            .hold_and_spin
            .as_ref()
            .ok_or(GameError::Invariant("feature without a feature model"))?;
        let rows = self.model.grid.rows.max(1) as usize;

        if !feature.is_full() {
            if feature.respins_left == 0 {
                return Err(GameError::Invariant("feature in progress with no respins"));
            }
            let mut landed = Vec::new();
            for index in 0..feature.cells.len() {
                if feature.cells[index].is_some() {
                    continue;
                }
                if rng.below(hs.orb_land_chance.den)? < hs.orb_land_chance.num {
                    let value = draw_orb_value(hs, rng)?;
                    feature.cells[index] = Some(value);
                    landed.push(LockedOrb {
                        reel: (index / rows) as u8,
                        row: (index % rows) as u8,
                        value,
                    });
                }
            }
            feature.respins_played = feature
                .respins_played
                .checked_add(1)
                .ok_or(GameError::Overflow)?;
            feature.respins_left = if !landed.is_empty() && hs.reset_respins_on_new_orb {
                hs.initial_respins
            } else {
                feature.respins_left - 1
            };
            events.push(Event::HoldAndSpinRespin {
                respin: feature.respins_played,
                landed,
                respins_left: feature.respins_left,
                locked_total: feature.locked() as u8,
            });
        }

        if feature.is_full() || feature.respins_left == 0 {
            let full_grid = feature.is_full();
            let mut total = 0u64;
            for value in feature.cells.iter().flatten() {
                let award = match value {
                    OrbValue::BetMultiple(m) => u64::from(*m)
                        .checked_mul(feature.total_bet)
                        .ok_or(GameError::Overflow)?,
                    OrbValue::Jackpot(tier) => {
                        let award = jackpot_award(hs, *tier, feature.total_bet)?;
                        events.push(Event::JackpotAwarded { tier: *tier, award });
                        award
                    }
                };
                total = add(total, award)?;
            }
            if full_grid && let Some(tier) = hs.full_grid_award {
                let award = jackpot_award(hs, tier, feature.total_bet)?;
                events.push(Event::JackpotAwarded { tier, award });
                total = add(total, award)?;
            }
            let spin_total = add(feature.base_award, total)?;
            events.push(Event::HoldAndSpinCompleted {
                total_award: total,
                full_grid,
            });
            if total > 0 {
                let to = add(next.credits, total)?;
                events.push(Event::CreditsChanged {
                    from: next.credits,
                    to,
                    reason: CreditReason::FeatureAward,
                });
                next.credits = to;
            }
            events.push(Event::SpinCompleted {
                spin: next.spins,
                total_award: spin_total,
            });
            next.phase = Phase::Idle;
        }
        Ok(())
    }
}

fn locked_orbs(cells: &[Option<OrbValue>], rows: usize) -> Vec<LockedOrb> {
    let rows = rows.max(1);
    cells
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            value.map(|value| LockedOrb {
                reel: (index / rows) as u8,
                row: (index % rows) as u8,
                value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{DevRng, Draw, RecordingRng, ScriptedRng};
    use dragonlink_model::fixtures::synthetic_dev_fixture;
    use dragonlink_model::{
        BetModel, GridShape, JackpotDef, MathProvenance, Payline, Ratio, SymbolDef,
        WeightedOrbValue,
    };

    /// A tiny hand-made model for forcing feature outcomes: 2 reels × 2 rows,
    /// symbols X (regular) and ORB, one payline, no line pays. Trigger at 2 orbs,
    /// 3 respins, orb values ×1 (weight 1) and Mini (weight 1), Grand on a full grid.
    fn tiny() -> MathModel {
        let x = SymbolId(0);
        let orb = SymbolId(1);
        MathModel {
            id: "test-tiny".into(),
            provenance: MathProvenance::SyntheticDevFixture,
            description: "test".into(),
            grid: GridShape { reels: 2, rows: 2 },
            symbols: vec![
                SymbolDef {
                    id: x,
                    key: "X".into(),
                    kind: SymbolKind::Regular,
                },
                SymbolDef {
                    id: orb,
                    key: "ORB".into(),
                    kind: SymbolKind::Orb,
                },
            ],
            // Stop 0 shows X,X; stop 1 shows X,ORB; stop 2 shows ORB,ORB; stop 3 shows ORB,X.
            reel_strips: vec![vec![x, x, orb, orb], vec![x, x, orb, orb]],
            paylines: vec![Payline(vec![0, 0])],
            line_pays: vec![],
            scatter_pays: vec![],
            bet: BetModel {
                credits_per_line_options: vec![10, 20],
            },
            hold_and_spin: Some(HoldAndSpinModel {
                orb,
                trigger_count: 2,
                initial_respins: 3,
                reset_respins_on_new_orb: true,
                orb_land_chance: Ratio { num: 1, den: 4 },
                orb_values: vec![
                    WeightedOrbValue {
                        weight: 1,
                        value: OrbValue::BetMultiple(1),
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
                        total_bet_multiple: 5,
                    },
                    JackpotDef {
                        tier: JackpotTier::Grand,
                        total_bet_multiple: 100,
                    },
                ],
            }),
        }
    }

    fn d(bound: u32, value: u32) -> Draw {
        Draw { bound, value }
    }

    /// Follow the chain of credit events from `start`; returns the final amount.
    fn follow_credits(start: u64, events: &[Event]) -> u64 {
        let mut credits = start;
        for e in events {
            if let Event::CreditsChanged { from, to, .. } = e {
                assert_eq!(*from, credits, "credit events must chain");
                credits = *to;
            }
        }
        credits
    }

    #[test]
    fn a_spin_takes_the_bet_and_reports_every_reel() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        assert_eq!(game.total_bet(), 5);
        // Stops 0 on every reel: top rows A B C D A, no orbs in the window.
        let mut rng = ScriptedRng::new(vec![d(24, 0); 5]);
        let events = game.apply(Command::Spin, &mut rng).unwrap();
        assert_eq!(events[0], Event::BetPlaced { total_bet: 5 });
        assert_eq!(
            events[1],
            Event::CreditsChanged {
                from: 1000,
                to: 995,
                reason: CreditReason::Bet
            }
        );
        assert_eq!(events[2], Event::SpinStarted { spin: 1 });
        assert_eq!(
            events[3],
            Event::ReelStopped {
                reel: 0,
                stop: Some(0),
                symbols: vec!["A".into(), "E".into(), "F".into()]
            }
        );
        let stops = events
            .iter()
            .filter(|e| matches!(e, Event::ReelStopped { .. }))
            .count();
        assert_eq!(stops, 5);
        assert!(matches!(
            events.last(),
            Some(Event::SpinCompleted { spin: 1, .. })
        ));
        assert_eq!(rng.remaining(), 0);
        assert!(game.is_idle());
        assert_eq!(game.credits(), follow_credits(1000, &events));
    }

    #[test]
    fn reel_windows_wrap_around_the_strip() {
        let mut game = Game::new(synthetic_dev_fixture(), 100).unwrap();
        let mut rng = ScriptedRng::new(vec![d(24, 23), d(24, 0), d(24, 0), d(24, 0), d(24, 0)]);
        let events = game.apply(Command::Spin, &mut rng).unwrap();
        // Reel 0 strip ends …, F, H and starts A, E, …
        assert_eq!(
            events[3],
            Event::ReelStopped {
                reel: 0,
                stop: Some(23),
                symbols: vec!["H".into(), "A".into(), "E".into()]
            }
        );
    }

    #[test]
    fn refused_commands_change_nothing() {
        let mut game = Game::new(synthetic_dev_fixture(), 3).unwrap();
        let before = game.clone();
        let mut rng = DevRng::new(1);
        assert_eq!(
            game.apply(Command::Spin, &mut rng),
            Err(GameError::InsufficientCredits {
                needed: 5,
                available: 3
            })
        );
        assert_eq!(
            game.apply(Command::Advance, &mut rng),
            Err(GameError::NoFeatureInProgress)
        );
        assert_eq!(
            game.apply(Command::SetBet { index: 9 }, &mut rng),
            Err(GameError::NoSuchBet { index: 9 })
        );
        assert_eq!(game, before);
    }

    #[test]
    fn a_failing_outcome_source_leaves_the_state_untouched() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        let before = game.clone();
        // Only three of the five reel draws are available.
        let mut rng = ScriptedRng::new(vec![d(24, 0); 3]);
        assert!(matches!(
            game.apply(Command::Spin, &mut rng),
            Err(GameError::Rng(RngError::ScriptExhausted { position: 3 }))
        ));
        assert_eq!(game, before);
    }

    #[test]
    fn the_bet_option_scales_the_bet() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        let mut rng = DevRng::new(1);
        let events = game.apply(Command::SetBet { index: 2 }, &mut rng).unwrap();
        assert_eq!(
            events,
            vec![Event::BetChanged {
                credits_per_line: 5,
                total_bet: 25
            }]
        );
        assert_eq!(game.total_bet(), 25);
    }

    #[test]
    fn hold_and_spin_ends_when_the_respins_run_out() {
        let mut game = Game::new(tiny(), 100).unwrap();
        // Reel stops 1 and 1: window X,ORB | X,ORB → two orbs, values ×1 and Mini.
        // Then three respins on the two empty positions, nothing lands.
        let mut script = vec![d(4, 1), d(4, 1), d(2, 0), d(2, 1)];
        script.extend([d(4, 3); 6]);
        let mut rng = ScriptedRng::new(script);

        let spin = game.apply(Command::Spin, &mut rng).unwrap();
        assert!(spin.contains(&Event::FeatureTriggered {
            feature: Feature::HoldAndSpin,
            count: 2
        }));
        assert!(
            !spin
                .iter()
                .any(|e| matches!(e, Event::SpinCompleted { .. }))
        );
        assert!(!game.is_idle());
        assert_eq!(
            game.apply(Command::Spin, &mut rng),
            Err(GameError::FeatureInProgress)
        );
        assert_eq!(
            game.apply(Command::SetBet { index: 1 }, &mut rng),
            Err(GameError::FeatureInProgress)
        );

        let mut left = Vec::new();
        let mut all = spin;
        while !game.is_idle() {
            let step = game.apply(Command::Advance, &mut rng).unwrap();
            for e in &step {
                if let Event::HoldAndSpinRespin { respins_left, .. } = e {
                    left.push(*respins_left);
                }
            }
            all.extend(step);
        }
        assert_eq!(left, vec![2, 1, 0]);
        // Bet 10: ×1 pays 10, Mini pays 5 × 10 = 50. No Grand: the grid is not full.
        assert!(all.contains(&Event::JackpotAwarded {
            tier: JackpotTier::Mini,
            award: 50
        }));
        assert!(all.contains(&Event::HoldAndSpinCompleted {
            total_award: 60,
            full_grid: false
        }));
        assert_eq!(
            all.last(),
            Some(&Event::SpinCompleted {
                spin: 1,
                total_award: 60
            })
        );
        assert_eq!(game.credits(), 100 - 10 + 60);
        assert_eq!(game.credits(), follow_credits(100, &all));
        assert_eq!(rng.remaining(), 0);
    }

    #[test]
    fn a_new_orb_resets_the_respin_counter() {
        let mut game = Game::new(tiny(), 100).unwrap();
        // Two orbs to start (both ×1). Respin 1: nothing. Respin 2: first empty
        // position lands (×1), second does not → counter back to 3. Then three
        // respins on the one remaining position, nothing lands.
        let mut script = vec![d(4, 1), d(4, 1), d(2, 0), d(2, 0)];
        script.extend([d(4, 3), d(4, 3)]);
        script.extend([d(4, 0), d(2, 0), d(4, 3)]);
        script.extend([d(4, 3); 3]);
        let mut rng = ScriptedRng::new(script);
        let events = game.play_spin(&mut rng).unwrap();
        let left: Vec<u8> = events
            .iter()
            .filter_map(|e| match e {
                Event::HoldAndSpinRespin { respins_left, .. } => Some(*respins_left),
                _ => None,
            })
            .collect();
        assert_eq!(left, vec![2, 3, 2, 1, 0]);
        assert!(events.contains(&Event::HoldAndSpinCompleted {
            total_award: 30,
            full_grid: false
        }));
        assert_eq!(rng.remaining(), 0);
    }

    #[test]
    fn a_full_grid_ends_the_feature_and_adds_the_full_grid_jackpot() {
        let mut game = Game::new(tiny(), 100).unwrap();
        // Stops 2 and 2: all four positions are orbs (all ×1). No respin is played.
        let mut rng = ScriptedRng::new(vec![d(4, 2), d(4, 2), d(2, 0), d(2, 0), d(2, 0), d(2, 0)]);
        let events = game.play_spin(&mut rng).unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::HoldAndSpinRespin { .. }))
        );
        assert!(events.contains(&Event::JackpotAwarded {
            tier: JackpotTier::Grand,
            award: 1000
        }));
        // 4 × 10 from the orbs + 100 × 10 Grand.
        assert!(events.contains(&Event::HoldAndSpinCompleted {
            total_award: 1040,
            full_grid: true
        }));
        assert_eq!(game.credits(), 100 - 10 + 1040);
        assert_eq!(rng.remaining(), 0);
    }

    #[test]
    fn filling_the_grid_during_the_feature_ends_it_at_once() {
        let mut game = Game::new(tiny(), 100).unwrap();
        // Two orbs to start; the first respin lands both remaining positions.
        let mut rng = ScriptedRng::new(vec![
            d(4, 1),
            d(4, 1),
            d(2, 0),
            d(2, 0),
            d(4, 0),
            d(2, 0),
            d(4, 0),
            d(2, 0),
        ]);
        let events = game.play_spin(&mut rng).unwrap();
        let respins = events
            .iter()
            .filter(|e| matches!(e, Event::HoldAndSpinRespin { .. }))
            .count();
        assert_eq!(respins, 1);
        assert!(events.contains(&Event::HoldAndSpinCompleted {
            total_award: 1040,
            full_grid: true
        }));
        assert!(game.is_idle());
    }

    #[test]
    fn the_same_seed_gives_the_same_game() {
        let run = |seed: u64| {
            let mut game = Game::new(synthetic_dev_fixture(), 1_000_000).unwrap();
            let mut rng = DevRng::new(seed);
            let mut events = Vec::new();
            for _ in 0..500 {
                events.extend(game.play_spin(&mut rng).unwrap());
            }
            (events, game.credits())
        };
        assert_eq!(run(2026), run(2026));
        assert_ne!(run(2026).0, run(2027).0);
    }

    #[test]
    fn a_recorded_game_replays_event_for_event() {
        let mut game = Game::new(synthetic_dev_fixture(), 1_000_000).unwrap();
        let mut rec = RecordingRng::new(DevRng::new(77));
        let mut events = Vec::new();
        for _ in 0..300 {
            events.extend(game.play_spin(&mut rec).unwrap());
        }
        let mut replayed_game = Game::new(synthetic_dev_fixture(), 1_000_000).unwrap();
        let mut replay = ScriptedRng::new(rec.take_log());
        let mut replayed = Vec::new();
        for _ in 0..300 {
            replayed.extend(replayed_game.play_spin(&mut replay).unwrap());
        }
        assert_eq!(events, replayed);
        assert_eq!(replay.remaining(), 0);
        assert_eq!(game, replayed_game);
    }

    #[test]
    fn invariants_hold_over_many_seeded_games() {
        // Properties of our implementation under the synthetic fixture. The run
        // also has to reach the feature, or the feature checks prove nothing.
        let model = synthetic_dev_fixture();
        let initial = model
            .hold_and_spin
            .as_ref()
            .map(|h| h.initial_respins)
            .unwrap();
        let cells = model.grid.cells() as u8;
        let mut features = 0u32;
        for seed in 0..40u64 {
            let mut game = Game::new(model.clone(), 10_000_000).unwrap();
            let mut rng = DevRng::new(seed);
            for _ in 0..250 {
                let before = game.credits();
                let bet = game.total_bet();
                let events = game.play_spin(&mut rng).unwrap();
                assert!(game.is_idle());

                // Credits move only through credit events, and those chain.
                assert_eq!(follow_credits(before, &events), game.credits());

                // The spin's reported total equals its component awards.
                let lines: u64 = events
                    .iter()
                    .map(|e| match e {
                        Event::LineWin { award, .. } | Event::ScatterWin { award, .. } => *award,
                        _ => 0,
                    })
                    .sum();
                let feature: u64 = events
                    .iter()
                    .map(|e| match e {
                        Event::HoldAndSpinCompleted { total_award, .. } => *total_award,
                        _ => 0,
                    })
                    .sum();
                let Some(Event::SpinCompleted { total_award, .. }) = events.last() else {
                    panic!("a spin must end with SpinCompleted");
                };
                assert_eq!(*total_award, lines + feature);
                assert_eq!(game.credits(), before - bet + lines + feature);

                // Feature counters stay in range and the feature only grows.
                let mut locked_before = 0u8;
                for e in &events {
                    match e {
                        Event::HoldAndSpinStarted { respins, locked } => {
                            features += 1;
                            assert_eq!(*respins, initial);
                            locked_before = locked.len() as u8;
                        }
                        Event::HoldAndSpinRespin {
                            respins_left,
                            landed,
                            locked_total,
                            ..
                        } => {
                            assert!(*respins_left <= initial);
                            assert_eq!(*locked_total, locked_before + landed.len() as u8);
                            assert!(*locked_total <= cells);
                            locked_before = *locked_total;
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(features > 0, "the run never reached Hold & Spin");
    }

    #[test]
    fn state_survives_a_save_and_resume_mid_feature() {
        let mut game = Game::new(tiny(), 100).unwrap();
        let mut rng = ScriptedRng::new(vec![d(4, 1), d(4, 1), d(2, 0), d(2, 0)]);
        game.apply(Command::Spin, &mut rng).unwrap();
        let saved = serde_json::to_string(game.state()).unwrap();
        let state: GameState = serde_json::from_str(&saved).unwrap();
        let resumed = Game::resume(tiny(), state).unwrap();
        assert_eq!(resumed, game);
        assert!(!resumed.is_idle());
    }

    #[test]
    fn an_unrunnable_model_is_refused() {
        let mut model = tiny();
        model.reel_strips.pop();
        assert!(matches!(Game::new(model, 1), Err(GameError::Model(_))));
    }
}
