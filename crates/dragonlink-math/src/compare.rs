//! Structural and quantitative comparison of two configurations.
//!
//! Answers "what differs between these two versions, denominations or
//! implementations", and in particular whether a difference is only prize
//! scaling or reaches the reels and probabilities.

use dragonlink_model::MathModel;
use serde::Serialize;

use crate::config::GameConfiguration;
use crate::exact::{Analysis, AnalysisError, analyze};

/// One thing that differs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Difference {
    pub path: String,
    pub a: String,
    pub b: String,
}

/// One quantity side by side.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metric {
    pub name: &'static str,
    pub a: f64,
    pub b: f64,
}

/// How the two sets of line pays relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayRelation {
    Identical,
    /// Every pay in `b` is the pay in `a` times `numerator / denominator`.
    Scaled {
        numerator: u32,
        denominator: u32,
    },
    /// The pays differ by more than one common factor.
    Different,
}

/// The result of comparing configuration `a` with configuration `b`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Comparison {
    pub name_a: String,
    pub name_b: String,
    pub differences: Vec<Difference>,
    pub line_pays: PayRelation,
    /// True when reels, lines, feature parameters and value tables are the
    /// same, so that the two differ at most in pays and bet options.
    pub same_probabilities: bool,
    pub metrics: Vec<Metric>,
}

fn pay_relation(a: &MathModel, b: &MathModel) -> PayRelation {
    let key = |m: &MathModel, p: &dragonlink_model::LinePay| {
        (
            m.symbol(p.symbol)
                .map(|s| s.key.clone())
                .unwrap_or_default(),
            p.count,
        )
    };
    let mut pa: Vec<_> = a.line_pays.iter().map(|p| (key(a, p), p.pay)).collect();
    let mut pb: Vec<_> = b.line_pays.iter().map(|p| (key(b, p), p.pay)).collect();
    pa.sort();
    pb.sort();
    if pa == pb {
        return PayRelation::Identical;
    }
    if pa.len() != pb.len() || pa.iter().zip(&pb).any(|(x, y)| x.0 != y.0) {
        return PayRelation::Different;
    }
    let Some((first_a, first_b)) = pa
        .iter()
        .zip(&pb)
        .map(|(x, y)| (x.1, y.1))
        .find(|(x, _)| *x != 0)
    else {
        return PayRelation::Different;
    };
    let scaled = pa
        .iter()
        .zip(&pb)
        .all(|(x, y)| u64::from(y.1) * u64::from(first_a) == u64::from(x.1) * u64::from(first_b));
    if !scaled || first_b == 0 {
        return PayRelation::Different;
    }
    let gcd = {
        let (mut x, mut y) = (first_a, first_b);
        while y != 0 {
            (x, y) = (y, x % y);
        }
        x
    };
    PayRelation::Scaled {
        numerator: first_b / gcd,
        denominator: first_a / gcd,
    }
}

