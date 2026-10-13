//! Behavioural traces: what happened, in order, with optional times.
//!
//! One format serves both sides of a parity comparison (`docs/PARITY.md`):
//!
//! - a **rewrite** trace is what `dragonlink-core` emitted, with the math
//!   model's provenance and the outcome source named in the header;
//! - an **observation** trace is a transcription of a recording of the
//!   original: the same events, with what could be seen. Fields an observer
//!   cannot see (a reel's stop index) are left out.
//!
//! A trace is JSON Lines: a header object, then one record per line. The
//! header says what kind of trace it is, and [`compare`] says what kind of
//! comparison two traces amount to. Comparing two rewrite traces is a
//! determinism check and is labelled as such; it is never parity.
//!
//! [`replay`] goes one step further than lining two traces up: it drives the
//! simulation with the outcomes an observation shows, so that what still
//! differs is a difference in rules and not in luck. See the [`replay`
//! module](mod@replay) for what that does and does not establish.

pub mod replay;
pub mod transcript;

pub use replay::{
    Ambiguity, AmbiguousChoice, Inconsistency, InconsistencyReason, Reconstruction, ReplayReport,
    ReplayStop, ScriptedCommand, StopReason, UNOBSERVED_STARTING_CREDITS, reconstruct, replay,
    replay_kind,
};

pub use transcript::{
    MeterCheck, ObservationSource, Selection, SheetError, SheetRecord, SheetSummary, parse_sheet,
    sheet_to_trace,
};

use dragonlink_core::{DevRng, Event, Game, GameError};
use dragonlink_model::{MathModel, MathProvenance, SourceClass, TitleId};
use serde::{Deserialize, Serialize};

/// Identifier of the trace format.
pub const SCHEMA: &str = "dragonlink-trace/1";

/// Which side of a comparison a trace is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOrigin {
    /// Emitted by this project's simulation.
    Rewrite,
    /// Transcribed from a recording of an original game.
    Observation,
}

/// The first line of a trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceHeader {
    pub schema: String,
    pub origin: TraceOrigin,
    #[serde(default)]
    pub title: Option<TitleId>,
    /// `cabinet-au`, `cabinet-us`, `mobile`, … for an observation.
    #[serde(default)]
    pub scope: Option<String>,
    /// Rewrite: the math model's id and provenance, and the outcome source.
    #[serde(default)]
    pub math_id: Option<String>,
    #[serde(default)]
    pub math_provenance: Option<MathProvenance>,
    #[serde(default)]
    pub rng: Option<String>,
    #[serde(default)]
    pub seed: Option<u64>,
    /// Where the events came from.
    pub source_class: SourceClass,
    /// Observation: SHA-256 of the recording the events were transcribed from.
    #[serde(default)]
    pub capture_sha256: Option<String>,
    #[serde(default)]
    pub notes: String,
}

/// One event of a trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceRecord {
    /// Position in the trace, from 0.
    pub seq: u64,
    /// Milliseconds since the start of the trace, when known. The simulation
    /// has no clock, so a headless rewrite trace has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t_ms: Option<u64>,
    pub event: Event,
}

/// A whole trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub header: TraceHeader,
    pub records: Vec<TraceRecord>,
}

/// Why a trace is not acceptable.
#[derive(Debug, thiserror::Error)]
pub enum TraceError {
    #[error("the trace is empty")]
    Empty,
    #[error("line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("schema {0:?} is not {SCHEMA:?}")]
    Schema(String),
    #[error("record {index} has seq {seq}; records must be numbered from 0 without gaps")]
    Sequence { index: usize, seq: u64 },
    #[error("record {seq} is earlier in time than the record before it")]
    TimeGoesBack { seq: u64 },
    #[error(
        "a rewrite trace must name its math model, the model's provenance and its outcome source"
    )]
    RewriteWithoutModel,
    #[error("a rewrite trace's source class must match its math model's provenance")]
    RewriteSourceMismatch,
    #[error(
        "an observation trace must come from a recording, official mobile software or a cabinet, not {0:?}"
    )]
    ObservationSource(SourceClass),
    #[error("an observation trace must give the SHA-256 of the recording it was transcribed from")]
    ObservationWithoutCapture,
    #[error(
        "an observation trace cannot name a math model: the original's model is not what was observed"
    )]
    ObservationWithModel,
}

