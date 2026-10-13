//! Dragon Link preservation project — the Bevy application.
//!
//! **Placeholder mode.** No original asset is imported yet and no original
//! mathematics is known, so this shell shows generated coloured cells with
//! generic symbol names and runs the synthetic development fixture. A banner
//! says so on screen at all times. Nothing it shows is evidence of how any
//! Dragon Link game looks or behaves (`docs/EVIDENCE.md`).
//!
//! What it is for: proving the boundary between simulation and presentation.
//! The application sends commands to `dragonlink-core` and presents the
//! events it gets back. It never decides an outcome and never computes an
//! award. The only thing it adds is *when* each event is shown, and that
//! timing ([`REEL_STOP_INTERVAL`] and friends) is ours, chosen so the
//! sequence can be followed; none of it was measured on an original machine.
//!
//! Controls: Space or Enter spins, and steps a Hold & Spin feature; Up and
//! Down change the bet; Escape quits (not in a browser, where there is no
//! window to close).
//!
//! `--smoke-frames N` runs unattended: it spins by itself, and exits with
//! success after N frames. The Windows gate uses it (`docs/BUILDING.md`).
//! `--seed N` and `--credits N` set the test outcome source's seed and the
//! starting play credits.
//!
//! On the web (`wasm32-unknown-unknown`, bundled by `tools/dev/web.ps1`) there
//! is no command line, so the defaults apply, and the application draws into
//! the canvas that `web/index.html` provides. Everything specific to that
//! target is behind `cfg(target_arch = "wasm32")`.

use std::collections::VecDeque;

use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use dragonlink_core::{Command, CreditReason, DevRng, Event, Game};
use dragonlink_model::MathModel;
use dragonlink_model::fixtures::{golden_century_synthetic, synthetic_dev_fixture};
use dragonlink_model::{OrbValue, SymbolKind};

/// Seconds between one reel stopping and the next. Presentation only; ours.
const REEL_STOP_INTERVAL: f32 = 0.18;
/// Seconds before the first reel stops. Presentation only; ours.
const SPIN_LEAD_IN: f32 = 0.25;
/// Seconds between other events of a spin. Presentation only; ours.
const EVENT_INTERVAL: f32 = 0.12;
/// Seconds one frame of a smoke run counts for. A fixed step makes an
/// unattended run show the same thing on any display.
const SMOKE_FRAME_SECONDS: f32 = 1.0 / 60.0;
/// Frames between automatic key presses in smoke mode.
const SMOKE_PRESS_EVERY: u32 = 12;
/// Seconds a feature waits between its steps when they play by themselves.
const FEATURE_STEP_PAUSE: f32 = 0.5;
/// Background of a cell on a winning line, and of a feature banner.
const WIN_COLOR: Color = Color::srgb(0.85, 0.62, 0.12);

const USAGE: &str = "usage: dragonlink [--game golden-century|synthetic] [--smoke-frames N [--screenshot FILE]] [--seed N] [--credits N]";

/// Which math configuration the application runs. Both are synthetic: Golden
/// Century has the game's evidenced structure with invented numbers, the dev
/// fixture is deliberately unlike any title (`dragonlink_model::fixtures`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameChoice {
    GoldenCentury,
    Synthetic,
}

impl GameChoice {
    fn model(self) -> MathModel {
        match self {
            GameChoice::GoldenCentury => golden_century_synthetic(),
            GameChoice::Synthetic => synthetic_dev_fixture(),
        }
    }
}
/// Frames before the end of a smoke run at which the screenshot is taken.
const SCREENSHOT_FRAMES_BEFORE_END: u32 = 20;
/// How long before that the run starts looking for a settled frame to picture.
const SCREENSHOT_SETTLE_WINDOW: u32 = 600;

/// The canvas `web/index.html` provides for the application, as a CSS selector.
#[cfg(target_arch = "wasm32")]
const WEB_CANVAS: &str = "#dragonlink-canvas";

/// The line under the meters until something has happened.
#[cfg(not(target_arch = "wasm32"))]
const CONTROLS_HINT: &str =
    "Space: spin    Up/Down: bet    Left/Right: lines    Esc: quit    (features play themselves)";
/// The line under the meters until something has happened. A page has no
/// window to close, so there is no key for quitting.
#[cfg(target_arch = "wasm32")]
const CONTROLS_HINT: &str = "Space: spin    Up/Down: bet    Left/Right: lines";

