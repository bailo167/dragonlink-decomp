//! dragonlink-trace — record, validate and compare behavioural traces.
//!
//! ```text
//! dragonlink-trace record   --seed N --spins K [--out FILE]   # a rewrite trace of the synthetic fixture
//! dragonlink-trace validate <FILE>
//! dragonlink-trace compare  <A> <B> [--timing-tolerance-ms N] [--json]
//! dragonlink-trace replay   <TRACE> [--model synthetic|CONFIG] [--credits N] [--json]
//! dragonlink-trace transcribe <SHEET> --denom 1c --lines 50 --capture-sha256 HASH
//!                             --scope cabinet-au [--title T] [--state base] [--out FILE] [--json]
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
//! `transcribe` turns a window sheet (one line per game read from a
//! recording: the window, the denomination, the line count and the meters)
//! into an observation trace of one configuration, and reports what it left
//! out and whether the meters agree with each other. Without `--out` it only
//! reports, which is how to see which configurations a sheet holds.
//!
//! Observation traces are transcribed from recordings of the original, which
//! are source material; keep them in the private source root.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dragonlink_math::GameConfiguration;
use dragonlink_model::fixtures::synthetic_dev_fixture;
use dragonlink_model::{MathModel, SourceClass, TitleId};
use dragonlink_trace::{
    ComparisonKind, ObservationSource, ReplayReport, Selection, SheetSummary, Trace, compare,
    parse_sheet, record_rewrite, replay, sheet_to_trace,
};

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
    /// Turn a sheet of transcribed windows into an observation trace of one
    /// configuration, and report what was left out.
    Transcribe {
        /// The window sheet (JSON Lines; `dragonlink_trace::transcript`).
        sheet: PathBuf,
        /// The denomination to take, as displayed: 1c, 10c, $1.
        #[arg(long)]
        denom: String,
        /// The line count to take.
        #[arg(long)]
        lines: u32,
        /// The kind of game to take.
        #[arg(long, default_value = "base")]
        state: String,
        /// SHA-256 of the recording the sheet was read from.
        #[arg(long)]
        capture_sha256: String,
        /// What was recorded: cabinet-au, cabinet-us, mobile, with anything narrower.
        #[arg(long)]
        scope: String,
        /// The title, when known: golden_century, happy_and_prosperous, panda_magic, autumn_moon.
        #[arg(long, value_parser = parse_title)]
        title: Option<TitleId>,
        /// A note for the trace's header (who read it, from what).
        #[arg(long, default_value = "")]
        notes: String,
        /// Where to write the trace. A transcription of a recording is kept
        /// with the recording, in the private source root.
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

fn parse_title(text: &str) -> Result<TitleId, String> {
    serde_json::from_value(serde_json::Value::String(text.to_string()))
        .map_err(|_| format!("{text:?} is not a title this project knows"))
}

/// What a conversion did, one finding per line.
fn sheet_text(summary: &SheetSummary) -> String {
    let mut out = vec![format!("{} records on the sheet:", summary.records)];
    for (configuration, count) in &summary.configurations {
        out.push(format!("  {count:>5}  {configuration}"));
    }
    out.push(format!(
        "selected {}: {} games written, {} not a reel window",
        summary.selected, summary.games, summary.without_window
    ));
    out.push(format!(
        "reel windows: {} written, {} left out for an unread position",
        summary.reels_written, summary.reels_unread
    ));
    out.push(format!(
        "awards: {} written, {} unread",
        summary.awards_written, summary.awards_unread
    ));
    out.push(format!(
        "prize values: {} written, {} unread, {} not a whole multiple of the bet",
        summary.prize_values_written,
        summary.prize_values_unread,
        summary.prize_values_not_bet_multiples
    ));
    let m = &summary.meters;
    out.push(format!(
        "meters over {} consecutive pairs: {} agree, {} agree with the win not yet credited, {} UNEXPLAINED",
        m.pairs, m.settled, m.win_pending, m.unexplained
    ));
    out.join("\n")
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
        Cmd::Transcribe {
            sheet,
            denom,
            lines,
            state,
            capture_sha256,
            scope,
            title,
            notes,
            out,
            json,
        } => {
            let text = fs::read_to_string(&sheet)
                .with_context(|| format!("reading {}", sheet.display()))?;
            let records =
                parse_sheet(&text).with_context(|| format!("sheet {}", sheet.display()))?;
            let source = ObservationSource {
                title,
                scope,
                source_class: SourceClass::GameplayRecording,
                capture_sha256,
                notes,
            };
            let selection = Selection {
                state,
                denom,
                lines,
            };
            let (trace, summary) = sheet_to_trace(&records, &source, &selection)
                .with_context(|| format!("sheet {}", sheet.display()))?;
            trace
                .validate()
                .context("the trace that would be written")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!("{}", sheet_text(&summary));
            }
            if let Some(path) = out {
                fs::write(&path, trace.to_jsonl()?)
                    .with_context(|| format!("writing {}", path.display()))?;
                eprintln!("wrote {} events to {}", trace.records.len(), path.display());
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
    fn a_sheet_is_reported_with_what_it_holds_and_what_was_left_out() {
        // A sheet made up for the test.
        let line = |t: u32, credit: &str, win: &str, grid: &str| {
            format!(
                r#"{{"t":{t},"state":"base","denom":"2c","lines":25,"credit":"{credit}","bet":"1.00","win":"{win}","grid":{grid}}}"#
            )
        };
        let text = [
            line(1, "50.00", "0.00", r#"[["A","B","C"],["B","C","D"]]"#),
            line(2, "49.40", "0.40", r#"[["A","?","C"],["ORB:150","C","D"]]"#),
            line(3, "40.00", "0.00", r#"[["A","B","C"],["B","C","D"]]"#),
        ]
        .join("\n");
        let records = parse_sheet(&text).unwrap();
        let source = ObservationSource {
            title: Some(parse_title("golden_century").unwrap()),
            scope: "synthetic".into(),
            source_class: SourceClass::GameplayRecording,
            capture_sha256: "0".repeat(64),
            notes: String::new(),
        };
        let selection = Selection {
            state: "base".into(),
            denom: "2c".into(),
            lines: 25,
        };
        let (trace, summary) = sheet_to_trace(&records, &source, &selection).unwrap();
        trace.validate().unwrap();
        let report = sheet_text(&summary);
        assert!(report.contains("3 records on the sheet"));
        assert!(report.contains("base, 2c, 25 lines"));
        assert!(report.contains("reel windows: 5 written, 1 left out"));
        assert!(report.contains("prize values: 1 written"));
        assert!(report.contains("1 agree, 0 agree with the win not yet credited, 1 UNEXPLAINED"));
        assert!(parse_title("lightning_cash").is_err());
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
