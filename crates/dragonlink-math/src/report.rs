//! The per-game technical report, as Markdown.
//!
//! One report per game, version and platform. The layout follows
//! `docs/reverse-engineering/math/REPORT_TEMPLATE.md`. A section whose
//! content is not known says so; nothing is filled in to look complete.

use std::fmt::Write as _;

use dragonlink_model::Confidence;

use crate::compare::{Comparison, PayRelation};
use crate::config::{ConstantProvenance, GameConfiguration};
use crate::exact::Analysis;
use crate::progressive::ProgressiveEv;
use crate::sim::SimReport;

fn pct(x: f64) -> String {
    format!("{:.4}%", x * 100.0)
}

fn one_in(p: f64) -> String {
    if p > 0.0 {
        format!("1 in {:.1}", 1.0 / p)
    } else {
        "never".to_string()
    }
}

fn banner(config: &GameConfiguration) -> String {
    if config.is_synthetic() {
        "> **SYNTHETIC CONFIGURATION.** Every number below is a property of a fixture invented by \
         this project. None of it describes any Dragon Link game."
            .to_string()
    } else {
        format!(
            "> **{}.** Results are no better supported than the weakest constant: {}.",
            config.model.provenance.banner(),
            config.weakest_provenance().label()
        )
    }
}