/// Command-line options.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    game: GameChoice,
    smoke_frames: Option<u32>,
    /// Save a picture of the window near the end of a smoke run.
    screenshot: Option<String>,
    seed: u64,
    credits: u64,
}

impl Default for Cli {
    fn default() -> Self {
        Cli {
            game: GameChoice::GoldenCentury,
            smoke_frames: None,
            screenshot: None,
            seed: 1,
            credits: 10_000,
        }
    }
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .ok_or_else(|| format!("{name} needs a value"))?
                .parse::<u64>()
                .map_err(|_| format!("{name} needs a whole number"))
        };
        match arg.as_str() {
            "--smoke-frames" => {
                cli.smoke_frames = Some(
                    u32::try_from(value("--smoke-frames")?)
                        .map_err(|_| "--smoke-frames is too large".to_string())?,
                );
            }
            "--game" => {
                cli.game = match it.next().map(String::as_str) {
                    Some("golden-century") => GameChoice::GoldenCentury,
                    Some("synthetic") => GameChoice::Synthetic,
                    _ => return Err("--game is golden-century or synthetic".to_string()),
                };
            }
            "--seed" => cli.seed = value("--seed")?,
            "--credits" => cli.credits = value("--credits")?,
            "--screenshot" => {
                cli.screenshot = Some(
                    it.next()
                        .ok_or_else(|| "--screenshot needs a file name".to_string())?
                        .clone(),
                );
            }
            "--help" | "-h" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown option {other}\n{USAGE}")),
        }
    }
    if cli.screenshot.is_some() && cli.smoke_frames.is_none() {
        return Err("--screenshot needs --smoke-frames".to_string());
    }
    Ok(cli)
}

/// The simulation and its outcome source. The only place outcomes come from.
#[derive(Resource)]
struct Sim {
    game: Game,
    rng: DevRng,
}

/// Events waiting to be shown, each with the delay after the one before it.
#[derive(Resource, Default)]
struct Presentation {
    queue: VecDeque<(f32, Event)>,
    wait: f32,
    /// One line describing the last thing that happened.
    status: String,
    /// The credit meter as presented so far. It follows the credit events as
    /// they are shown, so a win does not appear before the reels that made it.
    shown_credits: u64,
    /// The win meter: what the current spin has paid so far, features included.
    shown_win: u64,
    /// Seconds left before a feature plays its next step by itself.
    step_timer: f32,
}

#[derive(Resource)]
struct Smoke {
    frames_left: Option<u32>,
    screenshot: Option<String>,
    frame: u32,
    /// Frames for which the automatic key press is withheld.
    hold: u32,
}

/// One position of the reel window.
#[derive(Component)]
struct Cell {
    reel: u8,
    row: u8,
}

#[derive(Component)]
struct CellLabel {
    reel: u8,
    row: u8,
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct StatusText;

/// Give each event of one command its delay. Reels stop one after another;
/// everything else follows at a steady pace.
fn schedule(events: Vec<Event>) -> Vec<(f32, Event)> {
    let mut first_reel = true;
    events
        .into_iter()
        .map(|event| {
            let delay = match &event {
                Event::BetPlaced { .. } | Event::SpinStarted { .. } | Event::BetChanged { .. } => {
                    0.0
                }
                // The bet leaves the meter at once; a win arrives after what caused it.
                Event::CreditsChanged {
                    reason: CreditReason::Bet,
                    ..
                } => 0.0,
                Event::ReelStopped { .. } if first_reel => {
                    first_reel = false;
                    SPIN_LEAD_IN
                }
                Event::ReelStopped { .. } => REEL_STOP_INTERVAL,
                Event::OrbValueShown { .. } => 0.0,
                _ => EVENT_INTERVAL,
            };
            (delay, event)
        })
        .collect()
}

fn orb_text(value: OrbValue) -> String {
    match value {
        OrbValue::BetMultiple(m) => format!("ORB\nx{m}"),
        OrbValue::Jackpot(tier) => format!("ORB\n{tier:?}"),
    }
}

/// A generated colour for a symbol. Placeholder art: the colours mean nothing.
fn cell_color(kind: Option<SymbolKind>, key: &str) -> Color {
    match kind {
        Some(SymbolKind::Wild) => Color::srgb(0.55, 0.25, 0.65),
        Some(SymbolKind::Scatter) => Color::srgb(0.15, 0.50, 0.55),
        Some(SymbolKind::Orb) => Color::srgb(0.80, 0.60, 0.10),
        _ => {
            let n = key
                .bytes()
                .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b.into()));
            let shade = 0.22 + (n % 5) as f32 * 0.05;
            Color::srgb(shade, shade + 0.04, shade + 0.10)
        }
    }
}

