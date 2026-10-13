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
//! 3. **A window can sit at more than one stop**, and on real reels several
//!    do: a run of four symbols that occurs twice puts two windows at two
//!    stops each, and a stack of one symbol longer than the window puts the
//!    same window at consecutive stops. Two different things reveal this.
//!    - *The chain.* Every run of `rows − 1` symbols must be left as often as
//!      it is entered. Where the distinct windows enter a run more often
//!      than they leave it, some window that leaves it is used more than
//!      once. The windows are given the fewest extra stops that balance the
//!      chain: the **shortest strip that shows exactly these windows**. That
//!      is derived, not estimated, and it is an assumption only in "the
//!      shortest": the counts are then tested against it.
//!    - *The counts.* A stack's window leads from a run back to the same
//!      run, so the chain balances whatever its number of stops. That number
//!      is estimated from how often the window was seen, and is the one
//!      statistical figure in a strip's length.
//! 4. **Uniform stops are a hypothesis to test.** If every stop is equally
//!    likely, a window is seen in proportion to the stops it sits at. The
//!    counts are tested against that. When the test fails, stops are
//!    weighted, windows are missing, or a window sits at more stops than the
//!    chain requires; the shares still stand, and the strip is not asserted.
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
/// Most runs the chain may be out of balance by. Beyond this the windows are
/// too incomplete for the shortest balancing to mean anything.
const MAX_IMBALANCE: usize = 8;
/// Most steps the search for an order may take.
const SEARCH_BUDGET: u64 = 5_000_000;

/// How a window's number of stops was arrived at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatBasis {
    /// Required for the windows to chain into a cycle. Derived.
    Chain,
    /// A stack: estimated from how often the window was seen.
    Count,
}

/// A window that sits at more than one stop of the reconstructed strip.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Repeat {
    pub symbols: Vec<String>,
    pub stops: usize,
    /// Times the window was seen.
    pub count: u64,
    pub basis: RepeatBasis,
}

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
    /// The counts against "every window is equally likely".
    pub uniformity: Option<Uniformity>,
    /// Stops on the shortest strip that shows exactly these windows, when
    /// the chain could be balanced.
    pub stops: Option<usize>,
    /// Windows that sit at more than one stop of it.
    pub repeats: Vec<Repeat>,
    /// The counts against that strip: each window seen in proportion to its
    /// stops. `None` when the chain could not be balanced.
    pub fit: Option<Uniformity>,
    pub strip: Strip,
}

fn uniformity(counts: &[u64], total: u64) -> Option<Uniformity> {
    fit(counts, &vec![1; counts.len()], total)
}

