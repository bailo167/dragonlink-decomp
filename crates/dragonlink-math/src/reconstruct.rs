//! Reconstructing a reel from the windows it was seen to stop on.
//!
//! This is the bottom rung of the ladder in `docs/DECISIONS.md` (D-013): when
//! a game's outcomes cannot be read from its software, its reels are
//! reconstructed from what many observed spins showed.
//!
//! # What can and cannot be recovered
//!
//! Each stopped reel shows a **window**: `rows` consecutive symbols of a
//! cyclic strip. Three things follow.
//!
//! 1. **The window distribution is what the mathematics depends on.** Every
//!    pay, scatter count and feature trigger is a function of the windows.
//!    Two strips with the same windows in the same proportions are the same
//!    reel as far as any return, hit rate or probability is concerned. So the
//!    target is the set of windows and their shares, and that is estimated
//!    directly from counts, with intervals.
//! 2. **The strip itself is assembled, not estimated**, when each window
//!    occurs at one stop: overlapping windows chain into the strip (each
//!    window's last `rows − 1` symbols are the next window's first). Where a
//!    run of `rows − 1` symbols occurs twice on the strip the chain branches,
//!    and more than one strip has exactly the observed windows. Those strips
//!    are mathematically identical (point 1); they differ only in what
//!    scrolls past, which a longer view of the reel would settle. The number
//!    of such strips is reported, never hidden.
//! 3. **Uniform stops are a hypothesis to test.** If every stop is equally
//!    likely, every window has the same share. The counts are tested against
//!    that. When the test fails, either stops are weighted or some window
//!    occurs at more than one stop; the shares still stand, and the strip is
//!    not assembled.
//!
//! Completeness is estimated, not assumed: the share of spins expected to
//! show a window not yet seen (Good–Turing) says whether more collection is
//! needed before a reconstruction is relied on.
//!
//! A reel reconstructed this way describes the implementation that was
//! observed. From the official mobile game it is a mobile reel, and its
//! constants are `inferred_statistically` with scope mobile.

use std::collections::BTreeMap;

use dragonlink_core::Event;
use serde::Serialize;

use crate::sim::{Interval, wilson};

/// Most candidate strips counted before giving up counting.
pub const CANDIDATE_CAP: usize = 1000;

/// One window and how often it was seen.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowStat {
    /// Symbols top to bottom.
    pub symbols: Vec<String>,
    pub count: u64,
    /// Share of this reel's stops showing the window, with a 95% interval.
    pub share: Interval,
}

/// The test of "every window is equally likely".
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Uniformity {
    pub chi_square: f64,
    pub degrees_of_freedom: usize,
    /// The statistic as a standard normal deviate (Wilson–Hilferty). Large
    /// positive values mean the counts are too uneven for equal shares.
    pub z: f64,
    /// False when the counts reject equal shares at about the 1% level.
    pub consistent: bool,
}

/// What could be said about the strip's order.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Strip {
    /// Exactly one cyclic strip has the observed windows. Given from an arbitrary start.
    Unique { symbols: Vec<String> },
    /// Several strips have exactly the observed windows; they are
    /// mathematically identical. `candidates` stops counting at [`CANDIDATE_CAP`].
    Ambiguous {
        candidates: usize,
        example: Vec<String>,
    },
    /// The windows do not chain into one cyclic strip.
    NotAssembled { reason: String },
}

/// One reel, reconstructed from observation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReelReconstruction {
    pub observations: u64,
    pub rows: usize,
    /// Distinct windows seen, most frequent first.
    pub windows: Vec<WindowStat>,
    /// Estimated share of spins that would show a window not seen yet.
    pub unseen_share: f64,
    pub uniformity: Option<Uniformity>,
    pub strip: Strip,
}

fn uniformity(counts: &[u64], total: u64) -> Option<Uniformity> {
    let k = counts.len();
    if k < 2 || total == 0 {
        return None;
    }
    let expected = total as f64 / k as f64;
    let chi_square: f64 = counts
        .iter()
        .map(|c| (*c as f64 - expected).powi(2) / expected)
        .sum();
    let df = (k - 1) as f64;
    // Wilson–Hilferty: (X/df)^(1/3) is close to normal.
    let z = ((chi_square / df).cbrt() - (1.0 - 2.0 / (9.0 * df))) / (2.0 / (9.0 * df)).sqrt();
    Some(Uniformity {
        chi_square,
        degrees_of_freedom: k - 1,
        z,
        consistent: z < 2.326,
    })
}