fn setup(mut commands: Commands, sim: Res<Sim>) {
    commands.spawn(Camera2d);
    let model = sim.game.model();
    let banner = format!(
        "PLACEHOLDER MODE - generated shapes, no original assets\n{}\nNot evidence of how any Dragon Link game looks or behaves.",
        model.provenance.banner().replace('\u{2014}', "-")
    );
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(14.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Text::new(banner),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.25)),
                TextLayout::justify(Justify::Center),
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|reels| {
                for reel in 0..model.grid.reels {
                    reels
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|column| {
                            for row in 0..model.grid.rows {
                                column
                                    .spawn((
                                        Cell { reel, row },
                                        Node {
                                            width: Val::Px(112.0),
                                            height: Val::Px(96.0),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.12, 0.13, 0.16)),
                                    ))
                                    .with_children(|cell| {
                                        cell.spawn((
                                            CellLabel { reel, row },
                                            Text::new("-"),
                                            TextFont {
                                                font_size: FontSize::Px(20.0),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                            TextLayout::justify(Justify::Center),
                                        ));
                                    });
                            }
                        });
                }
            });
            root.spawn((
                HudText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                StatusText,
                Text::new(CONTROLS_HINT),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.78, 0.82)),
            ));
        });
}

/// Turn key presses into commands. The simulation answers with events, which
/// are queued for presentation. A refused command only changes the status line.
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    smoke: Res<Smoke>,
    mut sim: ResMut<Sim>,
    mut presentation: ResMut<Presentation>,
    mut exit: MessageWriter<AppExit>,
) {
    // In a browser, leaving the event loop would only freeze the page.
    if cfg!(not(target_arch = "wasm32")) && keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }
    if !presentation.queue.is_empty() {
        return;
    }
    let auto = smoke.frames_left.is_some()
        && smoke.hold == 0
        && smoke.frame.is_multiple_of(SMOKE_PRESS_EVERY);
    // Features play by themselves (see `drive_features`); the player spins and
    // sets the stake.
    let go = (auto || keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter))
        && sim.game.is_idle();
    let bets = sim.game.model().bet.credits_per_line_options.len();
    let bet_index = sim.game.state().bet_index;
    let line_counts = sim.game.model().line_counts();
    let here = line_counts
        .iter()
        .position(|n| *n == sim.game.lines())
        .unwrap_or(0);
    let command = if go {
        Some(Command::Spin)
    } else if keys.just_pressed(KeyCode::ArrowUp) && bet_index + 1 < bets {
        Some(Command::SetBet {
            index: bet_index + 1,
        })
    } else if keys.just_pressed(KeyCode::ArrowDown) && bet_index > 0 {
        Some(Command::SetBet {
            index: bet_index - 1,
        })
    } else if keys.just_pressed(KeyCode::ArrowRight) && here + 1 < line_counts.len() {
        Some(Command::SetLines {
            lines: line_counts[here + 1],
        })
    } else if keys.just_pressed(KeyCode::ArrowLeft) && here > 0 {
        Some(Command::SetLines {
            lines: line_counts[here - 1],
        })
    } else {
        None
    };
    let Some(command) = command else { return };
    let Sim { game, rng } = &mut *sim;
    match game.apply(command, rng) {
        Ok(events) => {
            presentation.queue.extend(schedule(events));
            presentation.wait = presentation.queue.front().map_or(0.0, |(delay, _)| *delay);
        }
        Err(refusal) => presentation.status = format!("{refusal}"),
    }
}

/// Play a feature's steps by itself, with a pause between them, so the player
/// does not drive Hold & Spin or the free games by hand.
fn drive_features(
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut sim: ResMut<Sim>,
    mut presentation: ResMut<Presentation>,
) {
    if !presentation.queue.is_empty() {
        presentation.step_timer = FEATURE_STEP_PAUSE;
        return;
    }
    if sim.game.is_idle() {
        return;
    }
    let dt = if smoke.frames_left.is_some() {
        SMOKE_FRAME_SECONDS
    } else {
        time.delta_secs()
    };
    presentation.step_timer -= dt;
    if presentation.step_timer > 0.0 {
        return;
    }
    presentation.step_timer = FEATURE_STEP_PAUSE;
    let Sim { game, rng } = &mut *sim;
    if let Ok(events) = game.apply(Command::Advance, rng) {
        presentation.queue.extend(schedule(events));
        presentation.wait = presentation.queue.front().map_or(0.0, |(delay, _)| *delay);
    }
}