/// The counts against "each window is seen in proportion to its stops".
fn fit(counts: &[u64], stops: &[usize], total: u64) -> Option<Uniformity> {
    let k = counts.len();
    let all_stops: usize = stops.iter().sum();
    if k < 2 || total == 0 || all_stops == 0 || stops.len() != k {
        return None;
    }
    let per_stop = total as f64 / all_stops as f64;
    let chi_square: f64 = counts
        .iter()
        .zip(stops)
        .map(|(c, m)| {
            let expected = per_stop * *m as f64;
            (*c as f64 - expected).powi(2) / expected
        })
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

/// Every arrangement of `0..n`, by Heap's method.
fn for_each_permutation(n: usize, mut visit: impl FnMut(&[usize])) {
    let mut order: Vec<usize> = (0..n).collect();
    let mut counters = vec![0usize; n];
    visit(&order);
    let mut at = 0;
    while at < n {
        if counters[at] < at {
            if at % 2 == 0 {
                order.swap(0, at);
            } else {
                order.swap(counters[at], at);
            }
            visit(&order);
            counters[at] += 1;
            at = 0;
        } else {
            counters[at] = 0;
            at += 1;
        }
    }
}

/// How many stops each distinct window sits at on the shortest cyclic strip
/// that shows exactly these windows, as far as the chain decides it. Stacks
/// are left at one stop here.
fn chain_stops(windows: &[&Vec<String>], rows: usize) -> Result<Vec<usize>, String> {
    if rows < 2 {
        return Err("a one-row window carries no order".into());
    }
    // Runs of rows-1 symbols, numbered.
    let mut runs: BTreeMap<&[String], usize> = BTreeMap::new();
    let mut ends: Vec<(usize, usize)> = Vec::with_capacity(windows.len());
    for window in windows {
        let mut run_of = |symbols| {
            let next = runs.len();
            *runs.entry(symbols).or_insert(next)
        };
        let from = run_of(&window[..rows - 1]);
        ends.push((from, run_of(&window[1..])));
    }
    let run_count = runs.len();
    let mut leaving: Vec<Vec<usize>> = vec![Vec::new(); run_count];
    let mut balance = vec![0i64; run_count]; // entered minus left
    for (index, (from, to)) in ends.iter().enumerate() {
        leaving[*from].push(index);
        balance[*from] -= 1;
        balance[*to] += 1;
    }
    // Runs entered more often than left, and left more often than entered,
    // once for each time.
    let mut over: Vec<usize> = Vec::new();
    let mut under: Vec<usize> = Vec::new();
    for (run, b) in balance.iter().enumerate() {
        for _ in 0..b.unsigned_abs() {
            if *b > 0 { &mut over } else { &mut under }.push(run);
        }
    }
    let mut stops = vec![1usize; windows.len()];
    if over.is_empty() {
        return Ok(stops);
    }
    if over.len() > MAX_IMBALANCE {
        return Err(format!(
            "{} runs of symbols are entered more often than they are left: windows are missing",
            over.len()
        ));
    }
    // The shortest way from each over-entered run to each under-entered one,
    // through the windows seen (breadth first).
    let mut ways: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for start in &over {
        let mut came_by: Vec<Option<usize>> = vec![None; run_count];
        let mut reached = vec![false; run_count];
        reached[*start] = true;
        let mut queue = std::collections::VecDeque::from([*start]);
        while let Some(run) = queue.pop_front() {
            for window in &leaving[run] {
                let to = ends[*window].1;
                if !reached[to] {
                    reached[to] = true;
                    came_by[to] = Some(*window);
                    queue.push_back(to);
                }
            }
        }
        for target in &under {
            if !reached[*target] {
                continue;
            }
            let mut path = Vec::new();
            let mut run = *target;
            while let Some(window) = came_by[run] {
                path.push(window);
                run = ends[window].0;
            }
            ways.insert((*start, *target), path);
        }
    }
    // Pair them so that the fewest stops are added.
    let mut best: Option<usize> = None;
    let mut answers: Vec<Vec<usize>> = Vec::new();
    for_each_permutation(under.len(), |order| {
        let mut added = vec![0usize; windows.len()];
        let mut cost = 0usize;
        for (from, to) in over.iter().zip(order.iter().map(|at| under[*at])) {
            let Some(path) = ways.get(&(*from, to)) else {
                return;
            };
            cost += path.len();
            for window in path {
                added[*window] += 1;
            }
        }
        if best.is_none_or(|b| cost < b) {
            best = Some(cost);
            answers.clear();
        }
        if best == Some(cost) && !answers.contains(&added) {
            answers.push(added);
        }
    });
    match answers.as_slice() {
        [] => {
            Err("the chain cannot be closed through the windows seen: windows are missing".into())
        }
        // One unseen window leaves a chain that closes only by going round
        // the whole strip a second time. That is a strip which shows exactly
        // these windows, and it is not the reel.
        [added] if 2 * added.iter().sum::<usize>() > windows.len() => Err(
            "closing the chain would put most windows at a second stop: a window is missing".into(),
        ),
        [added] => {
            for (stop, extra) in stops.iter_mut().zip(added) {
                *stop += extra;
            }
            Ok(stops)
        }
        several => Err(format!(
            "the chain can be closed in {} equally short ways that put different windows at more              than one stop: more games are needed to tell them apart",
            several.len()
        )),
    }
}

/// Chain windows into cyclic strips; a window that sits at several stops is
/// given once for each, next to its copies. Returns the number of distinct
/// strips (capped) and one of them.
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
    let mut steps = 0u64;
    while let Some(depth) = path.len().checked_sub(1) {
        steps += 1;
        if steps > SEARCH_BUDGET {
            if example.is_some() {
                // Orders exist; how many could not be counted.
                count = count.max(CANDIDATE_CAP);
                break;
            }
            return Err("too many possible orders to search".into());
        }
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
            // Copies of one window are interchangeable: take them in order,
            // so that each distinct strip is counted once.
            let earlier_copy_unused =
                next > 0 && !used[next - 1] && windows[next - 1] == windows[next];
            if !used[next] && !earlier_copy_unused {
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

    let distinct: Vec<&Vec<String>> = counts.keys().copied().collect();
    let mut stops = None;
    let mut repeats = Vec::new();
    let mut fitted = None;
    let strip = if observed.iter().any(|w| w.len() != rows) {
        Strip::NotAssembled {
            reason: "the windows are not all the same height".into(),
        }
    } else if distinct.is_empty() {
        Strip::NotAssembled {
            reason: "nothing was observed".into(),
        }
    } else {
        match chain_stops(&distinct, rows) {
            Err(reason) => Strip::NotAssembled { reason },
            Ok(mut per_window) => {
                let from_chain = per_window.clone();
                // A stack's stops are not decided by the chain: estimate them
                // from the counts, against the stops everything else has.
                let is_stack = |w: &Vec<String>| w.windows(2).all(|pair| pair[0] == pair[1]);
                for _ in 0..3 {
                    let all: usize = per_window.iter().sum();
                    let per_stop = total as f64 / all as f64;
                    for (index, window) in distinct.iter().enumerate() {
                        if is_stack(window) {
                            let seen = all_counts[index] as f64;
                            per_window[index] = ((seen / per_stop).round() as usize).max(1);
                        }
                    }
                }
                stops = Some(per_window.iter().sum());
                for (index, window) in distinct.iter().enumerate() {
                    if per_window[index] > 1 {
                        repeats.push(Repeat {
                            symbols: (*window).clone(),
                            stops: per_window[index],
                            count: all_counts[index],
                            basis: if per_window[index] == from_chain[index] {
                                RepeatBasis::Chain
                            } else {
                                RepeatBasis::Count
                            },
                        });
                    }
                }
                repeats.sort_by(|a, b| b.stops.cmp(&a.stops).then(a.symbols.cmp(&b.symbols)));
                fitted = fit(&all_counts, &per_window, total);
                if fitted.is_some_and(|f| !f.consistent) {
                    Strip::NotAssembled {
                        reason: "the windows are not seen in proportion to the stops they sit at                                  on the shortest strip that shows them: stops are weighted,                                  windows are missing, or a window sits at more stops than the                                  chain requires"
                            .into(),
                    }
                } else {
                    let expanded: Vec<&Vec<String>> = distinct
                        .iter()
                        .zip(&per_window)
                        .flat_map(|(window, times)| std::iter::repeat_n(*window, *times))
                        .collect();
                    match assemble(&expanded, rows) {
                        Ok((1, symbols)) => Strip::Unique { symbols },
                        Ok((candidates, example)) => Strip::Ambiguous {
                            candidates,
                            example,
                        },
                        Err(reason) => Strip::NotAssembled { reason },
                    }
                }
            }
        }
    };
    ReelReconstruction {
        observations: total,
        rows,
        windows,
        unseen_share,
        uniformity,
        stops,
        repeats,
        fit: fitted,
        strip,
    }
}

/// Reconstruct every reel from a run of events (`ReelStopped`).
pub fn reconstruct_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
) -> Vec<ReelReconstruction> {
    windows_by_reel(events)
        .values()
        .map(|w| reconstruct_reel(w))
        .collect()
}

/// The windows each reel was seen to stop on, in the order observed
/// (`ReelStopped` events).
pub fn windows_by_reel<'a>(
    events: impl IntoIterator<Item = &'a Event>,
) -> BTreeMap<u8, Vec<Vec<String>>> {
    let mut per_reel: BTreeMap<u8, Vec<Vec<String>>> = BTreeMap::new();
    for event in events {
        if let Event::ReelStopped { reel, symbols, .. } = event {
            per_reel.entry(*reel).or_default().push(symbols.clone());
        }
    }
    per_reel
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
    use dragonlink_core::{DevRng, Game, OutcomeRng};
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

    /// `games` uniform stops of `reel`, as an observer would see them.
    fn play(reel: &[String], games: usize, seed: u64) -> Vec<Vec<String>> {
        let mut rng = DevRng::new(seed);
        (0..games)
            .map(|_| {
                let stop = rng.below(reel.len() as u32).unwrap() as usize;
                (0..3)
                    .map(|row| reel[(stop + row) % reel.len()].clone())
                    .collect()
            })
            .collect()
    }

    fn strip_of(r: &ReelReconstruction) -> &[String] {
        match &r.strip {
            Strip::Unique { symbols } => symbols,
            Strip::Ambiguous { example, .. } => example,
            Strip::NotAssembled { reason } => panic!("not assembled: {reason}"),
        }
    }

    #[test]
    fn a_run_of_four_that_occurs_twice_puts_two_windows_at_two_stops() {
        // "A B C D" occurs twice, so the windows A B C and B C D each sit at two stops.
        let reel = strip("A B C D E F G A B C D H I J K L M");
        // Seen once per stop, and seen in play.
        for observed in [windows_of(&reel, 3), play(&reel, 3000, 7)] {
            let r = reconstruct_reel(&observed);
            assert_eq!(r.stops, Some(reel.len()));
            assert_eq!(r.windows.len(), reel.len() - 2);
            let repeated: Vec<_> = r.repeats.iter().map(|x| x.symbols.join(" ")).collect();
            assert_eq!(repeated, ["A B C", "B C D"]);
            assert!(
                r.repeats
                    .iter()
                    .all(|x| x.stops == 2 && x.basis == RepeatBasis::Chain)
            );
            assert!(r.fit.unwrap().consistent);
            // The windows of the strip found are the windows of the reel, stop for stop.
            assert_eq!(
                sorted(windows_of(strip_of(&r), 3)),
                sorted(windows_of(&reel, 3))
            );
        }
        // Plain uniformity alone would have refused this reel.
        assert!(
            !reconstruct_reel(&play(&reel, 3000, 7))
                .uniformity
                .unwrap()
                .consistent
        );
    }

    #[test]
    fn a_stack_longer_than_the_window_is_sized_from_how_often_it_is_seen() {
        // Five S in a row: the window S S S sits at three stops.
        let reel = strip("A B S S S S S C D E F G H I J K");
        let r = reconstruct_reel(&play(&reel, 6000, 3));
        assert_eq!(r.stops, Some(reel.len()));
        assert_eq!(r.repeats.len(), 1);
        assert_eq!(r.repeats[0].symbols, strip("S S S"));
        assert_eq!(r.repeats[0].stops, 3);
        assert_eq!(r.repeats[0].basis, RepeatBasis::Count);
        assert!(same_cycle(strip_of(&r), &reel));
    }

    #[test]
    fn a_missing_window_is_not_papered_over_by_a_second_lap() {
        let reel = strip("A B C D E F G H I J K L");
        let mut observed = windows_of(&reel, 3);
        observed.retain(|w| w != &strip("D E F"));
        // The windows that remain do close into a cycle, by going round twice.
        let r = reconstruct_reel(&observed);
        assert!(
            matches!(&r.strip, Strip::NotAssembled { reason } if reason.contains("missing")),
            "{:?}",
            r.strip
        );
        assert_eq!(r.stops, None);
        // The same in play.
        let mut seen = play(&reel, 2000, 9);
        seen.retain(|w| w != &strip("D E F"));
        assert!(matches!(
            reconstruct_reel(&seen).strip,
            Strip::NotAssembled { .. }
        ));
    }

    #[test]
    fn a_window_seen_too_often_for_its_stops_is_not_explained_away() {
        // A plain reel, one stop of which was favoured.
        let reel = strip("A B C D E F G H I J K L M N O P");
        let mut seen = play(&reel, 1600, 4);
        seen.extend(std::iter::repeat_n(strip("C D E"), 150));
        let r = reconstruct_reel(&seen);
        assert_eq!(r.stops, Some(reel.len()));
        assert!(r.repeats.is_empty());
        assert!(!r.fit.unwrap().consistent);
        assert!(matches!(&r.strip, Strip::NotAssembled { reason } if reason.contains("weighted")));
    }

    #[test]
    fn every_arrangement_is_visited_once() {
        let mut seen = std::collections::BTreeSet::new();
        for_each_permutation(4, |order| {
            seen.insert(order.to_vec());
        });
        assert_eq!(seen.len(), 24);
        let mut visits = 0;
        for_each_permutation(1, |_| visits += 1);
        assert_eq!(visits, 1);
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
