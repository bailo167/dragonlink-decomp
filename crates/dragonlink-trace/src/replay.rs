//! Replaying observed outcomes through the simulation.
//!
//! [`compare`] can only line two traces up. Lined up against a free-running
//! rewrite, an observation differs at the first reel, because the two games
//! were not dealt the same outcomes. Replay removes that difference:
//!
//! 1. [`reconstruct`] reads the outcomes an observation shows (which symbols
//!    each reel stopped on, which value each orb carries, which positions
//!    landed an orb on each respin, which bet was chosen) and derives, under
//!    a given [`MathModel`], the commands and the scripted draws that make
//!    `dragonlink-core` produce those same outcomes. It follows the draw
//!    order documented in `dragonlink_core::game`.
//! 2. [`replay`] runs the simulation on that script and compares what it
//!    emitted with the observation. Whatever still differs (an award, a
//!    feature that did or did not start, a counter) is a difference in
//!    rules, not in luck.
//!
//! Reconstruction reads **only outcomes**. Awards, credit movements, feature
//! entry and counters in the observation are never used to steer the
//! simulation; they are what the comparison is for.
//!
//! # What this does and does not establish
//!
//! **No observation of an original game exists in this project yet.** The
//! mechanism is built and tested on rewrite traces relabelled as
//! observations, which proves that the mechanism works and nothing else. A
//! replay states what it amounts to ([`replay_kind`]): through the synthetic
//! fixture it is a test of this machinery; through a model that is not
//! cabinet or regulatory evidence it is a check of the rules under that
//! model's reels and tables; only through recovered cabinet mathematics is
//! it labelled parity, and then for the rules and the order of events on the
//! outcomes that were observed. No replay says anything about how outcomes
//! are distributed.
//!
//! # Ambiguity
//!
//! An observer sees a window, not a stop. When several stops of a strip show
//! the same symbols, or several entries of the value table carry the same
//! value, the observation does not say which was drawn. The lowest stop and
//! the first entry are scripted and the choice is recorded as an
//! [`Ambiguity`]. Under *this* simulation the choice cannot change anything
//! an observer could see: a stop decides only the window, a table entry only
//! the value. That is a property of our draw order, not a finding about the
//! original, and it has to be revisited if a model ever makes the hidden
//! choice matter.
//!
//! Each draw is also one of a range (every draw below an entry's weight
//! gives that entry; every draw below the land chance's numerator lands an
//! orb). The lowest of the range is scripted. That is not recorded as an
//! ambiguity: the values of a range are indistinguishable by construction.

use std::collections::BTreeMap;

use dragonlink_core::{Command, Draw, Event, Game, GameError, LockedOrb, ScriptedRng};
use dragonlink_model::{HoldAndSpinModel, MathModel, MathProvenance, OrbValue, SymbolKind};
use serde::Serialize;

use crate::{
    Comparison, ComparisonKind, SCHEMA, Trace, TraceHeader, TraceOrigin, TraceRecord, compare,
    source_for,
};

/// Credits a replay starts with when the observation has no credit event to
/// read the meter from. Arbitrary: with no credit event observed there is no
/// meter to agree with, only spins that must not be refused for lack of credits.
pub const UNOBSERVED_STARTING_CREDITS: u64 = 1_000_000_000;

/// A hidden choice the observation does not determine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AmbiguousChoice {
    /// Several stops of this reel's strip show the observed symbols.
    ReelStop { reel: u8 },
    /// Several entries of the value table carry the observed value.
    OrbValueEntry { reel: u8, row: u8 },
    /// Several bet options have the observed credits per line.
    BetOption,
}

/// One place where reconstruction had to choose among equivalent candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Ambiguity {
    /// The observation event the choice was made for.
    pub seq: u64,
    pub what: AmbiguousChoice,
    /// How many candidates show what was observed.
    pub candidates: usize,
    /// The one scripted: the lowest stop index, or the index of the first
    /// matching table entry or bet option.
    pub chosen: u32,
}

/// Why an observed event cannot have come from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InconsistencyReason {
    #[error("the model has no bet option of {credits_per_line} credits per line")]
    NoSuchBet { credits_per_line: u32 },
    #[error(
        "reel {reel} stopped outside a spin, so the trace does not say which spin it belongs to"
    )]
    ReelOutsideSpin { reel: u8 },
    #[error("reel {reel} does not exist; the model has {reels} reels")]
    NoSuchReel { reel: u8, reels: u8 },
    #[error("reel {reel} stopped more than once in one spin")]
    ReelRepeated { reel: u8 },
    #[error("reel {reel} was not observed stopping in this spin")]
    ReelMissing { reel: u8 },
    #[error("reel {reel} shows {observed} symbols; the model's window has {rows} rows")]
    WindowHeight { reel: u8, observed: usize, rows: u8 },
    #[error("reel {reel} shows {key:?}, which is not a symbol of the model")]
    UnknownSymbol { reel: u8, key: String },
    #[error("no stop of the model's strip for reel {reel} shows {symbols:?}")]
    NoSuchWindow { reel: u8, symbols: Vec<String> },
    #[error(
        "the trace gives stop {stop} for reel {reel}, where the model's strip does not show {symbols:?}"
    )]
    StatedStopDoesNotMatch {
        reel: u8,
        stop: u32,
        symbols: Vec<String>,
    },
    #[error("the window has an orb at reel {reel}, row {row}, but no value was observed for it")]
    OrbValueMissing { reel: u8, row: u8 },
    #[error("more than one value was observed for the orb at reel {reel}, row {row}")]
    OrbValueRepeated { reel: u8, row: u8 },
    #[error("the model's value table cannot give {value:?} (orb at reel {reel}, row {row})")]
    OrbValueNotInTable { reel: u8, row: u8, value: OrbValue },
    #[error("a Hold & Spin step was observed, but the model has no Hold & Spin")]
    NoFeatureInModel,
    #[error("a Hold & Spin step was observed before any spin, so the held positions are not known")]
    FeatureWithoutSpin,
    #[error("an orb landed at reel {reel}, row {row}, which is outside the model's window")]
    LandedOutsideWindow { reel: u8, row: u8 },
    #[error("an orb landed at reel {reel}, row {row}, which already held one")]
    LandedOnHeldPosition { reel: u8, row: u8 },
    #[error("two orbs landed at reel {reel}, row {row}, on one respin")]
    LandedTwice { reel: u8, row: u8 },
    #[error("an orb landed, but under the model's land chance none ever does")]
    LandingImpossible,
    #[error(
        "reel {reel}, row {row} stayed empty, but under the model's land chance every empty position lands an orb"
    )]
    StayingEmptyImpossible { reel: u8, row: u8 },
}

