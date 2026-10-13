//! Do two sets of observed games come from the same reel?
//!
//! Before any strip can be assembled there is a simpler question that needs
//! far fewer games: whether two configurations (two denominations, two line
//! counts, two machines) show the same reel at all. If they do, their games
//! can be pooled; if they do not, pooling them would build a reel that does
//! not exist.
//!
//! The measure is the **share of one set's stops whose window was also seen
//! in the other set**. On one reel it is high once enough games have been
//! seen, and it stays low between two different reels however many games
//! there are.
//!
//! How high is high enough depends on the reel and the sample, so the share
//! is not judged against a fixed threshold. It is judged against what the
//! same games give when the two labels mean nothing: the games are pooled,
//! dealt at random into two sets of the original sizes, and the share is
//! measured again, many times. The **p-value** is the proportion of such
//! deals with a share as low as the observed one or lower. A small p-value
//! says the two sets differ by more than chance arranges; a large one says
//! only that this sample shows no difference, which is not proof of the same
//! reel.
//!
//! The shuffles come from the caller's outcome source, so a result is
//! reproduced exactly from a seed.

use dragonlink_core::{OutcomeRng, RngError};
use serde::Serialize;

/// The comparison of two sets of windows of one reel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowOverlap {
    /// Stops observed in each set.
    pub a_observations: usize,
    pub b_observations: usize,
    /// Distinct windows in each set, and in both.
    pub a_windows: usize,
    pub b_windows: usize,
    pub shared_windows: usize,
    /// Share of `b`'s stops whose window was also seen in `a`.
    pub share: f64,
    /// Shuffles made.
    pub shuffles: u32,
    /// Proportion of random relabellings with a share this low or lower,
    /// counting the observed one: never below `1 / (shuffles + 1)`.
    pub p_value: f64,
}

fn share_seen(pool: &[usize], order: &[usize], a_len: usize, seen: &mut [u32], mark: u32) -> f64 {
    for index in &order[..a_len] {
        seen[pool[*index]] = mark;
    }
    let b = &order[a_len..];
    let hits = b.iter().filter(|index| seen[pool[**index]] == mark).count();
    hits as f64 / b.len() as f64
}