/// Show queued events when their time comes.
fn present(
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    mut presentation: ResMut<Presentation>,
    mut cells: Query<(&Cell, &mut BackgroundColor)>,
    mut labels: Query<(&CellLabel, &mut Text)>,
) {
    presentation.wait -= if smoke.frames_left.is_some() {
        SMOKE_FRAME_SECONDS
    } else {
        time.delta_secs()
    };
    while presentation.wait <= 0.0 {
        let Some((_, event)) = presentation.queue.pop_front() else {
            presentation.wait = 0.0;
            break;
        };
        let model = sim.game.model();
        let mut set_cell = |reel: u8, row: u8, text: String, color: Color| {
            for (cell, mut background) in &mut cells {
                if cell.reel == reel && cell.row == row {
                    background.0 = color;
                }
            }
            for (label, mut label_text) in &mut labels {
                if label.reel == reel && label.row == row {
                    label_text.0 = text.clone();
                }
            }
        };
        let orb_color = cell_color(Some(SymbolKind::Orb), "");
        match &event {
            Event::BetPlaced { .. } => {
                presentation.shown_win = 0;
            }
            Event::SpinStarted { spin } => {
                // The window shows the reels in motion, not the previous result.
                for reel in 0..model.grid.reels {
                    for row in 0..model.grid.rows {
                        set_cell(reel, row, "~".to_string(), Color::srgb(0.12, 0.13, 0.16));
                    }
                }
                presentation.status = format!("Spin {spin}");
            }
            Event::ReelStopped { reel, symbols, .. } => {
                for (row, key) in symbols.iter().enumerate() {
                    let kind = model.symbols.iter().find(|s| &s.key == key).map(|s| s.kind);
                    set_cell(*reel, row as u8, key.clone(), cell_color(kind, key));
                }
            }
            Event::OrbValueShown { reel, row, value } => {
                set_cell(*reel, *row, orb_text(*value), orb_color);
            }
            Event::LineWin {
                line,
                symbol,
                count,
                award,
            } => {
                // Light up the winning run on its payline.
                if let Some(payline) = model.paylines.get(*line as usize) {
                    for reel in 0..(*count).min(model.grid.reels) {
                        if let Some(row) = payline.0.get(reel as usize) {
                            for (cell, mut background) in &mut cells {
                                if cell.reel == reel && cell.row == *row {
                                    background.0 = WIN_COLOR;
                                }
                            }
                        }
                    }
                }
                presentation.status = format!("Line {}: {symbol} x{count} pays {award}", line + 1);
            }
            Event::ScatterWin {
                symbol,
                count,
                award,
            } => {
                presentation.status = format!("{symbol} x{count} pays {award}");
            }
            Event::FreeGamesStarted { games } => {
                presentation.status = format!("FREE GAMES  -  {games} games won");
            }
            Event::FreeGameStarted { number, remaining } => {
                presentation.status = format!("FREE GAME {number}  ({remaining} to come)");
            }
            Event::MysteryRevealed { symbol, count } => {
                presentation.status = format!("Mystery reveals {symbol} x{count}");
            }
            Event::FreeGamesRetriggered { added, remaining } => {
                presentation.status =
                    format!("RETRIGGER  +{added} free games  ({remaining} to come)");
            }
            Event::FreeGameCompleted { .. } => {}
            Event::FreeGamesCompleted {
                games_played,
                total_award,
            } => {
                presentation.status =
                    format!("FREE GAMES complete: {games_played} games paid {total_award}");
            }
            Event::HoldAndSpinStarted { respins, locked } => {
                // Everything that is not a held orb is cleared for the feature.
                for reel in 0..model.grid.reels {
                    for row in 0..model.grid.rows {
                        set_cell(reel, row, String::new(), Color::srgb(0.07, 0.07, 0.09));
                    }
                }
                for orb in locked {
                    set_cell(orb.reel, orb.row, orb_text(orb.value), orb_color);
                }
                presentation.status = format!("HOLD & SPIN - {respins} respins - Space to respin");
            }
            Event::HoldAndSpinRespin {
                landed,
                respins_left,
                locked_total,
                ..
            } => {
                for orb in landed {
                    set_cell(orb.reel, orb.row, orb_text(orb.value), orb_color);
                }
                presentation.status =
                    format!("{locked_total} held - {respins_left} respins left - Space to respin");
            }
            Event::JackpotAwarded { tier, award } => {
                presentation.status = format!("{tier:?} awarded: {award}");
            }
            Event::HoldAndSpinCompleted {
                total_award,
                full_grid,
            } => {
                presentation.status = if *full_grid {
                    format!("Feature complete, every position held: {total_award}")
                } else {
                    format!("Feature complete: {total_award}")
                };
            }
            Event::SpinCompleted { total_award, .. } if *total_award > 0 => {
                presentation.status = format!("Spin paid {total_award}");
            }
            Event::CreditsChanged { from, to, reason } => {
                presentation.shown_credits = *to;
                if matches!(reason, CreditReason::Win | CreditReason::FeatureAward) {
                    presentation.shown_win += to.saturating_sub(*from);
                }
            }
            _ => {}
        }
        presentation.wait += presentation.queue.front().map_or(0.0, |(delay, _)| *delay);
        if presentation.queue.is_empty() {
            presentation.wait = 0.0;
            break;
        }
    }
}