/// An observed event that is inconsistent with the model. This is a finding
/// about the observation and the model together, not an error: either the
/// model is not the observed game's, or the transcription is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Inconsistency {
    /// The observation event. For something that was expected and never seen
    /// (a reel that did not stop, an orb with no value), the spin's
    /// `SpinStarted` event.
    pub seq: u64,
    pub reason: InconsistencyReason,
}

/// A command the observation implies, with the draws that make the
/// simulation produce the outcome observed for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScriptedCommand {
    /// The observation event that implies the command: `BetChanged` for a
    /// bet, `SpinStarted` for a spin, `HoldAndSpinRespin` for a respin, and
    /// `HoldAndSpinCompleted` for a feature that ended without one.
    pub seq: u64,
    pub command: Command,
    /// In the order the simulation asks for them.
    pub draws: Vec<Draw>,
}

/// The script derived from an observation under a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reconstruction {
    /// The commands that could be scripted, in order. Ends before the first
    /// command with an inconsistency: nothing after it can be replayed
    /// without inventing an outcome.
    pub commands: Vec<ScriptedCommand>,
    /// Hidden choices the observation left open, over the whole observation.
    pub ambiguities: Vec<Ambiguity>,
    /// Everything found to be inconsistent with the model, over the whole
    /// observation (not only up to the first).
    pub inconsistencies: Vec<Inconsistency>,
    /// `seq` of the event implying the first command that could not be
    /// scripted. `None` when the whole observation was scripted.
    pub unscripted_from: Option<u64>,
}

impl Reconstruction {
    /// Whether every command the observation implies was scripted.
    pub fn is_complete(&self) -> bool {
        self.unscripted_from.is_none()
    }

    /// Number of scripted draws.
    pub fn draw_count(&self) -> usize {
        self.commands.iter().map(|c| c.draws.len()).sum()
    }

    /// The scripted draws of all commands, in order.
    pub fn draws(&self) -> Vec<Draw> {
        self.commands
            .iter()
            .flat_map(|c| c.draws.iter().copied())
            .collect()
    }
}

/// One reel strip as the reconstruction needs it.
struct ReelView<'a> {
    /// The strip's length: the bound of the reel's draw.
    bound: u32,
    /// The key of the symbol at each stop.
    keys: Vec<&'a str>,
    /// Whether the symbol at each stop is given an orb value by the simulation.
    orb: Vec<bool>,
}

/// A value-table draw for an observed value.
struct ValueDraw {
    draw: Draw,
    candidates: usize,
    entry: u32,
}

struct ModelView<'a> {
    model: &'a MathModel,
    rows: usize,
    reels: Vec<ReelView<'a>>,
    /// The feature and the total weight of its value table.
    feature: Option<(&'a HoldAndSpinModel, u32)>,
}