fn source_for(provenance: MathProvenance) -> SourceClass {
    match provenance {
        MathProvenance::SyntheticDevFixture => SourceClass::SyntheticFixture,
        MathProvenance::OfficialMobile => SourceClass::OfficialMobile,
        MathProvenance::Regulatory => SourceClass::Regulatory,
        MathProvenance::Cabinet => SourceClass::Cabinet,
    }
}

impl Trace {
    /// Check the header rules and the numbering and timing of the records.
    pub fn validate(&self) -> Result<(), TraceError> {
        let h = &self.header;
        if h.schema != SCHEMA {
            return Err(TraceError::Schema(h.schema.clone()));
        }
        match h.origin {
            TraceOrigin::Rewrite => {
                let (Some(_), Some(provenance), Some(_)) = (&h.math_id, h.math_provenance, &h.rng)
                else {
                    return Err(TraceError::RewriteWithoutModel);
                };
                if h.source_class != source_for(provenance) {
                    return Err(TraceError::RewriteSourceMismatch);
                }
            }
            TraceOrigin::Observation => {
                if !matches!(
                    h.source_class,
                    SourceClass::GameplayRecording
                        | SourceClass::OfficialMobile
                        | SourceClass::Cabinet
                ) {
                    return Err(TraceError::ObservationSource(h.source_class));
                }
                if h.capture_sha256
                    .as_deref()
                    .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(TraceError::ObservationWithoutCapture);
                }
                if h.math_id.is_some() || h.math_provenance.is_some() {
                    return Err(TraceError::ObservationWithModel);
                }
            }
        }
        let mut last_time = None;
        for (index, record) in self.records.iter().enumerate() {
            if record.seq != index as u64 {
                return Err(TraceError::Sequence {
                    index,
                    seq: record.seq,
                });
            }
            if let Some(t) = record.t_ms {
                if last_time.is_some_and(|last| t < last) {
                    return Err(TraceError::TimeGoesBack { seq: record.seq });
                }
                last_time = Some(t);
            }
        }
        Ok(())
    }

    /// The trace as JSON Lines.
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut out = serde_json::to_string(&self.header)?;
        out.push('\n');
        for record in &self.records {
            out.push_str(&serde_json::to_string(record)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse and validate a trace.
    pub fn from_jsonl(text: &str) -> Result<Trace, TraceError> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let (line, first) = lines.next().ok_or(TraceError::Empty)?;
        let header: TraceHeader =
            serde_json::from_str(first).map_err(|source| TraceError::Json {
                line: line + 1,
                source,
            })?;
        let records = lines
            .map(|(line, text)| {
                serde_json::from_str(text).map_err(|source| TraceError::Json {
                    line: line + 1,
                    source,
                })
            })
            .collect::<Result<Vec<TraceRecord>, _>>()?;
        let trace = Trace { header, records };
        trace.validate()?;
        Ok(trace)
    }

    /// Whether this trace says anything about an original game.
    pub fn is_about_the_original(&self) -> bool {
        self.header.origin == TraceOrigin::Observation
    }
}

/// Run `spins` spins of `model` from `seed`, playing features to their end,
/// and return the events as a rewrite trace. The trace has no times.
pub fn record_rewrite(
    model: &MathModel,
    title: Option<TitleId>,
    seed: u64,
    spins: u64,
    credits: u64,
) -> Result<Trace, GameError> {
    let mut game = Game::new(model.clone(), credits)?;
    let mut rng = DevRng::new(seed);
    let mut records = Vec::new();
    for _ in 0..spins {
        for event in game.play_spin(&mut rng)? {
            records.push(TraceRecord {
                seq: records.len() as u64,
                t_ms: None,
                event,
            });
        }
    }
    Ok(Trace {
        header: TraceHeader {
            schema: SCHEMA.to_string(),
            origin: TraceOrigin::Rewrite,
            title,
            scope: None,
            math_id: Some(model.id.clone()),
            math_provenance: Some(model.provenance),
            rng: Some(DevRng::NAME.to_string()),
            seed: Some(seed),
            source_class: source_for(model.provenance),
            capture_sha256: None,
            notes: model.provenance.banner().to_string(),
        },
        records,
    })
}

