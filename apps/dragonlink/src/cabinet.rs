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

use dragonlink_core::Feature;

use super::*;

/// The cabinet canvas: the game screen (down to [`SCREEN_BOTTOM`]) and the
/// pub button deck below it.
pub const CANVAS_W: f32 = 1920.0;
pub const CANVAS_H: f32 = 1300.0;

/// The reel grid: its centre and cell pitch, proportioned like the cabinet's
/// screen (wider reels than the phone layout the scene files describe).
const GRID_CENTER: (f32, f32) = (960.0, 506.0);
const PITCH: (f32, f32) = (324.0, 233.0);
/// Symbols are drawn at their own pixel size times this; the cabinet draws
/// them a little wider than tall.
const SYMBOL_SCALE: (f32, f32) = (1.186, 1.059);
/// The game's reel panel art (1240 x 629) fits a 244 x 200 pitch; it scales
/// with the pitch.
const PANEL_SCALE: (f32, f32) = (PITCH.0 / 244.0, PITCH.1 / 200.0);
/// The jackpots across the top.
const JACKPOT_BAR_W: f32 = 1640.0;
const JACKPOT_BAR_TOP: f32 = 8.0;
/// The screen below the reels, measured against cabinet footage (the screen
/// is 1920 wide; each offset is the footage's, scaled to it).
const REEL_BOTTOM: f32 = GRID_CENTER.1 + 1.5 * PITCH.1;
/// The dark meter band: its top, and the centre line of its meters.
const BAND_TOP: f32 = REEL_BOTTOM + 77.0;
const METER_Y: f32 = REEL_BOTTOM + 140.0;
/// "Tap here for player information and promotion".
const TAP_Y: f32 = REEL_BOTTOM + 214.0;
const SCREEN_BOTTOM: f32 = REEL_BOTTOM + 248.0;
/// The button deck under the screen's bezel.
const DECK_TOP: f32 = SCREEN_BOTTOM + 14.0;

/// Denominations on the deck's top row, in cents.
pub const DECK_DENOMS: [u64; 5] = [1, 2, 5, 10, 100];
/// Line counts on the deck's bottom row.
pub const DECK_LINES: [u16; 5] = [1, 5, 10, 25, 50];

/// The most lines a denomination allows (Australian Golden Century: 50 at
/// 1c and 2c, 25 at 5c and 10c, 5 at $1).
pub fn max_lines(cents: u64) -> u16 {
    match cents {
        0..=2 => 50,
        3..=10 => 25,
        _ => 5,
    }
}

/// UI images the cabinet screen needs from the pack (`ui/<name>.png`).
pub const REQUIRED_UI: [&str; 15] = [
    "background",
    "reel_background",
    "payline_tab",
    "jackpot_bar",
    "btn_spin_normal",
    "orb_mini",
    "orb_minor",
    "orb_major",
    "coin_0",
    "coin_1",
    "coin_2",
    "coin_5",
    "coin_6",
    "coin_8",
    "coin_9",
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
    /// The denomination badge ("2¢").
    Denom,
    /// "MIN BET 50" under the badge.
    MinBet,
    /// "Bet Multiplier 5" above the strip.
    Multiplier,
    /// "50" in a payline tab.
    Lines,
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

/// The cabinet screen's three faces: the condensed meter face, the bold sans
/// of its small captions, and the serif of its promotion line.
pub struct CabFonts {
    pub meter: Handle<Font>,
    pub sans: Handle<Font>,
    pub serif: Handle<Font>,
}

/// Load the cabinet's fonts from the operating system, where it has them.
pub fn load_fonts(fonts: &mut Assets<Font>) -> CabFonts {
    let mut first = |paths: &[&str]| {
        for p in paths {
            if let Ok(bytes) = std::fs::read(p) {
                return fonts.add(Font::from_bytes(bytes));
            }
        }
        Handle::default()
    };
    CabFonts {
        meter: first(&[
            "/System/Library/Fonts/Supplemental/Impact.ttf",
            "/Library/Fonts/Impact.ttf",
            "C:/Windows/Fonts/impact.ttf",
        ]),
        sans: first(&[
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
        ]),
        serif: first(&[
            "/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf",
            "C:/Windows/Fonts/timesbd.ttf",
        ]),
    }
}

/// The centre of a reel cell.
fn cell_center(reel: u8, row: u8) -> (f32, f32) {
    (
        GRID_CENTER.0 + (reel as f32 - 2.0) * PITCH.0,
        GRID_CENTER.1 + (row as f32 - 1.0) * PITCH.1,
    )
}

/// The size of one reel cell on the canvas.
pub fn cell_size() -> (f32, f32) {
    PITCH
}

/// The orb is drawn wider than its image's own proportions: on cabinet
/// footage the coin spans about 0.88 of the reel's width and 0.90 of a row's
/// height.
const ORB_SCALE: (f32, f32) = (1.12, 1.02);

/// The on-canvas size of symbol `key`'s image of `src` pixels.
pub fn symbol_size(key: &str, src: (f32, f32)) -> (f32, f32) {
    let k = if key == "orb" { ORB_SCALE } else { (1.0, 1.0) };
    (src.0 * SYMBOL_SCALE.0 * k.0, src.1 * SYMBOL_SCALE.1 * k.1)
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

/// Build the cabinet: the game screen, the meter strip and the button deck.
pub fn spawn(commands: &mut Commands, model: &MathModel, art: &ArtPack, fonts: &CabFonts) {
    let font = &fonts.meter;
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    let gold = Color::srgb(1.0, 0.86, 0.12);
    let pale = Color::srgb(0.86, 0.9, 1.0);
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
            // The Great Wall backdrop behind the screen.
            c.spawn(sprite(
                ui("background"),
                960.0,
                -30.0 + 621.0,
                1920.0,
                1242.0,
            ));

            // The reel panel and the payline tabs either side.
            let (gx, gy) = GRID_CENTER;
            c.spawn(sprite(
                ui("reel_background"),
                gx,
                gy + 4.5 * PANEL_SCALE.1,
                1240.0 * PANEL_SCALE.0,
                629.0 * PANEL_SCALE.1,
            ));
            for side in [-1.0f32, 1.0] {
                let x = gx + side * (2.0 * PITCH.0 + 196.0);
                c.spawn(sprite(ui("payline_tab"), x, gy, 134.0, 272.0));
                c.spawn((
                    CabMeter::Lines,
                    label("50", font, 64.0, gold, x, gy - 34.0, 134.0),
                ));
                c.spawn(label("LINES", font, 34.0, gold, x, gy + 34.0, 134.0));
            }

            // The reels: one cell per window position.
            let (cw, ch) = cell_size();
            for reel in 0..model.grid.reels {
                for row in 0..model.grid.rows {
                    let (x, y) = cell_center(reel, row);
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
                        spawn_hs_cell(cell, art, reel, row);
                        spawn_orb_overlay(cell, reel, row);
                        spawn_hs_spin(cell, art, reel, row);
                    });
                }
            }

            // The moving reels, drawn over the landed cells while they spin.
            spawn_strips(c, model.grid.reels);
            spawn_feature_layer(c, art);

            // The jackpots across the top.
            let (bar_w, bar_top) = (JACKPOT_BAR_W, JACKPOT_BAR_TOP);
            let bar_h = bar_w * 104.0 / 1225.0;
            c.spawn(sprite(
                ui("jackpot_bar"),
                960.0,
                bar_top + bar_h / 2.0,
                bar_w,
                bar_h,
            ));
            let k = bar_w / 1225.0;
            for (tier, bar_x) in [
                (JackpotTier::Grand, 171.0),
                (JackpotTier::Major, 487.0),
                (JackpotTier::Minor, 776.0),
                (JackpotTier::Mini, 1034.0),
            ] {
                c.spawn((
                    JackpotMeter { tier },
                    label(
                        "-",
                        font,
                        56.0,
                        Color::srgb(1.0, 1.0, 0.96),
                        960.0 - bar_w / 2.0 + bar_x * k,
                        bar_top + 70.0 * k,
                        420.0,
                    ),
                ));
            }

            spawn_meter_band(c, fonts, art, gold, pale);

            spawn_deck(c, &fonts.sans);

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

/// A small rounded on-screen tab ("GAMES MENU", "GAME RULES"): a gold-rimmed
/// yellow pill with dark lettering, as on the cabinet.
#[allow(clippy::too_many_arguments)]
fn pill(
    c: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    text: &str,
    cx: f32,
    cy: f32,
    w: f32,
    face: Color,
    ink: Color,
) {
    let h = 42.0;
    c.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(cy - h / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            border: UiRect::all(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(h / 2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(face),
        BorderColor::all(Color::srgb(0.85, 0.55, 0.08)),
    ))
    .with_children(|p| {
        // The tab's glossy upper half.
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(48.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.28)),
        ));
        p.spawn((
            Text::new(text),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(21.0),
                ..default()
            },
            TextLayout::no_wrap(),
            TextColor(ink),
        ));
    });
}