/// The technical report for `config`, from its exact `analysis` and, when
/// given, a simulation run.
pub fn game_report(
    config: &GameConfiguration,
    analysis: &Analysis,
    sim: Option<&SimReport>,
) -> String {
    let mut s = String::new();
    let id = &config.identity;
    let unknown = || "unknown".to_string();
    let model = &config.model;
    let _ = writeln!(s, "# Game mathematics report: {}\n", id.name);
    let _ = writeln!(s, "{}\n", banner(config));

    let _ = writeln!(s, "## Configuration identity\n");
    let _ = writeln!(s, "| | |\n|---|---|");
    let _ = writeln!(
        s,
        "| Title | {} |",
        id.title
            .map_or_else(unknown, |t| t.display_name().to_string())
    );
    let _ = writeln!(
        s,
        "| Market | {} |",
        id.market.clone().unwrap_or_else(unknown)
    );
    let _ = writeln!(
        s,
        "| Platform | {} |",
        id.platform.clone().unwrap_or_else(unknown)
    );
    let _ = writeln!(
        s,
        "| Program version | {} |",
        id.program_version.clone().unwrap_or_else(unknown)
    );
    let _ = writeln!(
        s,
        "| Specification | {} |",
        id.specification.clone().unwrap_or_else(unknown)
    );
    let _ = writeln!(
        s,
        "| Denomination | {} |",
        id.denomination.clone().unwrap_or_else(unknown)
    );
    let _ = writeln!(s, "| Model id | `{}` |", model.id);
    if !id.notes.is_empty() {
        let _ = writeln!(s, "| Notes | {} |", id.notes);
    }

    let _ = writeln!(s, "\n## Evidence confidence\n");
    let _ = writeln!(
        s,
        "Model provenance: **{:?}**. Constant groups by how they were obtained:\n",
        model.provenance
    );
    let tally = config.provenance_tally();
    for class in ConstantProvenance::ALL {
        if let Some(n) = tally.get(&class) {
            let _ = writeln!(s, "- {}: {n}", class.label());
        }
    }

    let _ = writeln!(s, "\n## Reel model\n");
    let _ = writeln!(
        s,
        "{} reels × {} rows, {} equally likely stop combinations.\n",
        model.grid.reels, model.grid.rows, analysis.stop_combinations
    );
    let _ = writeln!(s, "| Reel | Stops | Symbol counts |\n|---|---:|---|");
    for (reel, strip) in model.reel_strips.iter().enumerate() {
        let mut counts = std::collections::BTreeMap::new();
        for sym in strip {
            *counts.entry(sym.0).or_insert(0u32) += 1;
        }
        let text = counts
            .iter()
            .map(|(sym, n)| format!("{}×{n}", model.symbols[*sym as usize].key))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(s, "| {} | {} | {text} |", reel + 1, strip.len());
    }

    let _ = writeln!(s, "\n## Pay table\n");
    let _ = writeln!(
        s,
        "{} paylines. Line pays per credit bet on the line:\n",
        model.paylines.len()
    );
    let _ = writeln!(
        s,
        "| Outcome | Pay | Probability per line | Share of return |\n|---|---:|---:|---:|"
    );
    for o in &analysis.line_outcomes {
        let _ = writeln!(
            s,
            "| {} × {} | {} | {} | {} |",
            o.symbol,
            o.count,
            o.pay,
            pct(o.probability),
            pct(o.rtp)
        );
    }

    let _ = writeln!(s, "\n## RTP\n");
    let _ = writeln!(
        s,
        "| Component | Return | Share of total |\n|---|---:|---:|"
    );
    let share = |x: f64| {
        if analysis.total_rtp > 0.0 {
            pct(x / analysis.total_rtp)
        } else {
            pct(0.0)
        }
    };
    let _ = writeln!(
        s,
        "| Line pays | {} (exactly {}/{}) | {} |",
        pct(analysis.line_rtp),
        analysis.line_rtp_exact.0,
        analysis.line_rtp_exact.1,
        share(analysis.line_rtp)
    );
    let _ = writeln!(
        s,
        "| Scatter pays | {} | {} |",
        pct(analysis.scatter_rtp),
        share(analysis.scatter_rtp)
    );
    if let Some(f) = &analysis.feature {
        let _ = writeln!(
            s,
            "| Hold & Spin, credit orbs | {} | {} |",
            pct(f.rtp_credit_orbs),
            share(f.rtp_credit_orbs)
        );
        for j in &f.jackpots {
            let _ = writeln!(
                s,
                "| Hold & Spin, {:?} | {} | {} |",
                j.tier,
                pct(j.rtp),
                share(j.rtp)
            );
        }
    }
    let _ = writeln!(s, "| **Total** | **{}** | 100% |", pct(analysis.total_rtp));

    let _ = writeln!(s, "\n## Hit frequency\n");
    let _ = writeln!(
        s,
        "A given line pays with probability {}.",
        pct(analysis.line_hit_probability)
    );
    match &analysis.enumeration {
        Some(e) => {
            let _ = writeln!(
                s,
                "A whole spin's award, feature included, has standard deviation {:.4} × bet (exact).\n",
                e.total_variance.sqrt()
            );
            let _ = writeln!(
                s,
                "By enumeration of all {} stop combinations: a spin pays on a line or scatter with \
                 probability {} ({}); it pays or starts the feature with probability {} ({}). \
                 Base-game award: variance {:.4}, standard deviation {:.4}, largest {:.2} (× bet).",
                e.combinations,
                pct(e.base_hit_frequency),
                one_in(e.base_hit_frequency),
                pct(e.any_hit_frequency),
                one_in(e.any_hit_frequency),
                e.base_variance,
                e.base_variance.sqrt(),
                e.base_max
            );
        }
        None => {
            let _ = writeln!(
                s,
                "Whole-spin hit frequency and variance were not enumerated ({} stop combinations \
                 exceed the limit); see the simulation section.",
                analysis.stop_combinations
            );
        }
    }

    let _ = writeln!(s, "\n## Feature frequency and value\n");
    match &analysis.feature {
        None => {
            let _ = writeln!(s, "The model has no Hold & Spin feature.");
        }
        Some(f) => {
            let _ = writeln!(
                s,
                "Hold & Spin starts with probability {} ({}). Given that it starts: {:.3} orbs held \
                 at the end on average, {:.3} respins played, expected award {:.3} × bet. Mean \
                 value of one orb: {:.3} × bet.\n",
                pct(f.trigger_probability),
                one_in(f.trigger_probability),
                f.expected_final_orbs,
                f.expected_respins,
                f.value_given_trigger,
                f.mean_orb_value
            );
            let _ = writeln!(s, "| Orbs at start | Probability per spin |\n|---:|---:|");
            for (k, p) in &f.start_distribution {
                let _ = writeln!(s, "| {k} | {:.6e} |", p);
            }
            let _ = writeln!(
                s,
                "\n| Orbs at end | Probability given the feature |\n|---:|---:|"
            );
            for (k, p) in &f.final_distribution {
                let _ = writeln!(s, "| {k} | {:.6e} |", p);
            }
            let _ = writeln!(s, "\n## Jackpot model\n");
            let _ = writeln!(
                s,
                "| Level | Value in model (× bet) | Awards per spin | Frequency | Share of return |\n|---|---:|---:|---|---:|"
            );
            for j in &f.jackpots {
                let _ = writeln!(
                    s,
                    "| {:?} | {} | {:.6e} | {} | {} |",
                    j.tier,
                    j.total_bet_multiple,
                    j.awards_per_spin,
                    one_in(j.awards_per_spin),
                    pct(j.rtp)
                );
            }
            let _ = writeln!(
                s,
                "\nProgressive contribution: not modelled. The model holds fixed values; meter \
                 increments and resets are not represented."
            );
        }
    }

    let _ = writeln!(s, "\n## Overall EV\n");
    let _ = writeln!(
        s,
        "Return {} of the amount bet; expected result {:+.4} × bet per spin. Return excluding \
         jackpot levels: {}.",
        pct(analysis.total_rtp),
        analysis.total_rtp - 1.0,
        pct(analysis.rtp_excluding_jackpots())
    );

    let _ = writeln!(s, "\n## Denomination and bet effects\n");
    let _ = writeln!(
        s,
        "Bet options (credits per line): {:?}. In this model every pay and jackpot value is \
         proportional to the bet, so the bet changes monetary scale only. A configuration is one \
         denomination; compare configurations to see denomination effects.",
        model.bet.credits_per_line_options
    );

    let _ = writeln!(
        s,
        "\n## State dependence, RNG architecture, outcome selection\n"
    );
    let _ = writeln!(s, "| Question | Finding | Confidence |\n|---|---|---|");
    for (name, finding) in config.behaviour.entries() {
        let text = finding.value.clone().unwrap_or_else(unknown);
        let label = if finding.confidence == Confidence::Unknown && finding.value.is_none() {
            "UNKNOWN".to_string()
        } else {
            format!("{} ({})", finding.confidence.label(), finding.citation)
        };
        let _ = writeln!(s, "| {name} | {text} | {label} |");
    }
    let _ = writeln!(
        s,
        "\nThe model analysed here has no state between spins by construction. Whether the \
         implementation it describes has any is one of the questions above."
    );

    let _ = writeln!(s, "\n## Known version differences\n");
    let _ = writeln!(
        s,
        "None recorded in this configuration. Use `compare-games` against another version."
    );

    let _ = writeln!(s, "\n## Unknowns\n");
    let mut unknowns: Vec<String> = config
        .behaviour
        .entries()
        .iter()
        .filter(|(_, f)| f.value.is_none())
        .map(|(name, _)| (*name).to_string())
        .collect();
    unknowns.push("Free games (not modelled)".into());
    unknowns
        .push("Progressive meter increments, resets and contribution rates (not modelled)".into());
    for u in unknowns {
        let _ = writeln!(s, "- {u}");
    }

    let _ = writeln!(s, "\n## Evidence and provenance\n");
    let _ = writeln!(
        s,
        "| Constant | Provenance | Source | Note |\n|---|---|---|---|"
    );
    for r in &config.constants {
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} |",
            r.path,
            r.provenance.label(),
            r.source,
            r.note
        );
    }

    let _ = writeln!(s, "\n## Validation results\n");
    match sim {
        None => {
            let _ = writeln!(s, "No simulation was run for this report.");
        }
        Some(r) => {
            let within = |ok: bool| if ok { "inside" } else { "**OUTSIDE**" };
            let _ = writeln!(
                s,
                "Seeded simulation of our own state machine ({} spins, seed {}, generator `{}`). \
                 This checks the analysis against the implementation; it is not a comparison with \
                 any original game.\n",
                r.spins, r.seed, r.rng
            );
            let _ = writeln!(
                s,
                "| Quantity | Exact | Simulated (95% interval) | Exact is |\n|---|---:|---|---|"
            );
            let _ = writeln!(
                s,
                "| Total return | {} | {} ({} to {}) | {} |",
                pct(analysis.total_rtp),
                pct(r.rtp.estimate),
                pct(r.rtp.low),
                pct(r.rtp.high),
                within(r.rtp.contains(analysis.total_rtp))
            );
            if let Some(f) = &analysis.feature {
                let _ = writeln!(
                    s,
                    "| Feature frequency | {} | {} ({} to {}) | {} |",
                    pct(f.trigger_probability),
                    pct(r.feature_frequency.estimate),
                    pct(r.feature_frequency.low),
                    pct(r.feature_frequency.high),
                    within(r.feature_frequency.contains(f.trigger_probability))
                );
            }
            if let Some(e) = &analysis.enumeration {
                let _ = writeln!(
                    s,
                    "| Hit frequency | {} | {} ({} to {}) | {} |",
                    pct(e.any_hit_frequency),
                    pct(r.hit_frequency.estimate),
                    pct(r.hit_frequency.low),
                    pct(r.hit_frequency.high),
                    within(r.hit_frequency.contains(e.any_hit_frequency))
                );
            }
            let _ = writeln!(
                s,
                "\nSimulated standard deviation of one spin's award, feature included: {:.3} × bet \
                 (largest award seen {:.1} × bet). The return's interval assumes a normal mean; rare \
                 large awards make it optimistic for short runs.",
                r.std_dev, r.max_award
            );
            let _ = writeln!(
                s,
                "\nValidation against captured gameplay of the original: none (no captures held)."
            );
        }
    }
    s
}