fn hud(
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    mut hud: Single<&mut Text, (With<HudText>, Without<StatusText>)>,
    mut status: Single<&mut Text, (With<StatusText>, Without<HudText>)>,
) {
    let game = &sim.game;
    hud.0 = format!(
        "CREDITS {}     BET {} ({} x {} lines)     WIN {}     {}",
        presentation.shown_credits,
        game.total_bet(),
        game.credits_per_line(),
        game.lines(),
        presentation.shown_win,
        if !presentation.queue.is_empty() {
            "SPINNING"
        } else if game.is_idle() {
            "READY"
        } else {
            "FEATURE"
        }
    );
    if !presentation.status.is_empty() {
        status.0.clone_from(&presentation.status);
    }
}

fn smoke(
    mut smoke: ResMut<Smoke>,
    presentation: Res<Presentation>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    smoke.frame = smoke.frame.wrapping_add(1);
    smoke.hold = smoke.hold.saturating_sub(1);
    let Some(left) = smoke.frames_left else {
        return;
    };
    // The picture is taken on the first settled frame (nothing waiting to be
    // shown) inside the window before the end, or at the window's last frame.
    let settled = presentation.queue.is_empty() && smoke.hold == 0;
    let in_window = left <= SCREENSHOT_FRAMES_BEFORE_END + SCREENSHOT_SETTLE_WINDOW;
    if ((in_window && settled) || left == SCREENSHOT_FRAMES_BEFORE_END)
        && let Some(path) = smoke.screenshot.take()
    {
        // Keep the window as it is until the picture has been captured.
        smoke.hold = SCREENSHOT_FRAMES_BEFORE_END;
        // Only placeholder mode exists, so a picture of the window shows nothing
        // proprietary. Once imported assets can be shown, a picture of them must
        // not be written into the repository (docs/LEGAL.md, "Screenshots").
        #[cfg(not(target_arch = "wasm32"))]
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        #[cfg(target_arch = "wasm32")]
        let _ = (path, &mut commands);
    }
    if left == 0 {
        exit.write(AppExit::Success);
    } else {
        smoke.frames_left = Some(left - 1);
    }
}

/// The application's window. On the web it is the page's canvas, sized by the
/// element that contains it.
fn primary_window() -> Window {
    Window {
        title: "dragonlink-decomp - PLACEHOLDER MODE (synthetic, not Dragon Link)".into(),
        #[cfg(target_arch = "wasm32")]
        canvas: Some(WEB_CANVAS.into()),
        #[cfg(target_arch = "wasm32")]
        fit_canvas_to_parent: true,
        // The page decides which keys the browser keeps: it stops the game's
        // own keys from scrolling and leaves Tab, F5 and F12 alone.
        #[cfg(target_arch = "wasm32")]
        prevent_default_event_handling: false,
        ..default()
    }
}

