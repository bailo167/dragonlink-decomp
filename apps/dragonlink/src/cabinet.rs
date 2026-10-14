//! The Golden Century cabinet presentation.
//!
//! When an authentic asset pack is configured (`DRAGONLINK_ART`), the game is
//! drawn the way the real game draws it: every element at the position and
//! size its own scene files give, on the game's 1680 x 1050 design canvas,
//! scaled to fit the window. The coordinates below come from the game's scene
//! graph (`main.mercury`, `jackpotMeters.mercury`, `upi.mercury`,
//! `buttondeck.mercury`); a container node transforms its children about the
//! canvas centre (840, 525) by its position and scale.
//!
//! The pack is the owner's own material, built locally and never committed.
//! This module draws only what the pack supplies; if the pack is configured
//! but incomplete the application refuses to start rather than fall back to
//! placeholder tiles.

use std::io::Read;
use std::path::Path;

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::picking::events::PointerPress;
use bevy::ui::widget::NodeImageMode;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use super::*;

/// The game's design canvas.
pub const CANVAS_W: f32 = 1680.0;
pub const CANVAS_H: f32 = 1050.0;

/// The `Slot` container: the reel panel, tabs and reels.
const SLOT_POS: (f32, f32) = (840.0, 554.0);
const SLOT_SCALE: (f32, f32) = (1.1, 1.08);
/// The symbol templates' own scale on the reels.
const SYMBOL_SCALE: (f32, f32) = (0.894, 0.909);
/// The jackpot-meter container.
const METERS_POS: (f32, f32) = (841.0, 602.5);
const METERS_SCALE: (f32, f32) = (1.1, 0.872);

/// UI images the cabinet screen needs from the pack (`ui/<name>.png`).
pub const REQUIRED_UI: [&str; 10] = [
    "background",
    "reel_background",
    "payline_tab",
    "lines_label",
    "jackpot_bar",
    "upi_bar",
    "btn_minus",
    "btn_plus",
    "btn_maxbet",
    "btn_spin_normal",
];

/// The fidelity build: an authentic pack is configured and complete.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fidelity(pub bool);

/// The design canvas, re-centred and scaled to the window every frame.
#[derive(Component)]
pub struct CanvasRoot;

/// A cabinet meter value.
#[derive(Component, Clone, Copy)]
pub enum CabMeter {
    Credit,
    Bet,
    Win,
}

