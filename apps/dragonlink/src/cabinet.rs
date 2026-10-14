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

use dragonlink_core::{Feature, GamblePick};

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
/// "Bet Multiplier", the start prompt's top, the dark meter band's top and
/// the centre line of its meters (cabinet photo, scaled to this screen).
const MULTIPLIER_Y: f32 = REEL_BOTTOM + 44.0;
const PROMPT_Y: f32 = REEL_BOTTOM + 84.0;
const BAND_TOP: f32 = REEL_BOTTOM + 96.0;
const METER_Y: f32 = REEL_BOTTOM + 176.0;
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
pub const REQUIRED_UI: [&str; 23] = [
    "deck_panel",
    "key_blue",
    "key_red",
    "key_white",
    "key_gold",
    "key_dark",
    "spin_lit",
    "spin_dim",
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
        // Testing (`DRAGONLINK_ONTOP`): stay in front, for screen captures.
        window_level: if std::env::var_os("DRAGONLINK_ONTOP").is_some() {
            bevy::window::WindowLevel::AlwaysOnTop
        } else {
            bevy::window::WindowLevel::Normal
        },
        ..default()
    }
}

/// The cabinet screen's faces: the condensed meter face and the bold sans of
/// its small captions.
pub struct CabFonts {
    pub meter: Handle<Font>,
    pub sans: Handle<Font>,
    /// A face with the card suits.
    pub symbols: Handle<Font>,
    /// Bold italic, for the gamble screen.
    pub italic: Handle<Font>,
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
        symbols: first(&[
            "/System/Library/Fonts/Apple Symbols.ttf",
            "C:/Windows/Fonts/seguisym.ttf",
        ]),
        italic: first(&[
            "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
            "C:/Windows/Fonts/arialbi.ttf",
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

/// The height scale the coin's image is drawn at; what sits on the coin (its
/// tier plate) is drawn at this scale, at its own proportions, so it stays
/// inside the coin.
pub fn orb_scale(_src: (f32, f32)) -> f32 {
    SYMBOL_SCALE.1 * ORB_SCALE.1
}

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
            // Hold & Spin: the whole grid goes dark slate (footage); the
            // coins sit on it, open positions show their symbol faintly.
            c.spawn((
                HsBackdrop,
                sprite(ui("white"), gx, gy, 5.0 * PITCH.0, 3.0 * PITCH.1),
                Visibility::Hidden,
            ));
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
                        spawn_door_fx(cell, art, reel, row);
                        spawn_symbol_fx(cell, reel, row);
                    });
                }
            }

            // The moving reels, drawn over the landed cells while they spin.
            spawn_strips(c, model.grid.reels);
            spawn_wild_talls(c, model.grid.reels);
            spawn_feature_layer(c, art, &fonts.meter);
            spawn_hs_overlays(c, &fonts.meter);
            spawn_win_lines(c, &fonts.sans);

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

            spawn_deck(c, art, &fonts.sans, &fonts.symbols);
            spawn_gamble_screen(c, art, fonts);

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

/// The screen below the reels, laid out as the cabinet lays it out: "Bet
/// Multiplier" under the reels; one dark meter band with the information
/// button, GAME RULES, the lines box, CREDIT, BET and WIN (its value in a
/// box of its own, the start prompt above it); GAMES MENU over the
/// denomination box at the right.
fn spawn_meter_band(
    c: &mut ChildSpawnerCommands,
    fonts: &CabFonts,
    art: &ArtPack,
    gold: Color,
    pale: Color,
) {
    let font = &fonts.meter;
    let white = Color::srgb(0.97, 0.97, 0.97);
    let dark = Color::srgba(0.015, 0.01, 0.03, 0.9);
    let inset = Color::srgba(0.0, 0.0, 0.0, 0.55);
    let rim = Color::srgb(0.3, 0.3, 0.36);
    let rect = |l: f32, t: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(l),
        top: Val::Px(t),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    };
    // The band, across the whole screen.
    c.spawn((
        rect(0.0, BAND_TOP, CANVAS_W, SCREEN_BOTTOM - BAND_TOP),
        BackgroundColor(dark),
    ));

    // "Bet Multiplier 5", centred under the reels.
    c.spawn((
        CabMeter::Multiplier,
        label(
            "Bet Multiplier 1",
            &fonts.sans,
            27.0,
            white,
            GRID_CENTER.0,
            MULTIPLIER_Y,
            700.0,
        ),
    ));

    // The start prompts, at the band's top edge above the WIN box.
    for (which, count, key, text) in [
        (0u8, "3", "pennant", "Start FREE GAMES"),
        (1u8, "6", "orb", "Start HOLD & SPIN FEATURE"),
    ] {
        c.spawn((
            StartPrompt { which },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1180.0),
                top: Val::Px(PROMPT_Y),
                width: Val::Px(540.0),
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
                        font_size: FontSize::Px(27.0),
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
            font_size: FontSize::Px(27.0),
            ..default()
        },
        TextColor(white),
        TextLayout::justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(1180.0),
            top: Val::Px(PROMPT_Y + 2.0),
            width: Val::Px(540.0),
            ..default()
        },
    ));

    // The menu key at the far left: a yellow key with its three bars.
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(18.0)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(9.0),
            ..rect(16.0, METER_Y - 56.0, 54.0, 112.0)
        },
        BackgroundColor(Color::srgb(0.98, 0.8, 0.14)),
        BorderColor::all(Color::srgb(0.75, 0.45, 0.05)),
    ))
    .with_children(|b| {
        for w in [14.0, 22.0, 28.0] {
            b.spawn((
                Node {
                    width: Val::Px(w),
                    height: Val::Px(5.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.35, 0.22, 0.04)),
            ));
        }
    });

    // CREDIT / BET / WIN, their values, WIN's value in its own box.
    for (text, x, w) in [
        ("CREDIT", 300.0, 220.0),
        ("BET", 850.0, 120.0),
        ("WIN", 1205.0, 120.0),
    ] {
        c.spawn(label(text, font, 66.0, gold, x, METER_Y, w));
    }
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..rect(1280.0, METER_Y - 46.0, 440.0, 92.0)
        },
        BackgroundColor(inset),
        BorderColor::all(rim),
    ));
    for (meter, x, w) in [
        (CabMeter::Credit, 590.0, 360.0),
        (CabMeter::Bet, 1030.0, 240.0),
        (CabMeter::Win, 1500.0, 420.0),
    ] {
        c.spawn((meter, label("-", font, 70.0, pale, x, METER_Y, w)));
    }

    // GAMES MENU over the denomination box at the right.
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..rect(1745.0, METER_Y - 56.0, 166.0, 120.0)
        },
        BackgroundColor(Color::srgba(0.07, 0.06, 0.06, 0.95)),
        BorderColor::all(Color::srgb(0.45, 0.42, 0.36)),
    ));
    pill(
        c,
        &fonts.sans,
        "GAMES MENU",
        1828.0,
        METER_Y - 70.0,
        166.0,
        Color::srgb(0.99, 0.82, 0.25),
        Color::srgb(0.24, 0.14, 0.04),
    );
    c.spawn((
        CabMeter::Denom,
        label("1¢", font, 78.0, gold, 1828.0, METER_Y - 2.0, 166.0),
    ));
    c.spawn((
        CabMeter::MinBet,
        label(
            "MIN BET 50",
            &fonts.sans,
            18.0,
            white,
            1828.0,
            METER_Y + 46.0,
            166.0,
        ),
    ));
}

/// Alternate the two start prompts every few seconds.
pub fn start_prompts(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    mut prompts: Query<(&StartPrompt, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    let shown = ((time.elapsed_secs() / 3.0) as u32 % 2) as u8;
    let quiet = show.message().is_some() || show.stage != Stage::Base || gamble.active;
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

/// The button deck below the screen, as on the cabinet: a brushed-silver
/// panel with a blue light strip, two rows of six square backlit keys in
/// chrome bezels (GAMBLE over RESPIN, then the denominations over the line
/// counts) and the blue teardrop play button. While gambling the keys take
/// the gamble's choices.
fn spawn_deck(
    c: &mut ChildSpawnerCommands,
    art: &ArtPack,
    font: &Handle<Font>,
    symbols: &Handle<Font>,
) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
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
            ..default()
        },
        ImageNode {
            image: ui("deck_panel"),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    ));
    let (kw, kh, gx, gy) = (124.0, 64.0, 18.0, 10.0);
    let cluster = 6.0 * kw + 5.0 * gx;
    let (sw, sh) = (226.0, 174.0);
    let left = (CANVAS_W - (cluster + 70.0 + sw)) / 2.0;
    let top = DECK_TOP + 18.0;
    for row in 0..2u8 {
        for col in 0..6u8 {
            deck_button(
                c,
                font,
                symbols,
                CabButton::Slot { row, col },
                left + f32::from(col) * (kw + gx),
                top + f32::from(row) * (kh + gy),
                kw,
                kh,
            );
        }
    }
    deck_button(
        c,
        font,
        symbols,
        CabButton::Spin,
        left + cluster + 70.0,
        DECK_TOP + 2.0,
        sw,
        sh,
    );
}

/// What a deck button does now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Gamble,
    Respin,
    Denom(usize),
    Lines(u16),
    TakeWin,
    Pick(GamblePick),
    Off,
}

/// A deck button's role: the base game's, or the gamble's while gambling.
pub fn role(button: CabButton, gambling: bool) -> Role {
    match (button, gambling) {
        (CabButton::Spin, false) => Role::Respin,
        (CabButton::Spin, true) => Role::TakeWin,
        (CabButton::Slot { row: 0, col: 0 }, false) => Role::Gamble,
        (CabButton::Slot { row: 1, col: 0 }, false) => Role::Respin,
        (CabButton::Slot { row: 0, col }, false) => Role::Denom(col as usize - 1),
        (CabButton::Slot { row: _, col }, false) => Role::Lines(DECK_LINES[col as usize - 1]),
        (CabButton::Slot { row: 0, col: 0 }, true) => Role::TakeWin,
        (CabButton::Slot { row, col }, true) => match (row, col) {
            (0, 1) => Role::Pick(GamblePick::Red),
            (0, 2) => Role::Pick(GamblePick::Hearts),
            (0, 3) => Role::Pick(GamblePick::Diamonds),
            (1, 1) => Role::Pick(GamblePick::Black),
            (1, 2) => Role::Pick(GamblePick::Clubs),
            (1, 3) => Role::Pick(GamblePick::Spades),
            _ => Role::Off,
        },
    }
}

/// A button's caption for its role.
fn role_text(role: Role) -> String {
    match role {
        Role::Gamble => "GAMBLE".into(),
        Role::Respin => "RESPIN".into(),
        Role::Denom(i) => {
            let cents = DECK_DENOMS[i];
            if cents >= 100 {
                format!("${}", cents / 100)
            } else {
                format!("{cents}¢")
            }
        }
        Role::Lines(1) => "1 LINE".into(),
        Role::Lines(n) => format!("{n} LINES"),
        Role::TakeWin => "TAKE WIN".into(),
        Role::Pick(p) => pick_text(p).into(),
        Role::Off => String::new(),
    }
}

/// Whether a gamble choice is a red one (red, hearts, diamonds).
fn is_red(p: GamblePick) -> bool {
    matches!(
        p,
        GamblePick::Red | GamblePick::Hearts | GamblePick::Diamonds
    )
}

/// A gamble choice as its button reads.
fn pick_text(p: GamblePick) -> &'static str {
    match p {
        GamblePick::Red => "RED",
        GamblePick::Black => "BLACK",
        GamblePick::Hearts => "\u{2665}",
        GamblePick::Diamonds => "\u{2666}",
        GamblePick::Clubs => "\u{2663}",
        GamblePick::Spades => "\u{2660}",
    }
}

/// Act on a button press in its current role.
fn press(role: Role, input: &mut CabinetInput) {
    match role {
        Role::Gamble => input.gamble = true,
        Role::Respin => input.spin = true,
        Role::Denom(i) => input.denom = Some(i),
        Role::Lines(n) => input.lines = Some(n),
        Role::TakeWin => input.take_win = true,
        Role::Pick(p) => input.pick = Some(p),
        Role::Off => {}
    }
}

/// The lit cap of a deck button (its image is set by [`light_deck`]).
#[derive(Component)]
pub struct DeckCap(pub CabButton);

/// A deck button's caption.
#[derive(Component)]
pub struct DeckLabel(pub CabButton);

/// The two faces a deck caption can use: text, or the suit symbols.
#[derive(Component)]
pub struct DeckFonts {
    text: Handle<Font>,
    symbols: Handle<Font>,
}