/// What a comparison of two traces amounts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonKind {
    /// Two rewrite traces: a check that our simulation is deterministic. Not parity.
    Determinism,
    /// An observation against a rewrite run on recovered mathematics. A
    /// replay is this only when its model is cabinet or regulatory evidence,
    /// and then it concerns the rules and the order of events on the outcomes
    /// observed, not how outcomes are distributed.
    Parity,
    /// An observation against a rewrite run on synthetic or mobile-derived
    /// mathematics: the order and kinds of events can be compared, the
    /// outcomes cannot be expected to match.
    StructureOnly,
    /// Two observations: agreement between recordings.
    BetweenObservations,
    /// A replay of an observation's outcomes through a model that is neither
    /// synthetic nor evidence of cabinet mathematics (a mobile-derived one):
    /// a check of our rules under that model's reels and tables. Not parity
    /// with the cabinet game.
    RulesUnderModel,
    /// A replay through the synthetic fixture: a test of the replay
    /// machinery only. It says nothing about any original game.
    MachineryOnly,
}

/// Where two traces first differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Divergence {
    pub seq: u64,
    pub a: Option<Event>,
    pub b: Option<Event>,
}

/// Differences in event times, where both traces have them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimingStats {
    /// Matched events that carry a time in both traces.
    pub compared: usize,
    pub mean_abs_diff_ms: f64,
    pub max_abs_diff_ms: u64,
    /// Events whose times differ by more than the tolerance.
    pub beyond_tolerance: usize,
    pub tolerance_ms: u64,
}

/// The result of comparing two traces event by event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Comparison {
    pub kind: ComparisonKind,
    pub len_a: usize,
    pub len_b: usize,
    /// Leading events that match.
    pub matched: usize,
    pub first_divergence: Option<Divergence>,
    pub timing: Option<TimingStats>,
}

impl Comparison {
    /// Whether the two traces contain the same events in the same order.
    pub fn same_events(&self) -> bool {
        self.first_divergence.is_none()
    }
}

/// The event as an outside observer could record it: without fields that
/// cannot be seen.
pub fn observable(event: &Event) -> Event {
    match event {
        Event::ReelStopped { reel, symbols, .. } => Event::ReelStopped {
            reel: *reel,
            stop: None,
            symbols: symbols.clone(),
        },
        other => other.clone(),
    }
}

fn kind_of(a: &Trace, b: &Trace) -> ComparisonKind {
    let recovered = |t: &Trace| {
        t.header
            .math_provenance
            .is_some_and(MathProvenance::is_cabinet_math_evidence)
    };
    match (a.header.origin, b.header.origin) {
        (TraceOrigin::Rewrite, TraceOrigin::Rewrite) => ComparisonKind::Determinism,
        (TraceOrigin::Observation, TraceOrigin::Observation) => ComparisonKind::BetweenObservations,
        (TraceOrigin::Observation, TraceOrigin::Rewrite) if recovered(b) => ComparisonKind::Parity,
        (TraceOrigin::Rewrite, TraceOrigin::Observation) if recovered(a) => ComparisonKind::Parity,
        _ => ComparisonKind::StructureOnly,
    }
}