/// The rotating prompt above the meters ("3 [scatter] Start FREE GAMES",
/// "6 [orb] Start HOLD & SPIN FEATURE").
#[derive(Component)]
pub struct StartPrompt {
    /// Which of the two prompts this is.
    which: u8,
}

/// The screen below the reels: the bet multiplier, the start prompts, the
/// dark meter band with CREDIT / BET / WIN, the GAME RULES and GAMES MENU tabs,
/// the denomination box and the promotion line, placed as the cabinet places
/// them.
fn spawn_meter_band(
    c: &mut ChildSpawnerCommands,
    fonts: &CabFonts,
    art: &ArtPack,
    gold: Color,
    pale: Color,
) {
    let font = &fonts.meter;
    let white = Color::srgb(0.97, 0.97, 0.97);
    let dark = Color::srgba(0.015, 0.01, 0.03, 0.96);
    let rect = |l: f32, t: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(l),
        top: Val::Px(t),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    };
    // The band, and the promotion row under the denomination box.
    c.spawn((
        rect(0.0, BAND_TOP, 1735.0, SCREEN_BOTTOM - BAND_TOP),
        BackgroundColor(dark),
    ));
    c.spawn((
        rect(
            1735.0,
            METER_Y + 46.0,
            CANVAS_W - 1735.0,
            SCREEN_BOTTOM - METER_Y - 46.0,
        ),
        BackgroundColor(dark),
    ));

    // "Bet Multiplier 5", centred under the reels.
    c.spawn((
        CabMeter::Multiplier,
        label(
            "Bet Multiplier 1",
            &fonts.sans,
            25.0,
            white,
            GRID_CENTER.0 + 31.0,
            REEL_BOTTOM + 34.0,
            700.0,
        ),
    ));

    // The start prompts, right of centre above the band.
    for (which, count, key, text) in [
        (0u8, "3", "pennant", "Start FREE GAMES"),
        (1u8, "6", "orb", "Start HOLD & SPIN FEATURE"),
    ] {
        c.spawn((
            StartPrompt { which },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1200.0),
                top: Val::Px(REEL_BOTTOM + 41.0),
                width: Val::Px(510.0),
                height: Val::Px(36.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            if which == 0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ))
        .with_children(|row| {
            let caption = |s: &str| {
                (
                    Text::new(s),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.sans.clone()),
                        font_size: FontSize::Px(25.0),
                        ..default()
                    },
                    TextColor(white),
                )
            };
            row.spawn(caption(count));
            row.spawn((
                Node {
                    width: Val::Px(34.0),
                    height: Val::Px(30.0),
                    ..default()
                },
                ImageNode {
                    image: art.symbols.get(key).cloned().unwrap_or_default(),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            row.spawn(caption(text));
        });
    }

    // A feature's message ("Scatter wins 100") takes the prompts' place.
    c.spawn((
        FeatureMessage,
        Text::new(""),
        TextFont {
            font: bevy::text::FontSource::Handle(fonts.sans.clone()),
            font_size: FontSize::Px(25.0),
            ..default()
        },
        TextColor(white),
        TextLayout::justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(1200.0),
            top: Val::Px(REEL_BOTTOM + 43.0),
            width: Val::Px(510.0),
            ..default()
        },
    ));

    // The player-information button at the far left.
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(16.0)),
            ..rect(10.0, METER_Y - 46.0, 48.0, 92.0)
        },
        BackgroundColor(Color::srgb(0.98, 0.8, 0.14)),
        BorderColor::all(Color::srgb(0.75, 0.45, 0.05)),
    ));
    pill(
        c,
        &fonts.sans,
        "GAME RULES",
        152.0,
        METER_Y + 30.0,
        164.0,
        Color::srgb(1.0, 0.95, 0.72),
        Color::srgb(0.2, 0.22, 0.62),
    );

    // CREDIT / BET / WIN and their values.
    for (text, x, w) in [
        ("CREDIT", 326.0, 240.0),
        ("BET", 841.0, 140.0),
        ("WIN", 1280.0, 140.0),
    ] {
        c.spawn(label(text, font, 68.0, gold, x, METER_Y, w));
    }
    for (meter, x) in [
        (CabMeter::Credit, 598.0),
        (CabMeter::Bet, 1045.0),
        (CabMeter::Win, 1529.0),
    ] {
        c.spawn((meter, label("-", font, 74.0, pale, x, METER_Y, 360.0)));
    }

    // GAMES MENU and the denomination box at the right.
    pill(
        c,
        &fonts.sans,
        "GAMES MENU",
        1827.0,
        REEL_BOTTOM + 44.0,
        168.0,
        Color::srgb(0.99, 0.82, 0.25),
        Color::srgb(0.24, 0.14, 0.04),
    );
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..rect(1745.0, REEL_BOTTOM + 74.0, 168.0, 100.0)
        },
        BackgroundColor(Color::srgba(0.07, 0.06, 0.06, 0.92)),
        BorderColor::all(Color::srgb(0.42, 0.38, 0.3)),
    ));
    c.spawn((
        CabMeter::Denom,
        label("1¢", font, 74.0, gold, 1829.0, REEL_BOTTOM + 116.0, 168.0),
    ));
    c.spawn((
        CabMeter::MinBet,
        label(
            "MIN BET 50",
            &fonts.sans,
            19.0,
            white,
            1829.0,
            REEL_BOTTOM + 160.0,
            168.0,
        ),
    ));

    c.spawn(label(
        "Tap here for player information and promotion",
        &fonts.serif,
        27.0,
        Color::srgb(0.93, 0.2, 0.42),
        GRID_CENTER.0 + 50.0,
        TAP_Y,
        1000.0,
    ));
}