/// Say in the browser console why there are no original assets on screen.
/// Standard error goes nowhere on this target, and a page has no environment
/// to configure an asset set with.
#[cfg(target_arch = "wasm32")]
fn announce_placeholder_mode() {
    info!(
        "placeholder mode: the web build shows generated shapes and runs the synthetic development fixture; it cannot load an asset set"
    );
}

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_args(&args) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}");
            return AppExit::error();
        }
    };
    // Say plainly why there are no original assets on screen.
    #[cfg(not(target_arch = "wasm32"))]
    match dragonlink_assets::discover() {
        dragonlink_assets::AssetMode::Placeholder { reason } => {
            eprintln!("placeholder mode: {reason}");
        }
        dragonlink_assets::AssetMode::Imported { manifest, .. } => eprintln!(
            "placeholder mode: an asset set with {} assets was found, but the application cannot display imported assets yet",
            manifest.entries.len()
        ),
    }
    let game = match Game::new(cli.game.model(), cli.credits) {
        Ok(game) => game,
        Err(e) => {
            eprintln!("cannot start the game: {e}");
            return AppExit::error();
        }
    };
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(primary_window()),
        ..default()
    }));
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Startup, announce_placeholder_mode);
    app.insert_resource(ClearColor(Color::srgb(0.03, 0.03, 0.05)))
        .insert_resource(Sim {
            game,
            rng: DevRng::new(cli.seed),
        })
        .insert_resource(Presentation {
            shown_credits: cli.credits,
            ..default()
        })
        .insert_resource(Smoke {
            frames_left: cli.smoke_frames,
            screenshot: cli.screenshot,
            frame: 0,
            hold: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (smoke, input, drive_features, present, hud).chain())
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn options_parse_and_bad_ones_are_refused() {
        assert_eq!(parse_args(&[]), Ok(Cli::default()));
        assert_eq!(
            parse_args(&args(&[
                "--smoke-frames",
                "180",
                "--seed",
                "7",
                "--credits",
                "500"
            ])),
            Ok(Cli {
                game: GameChoice::GoldenCentury,
                smoke_frames: Some(180),
                seed: 7,
                credits: 500,
                screenshot: None,
            })
        );
        assert_eq!(
            parse_args(&args(&["--game", "synthetic"])).unwrap().game,
            GameChoice::Synthetic
        );
        assert_eq!(
            parse_args(&args(&[])).unwrap().game,
            GameChoice::GoldenCentury
        );
        assert!(parse_args(&args(&["--game", "nope"])).is_err());
        assert!(parse_args(&args(&["--smoke-frames"])).is_err());
        // A screenshot is taken near the end of a smoke run, so it needs one.
        assert!(parse_args(&args(&["--screenshot", "a.png"])).is_err());
        assert_eq!(
            parse_args(&args(&["--smoke-frames", "90", "--screenshot", "a.png"]))
                .unwrap()
                .screenshot
                .as_deref(),
            Some("a.png")
        );
        assert!(parse_args(&args(&["--seed", "x"])).is_err());
        assert!(parse_args(&args(&["--fullscreen"])).is_err());
        assert!(parse_args(&args(&["--help"])).is_err());
    }

    #[test]
    fn reels_are_staggered_and_no_event_is_dropped_or_reordered() {
        let mut game = Game::new(synthetic_dev_fixture(), 1000).unwrap();
        let mut rng = DevRng::new(3);
        let events = game.apply(Command::Spin, &mut rng).unwrap();
        let scheduled = schedule(events.clone());
        // Presentation adds delays only: same events, same order.
        let shown: Vec<Event> = scheduled.iter().map(|(_, e)| e.clone()).collect();
        assert_eq!(shown, events);
        let reel_delays: Vec<f32> = scheduled
            .iter()
            .filter(|(_, e)| matches!(e, Event::ReelStopped { .. }))
            .map(|(d, _)| *d)
            .collect();
        assert_eq!(
            reel_delays,
            [
                SPIN_LEAD_IN,
                REEL_STOP_INTERVAL,
                REEL_STOP_INTERVAL,
                REEL_STOP_INTERVAL,
                REEL_STOP_INTERVAL
            ]
        );
        assert!(scheduled.iter().all(|(d, _)| *d >= 0.0));
        for (delay, event) in &scheduled {
            match event {
                Event::CreditsChanged {
                    reason: CreditReason::Bet,
                    ..
                } => assert_eq!(*delay, 0.0),
                Event::CreditsChanged { .. } => assert!(*delay > 0.0),
                _ => {}
            }
        }
    }

    #[test]
    fn placeholder_colours_distinguish_special_symbols() {
        let orb = cell_color(Some(SymbolKind::Orb), "ORB");
        let wild = cell_color(Some(SymbolKind::Wild), "WILD");
        let regular = cell_color(Some(SymbolKind::Regular), "A");
        assert_ne!(orb, wild);
        assert_ne!(orb, regular);
        assert_eq!(orb_text(OrbValue::BetMultiple(4)), "ORB\nx4");
    }
}
