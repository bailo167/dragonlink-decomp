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
            c.spawn(sprite(ui("btn_minus"), 281.0, 993.0, 67.0 * 0.935, 104.0));
            c.spawn(sprite(ui("btn_plus"), 534.0, 993.0, 68.0 * 0.935, 104.0));
            c.spawn(sprite(
                ui("btn_maxbet"),
                1222.0,
                994.5,
                257.0 * 0.935,
                104.0,
            ));
            c.spawn(sprite(
                ui("btn_spin_normal"),
                1510.5,
                990.5,
                322.0 * 0.935,
                115.0,
            ));
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