/// Alternate the two start prompts every few seconds.
pub fn start_prompts(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    show: Res<FeatureShow>,
    mut prompts: Query<(&StartPrompt, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    let shown = ((time.elapsed_secs() / 3.0) as u32 % 2) as u8;
    let quiet = show.message().is_some() || show.stage != Stage::Base;
    for (p, mut vis) in &mut prompts {
        let want = if p.which == shown && !quiet {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// The pub-style button deck below the screen: denominations on the top row,
/// line counts on the bottom row, SPIN on the right. The
/// buttons are lit plastic caps in chrome bezels on a dark steel panel.
fn spawn_deck(c: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    // The black bezel under the screen, and the deck panel.
    c.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(SCREEN_BOTTOM),
            width: Val::Px(CANVAS_W),
            height: Val::Px(CANVAS_H - SCREEN_BOTTOM),
            ..default()
        },
        BackgroundColor(Color::srgb(0.02, 0.02, 0.02)),
    ));
    c.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(DECK_TOP),
            width: Val::Px(CANVAS_W),
            height: Val::Px(CANVAS_H - DECK_TOP),
            border: UiRect::top(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.11, 0.11, 0.12)),
        BorderColor::all(Color::srgb(0.5, 0.5, 0.53)),
    ))
    .with_children(|d| {
        // The brushed steel's sheen along the panel's top.
        d.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(35.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)),
        ));
    });
    let gap_y = 14.0;
    let bh = (CANVAS_H - DECK_TOP - 4.0 - 3.0 * gap_y) / 2.0;
    let row_y = [DECK_TOP + 4.0 + gap_y, DECK_TOP + 4.0 + 2.0 * gap_y + bh];
    let (bw, gap, left) = (274.0, 22.0, 30.0);
    for (i, cents) in DECK_DENOMS.into_iter().enumerate() {
        let text = if cents >= 100 {
            format!("${}", cents / 100)
        } else {
            format!("{cents}¢")
        };
        deck_button(
            c,
            font,
            CabButton::Denom(i),
            &text,
            left + i as f32 * (bw + gap),
            row_y[0],
            bw,
            bh,
        );
    }
    for (i, lines) in DECK_LINES.into_iter().enumerate() {
        let text = if lines == 1 {
            "1 LINE".to_string()
        } else {
            format!("{lines} LINES")
        };
        deck_button(
            c,
            font,
            CabButton::Lines(lines),
            &text,
            left + i as f32 * (bw + gap),
            row_y[1],
            bw,
            bh,
        );
    }
    deck_button(
        c,
        font,
        CabButton::Spin,
        "SPIN",
        1520.0,
        row_y[0],
        370.0,
        row_y[1] + bh - row_y[0],
    );
}

/// The play button's caption.
#[derive(Component)]
pub struct SpinLabel;

/// The lit cap of a deck button (its colour is set by [`light_deck`]).
#[derive(Component)]
pub struct DeckCap(pub CabButton);

#[allow(clippy::too_many_arguments)]
fn deck_button(
    c: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    button: CabButton,
    text: &str,
    left: f32,
    top: f32,
    w: f32,
    h: f32,
) {
    let size = match button {
        CabButton::Spin => 88.0,
        CabButton::Denom(_) => 38.0,
        _ => 28.0,
    };
    // The chrome bezel, and the backlit cap inside it.
    c.spawn((
        button,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(w),
            height: Val::Px(h),
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.52, 0.53, 0.56)),
        BorderColor::all(Color::srgb(0.82, 0.83, 0.86)),
    ))
    .with_children(|b| {
        b.spawn((
            DeckCap(button),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        ))
        .with_children(|cap| {
            // The light falls brightest on the cap's upper half.
            cap.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(46.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            ));
            cap.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(22.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
            ));
            let mut label = cap.spawn((
                Text::new(text),
                TextFont {
                    font: bevy::text::FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(Color::srgb(0.08, 0.07, 0.05)),
                TextLayout::justify(Justify::Center),
            ));
            if button == CabButton::Spin {
                label.insert(SpinLabel);
            }
        });
    })
    .observe(
        move |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| match button {
            CabButton::Spin => input.spin = true,
            CabButton::Denom(i) => input.denom = Some(i),
            CabButton::Lines(n) => input.lines = Some(n),
        },
    );
}