/// The comparison of two configurations, as Markdown.
pub fn comparison_report(c: &Comparison) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Comparison: {} ↔ {}\n", c.name_a, c.name_b);
    let verdict = match (c.differences.is_empty(), c.same_probabilities, c.line_pays) {
        (true, _, _) => "The two configurations are identical.".to_string(),
        (false, true, PayRelation::Scaled { numerator, denominator }) => format!(
            "Reels, lines and feature parameters are the same. Line pays in B are those of A × \
             {numerator}/{denominator}: a difference of prize scale."
        ),
        (false, true, _) => {
            "Reels, lines and feature parameters are the same; the differences are in pays, bet \
             options or identity."
                .to_string()
        }
        (false, false, _) => {
            "The configurations differ in reels, lines or feature parameters: more than prize scaling."
                .to_string()
        }
    };
    let _ = writeln!(s, "{verdict}\n");
    if !c.differences.is_empty() {
        let _ = writeln!(s, "## Differences\n\n| Constant | A | B |\n|---|---|---|");
        for d in &c.differences {
            let _ = writeln!(s, "| `{}` | {} | {} |", d.path, d.a, d.b);
        }
    }
    let _ = writeln!(
        s,
        "\n## Quantities\n\n| Quantity | A | B | B − A |\n|---|---:|---:|---:|"
    );
    for m in &c.metrics {
        let _ = writeln!(
            s,
            "| {} | {:.6} | {:.6} | {:+.6} |",
            m.name,
            m.a,
            m.b,
            m.b - m.a
        );
    }
    s
}

