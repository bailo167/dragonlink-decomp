//! dragonlink-capture — from a recording to one still per game.
//!
//! ```text
//! dragonlink-capture rest-frames <VIDEO> --reels X:Y:W:H [--out MOMENTS.json]
//! dragonlink-capture stills      <VIDEO> --moments MOMENTS.json --crop X:Y:W:H --out-dir DIR
//! ```
//!
//! Reading a recording of a machine by hand means looking at the window each
//! game ended on. `rest-frames` finds those moments without anybody watching:
//! it measures how much the reel area changes from one frame to the next, and
//! takes, for every game, the last moment of rest before the next game's
//! reels start to move. `stills` then cuts one picture per moment. The
//! pictures are what gets transcribed into a window sheet
//! (`dragonlink-trace transcribe`).
//!
//! Both commands run `ffmpeg`, which must be on the path. The recording is
//! read and never modified. A recording and stills cut from it are source
//! material: `stills` refuses to write into a Git work tree, and neither
//! belongs in the repository (docs/SOURCES.md).
//!
//! What this does not do: it does not read the pictures, and it knows nothing
//! about any game. The thresholds are in units of average change per pixel on
//! a small grey copy of the reel area; the defaults were found on one
//! hand-held recording of a cabinet and are options because another recording
//! may need others. `rest-frames` prints the spread of the measurements so a
//! threshold can be chosen with open eyes.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dragonlink_assets::enclosing_git_tree;
use serde::{Deserialize, Serialize};

/// Format of the moments file.
const SCHEMA: &str = "dragonlink-moments/1";
/// Size of the grey copy of the reel area the measurement is made on.
const SMALL_WIDTH: usize = 60;
const SMALL_HEIGHT: usize = 18;

#[derive(Parser, Debug)]
#[command(about = "Find the moments in a recording when the reels are at rest, and cut stills")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Find, for every game in a recording, the last moment of rest before
    /// the next game starts.
    RestFrames {
        video: PathBuf,
        /// The reel area in the picture, as X:Y:WIDTH:HEIGHT in pixels.
        #[arg(long, value_parser = parse_rect)]
        reels: Rect,
        /// Frames a second to measure at.
        #[arg(long, default_value_t = 10)]
        fps: u32,
        /// Change per pixel at or above which the reels have started to spin.
        #[arg(long, default_value_t = 22.0)]
        moving: f32,
        /// Change per pixel below which the picture is at rest.
        #[arg(long, default_value_t = 8.0)]
        rest: f32,
        /// Shortest time between two games, in seconds.
        #[arg(long, default_value_t = 1.2)]
        min_gap: f32,
        /// Write the moments to this file. Without it they are only counted.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Cut one still per moment.
    Stills {
        video: PathBuf,
        /// A moments file written by `rest-frames`.
        #[arg(long)]
        moments: PathBuf,
        /// The part of the picture to keep, as X:Y:WIDTH:HEIGHT in pixels.
        #[arg(long, value_parser = parse_rect)]
        crop: Rect,
        /// Folder for the stills. Must not be inside a Git work tree.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// A rectangle of the picture, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn parse_rect(text: &str) -> Result<Rect, String> {
    let parts: Vec<u32> = text
        .split(':')
        .map(|part| part.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("{text:?} is not X:Y:WIDTH:HEIGHT in whole pixels"))?;
    match parts[..] {
        [x, y, width, height] if width > 0 && height > 0 => Ok(Rect {
            x,
            y,
            width,
            height,
        }),
        _ => Err(format!(
            "{text:?} is not X:Y:WIDTH:HEIGHT with a width and a height"
        )),
    }
}

/// The moments found in one recording, and how they were found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Moments {
    schema: String,
    /// File name of the recording, without its folder.
    video: String,
    reels: Rect,
    fps: u32,
    moving: f32,
    rest: f32,
    min_gap: f32,
    /// Frames measured.
    frames: usize,
    /// Seconds into the recording, one per game, in order.
    moments: Vec<f64>,
}

