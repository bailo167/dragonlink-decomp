//! dragonlink-trace — record, validate and compare behavioural traces.
//!
//! ```text
//! dragonlink-trace record   --seed N --spins K [--out FILE]   # a rewrite trace of the synthetic fixture
//! dragonlink-trace validate <FILE>
//! dragonlink-trace compare  <A> <B> [--timing-tolerance-ms N] [--json]
//! dragonlink-trace replay   <TRACE> [--model synthetic|CONFIG] [--credits N] [--json]
//! ```
//!
//! `compare` says what kind of comparison it made. Two rewrite traces give a
//! determinism check, never parity. An observation against a rewrite that ran
//! synthetic mathematics compares structure only. See `docs/PARITY.md`.
//!
//! `replay` drives the simulation with the outcomes a trace shows (which
//! windows the reels stopped on, which values the prize symbols carried) and
//! reports what the simulation then did differently: with the luck taken out,
//! what is left is a difference in rules. It prints what the replay amounts
//! to, and writes nothing.
//!
//! Observation traces are transcribed from recordings of the original, which
//! are source material; keep them in the private source root.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dragonlink_math::GameConfiguration;
use dragonlink_model::MathModel;
use dragonlink_model::fixtures::synthetic_dev_fixture;
use dragonlink_trace::{ComparisonKind, ReplayReport, Trace, compare, record_rewrite, replay};

#[derive(Parser, Debug)]
#[command(about = "Record, validate and compare behavioural traces")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the synthetic fixture and write its events as a rewrite trace.
    Record {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 100)]
        spins: u64,
        #[arg(long, default_value_t = 1_000_000)]
        credits: u64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Check that a trace file is well formed and honestly labelled.
    Validate { file: PathBuf },
    /// Compare two traces event by event.
    Compare {
        a: PathBuf,
        b: PathBuf,
        /// Difference in event time that counts as a mismatch. Take it from the
        /// recording's frame interval.
        #[arg(long, default_value_t = 0)]
        timing_tolerance_ms: u64,
        #[arg(long)]
        json: bool,
    },
    /// Drive the simulation with the outcomes a trace shows and report what
    /// it did differently.
    Replay {
        /// The trace whose outcomes are replayed: usually an observation.
        trace: PathBuf,
        /// The mathematics to replay through: `synthetic`, or a configuration
        /// file (`dragonlink-analyze export-fixture` writes a template).
        #[arg(long, default_value = "synthetic")]
        model: String,
        /// Credits the replayed game starts with. Default: read from the
        /// trace's first credit movement.
        #[arg(long)]
        credits: Option<u64>,
        #[arg(long)]
        json: bool,
    },
}

/// The mathematics named by `spec`: the synthetic fixture or a configuration file.
fn load_model(spec: &str) -> Result<MathModel> {
    if spec == "synthetic" {
        return Ok(synthetic_dev_fixture());
    }
    let text = fs::read_to_string(spec).with_context(|| format!("reading configuration {spec}"))?;
    let config =
        GameConfiguration::from_json(&text).with_context(|| format!("configuration {spec}"))?;
    Ok(config.model)
}

fn event_json(event: Option<&dragonlink_core::Event>) -> String {
    event.map_or("(ends)".into(), |e| {
        serde_json::to_string(e).unwrap_or_default()
    })
}

/// The report as text, one finding per line.
fn replay_text(report: &ReplayReport) -> String {
    let mut out = vec![
        describe(report.comparison.kind).to_string(),
        format!(
            "model {} ({:?}); starting credits {}{}",
            report.model_id,
            report.model_provenance,
            report.starting_credits,
            if report.starting_credits_known {
                ""
            } else {
                " (not observed: an arbitrary amount)"
            }
        ),
        format!(
            "{} commands scripted from the trace; {} of {} scripted draws consumed",
            report.commands, report.draws_consumed, report.draws_reconstructed
        ),
        format!(
            "{} hidden choices the trace leaves open (lowest candidate scripted)",
            report.ambiguities.len()
        ),
    ];
    for found in &report.inconsistencies {
        out.push(format!(
            "inconsistent with the model at event {}: {}",
            found.seq, found.reason
        ));
    }
    if let Some(seq) = report.unscripted_from {
        out.push(format!(
            "nothing from event {seq} on was replayed: no outcome is invented"
        ));
    }
    if let Some(stop) = &report.stopped {
        out.push(format!(
            "the simulation could not follow the trace at event {}: {:?}",
            stop.seq, stop.reason
        ));
    }
    let comparison = &report.comparison;
    out.push(format!(
        "events: {} observed, {} replayed; {} leading events match",
        comparison.len_a, comparison.len_b, comparison.matched
    ));
    match &comparison.first_divergence {
        None => out.push("the simulation emitted the events of the trace".to_string()),
        Some(d) => {
            out.push(format!("first difference at event {}:", d.seq));
            out.push(format!("  trace:  {}", event_json(d.a.as_ref())));
            out.push(format!("  replay: {}", event_json(d.b.as_ref())));
        }
    }
    out.join("\n")
}

fn load(path: &Path) -> Result<Trace> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Trace::from_jsonl(&text).with_context(|| format!("trace {}", path.display()))
}

