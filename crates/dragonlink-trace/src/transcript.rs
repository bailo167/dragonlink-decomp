//! From a sheet of transcribed windows to an observation trace.
//!
//! Reading a recording produces, for each game played, what a person can see
//! when the reels have come to rest: the window, the denomination, the line
//! count and the three meters. A **window sheet** holds one such record per
//! line (JSON Lines). [`sheet_to_trace`] turns the records of one
//! configuration into an observation [`Trace`], which is what the reel
//! reconstruction and [`replay`](crate::replay) read.
//!
//! Nothing is filled in. A reel whose window has an unread position is left
//! out of the trace; an award that could not be read is left out; a prize
//! value that is not a whole multiple of the bet is counted and left out,
//! because the event format cannot say it. The [`SheetSummary`] says how much
//! was left out and why, and checks the meters against each other, which is
//! the cheapest test of whether a sheet was transcribed correctly.
//!
//! One trace holds one configuration (denomination, line count and kind of
//! game), because reels and tables are not assumed to be shared between them.

use std::collections::BTreeMap;

use dragonlink_core::Event;
use dragonlink_model::{JackpotTier, OrbValue, SourceClass, TitleId};
use serde::{Deserialize, Serialize};

use crate::{SCHEMA, Trace, TraceHeader, TraceOrigin, TraceRecord};

/// The symbol key of a prize symbol in a trace. On a sheet it carries what is
/// printed on it (`ORB:2000`, `ORB:MINI`).
pub const ORB: &str = "ORB";

/// One game as transcribed from a recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetRecord {
    /// The still the record was read from, when there was one.
    #[serde(default)]
    pub file: Option<String>,
    /// Seconds into the recording.
    pub t: f64,
    /// `base`, `free_games`, `hold_and_spin`, `moving` or `other`.
    pub state: String,
    /// The denomination as displayed: `1c`, `10c`, `$1`. `?` when unread.
    pub denom: String,
    /// Lines selected, when displayed.
    #[serde(default)]
    pub lines: Option<u32>,
    /// The meters as displayed, without a currency sign. `?` when unread.
    pub credit: String,
    pub bet: String,
    pub win: String,
    /// The status line's text, when legible.
    #[serde(default)]
    pub status: String,
    /// A win message shown beside it (`Line 9 wins 8`), when legible.
    #[serde(default)]
    pub win_text: String,
    /// Reels left to right, each top to bottom. Absent when the picture is
    /// not a reel window.
    pub grid: Option<Vec<Vec<String>>>,
    #[serde(default)]
    pub note: String,
}

/// Why a sheet cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum SheetError {
    #[error("line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("record at {t} s is earlier than the record before it")]
    TimeGoesBack { t: f64 },
    #[error("no record of the sheet matches the selection")]
    NothingSelected,
}

/// Read a sheet: one JSON object per line; blank lines are skipped.
pub fn parse_sheet(text: &str) -> Result<Vec<SheetRecord>, SheetError> {
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        records.push(
            serde_json::from_str(line).map_err(|source| SheetError::Json {
                line: index + 1,
                source,
            })?,
        );
    }
    Ok(records)
}

/// Cents in one credit of a denomination written as `1c`, `10c`, `$1`, `$2`.
pub fn denomination_cents(text: &str) -> Option<u64> {
    let text = text.trim();
    let cents = if let Some(dollars) = text.strip_prefix('$') {
        dollars.parse::<u64>().ok()?.checked_mul(100)?
    } else {
        text.strip_suffix('c')?.parse::<u64>().ok()?
    };
    (cents > 0).then_some(cents)
}

/// An amount of money as displayed (`617.30`, `1,250.00`, `$5`), in cents.
pub fn money_cents(text: &str) -> Option<u64> {
    let text = text.trim().trim_start_matches('$').replace(',', "");
    let (dollars, cents) = match text.split_once('.') {
        Some((dollars, cents)) => (dollars, cents),
        None => (text.as_str(), "00"),
    };
    if dollars.is_empty()
        || cents.len() != 2
        || !dollars.bytes().all(|b| b.is_ascii_digit())
        || !cents.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    dollars
        .parse::<u64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(cents.parse::<u64>().ok()?)
}