impl<'a> ModelView<'a> {
    fn new(model: &'a MathModel) -> Result<Self, GameError> {
        model.validate()?;
        let feature = match &model.hold_and_spin {
            Some(hs) => {
                let total: u64 = hs.orb_values.iter().map(|v| u64::from(v.weight)).sum();
                Some((hs, u32::try_from(total).map_err(|_| GameError::Overflow)?))
            }
            None => None,
        };
        let reels = model
            .reel_strips
            .iter()
            .map(|strip| {
                Ok(ReelView {
                    bound: u32::try_from(strip.len()).map_err(|_| GameError::Overflow)?,
                    keys: strip
                        .iter()
                        .map(|id| model.symbol(*id).map_or("", |s| s.key.as_str()))
                        .collect(),
                    // The simulation gives a value to every orb in the window,
                    // and only when the model has the feature.
                    orb: strip
                        .iter()
                        .map(|id| feature.is_some() && model.kind(*id) == Some(SymbolKind::Orb))
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        Ok(ModelView {
            model,
            rows: model.grid.rows as usize,
            reels,
            feature,
        })
    }

    /// The stop to script for `reel` showing `symbols`, and how many stops
    /// show them. A stop stated in the trace is used when it shows them.
    fn find_stop(
        &self,
        reel: u8,
        stated: Option<u32>,
        symbols: &[String],
    ) -> Result<(u32, usize), InconsistencyReason> {
        let Some(view) = self.reels.get(reel as usize) else {
            return Err(InconsistencyReason::NoSuchReel {
                reel,
                reels: self.model.grid.reels,
            });
        };
        if symbols.len() != self.rows {
            return Err(InconsistencyReason::WindowHeight {
                reel,
                observed: symbols.len(),
                rows: self.model.grid.rows,
            });
        }
        if let Some(key) = symbols
            .iter()
            .find(|key| !self.model.symbols.iter().any(|s| s.key == **key))
        {
            return Err(InconsistencyReason::UnknownSymbol {
                reel,
                key: key.clone(),
            });
        }
        let len = view.keys.len();
        let shows = |stop: usize| {
            len > 0
                && symbols
                    .iter()
                    .enumerate()
                    .all(|(row, key)| view.keys.get((stop + row) % len) == Some(&key.as_str()))
        };
        if let Some(stop) = stated {
            return if (stop as usize) < len && shows(stop as usize) {
                Ok((stop, 1))
            } else {
                Err(InconsistencyReason::StatedStopDoesNotMatch {
                    reel,
                    stop,
                    symbols: symbols.to_vec(),
                })
            };
        }
        let mut matching = (0..len).filter(|stop| shows(*stop));
        match matching.next() {
            // `stop` is below the strip's length, which fits in 32 bits.
            Some(stop) => Ok((stop as u32, 1 + matching.count())),
            None => Err(InconsistencyReason::NoSuchWindow {
                reel,
                symbols: symbols.to_vec(),
            }),
        }
    }

    /// The lowest value-table draw that gives `value`, with the number of
    /// entries that carry it. Entries of weight 0 can never be drawn.
    fn value_draw(&self, value: OrbValue) -> Option<ValueDraw> {
        let (hs, total) = self.feature?;
        let mut offset = 0u32;
        let mut first = None;
        let mut candidates = 0usize;
        for (entry, weighted) in hs.orb_values.iter().enumerate() {
            if weighted.weight > 0 && weighted.value == value {
                candidates += 1;
                first.get_or_insert((offset, entry));
            }
            // The total fits in 32 bits, so every partial sum does.
            offset = offset.saturating_add(weighted.weight);
        }
        first.map(|(value, entry)| ValueDraw {
            draw: Draw {
                bound: total,
                value,
            },
            candidates,
            entry: entry as u32,
        })
    }
}

/// A spin whose events are still arriving.
struct OpenSpin {
    /// `seq` of its `SpinStarted`.
    seq: u64,
    /// The stop chosen for each reel, once it has been seen and matched.
    stops: Vec<Option<u32>>,
    seen: Vec<bool>,
    /// Orb values observed, by position, each with the event that showed it.
    values: BTreeMap<(u8, u8), Vec<(u64, OrbValue)>>,
    broken: bool,
}

struct Scan {
    out: Reconstruction,
    spin: Option<OpenSpin>,
    /// Positions holding an orb in the feature the last spin could start,
    /// reel-major. `None` when they are not known.
    held: Option<Vec<bool>>,
    /// Feature steps observed since the last spin started.
    feature_steps: u32,
}

impl Scan {
    fn inconsistent(&mut self, seq: u64, reason: InconsistencyReason) {
        self.out.inconsistencies.push(Inconsistency { seq, reason });
    }

    /// The command implied at `seq` cannot be scripted.
    fn unscriptable(&mut self, seq: u64) {
        self.out.unscripted_from.get_or_insert(seq);
    }

    fn script(&mut self, seq: u64, command: Command, draws: Vec<Draw>) {
        if self.out.unscripted_from.is_none() {
            self.out.commands.push(ScriptedCommand {
                seq,
                command,
                draws,
            });
        }
    }

    fn orb_value(
        &mut self,
        view: &ModelView,
        seq: u64,
        reel: u8,
        row: u8,
        value: OrbValue,
    ) -> Option<Draw> {
        let Some(found) = view.value_draw(value) else {
            self.inconsistent(
                seq,
                InconsistencyReason::OrbValueNotInTable { reel, row, value },
            );
            return None;
        };
        if found.candidates > 1 {
            self.out.ambiguities.push(Ambiguity {
                seq,
                what: AmbiguousChoice::OrbValueEntry { reel, row },
                candidates: found.candidates,
                chosen: found.entry,
            });
        }
        Some(found.draw)
    }

    fn bet_changed(&mut self, view: &ModelView, seq: u64, credits_per_line: u32) {
        self.close_spin(view);
        let options = &view.model.bet.credits_per_line_options;
        let mut matching = options
            .iter()
            .enumerate()
            .filter(|(_, option)| **option == credits_per_line)
            .map(|(index, _)| index);
        let Some(index) = matching.next() else {
            self.inconsistent(seq, InconsistencyReason::NoSuchBet { credits_per_line });
            self.unscriptable(seq);
            return;
        };
        let candidates = 1 + matching.count();
        if candidates > 1 {
            self.out.ambiguities.push(Ambiguity {
                seq,
                what: AmbiguousChoice::BetOption,
                candidates,
                chosen: u32::try_from(index).unwrap_or(u32::MAX),
            });
        }
        self.script(seq, Command::SetBet { index }, Vec::new());
    }

    fn spin_started(&mut self, view: &ModelView, seq: u64) {
        self.close_spin(view);
        self.spin = Some(OpenSpin {
            seq,
            stops: vec![None; view.reels.len()],
            seen: vec![false; view.reels.len()],
            values: BTreeMap::new(),
            broken: false,
        });
        self.held = None;
        self.feature_steps = 0;
    }

    fn reel_stopped(
        &mut self,
        view: &ModelView,
        seq: u64,
        reel: u8,
        stated: Option<u32>,
        symbols: &[String],
    ) {
        let Some(mut spin) = self.spin.take() else {
            self.inconsistent(seq, InconsistencyReason::ReelOutsideSpin { reel });
            self.unscriptable(seq);
            return;
        };
        let index = reel as usize;
        let found = if spin.seen.get(index).copied().unwrap_or(false) {
            Err(InconsistencyReason::ReelRepeated { reel })
        } else {
            view.find_stop(reel, stated, symbols)
        };
        if let Some(seen) = spin.seen.get_mut(index) {
            *seen = true;
        }
        match found {
            Ok((stop, candidates)) => {
                if let Some(slot) = spin.stops.get_mut(index) {
                    *slot = Some(stop);
                }
                if candidates > 1 {
                    self.out.ambiguities.push(Ambiguity {
                        seq,
                        what: AmbiguousChoice::ReelStop { reel },
                        candidates,
                        chosen: stop,
                    });
                }
            }
            Err(reason) => {
                spin.broken = true;
                self.inconsistent(seq, reason);
            }
        }
        self.spin = Some(spin);
    }

    fn orb_value_shown(&mut self, seq: u64, reel: u8, row: u8, value: OrbValue) {
        // A value shown outside a spin drives nothing in the simulation; the
        // comparison reports it.
        if let Some(spin) = &mut self.spin {
            spin.values
                .entry((reel, row))
                .or_default()
                .push((seq, value));
        }
    }

    /// The open spin has all the events it will get: script it.
    fn close_spin(&mut self, view: &ModelView) {
        let Some(spin) = self.spin.take() else {
            return;
        };
        let mut broken = spin.broken;
        for (reel, seen) in spin.seen.iter().enumerate() {
            if !seen {
                broken = true;
                self.inconsistent(
                    spin.seq,
                    InconsistencyReason::ReelMissing { reel: reel as u8 },
                );
            }
        }
        // Draw order of a spin: one stop per reel, left to right, then one
        // value per orb in the window, reel-major.
        let mut draws = Vec::new();
        let mut value_draws = Vec::new();
        let mut held = Vec::with_capacity(view.model.grid.cells());
        for (reel, (stop, reel_view)) in spin.stops.iter().zip(&view.reels).enumerate() {
            let Some(stop) = *stop else {
                // Already reported, as the reel's own inconsistency or as missing.
                broken = true;
                continue;
            };
            draws.push(Draw {
                bound: reel_view.bound,
                value: stop,
            });
            let len = reel_view.orb.len().max(1);
            for row in 0..view.rows {
                let is_orb = reel_view
                    .orb
                    .get((stop as usize + row) % len)
                    .copied()
                    .unwrap_or(false);
                held.push(is_orb);
                if !is_orb {
                    // A value observed where the model's window has no orb
                    // drives nothing; the comparison reports it.
                    continue;
                }
                let (reel, row) = (reel as u8, row as u8);
                match spin.values.get(&(reel, row)).map(Vec::as_slice) {
                    None | Some([]) => {
                        broken = true;
                        self.inconsistent(
                            spin.seq,
                            InconsistencyReason::OrbValueMissing { reel, row },
                        );
                    }
                    Some([(seq, value)]) => match self.orb_value(view, *seq, reel, row, *value) {
                        Some(draw) => value_draws.push(draw),
                        None => broken = true,
                    },
                    Some([_, (seq, _), ..]) => {
                        broken = true;
                        self.inconsistent(
                            *seq,
                            InconsistencyReason::OrbValueRepeated { reel, row },
                        );
                    }
                }
            }
        }
        if broken {
            self.unscriptable(spin.seq);
            self.held = None;
        } else {
            draws.extend(value_draws);
            self.script(spin.seq, Command::Spin, draws);
            self.held = Some(held);
        }
    }

    /// The held positions a feature step starts from. `None`, with the step
    /// marked unscriptable, when the model has no feature or they are unknown.
    fn feature_state<'a>(
        &mut self,
        view: &ModelView<'a>,
        seq: u64,
    ) -> Option<(&'a HoldAndSpinModel, u32, Vec<bool>)> {
        self.close_spin(view);
        let Some((hs, total)) = view.feature else {
            self.inconsistent(seq, InconsistencyReason::NoFeatureInModel);
            self.unscriptable(seq);
            return None;
        };
        let Some(held) = self.held.take() else {
            // After a spin that could not be scripted this is a consequence,
            // not a second finding.
            if self.out.unscripted_from.is_none() {
                self.inconsistent(seq, InconsistencyReason::FeatureWithoutSpin);
                self.unscriptable(seq);
            }
            return None;
        };
        Some((hs, total, held))
    }

    fn respin(&mut self, view: &ModelView, seq: u64, landed: &[LockedOrb]) {
        let state = self.feature_state(view, seq);
        self.feature_steps = self.feature_steps.saturating_add(1);
        let Some((hs, _, mut held)) = state else {
            return;
        };
        let mut broken = false;
        // Position → the value draw of the orb that landed there.
        let mut landing: BTreeMap<usize, Draw> = BTreeMap::new();
        for orb in landed {
            let (reel, row) = (orb.reel, orb.row);
            let Some(index) = view.model.grid.index(reel, row) else {
                broken = true;
                self.inconsistent(seq, InconsistencyReason::LandedOutsideWindow { reel, row });
                continue;
            };
            if held.get(index).copied().unwrap_or(true) {
                broken = true;
                self.inconsistent(seq, InconsistencyReason::LandedOnHeldPosition { reel, row });
                continue;
            }
            if landing.contains_key(&index) {
                broken = true;
                self.inconsistent(seq, InconsistencyReason::LandedTwice { reel, row });
                continue;
            }
            match self.orb_value(view, seq, reel, row, orb.value) {
                Some(draw) => {
                    landing.insert(index, draw);
                }
                None => broken = true,
            }
        }
        // Draw order of a respin: for each empty position, reel-major, one
        // land draw (an orb lands when it is below the numerator), followed,
        // if it landed, by one value draw.
        let chance = hs.orb_land_chance;
        let rows = view.rows.max(1);
        let mut draws = Vec::new();
        let mut reported_stay = false;
        for index in (0..held.len()).filter(|index| !held[*index]) {
            match landing.get(&index) {
                Some(value_draw) => {
                    if chance.num == 0 {
                        broken = true;
                        self.inconsistent(seq, InconsistencyReason::LandingImpossible);
                    } else {
                        draws.push(Draw {
                            bound: chance.den,
                            value: 0,
                        });
                        draws.push(*value_draw);
                    }
                }
                None => {
                    if chance.num >= chance.den {
                        broken = true;
                        if !reported_stay {
                            reported_stay = true;
                            self.inconsistent(
                                seq,
                                InconsistencyReason::StayingEmptyImpossible {
                                    reel: (index / rows) as u8,
                                    row: (index % rows) as u8,
                                },
                            );
                        }
                    } else {
                        draws.push(Draw {
                            bound: chance.den,
                            value: chance.num,
                        });
                    }
                }
            }
        }
        if broken {
            self.unscriptable(seq);
            return;
        }
        for index in landing.keys() {
            if let Some(cell) = held.get_mut(*index) {
                *cell = true;
            }
        }
        self.held = Some(held);
        self.script(seq, Command::Advance, draws);
    }

    fn feature_completed(&mut self, view: &ModelView, seq: u64) {
        let steps = self.feature_steps;
        self.feature_steps = steps.saturating_add(1);
        if steps > 0 {
            // The respin before it was the command that completed the feature.
            self.close_spin(view);
            return;
        }
        // A feature that completed with no respin observed still takes one
        // command in the simulation, and it draws nothing.
        if let Some((_, _, held)) = self.feature_state(view, seq) {
            self.held = Some(held);
            self.script(seq, Command::Advance, Vec::new());
        }
    }
}

/// Derive, under `model`, the commands and scripted draws that make the
/// simulation produce the outcomes `observation` shows.
///
/// Only outcomes are read: reel windows, orb values, which positions landed
/// an orb, and bet changes. A reel's stated stop is used when the trace has
/// one (a rewrite trace does) and it shows the observed symbols.
///
/// An observation the model cannot have produced is not an error: it is
/// reported in [`Reconstruction::inconsistencies`], and the script ends
/// before the first command it affects. The error case is a model that
/// cannot be run at all.
pub fn reconstruct(model: &MathModel, observation: &Trace) -> Result<Reconstruction, GameError> {
    let view = ModelView::new(model)?;
    let mut scan = Scan {
        out: Reconstruction {
            commands: Vec::new(),
            ambiguities: Vec::new(),
            inconsistencies: Vec::new(),
            unscripted_from: None,
        },
        spin: None,
        held: None,
        feature_steps: 0,
    };
    for record in &observation.records {
        let seq = record.seq;
        match &record.event {
            Event::BetChanged {
                credits_per_line, ..
            } => scan.bet_changed(&view, seq, *credits_per_line),
            Event::SpinStarted { .. } => scan.spin_started(&view, seq),
            Event::ReelStopped {
                reel,
                stop,
                symbols,
            } => scan.reel_stopped(&view, seq, *reel, *stop, symbols),
            Event::OrbValueShown { reel, row, value } => {
                scan.orb_value_shown(seq, *reel, *row, *value);
            }
            Event::HoldAndSpinRespin { landed, .. } => scan.respin(&view, seq, landed),
            Event::HoldAndSpinCompleted { .. } => scan.feature_completed(&view, seq),
            // What the game did with the outcomes. Never used to steer the
            // replay: these are what the comparison checks.
            Event::BetPlaced { .. }
            | Event::CreditsChanged { .. }
            | Event::LineWin { .. }
            | Event::ScatterWin { .. }
            | Event::FeatureTriggered { .. }
            | Event::HoldAndSpinStarted { .. }
            | Event::JackpotAwarded { .. }
            | Event::SpinCompleted { .. } => {}
        }
    }
    scan.close_spin(&view);
    Ok(scan.out)
}

/// What a replay of a trace of `origin` through a model of `provenance`
/// amounts to. Only recovered cabinet mathematics can make it parity.
pub fn replay_kind(origin: TraceOrigin, provenance: MathProvenance) -> ComparisonKind {
    match origin {
        TraceOrigin::Rewrite => ComparisonKind::Determinism,
        TraceOrigin::Observation if provenance.is_cabinet_math_evidence() => ComparisonKind::Parity,
        TraceOrigin::Observation => match provenance {
            MathProvenance::SyntheticDevFixture => ComparisonKind::MachineryOnly,
            _ => ComparisonKind::RulesUnderModel,
        },
    }
}

/// Why a replay ended before the end of its script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StopReason {
    /// The simulation refused the command: it was not in the state the
    /// observation implies, or it asked for a draw the observation gives no
    /// outcome for.
    Refused { error: String },
    /// The simulation carried the command out without asking for every draw
    /// scripted for it.
    DrawsLeftOver { scripted: usize, consumed: usize },
}

/// The command at which a replay stopped. A finding: our simulation and the
/// observation disagree about what could happen next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayStop {
    /// The observation event that implied the command.
    pub seq: u64,
    pub command: Command,
    pub reason: StopReason,
}

/// The result of replaying an observation's outcomes through a model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReplayReport {
    pub model_id: String,
    pub model_provenance: MathProvenance,
    /// Credits the replayed game started with.
    pub starting_credits: u64,
    /// Whether they were given or read from the observation's first credit
    /// event, rather than being [`UNOBSERVED_STARTING_CREDITS`].
    pub starting_credits_known: bool,
    /// Commands scripted from the observation.
    pub commands: usize,
    /// Draws scripted for them.
    pub draws_reconstructed: usize,
    /// Draws the simulation asked for.
    pub draws_consumed: usize,
    /// Whether every scripted draw was asked for.
    pub all_draws_consumed: bool,
    pub ambiguities: Vec<Ambiguity>,
    pub inconsistencies: Vec<Inconsistency>,
    /// See [`Reconstruction::unscripted_from`].
    pub unscripted_from: Option<u64>,
    /// Set when the simulation could not follow the script to its end.
    pub stopped: Option<ReplayStop>,
    /// The observation (`a`) against the replayed rewrite trace (`b`). Its
    /// kind is [`replay_kind`]. When the replay is not complete the rewrite
    /// is shorter than the observation, and a divergence where `b` ends is
    /// where the replay ended, not a difference in rules.
    pub comparison: Comparison,
    /// What the simulation emitted on the scripted outcomes.
    #[serde(skip)]
    pub rewrite: Trace,
}