fn structural(a: &GameConfiguration, b: &GameConfiguration) -> Vec<Difference> {
    let mut out = Vec::new();
    let mut diff = |path: &str, x: String, y: String| {
        if x != y {
            out.push(Difference {
                path: path.to_string(),
                a: x,
                b: y,
            });
        }
    };
    let opt = |o: &Option<String>| o.clone().unwrap_or_else(|| "unknown".into());
    diff(
        "identity.title",
        format!("{:?}", a.identity.title),
        format!("{:?}", b.identity.title),
    );
    diff(
        "identity.market",
        opt(&a.identity.market),
        opt(&b.identity.market),
    );
    diff(
        "identity.platform",
        opt(&a.identity.platform),
        opt(&b.identity.platform),
    );
    diff(
        "identity.program_version",
        opt(&a.identity.program_version),
        opt(&b.identity.program_version),
    );
    diff(
        "identity.specification",
        opt(&a.identity.specification),
        opt(&b.identity.specification),
    );
    diff(
        "identity.denomination",
        opt(&a.identity.denomination),
        opt(&b.identity.denomination),
    );

    let (ma, mb) = (&a.model, &b.model);
    diff(
        "model.provenance",
        format!("{:?}", ma.provenance),
        format!("{:?}", mb.provenance),
    );
    diff(
        "grid",
        format!("{}x{}", ma.grid.reels, ma.grid.rows),
        format!("{}x{}", mb.grid.reels, mb.grid.rows),
    );
    let symbols = |m: &MathModel| {
        m.symbols
            .iter()
            .map(|s| format!("{}:{:?}", s.key, s.kind))
            .collect::<Vec<_>>()
            .join(" ")
    };
    diff("symbols", symbols(ma), symbols(mb));
    for reel in 0..ma.reel_strips.len().max(mb.reel_strips.len()) {
        let describe = |m: &MathModel| match m.reel_strips.get(reel) {
            None => "absent".to_string(),
            Some(strip) => {
                let mut counts = std::collections::BTreeMap::new();
                for id in strip {
                    let key = m.symbol(*id).map_or("?", |s| s.key.as_str());
                    *counts.entry(key).or_insert(0u32) += 1;
                }
                let counts = counts
                    .iter()
                    .map(|(k, n)| format!("{k}×{n}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} stops ({counts})", strip.len())
            }
        };
        let order = |m: &MathModel| {
            m.reel_strips.get(reel).map(|strip| {
                strip
                    .iter()
                    .map(|id| m.symbol(*id).map_or("?", |s| s.key.as_str()).to_string())
                    .collect::<Vec<String>>()
            })
        };
        let (da, db) = (describe(ma), describe(mb));
        if da != db {
            diff(&format!("reel_strips[{reel}]"), da, db);
        } else if order(ma) != order(mb) {
            diff(
                &format!("reel_strips[{reel}]"),
                format!("{da}, order A"),
                format!("{db}, same symbol counts in a different order"),
            );
        }
    }
    diff(
        "paylines",
        format!("{:?}", ma.paylines),
        format!("{:?}", mb.paylines),
    );
    let pays = |m: &MathModel| {
        let mut v: Vec<String> = m
            .line_pays
            .iter()
            .map(|p| {
                format!(
                    "{}×{}={}",
                    m.symbol(p.symbol).map_or("?", |s| s.key.as_str()),
                    p.count,
                    p.pay
                )
            })
            .collect();
        v.sort();
        v.join(" ")
    };
    diff("line_pays", pays(ma), pays(mb));
    let scatters = |m: &MathModel| {
        let mut v: Vec<String> = m
            .scatter_pays
            .iter()
            .map(|p| {
                format!(
                    "{}×{}={}×bet",
                    m.symbol(p.symbol).map_or("?", |s| s.key.as_str()),
                    p.count,
                    p.total_bet_multiple
                )
            })
            .collect();
        v.sort();
        v.join(" ")
    };
    diff("scatter_pays", scatters(ma), scatters(mb));
    diff(
        "bet",
        format!("{:?}", ma.bet.credits_per_line_options),
        format!("{:?}", mb.bet.credits_per_line_options),
    );
    match (&ma.hold_and_spin, &mb.hold_and_spin) {
        (None, None) => {}
        (x, y) if x.is_none() || y.is_none() => diff(
            "hold_and_spin",
            if x.is_some() { "present" } else { "absent" }.into(),
            if y.is_some() { "present" } else { "absent" }.into(),
        ),
        (Some(x), Some(y)) => {
            diff(
                "hold_and_spin.trigger_count",
                x.trigger_count.to_string(),
                y.trigger_count.to_string(),
            );
            diff(
                "hold_and_spin.initial_respins",
                x.initial_respins.to_string(),
                y.initial_respins.to_string(),
            );
            diff(
                "hold_and_spin.reset_respins_on_new_orb",
                x.reset_respins_on_new_orb.to_string(),
                y.reset_respins_on_new_orb.to_string(),
            );
            diff(
                "hold_and_spin.orb_land_chance",
                format!("{}/{}", x.orb_land_chance.num, x.orb_land_chance.den),
                format!("{}/{}", y.orb_land_chance.num, y.orb_land_chance.den),
            );
            diff(
                "hold_and_spin.orb_values",
                format!("{:?}", x.orb_values),
                format!("{:?}", y.orb_values),
            );
            diff(
                "hold_and_spin.full_grid_award",
                format!("{:?}", x.full_grid_award),
                format!("{:?}", y.full_grid_award),
            );
            diff(
                "hold_and_spin.jackpots",
                format!("{:?}", x.jackpots),
                format!("{:?}", y.jackpots),
            );
        }
        _ => {}
    }
    out
}

fn metrics(a: &Analysis, b: &Analysis) -> Vec<Metric> {
    let feature = |x: &Analysis, f: fn(&crate::exact::FeatureAnalysis) -> f64| {
        x.feature.as_ref().map_or(0.0, f)
    };
    let mut out = vec![
        Metric {
            name: "total RTP",
            a: a.total_rtp,
            b: b.total_rtp,
        },
        Metric {
            name: "line RTP",
            a: a.line_rtp,
            b: b.line_rtp,
        },
        Metric {
            name: "scatter RTP",
            a: a.scatter_rtp,
            b: b.scatter_rtp,
        },
        Metric {
            name: "feature RTP",
            a: feature(a, |f| f.rtp),
            b: feature(b, |f| f.rtp),
        },
        Metric {
            name: "line hit probability (per line)",
            a: a.line_hit_probability,
            b: b.line_hit_probability,
        },
        Metric {
            name: "feature trigger probability",
            a: feature(a, |f| f.trigger_probability),
            b: feature(b, |f| f.trigger_probability),
        },
        Metric {
            name: "feature value given trigger (× bet)",
            a: feature(a, |f| f.value_given_trigger),
            b: feature(b, |f| f.value_given_trigger),
        },
        Metric {
            name: "full grid probability per spin",
            a: feature(a, |f| f.full_grid_per_spin),
            b: feature(b, |f| f.full_grid_per_spin),
        },
    ];
    if let (Some(x), Some(y)) = (&a.enumeration, &b.enumeration) {
        out.push(Metric {
            name: "hit frequency",
            a: x.any_hit_frequency,
            b: y.any_hit_frequency,
        });
        out.push(Metric {
            name: "base-game variance",
            a: x.base_variance,
            b: y.base_variance,
        });
    }
    out
}

/// Compare two configurations. `enumeration_limit` is as for [`analyze`].
pub fn compare(
    a: &GameConfiguration,
    b: &GameConfiguration,
    enumeration_limit: u128,
) -> Result<Comparison, AnalysisError> {
    let analysis_a = analyze(&a.model, enumeration_limit)?;
    let analysis_b = analyze(&b.model, enumeration_limit)?;
    let differences = structural(a, b);
    let probability_paths = [
        "grid",
        "symbols",
        "reel_strips",
        "paylines",
        "hold_and_spin",
    ];
    let same_probabilities = !differences
        .iter()
        .any(|d| probability_paths.iter().any(|p| d.path.starts_with(p)));
    Ok(Comparison {
        name_a: a.identity.name.clone(),
        name_b: b.identity.name.clone(),
        differences,
        line_pays: pay_relation(&a.model, &b.model),
        same_probabilities,
        metrics: metrics(&analysis_a, &analysis_b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GameConfiguration {
        GameConfiguration::synthetic_fixture()
    }

    #[test]
    fn a_configuration_is_identical_to_itself() {
        let c = compare(&fixture(), &fixture(), 0).unwrap();
        assert!(c.differences.is_empty());
        assert_eq!(c.line_pays, PayRelation::Identical);
        assert!(c.same_probabilities);
        assert!(c.metrics.iter().all(|m| m.a == m.b));
    }

    #[test]
    fn scaled_pays_are_recognised_as_prize_scaling_only() {
        let a = fixture();
        let mut b = fixture();
        for p in &mut b.model.line_pays {
            p.pay *= 3;
        }
        let c = compare(&a, &b, 0).unwrap();
        assert_eq!(
            c.line_pays,
            PayRelation::Scaled {
                numerator: 3,
                denominator: 1
            }
        );
        assert!(c.same_probabilities);
        assert_eq!(c.differences.len(), 1);
        assert_eq!(c.differences[0].path, "line_pays");
        let line = c.metrics.iter().find(|m| m.name == "line RTP").unwrap();
        assert!((line.b - 3.0 * line.a).abs() < 1e-12);
        let trigger = c
            .metrics
            .iter()
            .find(|m| m.name == "feature trigger probability")
            .unwrap();
        assert_eq!(trigger.a, trigger.b);
    }

    #[test]
    fn a_change_to_one_pay_is_not_scaling() {
        let a = fixture();
        let mut b = fixture();
        b.model.line_pays[0].pay += 1;
        assert_eq!(
            compare(&a, &b, 0).unwrap().line_pays,
            PayRelation::Different
        );
    }

    #[test]
    fn a_reel_change_is_reported_as_a_change_in_probabilities() {
        let a = fixture();
        // Same symbols on reel 0, rotated: no quantity changes, but it is reported.
        let mut rotated = fixture();
        rotated.model.reel_strips[0].rotate_left(1);
        let c = compare(&a, &rotated, 0).unwrap();
        assert!(!c.same_probabilities);
        assert!(c.differences[0].b.contains("different order"));
        let total = c.metrics.iter().find(|m| m.name == "total RTP").unwrap();
        assert!((total.a - total.b).abs() < 1e-12);

        // One stop replaced: the symbol counts and the return change.
        let mut changed = fixture();
        let wild = changed
            .model
            .symbols
            .iter()
            .find(|s| s.key == "WILD")
            .unwrap()
            .id;
        changed.model.reel_strips[1][0] = wild;
        let c = compare(&a, &changed, 0).unwrap();
        assert!(!c.same_probabilities);
        assert!(c.differences.iter().any(|d| d.path == "reel_strips[1]"));
        let line = c.metrics.iter().find(|m| m.name == "line RTP").unwrap();
        assert!(line.a != line.b);
    }

    #[test]
    fn feature_and_identity_changes_are_listed_by_path() {
        let a = fixture();
        let mut b = fixture();
        b.identity.program_version = Some("2".into());
        if let Some(hs) = &mut b.model.hold_and_spin {
            hs.trigger_count = 5;
        }
        let c = compare(&a, &b, 0).unwrap();
        let paths: Vec<&str> = c.differences.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(
            paths,
            ["identity.program_version", "hold_and_spin.trigger_count"]
        );
        assert!(!c.same_probabilities);
    }
}
