//! The synthetic Golden Century configuration runs and has a sane shape.
//!
//! This checks only that the invented numbers produce a *playable* game: it
//! validates, pays on lines, reaches both the scatter and the Hold & Spin
//! feature, and returns somewhere between a black hole and free money. It
//! makes no claim about Dragon Link — every figure here is a property of a
//! synthetic fixture, not of any original game.

use dragonlink_math::sim::simulate;
use dragonlink_model::fixtures::golden_century_synthetic;

#[test]
fn the_synthetic_golden_century_is_playable() {
    let model = golden_century_synthetic();
    model.validate().expect("the synthetic model is runnable");
    assert_eq!(model.paylines.len(), 50);
    assert_eq!(model.bet.line_options, vec![5, 25, 50]);
    assert_eq!(model.symbols.len(), 12);

    let report = simulate(&model, 1, 400_000).expect("simulate");
    eprintln!(
        "synthetic GC: RTP {:.1}% (line {:.1}, scatter {:.1}, feature {:.1}); hit {:.1}%; feature about 1 in {:.0}",
        report.rtp.estimate * 100.0,
        report.line_rtp * 100.0,
        report.scatter_rtp * 100.0,
        report.feature_rtp * 100.0,
        report.hit_frequency.estimate * 100.0,
        1.0 / report.feature_frequency.estimate.max(1e-9),
    );

    // A playable return, not free money and not a black hole. The exact value
    // is synthetic and not a target.
    assert!(
        (0.5..1.5).contains(&report.rtp.estimate),
        "synthetic RTP {:.3} is outside a playable range",
        report.rtp.estimate
    );
    // Lines, scatters and the feature are all reached.
    assert!(report.line_rtp > 0.0, "lines never paid");
    assert!(report.scatter_rtp > 0.0, "scatters never paid");
    assert!(
        report.feature_frequency.estimate > 0.0,
        "Hold & Spin never triggered"
    );
    // The feature is neither so common it dominates nor so rare it never shows.
    assert!(
        (1.0 / 1000.0..1.0 / 40.0).contains(&report.feature_frequency.estimate),
        "feature frequency {:.5} is outside a sensible range",
        report.feature_frequency.estimate
    );
    assert!(report.hit_frequency.estimate > 0.05 && report.hit_frequency.estimate < 0.95);
}
