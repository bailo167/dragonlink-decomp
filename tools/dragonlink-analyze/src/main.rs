//! dragonlink-analyze — quantitative analysis of a game configuration.
//!
//! ```text
//! dragonlink-analyze export-fixture [--out FILE]
//! dragonlink-analyze analyze-game  <CONFIG> [--simulate SPINS] [--seed N] [--json] [--out FILE]
//! dragonlink-analyze compare-games <A> <B> [--json]
//! dragonlink-analyze progressive-ev <CONFIG> [--bet-index N] [--meter grand=250000]... [--json]
//! dragonlink-analyze simulate      <CONFIG> --spins N [--seed N]
//! dragonlink-analyze reconstruct-reels <TRACE> [--json]
//! ```
//!
//! `<CONFIG>` is a configuration file (`docs/reverse-engineering/math/CONFIGURATION.md`)
//! or the word `synthetic` for the built-in synthetic fixture.
//!
//! Results describe the configuration given. The built-in fixture is invented
//! by this project and describes no original game.
//!
//! Recovered configurations are proprietary material and live in the private
//! source root. This tool refuses to write output about one into the
//! repository (`docs/DECISIONS.md`, D-010).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dragonlink_core::DevRng;
use dragonlink_math::GameConfiguration;
use dragonlink_math::compare::compare;
use dragonlink_math::exact::{DEFAULT_ENUMERATION_LIMIT, analyze};
use dragonlink_math::overlap::window_overlap;
use dragonlink_math::progressive::progressive_ev;
use dragonlink_math::reconstruct::{RepeatBasis, Strip, reconstruct_from_events, windows_by_reel};
use dragonlink_math::report::{comparison_report, game_report, progressive_report};
use dragonlink_math::sim::simulate;
use dragonlink_model::JackpotTier;