/// Chain windows that each occur at one stop into cyclic strips. Returns the
/// number of distinct strips (capped) and one of them.
fn assemble(windows: &[&Vec<String>], rows: usize) -> Result<(usize, Vec<String>), String> {
    let n = windows.len();
    if rows < 2 {
        return Err("a one-row window carries no order".into());
    }
    // Each window joins its first rows-1 symbols to its last rows-1 symbols.
    let mut leaving: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    let mut arriving: BTreeMap<&[String], usize> = BTreeMap::new();
    for (i, w) in windows.iter().enumerate() {
        leaving.entry(&w[..rows - 1]).or_default().push(i);
        *arriving.entry(&w[1..]).or_insert(0) += 1;
    }
    for (node, out) in &leaving {
        if arriving.get(node).copied().unwrap_or(0) != out.len() {
            return Err(format!(
                "the run {node:?} is entered and left a different number of times: windows are missing"
            ));
        }
    }
    if arriving.keys().any(|node| !leaving.contains_key(node)) {
        return Err("a run of symbols is entered but never left: windows are missing".into());
    }

    // Count the ways to walk every window exactly once, starting from window 0.
    let mut used = vec![false; n];
    let mut path = vec![0usize];
    used[0] = true;
    let mut count = 0usize;
    let mut example: Option<Vec<usize>> = None;
    // Iterative depth-first search: `choice[d]` is the next option to try at depth d.
    let mut choice = vec![0usize];
    while let Some(depth) = path.len().checked_sub(1) {
        let current = windows[path[depth]];
        let options = leaving.get(&current[1..]).map_or(&[][..], Vec::as_slice);
        if path.len() == n {
            // Complete: it is a cycle if the last window leads back to the first.
            if options.contains(&0) {
                count += 1;
                if example.is_none() {
                    example = Some(path.clone());
                }
                if count >= CANDIDATE_CAP {
                    break;
                }
            }
            used[path[depth]] = false;
            path.pop();
            choice.pop();
            continue;
        }
        let mut advanced = false;
        while choice[depth] < options.len() {
            let next = options[choice[depth]];
            choice[depth] += 1;
            if !used[next] {
                used[next] = true;
                path.push(next);
                choice.push(0);
                advanced = true;
                break;
            }
        }
        if !advanced {
            if depth == 0 {
                break;
            }
            used[path[depth]] = false;
            path.pop();
            choice.pop();
        }
    }
    match example {
        Some(order) => Ok((
            count,
            order.iter().map(|i| windows[*i][0].clone()).collect(),
        )),
        None => Err("the windows do not form a single cycle".into()),
    }
}

/// Reconstruct one reel from the windows it was observed to show.
pub fn reconstruct_reel(observed: &[Vec<String>]) -> ReelReconstruction {
    let rows = observed.first().map_or(0, Vec::len);
    let mut counts: BTreeMap<&Vec<String>, u64> = BTreeMap::new();
    for window in observed {
        *counts.entry(window).or_insert(0) += 1;
    }
    let total = observed.len() as u64;
    let mut windows: Vec<WindowStat> = counts
        .iter()
        .map(|(symbols, count)| WindowStat {
            symbols: (*symbols).clone(),
            count: *count,
            share: wilson(*count, total),
        })
        .collect();
    windows.sort_by(|a, b| b.count.cmp(&a.count).then(a.symbols.cmp(&b.symbols)));
    let seen_once = counts.values().filter(|c| **c == 1).count() as f64;
    let unseen_share = if total > 0 {
        seen_once / total as f64
    } else {
        1.0
    };
    let all_counts: Vec<u64> = counts.values().copied().collect();
    let uniformity = uniformity(&all_counts, total);

    let strip = if observed.iter().any(|w| w.len() != rows) {
        Strip::NotAssembled {
            reason: "the windows are not all the same height".into(),
        }
    } else if uniformity.is_some_and(|u| !u.consistent) {
        Strip::NotAssembled {
            reason: "the windows are not equally frequent: stops are weighted, or a window occurs \
                     at more than one stop"
                .into(),
        }
    } else {
        let distinct: Vec<&Vec<String>> = counts.keys().copied().collect();
        if distinct.is_empty() {
            Strip::NotAssembled {
                reason: "nothing was observed".into(),
            }
        } else {
            match assemble(&distinct, rows) {
                Ok((1, symbols)) => Strip::Unique { symbols },
                Ok((candidates, example)) => Strip::Ambiguous {
                    candidates,
                    example,
                },
                Err(reason) => Strip::NotAssembled { reason },
            }
        }
    };
    ReelReconstruction {
        observations: total,
        rows,
        windows,
        unseen_share,
        uniformity,
        strip,
    }
}