/// Light the deck: the chosen denomination and line count lit gold, the
/// others lit white, lines above the denomination's maximum dark, and SPIN
/// glowing while a game can start.
#[allow(clippy::too_many_arguments)]
pub fn light_deck(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    presentation: Res<Presentation>,
    show: Res<FeatureShow>,
    mut caps: Query<(&DeckCap, &mut BackgroundColor)>,
    mut spin_label: Query<(&mut Text, &mut TextFont), With<SpinLabel>>,
) {
    if !fidelity.0 {
        return;
    }
    // The play button reads START FEATURE while a feature waits for it.
    let (word, size) = if matches!(show.awaiting, Some(Await::Start(_))) {
        (
            "START
FEATURE",
            52.0,
        )
    } else {
        ("SPIN", 88.0)
    };
    for (mut text, mut font) in &mut spin_label {
        if text.0 != word {
            text.0 = word.to_string();
            font.font_size = FontSize::Px(size);
        }
    }
    let waiting = show.awaiting.is_some();
    let cents = denom.cents();
    let idle = sim.game.is_idle() && presentation.queue.is_empty();
    let lines = sim.game.lines();
    let lit = Color::srgb(0.95, 0.93, 0.84);
    let chosen = Color::srgb(1.0, 0.8, 0.16);
    let dark = Color::srgb(0.2, 0.2, 0.2);
    let glow = 0.82 + 0.18 * (time.elapsed_secs() * 4.0).sin().abs();
    for (cap, mut bg) in &mut caps {
        let face = match cap.0 {
            CabButton::Denom(i) if DECK_DENOMS[i] == cents => chosen,
            CabButton::Denom(_) => lit,
            CabButton::Lines(n) if n > max_lines(cents) => dark,
            CabButton::Lines(n) if n == lines => chosen,
            CabButton::Lines(_) => lit,
            CabButton::Spin if idle || waiting => Color::srgb(0.2 * glow, 0.88 * glow, 0.26 * glow),
            CabButton::Spin => Color::srgb(0.1, 0.3, 0.12),
        };
        if bg.0 != face {
            bg.0 = face;
        }
    }
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
#[allow(clippy::too_many_arguments)]
pub fn update_meters(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    denom: Res<Denom>,
    rollup: Res<Rollup>,
    show: Res<FeatureShow>,
    mut meters: Query<(&CabMeter, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let cents = denom.cents();
    // Credits count up with the win meter.
    let pending = rollup.to.saturating_sub(rollup.shown());
    for (meter, mut text) in &mut meters {
        let value = match meter {
            CabMeter::Credit if show.in_feature() => money(
                presentation
                    .shown_credits
                    .saturating_sub(presentation.shown_win),
                cents,
            ),
            CabMeter::Credit => money(presentation.shown_credits.saturating_sub(pending), cents),
            CabMeter::Bet => money(sim.game.total_bet(), cents),
            CabMeter::Win => match &show.collect {
                Some(c) => money(c.shown, cents),
                None => money(rollup.shown(), cents),
            },
            CabMeter::Denom => {
                if cents >= 100 {
                    format!("${}", cents / 100)
                } else {
                    format!("{cents}¢")
                }
            }
            CabMeter::MinBet => format!("MIN BET {}", sim.game.lines()),
            CabMeter::Multiplier => show
                .caption(time.elapsed_secs())
                .unwrap_or_else(|| format!("Bet Multiplier {}", sim.game.credits_per_line())),
            CabMeter::Lines => sim.game.lines().to_string(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// The cabinet's buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CabButton {
    Spin,
    /// A denomination, by index into [`DECK_DENOMS`].
    Denom(usize),
    /// A line count.
    Lines(u16),
}

/// Button presses this frame, read by `input` alongside the keyboard.
#[derive(Resource, Default)]
pub struct CabinetInput {
    pub spin: bool,
    pub denom: Option<usize>,
    pub lines: Option<u16>,
    /// A SPIN press that completed a win rollup rather than start a game.
    pub skip_spin: bool,
}

/// Presses are set by the buttons' pointer observers (before `Update`) and
/// cleared once `input` has read them.
pub fn clear_buttons(mut input: ResMut<CabinetInput>) {
    *input = CabinetInput::default();
}

// ---------------------------------------------------------------------------
// Reel motion
// ---------------------------------------------------------------------------

/// The machine's pace, measured frame by frame on cabinet footage (base-game
/// spins at 30 fps): the first reel lands about 0.71 s after the reels start
/// (its stop is presented here, then it runs on about 3.4 symbols, ~0.19 s),
/// and each further reel lands 0.45 s after the one before.
pub const TIMING: Timing = Timing {
    lead_in: 0.52,
    reel_interval: 0.45,
    event_interval: 0.14,
};

/// Reel speed in symbols per second while spinning (footage: 20, reached
/// within the first frame, with no visible kick).
const SPIN_SPEED: f32 = 20.0;
/// How far a stopping reel runs past its rest position, in symbols (footage:
/// about 0.35, a hard stop at full speed).
const OVERSHOOT: f32 = 0.35;
/// How long the slide back to rest takes, in seconds (footage: about 0.24,
/// at a near-constant speed).
const SETTLE_SECONDS: f32 = 0.24;
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

    /// The strip index and the symbol (or landed cell label) a strip slot
    /// shows now.
    fn slot_entry(&self, reel: usize, index: u8) -> (i64, &str) {
        let n = self.states[reel].pos.floor() as i64 - 1 + index as i64;
        (n, self.key_at(reel, n))
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

/// The symbol a landed cell label stands for: a lettered orb is an orb.
fn symbol_of(label: &str) -> &str {
    if label.starts_with("ORB") {
        "orb"
    } else {
        label
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
    GRID_CENTER.0 + (reel as f32 - 2.0) * PITCH.0
}

fn row_center_y(row: f32) -> f32 {
    GRID_CENTER.1 + (row - 1.0) * PITCH.1
}

fn pitch() -> f32 {
    PITCH.1
}

fn column_width() -> f32 {
    PITCH.0
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
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .with_children(|slot_node| {
                    slot_node.spawn((
                        StripTier { reel, slot: index },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode {
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                        Visibility::Hidden,
                    ));
                    for digit in 0..COIN_SLOTS {
                        slot_node.spawn((
                            StripDigit {
                                reel,
                                slot: index,
                                index: digit,
                            },
                            Node::default(),
                            ImageNode {
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                            Visibility::Hidden,
                        ));
                    }
                });
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
    let dt = smoke.dt(&time).min(0.05);
    for reel in 0..reels.states.len() {
        let spinning = anim.spinning.get(reel).copied().unwrap_or(false);
        let state = &mut reels.states[reel];
        state.stopped_now = false;
        match state.motion {
            Motion::Idle if spinning => {
                // Start from the last result: the landed symbols stay on the
                // strip and scroll away.
                state.pos = state.pos.round();
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
                // Full speed from the first frame, as on the machine.
                state.pos -= SPIN_SPEED * dt;
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
                // A steady slide back, easing only at the very end.
                let back = 0.85 * k + 0.15 * ease_out(k);
                state.pos = target - OVERSHOOT * (1.0 - back);
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
        let (n, raw) = reels.slot_entry(reel, slot.index);
        let key = symbol_of(raw).to_string();
        // Centre of this symbol relative to the column top.
        let cy = p / 2.0 + (n as f32 - pos) * p;
        let (w, h) = art
            .sizes
            .get(key.as_str())
            .map_or((cw, ch), |s| symbol_size(&key, *s));
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

/// The game's own sound clips from the pack (`audio/<name>.ogg`), by name,
/// with each clip's length.
#[derive(Resource, Default)]
pub struct Sounds {
    clips: std::collections::HashMap<String, (Handle<AudioSource>, f32)>,
    /// Whether any reel was moving last frame.
    reels_were_moving: bool,
}

/// An Ogg Vorbis clip's length in seconds, from its last page's granule
/// position and the identification header's sample rate.
fn ogg_seconds(bytes: &[u8]) -> Option<f32> {
    let id = bytes.windows(7).position(|w| w == b"\x01vorbis")?;
    let rate = u32::from_le_bytes(bytes.get(id + 12..id + 16)?.try_into().ok()?);
    let last = bytes.windows(4).rposition(|w| w == b"OggS")?;
    let granule = i64::from_le_bytes(bytes.get(last + 6..last + 14)?.try_into().ok()?);
    (rate > 0 && granule > 0).then(|| granule as f32 / rate as f32)
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
                    let seconds = std::fs::read(&path)
                        .ok()
                        .and_then(|b| ogg_seconds(&b))
                        .unwrap_or(1.0);
                    clips.insert(
                        stem.to_string(),
                        (asset_server.load(format!("audio/{stem}.ogg")), seconds),
                    );
                }
            }
        }
        Sounds { clips, ..default() }
    }

    /// Play a clip once; returns the playing entity.
    pub fn play(&self, commands: &mut Commands, name: &str) -> Option<Entity> {
        let (clip, _) = self.clips.get(name)?;
        Some(
            commands
                .spawn((AudioPlayer::new(clip.clone()), PlaybackSettings::DESPAWN))
                .id(),
        )
    }

    fn seconds(&self, name: &str) -> Option<f32> {
        self.clips.get(name).map(|(_, s)| *s)
    }
}

/// The reels' sound: each reel's stop clink as it hits its stop (footage:
/// the clink falls on the hard stop). Pressing SPIN and the spinning reels
/// themselves are silent on the machine.
pub fn reel_audio(
    fidelity: Res<Fidelity>,
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    reels: Res<Reels>,
) {
    if !fidelity.0 || sounds.clips.is_empty() {
        return;
    }
    let moving = (0..reels.states.len()).any(|r| reels.moving(r as u8));
    sounds.reels_were_moving = moving;
    let stopped: Vec<usize> = reels.just_stopped().collect();
    for reel in stopped {
        sounds.play(&mut commands, &format!("reel_clink{}", (reel + 1).min(5)));
    }
}

// ---------------------------------------------------------------------------
// Win rollup
// ---------------------------------------------------------------------------

/// The win meter counting up to a win while the game's win tune plays, as on
/// the machine. A press of SPIN during the count completes it.
#[derive(Resource, Default)]
pub struct Rollup {
    from: u64,
    to: u64,
    t: f32,
    duration: f32,
    /// The win total last seen from the presentation.
    seen: u64,
    tune: Option<Entity>,
}

impl Rollup {
    pub fn active(&self) -> bool {
        self.t < self.duration
    }

    /// The win shown on the meter now.
    pub fn shown(&self) -> u64 {
        if self.duration <= 0.0 {
            return self.to;
        }
        let k = (self.t / self.duration).clamp(0.0, 1.0);
        self.from + ((self.to - self.from) as f32 * k).round() as u64
    }
}

/// Which win tune a win plays: the game's 32 tunes rise with the win's size
/// against the bet, from a short sting to the long big-win music.
pub fn win_tune(win: u64, total_bet: u64) -> u32 {
    if total_bet == 0 {
        return 1;
    }
    let ratio = win as f32 / total_bet as f32;
    let idx = 1.0 + (ratio * 4.0 + 1.0).log2() * 3.4;
    (idx.floor() as u32).clamp(1, 32)
}

/// Start, advance and finish the win rollup; count credits up alongside it.
#[allow(clippy::too_many_arguments)]
pub fn win_rollup(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    show: Res<FeatureShow>,
    presentation: Res<Presentation>,
    mut rollup: ResMut<Rollup>,
    sounds: Res<Sounds>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    let target = presentation.shown_win;
    if target < rollup.seen {
        // A new game: the meter starts from nothing and any tune stops.
        if let Some(id) = rollup.tune.take() {
            commands.entity(id).try_despawn();
        }
        *rollup = Rollup::default();
    }
    if target > rollup.seen && show.collected.is_some() {
        // The coins were just counted onto the meter: it shows the total.
        rollup.from = target;
        rollup.to = target;
        rollup.t = 0.0;
        rollup.duration = 0.0;
    } else if target > rollup.seen {
        let from = rollup.shown();
        // The tune follows this win, not what the feature has paid so far.
        let tune = win_tune(target - rollup.seen, sim.game.total_bet());
        let name = format!("wintune{tune:02}");
        if let Some(id) = rollup.tune.take() {
            commands.entity(id).try_despawn();
        }
        rollup.tune = sounds.play(&mut commands, &name);
        rollup.duration = sounds.seconds(&name).unwrap_or(1.5).max(0.3);
        rollup.from = from;
        rollup.to = target;
        rollup.t = 0.0;
    }
    rollup.seen = target;
    if rollup.active() {
        rollup.t += smoke.dt(&time);
    }
}

/// A SPIN press during a rollup completes it (and silences the tune) instead
/// of starting a game.
pub fn rollup_skip(
    fidelity: Res<Fidelity>,
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<CabinetInput>,
    mut rollup: ResMut<Rollup>,
    mut commands: Commands,
) {
    if !fidelity.0 || !rollup.active() {
        return;
    }
    let pressed =
        input.spin || keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    if pressed {
        rollup.t = rollup.duration;
        if let Some(id) = rollup.tune.take() {
            commands.entity(id).try_despawn();
        }
        input.skip_spin = true;
    }
}

// ---------------------------------------------------------------------------
// Orb prizes
// ---------------------------------------------------------------------------

/// One digit slot of the prize lettered on an orb.
#[derive(Component)]
pub struct CoinGlyph {
    reel: u8,
    row: u8,
    index: u8,
}

/// A jackpot-tier label (MINI / MINOR / MAJOR) drawn over an orb.
#[derive(Component)]
pub struct OrbTier {
    reel: u8,
    row: u8,
}

/// Digit slots per orb.
const COIN_SLOTS: u8 = 6;
/// Digit image height as a share of the reel cell's height (cabinet footage:
/// a coin's digits stand about 0.29 of a row tall; the glyph images carry a
/// little padding around the ink).
const COIN_DIGIT_SHARE: f32 = 0.33;

/// An orb's prize as the machine letters it: credits at cent denominations,
/// whole dollars at a dollar or more.
pub fn coin_text(credits: u64, cents: u64) -> String {
    if cents >= 100 {
        (credits * cents / 100).to_string()
    } else {
        credits.to_string()
    }
}

/// The orb prize overlay of one cell: a tier label image, and a centred row of
/// digit images in the game's own coin face.
pub fn spawn_orb_overlay(cell: &mut ChildSpawnerCommands, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    cell.spawn((
        OrbTier { reel, row },
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
    cell.spawn(Node {
        position_type: PositionType::Absolute,
        width: Val::Px(cw),
        height: Val::Px(ch),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row_node| {
        for index in 0..COIN_SLOTS {
            row_node.spawn((
                CoinGlyph { reel, row, index },
                Node::default(),
                ImageNode {
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                Visibility::Hidden,
            ));
        }
    });
}

/// Show each landed orb's prize: its value in the coin digits, or its tier.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn orb_overlay(
    fidelity: Res<Fidelity>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    art: Res<ArtPack>,
    reels: Res<Reels>,
    labels: Query<(&CellLabel, &Text)>,
    mut glyphs: Query<(&CoinGlyph, &mut ImageNode, &mut Visibility, &mut Node), Without<OrbTier>>,
    mut tiers: Query<(&OrbTier, &mut ImageNode, &mut Visibility, &mut Node), Without<CoinGlyph>>,
) {
    if !fidelity.0 {
        return;
    }
    let total_bet = sim.game.total_bet();
    let cents = denom.cents();
    let (_, ch) = cell_size();
    let digit_h = ch * COIN_DIGIT_SHARE;
    // An orb shown before any game was played (drawn from the strips) has no
    // drawn prize; it carries the same cosmetic face it shows on the moving
    // strip, so every coin on screen is lettered, as on the machine.
    let face = |reel: u8, row: u8| -> Option<String> {
        if reels.moving(reel) {
            return None;
        }
        let state = reels.states.get(reel as usize)?;
        let (_, text) = labels
            .iter()
            .find(|(l, _)| l.reel == reel && l.row == row)?;
        match text.0.strip_prefix("ORB\n") {
            Some(f) => Some(f.to_string()),
            None if text.0 == "orb" => {
                Some(passing_face(reel, state.pos.round() as i64 + i64::from(row)).to_string())
            }
            None => None,
        }
    };
    for (g, mut image, mut vis, mut node) in &mut glyphs {
        let text = face(g.reel, g.row)
            .and_then(|f| face_credits(&f, total_bet))
            .map(|c| coin_text(c, cents));
        let glyph = text
            .as_deref()
            .and_then(|t| t.chars().nth(g.index as usize))
            .and_then(|c| {
                let key = format!("coin_{c}");
                art.ui.get(&key).zip(art.sizes.get(&key))
            });
        match glyph {
            Some((handle, (w, h))) => {
                if image.image != *handle {
                    image.image = handle.clone();
                }
                let scale = digit_h / 73.0;
                node.width = Val::Px(w * scale);
                node.height = Val::Px(h * scale);
                node.display = Display::Flex;
                *vis = Visibility::Visible;
            }
            // An unused digit takes no room, so the prize stays centred.
            None => {
                node.display = Display::None;
                *vis = Visibility::Hidden;
            }
        }
    }
    // The tier plate covers the coin itself, at the coin's drawn size.
    let (cw, ch) = cell_size();
    let (pw, ph) = art
        .sizes
        .get("orb")
        .map_or((cw, ch), |s| symbol_size("orb", *s));
    for (o, mut image, mut vis, mut node) in &mut tiers {
        let Some(h) = face(o.reel, o.row)
            .as_deref()
            .and_then(tier_image)
            .and_then(|k| art.ui.get(k))
        else {
            *vis = Visibility::Hidden;
            continue;
        };
        if image.image != *h {
            image.image = h.clone();
        }
        node.width = Val::Px(pw);
        node.height = Val::Px(ph);
        node.left = Val::Px((cw - pw) / 2.0);
        node.top = Val::Px((ch - ph) / 2.0);
        *vis = Visibility::Visible;
    }
}

/// One digit of the prize on an orb passing on a moving reel strip.
#[derive(Component)]
pub struct StripDigit {
    reel: u8,
    slot: u8,
    index: u8,
}

/// The prize lettered on an orb that passes by while the reel spins: the
/// orbs that land carry their real prize; the others carry prizes in the
/// game's range, as the machine shows them on the moving strip.
fn passing_face(reel: u8, n: i64) -> &'static str {
    const FACES: [&str; 20] = [
        "x1", "x1", "x1", "x1", "x2", "x2", "x2", "x3", "x3", "x4", "x5", "x5", "x10", "x15",
        "x20", "x50", "x100", "Mini", "Mini", "Minor",
    ];
    let h = (n as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(17)
        ^ u64::from(reel).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    FACES[(h % FACES.len() as u64) as usize]
}

/// The tier plate image of a coin face ("Mini" and so on), if it has one.
fn tier_image(face: &str) -> Option<&'static str> {
    match face {
        "Mini" => Some("orb_mini"),
        "Minor" => Some("orb_minor"),
        "Major" => Some("orb_major"),
        _ => None,
    }
}

/// The face's prize in credits, for a bet-multiple face ("x5").
fn face_credits(face: &str, total_bet: u64) -> Option<u64> {
    face.strip_prefix('x')
        .and_then(|m| m.parse::<u64>().ok())
        .map(|m| m.saturating_mul(total_bet))
}

/// The face of the strip entry `raw` at strip index `n`: a landing orb's
/// drawn prize, or a passing orb's cosmetic one.
fn strip_face(raw: &str, reel: u8, n: i64) -> Option<&str> {
    match raw.strip_prefix("ORB\n") {
        Some(f) => Some(f),
        None if raw == "orb" => Some(passing_face(reel, n)),
        None => None,
    }
}

/// The tier plate on an orb passing on a moving reel strip.
#[derive(Component)]
pub struct StripTier {
    reel: u8,
    slot: u8,
}

/// Put the tier plates on the moving strips' tier coins.
pub fn strip_tiers(
    fidelity: Res<Fidelity>,
    art: Res<ArtPack>,
    reels: Res<Reels>,
    mut plates: Query<(&StripTier, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    for (p, mut image, mut vis) in &mut plates {
        let plate = reels
            .moving(p.reel)
            .then(|| {
                let (n, raw) = reels.slot_entry(p.reel as usize, p.slot);
                strip_face(raw, p.reel, n).and_then(tier_image)
            })
            .flatten()
            .and_then(|k| art.ui.get(k));
        match plate {
            Some(h) => {
                if image.image != *h {
                    image.image = h.clone();
                }
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// Letter the orbs on the moving strips.
#[allow(clippy::type_complexity)]
pub fn strip_coins(
    fidelity: Res<Fidelity>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    art: Res<ArtPack>,
    reels: Res<Reels>,
    mut digits: Query<(&StripDigit, &mut ImageNode, &mut Visibility, &mut Node)>,
) {
    if !fidelity.0 {
        return;
    }
    let total_bet = sim.game.total_bet();
    let cents = denom.cents();
    let digit_h = cell_size().1 * COIN_DIGIT_SHARE;
    for (d, mut image, mut vis, mut node) in &mut digits {
        let text = reels.moving(d.reel).then(|| {
            let (n, raw) = reels.slot_entry(d.reel as usize, d.slot);
            strip_face(raw, d.reel, n)
                .and_then(|f| face_credits(f, total_bet))
                .map(|c| coin_text(c, cents))
        });
        let glyph = text
            .flatten()
            .as_deref()
            .and_then(|t| t.chars().nth(d.index as usize))
            .and_then(|c| {
                let key = format!("coin_{c}");
                art.ui.get(&key).cloned().zip(art.sizes.get(&key).copied())
            });
        match glyph {
            Some((handle, (w, h))) => {
                if image.image != handle {
                    image.image = handle;
                }
                let scale = digit_h / 73.0;
                node.width = Val::Px(w * scale);
                node.height = Val::Px(h * scale);
                node.display = Display::Flex;
                *vis = Visibility::Inherited;
            }
            None => {
                node.display = Display::None;
                *vis = Visibility::Hidden;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/// What the cabinet is waiting for the player to press.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Await {
    /// START FEATURE, after a feature is won (footage: "PRESS START FEATURE
    /// BUTTON" over the reels until it is pressed).
    Start(Feature),
    /// SPIN, before each Hold & Spin respin ("Press SPIN button").
    Respin,
}

/// Which feature screen is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Stage {
    #[default]
    Base,
    FreeGames {
        number: u32,
        total: u32,
    },
    FreeGamesDone,
    HoldSpin {
        respins_left: u8,
    },
    HoldSpinDone,
}

/// The feature presentation: what is showing and what is awaited.
#[derive(Resource, Default)]
pub struct FeatureShow {
    pub awaiting: Option<Await>,
    pub stage: Stage,
    pub games_won: u8,
    /// The scatter pay of the game that won the free games, in credits.
    pub scatter_award: u64,
    /// Seconds spent waiting for the current press.
    waited: f32,
    /// A respin to animate: the positions orbs just landed on.
    respin_landed: Option<Vec<(u8, u8)>>,
    respin: Option<RespinAnim>,
    /// Jackpots the feature awarded, in award order.
    jackpots: Vec<(JackpotTier, u64)>,
    /// The count after the feature: about to start, running, finished.
    collect_start: bool,
    collect: Option<Collect>,
    pub collected: Option<u64>,
}

impl FeatureShow {
    /// Follow an event as it is presented.
    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::SpinStarted { .. } => {
                self.stage = Stage::Base;
                self.scatter_award = 0;
                self.collected = None;
            }
            Event::JackpotAwarded { tier, award } => self.jackpots.push((*tier, *award)),
            Event::ScatterWin { award, .. } => self.scatter_award = *award,
            Event::FreeGamesStarted { games } => {
                self.games_won = *games;
                self.wait_for(Await::Start(Feature::FreeGames));
            }
            Event::FreeGameStarted { number, remaining } => {
                self.stage = Stage::FreeGames {
                    number: *number,
                    total: number + remaining,
                };
            }
            Event::FreeGamesRetriggered { remaining, .. } => {
                if let Stage::FreeGames { number, total } = &mut self.stage {
                    *total = *number + remaining;
                }
            }
            Event::FreeGamesCompleted { .. } => self.stage = Stage::FreeGamesDone,
            Event::HoldAndSpinStarted { respins, .. } => {
                self.jackpots.clear();
                self.stage = Stage::HoldSpin {
                    respins_left: *respins,
                };
                self.wait_for(Await::Start(Feature::HoldAndSpin));
            }
            Event::HoldAndSpinRespin {
                respins_left,
                landed,
                ..
            } => {
                self.respin_landed = Some(landed.iter().map(|o| (o.reel, o.row)).collect());
                self.stage = Stage::HoldSpin {
                    respins_left: *respins_left,
                };
                if *respins_left > 0 {
                    self.wait_for(Await::Respin);
                }
            }
            Event::HoldAndSpinCompleted { .. } => {
                self.awaiting = None;
                self.stage = Stage::HoldSpinDone;
                self.collect_start = true;
            }
            _ => {}
        }
    }

    fn wait_for(&mut self, what: Await) {
        self.awaiting = Some(what);
        self.waited = 0.0;
    }

    /// A feature is under way: the credit meter holds still and its wins
    /// collect on the win meter, paid to credit when the feature completes
    /// (footage: CREDIT unchanged through the free games, then raised by the
    /// whole win after "FEATURE COMPLETED").
    pub fn in_feature(&self) -> bool {
        matches!(self.awaiting, Some(Await::Start(_)))
            || matches!(self.stage, Stage::FreeGames { .. } | Stage::HoldSpin { .. })
            || self.collect_start
            || self.collect.is_some()
    }

    /// The Hold & Spin grid is showing.
    pub fn hold_and_spin(&self) -> bool {
        matches!(self.stage, Stage::HoldSpin { .. } | Stage::HoldSpinDone)
    }

    /// The line under the reels, in place of "Bet Multiplier".
    pub fn caption(&self, t: f32) -> Option<String> {
        let alternate = ((t / 1.5) as u32).is_multiple_of(2);
        match (self.awaiting, self.stage) {
            (Some(Await::Start(Feature::FreeGames)), _) if alternate => {
                Some(format!("{} FREE GAMES WON!", self.games_won))
            }
            (Some(Await::Start(_)), _) => Some("Press START FEATURE button".into()),
            (Some(Await::Respin), _) => Some("Press SPIN button".into()),
            (_, Stage::FreeGames { number, total }) => {
                Some(format!("Free Game {number} of {total}"))
            }
            _ => None,
        }
    }

    /// The message at the right, in place of the start prompts.
    pub fn message(&self) -> Option<String> {
        let free_games = matches!(self.awaiting, Some(Await::Start(Feature::FreeGames)))
            || matches!(self.stage, Stage::FreeGames { .. });
        (free_games && self.scatter_award > 0)
            .then(|| format!("Scatter wins {}", self.scatter_award))
    }
}

/// A SPIN press while a feature waits releases it (and starts no game). An
/// unattended run presses by itself after a moment.
#[allow(clippy::too_many_arguments)]
pub fn feature_press(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    smoke: Res<Smoke>,
    mut input: ResMut<CabinetInput>,
    mut show: ResMut<FeatureShow>,
    mut presentation: ResMut<Presentation>,
) {
    if !fidelity.0 || show.awaiting.is_none() || show.blocks() {
        return;
    }
    show.waited += smoke.dt(&time);
    let pressed = (input.spin && !input.skip_spin)
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter);
    let auto = smoke.frames_left.is_some() && show.waited > 2.5;
    if pressed || auto {
        show.awaiting = None;
        input.skip_spin = true;
        // The press plays the next step at once.
        presentation.step_timer = 0.0;
    }
}

/// The feature banner across the top of the reels.
#[derive(Component)]
pub struct FeatureBanner;
/// The text of the feature banner.
#[derive(Component)]
pub struct FeatureBannerText;
/// A digit of the "FREE GAME n OF m" counter: which number, which digit.
#[derive(Component)]
pub struct CounterDigit {
    number: u8,
    index: u8,
}
/// The dark cell behind an empty Hold & Spin position.
#[derive(Component)]
pub struct HsCell {
    reel: u8,
    row: u8,
}
/// The Hold & Spin grid lines over the reels.
#[derive(Component)]
pub struct HsFrame;
/// The message at the right of the meter band during a feature.
#[derive(Component)]
pub struct FeatureMessage;

const BANNER_SIZE: (f32, f32) = (760.0, 96.0);
const BANNER_Y: f32 = GRID_CENTER.1 - 1.5 * PITCH.1 + 6.0;
/// Where the counter's two numbers sit across the banner, and their height.
const COUNTER_X: [f32; 2] = [0.59, 0.795];
const COUNTER_DIGIT_H: f32 = 52.0;

/// The banner, its counter digits, the Hold & Spin grid lines.
pub fn spawn_feature_layer(c: &mut ChildSpawnerCommands, art: &ArtPack) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    let (gx, gy) = GRID_CENTER;
    c.spawn((
        HsFrame,
        sprite(ui("hs_frame"), gx, gy, 5.0 * PITCH.0, 3.0 * PITCH.1),
        Visibility::Hidden,
    ));
    let (bw, bh) = BANNER_SIZE;
    c.spawn((
        FeatureBanner,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(gx - bw / 2.0),
            top: Val::Px(BANNER_Y - bh / 2.0),
            width: Val::Px(bw),
            height: Val::Px(bh),
            ..default()
        },
        ImageNode {
            image: ui("banner_base"),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(5),
        Visibility::Hidden,
    ))
    .with_children(|b| {
        b.spawn((
            FeatureBannerText,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(bw * 0.04),
                top: Val::Px(bh * 0.02),
                width: Val::Px(bw * 0.92),
                height: Val::Px(bh * 0.96),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        for number in 0..2u8 {
            b.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(bw * COUNTER_X[number as usize] - 60.0),
                top: Val::Px(0.0),
                width: Val::Px(120.0),
                height: Val::Px(bh),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                for index in 0..2u8 {
                    row.spawn((
                        CounterDigit { number, index },
                        Node::default(),
                        ImageNode {
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                        Visibility::Hidden,
                    ));
                }
            });
        }
    });
}

/// The dark panel an empty Hold & Spin position shows.
pub fn spawn_hs_cell(cell: &mut ChildSpawnerCommands, art: &ArtPack, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    cell.spawn((
        HsCell { reel, row },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(cw),
            height: Val::Px(ch),
            ..default()
        },
        ImageNode {
            image: art.ui.get("hs_cell").cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Stretch,
            color: Color::srgba(1.0, 1.0, 1.0, 0.86),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Show the feature banner, counter, grid and messages for the stage.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn feature_screen(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    art: Res<ArtPack>,
    labels: Query<(&CellLabel, &Text), Without<FeatureMessage>>,
    mut banner: Query<&mut Visibility, (With<FeatureBanner>, Without<HsFrame>, Without<HsCell>)>,
    mut banner_text: Query<
        &mut ImageNode,
        (
            With<FeatureBannerText>,
            Without<CounterDigit>,
            Without<HsCell>,
        ),
    >,
    mut digits: Query<
        (&CounterDigit, &mut ImageNode, &mut Visibility, &mut Node),
        (
            Without<FeatureBanner>,
            Without<FeatureBannerText>,
            Without<HsCell>,
            Without<HsFrame>,
        ),
    >,
    mut frame: Query<
        &mut Visibility,
        (
            With<HsFrame>,
            Without<FeatureBanner>,
            Without<CounterDigit>,
            Without<HsCell>,
        ),
    >,
    mut cells: Query<
        (&HsCell, &mut Visibility),
        (
            Without<FeatureBanner>,
            Without<HsFrame>,
            Without<CounterDigit>,
        ),
    >,
    mut message: Query<&mut Text, (With<FeatureMessage>, Without<CellLabel>)>,
) {
    if !fidelity.0 {
        return;
    }
    let text = match (show.awaiting, show.stage) {
        (Some(Await::Start(_)), _) => Some("banner_start_feature".to_string()),
        (_, Stage::FreeGames { .. }) => Some("banner_free_game".to_string()),
        (_, Stage::FreeGamesDone) => Some("banner_feature_completed".to_string()),
        (_, Stage::HoldSpin { respins_left }) => {
            Some(format!("banner_spins_{}", respins_left.clamp(1, 3)))
        }
        (_, Stage::HoldSpinDone) => Some("banner_spins_completed".to_string()),
        _ => None,
    };
    let handle = text.as_deref().and_then(|k| art.ui.get(k));
    for mut vis in &mut banner {
        let want = if handle.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    if let Some(h) = handle {
        for mut image in &mut banner_text {
            if image.image != *h {
                image.image = h.clone();
            }
        }
    }
    // The counter's numbers, in the banner's own gold digits.
    let counter = match (show.awaiting, show.stage) {
        (None | Some(Await::Respin), Stage::FreeGames { number, total }) => {
            Some([number.to_string(), total.to_string()])
        }
        _ => None,
    };
    for (d, mut image, mut vis, mut node) in &mut digits {
        let glyph = counter
            .as_ref()
            .and_then(|n| n[d.number as usize].chars().nth(d.index as usize))
            .and_then(|c| {
                let key = format!("bdigit_{c}");
                art.ui.get(&key).zip(art.sizes.get(&key))
            });
        match glyph {
            Some((h, (w, gh))) => {
                if image.image != *h {
                    image.image = h.clone();
                }
                let k = COUNTER_DIGIT_H / gh;
                node.width = Val::Px(w * k);
                node.height = Val::Px(gh * k);
                node.display = Display::Flex;
                *vis = Visibility::Inherited;
            }
            None => {
                node.display = Display::None;
                *vis = Visibility::Hidden;
            }
        }
    }
    // The Hold & Spin grid: lines over the reels, dark empty positions.
    let hs = show.hold_and_spin();
    for mut vis in &mut frame {
        let want = if hs {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (cell, mut vis) in &mut cells {
        let empty = labels
            .iter()
            .any(|(l, t)| l.reel == cell.reel && l.row == cell.row && t.0.is_empty());
        let want = if hs && empty {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    let msg = show.message().unwrap_or_default();
    for mut t in &mut message {
        if t.0 != msg {
            t.0 = msg.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Hold & Spin respins and collection
// ---------------------------------------------------------------------------

/// A respin in motion: each open position spins, then stops in turn.
#[derive(Clone, Debug, Default)]
pub struct RespinAnim {
    t: f32,
    /// Positions spinning, with the time each stops and whether an orb lands.
    stops: Vec<(u8, u8, f32, bool)>,
    /// Stops already sounded.
    sounded: usize,
}

/// Coins being counted into the win after the feature (footage: each coin
/// in turn, with the Hold & Spin "doonk").
#[derive(Clone, Debug, Default)]
pub struct Collect {
    t: f32,
    /// Coin amounts in counting order.
    amounts: Vec<u64>,
    counted: usize,
    /// The total so far, shown on the win meter.
    pub shown: u64,
}

/// Seconds a respin spins before its first position stops, and between
/// stops.
const RESPIN_FIRST_STOP: f32 = 0.55;
const RESPIN_STOP_GAP: f32 = 0.09;
/// Seconds per coin counted, and the hold after the last.
const COLLECT_STEP: f32 = 0.45;
const COLLECT_HOLD: f32 = 1.2;
/// Symbols per second the open positions scroll at during a respin.
const RESPIN_SPEED: f32 = 9.0;

/// One open position's spinning strip.
#[derive(Component)]
pub struct HsSpin {
    reel: u8,
    row: u8,
}
/// An image on an open position's spinning strip.
#[derive(Component)]
pub struct HsSpinImage {
    index: u8,
}

/// The strip an open Hold & Spin position shows while it respins.
pub fn spawn_hs_spin(cell: &mut ChildSpawnerCommands, art: &ArtPack, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    let orb = art.symbols.get("orb").cloned().unwrap_or_default();
    let (ow, oh) = art
        .sizes
        .get("orb")
        .map_or((cw, ch), |s| symbol_size("orb", *s));
    cell.spawn((
        HsSpin { reel, row },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(cw),
            height: Val::Px(ch),
            overflow: Overflow::clip(),
            ..default()
        },
        ImageNode {
            image: art.ui.get("hs_cell").cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Visibility::Hidden,
    ))
    .with_children(|s| {
        for index in 0..2u8 {
            s.spawn((
                HsSpinImage { index },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px((cw - ow * 0.86) / 2.0),
                    width: Val::Px(ow * 0.86),
                    height: Val::Px(oh * 0.86),
                    ..default()
                },
                ImageNode {
                    image: orb.clone(),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
        }
    });
}

impl FeatureShow {
    /// The presentation holds its next event while a respin spins or coins
    /// are counted.
    pub fn blocks(&self) -> bool {
        self.respin_landed.is_some()
            || self.respin.is_some()
            || self.collect_start
            || self.collect.is_some()
    }
}

/// Run the respin spins and the coin count.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn hold_and_spin_motion(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    mut show: ResMut<FeatureShow>,
    sounds: Res<Sounds>,
    mut commands: Commands,
    labels: Query<(&CellLabel, &Text)>,
    mut spins: Query<(&HsSpin, &mut Visibility, &Children)>,
    mut images: Query<(&HsSpinImage, &mut Node)>,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time).min(0.05);
    let label = |reel: u8, row: u8| -> String {
        labels
            .iter()
            .find(|(l, _)| l.reel == reel && l.row == row)
            .map(|(_, t)| t.0.clone())
            .unwrap_or_default()
    };

    // A respin was presented: every open position, and every position an
    // orb just landed on, spins and stops in turn.
    if let Some(landed) = show.respin_landed.take() {
        let mut stops = Vec::new();
        let (reels, rows) = (sim.game.model().grid.reels, sim.game.model().grid.rows);
        for reel in 0..reels {
            for row in 0..rows {
                let new = landed.contains(&(reel, row));
                if new || label(reel, row).is_empty() {
                    let k = stops.len() as f32;
                    stops.push((reel, row, RESPIN_FIRST_STOP + k * RESPIN_STOP_GAP, new));
                }
            }
        }
        sounds.play(
            &mut commands,
            &format!("betbeep{}_reduced", 1 + stops.len() % 3),
        );
        show.respin = Some(RespinAnim { stops, ..default() });
    }
    if let Some(anim) = show.respin.as_mut() {
        anim.t += dt;
        while let Some(&(_, _, at, orb)) = anim.stops.get(anim.sounded) {
            if anim.t < at {
                break;
            }
            let clip = if orb {
                format!("SND_HandSDoonk{}", 1 + anim.sounded % 5)
            } else {
                format!("reel_clink{}", 1 + anim.sounded % 5)
            };
            sounds.play(&mut commands, &clip);
            anim.sounded += 1;
        }
        if anim.sounded >= anim.stops.len() {
            show.respin = None;
        }
    }
    let anim = show.respin.clone();
    let p = cell_size().1;
    for (spin, mut vis, children) in &mut spins {
        let running = anim.as_ref().and_then(|a| {
            a.stops
                .iter()
                .find(|s| s.0 == spin.reel && s.1 == spin.row)
                .filter(|s| a.t < s.2)
                .map(|_| a.t)
        });
        let want = if running.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if let Some(t) = running {
            // Orbs pass down the position, one every two symbols.
            let offset = (t * RESPIN_SPEED + spin.reel as f32 * 0.37 + spin.row as f32 * 0.61)
                .rem_euclid(2.0);
            for child in children.iter() {
                if let Ok((img, mut node)) = images.get_mut(child) {
                    let h = match node.height {
                        Val::Px(h) => h,
                        _ => p,
                    };
                    let y = (offset - 1.0 - 2.0 * img.index as f32) * p + (p - h) / 2.0;
                    node.top = Val::Px(y);
                }
            }
        }
    }

    // The count after the feature.
    let total_bet = sim.game.total_bet();
    if show.collect_start {
        show.collect_start = false;
        let mut amounts = Vec::new();
        let mut jackpots = show.jackpots.clone();
        let (reels, rows) = (sim.game.model().grid.reels, sim.game.model().grid.rows);
        for reel in 0..reels {
            for row in 0..rows {
                let l = label(reel, row);
                let Some(v) = l.strip_prefix("ORB\n") else {
                    continue;
                };
                let amount = match v.strip_prefix('x').and_then(|m| m.parse::<u64>().ok()) {
                    Some(m) => m * total_bet,
                    None => jackpots
                        .iter()
                        .position(|(tier, _)| format!("{tier:?}") == v)
                        .map(|i| jackpots.remove(i).1)
                        .unwrap_or(0),
                };
                amounts.push(amount);
            }
        }
        show.collect = Some(Collect {
            amounts,
            ..default()
        });
    }
    let mut done = false;
    if let Some(c) = show.collect.as_mut() {
        c.t += dt;
        while c.counted < c.amounts.len() && c.t >= (c.counted as f32 + 1.0) * COLLECT_STEP {
            c.shown += c.amounts[c.counted];
            sounds.play(
                &mut commands,
                &format!("SND_HandSDoonk{}", 1 + c.counted % 5),
            );
            c.counted += 1;
        }
        done = c.counted >= c.amounts.len()
            && c.t >= c.amounts.len() as f32 * COLLECT_STEP + COLLECT_HOLD;
    }
    if done {
        show.collected = show.collect.take().map(|c| c.shown);
    }
}