#[derive(Parser, Debug)]
#[command(about = "Analyse, compare and price game configurations")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Write the synthetic fixture as a configuration file (a template for the format).
    ExportFixture {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// The technical report for one configuration.
    AnalyzeGame {
        config: String,
        /// Also run a seeded simulation of this many spins as a cross-check.
        #[arg(long)]
        simulate: Option<u64>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Enumerate every stop combination when there are at most this many.
        #[arg(long, default_value_t = DEFAULT_ENUMERATION_LIMIT)]
        enumeration_limit: u128,
        /// Print the analysis as JSON instead of the Markdown report.
        #[arg(long)]
        json: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// What differs between two configurations.
    CompareGames {
        a: String,
        b: String,
        #[arg(long, default_value_t = DEFAULT_ENUMERATION_LIMIT)]
        enumeration_limit: u128,
        #[arg(long)]
        json: bool,
    },
    /// Expected value at given jackpot meter values.
    ProgressiveEv {
        config: String,
        /// Bet option (index into the configuration's credits-per-line options).
        #[arg(long, default_value_t = 0)]
        bet_index: usize,
        /// A displayed meter value in credits, e.g. `--meter grand=250000`. Repeatable.
        #[arg(long = "meter", value_parser = parse_meter)]
        meters: Vec<(JackpotTier, f64)>,
        #[arg(long)]
        json: bool,
    },
    /// Rebuild each reel from the windows a trace shows it stopping on.
    ReconstructReels {
        /// A trace file (`dragonlink-trace/1`), rewrite or observation.
        trace: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Whether two traces show the same reels: for each reel, the share of
    /// the second trace's stops whose window was also seen in the first, with
    /// a permutation test of that share.
    CompareWindows {
        /// Two trace files (`dragonlink-trace/1`), each of one configuration.
        a: PathBuf,
        b: PathBuf,
        /// Random relabellings per reel. The smallest p-value that can be
        /// reported is one over this number plus one.
        #[arg(long, default_value_t = 4000)]
        shuffles: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        json: bool,
    },
    /// A seeded Monte Carlo run with confidence intervals.
    Simulate {
        config: String,
        #[arg(long)]
        spins: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

fn parse_meter(text: &str) -> Result<(JackpotTier, f64), String> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| "expected LEVEL=CREDITS, e.g. grand=250000".to_string())?;
    let tier = match name.to_ascii_lowercase().as_str() {
        "mini" => JackpotTier::Mini,
        "minor" => JackpotTier::Minor,
        "major" => JackpotTier::Major,
        "grand" => JackpotTier::Grand,
        other => {
            return Err(format!(
                "unknown level {other:?} (mini, minor, major, grand)"
            ));
        }
    };
    let value: f64 = value
        .parse()
        .map_err(|_| format!("{value:?} is not a number of credits"))?;
    Ok((tier, value))
}

fn load(spec: &str) -> Result<GameConfiguration> {
    if spec == "synthetic" {
        return Ok(GameConfiguration::synthetic_fixture());
    }
    if spec == "golden-century" {
        return Ok(GameConfiguration::golden_century_synthetic());
    }
    if spec == "autumn-moon" {
        return Ok(GameConfiguration::autumn_moon_synthetic());
    }
    if spec == "happy-prosperous" {
        return Ok(GameConfiguration::happy_prosperous_synthetic());
    }
    if spec == "panda-magic" {
        return Ok(GameConfiguration::panda_magic_synthetic());
    }
    let text = fs::read_to_string(spec).with_context(|| format!("reading configuration {spec}"))?;
    GameConfiguration::from_json(&text).with_context(|| format!("configuration {spec}"))
}

/// The repository this tool was built from, when it still exists.
fn repository_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()
}

/// Refuse to write output about a non-synthetic configuration into the repository.
fn check_destination(config: &GameConfiguration, out: &Path) -> Result<()> {
    if config.is_synthetic() {
        return Ok(());
    }
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .with_context(|| format!("output folder {}", parent.display()))?;
    if let Some(root) = repository_root()
        && parent.starts_with(&root)
    {
        bail!(
            "refusing to write output about a non-synthetic configuration into the repository; \
             write it next to the configuration in the private source root (docs/DECISIONS.md, D-010)"
        );
    }
    Ok(())
}

fn emit(text: &str, out: Option<&Path>) -> Result<()> {
    match out {
        Some(path) => {
            fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {}", path.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::ExportFixture { out } => {
            let config = GameConfiguration::synthetic_fixture();
            emit(&(config.to_json()? + "\n"), out.as_deref())
        }
        Cmd::AnalyzeGame {
            config,
            simulate: spins,
            seed,
            enumeration_limit,
            json,
            out,
        } => {
            let config = load(&config)?;
            if let Some(path) = &out {
                check_destination(&config, path)?;
            }
            let analysis = analyze(&config.model, enumeration_limit)?;
            let sim = spins
                .map(|n| simulate(&config.model, seed, n))
                .transpose()
                .context("simulating")?;
            let text = if json {
                serde_json::to_string_pretty(&serde_json::json!({
                    "configuration": config.identity,
                    "synthetic": config.is_synthetic(),
                    "model_provenance": config.model.provenance,
                    "analysis": analysis,
                    "simulation": sim,
                }))? + "\n"
            } else {
                game_report(&config, &analysis, sim.as_ref())
            };
            emit(&text, out.as_deref())
        }
        Cmd::CompareGames {
            a,
            b,
            enumeration_limit,
            json,
        } => {
            let comparison = compare(&load(&a)?, &load(&b)?, enumeration_limit)?;
            let text = if json {
                serde_json::to_string_pretty(&comparison)? + "\n"
            } else {
                comparison_report(&comparison)
            };
            emit(&text, None)
        }
        Cmd::ProgressiveEv {
            config,
            bet_index,
            meters,
            json,
        } => {
            let config = load(&config)?;
            let analysis = analyze(&config.model, 0)?;
            let ev = progressive_ev(&config.model, &analysis, bet_index, &meters)?;
            let text = if json {
                serde_json::to_string_pretty(&ev)? + "\n"
            } else {
                progressive_report(&config, &ev)
            };
            emit(&text, None)
        }
        Cmd::ReconstructReels { trace, json } => {
            let text = fs::read_to_string(&trace)
                .with_context(|| format!("reading trace {}", trace.display()))?;
            let trace = dragonlink_trace::Trace::from_jsonl(&text).context("the trace")?;
            let reels = reconstruct_from_events(trace.records.iter().map(|r| &r.event));
            if json {
                return emit(&(serde_json::to_string_pretty(&reels)? + "\n"), None);
            }
            let mut out = String::new();
            out.push_str(&format!(
                "Reels reconstructed from a {:?} trace (source {:?}). They describe what was observed, nothing else.\n\n",
                trace.header.origin, trace.header.source_class
            ));
            for (reel, r) in reels.iter().enumerate() {
                out.push_str(&format!(
                    "reel {}: {} stops observed, {} distinct windows, about {:.2}% of spins would show a window not seen yet\n",
                    reel + 1,
                    r.observations,
                    r.windows.len(),
                    r.unseen_share * 100.0
                ));
                if let Some(u) = &r.uniformity {
                    out.push_str(&format!(
                        "  equal shares: {} (chi-square {:.1} on {} degrees of freedom, z = {:+.2})\n",
                        if u.consistent { "consistent" } else { "REJECTED" },
                        u.chi_square,
                        u.degrees_of_freedom,
                        u.z
                    ));
                }
                if let Some(stops) = r.stops {
                    let chain = r
                        .repeats
                        .iter()
                        .filter(|x| x.basis == RepeatBasis::Chain)
                        .count();
                    out.push_str(&format!(
                        "  shortest strip showing exactly these windows: {stops} stops; {chain} windows sit at more than one stop by the chain (derived), {} stacks sized from counts (estimated)\n",
                        r.repeats.len() - chain
                    ));
                    if let Some(f) = &r.fit {
                        out.push_str(&format!(
                            "  seen in proportion to stops: {} (chi-square {:.1} on {} degrees of freedom, z = {:+.2})\n",
                            if f.consistent { "consistent" } else { "REJECTED" },
                            f.chi_square,
                            f.degrees_of_freedom,
                            f.z
                        ));
                    }
                }
                match &r.strip {
                    Strip::Unique { symbols } => {
                        out.push_str(&format!("  strip ({} stops, unique): {}\n", symbols.len(), symbols.join(" ")));
                    }
                    Strip::Ambiguous { candidates, example } => out.push_str(&format!(
                        "  strip ({} stops): {} orders have exactly these windows and are mathematically identical; one of them: {}\n",
                        example.len(),
                        candidates,
                        example.join(" ")
                    )),
                    Strip::NotAssembled { reason } => {
                        out.push_str(&format!("  strip not assembled: {reason}\n"));
                    }
                }
            }
            emit(&out, None)
        }
        Cmd::CompareWindows {
            a,
            b,
            shuffles,
            seed,
            json,
        } => {
            let load_trace = |path: &Path| -> Result<dragonlink_trace::Trace> {
                let text = fs::read_to_string(path)
                    .with_context(|| format!("reading trace {}", path.display()))?;
                dragonlink_trace::Trace::from_jsonl(&text)
                    .with_context(|| format!("trace {}", path.display()))
            };
            let (ta, tb) = (load_trace(&a)?, load_trace(&b)?);
            let wa = windows_by_reel(ta.records.iter().map(|r| &r.event));
            let wb = windows_by_reel(tb.records.iter().map(|r| &r.event));
            let mut rng = DevRng::new(seed);
            let mut reels = Vec::new();
            for (reel, windows) in &wa {
                let other = wb.get(reel).map(Vec::as_slice).unwrap_or(&[]);
                if let Some(found) =
                    window_overlap(windows, other, shuffles, &mut rng).context("shuffling")?
                {
                    reels.push((*reel, found));
                }
            }
            if json {
                return emit(&(serde_json::to_string_pretty(&reels)? + "\n"), None);
            }
            let scope = |t: &dragonlink_trace::Trace| {
                t.header
                    .scope
                    .clone()
                    .unwrap_or_else(|| "no scope stated".to_string())
            };
            let mut out = format!(
                "A: {}\nB: {}\nShare of B's stops whose window was also seen in A, with the proportion of {shuffles} random relabellings (seed {seed}) giving a share as low. A small p says the two differ by more than chance arranges; a large one says only that this sample shows no difference.\n\n",
                scope(&ta),
                scope(&tb)
            );
            for (reel, f) in &reels {
                out.push_str(&format!(
                    "reel {}: {:5.1}% of {} stops (A has {} stops; windows: {} in A, {} in B, {} in both)   p = {:.4}\n",
                    reel + 1,
                    f.share * 100.0,
                    f.b_observations,
                    f.a_observations,
                    f.a_windows,
                    f.b_windows,
                    f.shared_windows,
                    f.p_value
                ));
            }
            if reels.is_empty() {
                out.push_str("no reel has stops in both traces\n");
            }
            emit(&out, None)
        }
        Cmd::Simulate {
            config,
            spins,
            seed,
        } => {
            let config = load(&config)?;
            let report = simulate(&config.model, seed, spins).context("simulating")?;
            emit(&(serde_json::to_string_pretty(&report)? + "\n"), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meters_parse_by_level_name() {
        assert_eq!(
            parse_meter("grand=250000"),
            Ok((JackpotTier::Grand, 250_000.0))
        );
        assert_eq!(parse_meter("Major=12.5"), Ok((JackpotTier::Major, 12.5)));
        assert!(parse_meter("grand").is_err());
        assert!(parse_meter("mega=1").is_err());
        assert!(parse_meter("mini=lots").is_err());
    }

    #[test]
    fn the_built_in_fixture_loads_by_name() {
        assert!(load("synthetic").unwrap().is_synthetic());
        assert!(load("no-such-file.json").is_err());
    }

    #[test]
    fn output_about_a_recovered_configuration_may_not_go_into_the_repository() {
        let synthetic = GameConfiguration::synthetic_fixture();
        let inside = repository_root().unwrap().join("report.md");
        assert!(check_destination(&synthetic, &inside).is_ok());

        let mut recovered = GameConfiguration::synthetic_fixture();
        recovered.model.provenance = dragonlink_model::MathProvenance::Cabinet;
        assert!(check_destination(&recovered, &inside).is_err());
        let outside = std::env::temp_dir().join("report.md");
        assert!(check_destination(&recovered, &outside).is_ok());
    }
}
