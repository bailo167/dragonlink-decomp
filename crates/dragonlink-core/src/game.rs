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
    FreeGamesModel, HoldAndSpinModel, JackpotTier, MathModel, ModelError, OrbValue, SymbolId,
    SymbolKind,
};
use serde::{Deserialize, Serialize};

use crate::eval::{Win, evaluate_lines};
use crate::event::{CreditReason, Event, Feature, LockedOrb};
use crate::grid::Grid;
use crate::rng::{OutcomeRng, RngError};

/// What the player can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Choose a bet option by index. Only while idle.
    SetBet { index: usize },
    /// Choose how many paylines to play: one of the model's line counts.
    /// Only while idle.
    SetLines { lines: u16 },
    /// Choose a bet option and a number of paylines at once. Only while idle.
    SetStake { index: usize, lines: u16 },
    /// Place the bet and spin. Only while idle.
    Spin,
    /// Play the next step of the feature in progress.
    Advance,
    /// Gamble the last win on the colour or suit of a card. Only while idle,
    /// with a win to gamble and the gamble limit not reached.
    Gamble { pick: GamblePick },
    /// Keep the last win and end the gamble.
    TakeWin,
}

/// A gamble choice: a colour pays double, a suit four times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamblePick {
    Red,
    Black,
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl GamblePick {
    /// What the pick multiplies the stake by when it wins.
    pub fn multiplier(self) -> u64 {
        match self {
            GamblePick::Red | GamblePick::Black => 2,
            _ => 4,
        }
    }

    /// Whether the pick wins against `card` (0..52: suit = card / 13 in the
    /// order hearts, diamonds, clubs, spades; rank = card % 13, ace first).
    pub fn wins(self, card: u8) -> bool {
        let suit = card / 13;
        match self {
            GamblePick::Red => suit < 2,
            GamblePick::Black => suit >= 2,
            GamblePick::Hearts => suit == 0,
            GamblePick::Diamonds => suit == 1,
            GamblePick::Clubs => suit == 2,
            GamblePick::Spades => suit == 3,
        }
    }
}

/// Gambles allowed in a row on one win. Synthetic: our own limit, not a
/// recovered cabinet rule.
pub const GAMBLE_LIMIT: u8 = 5;

/// Why a command was refused. The state is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GameError {
    #[error("the math model is not runnable: {0}")]
    Model(#[from] ModelError),
    #[error("outcome source failed: {0}")]
    Rng(#[from] RngError),
    #[error("a feature is in progress")]
    FeatureInProgress,
    #[error("there is no win to gamble")]
    NothingToGamble,
    #[error("no feature is in progress")]
    NoFeatureInProgress,
    #[error("the bet is {needed} credits but only {available} are available")]
    InsufficientCredits { needed: u64, available: u64 },
    #[error("there is no bet option {index}")]
    NoSuchBet { index: usize },
    #[error("the game does not offer {lines} lines")]
    NoSuchLineCount { lines: u16 },
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

/// Free games in progress. Taken out of the phase while one is played, so the
/// credit meter can change without aliasing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeGamesState {
    /// Free games still to play, including any added by a retrigger.
    pub games_left: u32,
    /// Free games played so far.
    pub games_played: u32,
    /// Everything paid since the feature began, the triggering spin included.
    pub total_award: u64,
    /// What the triggering spin itself paid; the free games are `total_award`
    /// less this.
    pub base_award: u64,
    /// The stake fixed when the feature began.
    pub total_bet: u64,
    pub credits_per_line: u32,
    pub lines: u16,
}

/// Where the game is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    HoldAndSpin(HoldAndSpinState),
    FreeGames(FreeGamesState),
}

/// Everything that changes while playing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    /// Play credits. Simulation credits only; they stand for no money.
    pub credits: u64,
    pub bet_index: usize,
    /// Paylines played. `None` is every payline of the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u16>,
    pub phase: Phase,
    /// Spins started so far.
    pub spins: u64,
    /// The window left by the last spin.
    pub last_grid: Option<Grid>,
    /// The win that may be gambled (already in `credits`), and the gambles
    /// taken on it so far.
    #[serde(default)]
    pub gamble_stake: u64,
    #[serde(default)]
    pub gambles: u8,
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

