//! Observable events.
//!
//! One vocabulary serves both sides of a parity comparison: the simulation
//! emits these, and an observer transcribing a recording of the original
//! writes the same ones (`dragonlink-trace`). Symbols are named by key, not
//! by number, so a trace reads the same whoever produced it. Fields an
//! observer cannot see (a reel's stop index) are optional.

use dragonlink_model::{JackpotTier, OrbValue};
use serde::{Deserialize, Serialize};

/// A feature that a spin can start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    HoldAndSpin,
    FreeGames,
}

/// Why the credit meter moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreditReason {
    Bet,
    Win,
    FeatureAward,
    /// A gamble won (credits up) or lost (the stake taken back).
    Gamble,
    /// The denomination changed: the same money in the new credit.
    Denomination,
}

/// An orb held in place during Hold & Spin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockedOrb {
    pub reel: u8,
    /// Row 0 is the top.
    pub row: u8,
    pub value: OrbValue,
}

/// Something that happened, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    BetChanged {
        credits_per_line: u32,
        total_bet: u64,
    },
    BetPlaced {
        total_bet: u64,
    },
    CreditsChanged {
        from: u64,
        to: u64,
        reason: CreditReason,
    },
    SpinStarted {
        spin: u64,
    },
    ReelStopped {
        reel: u8,
        /// Index of the top visible stop on the reel strip. Not observable
        /// from outside; absent in transcribed recordings.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop: Option<u32>,
        /// Visible symbols, top to bottom, by key.
        symbols: Vec<String>,
    },
    OrbValueShown {
        reel: u8,
        row: u8,
        value: OrbValue,
    },
    LineWin {
        line: u16,
        symbol: String,
        count: u8,
        award: u64,
    },
    ScatterWin {
        symbol: String,
        count: u8,
        award: u64,
    },
    FeatureTriggered {
        feature: Feature,
        /// Number of triggering symbols in the window.
        count: u8,
    },
    HoldAndSpinStarted {
        respins: u8,
        locked: Vec<LockedOrb>,
    },
    HoldAndSpinRespin {
        /// 1 for the first respin of the feature.
        respin: u32,
        landed: Vec<LockedOrb>,
        respins_left: u8,
        locked_total: u8,
    },
    JackpotAwarded {
        tier: JackpotTier,
        award: u64,
    },
    HoldAndSpinCompleted {
        total_award: u64,
        full_grid: bool,
    },
    FreeGamesStarted {
        /// Free games awarded at the start.
        games: u8,
    },
    FreeGameStarted {
        /// 1 for the first free game.
        number: u32,
        /// Free games left to play after this one, before any retrigger.
        remaining: u32,
    },
    /// The mystery symbol revealed to `symbol` across all `count` of its
    /// positions in a free game.
    MysteryRevealed {
        symbol: String,
        count: u8,
    },
    /// A wild filled its reel in a free game.
    WildExpanded {
        reel: u8,
    },
    /// The free games' BIG reel stopped on `symbol`, covering `width` reels
    /// from `first`.
    BigSymbol {
        symbol: String,
        first: u8,
        width: u8,
    },
    /// The free games carry on after a Hold & Spin that one of them started.
    FreeGamesResumed {
        /// Free games left to play.
        remaining: u32,
    },
    FreeGamesRetriggered {
        /// Free games added.
        added: u8,
        /// Free games remaining after the retrigger.
        remaining: u32,
    },
    FreeGameCompleted {
        number: u32,
        /// What this free game paid (lines and scatters).
        award: u64,
    },
    FreeGamesCompleted {
        games_played: u32,
        /// Everything the free games paid together.
        total_award: u64,
    },
    SpinCompleted {
        spin: u64,
        /// Everything the spin paid, features included.
        total_award: u64,
    },
    /// A gamble's card is turned.
    GambleResolved {
        pick: crate::game::GamblePick,
        /// 0..52: suit = card / 13 (hearts, diamonds, clubs, spades), rank =
        /// card % 13 (ace first).
        card: u8,
        won: bool,
        /// What was gambled, and what the gamble now holds (0 when lost).
        stake: u64,
        award: u64,
    },
    /// The player kept the win.
    WinTaken {
        award: u64,
    },
}

impl Event {
    /// The event's type name as it appears in a trace.
    pub fn kind(&self) -> &'static str {
        match self {
            Event::BetChanged { .. } => "bet_changed",
            Event::BetPlaced { .. } => "bet_placed",
            Event::CreditsChanged { .. } => "credits_changed",
            Event::SpinStarted { .. } => "spin_started",
            Event::ReelStopped { .. } => "reel_stopped",
            Event::OrbValueShown { .. } => "orb_value_shown",
            Event::LineWin { .. } => "line_win",
            Event::ScatterWin { .. } => "scatter_win",
            Event::FeatureTriggered { .. } => "feature_triggered",
            Event::HoldAndSpinStarted { .. } => "hold_and_spin_started",
            Event::HoldAndSpinRespin { .. } => "hold_and_spin_respin",
            Event::JackpotAwarded { .. } => "jackpot_awarded",
            Event::HoldAndSpinCompleted { .. } => "hold_and_spin_completed",
            Event::FreeGamesStarted { .. } => "free_games_started",
            Event::FreeGameStarted { .. } => "free_game_started",
            Event::MysteryRevealed { .. } => "mystery_revealed",
            Event::FreeGamesResumed { .. } => "free_games_resumed",
            Event::WildExpanded { .. } => "wild_expanded",
            Event::BigSymbol { .. } => "big_symbol",
            Event::FreeGamesRetriggered { .. } => "free_games_retriggered",
            Event::FreeGameCompleted { .. } => "free_game_completed",
            Event::FreeGamesCompleted { .. } => "free_games_completed",
            Event::SpinCompleted { .. } => "spin_completed",
            Event::GambleResolved { .. } => "gamble_resolved",
            Event::WinTaken { .. } => "win_taken",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_matches_the_serialised_tag() {
        let events = [
            Event::BetPlaced { total_bet: 5 },
            Event::ReelStopped {
                reel: 0,
                stop: Some(3),
                symbols: vec!["A".into()],
            },
            Event::HoldAndSpinRespin {
                respin: 1,
                landed: vec![LockedOrb {
                    reel: 1,
                    row: 2,
                    value: OrbValue::Jackpot(JackpotTier::Mini),
                }],
                respins_left: 3,
                locked_total: 7,
            },
            Event::FreeGamesResumed { remaining: 2 },
            Event::SpinCompleted {
                spin: 1,
                total_award: 0,
            },
        ];
        for e in events {
            let json = serde_json::to_value(&e).unwrap();
            assert_eq!(json["type"], e.kind());
            let back: Event = serde_json::from_value(json).unwrap();
            assert_eq!(back, e);
        }
    }

    #[test]
    fn an_unobserved_stop_is_omitted_and_reads_back_as_absent() {
        let e = Event::ReelStopped {
            reel: 2,
            stop: None,
            symbols: vec!["A".into(), "B".into(), "C".into()],
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("\"stop\""));
        assert_eq!(serde_json::from_str::<Event>(&json).unwrap(), e);
    }
}