/// The progressive expected-value table, as Markdown.
pub fn progressive_report(config: &GameConfiguration, ev: &ProgressiveEv) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Expected value by meter: {}\n", config.identity.name);
    let _ = writeln!(s, "{}\n", banner(config));
    let _ = writeln!(
        s,
        "Bet: {} credits per line, {} credits in total. Return from fixed pays: {}.\n",
        ev.credits_per_line,
        ev.total_bet,
        pct(ev.rtp_excluding_jackpots)
    );
    let _ = writeln!(
        s,
        "| Level | Meter (credits) | From | Frequency | Share of return | Break-even meter (credits) |\n|---|---:|---|---|---:|---:|"
    );
    for t in &ev.tiers {
        let _ = writeln!(
            s,
            "| {:?} | {:.2} | {} | {} | {} | {} |",
            t.tier,
            t.meter_credits,
            if t.supplied { "supplied" } else { "model" },
            one_in(t.awards_per_spin),
            pct(t.rtp),
            t.break_even_credits
                .map_or("cannot be won".to_string(), |v| format!("{v:.2}"))
        );
    }
    let _ = writeln!(
        s,
        "\n**Return at these meters: {}.** Expected result per spin: {:+.4} credits.\n",
        pct(ev.rtp),
        ev.expected_credits_per_spin
    );
    let _ = writeln!(
        s,
        "Break-even is the meter value at which the return is 100% with the other levels unchanged. \
         It assumes award probabilities do not depend on the meters and describes the long-run \
         average only: a level won once in very many spins returns its share to whoever wins it."
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::compare;
    use crate::exact::analyze;
    use crate::progressive::progressive_ev;
    use crate::sim::simulate;

    #[test]
    fn a_report_has_every_section_and_says_it_is_synthetic() {
        let config = GameConfiguration::synthetic_fixture();
        let analysis = analyze(&config.model, 0).unwrap();
        let sim = simulate(&config.model, 3, 20_000).unwrap();
        let report = game_report(&config, &analysis, Some(&sim));
        assert!(
            report
                .lines()
                .nth(2)
                .unwrap()
                .contains("SYNTHETIC CONFIGURATION")
        );
        for heading in [
            "## Configuration identity",
            "## Evidence confidence",
            "## Reel model",
            "## Pay table",
            "## RTP",
            "## Hit frequency",
            "## Feature frequency and value",
            "## Jackpot model",
            "## Overall EV",
            "## Denomination and bet effects",
            "## State dependence, RNG architecture, outcome selection",
            "## Known version differences",
            "## Unknowns",
            "## Evidence and provenance",
            "## Validation results",
        ] {
            assert!(report.contains(heading), "missing {heading}");
        }
        // Unknown behaviour is printed as unknown, not left out.
        assert!(report.contains("| RNG architecture | unknown | UNKNOWN |"));
        assert!(report.contains("synthetic (invented by this project)"));
    }

    #[test]
    fn the_same_inputs_give_the_same_report() {
        let config = GameConfiguration::synthetic_fixture();
        let analysis = analyze(&config.model, 0).unwrap();
        assert_eq!(
            game_report(&config, &analysis, None),
            game_report(&config, &analysis, None)
        );
    }

    #[test]
    fn comparison_and_progressive_reports_render() {
        let a = GameConfiguration::synthetic_fixture();
        let mut b = GameConfiguration::synthetic_fixture();
        for p in &mut b.model.line_pays {
            p.pay *= 2;
        }
        let text = comparison_report(&compare(&a, &b, 0).unwrap());
        assert!(text.contains("difference of prize scale"));
        assert!(text.contains("`line_pays`"));
        let same = comparison_report(&compare(&a, &a, 0).unwrap());
        assert!(same.contains("identical"));

        let analysis = analyze(&a.model, 0).unwrap();
        let ev = progressive_ev(&a.model, &analysis, 0, &[]).unwrap();
        let text = progressive_report(&a, &ev);
        assert!(text.contains("SYNTHETIC CONFIGURATION"));
        assert!(text.contains("Break-even"));
    }
}