/// Draw the symbol a mystery reveals to. The weights are nonzero and sum to a
/// positive total (checked by [`MathModel::validate`]).
fn draw_reveal(fg: &FreeGamesModel, rng: &mut dyn OutcomeRng) -> Result<SymbolId, GameError> {
    let total: u64 = fg.reveal_values.iter().map(|v| u64::from(v.weight)).sum();
    let total = u32::try_from(total).map_err(|_| GameError::Overflow)?;
    let mut pick = rng.below(total)?;
    for entry in &fg.reveal_values {
        if pick < entry.weight {
            return Ok(entry.symbol);
        }
        pick -= entry.weight;
    }
    Err(GameError::Invariant("reveal draw past the table"))
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
                lines: None,
                phase: Phase::Idle,
                spins: 0,
                last_grid: None,
                gamble_stake: 0,
                gambles: 0,
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

    /// Paylines played: the first this many of the model's.
    pub fn lines(&self) -> u16 {
        self.state
            .lines
            .unwrap_or(u16::try_from(self.model.paylines.len()).unwrap_or(u16::MAX))
    }

    /// Credits one spin costs at the current bet option and line count.
    pub fn total_bet(&self) -> u64 {
        u64::from(self.credits_per_line()) * u64::from(self.lines())
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
            Command::SetBet { index } => {
                self.set_stake(&mut next, index, self.lines(), &mut events)?;
            }
            Command::SetLines { lines } => {
                self.set_stake(&mut next, self.state.bet_index, lines, &mut events)?;
            }
            Command::SetStake { index, lines } => {
                self.set_stake(&mut next, index, lines, &mut events)?;
            }
            Command::Spin => self.spin(&mut next, rng, &mut events)?,
            Command::Advance => self.advance(&mut next, rng, &mut events)?,
            Command::Gamble { pick } => Self::gamble(&mut next, pick, rng, &mut events)?,
            Command::TakeWin => {
                if next.gamble_stake > 0 {
                    events.push(Event::WinTaken {
                        award: next.gamble_stake,
                    });
                }
                next.gamble_stake = 0;
                next.gambles = 0;
            }
        }
        // A completed game's win may be gambled.
        if let Some(Event::SpinCompleted { total_award, .. }) = events
            .iter()
            .rev()
            .find(|e| matches!(e, Event::SpinCompleted { .. }))
        {
            next.gamble_stake = *total_award;
            next.gambles = 0;
        }
        self.state = next;
        Ok(events)
    }

    /// Whether the last win can be gambled now.
    pub fn can_gamble(&self) -> bool {
        self.is_idle() && self.state.gamble_stake > 0 && self.state.gambles < GAMBLE_LIMIT
    }

    fn gamble(
        next: &mut GameState,
        pick: GamblePick,
        rng: &mut dyn OutcomeRng,
        events: &mut Vec<Event>,
    ) -> Result<(), GameError> {
        if next.phase != Phase::Idle {
            return Err(GameError::FeatureInProgress);
        }
        if next.gamble_stake == 0 || next.gambles >= GAMBLE_LIMIT {
            return Err(GameError::NothingToGamble);
        }
        let card = rng.below(52)? as u8;
        let stake = next.gamble_stake;
        let won = pick.wins(card);
        let to = if won {
            stake
                .checked_mul(pick.multiplier())
                .ok_or(GameError::Overflow)?
        } else {
            0
        };
        // The stake is already in credits: a win adds the difference, a loss
        // takes the stake back.
        let credits = if won {
            add(next.credits, to - stake)?
        } else {
            next.credits.checked_sub(stake).ok_or(GameError::Overflow)?
        };
        events.push(Event::GambleResolved {
            pick,
            card,
            won,
            stake,
            award: to,
        });
        events.push(Event::CreditsChanged {
            from: next.credits,
            to: credits,
            reason: CreditReason::Gamble,
        });
        next.credits = credits;
        next.gambles += 1;
        next.gamble_stake = to;
        if !won || next.gambles >= GAMBLE_LIMIT {
            next.gamble_stake = if won { to } else { 0 };
        }
        Ok(())
    }

    /// Spin, then play any feature to its end. For headless runs.
    pub fn play_spin(&mut self, rng: &mut dyn OutcomeRng) -> Result<Vec<Event>, GameError> {
        let mut events = self.apply(Command::Spin, rng)?;
        while !self.is_idle() {
            events.extend(self.apply(Command::Advance, rng)?);
        }
        Ok(events)
    }

    fn set_stake(
        &self,
        next: &mut GameState,
        index: usize,
        lines: u16,
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
        if !self.model.line_counts().contains(&lines) {
            return Err(GameError::NoSuchLineCount { lines });
        }
        next.bet_index = index;
        next.lines = (usize::from(lines) != self.model.paylines.len()).then_some(lines);
        events.push(Event::BetChanged {
            credits_per_line,
            total_bet: u64::from(credits_per_line) * u64::from(lines),
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
        next.gamble_stake = 0;
        next.gambles = 0;
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
        let wins = evaluate_lines(
            &self.model,
            &grid,
            credits_per_line,
            total_bet,
            usize::from(self.lines()),
        )
        .ok_or(GameError::Overflow)?;
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

        // Feature. Hold & Spin takes precedence over free games on a spin
        // that could start both.
        let orb_count = orbs.iter().flatten().count();
        let scatter_count = self
            .model
            .scatter_symbol()
            .map(|sc| grid.iter().filter(|(_, _, s)| *s == sc).count())
            .unwrap_or(0);
        let hs_trigger = self
            .model
            .hold_and_spin
            .as_ref()
            .filter(|hs| orb_count >= hs.trigger_count as usize);
        let fg_trigger = self
            .model
            .free_games
            .as_ref()
            .filter(|fg| hs_trigger.is_none() && scatter_count >= fg.trigger_count as usize);
        next.last_grid = Some(grid);
        if let Some(hs) = hs_trigger {
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
        } else if let Some(fg) = fg_trigger {
            events.push(Event::FeatureTriggered {
                feature: Feature::FreeGames,
                count: scatter_count as u8,
            });
            events.push(Event::FreeGamesStarted {
                games: fg.initial_games,
            });
            next.phase = Phase::FreeGames(FreeGamesState {
                games_left: u32::from(fg.initial_games),
                games_played: 0,
                total_award: base_award,
                base_award,
                total_bet,
                credits_per_line,
                lines: self.lines(),
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
        match next.phase {
            Phase::HoldAndSpin(_) => self.advance_hold_and_spin(next, rng, events),
            Phase::FreeGames(_) => self.advance_free_games(next, rng, events),
            Phase::Idle => Err(GameError::NoFeatureInProgress),
        }
    }

    /// Play one free game.
    fn advance_free_games(
        &self,
        next: &mut GameState,
        rng: &mut dyn OutcomeRng,
        events: &mut Vec<Event>,
    ) -> Result<(), GameError> {
        let fg = self.model.free_games.as_ref().ok_or(GameError::Invariant(
            "free games without a free-games model",
        ))?;
        // Take the state out so the credit meter can change without aliasing.
        let Phase::FreeGames(mut state) = std::mem::replace(&mut next.phase, Phase::Idle) else {
            return Err(GameError::NoFeatureInProgress);
        };
        let rows = self.model.grid.rows as usize;
        let number = state
            .games_played
            .checked_add(1)
            .ok_or(GameError::Overflow)?;
        state.games_played = number;
        state.games_left = state.games_left.saturating_sub(1);
        events.push(Event::FreeGameStarted {
            number,
            remaining: state.games_left,
        });

        // The free reels.
        let mut cells = Vec::with_capacity(self.model.grid.cells());
        for (reel, strip) in fg.free_reel_strips.iter().enumerate() {
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
        let mut grid = Grid::new(self.model.grid, cells).ok_or(GameError::Invariant(
            "free window size does not match the model",
        ))?;

        // The reveal: one drawn symbol fills every mystery position.
        if let Some(mystery) = fg.mystery {
            let count = grid.iter().filter(|(_, _, s)| *s == mystery).count();
            if count > 0 {
                let revealed = draw_reveal(fg, rng)?;
                let new_cells: Vec<SymbolId> = grid
                    .iter()
                    .map(|(_, _, s)| if s == mystery { revealed } else { s })
                    .collect();
                grid = Grid::new(self.model.grid, new_cells)
                    .ok_or(GameError::Invariant("reveal window size"))?;
                events.push(Event::MysteryRevealed {
                    symbol: self.key(revealed),
                    count: count as u8,
                });
            }
        }

        // Pays. A free game takes no bet.
        let wins = evaluate_lines(
            &self.model,
            &grid,
            state.credits_per_line,
            state.total_bet,
            usize::from(state.lines),
        )
        .ok_or(GameError::Overflow)?;
        let mut award = 0u64;
        for win in &wins {
            award = add(award, win.award())?;
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
        if award > 0 {
            let to = add(next.credits, award)?;
            events.push(Event::CreditsChanged {
                from: next.credits,
                to,
                reason: CreditReason::Win,
            });
            next.credits = to;
        }
        state.total_award = add(state.total_award, award)?;

        // Retrigger.
        if fg.retrigger_count > 0 {
            let scatters = self
                .model
                .scatter_symbol()
                .map(|sc| grid.iter().filter(|(_, _, s)| *s == sc).count())
                .unwrap_or(0);
            if scatters >= fg.retrigger_count as usize {
                state.games_left = state
                    .games_left
                    .checked_add(u32::from(fg.retrigger_games))
                    .ok_or(GameError::Overflow)?;
                events.push(Event::FreeGamesRetriggered {
                    added: fg.retrigger_games,
                    remaining: state.games_left,
                });
            }
        }
        events.push(Event::FreeGameCompleted { number, award });

        if state.games_left == 0 {
            events.push(Event::FreeGamesCompleted {
                games_played: state.games_played,
                total_award: state.total_award - state.base_award,
            });
            events.push(Event::SpinCompleted {
                spin: next.spins,
                total_award: state.total_award,
            });
            // next.phase is already Idle.
        } else {
            next.phase = Phase::FreeGames(state);
        }
        Ok(())
    }

    /// Play one Hold & Spin respin, and finish the feature when it ends.
    fn advance_hold_and_spin(
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
                line_options: Vec::new(),
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
            free_games: None,
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
    fn the_player_chooses_how_many_lines_to_play() {
        let mut model = synthetic_dev_fixture();
        model.bet.line_options = vec![1, 3, 5];
        model.validate().unwrap();
        let mut game = Game::new(model.clone(), 100_000).unwrap();
        let mut rng = DevRng::new(4);
        // All lines until told otherwise.
        assert_eq!((game.lines(), game.total_bet()), (5, 5));
        let events = game
            .apply(Command::SetLines { lines: 3 }, &mut rng)
            .unwrap();
        assert_eq!(
            events,
            vec![Event::BetChanged {
                credits_per_line: 1,
                total_bet: 3
            }]
        );
        assert_eq!((game.lines(), game.total_bet()), (3, 3));
        // A bet option and a line count together are one change.
        let events = game
            .apply(Command::SetStake { index: 3, lines: 1 }, &mut rng)
            .unwrap();
        assert_eq!(
            events,
            vec![Event::BetChanged {
                credits_per_line: 10,
                total_bet: 10
            }]
        );
        // A count the game does not offer is refused, and nothing changes.
        for lines in [0, 2, 4, 6] {
            assert_eq!(
                game.apply(Command::SetLines { lines }, &mut rng),
                Err(GameError::NoSuchLineCount { lines })
            );
        }
        assert_eq!((game.lines(), game.credits_per_line()), (1, 10));

        // Only the lines played can win, and the bet taken is for those lines.
        game.apply(Command::SetStake { index: 0, lines: 1 }, &mut rng)
            .unwrap();
        let mut staked = 0;
        for _ in 0..3000 {
            let before = game.credits();
            let events = game.play_spin(&mut rng).unwrap();
            assert_eq!(events[0], Event::BetPlaced { total_bet: 1 });
            staked += 1;
            for event in &events {
                if let Event::LineWin { line, .. } = event {
                    assert_eq!(*line, 0, "a line that was not played paid");
                }
            }
            assert!(game.credits() + 1 >= before);
        }
        assert_eq!(staked, 3000);
        // A saved game keeps its line count; a save from before lines existed means all lines.
        let saved = serde_json::to_string(game.state()).unwrap();
        assert!(saved.contains("\"lines\":1"));
        let resumed = Game::resume(model.clone(), serde_json::from_str(&saved).unwrap()).unwrap();
        assert_eq!(resumed.lines(), 1);
        let all = Game::new(model, 10).unwrap();
        assert!(
            !serde_json::to_string(all.state())
                .unwrap()
                .contains("lines")
        );

        // A model without line options offers exactly "all of them".
        let mut plain = Game::new(synthetic_dev_fixture(), 10).unwrap();
        assert_eq!(
            plain.apply(Command::SetLines { lines: 3 }, &mut rng),
            Err(GameError::NoSuchLineCount { lines: 3 })
        );
        assert!(
            plain
                .apply(Command::SetLines { lines: 5 }, &mut rng)
                .is_ok()
        );
    }

    #[test]
    fn line_options_that_make_no_sense_are_refused() {
        for bad in [vec![0, 5], vec![3, 3], vec![5, 3], vec![1, 6]] {
            let mut model = synthetic_dev_fixture();
            model.bet.line_options = bad.clone();
            assert_eq!(
                model.validate(),
                Err(dragonlink_model::ModelError::BadLineOptions),
                "{bad:?}"
            );
        }
        // The fixture as written does not mention lines, so its exported form is unchanged.
        let json = serde_json::to_string(&synthetic_dev_fixture()).unwrap();
        assert!(!json.contains("\"line_options\""));
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

    #[test]
    fn a_gamble_doubles_on_colour_quadruples_on_suit_and_loses_the_stake() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        // Nothing to gamble before a win.
        let mut rng = ScriptedRng::new(vec![d(52, 0)]);
        assert_eq!(
            game.apply(
                Command::Gamble {
                    pick: GamblePick::Red
                },
                &mut rng
            ),
            Err(GameError::NothingToGamble)
        );
        // A win of 10 credits, already paid.
        game.state.gamble_stake = 10;
        // Ace of hearts: red wins double.
        let mut rng = ScriptedRng::new(vec![d(52, 0)]);
        let ev = game
            .apply(
                Command::Gamble {
                    pick: GamblePick::Red,
                },
                &mut rng,
            )
            .unwrap();
        assert!(ev.contains(&Event::GambleResolved {
            pick: GamblePick::Red,
            card: 0,
            won: true,
            stake: 10,
            award: 20,
        }));
        assert_eq!(game.credits(), 1010);
        assert_eq!(game.state().gamble_stake, 20);
        // Ace of spades: spades wins four times.
        let mut rng = ScriptedRng::new(vec![d(52, 39)]);
        game.apply(
            Command::Gamble {
                pick: GamblePick::Spades,
            },
            &mut rng,
        )
        .unwrap();
        assert_eq!(game.credits(), 1070);
        assert_eq!(game.state().gamble_stake, 80);
        // Two of diamonds: black loses the stake.
        let mut rng = ScriptedRng::new(vec![d(52, 14)]);
        game.apply(
            Command::Gamble {
                pick: GamblePick::Black,
            },
            &mut rng,
        )
        .unwrap();
        assert_eq!(game.credits(), 990);
        assert_eq!(game.state().gamble_stake, 0);
        assert!(!game.can_gamble());
    }

    #[test]
    fn the_gamble_stops_at_its_limit_and_take_win_keeps_the_win() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        game.state.gamble_stake = 1;
        for _ in 0..GAMBLE_LIMIT {
            let mut rng = ScriptedRng::new(vec![d(52, 0)]);
            game.apply(
                Command::Gamble {
                    pick: GamblePick::Red,
                },
                &mut rng,
            )
            .unwrap();
        }
        assert!(!game.can_gamble());
        let credits = game.credits();
        game.apply(Command::TakeWin, &mut ScriptedRng::new(vec![]))
            .unwrap();
        assert_eq!(game.credits(), credits);
        assert_eq!(game.state().gamble_stake, 0);
    }
}