/// Which records of a sheet go into the trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The kind of game. Usually `base`.
    pub state: String,
    /// The denomination as displayed.
    pub denom: String,
    /// The line count.
    pub lines: u32,
}

/// How an observation trace describes itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSource {
    pub title: Option<TitleId>,
    /// `cabinet-au`, `cabinet-us`, `mobile`, with anything narrower.
    pub scope: String,
    pub source_class: SourceClass,
    /// SHA-256 of the recording.
    pub capture_sha256: String,
    pub notes: String,
}

/// How the meters of consecutive records relate. Each pair of consecutive
/// selected records with all meters read is put in exactly one class.
///
/// The credit a record should show is the credit before it (with any win
/// that had not reached the meter yet), less this game's bet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct MeterCheck {
    /// Pairs whose meters were all read.
    pub pairs: usize,
    /// The credit meter shows that amount plus this game's win.
    pub settled: usize,
    /// It shows that amount without this game's win: the win was on the win
    /// meter and had not reached the credit meter when the record was read.
    pub win_pending: usize,
    /// Neither: a misread meter, a game missing between the two records, or
    /// money put in or taken out.
    pub unexplained: usize,
}

/// What a conversion did, and what it left out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SheetSummary {
    /// Records on the sheet.
    pub records: usize,
    /// Records by configuration (`state, denomination, lines`), so the
    /// selections worth making can be seen.
    pub configurations: BTreeMap<String, usize>,
    /// Records the selection matched.
    pub selected: usize,
    /// Of those, records with no window.
    pub without_window: usize,
    /// Games written to the trace.
    pub games: usize,
    /// Reel windows written, and reel windows left out for an unread position.
    pub reels_written: usize,
    pub reels_unread: usize,
    /// Games whose award was written, and games whose win meter was unread or
    /// not a whole number of credits.
    pub awards_written: usize,
    pub awards_unread: usize,
    /// Prize values written; prize values left out because unread; and prize
    /// values left out because they are not a whole multiple of the bet.
    pub prize_values_written: usize,
    pub prize_values_unread: usize,
    pub prize_values_not_bet_multiples: usize,
    pub meters: MeterCheck,
}

/// What is printed on a prize symbol, as the event format can say it.
enum Prize {
    Value(OrbValue),
    Unread,
    NotABetMultiple,
}

fn prize(text: &str, denom_cents: u64, bet_cents: Option<u64>) -> Prize {
    let tier = match text.to_ascii_uppercase().as_str() {
        "MINI" => Some(JackpotTier::Mini),
        "MINOR" => Some(JackpotTier::Minor),
        "MAJOR" => Some(JackpotTier::Major),
        "GRAND" => Some(JackpotTier::Grand),
        _ => None,
    };
    if let Some(tier) = tier {
        return Prize::Value(OrbValue::Jackpot(tier));
    }
    // A figure with a currency sign is money; without one it is credits.
    let cents = if text.starts_with('$') {
        money_cents(text)
    } else {
        text.replace(',', "")
            .parse::<u64>()
            .ok()
            .and_then(|credits| credits.checked_mul(denom_cents))
    };
    let (Some(cents), Some(bet)) = (cents, bet_cents.filter(|bet| *bet > 0)) else {
        return Prize::Unread;
    };
    if cents % bet != 0 {
        return Prize::NotABetMultiple;
    }
    match u32::try_from(cents / bet) {
        Ok(multiple) => Prize::Value(OrbValue::BetMultiple(multiple)),
        Err(_) => Prize::NotABetMultiple,
    }
}

fn configuration(record: &SheetRecord) -> String {
    format!(
        "{}, {}, {}",
        record.state,
        record.denom,
        record
            .lines
            .map_or("lines unread".to_string(), |n| format!("{n} lines"))
    )
}