impl ReplayReport {
    /// Whether the whole observation was scripted and the simulation followed
    /// the script to its end, consuming every draw.
    pub fn is_complete(&self) -> bool {
        self.unscripted_from.is_none() && self.stopped.is_none() && self.all_draws_consumed
    }

    /// Whether the replay is complete and the simulation emitted the events
    /// that were observed. What that agreement is worth is
    /// `self.comparison.kind`.
    pub fn agrees(&self) -> bool {
        self.is_complete() && self.comparison.same_events()
    }
}

/// Replay the outcomes `observation` shows through `model` and compare what
/// the simulation emits with the observation.
///
/// The replayed game starts with `credits` play credits; when that is `None`,
/// with the amount the observation's first credit event starts from. It
/// starts on the model's first bet option: an observation of a game on
/// another bet must begin with a `BetChanged` event.
///
/// Each command gets its own [`ScriptedRng`] holding exactly the draws
/// reconstructed for it, so a command that asks for more is refused with the
/// state untouched, and one that asks for fewer is noticed at once.
///
/// The error case is a model that cannot be run. Everything about the
/// observation, including a script the simulation cannot follow, is in the
/// report.
pub fn replay(
    model: &MathModel,
    observation: &Trace,
    credits: Option<u64>,
) -> Result<ReplayReport, GameError> {
    let reconstruction = reconstruct(model, observation)?;
    let observed_credits = observation
        .records
        .iter()
        .find_map(|record| match record.event {
            Event::CreditsChanged { from, .. } => Some(from),
            _ => None,
        });
    let known_credits = credits.or(observed_credits);
    let starting_credits = known_credits.unwrap_or(UNOBSERVED_STARTING_CREDITS);

    let mut game = Game::new(model.clone(), starting_credits)?;
    let mut records: Vec<TraceRecord> = Vec::new();
    let mut draws_consumed = 0usize;
    let mut stopped = None;
    for step in &reconstruction.commands {
        let mut rng = ScriptedRng::new(step.draws.clone());
        let events = match game.apply(step.command, &mut rng) {
            Ok(events) => events,
            Err(error) => {
                stopped = Some(ReplayStop {
                    seq: step.seq,
                    command: step.command,
                    reason: StopReason::Refused {
                        error: error.to_string(),
                    },
                });
                break;
            }
        };
        for event in events {
            records.push(TraceRecord {
                seq: records.len() as u64,
                t_ms: None,
                event,
            });
        }
        let consumed = step.draws.len() - rng.remaining();
        draws_consumed += consumed;
        if rng.remaining() > 0 {
            stopped = Some(ReplayStop {
                seq: step.seq,
                command: step.command,
                reason: StopReason::DrawsLeftOver {
                    scripted: step.draws.len(),
                    consumed,
                },
            });
            break;
        }
    }

    let rewrite = Trace {
        header: TraceHeader {
            schema: SCHEMA.to_string(),
            origin: TraceOrigin::Rewrite,
            title: observation.header.title,
            scope: None,
            math_id: Some(model.id.clone()),
            math_provenance: Some(model.provenance),
            rng: Some(ScriptedRng::NAME.to_string()),
            seed: None,
            source_class: source_for(model.provenance),
            capture_sha256: None,
            notes: format!(
                "{}. Replay: outcomes were scripted from another trace, not drawn.",
                model.provenance.banner()
            ),
        },
        records,
    };
    let mut comparison = compare(observation, &rewrite, 0);
    comparison.kind = replay_kind(observation.header.origin, model.provenance);

    let draws_reconstructed = reconstruction.draw_count();
    Ok(ReplayReport {
        model_id: model.id.clone(),
        model_provenance: model.provenance,
        starting_credits,
        starting_credits_known: known_credits.is_some(),
        commands: reconstruction.commands.len(),
        draws_reconstructed,
        draws_consumed,
        all_draws_consumed: draws_consumed == draws_reconstructed,
        ambiguities: reconstruction.ambiguities,
        inconsistencies: reconstruction.inconsistencies,
        unscripted_from: reconstruction.unscripted_from,
        stopped,
        comparison,
        rewrite,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{as_observation, rewrite};
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    /// Long enough to hold many features, and every stop of every reel.
    const SPINS: u64 = 3000;

    fn observed(seed: u64) -> Trace {
        as_observation(&rewrite(seed, SPINS), 100)
    }

    /// The first inconsistency reconstruction finds in `trace`.
    fn first_reason(trace: &Trace) -> InconsistencyReason {
        let found = reconstruct(&synthetic_dev_fixture(), trace).unwrap();
        assert!(!found.is_complete());
        found.inconsistencies[0].reason.clone()
    }

    /// Index of the first record of `trace` the predicate holds for.
    fn position(trace: &Trace, wanted: impl Fn(&Event) -> bool) -> usize {
        trace.records.iter().position(|r| wanted(&r.event)).unwrap()
    }

    #[test]
    fn an_observation_of_our_own_game_replays_to_the_same_events() {
        let model = synthetic_dev_fixture();
        let observation = observed(11);
        // The stretch holds everything a replay has to follow.
        for wanted in ["orb_value_shown", "hold_and_spin_respin", "jackpot_awarded"] {
            assert!(
                observation.records.iter().any(|r| r.event.kind() == wanted),
                "{wanted}"
            );
        }
        assert!(
            observation
                .records
                .iter()
                .all(|r| !matches!(r.event, Event::ReelStopped { stop: Some(_), .. }))
        );

        let report = replay(&model, &observation, None).unwrap();
        assert!(report.inconsistencies.is_empty());
        assert!(report.is_complete());
        assert!(report.agrees(), "{:?}", report.comparison.first_divergence);
        assert_eq!(report.comparison.matched, observation.records.len());
        assert_eq!(report.rewrite.records.len(), observation.records.len());
        assert_eq!(report.draws_consumed, report.draws_reconstructed);
        assert!(report.draws_consumed as u64 >= SPINS * 5);
        // The meter was read from the observation's first credit movement.
        assert!(report.starting_credits_known);
        assert_eq!(report.starting_credits, 1_000_000);
        // Through the synthetic fixture this is a test of the machinery, not parity.
        assert_eq!(report.comparison.kind, ComparisonKind::MachineryOnly);
        assert_eq!(report.model_provenance, MathProvenance::SyntheticDevFixture);
        report.rewrite.validate().unwrap();
        assert_eq!(report.rewrite.header.origin, TraceOrigin::Rewrite);
        assert_eq!(report.rewrite.header.rng.as_deref(), Some("scripted"));
    }

    #[test]
    fn a_rewrite_trace_replays_to_itself_exactly_and_that_is_determinism() {
        let model = synthetic_dev_fixture();
        let original = rewrite(5, SPINS);
        let report = replay(&model, &original, None).unwrap();
        assert!(report.agrees());
        assert_eq!(report.comparison.kind, ComparisonKind::Determinism);
        // Stated stops were used, so the events are the same to the last field.
        let events = |t: &Trace| {
            t.records
                .iter()
                .map(|r| r.event.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(events(&report.rewrite), events(&original));
        // And no reel stop was left open.
        assert!(
            !report
                .ambiguities
                .iter()
                .any(|a| matches!(a.what, AmbiguousChoice::ReelStop { .. }))
        );
    }

    #[test]
    fn hidden_choices_are_recorded_and_do_not_change_what_is_seen() {
        let model = synthetic_dev_fixture();
        let view = ModelView::new(&model).unwrap();
        // Does any strip show the same window at two stops? Then an observer
        // cannot tell which stop was drawn, and replay must say so.
        let repeated_window = view.reels.iter().any(|reel| {
            let len = reel.keys.len();
            let window = |stop: usize| (0..view.rows).map(move |row| reel.keys[(stop + row) % len]);
            (0..len).any(|a| (a + 1..len).any(|b| window(a).eq(window(b))))
        });
        let report = replay(&model, &observed(7), None).unwrap();
        assert!(report.agrees());
        let stop_choices: Vec<_> = report
            .ambiguities
            .iter()
            .filter(|a| matches!(a.what, AmbiguousChoice::ReelStop { .. }))
            .collect();
        assert_eq!(!stop_choices.is_empty(), repeated_window);
        assert!(stop_choices.iter().all(|a| a.candidates > 1));

        // A value carried by two table entries is a recorded choice as well.
        let mut doubled = model.clone();
        let hs = doubled.hold_and_spin.as_mut().unwrap();
        let first = hs.orb_values[0].clone();
        hs.orb_values.push(first);
        let report = replay(&doubled, &observed(7), None).unwrap();
        assert!(report.is_complete());
        assert!(report.ambiguities.iter().any(|a| matches!(
            a.what,
            AmbiguousChoice::OrbValueEntry { .. }
        ) && a.candidates == 2
            && a.chosen == 0));
    }

    #[test]
    fn a_difference_in_rules_shows_as_a_difference_in_events_not_in_outcomes() {
        let observation = observed(11);
        // The same reels and tables, with one rule changed: every line pay doubled.
        let mut other = synthetic_dev_fixture();
        for pay in &mut other.line_pays {
            pay.pay *= 2;
        }
        let report = replay(&other, &observation, None).unwrap();
        // Every outcome could be scripted and followed: the luck is the same.
        assert!(report.inconsistencies.is_empty());
        assert!(report.is_complete());
        // What differs is an award.
        assert!(!report.agrees());
        let divergence = report.comparison.first_divergence.as_ref().unwrap();
        match (&divergence.a, &divergence.b) {
            (Some(Event::LineWin { .. }), Some(Event::LineWin { .. })) => {}
            other => panic!("expected the first line win to differ, got {other:?}"),
        }

        // A model in which the feature needs one orb more: the observed
        // feature steps cannot be followed, and the replay says where.
        let mut harder = synthetic_dev_fixture();
        harder.hold_and_spin.as_mut().unwrap().trigger_count += 1;
        let report = replay(&harder, &observation, None).unwrap();
        assert!(!report.agrees());
        assert!(report.stopped.is_some() || !report.comparison.same_events());
    }

    #[test]
    fn an_observation_the_model_cannot_have_produced_is_a_finding_not_a_guess() {
        let model = synthetic_dev_fixture();
        let view = ModelView::new(&model).unwrap();
        // A window of real symbols that no stop of reel 0 shows.
        let keys: Vec<String> = model.symbols.iter().map(|s| s.key.clone()).collect();
        let absent = keys
            .iter()
            .flat_map(|a| {
                keys.iter()
                    .map(move |b| vec![a.clone(), b.clone(), a.clone()])
            })
            .find(|window| view.find_stop(0, None, window).is_err())
            .unwrap();

        let mut observation = observed(3);
        // The first reel of the fifth spin.
        let spin_starts: Vec<usize> = observation
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r.event, Event::SpinStarted { .. }))
            .map(|(index, _)| index)
            .collect();
        let at = spin_starts[4]
            + position(
                &Trace {
                    header: observation.header.clone(),
                    records: observation.records[spin_starts[4]..].to_vec(),
                },
                |e| matches!(e, Event::ReelStopped { reel: 0, .. }),
            );
        if let Event::ReelStopped { symbols, .. } = &mut observation.records[at].event {
            *symbols = absent.clone();
        }

        let report = replay(&model, &observation, None).unwrap();
        assert_eq!(report.inconsistencies.len(), 1);
        assert_eq!(report.inconsistencies[0].seq, at as u64);
        assert_eq!(
            report.inconsistencies[0].reason,
            InconsistencyReason::NoSuchWindow {
                reel: 0,
                symbols: absent
            }
        );
        // Nothing from that spin on is replayed, and nothing is invented for it.
        assert_eq!(report.unscripted_from, Some(spin_starts[4] as u64));
        assert!(!report.is_complete());
        assert!(!report.agrees());
        // The replay holds the four spins before it, whole, and stops there.
        let spins_replayed = report
            .rewrite
            .records
            .iter()
            .filter(|r| matches!(r.event, Event::SpinStarted { .. }))
            .count();
        assert_eq!(spins_replayed, 4);
        assert!(matches!(
            report.rewrite.records.last().map(|r| &r.event),
            Some(Event::SpinCompleted { .. })
        ));
        assert_eq!(report.comparison.matched, report.rewrite.records.len());
        assert!(report.comparison.matched <= spin_starts[4]);
        assert!(report.stopped.is_none());
    }

    #[test]
    fn each_kind_of_impossible_observation_is_named() {
        let base = observed(3);
        let reel_at = position(&base, |e| matches!(e, Event::ReelStopped { reel: 2, .. }));

        let mut trace = base.clone();
        if let Event::ReelStopped { symbols, .. } = &mut trace.records[reel_at].event {
            symbols[1] = "NOT-A-SYMBOL".to_string();
        }
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::UnknownSymbol { reel: 2, .. }
        ));

        let mut trace = base.clone();
        if let Event::ReelStopped { symbols, .. } = &mut trace.records[reel_at].event {
            symbols.pop();
        }
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::WindowHeight {
                reel: 2,
                observed: 2,
                rows: 3
            }
        );

        let mut trace = base.clone();
        if let Event::ReelStopped { reel, .. } = &mut trace.records[reel_at].event {
            *reel = 9;
        }
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::NoSuchReel { reel: 9, reels: 5 }
        ));

        // The same reel twice, and so another reel never.
        let mut trace = base.clone();
        if let Event::ReelStopped { reel, .. } = &mut trace.records[reel_at].event {
            *reel = 1;
        }
        let found = reconstruct(&synthetic_dev_fixture(), &trace).unwrap();
        let reasons: Vec<_> = found.inconsistencies.iter().map(|i| &i.reason).collect();
        assert!(reasons.contains(&&InconsistencyReason::ReelRepeated { reel: 1 }));
        assert!(reasons.contains(&&InconsistencyReason::ReelMissing { reel: 2 }));

        // A reel that stops before any spin has started.
        let mut trace = base.clone();
        let start = position(&trace, |e| matches!(e, Event::SpinStarted { .. }));
        trace.records.remove(start);
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::ReelOutsideSpin { reel: 0 }
        );
        let mut trace = base.clone();
        let stray = trace.records[reel_at].clone();
        trace.records.insert(0, stray);
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::ReelOutsideSpin { reel: 2 }
        );

        // An orb in the window with no value seen, and one with two.
        let value_at = position(&base, |e| matches!(e, Event::OrbValueShown { .. }));
        let mut trace = base.clone();
        trace.records.remove(value_at);
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::OrbValueMissing { .. }
        ));
        let mut trace = base.clone();
        let again = trace.records[value_at].clone();
        trace.records.insert(value_at, again);
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::OrbValueRepeated { .. }
        ));
        let mut trace = base.clone();
        if let Event::OrbValueShown { value, .. } = &mut trace.records[value_at].event {
            *value = OrbValue::BetMultiple(4_000_000);
        }
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::OrbValueNotInTable { .. }
        ));

        // A bet the model does not offer.
        let mut trace = base.clone();
        trace.records.insert(
            0,
            TraceRecord {
                seq: 0,
                t_ms: None,
                event: Event::BetChanged {
                    credits_per_line: 4_000_000,
                    total_bet: 4_000_000,
                },
            },
        );
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::NoSuchBet {
                credits_per_line: 4_000_000
            }
        );

        // A stated stop that does not show the window written beside it.
        let mut trace = rewrite(3, 50);
        let at = position(&trace, |e| matches!(e, Event::ReelStopped { .. }));
        if let Event::ReelStopped { symbols, .. } = &mut trace.records[at].event {
            symbols.reverse();
            symbols[0] = "NOT-A-SYMBOL".to_string();
        }
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::UnknownSymbol { .. }
                | InconsistencyReason::StatedStopDoesNotMatch { .. }
        ));
    }

    #[test]
    fn impossible_feature_steps_are_named() {
        let base = observed(11);
        let respin_at = position(
            &base,
            |e| matches!(e, Event::HoldAndSpinRespin { landed, .. } if !landed.is_empty()),
        );

        let mut trace = base.clone();
        if let Event::HoldAndSpinRespin { landed, .. } = &mut trace.records[respin_at].event {
            landed[0].reel = 9;
        }
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::LandedOutsideWindow {
                reel: 9,
                row: match &base.records[respin_at].event {
                    Event::HoldAndSpinRespin { landed, .. } => landed[0].row,
                    _ => unreachable!(),
                }
            }
        );

        let mut trace = base.clone();
        if let Event::HoldAndSpinRespin { landed, .. } = &mut trace.records[respin_at].event {
            let again = landed[0];
            landed.push(again);
        }
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::LandedTwice { .. }
        ));

        // The same orbs landing again on the next step: they are held by then.
        let mut trace = base.clone();
        let again = trace.records[respin_at].clone();
        trace.records.insert(respin_at + 1, again);
        assert!(matches!(
            first_reason(&trace),
            InconsistencyReason::LandedOnHeldPosition { .. }
        ));

        // A feature step with no spin before it.
        let mut trace = base.clone();
        trace.records = trace.records.split_off(respin_at);
        assert_eq!(
            first_reason(&trace),
            InconsistencyReason::FeatureWithoutSpin
        );

        // A model without the feature.
        let mut plain = synthetic_dev_fixture();
        plain.hold_and_spin = None;
        let found = reconstruct(&plain, &base).unwrap();
        assert!(
            found
                .inconsistencies
                .iter()
                .any(|i| i.reason == InconsistencyReason::NoFeatureInModel)
        );

        // Land chances under which what was seen cannot happen.
        let mut never = synthetic_dev_fixture();
        never.hold_and_spin.as_mut().unwrap().orb_land_chance.num = 0;
        let found = reconstruct(&never, &base).unwrap();
        assert!(
            found
                .inconsistencies
                .iter()
                .any(|i| i.reason == InconsistencyReason::LandingImpossible)
        );
        let mut always = synthetic_dev_fixture();
        let chance = &mut always.hold_and_spin.as_mut().unwrap().orb_land_chance;
        chance.num = chance.den;
        let found = reconstruct(&always, &base).unwrap();
        assert!(
            found
                .inconsistencies
                .iter()
                .any(|i| matches!(i.reason, InconsistencyReason::StayingEmptyImpossible { .. }))
        );
    }

    #[test]
    fn a_replay_says_what_it_amounts_to() {
        use ComparisonKind::{Determinism, MachineryOnly, Parity, RulesUnderModel};
        use MathProvenance::{Cabinet, OfficialMobile, Regulatory, SyntheticDevFixture};
        for provenance in [SyntheticDevFixture, OfficialMobile, Regulatory, Cabinet] {
            assert_eq!(replay_kind(TraceOrigin::Rewrite, provenance), Determinism);
        }
        let of = |provenance| replay_kind(TraceOrigin::Observation, provenance);
        assert_eq!(of(SyntheticDevFixture), MachineryOnly);
        // Mobile mathematics is not evidence of cabinet mathematics.
        assert_eq!(of(OfficialMobile), RulesUnderModel);
        assert_eq!(of(Cabinet), Parity);
        assert_eq!(
            of(Regulatory) == Parity,
            Regulatory.is_cabinet_math_evidence()
        );
    }

    #[test]
    fn credits_are_given_read_or_admitted_to_be_unknown() {
        let model = synthetic_dev_fixture();
        let observation = observed(2);
        let given = replay(&model, &observation, Some(1_000_000)).unwrap();
        assert!(given.starting_credits_known && given.agrees());
        // The wrong starting meter is a difference the comparison finds.
        let wrong = replay(&model, &observation, Some(999)).unwrap();
        assert!(!wrong.agrees());
        // An observation with no credit movement in it at all.
        let mut silent = observation.clone();
        silent
            .records
            .retain(|r| !matches!(r.event, Event::CreditsChanged { .. }));
        let report = replay(&model, &silent, None).unwrap();
        assert!(!report.starting_credits_known);
        assert_eq!(report.starting_credits, UNOBSERVED_STARTING_CREDITS);
        assert!(report.is_complete());
    }

    #[test]
    fn an_empty_observation_replays_to_nothing() {
        let mut empty = observed(1);
        empty.records.clear();
        let report = replay(&synthetic_dev_fixture(), &empty, None).unwrap();
        assert!(report.agrees());
        assert_eq!(report.commands, 0);
        assert_eq!(report.draws_consumed, 0);
    }
}