/// Compare the windows of one reel seen in two sets of games. `None` when
/// either set is empty.
pub fn window_overlap(
    a: &[Vec<String>],
    b: &[Vec<String>],
    shuffles: u32,
    rng: &mut dyn OutcomeRng,
) -> Result<Option<WindowOverlap>, RngError> {
    if a.is_empty() || b.is_empty() {
        return Ok(None);
    }
    // Windows as small integers, so a shuffle moves numbers and not strings.
    let mut ids: std::collections::BTreeMap<&[String], usize> = std::collections::BTreeMap::new();
    let pool: Vec<usize> = a
        .iter()
        .chain(b)
        .map(|window| {
            let next = ids.len();
            *ids.entry(window.as_slice()).or_insert(next)
        })
        .collect();
    let distinct = |part: &[usize]| {
        let mut seen = vec![false; ids.len()];
        for id in part {
            seen[*id] = true;
        }
        seen
    };
    let (in_a, in_b) = (distinct(&pool[..a.len()]), distinct(&pool[a.len()..]));
    let count = |flags: &[bool]| flags.iter().filter(|f| **f).count();
    let shared = in_a.iter().zip(&in_b).filter(|(x, y)| **x && **y).count();

    let mut order: Vec<usize> = (0..pool.len()).collect();
    let mut seen = vec![0u32; ids.len()];
    let observed = share_seen(&pool, &order, a.len(), &mut seen, 1);
    let mut as_low = 1u64; // the observed arrangement itself
    for round in 0..shuffles {
        // Fisher-Yates.
        for at in (1..order.len()).rev() {
            let bound = u32::try_from(at + 1).map_err(|_| RngError::ZeroBound)?;
            let other = rng.below(bound)? as usize;
            order.swap(at, other);
        }
        let share = share_seen(&pool, &order, a.len(), &mut seen, round.saturating_add(2));
        // A tolerance for the division: equal counts give equal shares.
        if share <= observed + 1e-12 {
            as_low += 1;
        }
    }
    Ok(Some(WindowOverlap {
        a_observations: a.len(),
        b_observations: b.len(),
        a_windows: count(&in_a),
        b_windows: count(&in_b),
        shared_windows: shared,
        share: observed,
        shuffles,
        p_value: as_low as f64 / (f64::from(shuffles) + 1.0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_core::DevRng;

    /// A strip of made-up symbols, and windows of three drawn from it.
    fn strip(seed: u64, len: usize) -> Vec<String> {
        let mut rng = DevRng::new(seed);
        (0..len)
            .map(|_| format!("S{}", rng.below(9).unwrap()))
            .collect()
    }

    fn draws(strip: &[String], games: usize, seed: u64) -> Vec<Vec<String>> {
        let mut rng = DevRng::new(seed);
        (0..games)
            .map(|_| {
                let stop = rng.below(strip.len() as u32).unwrap() as usize;
                (0..3)
                    .map(|row| strip[(stop + row) % strip.len()].clone())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn two_samples_of_one_reel_are_not_told_apart() {
        let reel = strip(1, 60);
        let (a, b) = (draws(&reel, 250, 10), draws(&reel, 250, 11));
        let found = window_overlap(&a, &b, 500, &mut DevRng::new(5))
            .unwrap()
            .unwrap();
        assert!(found.share > 0.9, "{found:?}");
        assert!(found.p_value > 0.05, "{found:?}");
        assert_eq!(found.a_observations, 250);
        assert_eq!(found.shuffles, 500);
    }

    #[test]
    fn samples_of_two_reels_are_told_apart() {
        let (one, other) = (strip(1, 60), strip(2, 60));
        let (a, b) = (draws(&one, 250, 10), draws(&other, 250, 11));
        let found = window_overlap(&a, &b, 500, &mut DevRng::new(5))
            .unwrap()
            .unwrap();
        // The lowest value 500 shuffles can give.
        assert!((found.p_value - 1.0 / 501.0).abs() < 1e-12, "{found:?}");
        assert!(found.share < 0.5);
        assert!(found.shared_windows < found.a_windows);

        // A reel that differs in a few stops only is still told apart, given enough games.
        let mut edited = one.clone();
        for at in [5, 20, 21, 40, 41, 42] {
            edited[at] = "X".to_string();
        }
        let (a, b) = (draws(&one, 600, 10), draws(&edited, 600, 11));
        let found = window_overlap(&a, &b, 500, &mut DevRng::new(5))
            .unwrap()
            .unwrap();
        assert!(found.p_value < 0.01, "{found:?}");
        assert!(found.share > 0.6);
    }

    #[test]
    fn the_same_seed_gives_the_same_answer_and_empty_sets_give_none() {
        let reel = strip(3, 40);
        let (a, b) = (draws(&reel, 80, 1), draws(&reel, 60, 2));
        let first = window_overlap(&a, &b, 200, &mut DevRng::new(9)).unwrap();
        let again = window_overlap(&a, &b, 200, &mut DevRng::new(9)).unwrap();
        assert_eq!(first, again);
        assert_eq!(
            window_overlap(&a, &[], 10, &mut DevRng::new(1)).unwrap(),
            None
        );
        assert_eq!(
            window_overlap(&[], &b, 10, &mut DevRng::new(1)).unwrap(),
            None
        );
        // With no shuffles there is no test, and the p-value says so by being 1.
        let untested = window_overlap(&a, &b, 0, &mut DevRng::new(1))
            .unwrap()
            .unwrap();
        assert!((untested.p_value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_share_is_of_the_second_sets_stops() {
        let w = |s: &str| vec![s.to_string(); 3];
        let a = vec![w("A"), w("B")];
        let b = vec![w("A"), w("A"), w("A"), w("C")];
        let found = window_overlap(&a, &b, 0, &mut DevRng::new(1))
            .unwrap()
            .unwrap();
        assert!((found.share - 0.75).abs() < 1e-12);
        assert_eq!(
            (found.a_windows, found.b_windows, found.shared_windows),
            (2, 2, 1)
        );
    }
}