/// Turn the records of `sheet` that match `selection` into an observation
/// trace. The summary covers the whole sheet where it says so.
pub fn sheet_to_trace(
    sheet: &[SheetRecord],
    source: &ObservationSource,
    selection: &Selection,
) -> Result<(Trace, SheetSummary), SheetError> {
    let mut summary = SheetSummary {
        records: sheet.len(),
        ..SheetSummary::default()
    };
    let mut last_t = f64::NEG_INFINITY;
    for record in sheet {
        if record.t < last_t {
            return Err(SheetError::TimeGoesBack { t: record.t });
        }
        last_t = record.t;
        *summary
            .configurations
            .entry(configuration(record))
            .or_insert(0) += 1;
    }
    let denom_cents = denomination_cents(&selection.denom);
    let selected: Vec<&SheetRecord> = sheet
        .iter()
        .filter(|r| {
            r.state == selection.state
                && r.denom == selection.denom
                && r.lines == Some(selection.lines)
        })
        .collect();
    summary.selected = selected.len();
    let Some(denom_cents) = denom_cents.filter(|_| !selected.is_empty()) else {
        return Err(SheetError::NothingSelected);
    };

    let mut records: Vec<TraceRecord> = Vec::new();
    let mut push = |t: f64, event: Event| {
        records.push(TraceRecord {
            seq: records.len() as u64,
            // Seconds to milliseconds; a recording is far shorter than u64 allows.
            t_ms: Some((t * 1000.0).round().max(0.0) as u64),
            event,
        });
    };
    let mut last_bet: Option<u64> = None;
    // Credit of the previous selected record and the win it had not yet
    // counted in, when its meters were read.
    let mut previous: Option<(u64, u64)> = None;
    for record in &selected {
        let bet_cents = money_cents(&record.bet);
        let win_cents = money_cents(&record.win);
        let credit_cents = money_cents(&record.credit);

        // The meters against the record before.
        let mut pending = 0;
        if let (Some(credit), Some(bet), Some(win), Some((before, pending_before))) =
            (credit_cents, bet_cents, win_cents, previous)
        {
            let base = i128::from(before) + i128::from(pending_before) - i128::from(bet);
            summary.meters.pairs += 1;
            if base + i128::from(win) == i128::from(credit) {
                summary.meters.settled += 1;
            } else if base == i128::from(credit) {
                summary.meters.win_pending += 1;
                pending = win;
            } else {
                summary.meters.unexplained += 1;
            }
        }
        previous = credit_cents
            .filter(|_| bet_cents.is_some() && win_cents.is_some())
            .map(|credit| (credit, pending));

        let Some(grid) = &record.grid else {
            summary.without_window += 1;
            continue;
        };
        let spin = summary.games as u64 + 1;
        summary.games += 1;

        // The bet, when it is a whole number of credits on each line.
        let bet_credits = bet_cents
            .filter(|cents| cents % denom_cents == 0)
            .map(|cents| cents / denom_cents);
        if let Some(total_bet) = bet_credits
            && last_bet != Some(total_bet)
            && total_bet % u64::from(selection.lines) == 0
            && let Ok(credits_per_line) = u32::try_from(total_bet / u64::from(selection.lines))
        {
            push(
                record.t,
                Event::BetChanged {
                    credits_per_line,
                    total_bet,
                },
            );
            last_bet = Some(total_bet);
        }
        push(record.t, Event::SpinStarted { spin });

        let mut prizes = Vec::new();
        for (reel, column) in grid.iter().enumerate() {
            let Ok(reel) = u8::try_from(reel) else {
                continue;
            };
            let mut symbols = Vec::with_capacity(column.len());
            let mut unread = column.is_empty();
            for (row, cell) in column.iter().enumerate() {
                if let Some(printed) = cell.strip_prefix("ORB:") {
                    symbols.push(ORB.to_string());
                    if let Ok(row) = u8::try_from(row) {
                        prizes.push((reel, row, printed));
                    }
                } else if cell == ORB {
                    symbols.push(ORB.to_string());
                } else if cell.is_empty() || cell.contains('?') || cell.starts_with("OTHER:") {
                    unread = true;
                } else {
                    symbols.push(cell.clone());
                }
            }
            if unread {
                summary.reels_unread += 1;
            } else {
                summary.reels_written += 1;
                push(
                    record.t,
                    Event::ReelStopped {
                        reel,
                        stop: None,
                        symbols,
                    },
                );
            }
        }
        for (reel, row, printed) in prizes {
            match prize(printed, denom_cents, bet_cents) {
                Prize::Value(value) => {
                    summary.prize_values_written += 1;
                    push(record.t, Event::OrbValueShown { reel, row, value });
                }
                Prize::Unread => summary.prize_values_unread += 1,
                Prize::NotABetMultiple => summary.prize_values_not_bet_multiples += 1,
            }
        }
        match win_cents.filter(|cents| cents % denom_cents == 0) {
            Some(cents) => {
                summary.awards_written += 1;
                push(
                    record.t,
                    Event::SpinCompleted {
                        spin,
                        total_award: cents / denom_cents,
                    },
                );
            }
            None => summary.awards_unread += 1,
        }
    }

    let trace = Trace {
        header: TraceHeader {
            schema: SCHEMA.to_string(),
            origin: TraceOrigin::Observation,
            title: source.title,
            scope: Some(format!(
                "{}; {} game, {}, {} lines",
                source.scope, selection.state, selection.denom, selection.lines
            )),
            math_id: None,
            math_provenance: None,
            rng: None,
            seed: None,
            source_class: source.source_class,
            capture_sha256: Some(source.capture_sha256.clone()),
            notes: source.notes.clone(),
        },
        records,
    };
    Ok((trace, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet line. Everything in these tests is made up for the test.
    fn line(t: f64, denom: &str, lines: u32, meters: [&str; 3], grid: &str) -> String {
        format!(
            r#"{{"file":"t{t}.jpg","t":{t},"state":"base","denom":"{denom}","lines":{lines},"credit":"{}","bet":"{}","win":"{}","status":"","grid":{grid},"note":""}}"#,
            meters[0], meters[1], meters[2]
        )
    }

    const PLAIN: &str =
        r#"[["A","B","C"],["B","C","D"],["C","D","E"],["D","E","A"],["E","A","B"]]"#;
    const PRIZES: &str = r#"[["ORB:200","ORB:MINI","C"],["B","ORB:$3","D"],["C","D","E"],["D","?","A"],["E","A","ORB:75"]]"#;

    fn source() -> ObservationSource {
        ObservationSource {
            title: None,
            scope: "synthetic sheet".to_string(),
            source_class: SourceClass::GameplayRecording,
            capture_sha256: "0".repeat(64),
            notes: "written by a test".to_string(),
        }
    }

    fn select(denom: &str, lines: u32) -> Selection {
        Selection {
            state: "base".to_string(),
            denom: denom.to_string(),
            lines,
        }
    }

    #[test]
    fn money_and_denominations_are_read_as_displayed() {
        assert_eq!(denomination_cents("1c"), Some(1));
        assert_eq!(denomination_cents("10c"), Some(10));
        assert_eq!(denomination_cents("$1"), Some(100));
        assert_eq!(denomination_cents("$2"), Some(200));
        for bad in ["?", "", "c", "$", "0c", "1", "ten"] {
            assert_eq!(denomination_cents(bad), None, "{bad:?}");
        }
        assert_eq!(money_cents("617.30"), Some(61_730));
        assert_eq!(money_cents("1,250.00"), Some(125_000));
        assert_eq!(money_cents("$5"), Some(500));
        assert_eq!(money_cents("0.00"), Some(0));
        for bad in ["?", "", "12.3", "12.345", ".50", "1.2x", "-1.00"] {
            assert_eq!(money_cents(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_sheet_becomes_an_observation_trace_of_one_configuration() {
        let text = [
            line(10.0, "1c", 50, ["100.00", "1.00", "0.00"], PLAIN),
            line(13.5, "1c", 50, ["99.50", "1.00", "0.50"], PLAIN),
            // Another configuration on the same sheet.
            line(20.0, "$1", 5, ["94.50", "5.00", "0.00"], PLAIN),
            line(30.0, "1c", 50, ["98.50", "1.00", "0.00"], PLAIN),
        ]
        .join("\n");
        let sheet = parse_sheet(&text).unwrap();
        let (trace, summary) = sheet_to_trace(&sheet, &source(), &select("1c", 50)).unwrap();
        trace.validate().unwrap();
        assert!(trace.is_about_the_original());
        assert_eq!(trace.header.origin, TraceOrigin::Observation);
        assert_eq!(
            trace.header.scope.as_deref(),
            Some("synthetic sheet; base game, 1c, 50 lines")
        );
        assert_eq!(summary.records, 4);
        assert_eq!(summary.selected, 3);
        assert_eq!(summary.games, 3);
        assert_eq!(summary.reels_written, 15);
        assert_eq!(summary.awards_written, 3);
        assert_eq!(summary.configurations.get("base, 1c, 50 lines"), Some(&3));
        assert_eq!(summary.configurations.get("base, $1, 5 lines"), Some(&1));

        let kinds: Vec<&str> = trace.records.iter().map(|r| r.event.kind()).collect();
        // One bet change at the start, then per game: start, five reels, completion.
        assert_eq!(kinds.len(), 1 + 3 * 7);
        assert_eq!(kinds[0], "bet_changed");
        assert_eq!(
            trace.records[0].event,
            Event::BetChanged {
                credits_per_line: 2,
                total_bet: 100
            }
        );
        assert_eq!(
            trace.records[2].event,
            Event::ReelStopped {
                reel: 0,
                stop: None,
                symbols: vec!["A".into(), "B".into(), "C".into()]
            }
        );
        // The second game paid 50 cents at one cent a credit.
        assert!(trace.records.iter().any(|r| r.event
            == Event::SpinCompleted {
                spin: 2,
                total_award: 50
            }));
        assert_eq!(trace.records[1].t_ms, Some(10_000));
        assert_eq!(trace.records.last().unwrap().t_ms, Some(30_000));
    }

    #[test]
    fn the_meters_are_checked_against_each_other() {
        let text = [
            line(1.0, "1c", 50, ["100.00", "1.00", "0.00"], PLAIN),
            // Win of this game already in the credit meter.
            line(2.0, "1c", 50, ["99.50", "1.00", "0.50"], PLAIN),
            // Win of this game on the win meter, not yet in the credit meter.
            line(3.0, "1c", 50, ["98.50", "1.00", "2.00"], PLAIN),
            // The 2.00 has arrived by now, and this game pays nothing.
            line(4.0, "1c", 50, ["99.50", "1.00", "0.00"], PLAIN),
            line(5.0, "1c", 50, ["98.50", "1.00", "0.00"], PLAIN),
            // A misread digit.
            line(6.0, "1c", 50, ["91.50", "1.00", "0.00"], PLAIN),
            // An unread meter is not a pair.
            line(7.0, "1c", 50, ["?", "1.00", "0.00"], PLAIN),
            line(8.0, "1c", 50, ["89.50", "1.00", "0.00"], PLAIN),
        ]
        .join("\n");
        let (_, summary) =
            sheet_to_trace(&parse_sheet(&text).unwrap(), &source(), &select("1c", 50)).unwrap();
        assert_eq!(
            summary.meters,
            MeterCheck {
                pairs: 5,
                settled: 3,
                win_pending: 1,
                unexplained: 1,
            }
        );
    }

    #[test]
    fn prize_symbols_keep_their_position_and_only_values_that_can_be_said() {
        // Bet 1.00 at 1c is 100 credits.
        let text = line(1.0, "1c", 50, ["100.00", "1.00", "0.00"], PRIZES);
        let (trace, summary) =
            sheet_to_trace(&parse_sheet(&text).unwrap(), &source(), &select("1c", 50)).unwrap();
        // The reel with an unread position is left out, not guessed.
        assert_eq!(summary.reels_written, 4);
        assert_eq!(summary.reels_unread, 1);
        assert!(
            !trace
                .records
                .iter()
                .any(|r| matches!(r.event, Event::ReelStopped { reel: 3, .. }))
        );
        assert!(trace.records.iter().any(|r| r.event
            == Event::ReelStopped {
                reel: 0,
                stop: None,
                symbols: vec!["ORB".into(), "ORB".into(), "C".into()]
            }));
        let values: Vec<_> = trace
            .records
            .iter()
            .filter_map(|r| match r.event {
                Event::OrbValueShown { reel, row, value } => Some((reel, row, value)),
                _ => None,
            })
            .collect();
        assert_eq!(
            values,
            vec![
                (0, 0, OrbValue::BetMultiple(2)),
                (0, 1, OrbValue::Jackpot(JackpotTier::Mini)),
                (1, 1, OrbValue::BetMultiple(3)),
            ]
        );
        // 75 credits on a 100-credit bet is not a whole multiple: counted, not rounded.
        assert_eq!(summary.prize_values_written, 3);
        assert_eq!(summary.prize_values_not_bet_multiples, 1);
        assert_eq!(summary.prize_values_unread, 0);
    }

    #[test]
    fn pictures_that_are_not_windows_and_unread_awards_are_counted() {
        let text = [
            r#"{"t":1.0,"state":"base","denom":"1c","lines":50,"credit":"5.00","bet":"1.00","win":"0.00","grid":null,"note":"a message box"}"#.to_string(),
            line(2.0, "1c", 50, ["4.00", "1.00", "?"], PLAIN),
            String::new(),
        ]
        .join("\n");
        let (trace, summary) =
            sheet_to_trace(&parse_sheet(&text).unwrap(), &source(), &select("1c", 50)).unwrap();
        assert_eq!(summary.selected, 2);
        assert_eq!(summary.without_window, 1);
        assert_eq!(summary.games, 1);
        assert_eq!(summary.awards_unread, 1);
        assert!(
            !trace
                .records
                .iter()
                .any(|r| matches!(r.event, Event::SpinCompleted { .. }))
        );
    }

    #[test]
    fn sheets_that_cannot_be_used_are_errors() {
        assert!(matches!(
            parse_sheet("{\"t\": 1.0}"),
            Err(SheetError::Json { line: 1, .. })
        ));
        // An unknown field is a mistake in the sheet, not something to ignore.
        let extra = line(1.0, "1c", 50, ["1.00", "1.00", "0.00"], PLAIN)
            .replace("\"note\"", "\"guess\":true,\"note\"");
        assert!(matches!(
            parse_sheet(&extra),
            Err(SheetError::Json { line: 1, .. })
        ));
        let sheet = parse_sheet(&line(1.0, "1c", 50, ["1.00", "1.00", "0.00"], PLAIN)).unwrap();
        assert!(matches!(
            sheet_to_trace(&sheet, &source(), &select("2c", 50)),
            Err(SheetError::NothingSelected)
        ));
        assert!(matches!(
            sheet_to_trace(&sheet, &source(), &select("1c", 25)),
            Err(SheetError::NothingSelected)
        ));
        let backwards = [
            line(5.0, "1c", 50, ["1.00", "1.00", "0.00"], PLAIN),
            line(4.0, "1c", 50, ["1.00", "1.00", "0.00"], PLAIN),
        ]
        .join("\n");
        assert!(matches!(
            sheet_to_trace(
                &parse_sheet(&backwards).unwrap(),
                &source(),
                &select("1c", 50)
            ),
            Err(SheetError::TimeGoesBack { .. })
        ));
    }

    #[test]
    fn a_trace_from_a_sheet_feeds_replay_without_inventing_anything() {
        // A sheet of windows the synthetic fixture cannot show: replay reports
        // them as inconsistent with the model instead of forcing a match.
        let text = line(1.0, "1c", 50, ["100.00", "1.00", "0.00"], PLAIN);
        let (trace, _) =
            sheet_to_trace(&parse_sheet(&text).unwrap(), &source(), &select("1c", 50)).unwrap();
        let model = dragonlink_model::fixtures::synthetic_dev_fixture();
        let report = crate::replay(&model, &trace, None).unwrap();
        assert!(!report.inconsistencies.is_empty());
        assert!(!report.agrees());
        assert_eq!(report.comparison.kind, crate::ComparisonKind::MachineryOnly);
    }
}
