//! dragonlink-trace — record, validate and compare behavioural traces.
//!
//! ```text
//! dragonlink-trace record   --seed N --spins K [--out FILE]   # a rewrite trace of the synthetic fixture
//! dragonlink-trace validate <FILE>
//! dragonlink-trace compare  <A> <B> [--timing-tolerance-ms N] [--json]
//! ```
//!
//! `compare` says what kind of comparison it made. Two rewrite traces give a
//! determinism check, never parity. An observation against a rewrite that ran
//! synthetic mathematics compares structure only. See `docs/PARITY.md`.
//!
//! Observation traces are transcribed from recordings of the original, which
//! are source material; keep them in the private source root.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dragonlink_model::fixtures::synthetic_dev_fixture;
use dragonlink_trace::{ComparisonKind, Trace, compare, record_rewrite};

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
                        println!(
                            "  A: {}",
                            d.a.as_ref()
                                .map_or("(ends)".into(), |e| serde_json::to_string(e)
                                    .unwrap_or_default())
                        );
                        println!(
                            "  B: {}",
                            d.b.as_ref()
                                .map_or("(ends)".into(), |e| serde_json::to_string(e)
                                    .unwrap_or_default())
                        );
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