/// How much each frame differs from the one before: the mean absolute
/// difference per byte. The first frame has nothing before it and gets 0.
/// A final partial frame is ignored.
fn difference_series(mut reader: impl Read, frame_bytes: usize) -> std::io::Result<Vec<f32>> {
    let mut series = Vec::new();
    if frame_bytes == 0 {
        return Ok(series);
    }
    let mut previous = vec![0u8; frame_bytes];
    let mut frame = vec![0u8; frame_bytes];
    loop {
        let mut filled = 0;
        while filled < frame_bytes {
            let n = reader.read(&mut frame[filled..])?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        if filled < frame_bytes {
            return Ok(series);
        }
        if series.is_empty() {
            series.push(0.0);
        } else {
            let total: u64 = frame
                .iter()
                .zip(&previous)
                .map(|(a, b)| u64::from(a.abs_diff(*b)))
                .sum();
            series.push(total as f32 / frame_bytes as f32);
        }
        std::mem::swap(&mut previous, &mut frame);
    }
}

/// The frames of rest that end each game: for every start of heavy motion
/// that comes straight out of rest, the frame two before it.
///
/// A game's reels stop one after another, so the change falls away gradually
/// and then the picture is still until the next game starts with all reels at
/// once. That start is the sharpest thing in the series, and the frames just
/// before it are when the window of the game that ended can be read. Two
/// frames back, not one, because the frame straddling the start may already
/// be blurred.
fn rest_frames(series: &[f32], moving: f32, rest: f32, min_gap_frames: usize) -> Vec<usize> {
    let mut starts: Vec<usize> = Vec::new();
    for at in 2..series.len() {
        let from_rest = series[at - 1] < rest && series[at - 2] < rest;
        let far_enough = starts
            .last()
            .is_none_or(|last| at - last >= min_gap_frames.max(1));
        if series[at] >= moving && from_rest && far_enough {
            starts.push(at);
        }
    }
    starts.into_iter().map(|start| start - 2).collect()
}

/// The values of `series` at the given shares of the way through its sorted order.
fn percentiles(series: &[f32], shares: &[f32]) -> Vec<f32> {
    let mut sorted = series.to_vec();
    sorted.sort_by(f32::total_cmp);
    shares
        .iter()
        .filter_map(|share| {
            let at = ((sorted.len() as f32 * share) as usize).min(sorted.len().checked_sub(1)?);
            sorted.get(at).copied()
        })
        .collect()
}

fn ffmpeg() -> Command {
    let mut command = Command::new("ffmpeg");
    command.args(["-nostdin", "-v", "error"]);
    command
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The name of the still for a moment: `t01520.7.jpg` is 1520.7 seconds in.
fn still_name(seconds: f64) -> String {
    format!("t{seconds:07.1}.jpg")
}

fn rest_frames_command(
    video: &Path,
    reels: Rect,
    fps: u32,
    (moving, rest, min_gap): (f32, f32, f32),
    out: Option<&Path>,
) -> Result<()> {
    if fps == 0 {
        bail!("--fps must be at least 1");
    }
    if !video.is_file() {
        bail!("{} is not a file", video.display());
    }
    let filter = format!(
        "crop={}:{}:{}:{},scale={SMALL_WIDTH}:{SMALL_HEIGHT},format=gray",
        reels.width, reels.height, reels.x, reels.y
    );
    let mut child = ffmpeg()
        .arg("-i")
        .arg(video)
        .args([
            "-vf",
            &filter,
            "-r",
            &fps.to_string(),
            "-f",
            "rawvideo",
            "-",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .context("starting ffmpeg (is it installed and on the path?)")?;
    let stdout = child.stdout.take().context("ffmpeg has no output")?;
    let series = difference_series(stdout, SMALL_WIDTH * SMALL_HEIGHT)
        .context("reading frames from ffmpeg")?;
    let status = child.wait().context("waiting for ffmpeg")?;
    if !status.success() {
        bail!("ffmpeg could not read {}", video.display());
    }

    let min_gap_frames = (min_gap * fps as f32).round().max(1.0) as usize;
    let frames = rest_frames(&series, moving, rest, min_gap_frames);
    let moments: Vec<f64> = frames
        .iter()
        .map(|frame| *frame as f64 / f64::from(fps))
        .collect();
    println!(
        "{} frames measured at {fps} a second; {} games found",
        series.len(),
        moments.len()
    );
    let spread = percentiles(&series, &[0.10, 0.25, 0.50, 0.75, 0.90, 0.99]);
    println!(
        "change per pixel at the 10th, 25th, 50th, 75th, 90th and 99th percentile: {}",
        spread
            .iter()
            .map(|v| format!("{v:.1}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if let (Some(first), Some(last)) = (moments.first(), moments.last())
        && moments.len() > 1
    {
        println!(
            "first at {first:.1} s, last at {last:.1} s, {:.1} s between games on average",
            (last - first) / (moments.len() - 1) as f64
        );
    }
    if let Some(out) = out {
        let record = Moments {
            schema: SCHEMA.to_string(),
            video: file_name(video),
            reels,
            fps,
            moving,
            rest,
            min_gap,
            frames: series.len(),
            moments,
        };
        fs::write(out, serde_json::to_string_pretty(&record)? + "\n")
            .with_context(|| format!("writing {}", out.display()))?;
        println!("moments: {}", out.display());
    }
    Ok(())
}

fn stills_command(video: &Path, moments: &Path, crop: Rect, out_dir: &Path) -> Result<()> {
    let text =
        fs::read_to_string(moments).with_context(|| format!("reading {}", moments.display()))?;
    let record: Moments =
        serde_json::from_str(&text).with_context(|| format!("moments {}", moments.display()))?;
    if record.schema != SCHEMA {
        bail!("{} is not a {SCHEMA} file", moments.display());
    }
    if record.video != file_name(video) {
        bail!(
            "these moments were found in {:?}, not in {:?}",
            record.video,
            file_name(video)
        );
    }
    let existing = out_dir
        .ancestors()
        .find(|p| p.exists())
        .unwrap_or(Path::new("."));
    if let Some(tree) = existing
        .canonicalize()
        .ok()
        .as_deref()
        .and_then(enclosing_git_tree)
    {
        bail!(
            "refusing to write stills of a recording into the Git work tree {}: they are source material",
            tree.display()
        );
    }
    fs::create_dir_all(out_dir)?;
    let filter = format!("crop={}:{}:{}:{}", crop.width, crop.height, crop.x, crop.y);
    let mut written = 0usize;
    for seconds in &record.moments {
        let target = out_dir.join(still_name(*seconds));
        if target.is_file() {
            continue;
        }
        let status = ffmpeg()
            .args(["-y", "-ss", &format!("{seconds:.3}"), "-i"])
            .arg(video)
            .args(["-frames:v", "1", "-vf", &filter, "-q:v", "3"])
            .arg(&target)
            .status()
            .context("starting ffmpeg (is it installed and on the path?)")?;
        if !status.success() {
            bail!("ffmpeg could not cut a still at {seconds:.1} s");
        }
        written += 1;
    }
    println!(
        "{written} stills written, {} already there: {}",
        record.moments.len() - written,
        out_dir.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::RestFrames {
            video,
            reels,
            fps,
            moving,
            rest,
            min_gap,
            out,
        } => rest_frames_command(&video, reels, fps, (moving, rest, min_gap), out.as_deref()),
        Cmd::Stills {
            video,
            moments,
            crop,
            out_dir,
        } => stills_command(&video, &moments, crop, &out_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One game as the measurement sees it: all reels start at once, stop one
    /// after another, and then the picture is still.
    fn game(series: &mut Vec<f32>, still_frames: usize) {
        series.extend([
            40.0, 42.0, 38.0, 41.0, 33.0, 30.0, 24.0, 19.0, 13.0, 9.0, 5.0,
        ]);
        series.extend(std::iter::repeat_n(0.6, still_frames));
    }

    #[test]
    fn the_last_frame_of_rest_before_each_game_is_found() {
        let mut series = vec![0.0, 0.5, 0.4];
        let mut expected = Vec::new();
        for still in [6, 9, 5, 30, 7] {
            // The game starts here; the game before it is read two frames back.
            expected.push(series.len() - 2);
            game(&mut series, still);
        }
        assert_eq!(rest_frames(&series, 22.0, 8.0, 12), expected);
        // The recording ends on the last game's rest, which no later start marks.
        assert_eq!(expected.len(), 5);
    }

    #[test]
    fn motion_that_does_not_come_out_of_rest_is_not_a_new_game() {
        // A camera swing: heavy change rising out of moderate change.
        let series = [0.5, 0.5, 12.0, 30.0, 45.0, 30.0, 12.0, 0.5, 0.5];
        assert!(rest_frames(&series, 22.0, 8.0, 12).is_empty());
        // A flicker of one frame at rest inside a spin is not a game either:
        // two frames of rest are needed before a start.
        let series = [0.5, 0.5, 40.0, 30.0, 2.0, 35.0, 30.0, 20.0, 5.0, 0.5];
        assert_eq!(rest_frames(&series, 22.0, 8.0, 1), vec![0]);
    }

    #[test]
    fn two_starts_closer_than_the_shortest_game_count_once() {
        let series = [0.5, 0.5, 40.0, 0.5, 0.5, 40.0, 0.5, 0.5, 0.5, 0.5, 40.0];
        assert_eq!(rest_frames(&series, 22.0, 8.0, 6), vec![0, 8]);
        assert_eq!(rest_frames(&series, 22.0, 8.0, 1), vec![0, 3, 8]);
        assert!(rest_frames(&[], 22.0, 8.0, 6).is_empty());
        assert!(rest_frames(&[50.0, 50.0], 22.0, 8.0, 6).is_empty());
    }

    #[test]
    fn frames_are_compared_byte_by_byte_and_a_partial_frame_is_ignored() {
        // Three frames of four bytes, and two stray bytes.
        let bytes = [10u8, 10, 10, 10, 10, 14, 6, 10, 255, 0, 6, 10, 9, 9];
        let series = difference_series(&bytes[..], 4).unwrap();
        assert_eq!(series, vec![0.0, 2.0, (245.0 + 14.0) / 4.0]);
        assert!(difference_series(&bytes[..3], 4).unwrap().is_empty());
        assert!(difference_series(&bytes[..], 0).unwrap().is_empty());
        // A reader that returns one byte at a time gives the same answer.
        struct Slow<'a>(&'a [u8]);
        impl Read for Slow<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let n = self.0.len().min(1).min(buffer.len());
                buffer[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        assert_eq!(difference_series(Slow(&bytes), 4).unwrap(), series);
    }

    #[test]
    fn rectangles_moments_and_names_are_read_and_written_plainly() {
        assert_eq!(
            parse_rect("200:450:1440:430"),
            Ok(Rect {
                x: 200,
                y: 450,
                width: 1440,
                height: 430
            })
        );
        for bad in [
            "",
            "1:2:3",
            "1:2:3:4:5",
            "a:b:c:d",
            "1:2:0:4",
            "1:2:3:0",
            "-1:2:3:4",
        ] {
            assert!(parse_rect(bad).is_err(), "{bad:?}");
        }
        assert_eq!(still_name(5.4), "t00005.4.jpg");
        assert_eq!(still_name(1520.7), "t01520.7.jpg");
        assert_eq!(
            percentiles(&[3.0, 1.0, 2.0, 4.0], &[0.0, 0.5, 0.99]),
            vec![1.0, 3.0, 4.0]
        );
        assert!(percentiles(&[], &[0.5]).is_empty());
    }

    #[test]
    fn stills_are_refused_inside_a_git_work_tree_and_for_another_recording() {
        let dir = tempfile::tempdir().unwrap();
        let moments = dir.path().join("moments.json");
        let record = Moments {
            schema: SCHEMA.to_string(),
            video: "session.mp4".to_string(),
            reels: parse_rect("0:0:10:10").unwrap(),
            fps: 10,
            moving: 22.0,
            rest: 8.0,
            min_gap: 1.2,
            frames: 0,
            moments: Vec::new(),
        };
        fs::write(&moments, serde_json::to_string(&record).unwrap()).unwrap();
        let crop = parse_rect("0:0:10:10").unwrap();

        // The moments belong to another recording.
        let error = stills_command(
            &dir.path().join("other.mp4"),
            &moments,
            crop,
            &dir.path().join("out"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("not in"), "{error}");

        // A folder inside a work tree.
        let tree = dir.path().join("repo");
        fs::create_dir_all(tree.join(".git")).unwrap();
        let error = stills_command(
            &dir.path().join("session.mp4"),
            &moments,
            crop,
            &tree.join("stills"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("refusing"), "{error}");
        assert!(!tree.join("stills").exists());

        // With no moments there is nothing to cut, and ffmpeg is never started.
        let out = dir.path().join("out");
        stills_command(&dir.path().join("session.mp4"), &moments, crop, &out).unwrap();
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
    }
}