fn describe(kind: ComparisonKind) -> &'static str {
    match kind {
        ComparisonKind::Determinism => {
            "DETERMINISM CHECK: both traces come from our simulation. This is not parity."
        }
        ComparisonKind::Parity => {
            "PARITY: an observation of the original against a rewrite run on recovered mathematics."
        }
        ComparisonKind::StructureOnly => {
            "STRUCTURE ONLY: the rewrite did not run recovered mathematics, so outcomes are not \
             expected to match; only the order and kinds of events are meaningful."
        }
        ComparisonKind::BetweenObservations => {
            "BETWEEN OBSERVATIONS: agreement between two transcriptions of the original."
        }
        ComparisonKind::RulesUnderModel => {
            "RULES UNDER A MODEL: an observation's outcomes replayed through mathematics that              is not evidence of the cabinet game's. This checks our rules under that model; it              is not parity with the cabinet game."
        }
        ComparisonKind::MachineryOnly => {
            "MACHINERY ONLY: outcomes replayed through the synthetic fixture. This tests the              replay tooling and says nothing about any original game. It is not parity."
        }
    }
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::Record {
            seed,
            spins,
            credits,
            out,
        } => {
            let trace = record_rewrite(&synthetic_dev_fixture(), None, seed, spins, credits)
                .context("running the simulation")?;
            let text = trace.to_jsonl()?;
            match out {
                Some(path) => {
                    fs::write(&path, text)
                        .with_context(|| format!("writing {}", path.display()))?;
                    eprintln!("wrote {} events to {}", trace.records.len(), path.display());
                }
                None => print!("{text}"),
            }
            Ok(())
        }
        Cmd::Validate { file } => {
            let trace = load(&file)?;
            println!(
                "valid: {:?} trace, {} events, source {:?}",
                trace.header.origin,
                trace.records.len(),
                trace.header.source_class
            );
            Ok(())
        }
        Cmd::Compare {
            a,
            b,
            timing_tolerance_ms,
            json,
        } => {
            let comparison = compare(&load(&a)?, &load(&b)?, timing_tolerance_ms);
            if json {
                println!("{}", serde_json::to_string_pretty(&comparison)?);
            } else {
                println!("{}", describe(comparison.kind));
                println!(
                    "events: {} and {}; {} leading events match",
                    comparison.len_a, comparison.len_b, comparison.matched
                );
                match &comparison.first_divergence {
                    None => println!("the two traces contain the same events in the same order"),
                    Some(d) => {
                        println!("first difference at event {}:", d.seq);
                        println!("  A: {}", event_json(d.a.as_ref()));
                        println!("  B: {}", event_json(d.b.as_ref()));
                    }
                }
                if let Some(t) = &comparison.timing {
                    println!(
                        "timing over {} events: mean difference {:.1} ms, largest {} ms, {} beyond the {} ms tolerance",
                        t.compared,
                        t.mean_abs_diff_ms,
                        t.max_abs_diff_ms,
                        t.beyond_tolerance,
                        t.tolerance_ms
                    );
                }
            }
            if !comparison.same_events() {
                std::process::exit(1);
            }
            Ok(())
        }
        Cmd::Replay {
            trace,
            model,
            credits,
            json,
        } => {
            let report = replay(&load_model(&model)?, &load(&trace)?, credits)
                .context("the model cannot be run")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", replay_text(&report));
            }
            if !report.agrees() {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_comparison_says_what_it_is() {
        assert!(describe(ComparisonKind::Determinism).contains("not parity"));
        assert!(describe(ComparisonKind::StructureOnly).contains("not"));
        assert!(describe(ComparisonKind::Parity).starts_with("PARITY"));
        assert!(describe(ComparisonKind::BetweenObservations).contains("transcriptions"));
        assert!(describe(ComparisonKind::RulesUnderModel).contains("not parity"));
        assert!(describe(ComparisonKind::MachineryOnly).contains("not parity"));
    }

    #[test]
    fn a_replay_is_reported_with_what_it_amounts_to_and_where_it_differs() {
        let model = load_model("synthetic").unwrap();
        let mut trace = record_rewrite(&model, None, 4, 200, 1_000_000).unwrap();
        // Relabelled as an observation, as the crate's own tests do.
        trace.header.origin = dragonlink_trace::TraceOrigin::Observation;
        let report = replay(&model, &trace, None).unwrap();
        assert!(report.agrees());
        let text = replay_text(&report);
        assert!(text.starts_with("MACHINERY ONLY"));
        assert!(text.contains("the simulation emitted the events of the trace"));
        assert!(!text.contains("inconsistent"));

        // Through other rules the same outcomes give other events.
        let mut other = model.clone();
        for pay in &mut other.line_pays {
            pay.pay += 1;
        }
        let report = replay(&other, &trace, None).unwrap();
        assert!(!report.agrees());
        let text = replay_text(&report);
        assert!(text.contains("first difference at event"));
        assert!(text.contains("  replay: {"));

        // A configuration file is another way to name the mathematics.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = GameConfiguration::synthetic_fixture();
        fs::write(&path, config.to_json().unwrap()).unwrap();
        assert_eq!(load_model(path.to_str().unwrap()).unwrap(), config.model);
        assert!(load_model("no-such-file.json").is_err());
    }

    #[test]
    fn a_recorded_trace_loads_back_and_a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.jsonl");
        let trace = record_rewrite(&synthetic_dev_fixture(), None, 3, 5, 1000).unwrap();
        fs::write(&path, trace.to_jsonl().unwrap()).unwrap();
        assert_eq!(load(&path).unwrap(), trace);
        assert!(load(&dir.path().join("absent.jsonl")).is_err());
    }
}