/// Reconstruct every reel from a run of events (`ReelStopped`).
pub fn reconstruct_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
) -> Vec<ReelReconstruction> {
    let mut per_reel: BTreeMap<u8, Vec<Vec<String>>> = BTreeMap::new();
    for event in events {
        if let Event::ReelStopped { reel, symbols, .. } = event {
            per_reel.entry(*reel).or_default().push(symbols.clone());
        }
    }
    per_reel.values().map(|w| reconstruct_reel(w)).collect()
}

/// The windows of a cyclic strip, one per stop.
pub fn windows_of(strip: &[String], rows: usize) -> Vec<Vec<String>> {
    (0..strip.len())
        .map(|stop| {
            (0..rows)
                .map(|row| strip[(stop + row) % strip.len()].clone())
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::analyze;
    use dragonlink_core::{DevRng, Game};
    use dragonlink_model::SymbolId;
    use dragonlink_model::fixtures::synthetic_dev_fixture;

    fn strip(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    fn sorted(mut windows: Vec<Vec<String>>) -> Vec<Vec<String>> {
        windows.sort();
        windows
    }

    /// Whether `a` is a rotation of `b`.
    fn same_cycle(a: &[String], b: &[String]) -> bool {
        a.len() == b.len()
            && (0..a.len()).any(|k| (0..a.len()).all(|i| a[(i + k) % a.len()] == b[i]))
    }

    #[test]
    fn a_strip_without_repeated_runs_is_recovered_exactly() {
        let truth = strip("A B C D E F G");
        let r = reconstruct_reel(&windows_of(&truth, 3));
        assert_eq!(r.observations, 7);
        assert_eq!(r.windows.len(), 7);
        let Strip::Unique { symbols } = &r.strip else {
            panic!("expected a unique strip, got {:?}", r.strip);
        };
        assert!(same_cycle(symbols, &truth));
        assert!(r.uniformity.unwrap().consistent);
    }

    #[test]
    fn a_run_repeated_twice_still_leaves_one_strip() {
        // "A B" occurs twice with different continuations. Chaining the other
        // way round splits into two separate loops, so only one strip fits.
        let truth = strip("A B C X A B D Y");
        let r = reconstruct_reel(&windows_of(&truth, 3));
        let Strip::Unique { symbols } = &r.strip else {
            panic!("expected a unique strip, got {:?}", r.strip);
        };
        assert!(same_cycle(symbols, &truth));
    }

    #[test]
    fn a_run_repeated_three_times_makes_the_order_ambiguous_but_not_the_windows() {
        // "A B" occurs three times: its continuations can be visited in two
        // orders, giving two different strips with exactly the same windows.
        let truth = strip("A B C A B D A B E");
        let other = strip("A B C A B E A B D");
        let observed = windows_of(&truth, 3);
        assert_eq!(sorted(windows_of(&other, 3)), sorted(observed.clone()));
        assert!(!same_cycle(&truth, &other));
        let r = reconstruct_reel(&observed);
        let Strip::Ambiguous {
            candidates,
            example,
        } = &r.strip
        else {
            panic!("expected ambiguity, got {:?}", r.strip);
        };
        assert_eq!(*candidates, 2);
        // Whichever order is offered, it has exactly the observed windows.
        assert_eq!(sorted(windows_of(example, 3)), sorted(observed));
        assert!(same_cycle(example, &truth) || same_cycle(example, &other));
    }

    #[test]
    fn every_strip_of_the_fixture_is_reconstructed_to_the_same_mathematics() {
        // Replace each reel of the fixture by its reconstruction from one pass
        // over its stops. Whatever order the reconstruction chose, the exact
        // analysis must be unchanged: the windows are what the game depends on.
        let model = synthetic_dev_fixture();
        let before = analyze(&model, 0).unwrap();
        let mut rebuilt = model.clone();
        for (reel, original) in model.reel_strips.iter().enumerate() {
            let keys: Vec<String> = original
                .iter()
                .map(|id| model.symbol(*id).unwrap().key.clone())
                .collect();
            let r = reconstruct_reel(&windows_of(&keys, 3));
            let symbols = match r.strip {
                Strip::Unique { symbols } => symbols,
                Strip::Ambiguous { example, .. } => example,
                Strip::NotAssembled { reason } => panic!("reel {reel}: {reason}"),
            };
            assert_eq!(
                sorted(windows_of(&symbols, 3)),
                sorted(windows_of(&keys, 3))
            );
            rebuilt.reel_strips[reel] = symbols
                .iter()
                .map(|key| model.symbols.iter().find(|s| &s.key == key).unwrap().id)
                .collect::<Vec<SymbolId>>();
        }
        let after = analyze(&rebuilt, 0).unwrap();
        assert!((before.total_rtp - after.total_rtp).abs() < 1e-12);
        assert!((before.line_rtp - after.line_rtp).abs() < 1e-12);
        assert_eq!(before.orb_distribution.len(), after.orb_distribution.len());
        for (a, b) in before.orb_distribution.iter().zip(&after.orb_distribution) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn reels_are_reconstructed_from_simulated_play() {
        // Our own state machine stands in for an observed game.
        let model = synthetic_dev_fixture();
        let mut game = Game::new(model.clone(), u64::MAX / 2).unwrap();
        let mut rng = DevRng::new(5);
        let mut events = Vec::new();
        for _ in 0..20_000 {
            events.extend(game.play_spin(&mut rng).unwrap());
        }
        let reels = reconstruct_from_events(&events);
        assert_eq!(reels.len(), 5);
        for (reel, r) in reels.iter().enumerate() {
            assert_eq!(r.observations, 20_000);
            // Every stop was seen, and nothing suggests unseen windows remain.
            assert_eq!(
                r.windows.len(),
                model.reel_strips[reel].len(),
                "reel {reel}"
            );
            assert_eq!(r.unseen_share, 0.0);
            assert!(
                r.uniformity.unwrap().consistent,
                "reel {reel}: {:?}",
                r.uniformity
            );
            // Each window's true share, 1/24, lies in its interval (allowing the
            // few misses expected among 120 intervals at 95%).
            let misses = r
                .windows
                .iter()
                .filter(|w| !w.share.contains(1.0 / 24.0))
                .count();
            assert!(misses <= 4, "reel {reel}: {misses} intervals miss");
            let keys: Vec<String> = model.reel_strips[reel]
                .iter()
                .map(|id| model.symbol(*id).unwrap().key.clone())
                .collect();
            let assembled = match &r.strip {
                Strip::Unique { symbols } => symbols,
                Strip::Ambiguous { example, .. } => example,
                Strip::NotAssembled { reason } => panic!("reel {reel}: {reason}"),
            };
            assert_eq!(
                sorted(windows_of(assembled, 3)),
                sorted(windows_of(&keys, 3))
            );
        }
    }

    #[test]
    fn too_few_spins_are_reported_as_incomplete_not_as_a_strip() {
        let truth = strip("A B C D E F G H I J K L");
        let observed: Vec<Vec<String>> = windows_of(&truth, 3).into_iter().take(5).collect();
        let r = reconstruct_reel(&observed);
        // Every window was seen once: by Good–Turing the next spin is expected to be new.
        assert_eq!(r.unseen_share, 1.0);
        assert!(matches!(r.strip, Strip::NotAssembled { .. }));
    }

    #[test]
    fn weighted_stops_are_detected_and_the_strip_is_not_asserted() {
        let truth = strip("A B C D E F");
        let mut observed = Vec::new();
        for (stop, window) in windows_of(&truth, 3).into_iter().enumerate() {
            // The first stop is five times as likely as the others.
            for _ in 0..(if stop == 0 { 500 } else { 100 }) {
                observed.push(window.clone());
            }
        }
        let r = reconstruct_reel(&observed);
        assert!(!r.uniformity.unwrap().consistent);
        assert!(matches!(&r.strip, Strip::NotAssembled { reason } if reason.contains("weighted")));
        // The shares still stand: the heavy window is about half of all stops.
        assert_eq!(r.windows[0].count, 500);
        assert!(r.windows[0].share.contains(0.5));
    }

    #[test]
    fn empty_and_ragged_input_do_not_panic() {
        assert!(matches!(
            reconstruct_reel(&[]).strip,
            Strip::NotAssembled { .. }
        ));
        let ragged = vec![strip("A B C"), strip("A B")];
        assert!(matches!(
            reconstruct_reel(&ragged).strip,
            Strip::NotAssembled { .. }
        ));
        let one_row = vec![strip("A"), strip("B")];
        assert!(matches!(
            reconstruct_reel(&one_row).strip,
            Strip::NotAssembled { .. }
        ));
    }
}