#[allow(clippy::too_many_arguments)]
fn deck_button(
    c: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    symbols: &Handle<Font>,
    button: CabButton,
    left: f32,
    top: f32,
    w: f32,
    h: f32,
) {
    // The play button's caption sits in the teardrop's wide upper part.
    let pad = if button == CabButton::Spin {
        UiRect::bottom(Val::Px(h * 0.18))
    } else {
        UiRect::all(Val::Px(4.0))
    };
    c.spawn((
        button,
        DeckCap(button),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(w),
            height: Val::Px(h),
            padding: pad,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    ))
    .with_children(|b| {
        b.spawn((
            DeckLabel(button),
            Text::new(role_text(role(button, false))),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Center),
            DeckFonts {
                text: font.clone(),
                symbols: symbols.clone(),
            },
        ));
    })
    .observe(
        move |_press: On<PointerPress>,
              mut input: ResMut<CabinetInput>,
              gamble: Res<GambleShow>| { press(role(button, gamble.active), &mut input) },
    );
}

/// Light the deck and set its captions, as the cabinet lights it: keys glow
/// blue, the chosen denomination and line count white, lines above the
/// denomination's maximum dark, GAMBLE gold while a win can be gambled;
/// while gambling the red choices glow red, the black ones blue and TAKE WIN
/// gold. The play button glows while it can be pressed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn light_deck(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    presentation: Res<Presentation>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    rollup: Res<Rollup>,
    mut caps: Query<(&DeckCap, &mut ImageNode)>,
    mut labels: Query<(
        &DeckLabel,
        &DeckFonts,
        &mut Text,
        &mut TextFont,
        &mut TextColor,
    )>,
) {
    if !fidelity.0 {
        return;
    }
    let gambling = gamble.active;
    let waiting = show.awaiting.is_some();
    let cents = denom.cents();
    let idle = sim.game.is_idle() && presentation.queue.is_empty();
    let can_gamble = idle && sim.game.can_gamble() && !rollup.active() && !waiting;
    let lines = sim.game.lines();
    let glow = 0.84 + 0.16 * (time.elapsed_secs() * 3.5).sin().abs();
    let face = |cap: CabButton| -> (&'static str, f32) {
        match (cap, role(cap, gambling)) {
            (CabButton::Spin, _) if gambling || idle || waiting => ("spin_lit", glow),
            (CabButton::Spin, _) => ("spin_dim", 1.0),
            // The top row glows red, the bottom row blue (cabinet footage).
            (_, Role::Denom(i)) if DECK_DENOMS[i] == cents => ("key_white", 1.0),
            (_, Role::Denom(_)) => ("key_red", 1.0),
            (_, Role::Lines(n)) if n > max_lines(cents) => ("key_dark", 1.0),
            (_, Role::Lines(n)) if n == lines => ("key_white", 1.0),
            (_, Role::Lines(_)) => ("key_blue", 1.0),
            (_, Role::Gamble) if can_gamble => ("key_gold", glow),
            (_, Role::Gamble) => ("key_red", 0.55),
            (_, Role::Respin) => ("key_blue", 1.0),
            (_, Role::TakeWin) => ("key_gold", glow),
            (_, Role::Pick(p)) if is_red(p) => ("key_red", 1.0),
            (_, Role::Pick(_)) => ("key_blue", 1.0),
            (_, Role::Off) => ("key_dark", 1.0),
        }
    };
    for (cap, mut image) in &mut caps {
        let (key, k) = face(cap.0);
        if let Some(h) = art.ui.get(key)
            && image.image != *h
        {
            image.image = h.clone();
        }
        let tint = Color::srgb(k, k, k);
        if image.color != tint {
            image.color = tint;
        }
    }
    for (label, fonts, mut text, mut font, mut color) in &mut labels {
        let r = role(label.0, gambling);
        let (word, size) = match (label.0, r) {
            (CabButton::Spin, _) if gambling => ("TAKE\nWIN".to_string(), 30.0),
            (CabButton::Spin, _) if matches!(show.awaiting, Some(Await::Start(_))) => {
                ("START\nFEATURE".to_string(), 26.0)
            }
            (CabButton::Spin, _) => ("SPIN".to_string(), 40.0),
            (_, Role::Pick(p)) if !matches!(p, GamblePick::Red | GamblePick::Black) => {
                (pick_text(p).to_string(), 38.0)
            }
            (_, Role::Denom(_)) => (role_text(r), 28.0),
            _ => (role_text(r), 19.0),
        };
        if text.0 != word {
            text.0 = word;
        }
        if font.font_size != FontSize::Px(size) {
            font.font_size = FontSize::Px(size);
        }
        let suit = matches!(r, Role::Pick(p) if !matches!(p, GamblePick::Red | GamblePick::Black));
        let want = if suit { &fonts.symbols } else { &fonts.text };
        let source = bevy::text::FontSource::Handle(want.clone());
        if font.font != source {
            font.font = source;
        }
        // Dark lettering on the white and gold caps, white on the others.
        let ink = match face(label.0).0 {
            "key_white" | "key_gold" => Color::srgb(0.06, 0.08, 0.2),
            "key_dark" => Color::srgb(0.45, 0.5, 0.62),
            _ => Color::WHITE,
        };
        if color.0 != ink {
            color.0 = ink;
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
    gamble: Res<GambleShow>,
    mut meters: Query<(&CabMeter, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let cents = denom.cents();
    // Credits count up with the win meter.
    let pending = rollup.to.saturating_sub(rollup.shown());
    // During a win, a feature or the gamble the meters alternate between
    // dollars and credits, as the cabinet's do.
    let busy =
        gamble.active || show.in_feature() || rollup.to > 0 || gamble.result.is_some_and(|w| w > 0);
    let in_credits = busy && ((time.elapsed_secs() / 2.5) as u32 % 2 == 1);
    let amount = |credits: u64, cents: u64| {
        if in_credits {
            grouped(credits)
        } else {
            money(credits, cents)
        }
    };
    for (meter, mut text) in &mut meters {
        let value = match meter {
            CabMeter::Credit if show.in_feature() => amount(
                presentation
                    .shown_credits
                    .saturating_sub(presentation.shown_win),
                cents,
            ),
            CabMeter::Credit => {
                let at_rest = sim.game.is_idle() && presentation.queue.is_empty();
                let held = if at_rest && (sim.game.can_gamble() || gamble.active) {
                    sim.game.state().gamble_stake
                } else {
                    0
                };
                amount(
                    presentation.shown_credits.saturating_sub(pending.max(held)),
                    cents,
                )
            }
            CabMeter::Bet => amount(sim.game.total_bet(), cents),
            CabMeter::Win if gamble.active => amount(gamble.stake, cents),
            CabMeter::Win => match gamble.result {
                Some(w) => amount(w, cents),
                None => amount(rollup.shown(), cents),
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
    /// The play button.
    Spin,
    /// One of the deck's two rows of six.
    Slot { row: u8, col: u8 },
}

/// Button presses this frame, read by `input` alongside the keyboard.
#[derive(Resource, Default)]
pub struct CabinetInput {
    pub spin: bool,
    pub denom: Option<usize>,
    pub lines: Option<u16>,
    pub gamble: bool,
    pub take_win: bool,
    pub pick: Option<GamblePick>,
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
            // Faint copies above and below each symbol: the smear of a reel
            // at full speed (cabinet footage), drawn behind the symbols.
            for index in 0..STRIP_SLOTS as u8 {
                for offset in BLUR_OFFSETS {
                    col.spawn((
                        StripGhost {
                            reel,
                            index,
                            offset,
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        ImageNode {
                            image_mode: NodeImageMode::Stretch,
                            color: Color::srgba(1.0, 1.0, 1.0, BLUR_ALPHA),
                            ..default()
                        },
                        Visibility::Hidden,
                    ));
                }
            }
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
    mut ghosts: Query<
        (&StripGhost, &mut ImageNode, &mut Node, &mut Visibility),
        (Without<StripColumn>, Without<StripSlot>),
    >,
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
    for (g, mut image, mut node, mut vis) in &mut ghosts {
        let reel = g.reel as usize;
        // Only at full speed: spinning, or running on to its stop.
        let fast = matches!(
            reels.states[reel].motion,
            Motion::Spinning { .. } | Motion::Stopping { bounce: None, .. }
        );
        let want = if fast {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if !fast {
            continue;
        }
        let pos = reels.states[reel].pos;
        let (n, raw) = reels.slot_entry(reel, g.index);
        let key = symbol_of(raw).to_string();
        let cy = p / 2.0 + (n as f32 - pos + g.offset) * p;
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

/// The motion smear: copies this far (in symbols) from each symbol, and how
/// strongly they show.
const BLUR_OFFSETS: [f32; 2] = [-0.16, 0.16];
const BLUR_ALPHA: f32 = 0.38;

/// A faint copy of a moving symbol, for the motion smear.
#[derive(Component)]
pub struct StripGhost {
    reel: u8,
    index: u8,
    offset: f32,
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
    /// Reels of this spin that have landed coins.
    coin_reels: u32,
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

    /// Play a clip on a loop at `volume`; returns the playing entity.
    pub fn play_loop(&self, commands: &mut Commands, name: &str, volume: f32) -> Option<Entity> {
        let (clip, _) = self.clips.get(name)?;
        Some(
            commands
                .spawn((
                    AudioPlayer::new(clip.clone()),
                    PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(volume)),
                ))
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
    if moving && !sounds.reels_were_moving {
        sounds.coin_reels = 0;
    }
    sounds.reels_were_moving = moving;
    let stopped: Vec<usize> = reels.just_stopped().collect();
    for reel in stopped {
        // A reel that lands coins sounds the Hold & Spin "doonk" (footage),
        // rising with each reel of coins.
        let coins = reels.states[reel]
            .landing
            .iter()
            .any(|l| l.starts_with("ORB"));
        if reels.states[reel].landing.iter().any(|l| l == "pennant") {
            sounds.play(&mut commands, "SND_Scatter");
        }
        if coins {
            sounds.coin_reels += 1;
            let n = sounds.coin_reels.min(5);
            sounds.play(&mut commands, &format!("SND_HandSDoonk{n}"));
        } else {
            sounds.play(&mut commands, &format!("reel_clink{}", (reel + 1).min(5)));
        }
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

/// The scale for a prize's digit images: their own height to `digit_h`, or
/// smaller so the whole prize fits across the coin.
fn coin_digit_scale(text: &str, art: &ArtPack, digit_h: f32) -> f32 {
    let scale = digit_h / 73.0;
    let width: f32 = text
        .chars()
        .filter_map(|c| art.sizes.get(&coin_key(c)))
        .map(|(w, _)| w * scale)
        .sum();
    let (cw, ch) = cell_size();
    let coin_w = art
        .sizes
        .get("orb")
        .map_or(cw, |s| symbol_size("orb", *s).0)
        .min(cw.max(ch));
    let room = coin_w * COIN_TEXT_WIDTH;
    if width > room {
        scale * room / width
    } else {
        scale
    }
}

/// The widest a prize may run, as a share of the coin's width.
const COIN_TEXT_WIDTH: f32 = 0.74;

/// An orb's prize as the machine letters it: credits at cent denominations,
/// dollars ("$20") at a dollar or more.
pub fn coin_text(credits: u64, cents: u64) -> String {
    if cents >= 100 {
        format!("${}", credits * cents / 100)
    } else {
        credits.to_string()
    }
}

/// The pack image of a coin character.
fn coin_key(c: char) -> String {
    match c {
        '$' => "coin_dollar".to_string(),
        ':' => "coin_colon".to_string(),
        d => format!("coin_{d}"),
    }
}

/// The orb prize overlay of one cell: a tier label image, and a centred row of
/// digit images in the game's own coin face.
pub fn spawn_orb_overlay(cell: &mut ChildSpawnerCommands, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    // The plate and digits stand in front of the coin's fire ring.
    cell.spawn((
        OrbTier { reel, row },
        ZIndex(1),
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
    cell.spawn((
        ZIndex(1),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(cw),
            height: Val::Px(ch),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ))
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
        let scale = text
            .as_deref()
            .map_or(1.0, |t| coin_digit_scale(t, &art, digit_h));
        let glyph = text
            .as_deref()
            .and_then(|t| t.chars().nth(g.index as usize))
            .and_then(|c| {
                let key = coin_key(c);
                art.ui.get(&key).zip(art.sizes.get(&key))
            });
        match glyph {
            Some((handle, (w, h))) => {
                if image.image != *handle {
                    image.image = handle.clone();
                }
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
    // The tier plate sits on the coin at the coin's own scale.
    let (cw, ch) = cell_size();
    let k = art.sizes.get("orb").map_or(1.0, |s| orb_scale(*s));
    for (o, mut image, mut vis, mut node) in &mut tiers {
        let Some((key, h)) = face(o.reel, o.row)
            .as_deref()
            .and_then(tier_image)
            .and_then(|key| art.ui.get(key).map(|h| (key, h)))
        else {
            *vis = Visibility::Hidden;
            continue;
        };
        if image.image != *h {
            image.image = h.clone();
        }
        let (pw, ph) = art.sizes.get(key).map_or((cw, ch), |s| (s.0 * k, s.1 * k));
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
        let text = text.flatten();
        let scale = text
            .as_deref()
            .map_or(1.0, |t| coin_digit_scale(t, &art, digit_h));
        let glyph = text
            .as_deref()
            .and_then(|t| t.chars().nth(d.index as usize))
            .and_then(|c| {
                let key = coin_key(c);
                art.ui.get(&key).cloned().zip(art.sizes.get(&key).copied())
            });
        match glyph {
            Some((handle, (w, h))) => {
                if image.image != handle {
                    image.image = handle;
                }
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

/// A doors reveal under way: seconds in, the revealed symbol, the positions.
pub type Reveal = (f32, String, Vec<(u8, u8)>);

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
    /// Seconds into the Hold & Spin trigger sequence, while it plays.
    intro: Option<f32>,
    /// The Hold & Spin grid is up.
    pub grid: bool,
    /// The respin counter as shown, and the value it goes to next.
    counter: u8,
    pending_left: u8,
    /// START FEATURE has been pressed for this feature; the counter's
    /// count-up, while it runs.
    started: bool,
    count_up: Option<f32>,
    /// Flame bursts: position and start time on `clock`.
    bursts: Vec<(u8, u8, f32)>,
    /// Seconds of feature presentation, for the animations.
    clock: f32,
    transition_at: Option<f32>,
    /// The WINNER total while the coins are counted.
    pub winner: Option<u64>,
    /// Respins run so far (each starts a fresh stretch of strip).
    respins_run: i64,
    /// The whir playing under a respin.
    whir: Option<Entity>,
    /// The bolt in flight (its coin, its start on `clock`), and the last
    /// splash on the WINNER box.
    bolt: Option<(u8, u8, f32)>,
    splash_at: Option<f32>,
    /// The doors reveal under way: seconds in, the symbol, the positions.
    pub reveal: Option<Reveal>,
    /// The current free game has paid (its win shows on the banner).
    game_won: bool,
    /// The game's winning lines (line, count, award), shown in turn, the
    /// seconds they have shown, and their message.
    pub win_lines: Vec<(u16, u8, u64)>,
    win_t: f32,
    pub win_note: Option<String>,
    /// The winning line being shown (line, count), and whether a scatter
    /// win is being shown.
    win_current: Option<(u16, u8)>,
    scatter_shown: bool,
    scatter_paid: bool,
}

impl FeatureShow {
    /// Follow an event as it is presented.
    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::SpinStarted { .. } => {
                self.win_lines.clear();
                self.scatter_paid = false;
                self.stage = Stage::Base;
                self.scatter_award = 0;
                self.collected = None;
                self.grid = false;
                self.winner = None;
                self.intro = None;
            }
            Event::JackpotAwarded { tier, award } => self.jackpots.push((*tier, *award)),
            Event::ScatterWin { award, .. } if !matches!(self.stage, Stage::FreeGames { .. }) => {
                self.scatter_award = *award;
                if self.win_lines.is_empty() {
                    self.win_t = 0.0;
                }
                self.scatter_paid = true;
            }
            Event::FreeGamesStarted { games } => {
                self.games_won = *games;
                self.wait_for(Await::Start(Feature::FreeGames));
            }
            Event::MysteryRevealed { symbol, .. } => {
                self.reveal = Some((0.0, symbol.clone(), Vec::new()));
            }
            Event::LineWin {
                line, count, award, ..
            } => {
                if self.win_lines.is_empty() {
                    self.win_t = 0.0;
                }
                self.win_lines.push((*line, *count, *award));
                if matches!(self.stage, Stage::FreeGames { .. }) {
                    self.game_won = true;
                }
            }
            Event::ScatterWin { .. } if matches!(self.stage, Stage::FreeGames { .. }) => {
                self.game_won = true;
                self.scatter_paid = true;
            }
            Event::FreeGameStarted { number, remaining } => {
                self.game_won = false;
                self.win_lines.clear();
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
                // The trigger sequence plays first; it ends waiting for
                // START FEATURE.
                self.intro = Some(0.0);
                self.grid = false;
                self.counter = 0;
                self.pending_left = *respins;
                self.started = false;
                self.bursts.clear();
                self.winner = None;
                self.transition_at = None;
                self.bolt = None;
                self.splash_at = None;
            }
            Event::HoldAndSpinRespin {
                respins_left,
                landed,
                ..
            } => {
                self.respin_landed = Some(landed.iter().map(|o| (o.reel, o.row)).collect());
                self.pending_left = *respins_left;
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

    fn reset_wait(&mut self) {
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
        if let Some(note) = &self.win_note {
            return Some(note.clone());
        }
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
pub fn spawn_feature_layer(c: &mut ChildSpawnerCommands, art: &ArtPack, font: &Handle<Font>) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    let (gx, gy) = GRID_CENTER;
    c.spawn((
        HsFrame,
        // The grid art is the reel panel's own size (749 x 380): laid exactly
        // over the panel, so its border is the panel's border.
        sprite(
            ui("hs_frame"),
            gx,
            gy + 4.5 * PANEL_SCALE.1,
            1240.0 * PANEL_SCALE.0,
            629.0 * PANEL_SCALE.1,
        ),
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
        spawn_banner_amount(b, font, bw, bh);
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
        // A slate veil (a white tile, tinted), part see-through over the
        // faded symbol (footage: open positions go dark slate, the symbol
        // faintly visible through).
        ImageNode {
            image: art.ui.get("white").cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Stretch,
            color: HS_VEIL,
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
    rollup: Res<Rollup>,
    denom: Res<Denom>,
    mut amount: Query<
        (&mut Text, &mut Visibility),
        (
            With<BannerAmount>,
            Without<CellLabel>,
            Without<FeatureMessage>,
            Without<FeatureBanner>,
            Without<HsFrame>,
            Without<HsCell>,
            Without<CounterDigit>,
        ),
    >,
) {
    if !fidelity.0 {
        return;
    }
    // A free game that has paid shows "WIN $x WIN" while its win counts up.
    let game_win = (matches!(show.stage, Stage::FreeGames { .. })
        && show.game_won
        && show.reveal.is_none()
        && rollup.to > rollup.from)
        .then(|| rollup.shown().saturating_sub(rollup.from));
    for (mut text, mut vis) in &mut amount {
        let s = game_win
            .map(|w| money(w, denom.cents()))
            .unwrap_or_default();
        if text.0 != s {
            text.0 = s;
        }
        let want = if game_win.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    let text = match (show.awaiting, show.stage) {
        (_, Stage::FreeGames { .. }) if game_win.is_some() => Some("banner_win".to_string()),
        (_, Stage::HoldSpin { .. }) if show.intro.is_some() => None,
        // The WINNER total takes the top of the screen while it shows.
        _ if show.winner.is_some() => None,
        (Some(Await::Start(_)), _) => Some("banner_start_feature".to_string()),
        (_, Stage::FreeGames { .. }) => Some("banner_free_game".to_string()),
        (_, Stage::FreeGamesDone) => Some("banner_feature_completed".to_string()),
        (_, Stage::HoldSpin { .. }) if show.counter > 0 => {
            Some(format!("banner_spins_{}", show.counter.min(3)))
        }
        (_, Stage::HoldSpin { .. }) => None,
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
        (None | Some(Await::Respin), Stage::FreeGames { number, total }) if game_win.is_none() => {
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
    let hs = show.grid && show.hold_and_spin();
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
            .any(|(l, t)| l.reel == cell.reel && l.row == cell.row && open(&t.0));
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
// Hold & Spin
// ---------------------------------------------------------------------------
//
// The sequence follows cabinet footage of the feature (seconds after the
// winning game's last reel stops): the feature bell (~0.4 s); the other
// symbols dim and every coin bursts into flame with the trigger sound
// (~3.2 s); fire dragons and a fireball sweep the screen (~4.0 s) and clear
// to the Hold & Spin grid; "PRESS START FEATURE BUTTON" (~6.6 s). After the
// press the respin counter counts up 1, 2, 3, and the first respin starts.
// Held coins burn with a fire ring; open positions show dimmed symbols and
// spin as small reels of their own, stopping one by one; a coin that lands
// bursts in with the Hold & Spin "doonk". The counter drops as a respin
// starts and returns to 3 when a coin has landed. At the end each coin is
// counted in turn into a flame-bordered WINNER total.

/// Seconds into the trigger sequence for each step.
const INTRO_BELL: f32 = 0.4;
const INTRO_BURST: f32 = 3.2;
const INTRO_TRANSITION: f32 = 4.0;
const INTRO_GRID: f32 = 4.75;
const INTRO_END: f32 = 6.6;
/// The counter's count-up after START FEATURE: seconds per step, and the
/// pause before the first respin.
const COUNT_STEP: f32 = 0.14;
const COUNT_HOLD: f32 = 0.6;
/// A respin: seconds before the first open position stops, and between stops.
const RESPIN_FIRST_STOP: f32 = 0.9;
const RESPIN_STOP_GAP: f32 = 0.12;
/// Symbols per second an open position scrolls at while it respins.
const RESPIN_SPEED: f32 = 11.0;
/// The count at the end: seconds per coin, and the hold after the last.
const COLLECT_STEP: f32 = 1.2;
const COLLECT_HOLD: f32 = 2.0;
/// Frames per second of the game's animations.
const ANIM_FPS: f32 = 30.0;
/// Symbols an open position shows while it respins.
const RESPIN_SYMBOLS: [&str; 12] = [
    "9", "10", "J", "Q", "K", "vase", "chest", "women", "warriors", "emperor", "pennant", "orb",
];
/// The tint of a dimmed symbol on an open Hold & Spin position.
pub const DIM_TINT: Color = Color::srgb(0.62, 0.62, 0.68);
/// The slate under the whole Hold & Spin grid.
const HS_BACKDROP: Color = Color::srgb(0.17, 0.16, 0.23);
/// The veil over an open Hold & Spin position.
const HS_VEIL: Color = Color::srgba(0.24, 0.21, 0.30, 0.74);

/// A respin in motion: each open position spins, then stops in turn.
#[derive(Clone, Debug, Default)]
pub struct RespinAnim {
    t: f32,
    /// Where on the strips this respin starts: each respin runs a fresh
    /// stretch of strip, so a position does not keep landing on the same
    /// symbol.
    base: i64,
    /// Positions spinning, with the time each stops and whether a coin lands.
    stops: Vec<(u8, u8, f32, bool)>,
    /// Stops already made.
    done: usize,
}

/// The coins being counted into the WINNER total.
#[derive(Clone, Debug, Default)]
pub struct Collect {
    t: f32,
    /// Each coin's position and amount, in counting order.
    coins: Vec<(u8, u8, u64)>,
    /// Bolts sent, and coins added to the total.
    launched: usize,
    counted: usize,
}

/// Seconds a collection bolt takes from its coin to the WINNER box.
const BOLT_FLIGHT: f32 = 0.55;
/// The game's collection bolts (frames 464 x 288): each starts its fire ring
/// at a point of its frame and ends at the frame's top centre (measured on
/// the frames), the point the game sends every bolt to.
const BOLT_FRAME: (f32, f32) = (464.0, 288.0);
const BOLT_RINGS: [(f32, f32); 5] = [
    (75.0, 134.0),
    (74.0, 201.0),
    (269.0, 70.0),
    (309.0, 201.0),
    (153.0, 201.0),
];
const BOLT_ENDS: [f32; 5] = [228.0, 209.0, 233.0, 194.0, 268.0];

/// A bolt from a coin to the WINNER total: which animation, mirrored or not,
/// and its frame's rectangle on the canvas, stretched so the ring starts on
/// the coin and the bolt ends in the middle of the total. Of the five
/// animations (each either way round) the least stretched is used.
fn bolt_for(reel: u8, row: u8) -> (usize, bool, f32, f32, f32, f32) {
    let (cx, cy) = cell_center(reel, row);
    let (tx, ty) = (960.0, WINNER_TOP + WINNER_BOX.1 * 0.5);
    let mut best = (0, false, 1.0, 1.0, f32::MAX);
    for (k, (&(ax, ay), &ex)) in BOLT_RINGS.iter().zip(BOLT_ENDS.iter()).enumerate() {
        for flip in [false, true] {
            let (ax, ex) = if flip {
                (BOLT_FRAME.0 - ax, BOLT_FRAME.0 - ex)
            } else {
                (ax, ex)
            };
            let dx = ex - ax;
            let sy = (cy - ty) / ay;
            let sx = if (tx - cx).abs() < 1.0 {
                sy * 0.4
            } else {
                (tx - cx) / dx
            };
            if sx <= 0.0 || sy <= 0.0 {
                continue;
            }
            let distortion = (sx / sy).ln().abs();
            if distortion < best.4 {
                best = (k, flip, sx, sy, distortion);
            }
        }
    }
    let (k, flip, sx, sy, _) = best;
    let (ax, ay) = BOLT_RINGS[k];
    let ax = if flip { BOLT_FRAME.0 - ax } else { ax };
    (
        k,
        flip,
        cx - ax * sx,
        cy - ay * sy,
        BOLT_FRAME.0 * sx,
        BOLT_FRAME.1 * sy,
    )
}

/// The top of the WINNER total's flame box.
const WINNER_TOP: f32 = 42.0;
/// The WINNER total's flame box.
const WINNER_BOX: (f32, f32) = (560.0, 104.0);
/// Where the splash animation (512 x 256) draws its box: x 40..472,
/// y 145..223 (measured on its frames).
const SPLASH_BOX: (f32, f32, f32, f32) = (40.0, 145.0, 432.0, 78.0);

/// The WINNER plate's height: the jackpot bar's band, clear of the reels.
const WINNER_PLATE_H: f32 = 148.0;

/// The slate under the Hold & Spin grid.
#[derive(Component)]
pub struct HsBackdrop;

/// Show the slate under the Hold & Spin grid.
pub fn hs_backdrop(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    mut q: Query<(&mut Visibility, &mut ImageNode), With<HsBackdrop>>,
) {
    if !fidelity.0 {
        return;
    }
    let on = show.grid && show.hold_and_spin();
    for (mut v, mut image) in &mut q {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        if image.color != HS_BACKDROP {
            image.color = HS_BACKDROP;
        }
    }
}

/// The plate behind the WINNER display.
#[derive(Component)]
pub struct WinnerPlate;

/// Show the WINNER plate with the total.
pub fn winner_plate(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    mut plates: Query<&mut Visibility, With<WinnerPlate>>,
) {
    if !fidelity.0 {
        return;
    }
    for mut v in &mut plates {
        let want = if show.winner.is_some() && !gamble.active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
}

/// The collection bolt over the reels.
#[derive(Component)]
pub struct HsBolt;
/// The splash on the WINNER box as a bolt lands.
#[derive(Component)]
pub struct WinnerSplash;

/// A label that marks an open position's dimmed symbol.
const DIM: &str = "DIM:";

/// The symbol key a cell label draws, and whether it is dimmed.
pub fn label_symbol(label: &str) -> (&str, bool) {
    match label.strip_prefix(DIM) {
        Some(key) => (key, true),
        None if label.starts_with("ORB") => ("orb", false),
        None => (label, false),
    }
}

/// An open Hold & Spin position: no coin held.
fn open(label: &str) -> bool {
    !label.starts_with("ORB")
}

/// A symbol for an open position, from a stable hash.
fn respin_symbol(reel: u8, row: u8, n: i64) -> &'static str {
    let h = (n as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(23)
        ^ (u64::from(reel) * 7 + u64::from(row)).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    // Coins pass less often than other symbols.
    let i = (h % 23) as usize;
    if i >= RESPIN_SYMBOLS.len() + 10 {
        "orb"
    } else {
        RESPIN_SYMBOLS[i % (RESPIN_SYMBOLS.len() - 1)]
    }
}

impl RespinAnim {
    /// A respinning position's strip now: its scroll position, and the strip
    /// index it comes to rest on and whether a coin lands there. The scroll
    /// runs so it stops exactly on that index.
    fn strip(&self, reel: u8, row: u8) -> Option<(f32, i64, bool)> {
        let &(_, _, at, coin) = self
            .stops
            .iter()
            .find(|s| s.0 == reel && s.1 == row)
            .filter(|s| self.t < s.2)?;
        let travel = Self::travel(at);
        // Easing into the stop: the last symbols, a landing coin among
        // them, slow down and come into view before it stops.
        let u = (self.t / at).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - u) * (1.0 - u);
        Some((
            self.base as f32 + travel as f32 * eased,
            self.base + travel,
            coin,
        ))
    }

    /// Symbols a position runs through before a stop at `at` seconds.
    fn travel(at: f32) -> i64 {
        ((at * RESPIN_SPEED * 0.6).round() as i64).max(3)
    }

    /// The strip index a position stopping at `at` comes to rest on.
    fn end_of(&self, at: f32) -> i64 {
        self.base + Self::travel(at)
    }
}

/// The symbol at strip index `n` of a respinning position that comes to
/// rest at `end` (a coin there if one lands; never a coin otherwise).
fn respin_key(reel: u8, row: u8, n: i64, end: i64, coin: bool) -> &'static str {
    let key = respin_symbol(reel, row, n);
    if n == end {
        if coin {
            "orb"
        } else if key == "orb" {
            "Q"
        } else {
            key
        }
    } else if (n - end).abs() == 1 && key == "orb" {
        // No stray coin right beside the landing one.
        "K"
    } else {
        key
    }
}

/// An open position's spinning strip.
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
/// A digit of a coin passing on a respinning position.
#[derive(Component)]
pub struct HsSpinDigit {
    index: u8,
}

/// Letter the coins passing on respinning positions, as the machine does.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn hs_spin_digits(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    spins: Query<(&HsSpin, &Children)>,
    images: Query<(&HsSpinImage, &Children)>,
    labels: Query<(&CellLabel, &Text)>,
    mut digits: Query<(&HsSpinDigit, &mut ImageNode, &mut Node)>,
) {
    if !fidelity.0 {
        return;
    }
    let Some(anim) = show.respin.as_ref() else {
        return;
    };
    let total_bet = sim.game.total_bet();
    let cents = denom.cents();
    let ch = cell_size().1;
    for (spin, spin_children) in &spins {
        let Some((pos, end, coin)) = anim.strip(spin.reel, spin.row) else {
            continue;
        };
        let base = pos.floor() as i64;
        // The coin landing here carries its drawn prize.
        let landing = labels
            .iter()
            .find(|(l, _)| l.reel == spin.reel && l.row == spin.row)
            .and_then(|(_, t)| t.0.strip_prefix("ORB\n"))
            .map(str::to_string);
        for image_entity in spin_children.iter() {
            let Ok((img, children)) = images.get(image_entity) else {
                continue;
            };
            let n = base + i64::from(img.index);
            if coin && n == end {
                let text = landing
                    .as_deref()
                    .and_then(|f| face_credits(f, total_bet))
                    .map(|c| coin_text(c, cents));
                letter(&art, ch, text.as_deref(), children, &mut digits);
                continue;
            }
            let text = (respin_key(spin.reel, spin.row, n, end, coin) == "orb").then(|| {
                // Passing coins carry bet multiples, never a tier plate.
                let face = passing_face(spin.reel, n * 3 + i64::from(spin.row));
                let face = if tier_image(face).is_some() {
                    "x2"
                } else {
                    face
                };
                face_credits(face, total_bet).map(|c| coin_text(c, cents))
            });
            letter(&art, ch, text.flatten().as_deref(), children, &mut digits);
        }
    }
}

/// Letter the digit slots of a coin passing on a respinning position.
fn letter(
    art: &ArtPack,
    ch: f32,
    text: Option<&str>,
    children: &Children,
    digits: &mut Query<(&HsSpinDigit, &mut ImageNode, &mut Node)>,
) {
    let digit_h = ch * COIN_DIGIT_SHARE;
    let scale = text.map_or(1.0, |t| coin_digit_scale(t, art, digit_h));
    for child in children.iter() {
        let Ok((d, mut image, mut node)) = digits.get_mut(child) else {
            continue;
        };
        let glyph = text
            .and_then(|t| t.chars().nth(d.index as usize))
            .and_then(|c| {
                let key = coin_key(c);
                art.ui.get(&key).cloned().zip(art.sizes.get(&key).copied())
            });
        match glyph {
            Some((h, (w, gh))) => {
                if image.image != h {
                    image.image = h;
                }
                node.width = Val::Px(w * scale);
                node.height = Val::Px(gh * scale);
                node.display = Display::Flex;
            }
            None => node.display = Display::None,
        }
    }
}

/// The fire ring round a held coin.
#[derive(Component)]
pub struct HsRing {
    reel: u8,
    row: u8,
}
/// The flame burst as a coin lands or is counted.
#[derive(Component)]
pub struct HsBurst {
    reel: u8,
    row: u8,
}
/// The full-screen fire of the transition into the feature.
#[derive(Component)]
pub struct HsTransition;
/// The WINNER total and its flame border.
#[derive(Component)]
pub struct WinnerBox;
#[derive(Component)]
pub struct WinnerText;

/// The spinning strip, fire ring and flame burst of one position.
pub fn spawn_hs_spin(cell: &mut ChildSpawnerCommands, art: &ArtPack, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    let (ow, oh) = art
        .sizes
        .get("orb")
        .map_or((cw, ch), |s| symbol_size("orb", *s));
    let centred = |w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px((cw - w) / 2.0),
        top: Val::Px((ch - h) / 2.0),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    };
    cell.spawn((
        HsSpin { reel, row },
        // A respinning position covers the coin about to land on it.
        ZIndex(2),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(cw),
            height: Val::Px(ch),
            overflow: Overflow::clip(),
            ..default()
        },
        // The slate face, covering the symbol resting under it.
        ImageNode {
            image: art.ui.get("white").cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Stretch,
            color: HS_BACKDROP,
            ..default()
        },
        Visibility::Hidden,
    ))
    .with_children(|s| {
        // The veil a resting open position wears, so a respinning one
        // matches it; coins pass in front of it.
        s.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                image: art.ui.get("white").cloned().unwrap_or_default(),
                image_mode: NodeImageMode::Stretch,
                color: HS_VEIL,
                ..default()
            },
            ZIndex(1),
        ));
        for index in 0..3u8 {
            s.spawn((
                HsSpinImage { index },
                ZIndex(0),
                Node {
                    position_type: PositionType::Absolute,
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
            .with_children(|img| {
                // A passing coin's prize.
                for digit in 0..COIN_SLOTS {
                    img.spawn((
                        HsSpinDigit { index: digit },
                        Node {
                            display: Display::None,
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
    });
    cell.spawn((
        HsRing { reel, row },
        centred(ow * 1.02, oh * 1.02),
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Visibility::Hidden,
    ));
    cell.spawn((
        HsBurst { reel, row },
        centred(ow * 1.3, oh * 1.3),
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(4),
        Visibility::Hidden,
    ));
}

/// The transition fire and the WINNER box, over everything.
pub fn spawn_hs_overlays(c: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    c.spawn((
        HsTransition,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(CANVAS_W),
            height: Val::Px(SCREEN_BOTTOM),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(30),
        Visibility::Hidden,
    ));
    c.spawn((
        HsBolt,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(BOLT_FRAME.0),
            height: Val::Px(BOLT_FRAME.1),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(22),
        Visibility::Hidden,
    ));
    // The WINNER display takes the place of the jackpot meters at the top of
    // the screen while the coins are counted (the cabinet shows it on its
    // top screen): a dark plate, "WINNER", the flame-bordered total.
    c.spawn((
        WinnerPlate,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(960.0 - JACKPOT_BAR_W / 2.0),
            top: Val::Px(0.0),
            width: Val::Px(JACKPOT_BAR_W),
            height: Val::Px(WINNER_PLATE_H),
            border: UiRect::all(Val::Px(4.0)),
            border_radius: BorderRadius::bottom(Val::Px(14.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.02, 0.03)),
        BorderColor::all(Color::srgb(0.85, 0.6, 0.2)),
        GlobalZIndex(19),
        Visibility::Hidden,
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("WINNER"),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(40.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.32, 0.12)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(-4.0),
                ..default()
            },
        ));
    });
    let (w, h) = WINNER_BOX;
    let (sx, sy) = (w / SPLASH_BOX.2, h / SPLASH_BOX.3);
    c.spawn((
        WinnerSplash,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(960.0 - w / 2.0 - SPLASH_BOX.0 * sx),
            top: Val::Px(WINNER_TOP - SPLASH_BOX.1 * sy),
            width: Val::Px(512.0 * sx),
            height: Val::Px(256.0 * sy),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(21),
        Visibility::Hidden,
    ));
    c.spawn((
        WinnerBox,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(960.0 - w / 2.0),
            top: Val::Px(WINNER_TOP),
            width: Val::Px(w),
            height: Val::Px(h),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(20),
        Visibility::Hidden,
    ))
    .with_children(|b| {
        b.spawn((
            WinnerText,
            Text::new(""),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(70.0),
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.93, 0.36)),
        ));
    });
}

impl FeatureShow {
    /// Whether a coin's fire has gone to the WINNER total (its bolt has
    /// left), or the count is over.
    fn fire_taken(&self, reel: u8, row: u8) -> bool {
        match &self.collect {
            Some(c) => c.coins[..c.launched]
                .iter()
                .any(|(r, w, _)| *r == reel && *w == row),
            None => self.winner.is_some(),
        }
    }

    /// The presentation holds its next event while the trigger sequence,
    /// the counter, a respin or the count is playing.
    pub fn blocks(&self) -> bool {
        self.intro.is_some()
            || self.reveal.is_some()
            || self.count_up.is_some()
            || self.respin_landed.is_some()
            || self.respin.is_some()
            || self.collect_start
            || self.collect.is_some()
    }
}

/// Did `from..to` pass `at`?
fn crossed(from: f32, to: f32, at: f32) -> bool {
    from < at && to >= at
}

/// Run the Hold & Spin sequence: the trigger, the counter, the respins and
/// the count.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn hold_and_spin_motion(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    mut show: ResMut<FeatureShow>,
    sounds: Res<Sounds>,
    mut commands: Commands,
    mut labels: Query<(&CellLabel, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time).min(0.05);
    show.clock += dt;
    let clock = show.clock;
    let model = sim.game.model();
    let (reels, rows) = (model.grid.reels, model.grid.rows);
    let label_of = |labels: &Query<(&CellLabel, &mut Text)>, reel: u8, row: u8| -> String {
        labels
            .iter()
            .find(|(l, _)| l.reel == reel && l.row == row)
            .map(|(_, t)| t.0.clone())
            .unwrap_or_default()
    };

    // The trigger sequence.
    if let Some(t0) = show.intro {
        let t = t0 + dt;
        show.intro = Some(t);
        if crossed(t0, t, INTRO_BELL) {
            sounds.play(&mut commands, "feature_bell");
        }
        if crossed(t0, t, INTRO_BURST) {
            sounds.play(&mut commands, "SND_FeatureTrigger");
            for (l, mut text) in &mut labels {
                if open(&text.0) {
                    if !text.0.starts_with(DIM) {
                        text.0 = format!("{DIM}{}", text.0);
                    }
                } else {
                    show.bursts.push((l.reel, l.row, clock));
                }
            }
        }
        if crossed(t0, t, INTRO_TRANSITION) {
            show.transition_at = Some(clock);
        }
        if crossed(t0, t, INTRO_GRID) {
            show.grid = true;
        }
        if t >= INTRO_END {
            show.intro = None;
            show.awaiting = Some(Await::Start(Feature::HoldAndSpin));
            show.reset_wait();
        }
    }

    // START FEATURE pressed: the counter counts up to the respins.
    if matches!(show.stage, Stage::HoldSpin { .. })
        && show.grid
        && show.intro.is_none()
        && show.awaiting.is_none()
        && !show.started
    {
        show.started = true;
        show.count_up = Some(0.0);
    }
    if let Some(t0) = show.count_up {
        let t = t0 + dt;
        show.count_up = Some(t);
        let target = show.pending_left;
        let steps = (t / COUNT_STEP) as u8;
        while show.counter < target && show.counter < steps {
            show.counter += 1;
            sounds.play(&mut commands, "SND_Fea_Inc");
        }
        if show.counter >= target && t >= f32::from(target) * COUNT_STEP + COUNT_HOLD {
            show.count_up = None;
        }
    }

    // A respin was presented: every open position spins and stops in turn.
    if let Some(landed) = show.respin_landed.take() {
        show.counter = show.counter.saturating_sub(1);
        let mut stops = Vec::new();
        for reel in 0..reels {
            for row in 0..rows {
                let new = landed.contains(&(reel, row));
                if new || open(&label_of(&labels, reel, row)) {
                    let k = stops.len() as f32;
                    stops.push((reel, row, RESPIN_FIRST_STOP + k * RESPIN_STOP_GAP, new));
                }
            }
        }
        show.respins_run += 1;
        if let Some(id) = show.whir.take() {
            commands.entity(id).try_despawn();
        }
        show.whir = sounds.play_loop(&mut commands, "SND_ST_Whir_Loop", 0.7);
        show.respin = Some(RespinAnim {
            stops,
            base: show.respins_run * 1009,
            ..default()
        });
    }
    let mut finished = false;
    if let Some(mut anim) = show.respin.take() {
        anim.t += dt;
        while let Some(&(reel, row, at, coin)) = anim.stops.get(anim.done) {
            if anim.t < at {
                break;
            }
            if coin {
                sounds.play(
                    &mut commands,
                    &format!("SND_HandSDoonk{}", 1 + anim.done % 5),
                );
                show.bursts.push((reel, row, clock));
            } else {
                sounds.play(&mut commands, "reel_clink1");
                // The position comes to rest on the strip's landing symbol.
                let end = anim.end_of(at);
                for (l, mut text) in &mut labels {
                    if l.reel == reel && l.row == row && open(&text.0) {
                        text.0 = format!("{DIM}{}", respin_key(reel, row, end, end, false));
                    }
                }
            }
            anim.done += 1;
        }
        if anim.done >= anim.stops.len() {
            finished = true;
        } else {
            show.respin = Some(anim);
        }
    }
    if finished && let Some(id) = show.whir.take() {
        commands.entity(id).try_despawn();
    }
    if finished {
        if show.pending_left > show.counter {
            sounds.play(&mut commands, "SND_Fea_Inc");
        }
        show.counter = show.pending_left;
    }

    // The count after the feature.
    let total_bet = sim.game.total_bet();
    if show.collect_start && show.respin.is_none() {
        show.collect_start = false;
        let mut coins = Vec::new();
        let mut jackpots = show.jackpots.clone();
        for reel in 0..reels {
            for row in 0..rows {
                let l = label_of(&labels, reel, row);
                let Some(face) = l.strip_prefix("ORB\n") else {
                    continue;
                };
                let amount = face_credits(face, total_bet).unwrap_or_else(|| {
                    jackpots
                        .iter()
                        .position(|(tier, _)| format!("{tier:?}") == face)
                        .map(|i| jackpots.remove(i).1)
                        .unwrap_or(0)
                });
                coins.push((reel, row, amount));
            }
        }
        show.winner = Some(0);
        show.collect = Some(Collect { coins, ..default() });
    }
    let mut done = None;
    if let Some(mut c) = show.collect.take() {
        c.t += dt;
        // Each coin in turn: its fire bolt leaves the coin, and when it
        // reaches the WINNER box the box splashes and the total rises.
        while c.launched < c.coins.len() && c.t >= c.launched as f32 * COLLECT_STEP + 0.2 {
            let (reel, row, _) = c.coins[c.launched];
            show.bolt = Some((reel, row, clock));
            c.launched += 1;
        }
        while c.counted < c.coins.len()
            && c.t >= c.counted as f32 * COLLECT_STEP + 0.2 + BOLT_FLIGHT
        {
            let (_, _, amount) = c.coins[c.counted];
            show.winner = Some(show.winner.unwrap_or(0) + amount);
            show.splash_at = Some(clock);
            sounds.play(
                &mut commands,
                &format!("SND_HandSDoonk{}", 1 + c.counted % 5),
            );
            c.counted += 1;
        }
        if c.counted >= c.coins.len() && c.t >= c.coins.len() as f32 * COLLECT_STEP + COLLECT_HOLD {
            done = show.winner;
        } else {
            show.collect = Some(c);
        }
    }
    if done.is_some() {
        show.collected = done;
    }
}

/// Draw the Hold & Spin animations: spinning positions, fire rings, bursts,
/// the transition fire and the WINNER box.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn hold_and_spin_draw(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    art: Res<ArtPack>,
    denom: Res<Denom>,
    labels: Query<(&CellLabel, &Text), Without<WinnerText>>,
    mut spins: Query<(&HsSpin, &mut Visibility, &Children)>,
    mut images: Query<(&HsSpinImage, &mut Node, &mut ImageNode, &mut ZIndex), Without<HsSpin>>,
    mut rings: Query<
        (&HsRing, &mut ImageNode, &mut Visibility),
        (Without<HsSpin>, Without<HsSpinImage>, Without<HsBurst>),
    >,
    mut bursts: Query<
        (&HsBurst, &mut ImageNode, &mut Visibility),
        (Without<HsSpin>, Without<HsSpinImage>, Without<HsRing>),
    >,
    mut fx: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<HsTransition>,
            Without<HsSpin>,
            Without<HsSpinImage>,
            Without<HsRing>,
            Without<HsBurst>,
            Without<WinnerBox>,
        ),
    >,
    mut winner: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<WinnerBox>,
            Without<HsSpin>,
            Without<HsSpinImage>,
            Without<HsRing>,
            Without<HsBurst>,
            Without<HsTransition>,
        ),
    >,
    mut winner_text: Query<&mut Text, With<WinnerText>>,
    mut bolt: Query<
        (&mut ImageNode, &mut Visibility, &mut Node),
        (
            With<HsBolt>,
            Without<WinnerSplash>,
            Without<WinnerBox>,
            Without<HsTransition>,
            Without<HsSpin>,
            Without<HsSpinImage>,
            Without<HsRing>,
            Without<HsBurst>,
        ),
    >,
    mut splash: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<WinnerSplash>,
            Without<HsBolt>,
            Without<WinnerBox>,
            Without<HsTransition>,
            Without<HsSpin>,
            Without<HsSpinImage>,
            Without<HsRing>,
            Without<HsBurst>,
        ),
    >,
) {
    if !fidelity.0 {
        return;
    }
    let clock = show.clock;
    let frame = |name: &str, t: f32, looped: bool| -> Option<Handle<Image>> {
        let frames = art.anims.get(name)?;
        let n = (t.max(0.0) * ANIM_FPS) as usize;
        let i = if looped {
            n % frames.len().max(1)
        } else if n < frames.len() {
            n
        } else {
            return None;
        };
        frames.get(i).cloned()
    };
    let set = |vis: &mut Visibility, on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    };
    let label = |reel: u8, row: u8| -> &str {
        labels
            .iter()
            .find(|(l, _)| l.reel == reel && l.row == row)
            .map_or("", |(_, t)| t.0.as_str())
    };
    let spinning =
        |reel: u8, row: u8| -> Option<(f32, i64, bool)> { show.respin.as_ref()?.strip(reel, row) };

    // Open positions spinning: dimmed symbols scrolling, coins bright.
    let (cw, ch) = cell_size();
    for (spin, mut vis, children) in &mut spins {
        let t = spinning(spin.reel, spin.row);
        set(&mut vis, t.is_some());
        let Some((pos, end, coin)) = t else { continue };
        let base = pos.floor() as i64;
        for child in children.iter() {
            let Ok((img, mut node, mut image, mut z)) = images.get_mut(child) else {
                continue;
            };
            let n = base + i64::from(img.index);
            let key = respin_key(spin.reel, spin.row, n, end, coin);
            let (w, h) = art
                .sizes
                .get(key)
                .map_or((cw, ch), |s| symbol_size(key, *s));
            // Symbols taller than the position shrink to fit it; a coin keeps
            // its size, so it lands without a jump.
            let fit = if key == "orb" {
                1.0
            } else {
                (ch * 0.96 / h).min(1.0)
            };
            let (w, h) = (w * fit, h * fit);
            if let Some(handle) = art.symbols.get(key)
                && image.image != *handle
            {
                image.image = handle.clone();
            }
            // Symbols dimmed under the veil, as on resting positions; coins
            // bright in front of it.
            let (tint, layer) = if key == "orb" {
                (Color::WHITE, ZIndex(2))
            } else {
                (DIM_TINT, ZIndex(0))
            };
            if image.color != tint {
                image.color = tint;
            }
            if *z != layer {
                *z = layer;
            }
            // Symbols move down through the position.
            let cy = ch / 2.0 + (pos - n as f32) * ch;
            node.left = Val::Px((cw - w) / 2.0);
            node.top = Val::Px(cy - h / 2.0);
            node.width = Val::Px(w);
            node.height = Val::Px(h);
        }
    }

    // Fire rings on held coins, and bursts.
    let hs = show.grid && show.hold_and_spin();
    for (r, mut image, mut vis) in &mut rings {
        let held = hs
            && !show.fire_taken(r.reel, r.row)
            && !open(label(r.reel, r.row))
            && spinning(r.reel, r.row).is_none()
            && !label(r.reel, r.row).is_empty();
        let phase = f32::from(r.reel * 3 + r.row) * 0.21;
        let f = if held {
            frame("ring", clock + phase, true)
        } else {
            None
        };
        match f {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }
    for (b, mut image, mut vis) in &mut bursts {
        let start = show
            .bursts
            .iter()
            .rev()
            .find(|(r, w, _)| *r == b.reel && *w == b.row)
            .map(|(_, _, s)| *s);
        match start.and_then(|s| frame("burst", clock - s, false)) {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }

    // The transition fire over the whole screen.
    for (mut image, mut vis) in &mut fx {
        match show
            .transition_at
            .and_then(|s| frame("transition", clock - s, false))
        {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }

    // The collection bolt and the splash it makes on the WINNER box.
    for (mut image, mut vis, mut node) in &mut bolt {
        let f = show.bolt.and_then(|(reel, row, s)| {
            let (k, flip, left, top, w, h) = bolt_for(reel, row);
            frame(&format!("bolt{k}"), clock - s, false).map(|f| (f, flip, left, top, w, h))
        });
        match f {
            Some((f, flip, left, top, w, h)) => {
                if image.image != f {
                    image.image = f;
                }
                image.flip_x = flip;
                node.left = Val::Px(left);
                node.top = Val::Px(top);
                node.width = Val::Px(w);
                node.height = Val::Px(h);
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }
    for (mut image, mut vis) in &mut splash {
        match show
            .splash_at
            .and_then(|s| frame("splash", clock - s, false))
        {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }

    // The WINNER total.
    for (mut image, mut vis) in &mut winner {
        // The gamble shows the jackpots, not the WINNER total.
        let winner = show.winner.filter(|_| !gamble.active);
        match winner.and(frame("flameborder_sm", clock, true)) {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }
    let amount = show
        .winner
        .filter(|_| !gamble.active)
        .map(|w| money(w, denom.cents()))
        .unwrap_or_default();
    for mut t in &mut winner_text {
        if t.0 != amount {
            t.0 = amount.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Free games: the doors reveal and the win banner
// ---------------------------------------------------------------------------
//
// Footage: the doors land with the reels; about 1.2 s after the last reel
// stops every door swings open together (the game's own 42-frame sequence,
// about 0.7 s) on the symbol they reveal; then the game's wins show, and the
// banner reads "WIN $x WIN" while the free game's win counts up.

/// Seconds after the reveal is presented that the doors open, and their
/// frame rate.
const REVEAL_OPEN: f32 = 1.2;
const REVEAL_FPS: f32 = 60.0;

/// The doors opening on one position.
#[derive(Component)]
pub struct DoorFx {
    reel: u8,
    row: u8,
}

/// The amount on the free games' win banner.
#[derive(Component)]
pub struct BannerAmount;

/// The opening doors of one position.
pub fn spawn_door_fx(cell: &mut ChildSpawnerCommands, art: &ArtPack, reel: u8, row: u8) {
    let (cw, ch) = cell_size();
    let (w, h) = art
        .sizes
        .get("doors")
        .map_or((cw, ch), |s| symbol_size("doors", *s));
    cell.spawn((
        DoorFx { reel, row },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px((cw - w) / 2.0),
            top: Val::Px((ch - h) / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// The amount text inside the feature banner.
pub fn spawn_banner_amount(b: &mut ChildSpawnerCommands, font: &Handle<Font>, w: f32, h: f32) {
    b.spawn((
        BannerAmount,
        Text::new(""),
        TextFont {
            font: bevy::text::FontSource::Handle(font.clone()),
            font_size: FontSize::Px(h * 0.62),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.35)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(w * 0.25),
            top: Val::Px(h * 0.12),
            width: Val::Px(w * 0.5),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Run the doors reveal: wait, open the doors on the revealed symbol.
pub fn door_reveal(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut show: ResMut<FeatureShow>,
    mut labels: Query<(&CellLabel, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time).min(0.05);
    let Some((t0, symbol, cells)) = show.reveal.take() else {
        return;
    };
    let t = t0 + dt;
    // The positions the doors stand on, as the reveal begins.
    let cells = if cells.is_empty() {
        labels
            .iter()
            .filter(|(_, text)| text.0 == "doors")
            .map(|(l, _)| (l.reel, l.row))
            .collect()
    } else {
        cells
    };
    if t0 < REVEAL_OPEN && t >= REVEAL_OPEN {
        for (l, mut text) in &mut labels {
            if cells.contains(&(l.reel, l.row)) {
                text.0 = symbol.clone();
            }
        }
    }
    let frames = 42.0;
    if t < REVEAL_OPEN + frames / REVEAL_FPS {
        show.reveal = Some((t, symbol, cells));
    }
}

/// Draw the doors opening.
pub fn door_draw(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    art: Res<ArtPack>,
    mut doors: Query<(&DoorFx, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    let frame = show.reveal.as_ref().and_then(|(t, _, cells)| {
        let n = ((t - REVEAL_OPEN) * REVEAL_FPS).floor();
        (n >= 0.0)
            .then(|| art.anims.get("reveal").and_then(|f| f.get(n as usize)))
            .flatten()
            .map(|h| (h.clone(), cells))
    });
    for (d, mut image, mut vis) in &mut doors {
        let f = frame
            .as_ref()
            .filter(|(_, cells)| cells.contains(&(d.reel, d.row)))
            .map(|(h, _)| h.clone());
        let want = match f {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                Visibility::Inherited
            }
            None => Visibility::Hidden,
        };
        if *vis != want {
            *vis = want;
        }
    }
}

// ---------------------------------------------------------------------------
// Gamble
// ---------------------------------------------------------------------------
//
// From cabinet footage of the gamble: the reels give way to a screen with a
// face-down card in the middle. At the left, "Full Gamble Selected", "Gamble
// Amount" in credits and dollars, a RED button and the heart and diamond
// buttons; at the right, HISTORY (the last cards turned), "Red/Black Gamble
// to win" (alternating with "Suit Gamble to win"), a BLACK button and the
// club and spade buttons; under them "Select a SUIT, RED/BLACK or TAKE WIN".
// A winning card turns face up with "WIN" across it and "Gamble Win" above.
// The card's outcome is the engine's (`Command::Gamble`), never this code's.

/// Cards kept in the history.
const HISTORY: usize = 5;
/// Seconds a turned card shows before the next choice (a win) or the end of
/// the gamble (a loss).
const GAMBLE_SHOW_WIN: f32 = 1.8;
const GAMBLE_SHOW_LOSS: f32 = 2.2;

/// The gamble as shown.
#[derive(Resource, Default)]
pub struct GambleShow {
    pub active: bool,
    /// The card turned up, whether it won, seconds since.
    card: Option<(u8, bool, f32)>,
    /// The last gamble made: its choice and what it staked.
    last: Option<(GamblePick, u64)>,
    /// Cards turned, most recent first.
    history: Vec<u8>,
    /// What is at stake now (credits).
    pub stake: u64,
    /// The win the gamble ended on, shown on the WIN meter until the next
    /// game.
    pub result: Option<u64>,
    /// Testing: the autoplay's choices, its next choice, its wait.
    auto: Option<String>,
    auto_step: usize,
    auto_wait: f32,
}

impl GambleShow {
    /// Testing: the gamble autoplay holds the unattended spin while a win can
    /// be gambled.
    pub fn holds_autospin(&self, can_gamble: bool) -> bool {
        can_gamble && self.auto.as_deref().is_some_and(|s| !s.is_empty())
    }
}

/// Enter, play and leave the gamble.
#[allow(clippy::too_many_arguments)]
pub fn gamble_control(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    smoke: Res<Smoke>,
    mut input: ResMut<CabinetInput>,
    mut gamble: ResMut<GambleShow>,
    mut sim: ResMut<Sim>,
    mut presentation: ResMut<Presentation>,
    show: Res<FeatureShow>,
    rollup: Res<Rollup>,
    sounds: Res<Sounds>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time).min(0.05);
    // Testing (`DRAGONLINK_AUTOGAMBLE`, an unattended run only): gamble each
    // win, picking the listed choices in turn ("rbs" = red, black, spades),
    // one press every 1.5 s, so the gamble can be recorded without a
    // keyboard.
    if smoke.frames_left.is_some() {
        if gamble.auto.is_none() {
            gamble.auto = Some(std::env::var("DRAGONLINK_AUTOGAMBLE").unwrap_or_default());
        }
        let picks = gamble.auto.clone().unwrap_or_default();
        if !picks.is_empty() {
            gamble.auto_wait -= dt;
            if !gamble.active {
                // Into the gamble as soon as a win can be gambled.
                input.gamble = true;
            } else if gamble.auto_wait <= 0.0 && gamble.card.is_none() {
                gamble.auto_wait = 1.5;
                {
                    let n = gamble.auto_step % picks.len();
                    gamble.auto_step += 1;
                    input.pick = match picks.as_bytes()[n] {
                        b'r' => Some(GamblePick::Red),
                        b'b' => Some(GamblePick::Black),
                        b'h' => Some(GamblePick::Hearts),
                        b'd' => Some(GamblePick::Diamonds),
                        b'c' => Some(GamblePick::Clubs),
                        b's' => Some(GamblePick::Spades),
                        _ => None,
                    };
                    if input.pick.is_none() {
                        input.take_win = true;
                    }
                }
            }
        }
    }
    // The keyboard's gamble keys: G gamble, R / B a colour, H / D / C / S a
    // suit, T take win.
    if keys.just_pressed(KeyCode::KeyG) {
        input.gamble = true;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        input.take_win = true;
    }
    for (key, pick) in [
        (KeyCode::KeyR, GamblePick::Red),
        (KeyCode::KeyB, GamblePick::Black),
        (KeyCode::KeyH, GamblePick::Hearts),
        (KeyCode::KeyD, GamblePick::Diamonds),
        (KeyCode::KeyC, GamblePick::Clubs),
        (KeyCode::KeyS, GamblePick::Spades),
    ] {
        if gamble.active && keys.just_pressed(key) {
            input.pick = Some(pick);
        }
    }
    if !gamble.active {
        let at_rest = sim.game.is_idle()
            && presentation.queue.is_empty()
            && show.awaiting.is_none()
            && !show.blocks()
            && !rollup.active();
        if input.gamble && at_rest && sim.game.can_gamble() {
            gamble.active = true;
            gamble.card = None;
            gamble.stake = sim.game.state().gamble_stake;
            input.skip_spin = true;
        }
        return;
    }
    // Every press while gambling belongs to the gamble.
    let take = input.take_win
        || input.spin
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter);
    input.skip_spin = true;
    if let Some((card, won, t)) = gamble.card {
        let t = t + dt;
        gamble.card = Some((card, won, t));
        if !won && t >= GAMBLE_SHOW_LOSS {
            gamble.active = false;
            gamble.card = None;
            gamble.result = Some(0);
        } else if won && t >= GAMBLE_SHOW_WIN {
            gamble.card = None;
            if !sim.game.can_gamble() {
                // The limit: the win is kept.
                gamble.active = false;
                gamble.result = Some(gamble.stake);
            }
        }
        return;
    }
    let Sim { game, rng } = &mut *sim;
    if let Some(pick) = input.pick.take() {
        if let Ok(events) = game.apply(Command::Gamble { pick }, rng) {
            for e in &events {
                match e {
                    Event::GambleResolved {
                        pick,
                        card,
                        won,
                        stake,
                        award,
                    } => {
                        gamble.last = Some((*pick, *stake));
                        gamble.card = Some((*card, *won, 0.0));
                        gamble.history.insert(0, *card);
                        gamble.history.truncate(HISTORY);
                        gamble.stake = *award;
                        if *won {
                            sounds.play(&mut commands, "wintune02");
                        }
                    }
                    Event::CreditsChanged { to, .. } => presentation.shown_credits = *to,
                    _ => {}
                }
            }
        }
    } else if take {
        let _ = game.apply(Command::TakeWin, rng);
        gamble.active = false;
        gamble.result = Some(gamble.stake);
    }
}

/// The gamble screen and its parts.
#[derive(Component)]
pub struct GambleRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GambleText {
    /// "Red/Black Gamble / Gamble Amount / n / $x".
    Amount,
    /// "Red/Black Gamble / to win ..." alternating with the suit gamble.
    ToWin,
    /// "Gamble Win / n / $x" over a winning card.
    GambleWin,
}
/// The card in the middle: its back, or the turned card's face.
#[derive(Component)]
pub struct GambleCard;
/// "WIN" stamped across a winning card.
#[derive(Component)]
pub struct GambleStamp;
/// A card in the history strip.
#[derive(Component)]
pub struct GambleHistory(usize);

/// The text style of the gamble screen: bold italic white with a dark
/// shadow (cabinet footage).
fn gamble_text(
    s: &str,
    font: &Handle<Font>,
    size: f32,
) -> (Text, TextFont, TextColor, TextLayout, TextShadow) {
    (
        Text::new(s),
        TextFont {
            font: bevy::text::FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.97, 0.94)),
        TextLayout::justify(Justify::Center),
        TextShadow {
            offset: Vec2::new(2.0, 3.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.85),
        },
    )
}

/// Build the gamble screen (hidden until a gamble): the game's feature
/// backdrop, the panels either side of the card, the card, the history.
pub fn spawn_gamble_screen(c: &mut ChildSpawnerCommands, art: &ArtPack, fonts: &CabFonts) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    // The whole screen between the jackpots and the meter band.
    let top = JACKPOT_BAR_TOP + JACKPOT_BAR_W * 104.0 / 1225.0 + 4.0;
    let (w, h) = (CANVAS_W, BAND_TOP - top);
    let at = |cx: f32, cy: f32, bw: f32, bh: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(cx - bw / 2.0),
        top: Val::Px(cy - bh / 2.0),
        width: Val::Px(bw),
        height: Val::Px(bh),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let italic = &fonts.italic;
    c.spawn((
        GambleRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(top),
            width: Val::Px(w),
            height: Val::Px(h),
            overflow: Overflow::clip(),
            ..default()
        },
        GlobalZIndex(12),
        Visibility::Hidden,
    ))
    .with_children(|g| {
        // The game's own sunset over the Great Wall, as the gamble shows it.
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                image: ui("feature_background"),
                image_mode: NodeImageMode::Stretch,
                // Dusk, as the cabinet shows it.
                color: Color::srgb(0.55, 0.5, 0.52),
                ..default()
            },
        ));
        // The gold rule joining the two tabs.
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(w * 0.5 - 490.0),
                top: Val::Px(173.0),
                width: Val::Px(980.0),
                height: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.92, 0.7, 0.25)),
        ));
        let (lx, cx, rx) = (w * 0.5 - 490.0, w * 0.5, w * 0.5 + 490.0);
        let pill = |g: &mut ChildSpawnerCommands, x: f32, y: f32, pw: f32, s: &str| {
            g.spawn((
                at(x, y, pw, 50.0),
                ImageNode {
                    image: ui("pill"),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ))
            .with_children(|p| {
                p.spawn(gamble_text(s, italic, 25.0));
            });
        };
        // Left: the tab, the amount, RED and the red suits.
        pill(g, lx, 175.0, 400.0, "Full Gamble Selected");
        g.spawn((at(lx, 290.0, 460.0, 170.0),)).with_children(|t| {
            t.spawn((GambleText::Amount, gamble_text("", italic, 30.0)));
        });
        gamble_button(g, art, GamblePick::Red, lx, 440.0, 420.0, 88.0);
        gamble_button(g, art, GamblePick::Hearts, lx - 106.0, 548.0, 190.0, 96.0);
        gamble_button(g, art, GamblePick::Diamonds, lx + 106.0, 548.0, 190.0, 96.0);

        // Middle: "Gamble Win" over the card, "WIN" across it.
        g.spawn((at(cx, 118.0, 400.0, 130.0),)).with_children(|t| {
            t.spawn((GambleText::GambleWin, gamble_text("", italic, 30.0)));
        });
        g.spawn((
            GambleCard,
            at(cx, 435.0, 300.0, 420.0),
            ImageNode {
                image: ui("card_back"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        g.spawn((
            GambleStamp,
            at(cx, 500.0, 420.0, 170.0),
            ImageNode {
                image: ui("win_stamp"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(13),
            Visibility::Hidden,
        ));

        // Right: HISTORY, the amount to win, BLACK and the black suits.
        g.spawn((
            Node {
                column_gap: Val::Px(6.0),
                padding: UiRect::horizontal(Val::Px(56.0)),
                justify_content: JustifyContent::FlexStart,
                ..at(rx, 175.0, 440.0, 50.0)
            },
            ImageNode {
                image: ui("pill"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_children(|t| {
            t.spawn(gamble_text("HISTORY", italic, 25.0));
            for i in 0..HISTORY {
                t.spawn((
                    GambleHistory(i),
                    Text::new(""),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.symbols.clone()),
                        font_size: FontSize::Px(28.0),
                        ..default()
                    },
                    TextColor(Color::BLACK),
                ));
            }
        });
        g.spawn((at(rx, 290.0, 460.0, 170.0),)).with_children(|t| {
            t.spawn((GambleText::ToWin, gamble_text("", italic, 30.0)));
        });
        gamble_button(g, art, GamblePick::Black, rx, 440.0, 420.0, 88.0);
        gamble_button(g, art, GamblePick::Clubs, rx - 106.0, 548.0, 190.0, 96.0);
        gamble_button(g, art, GamblePick::Spades, rx + 106.0, 548.0, 190.0, 96.0);

        g.spawn((at(cx, 690.0, 900.0, 50.0),)).with_children(|t| {
            t.spawn(gamble_text(
                "Select a SUIT, RED/BLACK or TAKE WIN",
                italic,
                26.0,
            ));
        });
    });
}

/// A gamble button on the screen: the gold-rimmed chrome RED / BLACK
/// lozenges and the gold suit ovals.
fn gamble_button(
    g: &mut ChildSpawnerCommands,
    art: &ArtPack,
    pick: GamblePick,
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
) {
    let key = match pick {
        GamblePick::Red => "btn_red",
        GamblePick::Black => "btn_black",
        GamblePick::Hearts => "suit_hearts",
        GamblePick::Diamonds => "suit_diamonds",
        GamblePick::Clubs => "suit_clubs",
        GamblePick::Spades => "suit_spades",
    };
    g.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(cy - h / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        ImageNode {
            image: art.ui.get(key).cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
    ))
    .observe(
        move |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| {
            input.pick = Some(pick);
        },
    );
}

/// The suit of a card (0..52) and whether it is red.
fn card_suit(card: u8) -> (&'static str, bool) {
    const SUITS: [&str; 4] = ["\u{2665}", "\u{2666}", "\u{2663}", "\u{2660}"];
    let suit = (card / 13).min(3);
    (SUITS[suit as usize], suit < 2)
}

/// What kind of gamble a pick is, as the screen names it.
fn gamble_kind(pick: Option<GamblePick>) -> &'static str {
    match pick {
        Some(GamblePick::Red | GamblePick::Black) | None => "Red/Black Gamble",
        Some(_) => "Suit Gamble",
    }
}

/// Draw the gamble screen.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn gamble_screen(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    gamble: Res<GambleShow>,
    denom: Res<Denom>,
    art: Res<ArtPack>,
    mut root: Query<&mut Visibility, With<GambleRoot>>,
    mut texts: Query<(&GambleText, &mut Text), Without<GambleHistory>>,
    mut card: Query<&mut ImageNode, (With<GambleCard>, Without<GambleStamp>)>,
    mut stamp: Query<&mut Visibility, (With<GambleStamp>, Without<GambleRoot>)>,
    mut history: Query<(&GambleHistory, &mut Text, &mut TextColor), Without<GambleText>>,
) {
    if !fidelity.0 {
        return;
    }
    let set = |vis: &mut Visibility, on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    };
    for mut v in &mut root {
        set(&mut v, gamble.active);
    }
    if !gamble.active {
        return;
    }
    let cents = denom.cents();
    let shown = gamble.card;
    let key = shown.map_or("card_back".to_string(), |(c, _, _)| format!("card_{c}"));
    if let Some(h) = art.ui.get(&key) {
        for mut image in &mut card {
            if image.image != *h {
                image.image = h.clone();
            }
        }
    }
    let won = shown.is_some_and(|(_, w, _)| w);
    for mut v in &mut stamp {
        set(&mut v, won);
    }
    for (h, mut t, mut c) in &mut history {
        let s = gamble.history.get(h.0).map(|card| card_suit(*card));
        let txt = s.map(|(s, _)| s.to_string()).unwrap_or_default();
        if t.0 != txt {
            t.0 = txt;
        }
        c.0 = if s.is_some_and(|(_, red)| red) {
            Color::srgb(0.95, 0.12, 0.12)
        } else {
            Color::srgb(0.05, 0.05, 0.08)
        };
    }
    // While a card is up the screen shows the gamble just made: its kind,
    // what was staked, what it won. Otherwise the stake now, and what each
    // kind of gamble would win.
    let amount = |credits: u64| format!("{credits}\n{}", money(credits, cents));
    let suit_turn = ((time.elapsed_secs() / 2.0) as u32) % 2 == 1;
    let (kind, staked) = match (shown, gamble.last) {
        (Some(_), Some((pick, stake))) => (gamble_kind(Some(pick)), stake),
        (_, last) => (gamble_kind(last.map(|l| l.0)), gamble.stake),
    };
    for (which, mut t) in &mut texts {
        let s = match which {
            GambleText::Amount => format!("{kind}\nGamble Amount\n{}", amount(staked)),
            GambleText::ToWin if shown.is_some() => {
                let mult = gamble.last.map_or(2, |(p, _)| p.multiplier());
                format!("{kind}\nto win\n{}", amount(staked.saturating_mul(mult)))
            }
            GambleText::ToWin if suit_turn => {
                format!("Suit Gamble\nto win\n{}", amount(staked.saturating_mul(4)))
            }
            GambleText::ToWin => format!(
                "Red/Black Gamble\nto win\n{}",
                amount(staked.saturating_mul(2))
            ),
            GambleText::GambleWin if won => format!("Gamble Win\n{}", amount(gamble.stake)),
            GambleText::GambleWin => String::new(),
        };
        if t.0 != s {
            t.0 = s;
        }
    }
}

// ---------------------------------------------------------------------------
// Winning lines
// ---------------------------------------------------------------------------
//
// Cabinet footage of a line win: the line is drawn with the game's own
// pieces, a numbered badge over the line tab at each end and a short dash in
// each gap between the reels, at the line's rows; it blinks (on about three
// quarters of a second in each second), each winning line in turn, and the
// message at the right reads "Line 1 wins 2".

/// Seconds each winning line shows, and the part of that it is lit.
const WIN_LINE_SECONDS: f32 = 1.0;
const WIN_LINE_LIT: f32 = 0.75;
/// The badge and dash sizes on the canvas (footage, scaled).
const LINE_BADGE: (f32, f32) = (104.0, 80.0);
const LINE_DASH: (f32, f32) = (52.0, 26.0);

/// A badge at one end of the shown line.
#[derive(Component)]
pub struct LineBadge {
    right: bool,
}
/// The number on a badge (lines above five; the first five badges carry
/// their own).
#[derive(Component)]
pub struct LineBadgeNumber;
/// The dash in one gap between reels.
#[derive(Component)]
pub struct LineDash {
    gap: u8,
}

/// The pieces of the shown winning line, hidden until a line wins.
pub fn spawn_win_lines(c: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    for right in [false, true] {
        c.spawn((
            LineBadge { right },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(LINE_BADGE.0),
                height: Val::Px(LINE_BADGE.1),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(6),
            Visibility::Hidden,
        ))
        .with_children(|b| {
            b.spawn((
                LineBadgeNumber,
                Text::new(""),
                TextFont {
                    font: bevy::text::FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(52.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::new(2.0, 2.0),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                },
            ));
        });
    }
    for gap in 0..4u8 {
        c.spawn((
            LineDash { gap },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(LINE_DASH.0),
                height: Val::Px(LINE_DASH.1),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            UiTransform::IDENTITY,
            GlobalZIndex(6),
            Visibility::Hidden,
        ));
    }
}

/// Show the winning lines in turn, and their message.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn win_lines(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    art: Res<ArtPack>,
    denom: Res<Denom>,
    gamble: Res<GambleShow>,
    mut show: ResMut<FeatureShow>,
    mut badges: Query<(&LineBadge, &mut Node, &mut ImageNode, &mut Visibility), Without<LineDash>>,
    mut numbers: Query<&mut Text, With<LineBadgeNumber>>,
    mut dashes: Query<
        (
            &LineDash,
            &mut Node,
            &mut ImageNode,
            &mut UiTransform,
            &mut Visibility,
        ),
        Without<LineBadge>,
    >,
) {
    if !fidelity.0 {
        return;
    }
    show.win_t += smoke.dt(&time);
    let showing = !show.win_lines.is_empty()
        && !gamble.active
        && show.awaiting.is_none()
        && !show.hold_and_spin()
        && show.reveal.is_none();
    let current = showing.then(|| {
        let i = (show.win_t / WIN_LINE_SECONDS) as usize % show.win_lines.len();
        show.win_lines[i]
    });
    show.win_current = current.map(|(line, count, _)| (line, count));
    show.scatter_shown = show.scatter_paid
        && !gamble.active
        && show.awaiting.is_none()
        && !show.hold_and_spin()
        && show.reveal.is_none();
    show.win_note = current.map(|(line, _, award)| {
        let shown = if (show.win_t / 2.5) as u32 % 2 == 1 {
            grouped(award)
        } else {
            money(award, denom.cents())
        };
        format!("Line {} wins {shown}", line + 1)
    });
    let lit = current.filter(|_| show.win_t % WIN_LINE_SECONDS < WIN_LINE_LIT);
    let rows = lit.and_then(|(line, _, _)| sim.game.model().paylines.get(line as usize));
    let set = |vis: &mut Visibility, on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    };
    let (Some((line, _, _)), Some(rows)) = (lit, rows) else {
        for (_, _, _, mut v) in &mut badges {
            set(&mut v, false);
        }
        for (_, _, _, _, mut v) in &mut dashes {
            set(&mut v, false);
        }
        return;
    };
    // The game's five line colours, by line number.
    let group = (line as usize % 5) + 1;
    let row_y = |reel: usize| {
        let row = rows.0.get(reel).copied().unwrap_or(1);
        cell_center(reel as u8, row).1
    };
    let badge_key = if line < 5 {
        format!("line{group}")
    } else {
        format!("line_blank{group}")
    };
    let tab_x = 2.0 * PITCH.0 + 196.0;
    for (b, mut node, mut image, mut vis) in &mut badges {
        let reel = if b.right {
            rows.0.len().saturating_sub(1)
        } else {
            0
        };
        let x = GRID_CENTER.0 + if b.right { tab_x } else { -tab_x };
        node.left = Val::Px(x - LINE_BADGE.0 / 2.0);
        node.top = Val::Px(row_y(reel) - LINE_BADGE.1 / 2.0);
        if let Some(h) = art.ui.get(&badge_key)
            && image.image != *h
        {
            image.image = h.clone();
        }
        set(&mut vis, true);
    }
    let number = if line < 5 {
        String::new()
    } else {
        (line + 1).to_string()
    };
    for mut t in &mut numbers {
        if t.0 != number {
            t.0 = number.clone();
        }
    }
    let dash_key = format!("line{group}_s");
    for (d, mut node, mut image, mut transform, mut vis) in &mut dashes {
        let a = d.gap as usize;
        let (y0, y1) = (row_y(a), row_y(a + 1));
        let x = (cell_center(d.gap, 0).0 + cell_center(d.gap + 1, 0).0) / 2.0;
        let y = (y0 + y1) / 2.0;
        node.left = Val::Px(x - LINE_DASH.0 / 2.0);
        node.top = Val::Px(y - LINE_DASH.1 / 2.0);
        // Slanted where the line steps between rows.
        let angle = (y1 - y0).atan2(PITCH.0);
        let want = UiTransform {
            rotation: Rot2::radians(angle),
            ..UiTransform::IDENTITY
        };
        if *transform != want {
            *transform = want;
        }
        if let Some(h) = art.ui.get(&dash_key)
            && image.image != *h
        {
            image.image = h.clone();
        }
        set(&mut vis, true);
    }
}

// ---------------------------------------------------------------------------
// Stacked emperors
// ---------------------------------------------------------------------------
//
// Cabinet footage: two or three emperors stacked on a reel become one tall
// full-length emperor (the game's own Wild2UP / Wild3UP animations: the heads
// wipe into the figure, which then burns with a fire border) and stay so
// until the reel spins again.

/// The tiles of the game's stack animations: each 130 x 118 frame pixels,
/// the first at (43, 39).
const WILD_TILE: (f32, f32) = (130.0, 118.0);
const WILD_TILE_AT: (f32, f32) = (43.0, 39.0);
/// Frames per second of the stack animations.
const WILD_FPS: f32 = 30.0;

/// Emperor stacks landed: reel, top row, height, when (seconds of `Time`).
#[derive(Resource, Default)]
pub struct WildStacks(Vec<(u8, u8, u8, f32)>);

/// The tall emperor drawn over one reel's stack.
#[derive(Component)]
pub struct WildTall {
    reel: u8,
}

/// One tall-emperor node per reel, over the reels.
pub fn spawn_wild_talls(c: &mut ChildSpawnerCommands, reels: u8) {
    for reel in 0..reels {
        c.spawn((
            WildTall { reel },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(2),
            Visibility::Hidden,
        ));
    }
}

/// Find emperor stacks as reels land; forget them as reels spin.
pub fn wild_stacks(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    reels: Res<Reels>,
    mut stacks: ResMut<WildStacks>,
) {
    if !fidelity.0 {
        return;
    }
    // A stack goes once its reel spins again (not while it settles).
    stacks
        .0
        .retain(|s| !matches!(reels.states[s.0 as usize].motion, Motion::Spinning { .. }));
    for reel in reels.just_stopped() {
        let landing = &reels.states[reel].landing;
        let mut row = 0;
        while row < landing.len() {
            if symbol_of(&landing[row]) != "emperor" {
                row += 1;
                continue;
            }
            let start = row;
            while row < landing.len() && symbol_of(&landing[row]) == "emperor" {
                row += 1;
            }
            let len = row - start;
            if len >= 2 {
                stacks
                    .0
                    .push((reel as u8, start as u8, len as u8, time.elapsed_secs()));
            }
        }
    }
}

/// Draw the tall emperors: the wipe, then the burning figure.
pub fn wild_talls(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    art: Res<ArtPack>,
    stacks: Res<WildStacks>,
    mut talls: Query<(&WildTall, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    // Each tile of the animation fills one reel cell.
    let (sx, sy) = (PITCH.0 * 0.97 / WILD_TILE.0, PITCH.1 / WILD_TILE.1);
    for (tall, mut node, mut image, mut vis) in &mut talls {
        let stack = stacks.0.iter().find(|s| s.0 == tall.reel);
        let frame = stack.and_then(|&(_, _, len, t0)| {
            let n = ((time.elapsed_secs() - t0) * WILD_FPS) as usize;
            let intro = art.anims.get(&format!("wild{len}_intro"))?;
            if n < intro.len() {
                return intro.get(n).cloned();
            }
            let lp = art.anims.get(&format!("wild{len}_loop"))?;
            lp.get((n - intro.len()) % lp.len().max(1)).cloned()
        });
        let want = match (stack, frame) {
            (Some(&(reel, top, len, _)), Some(h)) => {
                if image.image != h {
                    image.image = h;
                }
                let (cx, cy) = cell_center(reel, top);
                let frame_h = if len == 3 { 432.0 } else { 312.0 };
                node.left = Val::Px(cx - (WILD_TILE_AT.0 + WILD_TILE.0 / 2.0) * sx);
                node.top = Val::Px(cy - (WILD_TILE_AT.1 + WILD_TILE.1 / 2.0) * sy);
                node.width = Val::Px(216.0 * sx);
                node.height = Val::Px(frame_h * sy);
                Visibility::Inherited
            }
            _ => Visibility::Hidden,
        };
        if *vis != want {
            *vis = want;
        }
    }
}

// ---------------------------------------------------------------------------
// Winning symbols
// ---------------------------------------------------------------------------
//
// The symbols of the winning line being shown play the game's own win
// animations (Pic1-4, Scatter, Wild: an intro, then a loop), drawn at the
// symbol's own size; a scatter win animates every scatter.

/// The win animation drawn over one cell.
#[derive(Component)]
pub struct SymbolFx {
    reel: u8,
    row: u8,
}

/// Frames per second of the symbol win animations.
const SYMBOL_FX_FPS: f32 = 30.0;

/// The win animation over one cell (hidden until it wins).
pub fn spawn_symbol_fx(cell: &mut ChildSpawnerCommands, reel: u8, row: u8) {
    cell.spawn((
        SymbolFx { reel, row },
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Animate the symbols of the winning line being shown.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn symbol_fx(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    sim: Res<Sim>,
    art: Res<ArtPack>,
    stacks: Res<WildStacks>,
    labels: Query<(&CellLabel, &Text)>,
    mut fx: Query<(&SymbolFx, &mut Node, &mut ImageNode, &mut Visibility), Without<CellImage>>,
    mut images: Query<(&CellImage, &mut Visibility), Without<SymbolFx>>,
) {
    if !fidelity.0 {
        return;
    }
    // The positions animating now.
    let mut cells: Vec<(u8, u8)> = Vec::new();
    if let Some((line, count)) = show.win_current
        && let Some(rows) = sim.game.model().paylines.get(line as usize)
    {
        for reel in 0..count {
            if let Some(&row) = rows.0.get(reel as usize) {
                cells.push((reel, row));
            }
        }
    }
    if show.scatter_shown {
        for (l, t) in &labels {
            if t.0 == "pennant" {
                cells.push((l.reel, l.row));
            }
        }
    }
    let t = show.win_t;
    let (cw, ch) = cell_size();
    for (f, mut node, mut image, mut vis) in &mut fx {
        let on = cells.contains(&(f.reel, f.row)) && !stacks.0.iter().any(|s| s.0 == f.reel);
        let key = labels
            .iter()
            .find(|(l, _)| l.reel == f.reel && l.row == f.row)
            .map(|(_, t)| label_symbol(&t.0).0.to_string())
            .unwrap_or_default();
        let frame = on
            .then(|| {
                let n = (t * SYMBOL_FX_FPS) as usize;
                let intro = art.anims.get(&format!("{key}_intro"));
                let lp = art.anims.get(&format!("{key}_loop"))?;
                match intro {
                    Some(i) if n < i.len() => i.get(n).cloned(),
                    Some(i) => lp.get((n - i.len()) % lp.len().max(1)).cloned(),
                    None => lp.get(n % lp.len().max(1)).cloned(),
                }
            })
            .flatten();
        let want = match frame {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                let (w, hh) = art
                    .sizes
                    .get(key.as_str())
                    .map_or((cw, ch), |s| symbol_size(&key, *s));
                node.left = Val::Px((cw - w) / 2.0);
                node.top = Val::Px((ch - hh) / 2.0);
                node.width = Val::Px(w);
                node.height = Val::Px(hh);
                Visibility::Inherited
            }
            None => Visibility::Hidden,
        };
        let playing = want == Visibility::Inherited;
        if *vis != want {
            *vis = want;
        }
        // The still symbol gives way to its animation.
        if playing {
            for (ci, mut v) in &mut images {
                if ci.reel == f.reel && ci.row == f.row && *v != Visibility::Hidden {
                    *v = Visibility::Hidden;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Feature music
// ---------------------------------------------------------------------------

/// The feature music playing: its clip and entity.
#[derive(Resource, Default)]
pub struct Music(Option<(&'static str, Entity)>);

/// Play the feature's music while it runs: the game's own looped tracks from
/// its global sound cues (`GlobalSounds.mercury`), told apart on cabinet
/// footage: SND_Fea_BGM under the free games, SND_FreeSpin_BGM under Hold &
/// Spin (its banner counts "free spins").
pub fn feature_music(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    sounds: Res<Sounds>,
    mut music: ResMut<Music>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    let want = match show.stage {
        Stage::FreeGames { .. } => Some("SND_Fea_BGM"),
        Stage::HoldSpin { .. } if show.started => Some("SND_FreeSpin_BGM"),
        _ => None,
    };
    if music.0.map(|(n, _)| n) == want {
        return;
    }
    if let Some((_, id)) = music.0.take() {
        commands.entity(id).try_despawn();
    }
    if let Some(name) = want
        && let Some(id) = sounds.play_loop(&mut commands, name, 0.75)
    {
        music.0 = Some((name, id));
    }
}