/// Compare two traces event by event. When either is an observation, only
/// what an observer can see is compared. `timing_tolerance_ms` is the
/// difference in event time that counts as a mismatch; it should come from
/// the recording's frame interval, not be chosen to make a comparison pass.
pub fn compare(a: &Trace, b: &Trace, timing_tolerance_ms: u64) -> Comparison {
    let observed = a.is_about_the_original() || b.is_about_the_original();
    let view = |e: &Event| if observed { observable(e) } else { e.clone() };
    let matched = a
        .records
        .iter()
        .zip(&b.records)
        .take_while(|(x, y)| view(&x.event) == view(&y.event))
        .count();
    let first_divergence =
        (matched < a.records.len() || matched < b.records.len()).then(|| Divergence {
            seq: matched as u64,
            a: a.records.get(matched).map(|r| r.event.clone()),
            b: b.records.get(matched).map(|r| r.event.clone()),
        });
    let diffs: Vec<u64> = a
        .records
        .iter()
        .zip(&b.records)
        .take(matched)
        .filter_map(|(x, y)| Some(x.t_ms?.abs_diff(y.t_ms?)))
        .collect();
    let timing = (!diffs.is_empty()).then(|| TimingStats {
        compared: diffs.len(),
        mean_abs_diff_ms: diffs.iter().sum::<u64>() as f64 / diffs.len() as f64,
        max_abs_diff_ms: diffs.iter().copied().max().unwrap_or(0),
        beyond_tolerance: diffs.iter().filter(|d| **d > timing_tolerance_ms).count(),
        tolerance_ms: timing_tolerance_ms,
    });
    Comparison {
        kind: kind_of(a, b),
        len_a: a.records.len(),
        len_b: b.records.len(),
        matched,
        first_divergence,
        timing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    pub(crate) fn rewrite(seed: u64, spins: u64) -> Trace {
        record_rewrite(&synthetic_dev_fixture(), None, seed, spins, 1_000_000).unwrap()
    }

    /// A rewrite trace relabelled as a transcription, with stop indices removed
    /// and times added, as an observer would produce it.
    pub(crate) fn as_observation(trace: &Trace, ms_per_event: u64) -> Trace {
        Trace {
            header: TraceHeader {
                origin: TraceOrigin::Observation,
                scope: Some("cabinet-au".into()),
                math_id: None,
                math_provenance: None,
                rng: None,
                seed: None,
                source_class: SourceClass::GameplayRecording,
                capture_sha256: Some("0".repeat(64)),
                ..trace.header.clone()
            },
            records: trace
                .records
                .iter()
                .map(|r| TraceRecord {
                    seq: r.seq,
                    t_ms: Some(r.seq * ms_per_event),
                    event: observable(&r.event),
                })
                .collect(),
        }
    }

    #[test]
    fn a_rewrite_trace_is_labelled_with_its_model_and_round_trips() {
        let trace = rewrite(9, 50);
        trace.validate().unwrap();
        assert_eq!(trace.header.source_class, SourceClass::SyntheticFixture);
        assert_eq!(
            trace.header.math_provenance,
            Some(MathProvenance::SyntheticDevFixture)
        );
        assert!(trace.header.notes.contains("SYNTHETIC"));
        assert!(!trace.is_about_the_original());
        let text = trace.to_jsonl().unwrap();
        assert_eq!(text.lines().count(), trace.records.len() + 1);
        assert_eq!(Trace::from_jsonl(&text).unwrap(), trace);
    }

    #[test]
    fn the_same_seed_gives_the_same_trace_and_that_is_only_determinism() {
        let c = compare(&rewrite(4, 200), &rewrite(4, 200), 0);
        assert_eq!(c.kind, ComparisonKind::Determinism);
        assert!(c.same_events());
        assert_eq!(c.matched, c.len_a);
        assert!(c.timing.is_none());

        let different = compare(&rewrite(4, 200), &rewrite(5, 200), 0);
        assert!(!different.same_events());
        let d = different.first_divergence.unwrap();
        assert_eq!(d.seq as usize, different.matched);
        assert!(d.a.is_some() && d.b.is_some());
    }

    #[test]
    fn a_shorter_trace_diverges_where_it_ends() {
        let long = rewrite(4, 20);
        let mut short = long.clone();
        short.records.truncate(10);
        let c = compare(&long, &short, 0);
        assert_eq!(c.matched, 10);
        let d = c.first_divergence.unwrap();
        assert!(d.a.is_some() && d.b.is_none());
    }

    #[test]
    fn an_observation_is_compared_on_what_can_be_seen() {
        let ours = rewrite(4, 30);
        let seen = as_observation(&ours, 100);
        seen.validate().unwrap();
        assert!(seen.is_about_the_original());
        // Stop indices are absent from the observation and are not held against it.
        let c = compare(&seen, &ours, 0);
        assert!(c.same_events());
        // The rewrite ran synthetic mathematics, so this is not parity.
        assert_eq!(c.kind, ComparisonKind::StructureOnly);
        assert_eq!(
            compare(&seen, &seen, 0).kind,
            ComparisonKind::BetweenObservations
        );

        let mut recovered = ours.clone();
        recovered.header.math_provenance = Some(MathProvenance::Cabinet);
        recovered.header.source_class = SourceClass::Cabinet;
        assert_eq!(compare(&seen, &recovered, 0).kind, ComparisonKind::Parity);
    }

    #[test]
    fn timing_is_compared_within_a_stated_tolerance() {
        let ours = rewrite(4, 5);
        let a = as_observation(&ours, 100);
        let mut b = a.clone();
        for r in &mut b.records {
            r.t_ms = r.t_ms.map(|t| t + 20);
        }
        if let Some(r) = b.records.last_mut() {
            r.t_ms = r.t_ms.map(|t| t + 200);
        }
        let c = compare(&a, &b, 33);
        let t = c.timing.unwrap();
        assert_eq!(t.compared, a.records.len());
        assert_eq!(t.max_abs_diff_ms, 220);
        assert_eq!(t.beyond_tolerance, 1);
        assert_eq!(t.tolerance_ms, 33);
        assert!(t.mean_abs_diff_ms > 20.0);
    }

    #[test]
    fn headers_that_would_mislead_are_refused() {
        let good = rewrite(1, 2);

        // A rewrite that hides its model.
        let mut t = good.clone();
        t.header.math_id = None;
        assert!(matches!(t.validate(), Err(TraceError::RewriteWithoutModel)));

        // Synthetic mathematics presented as a cabinet source.
        let mut t = good.clone();
        t.header.source_class = SourceClass::Cabinet;
        assert!(matches!(
            t.validate(),
            Err(TraceError::RewriteSourceMismatch)
        ));

        // An "observation" with no recording behind it, or from a third party's page.
        let mut t = as_observation(&good, 10);
        t.header.capture_sha256 = None;
        assert!(matches!(
            t.validate(),
            Err(TraceError::ObservationWithoutCapture)
        ));
        let mut t = as_observation(&good, 10);
        t.header.source_class = SourceClass::ThirdParty;
        assert!(matches!(
            t.validate(),
            Err(TraceError::ObservationSource(_))
        ));

        // An observation that claims to know the original's model.
        let mut t = as_observation(&good, 10);
        t.header.math_id = Some("guess".into());
        assert!(matches!(
            t.validate(),
            Err(TraceError::ObservationWithModel)
        ));
    }

    #[test]
    fn malformed_traces_are_errors_with_a_line_number() {
        assert!(matches!(Trace::from_jsonl(""), Err(TraceError::Empty)));
        assert!(matches!(
            Trace::from_jsonl("{not json"),
            Err(TraceError::Json { line: 1, .. })
        ));
        let mut text = rewrite(1, 1).to_jsonl().unwrap();
        text.push_str("{\"seq\": 99}\n");
        assert!(matches!(
            Trace::from_jsonl(&text),
            Err(TraceError::Json { .. })
        ));

        let mut t = rewrite(1, 2);
        t.records[1].seq = 7;
        assert!(matches!(
            t.validate(),
            Err(TraceError::Sequence { index: 1, seq: 7 })
        ));
        let mut t = as_observation(&rewrite(1, 2), 10);
        t.records[2].t_ms = Some(0);
        assert!(matches!(t.validate(), Err(TraceError::TimeGoesBack { .. })));
        let mut t = rewrite(1, 1);
        t.header.schema = "dragonlink-trace/0".into();
        assert!(matches!(t.validate(), Err(TraceError::Schema(_))));
    }
}