/// Everything the pack must hold for this model, or the list of what is missing.
pub fn check_pack(dir: &str, model: &MathModel) -> Result<(), Vec<String>> {
    let root = Path::new(dir);
    let mut missing = Vec::new();
    for name in REQUIRED_UI {
        let p = root.join("ui").join(format!("{name}.png"));
        if !p.is_file() {
            missing.push(p.display().to_string());
        }
    }
    for s in &model.symbols {
        let p = root.join("symbols").join(format!("{}.png", s.key));
        if !p.is_file() {
            missing.push(p.display().to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// A PNG's pixel size, read from its header.
pub fn png_size(path: &Path) -> Option<(f32, f32)> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut h = [0u8; 24];
    f.read_exact(&mut h).ok()?;
    if &h[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes([h[16], h[17], h[18], h[19]]);
    let ht = u32::from_be_bytes([h[20], h[21], h[22], h[23]]);
    Some((w as f32, ht as f32))
}

/// The primary window for the cabinet: borderless full screen.
pub fn window() -> Window {
    Window {
        title: "Golden Century".into(),
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        ..default()
    }
}

/// Load the meter font from the operating system, if it has one.
pub fn load_font(fonts: &mut Assets<Font>) -> Handle<Font> {
    for p in [
        "/System/Library/Fonts/Supplemental/Impact.ttf",
        "/Library/Fonts/Impact.ttf",
        "C:/Windows/Fonts/impact.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(p) {
            return fonts.add(Font::from_bytes(bytes));
        }
    }
    Handle::default()
}

fn slot(x: f32, y: f32) -> (f32, f32) {
    (
        SLOT_POS.0 + SLOT_SCALE.0 * (x - 840.0),
        SLOT_POS.1 + SLOT_SCALE.1 * (y - 525.0),
    )
}

fn meters(x: f32, y: f32) -> (f32, f32) {
    (
        METERS_POS.0 + METERS_SCALE.0 * (x - 840.0),
        METERS_POS.1 + METERS_SCALE.1 * (y - 525.0),
    )
}

/// The size of one reel cell on the canvas.
pub fn cell_size() -> (f32, f32) {
    (
        264.0 * SYMBOL_SCALE.0 * SLOT_SCALE.0,
        220.0 * SYMBOL_SCALE.1 * SLOT_SCALE.1,
    )
}

/// The on-canvas size of a symbol image of `src` pixels.
pub fn symbol_size(src: (f32, f32)) -> (f32, f32) {
    (
        src.0 * SYMBOL_SCALE.0 * SLOT_SCALE.0,
        src.1 * SYMBOL_SCALE.1 * SLOT_SCALE.1,
    )
}

/// An image centred at `(cx, cy)` on the canvas, `w` by `h`.
fn sprite(img: Handle<Image>, cx: f32, cy: f32, w: f32, h: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(cy - h / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        ImageNode {
            image: img,
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    )
}

/// A text block of width `w` centred at `(cx, cy)`.
fn label(
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
    cx: f32,
    cy: f32,
    w: f32,
) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: bevy::text::FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(cy - size * 0.62),
            width: Val::Px(w),
            ..default()
        },
    )
}

/// Build the base-game screen.
pub fn spawn(commands: &mut Commands, model: &MathModel, art: &ArtPack, font: &Handle<Font>) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    let gold = Color::srgb(1.0, 0.84, 0.35);
    let white = Color::srgb(1.0, 1.0, 0.96);
    commands
        .spawn((
            CanvasRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(CANVAS_W),
                height: Val::Px(CANVAS_H),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|c| {
            // The Great Wall backdrop (overscanned past the canvas).
            c.spawn(sprite(ui("background"), 840.0, 525.0, 1800.0, 1164.0));

            // The reel panel, payline tabs and "5 LINES" labels.
            let (x, y) = slot(840.0, 539.5);
            c.spawn(sprite(
                ui("reel_background"),
                x,
                y,
                1240.0 * SLOT_SCALE.0,
                629.0 * SLOT_SCALE.1,
            ));
            for (tab_x, label_x) in [(204.5, 192.5), (1472.0, 1485.0)] {
                let (x, y) = slot(tab_x, 535.0);
                c.spawn(sprite(
                    ui("payline_tab"),
                    x,
                    y,
                    112.0 * SLOT_SCALE.0,
                    227.0 * SLOT_SCALE.1,
                ));
                let (x, y) = slot(label_x, 535.0);
                c.spawn(sprite(
                    ui("lines_label"),
                    x,
                    y,
                    112.0 * 0.5 * SLOT_SCALE.0,
                    206.0 * 0.5 * SLOT_SCALE.1,
                ));
            }

            // The reels: one cell per window position, at the game's grid.
            let (cw, ch) = cell_size();
            for reel in 0..model.grid.reels {
                for row in 0..model.grid.rows {
                    let (x, y) = slot(352.0 + 244.0 * reel as f32, 335.0 + 200.0 * row as f32);
                    let key0 = initial_key(model, reel, row);
                    c.spawn((
                        Cell { reel, row },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x - cw / 2.0),
                            top: Val::Px(y - ch / 2.0),
                            width: Val::Px(cw),
                            height: Val::Px(ch),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|cell| {
                        // The symbol key drives the image; it is never drawn.
                        cell.spawn((
                            CellLabel { reel, row },
                            Text::new(key0),
                            TextColor(Color::NONE),
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                        ));
                        cell.spawn((
                            CellImage { reel, row },
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(cw),
                                height: Val::Px(ch),
                                ..default()
                            },
                            ImageNode {
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                            Visibility::Hidden,
                        ));
                    });
                }
            }

            // The moving reels, drawn over the landed cells while they spin.
            spawn_strips(c, model.grid.reels);

            // The jackpot meter bar, directly above the reel panel.
            let (x, y) = meters(840.0, 53.0);
            c.spawn(sprite(
                ui("jackpot_bar"),
                x,
                y,
                1235.0 * METERS_SCALE.0,
                105.0 * METERS_SCALE.1,
            ));
            // Amounts sit in each tier's black display window.
            for (tier, bar_x) in [
                (JackpotTier::Grand, 171.0),
                (JackpotTier::Major, 487.0),
                (JackpotTier::Minor, 776.0),
                (JackpotTier::Mini, 1034.0),
            ] {
                let (x, y) = meters(222.5 + bar_x * 1235.0 / 1225.0, 70.0);
                c.spawn((
                    JackpotMeter { tier },
                    label("-", font, 40.0, white, x, y, 300.0),
                ));
            }

            // The bottom player bar, bottom-anchored at y = 1050.
            c.spawn(sprite(ui("upi_bar"), 850.0, 975.0, 1878.0, 150.0));
            for (button, img, x, y, w, h) in [
                (
                    CabButton::BetDown,
                    "btn_minus",
                    281.0,
                    993.0,
                    67.0 * 0.935,
                    104.0,
                ),
                (
                    CabButton::BetUp,
                    "btn_plus",
                    534.0,
                    993.0,
                    68.0 * 0.935,
                    104.0,
                ),
                (
                    CabButton::MaxBet,
                    "btn_maxbet",
                    1222.0,
                    994.5,
                    257.0 * 0.935,
                    104.0,
                ),
                (
                    CabButton::Spin,
                    "btn_spin_normal",
                    1510.5,
                    990.5,
                    322.0 * 0.935,
                    115.0,
                ),
            ] {
                c.spawn((button, sprite(ui(img), x, y, w, h))).observe(
                    move |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| match button {
                        CabButton::Spin => input.spin = true,
                        CabButton::MaxBet => input.max_bet = true,
                        CabButton::BetUp => input.bet_up = true,
                        CabButton::BetDown => input.bet_down = true,
                    },
                );
            }
            c.spawn((
                CabMeter::Bet,
                label("-", font, 44.0, white, 407.5, 993.0, 190.0),
            ));
            c.spawn((
                CabMeter::Win,
                label("-", font, 50.0, white, 837.0, 990.0, 400.0),
            ));
            c.spawn(label("CREDIT", font, 22.0, gold, 120.0, 940.0, 200.0));
            c.spawn((
                CabMeter::Credit,
                label("-", font, 34.0, white, 120.0, 985.0, 220.0),
            ));

            // The shared presentation systems still expect these; unused here.
            c.spawn((
                HudText,
                Text::new(""),
                Node {
                    display: Display::None,
                    ..default()
                },
            ));
            c.spawn((
                StatusText,
                Text::new(""),
                Node {
                    display: Display::None,
                    ..default()
                },
            ));
        });
}

/// The symbol the window shows before the first spin, from the strips.
fn initial_key(model: &MathModel, reel: u8, row: u8) -> String {
    let strip = &model.reel_strips[reel as usize];
    if strip.is_empty() {
        return String::new();
    }
    let stop = (reel as usize * 7 + 2) % strip.len();
    let id = strip[(stop + row as usize) % strip.len()];
    model.symbols[id.0 as usize].key.clone()
}

/// Keep the canvas centred and scaled to fit the window.
pub fn fit(
    fidelity: Res<Fidelity>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
    mut roots: Query<&mut Node, With<CanvasRoot>>,
) {
    if !fidelity.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let s = (window.width() / CANVAS_W).min(window.height() / CANVAS_H);
    if s <= 0.0 {
        return;
    }
    if (scale.0 - s).abs() > f32::EPSILON {
        scale.0 = s;
    }
    for mut node in &mut roots {
        node.left = Val::Px((window.width() / s - CANVAS_W) / 2.0);
        node.top = Val::Px((window.height() / s - CANVAS_H) / 2.0);
    }
}

/// On the authentic panel the symbol sits directly on the reel face; the
/// placeholder colour the shared presentation paints into a cell is cleared.
pub fn clear_cells(fidelity: Res<Fidelity>, mut cells: Query<&mut BackgroundColor, With<Cell>>) {
    if !fidelity.0 {
        return;
    }
    for mut bg in &mut cells {
        if bg.0 != Color::NONE {
            bg.0 = Color::NONE;
        }
    }
}

/// The cabinet's credit, bet and win meters.
pub fn update_meters(
    fidelity: Res<Fidelity>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    denom: Res<Denom>,
    mut meters: Query<(&CabMeter, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let cents = denom.cents();
    for (meter, mut text) in &mut meters {
        let value = match meter {
            CabMeter::Credit => money(presentation.shown_credits, cents),
            CabMeter::Bet => money(sim.game.total_bet(), cents),
            CabMeter::Win => money(presentation.shown_win, cents),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// The on-screen buttons of the player bar.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum CabButton {
    Spin,
    MaxBet,
    BetUp,
    BetDown,
}

/// Button presses this frame, read by `input` alongside the keyboard.
#[derive(Resource, Default)]
pub struct CabinetInput {
    pub spin: bool,
    pub max_bet: bool,
    pub bet_up: bool,
    pub bet_down: bool,
}

/// Presses are set by the buttons' pointer observers (before `Update`) and
/// cleared once `input` has read them.
pub fn clear_buttons(mut input: ResMut<CabinetInput>) {
    *input = CabinetInput::default();
}

// ---------------------------------------------------------------------------
// Reel motion
// ---------------------------------------------------------------------------

/// The machine's pace: the first reel stops a second after the spin starts,
/// the rest follow one by one.
pub const TIMING: Timing = Timing {
    lead_in: 1.0,
    reel_interval: 0.28,
    event_interval: 0.14,
};

/// Reel speed in symbols per second while spinning.
const SPIN_SPEED: f32 = 17.0;
/// How far a stopping reel runs past its rest position, in symbols.
const OVERSHOOT: f32 = 0.16;
/// How long the bounce back to rest takes, in seconds.
const SETTLE_SECONDS: f32 = 0.13;
/// The upward kick a reel gives as it starts, in symbols, and its duration.
const KICK: f32 = 0.14;
const KICK_SECONDS: f32 = 0.09;
/// Images per reel strip: one above the window, three in it, one below.
const STRIP_SLOTS: usize = 5;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Motion {
    #[default]
    Idle,
    /// Spinning, `t` seconds since the start.
    Spinning { t: f32 },
    /// Running on to `target` (an integer position), then bouncing back.
    Stopping { target: f32, bounce: Option<f32> },
}

/// One reel's scroll state. `pos` is the strip index at the top visible row,
/// as a real number; decreasing it moves the symbols down.
#[derive(Clone, Debug, Default)]
struct ReelState {
    motion: Motion,
    pos: f32,
    /// The landing symbols for rows 0..3, placed at `landing_at` when stopping.
    landing: Vec<String>,
    landing_at: i64,
    /// Set for one frame when the reel hits its stop (for the stop sound).
    stopped_now: bool,
}

/// All reels' motion.
#[derive(Resource, Default)]
pub struct Reels {
    states: Vec<ReelState>,
    /// The symbol keys the reels scroll through while spinning.
    strips: Vec<Vec<String>>,
}

impl Reels {
    /// Whether a reel is moving (so its landed cells are hidden).
    pub fn moving(&self, reel: u8) -> bool {
        self.states
            .get(reel as usize)
            .is_some_and(|s| s.motion != Motion::Idle)
    }

    /// Reels that hit their stop this frame.
    pub fn just_stopped(&self) -> impl Iterator<Item = usize> + '_ {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, s)| s.stopped_now)
            .map(|(i, _)| i)
    }

    fn key_at(&self, reel: usize, n: i64) -> &str {
        let s = &self.states[reel];
        let k = n - s.landing_at;
        if (0..s.landing.len() as i64).contains(&k) {
            return &s.landing[k as usize];
        }
        let strip = &self.strips[reel];
        if strip.is_empty() {
            return "";
        }
        &strip[n.rem_euclid(strip.len() as i64) as usize]
    }
}

/// One image in a reel strip.
#[derive(Component)]
pub struct StripSlot {
    reel: u8,
    index: u8,
}

/// The clip window of one reel strip.
#[derive(Component)]
pub struct StripColumn {
    reel: u8,
}

fn reel_center_x(reel: u8) -> f32 {
    slot(352.0 + 244.0 * reel as f32, 0.0).0
}

fn row_center_y(row: f32) -> f32 {
    slot(0.0, 335.0 + 200.0 * row).1
}

fn pitch() -> f32 {
    200.0 * SLOT_SCALE.1
}

fn column_width() -> f32 {
    244.0 * SLOT_SCALE.0
}

/// The strips the reels scroll through, as symbol keys.
pub fn reel_strips(model: &MathModel) -> Reels {
    let strips = model
        .reel_strips
        .iter()
        .map(|strip| {
            strip
                .iter()
                .map(|id| model.symbols[id.0 as usize].key.clone())
                .collect()
        })
        .collect::<Vec<Vec<String>>>();
    let states = (0..strips.len())
        .map(|reel| ReelState {
            pos: ((reel * 7 + 2) % strips[reel].len().max(1)) as f32,
            ..default()
        })
        .collect();
    Reels { states, strips }
}

/// The reel strips: a clipped column per reel over the panel, shown only
/// while the reel moves.
pub fn spawn_strips(c: &mut ChildSpawnerCommands, reels: u8) {
    let (cw, _) = cell_size();
    let top = row_center_y(0.0) - pitch() / 2.0;
    let height = pitch() * 3.0;
    for reel in 0..reels {
        let x = reel_center_x(reel);
        c.spawn((
            StripColumn { reel },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x - column_width() / 2.0),
                top: Val::Px(top),
                width: Val::Px(column_width()),
                height: Val::Px(height),
                overflow: Overflow::clip(),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|col| {
            for index in 0..STRIP_SLOTS as u8 {
                col.spawn((
                    StripSlot { reel, index },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px((column_width() - cw) / 2.0),
                        width: Val::Px(cw),
                        ..default()
                    },
                    ImageNode {
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            }
        });
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Run the reels: start them when a spin starts, stop each on its landing
/// symbols when its stop is presented, with the machine's overshoot and
/// bounce, and draw the strips while they move.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn spin_reels(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    anim: Res<ReelAnim>,
    art: Res<ArtPack>,
    mut reels: ResMut<Reels>,
    labels: Query<(&CellLabel, &Text)>,
    mut columns: Query<(&StripColumn, &mut Visibility)>,
    mut slots: Query<(&StripSlot, &mut ImageNode, &mut Node), Without<StripColumn>>,
) {
    if !fidelity.0 || reels.states.is_empty() {
        return;
    }
    let dt = if smoke.frames_left.is_some() {
        SMOKE_FRAME_SECONDS
    } else {
        time.delta_secs().min(0.05)
    };
    for reel in 0..reels.states.len() {
        let spinning = anim.spinning.get(reel).copied().unwrap_or(false);
        let state = &mut reels.states[reel];
        state.stopped_now = false;
        match state.motion {
            Motion::Idle if spinning => {
                state.pos = state.pos.round();
                state.landing.clear();
                state.motion = Motion::Spinning { t: 0.0 };
            }
            Motion::Idle => {}
            Motion::Spinning { .. } if !spinning => {
                // The stop has been presented: the cells hold the landing
                // symbols. They scroll in from above.
                let mut landing = vec![String::new(); 3];
                for (label, text) in &labels {
                    if label.reel as usize == reel && (label.row as usize) < 3 {
                        landing[label.row as usize] = text.0.clone();
                    }
                }
                let target = state.pos.floor() - 3.0;
                state.landing = landing;
                state.landing_at = target as i64;
                state.motion = Motion::Stopping {
                    target,
                    bounce: None,
                };
            }
            Motion::Spinning { t } => {
                let t = t + dt;
                // A short upward kick, then full speed.
                let v = if t < KICK_SECONDS {
                    -KICK / KICK_SECONDS
                } else {
                    let ramp = ((t - KICK_SECONDS) / 0.12).min(1.0);
                    SPIN_SPEED * ramp.max(0.25)
                };
                state.pos -= v * dt;
                state.motion = Motion::Spinning { t };
            }
            Motion::Stopping {
                target,
                bounce: None,
            } => {
                state.pos -= SPIN_SPEED * dt;
                if state.pos <= target - OVERSHOOT {
                    state.pos = target - OVERSHOOT;
                    state.stopped_now = true;
                    state.motion = Motion::Stopping {
                        target,
                        bounce: Some(0.0),
                    };
                }
            }
            Motion::Stopping {
                target,
                bounce: Some(b),
            } => {
                let b = b + dt;
                let k = (b / SETTLE_SECONDS).min(1.0);
                state.pos = target - OVERSHOOT * (1.0 - ease_out(k));
                state.motion = if k >= 1.0 {
                    state.pos = target;
                    Motion::Idle
                } else {
                    Motion::Stopping {
                        target,
                        bounce: Some(b),
                    }
                };
            }
        }
    }

    // Draw.
    let (cw, ch) = cell_size();
    let p = pitch();
    for (column, mut vis) in &mut columns {
        let want = if reels.moving(column.reel) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (slot, mut image, mut node) in &mut slots {
        let reel = slot.reel as usize;
        if !reels.moving(slot.reel) {
            continue;
        }
        let pos = reels.states[reel].pos;
        let n = pos.floor() as i64 - 1 + slot.index as i64;
        let key = reels.key_at(reel, n).to_string();
        // Centre of this symbol relative to the column top.
        let cy = p / 2.0 + (n as f32 - pos) * p;
        let (w, h) = art
            .sizes
            .get(key.as_str())
            .map_or((cw, ch), |s| symbol_size(*s));
        if let Some(handle) = art.symbols.get(key.as_str())
            && image.image != *handle
        {
            image.image = handle.clone();
        }
        node.top = Val::Px(cy - h / 2.0);
        node.height = Val::Px(h);
        node.width = Val::Px(w);
        node.left = Val::Px((column_width() - w) / 2.0);
    }
}

// ---------------------------------------------------------------------------
// Sound
// ---------------------------------------------------------------------------

/// The game's own sound clips from the pack (`audio/<name>.ogg`), by name.
#[derive(Resource, Default)]
pub struct Sounds {
    clips: std::collections::HashMap<String, Handle<AudioSource>>,
    /// The reel-spin loop while it plays.
    spin_loop: Option<Entity>,
}

impl Sounds {
    /// Load every clip in the pack's `audio/` folder.
    pub fn load(asset_server: &AssetServer, dir: &str) -> Sounds {
        let mut clips = std::collections::HashMap::new();
        if let Ok(entries) = std::fs::read_dir(Path::new(dir).join("audio")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ogg") {
                    continue;
                }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    clips.insert(
                        stem.to_string(),
                        asset_server.load(format!("audio/{stem}.ogg")),
                    );
                }
            }
        }
        Sounds {
            clips,
            spin_loop: None,
        }
    }

    /// Play a clip once.
    pub fn play(&self, commands: &mut Commands, name: &str) {
        if let Some(clip) = self.clips.get(name) {
            commands.spawn((AudioPlayer::new(clip.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

/// The reels' sound: the spin whirr while any reel moves, and each reel's
/// own stop sound as it lands.
pub fn reel_audio(
    fidelity: Res<Fidelity>,
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    reels: Res<Reels>,
) {
    if !fidelity.0 || sounds.clips.is_empty() {
        return;
    }
    let spinning = reels
        .states
        .iter()
        .any(|s| matches!(s.motion, Motion::Spinning { .. }));
    if spinning
        && sounds.spin_loop.is_none()
        && let Some(clip) = sounds.clips.get("whirring_reel")
    {
        let id = commands
            .spawn((AudioPlayer::new(clip.clone()), PlaybackSettings::LOOP))
            .id();
        sounds.spin_loop = Some(id);
    }
    let stopped: Vec<usize> = reels.just_stopped().collect();
    for reel in &stopped {
        let name = format!("reel{}", (reel + 1).min(5));
        sounds.play(&mut commands, &name);
    }
    // The whirr ends as the last reel starts to land.
    if !spinning && let Some(id) = sounds.spin_loop.take() {
        commands.entity(id).despawn();
    }
}
