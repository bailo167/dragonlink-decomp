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

/// The denominations the GAMES MENU offers, in cents, in its order (cabinet
/// footage of Golden Century's casino presentation: 1c 2c 5c 10c $1 $2,
/// docs/reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md).
pub const MENU_DENOMS: [u64; 6] = [1, 2, 5, 10, 100, 200];
/// The bet multipliers of the deck's top row, after GAMBLE (cabinet footage:
/// BET x 1, x 2, x 3, x 5, x 10).
pub const DECK_BETS: [u32; 5] = [1, 2, 3, 5, 10];

/// The lines a denomination plays. In the casino presentation lines are not
/// chosen: 50 at 1c and 2c, 25 at 10c, 5 at $1 (footage); 25 at 5c and 5 at
/// $2 follow the club machines' maximums.
pub fn lines_for(cents: u64) -> u16 {
    match cents {
        0..=2 => 50,
        3..=10 => 25,
        _ => 5,
    }
}

/// The clip a bet key sounds (footage: x1 G3, x2 C4, x5 G4, x10 C5, the
/// first two the game's own beeps, the others the app's shared bank; x3's E4
/// is the bank's beep between them, not heard on footage).
pub fn bet_beep(multiplier: u32) -> &'static str {
    match multiplier {
        1 => "betbeep1_reduced",
        2 => "betbeep3_reduced",
        3 => "bank_betbeep3",
        5 => "bank_betbeep4",
        _ => "bank_betbeep5",
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
    /// "MIN BET 50" (casino) or "$1 = 100 Credits" (club) under the badge.
    MinBet,
    /// "50 LINES" either side of the reels.
    Lines,
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
pub fn spawn(
    commands: &mut Commands,
    model: &MathModel,
    art: &ArtPack,
    fonts: &CabFonts,
    profile: Profile,
) {
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
            spawn_coin_shower(c);
            spawn_medallion(c, art);
            spawn_win_flares(c);

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

            spawn_meter_band(c, fonts, art, gold, pale, profile);

            spawn_deck(c, art, &fonts.sans, &fonts.symbols);
            spawn_gamble_screen(c, art, fonts);
            spawn_games_menu(c, art, fonts, profile);

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

/// The screen below the reels, laid out as the cabinet lays it out: the
/// status line under the reels ("N credits per line" or "Bet Multiplier
/// N"); one dark meter band with the tall pill and the tabs at the left,
/// CREDIT, BET and WIN (its value in a box of its own, the start prompt
/// above it), and the denomination box at the right under COLLECT (club) or
/// GAMES MENU (casino); "N LINES" either side of the reels.
fn spawn_meter_band(
    c: &mut ChildSpawnerCommands,
    fonts: &CabFonts,
    art: &ArtPack,
    gold: Color,
    pale: Color,
    profile: Profile,
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

    // The tall yellow pill at the far left, with its stacked bars (its
    // function is not shown on footage).
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
    let tab_face = Color::srgb(0.99, 0.82, 0.25);
    let tab_ink = Color::srgb(0.24, 0.14, 0.04);
    let club = profile == Profile::Club;
    // The club machines: GAMES MENU over GAME RULES at the left; the casino
    // style: GAME RULES at the left, GAMES MENU over the denomination box.
    if club {
        pill(
            c,
            &fonts.sans,
            "GAMES MENU",
            160.0,
            METER_Y - 30.0,
            150.0,
            tab_face,
            tab_ink,
        );
        pill(
            c,
            &fonts.sans,
            "GAME RULES",
            160.0,
            METER_Y + 30.0,
            150.0,
            tab_face,
            tab_ink,
        );
        c.spawn((
            rect(85.0, METER_Y - 52.0, 150.0, 44.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(
            |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| {
                input.menu = true;
            },
        );
    } else {
        pill(
            c,
            &fonts.sans,
            "GAME RULES",
            160.0,
            METER_Y,
            150.0,
            tab_face,
            tab_ink,
        );
    }

    // CREDIT / BET / WIN, their values, WIN's value in its own box.
    for (text, x, w) in [
        ("CREDIT", 345.0, 220.0),
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
        (CabMeter::Credit, 615.0, 330.0),
        (CabMeter::Bet, 1030.0, 240.0),
        (CabMeter::Win, 1500.0, 420.0),
    ] {
        c.spawn((meter, label("-", font, 70.0, pale, x, METER_Y, w)));
    }

    // The denomination box at the right, under GAMES MENU (casino) or
    // COLLECT (club); its small line is MIN BET or the credit conversion.
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
        if club { "COLLECT" } else { "GAMES MENU" },
        1828.0,
        METER_Y - 70.0,
        166.0,
        tab_face,
        tab_ink,
    );
    c.spawn((
        CabMeter::Denom,
        label("1¢", font, 78.0, gold, 1828.0, METER_Y - 2.0, 166.0),
    ));
    if !club {
        // The tab and the box under it are one touch target: the GAMES MENU.
        c.spawn((
            rect(1745.0, METER_Y - 92.0, 166.0, 156.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(
            |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| {
                input.menu = true;
            },
        );
    }
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

    // "N LINES" either side of the reels (footage, both presentations).
    for x in [68.0, CANVAS_W - 68.0] {
        c.spawn((
            Node {
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..rect(x - 52.0, GRID_CENTER.1 - 44.0, 104.0, 88.0)
            },
            BackgroundColor(Color::srgba(0.04, 0.03, 0.06, 0.9)),
            BorderColor::all(Color::srgb(0.9, 0.68, 0.22)),
        ));
        c.spawn((
            CabMeter::Lines,
            label(
                "50\nLINES",
                font,
                30.0,
                gold,
                x,
                GRID_CENTER.1 - 14.0,
                104.0,
            ),
        ));
    }
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

/// The button deck below the screen, as on the Australian Dragon Link
/// cabinets: a brushed-silver panel with a blue light strip, two rows of six
/// square keys whose legends are displays, and the teardrop PLAY button.
/// GAMBLE over TAKE WIN at the left; the bet multipliers along the top row,
/// each setting its bet and playing; PLAY keys along the bottom row. The
/// legends change with the game: GOOD / LUCK while the reels spin, CHOOSE on
/// the GAMES MENU, the gamble's choices while gambling.
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

/// How the cabinet presents the game, held as data: as on the Australian club
/// machines (the owner's recorded cabinet, and the only Golden Century deck
/// legible on footage), or in the casino style ("Bet Multiplier", fixed
/// lines), which rests mostly on a recording from a US casino. Chosen at
/// start: `DRAGONLINK_PRESENTATION=casino`, club otherwise
/// (docs/reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Profile {
    #[default]
    Club,
    Casino,
}

impl Profile {
    /// The profile the environment asks for.
    pub fn from_env() -> Profile {
        match std::env::var("DRAGONLINK_PRESENTATION").as_deref() {
            Ok("casino") => Profile::Casino,
            _ => Profile::Club,
        }
    }

    /// The denominations the GAMES MENU offers: the Queensland clubs' five,
    /// or the casino session's six.
    pub fn denoms(self) -> &'static [u64] {
        match self {
            Profile::Club => &MENU_DENOMS[..5],
            Profile::Casino => &MENU_DENOMS,
        }
    }

    /// Whether a denomination asks "ARE YOU SURE?" before it is taken.
    pub fn confirms(self) -> bool {
        self == Profile::Club
    }

    /// The lines a denomination starts on: one line on the club machines,
    /// its fixed count in the casino style.
    pub fn lines_on_change(self, cents: u64) -> u16 {
        match self {
            Profile::Club => 1,
            Profile::Casino => lines_for(cents),
        }
    }
}

/// Club: the five lines keys at a denomination (footage: 1/5/10/30/50 at 1c
/// and 2c, 1/5/10/20/25 at 10c, 1 to 5 at $1; 5c is not seen and takes
/// 10c's).
pub fn line_keys(cents: u64) -> [u16; 5] {
    match cents {
        0..=2 => [1, 5, 10, 30, 50],
        3..=10 => [1, 5, 10, 20, 25],
        _ => [1, 2, 3, 4, 5],
    }
}

/// Club: the five credits-per-line keys at a denomination, 0 where a key
/// is not offered and stays blank (footage: 1/2/3/5/10 at 1c, 1 to 5 at 2c,
/// 1 and 2 at $1 with three blank; 5c and 10c are not legible and take
/// 2c's).
pub fn credit_keys(cents: u64) -> [u32; 5] {
    match cents {
        0..=1 => [1, 2, 3, 5, 10],
        2..=10 => [1, 2, 3, 4, 5],
        _ => [1, 2, 0, 0, 0],
    }
}

/// What a deck button does now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Gamble,
    TakeWin,
    /// Casino: a bet multiplier; sets the bet and plays.
    Bet(u32),
    /// Club: credits per line; only selects.
    Credits(u32),
    /// Club: a line count; sets it and plays.
    Lines(u16),
    /// Plays the bet already set.
    Play,
    Pick(GamblePick),
    /// One letter of a message spelled across the keys.
    Letter(char),
    /// A credits key not offered at this denomination: no legend.
    Blank,
    /// The card-back lattice the gamble shows on keys it does not use.
    Lattice,
    Off,
}

/// What the deck is showing: the game, a spin, the gamble or the GAMES MENU.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DeckMode {
    #[default]
    Base,
    Spinning,
    Gamble,
    Menu,
}

/// The deck's state, kept by [`light_deck`] and read by the keys.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct DeckState {
    pub mode: DeckMode,
    pub profile: Profile,
    pub cents: u64,
    /// A win waits to be gambled ("PLAY ON, GAMBLE or TAKE WIN").
    pub offer: bool,
    /// Seconds since the mode or the offer began: the one clock of the
    /// deck's flash, the teardrop's legends and the meters' switch.
    pub since: f32,
}

impl DeckState {
    /// The club machines' 1 Hz flash: in its second half-second the
    /// flashing keys show their plain face (owner's recording).
    pub fn plain(&self) -> bool {
        ((self.since * 2.0) as u32) % 2 == 1
    }

    /// The meters switch to credits every other second while a win is
    /// offered and through the gamble, on the club machines only.
    pub fn in_credits(&self) -> bool {
        self.profile == Profile::Club
            && (self.offer || self.mode == DeckMode::Gamble)
            && (self.since as u32) % 2 == 1
    }
}

/// A deck button's role.
///
/// Footage of the Australian club deck: GAMBLE top left over TAKE WIN;
/// credits-per-line keys along the top, lines keys along the bottom; G O O D
/// over L U C K while the reels spin; C H O O S E on the GAMES MENU. In the
/// gamble Golden Century's deck shows the suits on top keys 2, 3, 5 and 6,
/// BLACK on bottom key 6 and a lattice on keys it does not use (VERIFIED);
/// RED and TAKE WIN are on the illegible bottom keys 2 and 1 (HYPOTHESIS). The
/// casino profile puts the bet multipliers along the top and PLAY keys along
/// the bottom: no casino deck is legible on footage. Top left stays purple
/// in the gamble and switches the half gamble (HYPOTHESIS: the screen's
/// "Full Gamble Selected" implies another mode; the standard allows it).
pub fn role(button: CabButton, deck: &DeckState) -> Role {
    const GOOD: [char; 4] = ['G', 'O', 'O', 'D'];
    const LUCK: [char; 4] = ['L', 'U', 'C', 'K'];
    const CHOOSE: [char; 6] = ['C', 'H', 'O', 'O', 'S', 'E'];
    let CabButton::Slot { row, col } = button else {
        return match deck.mode {
            DeckMode::Menu => Role::Off,
            DeckMode::Gamble => Role::Lattice,
            _ => Role::Play,
        };
    };
    let key = (col as usize).saturating_sub(1).min(4);
    match (deck.mode, row, col) {
        (DeckMode::Base | DeckMode::Gamble, 0, 0) => Role::Gamble,
        (DeckMode::Base | DeckMode::Gamble, 1, 0) => Role::TakeWin,
        (DeckMode::Base, 0, _) => match deck.profile {
            Profile::Club => match credit_keys(deck.cents)[key] {
                0 => Role::Blank,
                n => Role::Credits(n),
            },
            Profile::Casino => Role::Bet(DECK_BETS[key]),
        },
        (DeckMode::Base, _, _) => match deck.profile {
            Profile::Club => Role::Lines(line_keys(deck.cents)[key]),
            Profile::Casino => Role::Play,
        },
        (DeckMode::Spinning, _, 1..=4) => {
            Role::Letter(if row == 0 { GOOD } else { LUCK }[col as usize - 1])
        }
        (DeckMode::Spinning, _, _) => Role::Off,
        (DeckMode::Menu, 0, _) => Role::Letter(CHOOSE[(col as usize).min(5)]),
        (DeckMode::Menu, _, _) => Role::Off,
        (DeckMode::Gamble, _, _) => match (row, col) {
            (0, 1) => Role::Pick(GamblePick::Hearts),
            (0, 2) => Role::Pick(GamblePick::Diamonds),
            (0, 4) => Role::Pick(GamblePick::Clubs),
            (0, 5) => Role::Pick(GamblePick::Spades),
            (1, 1) => Role::Pick(GamblePick::Red),
            (1, 5) => Role::Pick(GamblePick::Black),
            _ => Role::Lattice,
        },
    }
}

/// "1 LINE", "5 LINES"; "1 CREDIT", "50 CREDITS".
fn plural(n: u64, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}S")
    }
}

/// A button's legend for its role, at `lines` lines and `bet` credits per
/// line now.
fn role_text(role: Role, lines: u16, bet: u32) -> String {
    match role {
        Role::Gamble => "GAMBLE".into(),
        Role::TakeWin => "TAKE\nWIN".into(),
        Role::Bet(m) => format!("BET x {m}\n{}", u64::from(m) * u64::from(lines)),
        Role::Credits(n) => n.to_string(),
        Role::Lines(n) => format!(
            "PLAY\n{}\n{}",
            plural(u64::from(n), "LINE"),
            plural(u64::from(n) * u64::from(bet), "CREDIT")
        ),
        Role::Play => format!(
            "PLAY\n{}\n{}",
            plural(u64::from(lines), "LINE"),
            plural(u64::from(bet) * u64::from(lines), "CREDIT")
        ),
        Role::Pick(p) => pick_text(p).into(),
        Role::Letter(c) => c.to_string(),
        Role::Blank | Role::Lattice | Role::Off => String::new(),
    }
}

/// Whether a gamble choice is a colour (red or black) rather than a suit.
fn is_colour(p: GamblePick) -> bool {
    matches!(p, GamblePick::Red | GamblePick::Black)
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
        Role::Play => input.spin = true,
        Role::Bet(m) => input.bet = Some(m),
        Role::Credits(n) => input.credits = Some(n),
        Role::Lines(n) => input.lines = Some(n),
        Role::TakeWin => input.take_win = true,
        Role::Pick(p) => input.pick = Some(p),
        Role::Letter(_) | Role::Blank | Role::Lattice | Role::Off => {}
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
            Text::new(""),
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
        move |_press: On<PointerPress>, mut input: ResMut<CabinetInput>, deck: Res<DeckState>| {
            press(role(button, &deck), &mut input);
        },
    );
}

/// Light the deck and set its legends, as the cabinet does: the selected
/// credits and lines keys pale white among red credits keys and blue lines
/// keys; GAMBLE purple and TAKE WIN gold, glowing while a win can be gambled;
/// the teardrop's PLAY / REPEAT BET at rest, START FEATURE blinking while a
/// feature waits, SPIN in Hold & Spin, a lattice in the gamble; GOOD LUCK
/// wiping in from the left as the reels start; on the club machines the 1 Hz
/// flash of the keys that can be pressed while a win is offered and in the
/// gamble, with the chosen key steady light blue once a card is turned.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn light_deck(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    rollup: Res<Rollup>,
    menu: Res<GamesMenu>,
    reels: Res<Reels>,
    denom: Res<Denom>,
    profile: Res<Profile>,
    mut deck: ResMut<DeckState>,
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
    let idle = sim.game.is_idle() && presentation.queue.is_empty();
    let cents = denom.cents();
    let can_gamble = idle && gamble_open(&sim.game, cents, &show) && !rollup.active() && !waiting;
    // A suit's four times the stake may pass the gamble win limit.
    let suits_open = within_gamble_limit(gamble.stake, 4, cents);
    let mode = if menu.showing() {
        DeckMode::Menu
    } else if gambling {
        DeckMode::Gamble
    } else if reels.any_moving() {
        DeckMode::Spinning
    } else {
        DeckMode::Base
    };
    let offer = mode == DeckMode::Base && can_gamble;
    if deck.mode != mode || deck.offer != offer {
        deck.since = 0.0;
    } else {
        deck.since += smoke.dt(&time).min(0.05);
    }
    deck.mode = mode;
    deck.offer = offer;
    deck.profile = *profile;
    deck.cents = cents;
    let d = *deck;
    let lines = sim.game.lines();
    let bet = sim.game.credits_per_line();
    let t = time.elapsed_secs();
    let glow = 0.84 + 0.16 * (t * 3.5).sin().abs();
    let start_wait = matches!(show.awaiting, Some(Await::Start(_)));
    // Once a card is turned the flash stops; the chosen key stays light blue.
    let chosen = gamble.card.and(gamble.last.map(|(p, _)| p));
    let flashing = |cap: CabButton, r: Role| -> bool {
        if d.profile != Profile::Club || !d.plain() {
            return false;
        }
        match (d.mode, cap) {
            (DeckMode::Base, CabButton::Slot { row, col }) => d.offer && (row == 1 || col <= 1),
            (DeckMode::Gamble, _) => chosen.is_none() && matches!(r, Role::Pick(_)),
            _ => false,
        }
    };
    let face = |cap: CabButton| -> (&'static str, f32) {
        let r = role(cap, &d);
        if flashing(cap, r) {
            return ("key_blue", 1.0);
        }
        match (cap, r) {
            (CabButton::Spin, Role::Off | Role::Lattice) => ("spin_dim", 1.0),
            (CabButton::Spin, _) if start_wait && ((t * 2.0) as u32) % 2 == 1 => ("spin_dim", 1.0),
            (CabButton::Spin, _) if idle || waiting => ("spin_lit", glow),
            (CabButton::Spin, _) => ("spin_dim", 1.0),
            (_, Role::Pick(p)) if chosen == Some(p) => ("key_blue", 1.0),
            (_, Role::Bet(m) | Role::Credits(m)) if m == bet => ("key_white", 1.0),
            (_, Role::Bet(_) | Role::Credits(_)) => ("key_red", 1.0),
            (_, Role::Blank) => ("key_red", 0.45),
            (_, Role::Lines(n)) if n == lines => ("key_white", 1.0),
            (_, Role::Lines(_) | Role::Play) => ("key_blue", 1.0),
            (_, Role::Gamble) if can_gamble || (gambling && gamble.stake >= 2) => {
                ("key_purple", glow)
            }
            (_, Role::Gamble) => ("key_purple", 0.5),
            (_, Role::TakeWin) if can_gamble || gambling => ("key_gold", glow),
            (_, Role::TakeWin) => ("key_gold", 0.5),
            (_, Role::Pick(p)) if !is_colour(p) && !suits_open => ("key_dark", 1.0),
            (_, Role::Pick(p)) if is_red(p) => ("key_red", 1.0),
            (_, Role::Pick(_)) => ("key_blue", 1.0),
            (_, Role::Lattice) => ("card_back", 1.0),
            (_, Role::Letter(_) | Role::Off) => ("key_dark", 1.0),
        }
    };
    for (cap, mut image) in &mut caps {
        let (key, k) = face(cap.0);
        // A pack built before the purple key draws GAMBLE blue.
        let h = art.ui.get(key).or_else(|| {
            (key == "key_purple")
                .then(|| art.ui.get("key_blue"))
                .flatten()
        });
        if let Some(h) = h
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
        let r = role(label.0, &d);
        let (word, size) = match (label.0, r) {
            (CabButton::Spin, Role::Off | Role::Lattice) => (String::new(), 40.0),
            // Footage: START FEATURE while a feature waits, SPIN through Hold
            // & Spin, PLAY and REPEAT BET in turn at rest (club).
            (CabButton::Spin, _) if start_wait => ("START\nFEATURE".to_string(), 26.0),
            (CabButton::Spin, _) if show.hold_and_spin() => ("SPIN".to_string(), 40.0),
            (CabButton::Spin, _)
                if d.profile == Profile::Club
                    && d.mode == DeckMode::Base
                    && idle
                    && (d.since as u32) % 2 == 1 =>
            {
                ("REPEAT\nBET".to_string(), 28.0)
            }
            (CabButton::Spin, _) => ("PLAY".to_string(), 40.0),
            // The plain face of the flash carries no legend.
            (cap, r) if flashing(cap, r) => (String::new(), 19.0),
            // Top left is blank purple in the gamble.
            (_, Role::Gamble) if d.mode == DeckMode::Gamble => (String::new(), 19.0),
            (_, Role::Pick(p)) if !is_colour(p) && !suits_open => (String::new(), 38.0),
            (_, Role::Pick(p)) if !is_colour(p) => (pick_text(p).to_string(), 38.0),
            // The letters wipe in from the left, one every 0.1 s.
            (CabButton::Slot { col, .. }, Role::Letter(_))
                if d.mode == DeckMode::Spinning && d.since < 0.1 * f32::from(col) =>
            {
                (String::new(), 44.0)
            }
            (_, Role::Letter(_)) => (role_text(r, lines, bet), 44.0),
            (_, Role::Credits(_)) => (role_text(r, lines, bet), 40.0),
            (_, Role::Lines(_) | Role::Play) => (role_text(r, lines, bet), 15.0),
            _ => (role_text(r, lines, bet), 19.0),
        };
        if text.0 != word {
            text.0 = word;
        }
        if font.font_size != FontSize::Px(size) {
            font.font_size = FontSize::Px(size);
        }
        let suit = matches!(r, Role::Pick(p) if !is_colour(p));
        let want = if suit { &fonts.symbols } else { &fonts.text };
        let source = bevy::text::FontSource::Handle(want.clone());
        if font.font != source {
            font.font = source;
        }
        // Dark lettering on the white and gold caps, white on the others;
        // spelled letters glow blue on a dark key (footage).
        let ink = match (r, face(label.0).0) {
            (Role::Letter(_), _) => Color::srgb(0.35, 0.65, 1.0),
            (_, "key_white" | "key_gold") => Color::srgb(0.06, 0.08, 0.2),
            (_, "key_dark") => Color::srgb(0.45, 0.5, 0.62),
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
    menu: Res<GamesMenu>,
    deck: Res<DeckState>,
    mut meters: Query<(&CabMeter, &mut Text)>,
) {
    if !fidelity.0 {
        return;
    }
    let cents = denom.cents();
    // Cents a change to a larger denomination left over: money on CREDIT.
    let residue = sim.game.state().residue;
    // Credits count up with the win meter.
    let pending = rollup.to.saturating_sub(rollup.shown());
    // The club machines switch to credits every other second while a win is
    // offered and through the gamble, never during a play or a count (and
    // the standard forbids it then); the casino style stays in dollars.
    let in_credits = deck.in_credits();
    let amount = |credits: u64, cents: u64| {
        if in_credits {
            grouped(credits)
        } else {
            money(credits, cents)
        }
    };
    for (meter, mut text) in &mut meters {
        let value = match meter {
            // A feature under way, from its trigger spin on (the game is in it
            // before the screen shows it): CREDIT holds still while the
            // trigger's win counts on WIN (cabinet footage).
            CabMeter::Credit if show.in_feature() || !sim.game.is_idle() => amount(
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
                let credits = presentation.shown_credits.saturating_sub(pending.max(held));
                if in_credits || residue == 0 {
                    amount(credits, cents)
                } else {
                    money(credits.saturating_mul(cents).saturating_add(residue), 1)
                }
            }
            // After a denomination change BET still shows the last game's bet
            // until the next game (footage).
            CabMeter::Bet => match menu.last_bet_cents {
                Some(c) if !in_credits => money(c, 1),
                _ => amount(sim.game.total_bet(), cents),
            },
            CabMeter::Win if gamble.active => amount(gamble.stake, cents),
            CabMeter::Win => match (gamble.result, show.grand_win()) {
                (Some(w), _) => amount(w, cents),
                (None, Some(g)) => amount(g, cents),
                (None, None) => amount(rollup.shown(), cents),
            },
            CabMeter::Denom => denom_text(cents),
            CabMeter::MinBet if deck.profile == Profile::Club => {
                let per_dollar = (100 / cents.max(1)).max(1);
                format!(
                    "$1 = {per_dollar} Credit{}",
                    if per_dollar == 1 { "" } else { "s" }
                )
            }
            CabMeter::MinBet => format!("MIN BET {}", sim.game.lines()),
            CabMeter::Lines if sim.game.lines() == 1 => "1\nLINE".to_string(),
            CabMeter::Lines => format!("{}\nLINES", sim.game.lines()),
            CabMeter::Multiplier => show.caption(time.elapsed_secs()).unwrap_or_else(|| {
                let t = time.elapsed_secs();
                let at_rest = sim.game.is_idle()
                    && presentation.queue.is_empty()
                    && !rollup.active()
                    && !gamble.active;
                let per_line = sim.game.credits_per_line();
                let bet_line = match deck.profile {
                    Profile::Club if per_line == 1 => "1 credit per line".to_string(),
                    Profile::Club => format!("{per_line} credits per line"),
                    Profile::Casino => format!("Bet Multiplier {per_line}"),
                };
                let turn = ((t / 2.0) as u32).is_multiple_of(2);
                if !at_rest {
                    bet_line
                } else if gamble_open(&sim.game, cents, &show) {
                    // The offer while a win can be gambled.
                    "PLAY ON, GAMBLE or TAKE WIN".to_string()
                } else if let Some(lost) = gamble.ended {
                    // Footage: the gamble's end message in turn with the bet.
                    match (turn, lost) {
                        (true, true) => "Gamble Completed - Gamble Feature Lost".to_string(),
                        (true, false) => "Gamble Completed".to_string(),
                        (false, _) => bet_line,
                    }
                } else if sim.game.credits() < sim.game.total_bet() {
                    "Insert More Credits or Change Selection".to_string()
                } else if turn {
                    // Idle: the bet line and "Game Over - Play Now", two
                    // seconds each (footage).
                    bet_line
                } else {
                    "Game Over - Play Now".to_string()
                }
            }),
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
    /// Casino: a bet key's multiplier (sets the bet and plays).
    pub bet: Option<u32>,
    /// Club: a credits-per-line key (only selects).
    pub credits: Option<u32>,
    /// Club: a lines key (sets the lines and plays).
    pub lines: Option<u16>,
    /// The GAMES MENU tab was touched.
    pub menu: bool,
    /// A denomination chosen on the GAMES MENU, in cents.
    pub denom: Option<u64>,
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

/// The cabinet's pace, measured on 60 fps cabinet footage (wEmf3ep-euA, 8
/// spins; BMavaGxuOD0): reel 1 comes to rest about 0.95 s after the reels
/// start, so it is deepest past its stop about 0.66 s in; each further reel
/// 0.45 s after the one before. A reel's stop is presented here, and it runs
/// on about 3.5 symbols to its stop and on past it. (The official mobile app
/// is slower: deepest 24 frames after the press, reels 14 frames apart.)
pub const TIMING: Timing = Timing {
    lead_in: 0.66 - RUN_ON,
    reel_interval: 0.45,
    event_interval: 0.14,
};

/// Reel speed in symbols per second while spinning: the cabinet's 20
/// (two-thirds of a symbol a 30 fps frame, on two cabinets), reached at once,
/// constant to the stop. The mobile app spins at 15.
const SPIN_SPEED: f32 = 20.0;
/// A stopping reel runs on at full speed through its stop to about 0.65 of
/// a symbol past it for one refresh, comes back to 0.40 past it and slides
/// home in a straight line in 8 frames at 30 fps (cabinet footage, 12
/// landings: the frame after the deepest is always +0.40, and no frame shows
/// a whole symbol past). The mobile app instead dips a whole symbol, snaps to
/// 0.36 and slides in 9 frames; copied from it, that read as a jolt. The
/// clink is at the snap.
const DEEP: f32 = 0.65;
const OVERSHOOT: f32 = 0.40;
const SETTLE_SECONDS: f32 = 8.0 / 30.0;
/// The refresh the reel shows at its deepest before the snap (never 0: the
/// snap is what sounds the clink).
const DEEP_HOLD: f32 = 1.0 / 60.0;
/// Seconds from a reel's presented stop to its deepest point: the run-on of
/// 3 to 4 symbols and one more, at a speed adjusted a little so every reel
/// lands on the beat.
const RUN_ON: f32 = (3.5 + DEEP) / SPIN_SPEED;
/// Images per reel strip: one above the window, three in it, one below.
const STRIP_SLOTS: usize = 5;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Motion {
    #[default]
    Idle,
    /// Spinning, `t` seconds since the start.
    Spinning { t: f32 },
    /// Running on to `target` (an integer position) at `speed`, then
    /// bouncing back.
    Stopping {
        target: f32,
        speed: f32,
        bounce: Option<f32>,
    },
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
    /// Seconds since every reel came to rest.
    rested: f32,
}

/// The real game shows a spin's result 5 frames (0.17 s) after the last reel
/// is still, and takes the next spin 3 frames (0.1 s) after it (measured on
/// every base-game spin and win of the real game's session).
pub const RESULT_AFTER_REST: f32 = 0.17;
pub const SPIN_AFTER_REST: f32 = 0.1;

impl Reels {
    /// Whether a reel is moving (so its landed cells are hidden).
    pub fn moving(&self, reel: u8) -> bool {
        self.states
            .get(reel as usize)
            .is_some_and(|s| s.motion != Motion::Idle)
    }

    /// Whether any reel is in motion on screen: spinning, running on to its
    /// stop, or sliding back after the overshoot. A reel's stop is presented
    /// before it comes to rest (it runs on about 3 symbols, overshoots and
    /// settles, some 0.4 s), so this is later than the game's stop.
    pub fn any_moving(&self) -> bool {
        self.states.iter().any(|s| s.motion != Motion::Idle)
    }

    /// Seconds every reel has been at rest on screen (0 while any moves).
    pub fn rested_for(&self) -> f32 {
        if self.any_moving() { 0.0 } else { self.rested }
    }

    /// Reels that hit their stop this frame.
    pub fn just_stopped(&self) -> impl Iterator<Item = usize> + '_ {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, s)| s.stopped_now)
            .map(|(i, _)| i)
    }

    /// Put a resting reel's window back to `labels` (top row first), as a
    /// denomination's last window returns: the next spin starts from it.
    pub fn show_window(&mut self, reel: usize, labels: Vec<String>) {
        if let Some(s) = self.states.get_mut(reel)
            && s.motion == Motion::Idle
        {
            s.landing_at = s.pos.round() as i64;
            s.landing = labels;
        }
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
    Reels {
        states,
        strips,
        rested: SPIN_AFTER_REST,
    }
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
    images: Res<Assets<Image>>,
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
                let speed = (state.pos - (target - DEEP)) / RUN_ON;
                state.motion = Motion::Stopping {
                    target,
                    speed,
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
                speed,
                bounce: None,
            } => {
                state.pos -= speed * dt;
                if state.pos <= target - DEEP {
                    // At its deepest for a frame (the bounce clock counts
                    // from here, negative until the snap).
                    state.pos = target - DEEP;
                    state.motion = Motion::Stopping {
                        target,
                        speed,
                        bounce: Some(-DEEP_HOLD),
                    };
                }
            }
            Motion::Stopping {
                target,
                speed,
                bounce: Some(b),
            } => {
                let b0 = b;
                let b = b + dt;
                if b0 < 0.0 && b >= 0.0 {
                    // The snap back, and the clink.
                    state.stopped_now = true;
                }
                let k = (b / SETTLE_SECONDS).clamp(0.0, 1.0);
                // A steady slide back (real game: linear, frame by frame).
                state.pos = if b < 0.0 {
                    target - DEEP
                } else {
                    target - OVERSHOOT * (1.0 - k)
                };
                state.motion = if k >= 1.0 {
                    state.pos = target;
                    Motion::Idle
                } else {
                    Motion::Stopping {
                        target,
                        speed,
                        bounce: Some(b),
                    }
                };
            }
        }
    }

    if reels.any_moving() {
        reels.rested = 0.0;
    } else {
        reels.rested += dt;
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
        if let Some(handle) = symbol_image(&art, &key, time.elapsed_secs())
            && image.image != handle
        {
            image.image = handle;
        }
        // While a reel settles, the symbols just outside the window are cut at
        // their own cell's edge: at rest the landed cells draw only the three
        // in the window, so an overhang drawn now would vanish as it stops.
        let mut top = cy - h / 2.0;
        let mut height = h;
        let mut rect = None;
        if let Motion::Stopping {
            target,
            bounce: Some(_),
            ..
        } = reels.states[reel].motion
            && let Some(size) = images.get(&image.image).map(|i| i.size_f32())
        {
            let first = target as i64;
            if n < first {
                // Above the window: nothing below its cell's bottom.
                height = (cy + p / 2.0 - top).clamp(0.0, h);
                rect = Some(Rect::new(0.0, 0.0, size.x, size.y * height / h));
            } else if n >= first + 3 {
                // Below the window: nothing above its cell's top.
                let cut = (cy - p / 2.0 - top).clamp(0.0, h);
                top += cut;
                height = h - cut;
                rect = Some(Rect::new(0.0, size.y * cut / h, size.x, size.y));
            }
        }
        if image.rect != rect {
            image.rect = rect;
        }
        node.top = Val::Px(top);
        node.height = Val::Px(height);
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
        if let Some(handle) = symbol_image(&art, &key, time.elapsed_secs())
            && image.image != handle
        {
            image.image = handle;
        }
        node.top = Val::Px(cy - h / 2.0);
        node.height = Val::Px(h);
        node.width = Val::Px(w);
        node.left = Val::Px((column_width() - w) / 2.0);
    }
}

/// A symbol's image now: the coin's own burning loop (the game's 29-frame
/// Corona sequence, on every coin it draws), other symbols' still images.
pub fn symbol_image(art: &ArtPack, key: &str, t: f32) -> Option<Handle<Image>> {
    if key == "orb"
        && let Some(frames) = art.anims.get("coin")
        && !frames.is_empty()
    {
        return frames.get((t * 30.0) as usize % frames.len()).cloned();
    }
    art.symbols.get(key).cloned()
}

/// The motion smear: copies this far (in symbols) from each symbol, and how
/// strongly they show. None: the cabinet draws its moving symbols crisp
/// (60 fps footage); the doubled images in some recordings are the camera's
/// exposure across two refreshes, not the game.
const BLUR_OFFSETS: [f32; 0] = [];
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
    /// Reels of this spin that have landed coins (while six were reachable),
    /// and coins landed so far.
    coin_reels: u32,
    coins_seen: u32,
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

    /// A clip's length in seconds.
    pub fn seconds(&self, name: &str) -> Option<f32> {
        self.clips.get(name).map(|(_, s)| *s)
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
        sounds.coins_seen = 0;
    }
    sounds.reels_were_moving = moving;
    let stopped: Vec<usize> = reels.just_stopped().collect();
    let last = reels.states.len().saturating_sub(1);
    for reel in stopped {
        // Measured on cabinet footage (audio audit, 283 plain landings and
        // 80+ coin landings): every landing clinks, except a reel that lands
        // coins while six can still be reached (coins on screen after it +
        // 3 x reels still to land >= 6). That reel plays SND_JPDoonk{n}, n
        // the count of such coin reels this spin. Flags just clink.
        let coins = reels.states[reel]
            .landing
            .iter()
            .filter(|l| l.starts_with("ORB"))
            .count() as u32;
        sounds.coins_seen += coins;
        let reachable = sounds.coins_seen + 3 * (last - reel) as u32 >= 6;
        if coins > 0 && reachable {
            sounds.coin_reels += 1;
            let n = sounds.coin_reels.min(5);
            sounds.play(&mut commands, &format!("SND_JPDoonk{n}"));
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
///
/// The real game, recorded with sound: 0.4x the bet played wintune04, 0.6x
/// wintune05, 1.6x wintune13, 2x wintune14 (and a free game's 2.6x counted
/// for about wintune15's length). The rule between and beyond those points is
/// not known; tunes are interpolated through them (PARTIAL), reaching the
/// longest tune at 100x the bet.
pub fn win_tune(win: u64, total_bet: u64) -> u32 {
    if total_bet == 0 {
        return 1;
    }
    let ratio = win as f32 / total_bet as f32;
    // (win / bet, tune): the measured points, then ours to the top.
    const POINTS: [(f32, f32); 7] = [
        (0.0, 1.0),
        (0.4, 4.0),
        (0.6, 5.0),
        (1.6, 13.0),
        (2.0, 14.0),
        (2.6, 15.0),
        (100.0, 32.0),
    ];
    let mut idx = 32.0;
    for w in POINTS.windows(2) {
        let ((r0, i0), (r1, i1)) = (w[0], w[1]);
        if ratio <= r1 {
            // Past the measured points the tunes rise with the log of the win.
            let k = if r0 >= 2.6 {
                (ratio / r0).ln() / (r1 / r0).ln()
            } else {
                (ratio - r0) / (r1 - r0)
            };
            idx = i0 + (i1 - i0) * k.clamp(0.0, 1.0);
            break;
        }
    }
    (idx.round() as u32).clamp(1, 32)
}

/// How long each win tune is heard (wintune01..32), in seconds: the game's
/// win count lasts exactly this long, not the length of the sound file, whose
/// tail is near silence (measured on the real game: 0.4x counted 0.37 s to
/// wintune04, 1.6x 1.90 s to wintune13, 2x 2.4 s to wintune14; these are the
/// last 10 ms of each tune within 20 dB of its peak).
const TUNE_AUDIBLE: [f32; 32] = [
    0.16, 0.21, 0.29, 0.41, 0.58, 1.25, 0.62, 0.84, 0.94, 1.05, 2.12, 1.46, 1.86, 2.28, 4.09, 4.07,
    5.24, 6.47, 7.62, 9.15, 12.38, 14.72, 14.93, 18.62, 19.47, 25.70, 30.88, 35.92, 38.94, 45.24,
    52.33, 57.55,
];

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
        // A free game's win plays no tune; the feature music carries on
        // (real game). It counts up over the tune's length all the same.
        let free_game = matches!(show.stage, Stage::FreeGames { .. });
        if !free_game {
            rollup.tune = sounds.play(&mut commands, &name);
        }
        rollup.duration = TUNE_AUDIBLE[(tune as usize).clamp(1, 32) - 1];
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
    // GAMBLE during the count completes it and goes on into the gamble
    // (footage: WIN posts its value as the gamble opens).
    let gamble = input.gamble || keys.just_pressed(KeyCode::KeyG);
    if pressed || gamble {
        rollup.t = rollup.duration;
        if let Some(id) = rollup.tune.take() {
            commands.entity(id).try_despawn();
        }
    }
    if pressed {
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
    /// When the burst on the banner's counter began (`clock`).
    banner_burst: Option<f32>,
    /// The rumble before the coins are added, while it plays.
    rumble: Option<Entity>,
    /// Payout tracks played (the next is the next in turn).
    payout_track: usize,
    /// When the bursts on the WIN banner's letters began (`clock`).
    pub flares_at: Option<f32>,
    /// A full grid's Grand jackpot, while it is shown and after.
    pub grand: Option<u64>,
    grand_done: bool,
    /// This game won a progressive jackpot (the Major or the Grand).
    progressive: bool,
    /// Seconds "FREE SPINS COMPLETED" has shown, until the collection.
    completed_wait: Option<f32>,
    /// The WINNER total being paid onto WIN.
    pay: Option<Pay>,
    /// Paid onto WIN so far by the feature's end (coins, then the Grand).
    paid: u64,
    /// The rollup sound of a payment.
    pay_sound: Option<Entity>,
    /// Respins run so far (each starts a fresh stretch of strip).
    respins_run: i64,
    /// Sounds the presentation has cued, played by `feature_cues`.
    cues: Vec<&'static str>,
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
    /// The free games' trigger sequence: seconds since it began (see
    /// `free_games_trigger`), the flags waving, the time since START FEATURE
    /// was pressed (until the cut to the free games), and the banner's total
    /// counting up after the cut (seconds, total shown).
    fg_trigger: Option<f32>,
    fg_flags: bool,
    /// Seconds since a stack on a winning line began to stand (its win waits).
    pub wild: Option<f32>,
    /// A wild sound has played this game, and how many have played.
    wild_sounded: bool,
    wild_turn: usize,
    fg_cut: Option<f32>,
    fg_count: Option<(f32, u32)>,
}

impl FeatureShow {
    /// Follow an event as it is presented.
    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::SpinStarted { .. } => {
                self.flares_at = None;
                self.wild = None;
                self.wild_sounded = false;
                self.fg_trigger = None;
                self.fg_flags = false;
                self.fg_cut = None;
                self.grand = None;
                self.grand_done = false;
                self.progressive = false;
                self.win_lines.clear();
                self.scatter_paid = false;
                self.stage = Stage::Base;
                self.scatter_award = 0;
                self.collected = None;
                self.grid = false;
                self.winner = None;
                self.intro = None;
            }
            Event::JackpotAwarded { tier, award } => {
                self.progressive |= matches!(tier, JackpotTier::Major | JackpotTier::Grand);
                self.jackpots.push((*tier, *award));
            }
            Event::ScatterWin { award, .. } if !matches!(self.stage, Stage::FreeGames { .. }) => {
                self.scatter_award = *award;
                // The flags already wave when a trigger's win is shown.
                if self.win_lines.is_empty() && !self.fg_flags {
                    self.win_t = 0.0;
                }
                self.scatter_paid = true;
            }
            Event::FreeGamesStarted { games } => {
                // Free games won with Hold & Spin follow it: its board, total
                // and banner clear first.
                if matches!(self.stage, Stage::HoldSpinDone) {
                    self.stage = Stage::Base;
                    self.grid = false;
                    self.collected = None;
                    self.winner = None;
                    self.flares_at = None;
                    self.grand = None;
                    self.jackpots.clear();
                    self.counter = 0;
                    self.scatter_paid = false;
                }
                // The bell and the flags' sound have played already (the
                // trigger sequence, before the win was shown).
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
                // Each free game is a game of its own for the wild.
                self.wild = None;
                self.wild_sounded = false;
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
            Event::FreeGamesCompleted { .. } => {
                // Real game: "FEATURE COMPLETED" with SND_Fea_End, the music
                // cut at the same moment.
                self.cues.push("SND_Fea_End");
                self.stage = Stage::FreeGamesDone;
            }
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
                // Footage: SND_Fea_End with "FREE SPINS COMPLETED"; the music
                // stops at the same moment.
                self.cues.push("SND_Fea_End");
                self.awaiting = None;
                self.stage = Stage::HoldSpinDone;
                self.completed_wait = Some(0.0);
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
            (Some(Await::Respin), _) if self.respin.is_none() => Some("Press SPIN button".into()),
            (_, Stage::FreeGames { number, total }) => Some(format!(
                "Free Game {number} of {}",
                self.counter_total(total)
            )),
            _ => None,
        }
    }

    /// The message at the right, in place of the start prompts.
    pub fn message(&self) -> Option<String> {
        if let Some(note) = &self.win_note {
            return Some(note.clone());
        }
        // Footage: "Scatter wins N" from the flags' wave until START
        // FEATURE is pressed; blank through the free games.
        let waiting = matches!(self.awaiting, Some(Await::Start(Feature::FreeGames)));
        (waiting && self.scatter_award > 0).then(|| format!("Scatter wins {}", self.scatter_award))
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
    mut rollup: ResMut<Rollup>,
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
        // START FEATURE goes straight into the free games (cabinet footage:
        // the banner reads FREE GAME 1 OF n as the first game spins; the base
        // screen does not come back in between).
        if matches!(show.awaiting, Some(Await::Start(Feature::FreeGames))) {
            // The press is silent; the flags wave on until the cut to the
            // free games (real game: about a second later).
            rollup.t = rollup.duration;
            show.fg_cut = Some(0.0);
        }
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
        .then(|| rollup.shown().saturating_sub(rollup.from))
        .or(show.pay_banner());
    for (mut text, mut vis) in &mut amount {
        // The empty banner shows no amount until the first coin is added.
        let s = game_win
            .filter(|w| *w > 0)
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
        (_, Stage::FreeGames { .. } | Stage::HoldSpinDone) if game_win.is_some() => {
            Some("banner_win".to_string())
        }
        (_, Stage::HoldSpin { .. }) if show.intro.is_some() => None,
        // Between FREE SPINS COMPLETED and the WIN banner: no banner.
        (_, Stage::HoldSpinDone) if show.collect.is_some() => None,
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
            Some([number.to_string(), show.counter_total(total).to_string()])
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
    // The other positions are dark from the moment the coins burst (real
    // game: one frame after), before the board itself shows.
    let dark = show.hold_and_spin() && (show.grid || show.since_burst(1.0 / 30.0));
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
        let want = if dark && empty {
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

/// Seconds into the trigger sequence for each step (it begins when the
/// result shows, 0.17 s after the last reel is still). Real game, recorded
/// with sound and measured frame by frame (the sound's 0.09 s lag removed):
/// the feature bell as the reels settle; 2.8 s later every coin bursts into
/// flame and the other positions dim, to SND_FeatureTrigger; 0.2 s on the
/// fire dragons rise and fill the screen with fire, which clears to the HOLD &
/// SPIN medallion over the dimmed board; a second fire clears to the Hold &
/// Spin board; 0.57 s later SELECT SPIN TO START FEATURE and the Hold & Spin
/// music. The real game holds the medallion 3.8 s and then shows "COLLECT 15
/// TO WIN GRAND JACKPOT"; the owner wants the medallion for about a second
/// and no COLLECT screen, so the second fire follows the first at once.
const INTRO_BELL: f32 = 0.01;
const INTRO_BURST: f32 = 2.63;
const INTRO_TRANSITION: f32 = INTRO_BURST + 0.2;
/// The fire (54 frames) runs at 25 fps (2.15 s on the real game's screen).
const FIRE_FPS: f32 = 25.0;
/// The medallion stands from the first fire's turning see-through until the
/// second fire covers it.
const INTRO_MEDALLION: f32 = INTRO_TRANSITION + 1.4;
const INTRO_FIRE2: f32 = INTRO_TRANSITION + 2.0;
const INTRO_GRID: f32 = INTRO_FIRE2 + 0.9;
const INTRO_END: f32 = INTRO_FIRE2 + 1.87 + 0.57;
/// The counter's count-up after START FEATURE (real game: a SND_Fea_Inc tick
/// a step, the first 0.27 s after the press, then every 0.2 s; a burst on the
/// banner's 3), and the pause before the first respin, which starts by
/// itself 1.9 s after the count reaches 3 (the feature's step pause of 0.5 s
/// follows this).
const COUNT_FIRST: f32 = 0.27;
const COUNT_STEP: f32 = 0.2;
const COUNT_HOLD: f32 = 1.4;
/// A respin, from the real game (9 position stops measured frame by frame):
/// every open position of a reel stops together. The first reel passes its
/// stop 16 frames (0.53 s) after the press and each further reel 14 frames
/// (0.467 s) later, whether or not it has open positions; so a respin always
/// takes the same time.
const RESPIN_FIRST_PASS: f32 = 16.0 / 30.0;
const RESPIN_REEL_GAP: f32 = 14.0 / 30.0;
/// Positions per second a respinning position runs at (about half a cell a
/// frame), to the stop with no slowing.
const RESPIN_SPEED: f32 = 15.3;
/// How far past its stop a respinning position is, in cells, each frame
/// (30 fps) from the moment it passes the stop: a hard overshoot of half a
/// cell, then back over 11 frames (measured).
const RESPIN_BOUNCE: [f32; 12] = [
    0.0, 0.512, 0.36, 0.326, 0.285, 0.244, 0.209, 0.163, 0.116, 0.081, 0.047, 0.0,
];
const RESPIN_SETTLE: f32 = 11.0 / 30.0;
/// A coin that lands puts the counter back to three 6 frames after it passes
/// its stop, and bursts into flame the frame after (measured).
const RESPIN_RESET: f32 = 6.0 / 30.0;
const RESPIN_FLARE: f32 = 7.0 / 30.0;
/// A respin's sounds (real game, measured with the sound's lag removed):
/// every reel with positions spinning clinks at the bottom of their
/// overshoot, 2 frames after they pass their stop; a new coin sounds
/// SND_HandSDoonk1 a little after its reel's clink, and the first coin of the
/// respin, the one that puts the counter back to three, SND_GrandStrike
/// 0.07 s after that. A reel of coins only, with nothing spinning, is silent.
const RESPIN_CLINK: f32 = 2.0 / 30.0;
const RESPIN_DOONK: f32 = RESPIN_CLINK + 0.06;
const RESPIN_STRIKE: f32 = RESPIN_DOONK + 0.07;
/// The respin is over 4 to 5 frames after the last position settles.
const RESPIN_DONE: f32 = 4.5 / 30.0;
/// Frames per second of the game's animations.
const ANIM_FPS: f32 = 30.0;
/// Symbols an open position shows while it respins.
const RESPIN_SYMBOLS: [&str; 12] = [
    "9", "10", "J", "Q", "K", "vase", "chest", "women", "warriors", "emperor", "pennant", "orb",
];
/// The tint of a dimmed symbol on an open Hold & Spin position.
pub const DIM_TINT: Color = Color::srgb(0.62, 0.62, 0.68);
/// The slate under the whole Hold & Spin grid.
const HS_BACKDROP: Color = Color::srgb(0.035, 0.04, 0.075);
/// The veil over an open Hold & Spin position: near black with a hint of
/// navy, the symbol only just showing (cabinet photo: about RGB 10,12,20).
const HS_VEIL: Color = Color::srgba(0.03, 0.035, 0.07, 0.965);

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
    /// Stops already made, and coins landed so far in this respin.
    done: usize,
    coins: u32,
    /// Landed coins still to burst into flame: when, and where.
    flares: Vec<(f32, u8, u8)>,
    /// When the counter goes back to three.
    resets: Vec<f32>,
    /// Sounds still to play: when, and which.
    sounds: Vec<(f32, &'static str)>,
}

/// The coins being added to the WIN banner, then the Grand.
#[derive(Clone, Debug, Default)]
pub struct Collect {
    t: f32,
    /// Each step's coin position, amount and whether it is a jackpot, in
    /// order; the Grand, last, has no position (`GRAND_STEP`).
    coins: Vec<(u8, u8, u64, bool)>,
    /// Steps begun, and amounts added to the banner.
    launched: usize,
    counted: usize,
    /// The Grand's step has begun (its plaque shows).
    grand_up: bool,
}

/// The position of the Grand's step (it has no coin).
const GRAND_STEP: u8 = u8::MAX;

/// The end of Hold & Spin, from the real game recorded with sound (the
/// sound's lag removed): "FREE SPINS COMPLETED" with SND_Fea_End (3 s); a
/// rumble (SND_Rumble) from 0.37 s after; an empty WIN banner over the reels
/// at 2.86 s; then each coin in turn, reel by reel and top to bottom, 1.35 s
/// apart: it flares and its fire comet (the game's bolt animations) flies to
/// the banner, which adds its value at once, to SND_Small_5 (SND_Medium_1 for
/// a big coin); the rumble fades over the first second of this. 0.8 s after
/// the last value the banner's WIN letters burst (again every 59 frames until
/// the next game) and the count begins: the payout music, a coin shower,
/// and WIN counting the total up over the music's length (SPIN completes
/// it; the music stops). Our cabinet keeps its own meters; a Grand is added
/// to the banner after the coins, its plaque on the top while it is.
const COMPLETED_SECONDS: f32 = 3.0;
const COLLECT_RUMBLE: f32 = 0.37;
const COLLECT_BANNER: f32 = 2.86;
const COLLECT_FIRST: f32 = COLLECT_BANNER + 1.0 / 30.0;
const COLLECT_STEP: f32 = 1.347;
/// From a coin's flare to its value on the banner (the comet's flight).
const BOLT_FLIGHT: f32 = 0.45;
/// From the last value to the count, and to the first burst on WIN, and
/// the bursts' period.
const PAY_DELAY: f32 = 0.87;
const FLARE_DELAY: f32 = 0.8;
const FLARE_EVERY: f32 = 59.0 / 30.0;
/// The rumble fades out over this long from the first coin.
const RUMBLE_FADE: f32 = 1.0;
/// A coin worth at least this many total bets sounds SND_Medium_1 (real
/// game: a 10x coin did, 1x to 4x coins sounded SND_Small_5; the boundary
/// between is not known).
const BIG_COIN_RATIO: u64 = 5;
const PAY_AFTER: f32 = 0.4;
/// A payment at least this many times the bet rains coins to the payout
/// music (real game: 28x and up did; on the cabinet 22x did not, and counted
/// to wintune32 instead).
const PAY_SHOWER_RATIO: u64 = 25;
/// The payout music (the app's shared sound bank), in the order the real
/// game played it: a cabinet session's showers ran b, a, c, b; the app's
/// payout played b.
const PAYOUT_MUSIC: [&str; 3] = ["payout_music_b", "payout_music_a", "payout_music_c"];
const PAY_TUNE: &str = "wintune32";

/// The feature's total being counted onto WIN: the amount, the seconds since
/// the count began, and its length (the music's).
#[derive(Clone, Debug)]
pub struct Pay {
    amount: u64,
    t: f32,
    seconds: f32,
}

impl FeatureShow {
    /// The Grand's plaque shows (from its step until the count is over).
    pub fn grand_showing(&self) -> bool {
        self.grand.is_some()
            && (self.collect.as_ref().is_some_and(|c| c.grand_up) || self.pay.is_some())
    }

    /// What WIN shows while the feature's total is counted onto it.
    pub fn grand_win(&self) -> Option<u64> {
        let p = self.pay.as_ref()?;
        let k = (p.t / p.seconds).clamp(0.0, 1.0);
        Some(self.paid + (p.amount as f32 * k) as u64)
    }

    /// What the WIN banner over the reels shows: the coins added so far,
    /// then the total while it counts and after, until the next game.
    pub fn pay_banner(&self) -> Option<u64> {
        if let Some(p) = &self.pay {
            return Some(self.paid + p.amount);
        }
        if self.collect.is_some() {
            return self.winner;
        }
        (self.stage == Stage::HoldSpinDone && self.collected.is_some()).then_some(self.paid)
    }

    /// A big payment is counting (coins rain).
    pub fn pay_shower(&self, bet: u64) -> bool {
        self.pay
            .as_ref()
            .is_some_and(|p| p.t < p.seconds && p.amount >= PAY_SHOWER_RATIO * bet)
    }

    /// Seconds into the coins' collection.
    fn collect_t(&self) -> Option<f32> {
        self.collect.as_ref().map(|c| c.t)
    }

    /// The game's win may not be gambled: it won a progressive jackpot (the
    /// national standard allows no gamble after one).
    pub fn no_gamble(&self) -> bool {
        self.grand.is_some() || self.progressive
    }
}

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
    // The comet flies to the WIN banner over the reels.
    let (tx, ty) = (GRID_CENTER.0, BANNER_Y);
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
const WINNER_TOP: f32 = 33.0;
/// The WINNER total's flame box.
const WINNER_BOX: (f32, f32) = (440.0, 80.0);
/// Where the splash animation (512 x 256) draws its box: x 40..472,
/// y 145..223 (measured on its frames).
const SPLASH_BOX: (f32, f32, f32, f32) = (40.0, 145.0, 432.0, 78.0);

/// The WINNER plate's height: clear of the banner slot at the top of the
/// reels (the WIN banner counts there while WINNER shows).
const WINNER_PLATE_H: f32 = 114.0;

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
        let want = if show.grand_showing() && !gamble.active {
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
            .filter(|s| self.t < s.2 + RESPIN_SETTLE)?;
        let end = self.end_of(at);
        // Full speed to the stop, past it by half a cell, and back.
        let pos = if self.t < at {
            end as f32 - (at - self.t) * RESPIN_SPEED
        } else {
            let f = (self.t - at) * 30.0;
            let i = (f as usize).min(RESPIN_BOUNCE.len() - 1);
            let next = RESPIN_BOUNCE[(i + 1).min(RESPIN_BOUNCE.len() - 1)];
            end as f32 + RESPIN_BOUNCE[i] + (next - RESPIN_BOUNCE[i]) * f.fract()
        };
        Some((pos, end, coin))
    }

    /// Symbols a position runs through before it passes its stop at `at`.
    fn travel(at: f32) -> i64 {
        ((at * RESPIN_SPEED).round() as i64).max(3)
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
        centred(ow * 1.6, oh * 1.6),
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
                font_size: FontSize::Px(32.0),
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
                font_size: FontSize::Px(54.0),
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
                .any(|(r, w, _, _)| *r == reel && *w == row),
            None => self.winner.is_some(),
        }
    }

    /// The presentation holds its next event while the trigger sequence,
    /// the counter, a respin or the count is playing.
    pub fn blocks(&self) -> bool {
        self.intro.is_some()
            || self.fg_trigger.is_some_and(|t| t < FG_COUNT)
            || self.wild.is_some_and(|t| t < WILD_HOLD)
            || self.fg_cut.is_some()
            || self.completed_wait.is_some()
            || self.pay.is_some()
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
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<CabinetInput>,
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
        if crossed(t0, t, INTRO_TRANSITION) || crossed(t0, t, INTRO_FIRE2) {
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
        let step_at = |n: u8| COUNT_FIRST + f32::from(n) * COUNT_STEP;
        while show.counter < target && t >= step_at(show.counter) {
            show.counter += 1;
            sounds.play(&mut commands, "SND_Fea_Inc");
            if show.counter == target {
                show.banner_burst = Some(clock);
            }
        }
        if show.counter >= target && t >= step_at(target.saturating_sub(1)) + COUNT_HOLD {
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
                    let pass = RESPIN_FIRST_PASS + f32::from(reel) * RESPIN_REEL_GAP;
                    stops.push((reel, row, pass, new));
                }
            }
        }
        show.respins_run += 1;
        sounds.play(&mut commands, "SND_Button");
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
            // The reel's clink, once for its positions.
            let first_of_reel = anim.stops[..anim.done].iter().all(|s| s.0 != reel);
            if first_of_reel {
                anim.sounds.push((at + RESPIN_CLINK, "reel_clink1"));
            }
            if coin {
                anim.coins += 1;
                anim.sounds.push((at + RESPIN_DOONK, "SND_HandSDoonk1"));
                if anim.coins == 1 {
                    anim.sounds.push((at + RESPIN_STRIKE, "SND_GrandStrike"));
                }
                anim.resets.push(at + RESPIN_RESET);
                anim.flares.push((at + RESPIN_FLARE, reel, row));
            } else {
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
        // A landed coin puts the counter back at three and bursts into
        // flame, while the respin is still settling (real game).
        let now = anim.t;
        for (_, name) in anim.sounds.iter().filter(|s| s.0 <= now) {
            sounds.play(&mut commands, name);
        }
        anim.sounds.retain(|s| s.0 > now);
        if anim.resets.iter().any(|&r| r <= now) {
            anim.resets.retain(|&r| r > now);
            show.counter = show.counter.max(show.pending_left);
        }
        let due: Vec<(u8, u8)> = anim
            .flares
            .iter()
            .filter(|f| f.0 <= now)
            .map(|f| (f.1, f.2))
            .collect();
        if !due.is_empty() {
            anim.flares.retain(|f| f.0 > now);
            for (reel, row) in due {
                show.bursts.push((reel, row, clock));
            }
        }
        let last = anim.stops.iter().map(|s| s.2).fold(0.0, f32::max);
        if anim.done >= anim.stops.len()
            && anim.flares.is_empty()
            && anim.sounds.is_empty()
            && anim.t >= last + RESPIN_SETTLE + RESPIN_DONE
        {
            finished = true;
        } else {
            show.respin = Some(anim);
        }
    }
    if finished {
        show.counter = show.pending_left;
    }

    if let Some(w) = show.completed_wait {
        let w = w + dt;
        if w >= COMPLETED_SECONDS && show.respin.is_none() {
            show.completed_wait = None;
            show.collect_start = true;
        } else {
            show.completed_wait = Some(w);
        }
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
                coins.push((reel, row, amount, face_credits(face, total_bet).is_none()));
            }
        }
        // A full grid's Grand jackpot is added after the coins.
        let grand = jackpots
            .iter()
            .position(|(tier, _)| *tier == JackpotTier::Grand)
            .map(|i| jackpots.remove(i).1);
        if let Some(g) = grand {
            coins.push((GRAND_STEP, GRAND_STEP, g, true));
        }
        show.grand = grand;
        show.winner = None;
        show.paid = 0;
        show.flares_at = None;
        show.collect = Some(Collect { coins, ..default() });
    }
    let mut done = None;
    if let Some(mut c) = show.collect.take() {
        let t0 = c.t;
        c.t += dt;
        let crossed = |at: f32| t0 < at && c.t >= at;
        if crossed(COLLECT_RUMBLE) {
            show.rumble = sounds.play(&mut commands, "SND_Rumble");
        }
        // The empty WIN banner.
        if crossed(COLLECT_BANNER) {
            show.winner = Some(0);
        }
        // Each coin in turn flares and sends its comet to the banner.
        while c.launched < c.coins.len() && c.t >= COLLECT_FIRST + c.launched as f32 * COLLECT_STEP
        {
            let (reel, row, amount, jackpot) = c.coins[c.launched];
            if reel == GRAND_STEP {
                c.grand_up = true;
            } else {
                show.bolt = Some((reel, row, clock));
            }
            let name = if jackpot {
                "SND_Credit1"
            } else if amount >= BIG_COIN_RATIO * total_bet {
                "SND_Medium_1"
            } else {
                "SND_Small_5"
            };
            sounds.play(&mut commands, name);
            c.launched += 1;
        }
        // Its value on the banner as the comet arrives.
        while c.counted < c.launched
            && c.t >= COLLECT_FIRST + c.counted as f32 * COLLECT_STEP + BOLT_FLIGHT
        {
            let (_, _, amount, _) = c.coins[c.counted];
            show.winner = Some(show.winner.unwrap_or(0) + amount);
            c.counted += 1;
        }
        let last =
            COLLECT_FIRST + c.coins.len().saturating_sub(1) as f32 * COLLECT_STEP + BOLT_FLIGHT;
        if c.counted >= c.coins.len() && c.t >= last + FLARE_DELAY && show.flares_at.is_none() {
            show.flares_at = Some(clock);
        }
        if c.counted >= c.coins.len() && c.t >= last + PAY_DELAY {
            done = show.winner;
            if let Some(id) = show.rumble.take() {
                commands.entity(id).try_despawn();
            }
        } else {
            show.collect = Some(c);
        }
    }
    // The count: the payout music (or, below the shower, the long win tune)
    // while WIN counts the total up over its length.
    if let Some(total) = done {
        let shower = total >= PAY_SHOWER_RATIO * total_bet;
        let name = if shower {
            let n = PAYOUT_MUSIC[show.payout_track % PAYOUT_MUSIC.len()];
            show.payout_track += 1;
            n
        } else {
            PAY_TUNE
        };
        show.pay_sound = sounds.play(&mut commands, name);
        show.pay = Some(Pay {
            amount: total,
            t: 0.0,
            seconds: sounds.seconds(name).unwrap_or(60.0).max(1.0),
        });
        done = None;
    }
    let slam = input.spin
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || (smoke.frames_left.is_some()
            && show
                .pay
                .as_ref()
                .is_some_and(|p| p.t > 3.0 && p.t < p.seconds));
    if show.pay.is_some() && slam {
        input.skip_spin = true;
    }
    if let Some(mut p) = show.pay.take() {
        p.t += dt;
        // A press completes the count (once) and stops the music.
        if p.t < p.seconds && slam {
            p.t = p.seconds;
        }
        if p.t >= p.seconds
            && let Some(id) = show.pay_sound.take()
        {
            commands.entity(id).try_despawn();
        }
        if p.t >= p.seconds + PAY_AFTER {
            show.paid += p.amount;
            show.grand_done = true;
            done = Some(show.paid);
        } else {
            show.pay = Some(p);
        }
    }
    if let Some(total) = done {
        show.winner = None;
        show.collected = Some(total);
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
        (&mut ImageNode, &mut Node, &mut Visibility),
        (
            With<WinnerBox>,
            Without<HsSpin>,
            Without<HsSpinImage>,
            Without<HsRing>,
            Without<HsBurst>,
            Without<HsTransition>,
        ),
    >,
    mut winner_text: Query<
        (&mut Text, &mut TextColor, &mut TextFont, &mut Node),
        (
            With<WinnerText>,
            Without<WinnerBox>,
            Without<HsSpinImage>,
            Without<HsBolt>,
        ),
    >,
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
            // Full size; the position clips what runs past its edges.
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
    // The coins burn in their rings once their burst has opened (real game:
    // 0.2 s after it), through the medallion and the board.
    let hs = show.hold_and_spin() && (show.grid || show.since_burst(0.2));
    for (r, mut image, mut vis) in &mut rings {
        let held = hs
            && !show.fire_taken(r.reel, r.row)
            && !open(label(r.reel, r.row))
            && spinning(r.reel, r.row).is_none()
            && !label(r.reel, r.row).is_empty();
        // Every held coin's flame runs in step (real game).
        let f = if held {
            frame("ring", clock, true)
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
            .and_then(|s| frame("transition", (clock - s) * FIRE_FPS / ANIM_FPS, false))
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

    // The WINNER total; on a Grand, the GRAND JACKPOT plaque (its own
    // 729 x 108 shape, the title across its top, the amount below).
    let grand = show.grand_showing();
    for (mut image, mut node, mut vis) in &mut winner {
        // The gamble shows the jackpots, not the Grand.
        let winner = show.grand.filter(|_| grand && !gamble.active);
        let key = art.ui.get("grand_plaque").cloned();
        let (w, h) = WINNER_BOX;
        let w = if grand { h * 729.0 / 108.0 } else { w };
        if node.width != Val::Px(w) {
            node.width = Val::Px(w);
            node.left = Val::Px(960.0 - w / 2.0);
        }
        match winner.and(key) {
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
        .grand
        .filter(|_| grand && !gamble.active)
        .map(|w| money(w, denom.cents()))
        .unwrap_or_default();
    let red = grand;
    for (mut t, mut color, mut font, mut node) in &mut winner_text {
        if t.0 != amount {
            t.0 = amount.clone();
        }
        // On the plaque the amount sits in the band under GRAND JACKPOT.
        let (size, drop) = if red {
            (WINNER_BOX.1 * 0.5, WINNER_BOX.1 * 0.17)
        } else {
            (54.0, 0.0)
        };
        if font.font_size != FontSize::Px(size) {
            font.font_size = FontSize::Px(size);
            node.top = Val::Px(drop);
        }
        let want = if red {
            Color::srgb(0.95, 0.12, 0.1)
        } else {
            Color::srgb(0.86, 0.93, 0.36)
        };
        if color.0 != want {
            color.0 = want;
        }
    }
}

// ---------------------------------------------------------------------------
// Free games: the doors reveal and the win banner
// ---------------------------------------------------------------------------
//
// Real game (all six free games of a recorded feature, frame by frame and
// with sound): the doors land with the reels; 3 frames after the last reel is
// still every door swings open together (the game's own 42-frame sequence at
// 30 fps, 1.4 s) on the symbol they reveal, to SND_EndMask; the game's wins
// show 47 frames after the doors begin to open (or FEATURE COMPLETED, after
// the last game).

/// Seconds after the reveal is presented that the doors open (it is
/// presented when they open), their frame rate, and the seconds from their
/// opening to the game's wins.
const REVEAL_OPEN: f32 = 0.0;
const REVEAL_FPS: f32 = 30.0;
const REVEAL_WINS: f32 = 47.0 / 30.0;
/// The reveal is presented this long after the last reel is still.
pub const REVEAL_AFTER_REST: f32 = 3.0 / 30.0;

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
    sounds: Res<Sounds>,
    mut commands: Commands,
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
    if t0 <= REVEAL_OPEN && t > REVEAL_OPEN {
        sounds.play(&mut commands, "SND_EndMask");
        for (l, mut text) in &mut labels {
            if cells.contains(&(l.reel, l.row)) {
                text.0 = symbol.clone();
            }
        }
    }
    // The doors open, then the wins follow (the reveal holds them).
    if t < REVEAL_OPEN + REVEAL_WINS {
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
// Games menu
// ---------------------------------------------------------------------------
//
// From cabinet footage of Golden Century's casino presentation: touching the
// GAMES MENU tab replaces the game screen with the game's title splash and a
// gold-framed panel, "CHOOSE YOUR DENOMINATION" alternating every 2 s with
// "TOUCH SCREEN TO MAKE SELECTION", over one row of touch buttons 1c 2c 5c
// 10c $1 $2 (the cents orange, the dollars yellow). BET, WIN and the tabs go;
// CREDIT stays. One touch chooses: the panel goes at once, the splash stays
// about 0.3 s, then the game returns at the new denomination with the bet
// multiplier back to 1, its lines, and the reels as that denomination left
// them. The club machines offer 1c to $1 and ask "<denomination> GAME / ARE
// YOU SURE?" (YES at the left, NO at the right) over the row; the casino
// style takes the touch at once. Only this game is offered. The cabinet's splash shows the emperor before the
// Great Wall with the title logo; the mobile build has neither, so the game's
// own Great Wall backdrop carries a drawn title here.

/// Seconds the splash stays after a denomination is chosen.
const MENU_CLOSE: f32 = 0.3;
/// Seconds each of the chooser's two headers shows.
const MENU_HEADER: f32 = 2.0;

/// The GAMES MENU.
#[derive(Resource, Default)]
pub struct GamesMenu {
    pub open: bool,
    /// Seconds the splash still shows after a choice.
    closing: f32,
    /// Seconds since it opened.
    since: f32,
    /// A denomination change still to show on the reels: old and new cents.
    pub changed: Option<(u64, u64)>,
    /// The window each denomination last showed, by cents.
    windows: std::collections::HashMap<u64, Vec<String>>,
    /// The last game's bet in cents: after a change the BET meter still shows
    /// it until the next game (footage).
    pub last_bet_cents: Option<u64>,
    /// Club: the denomination touched, waiting on ARE YOU SURE?.
    pub pending: Option<u64>,
}

impl GamesMenu {
    /// Open the chooser.
    pub fn open_now(&mut self) {
        self.open = true;
        self.closing = 0.0;
        self.since = 0.0;
        self.pending = None;
    }

    /// A denomination was touched: the panel goes, the splash stays a moment.
    pub fn close(&mut self) {
        self.open = false;
        self.closing = MENU_CLOSE;
        self.pending = None;
    }

    /// Whether the chooser or its closing splash covers the game.
    pub fn showing(&self) -> bool {
        self.open || self.closing > 0.0
    }
}

/// The chooser's parts.
#[derive(Component)]
pub struct ChooserRoot;
#[derive(Component)]
pub struct ChooserPanel;
#[derive(Component)]
pub struct ChooserHeader;
/// Covers BET, WIN and the tabs on the meter band while the chooser shows.
#[derive(Component)]
pub struct ChooserCover;
/// Club: the ARE YOU SURE? panel, and its question.
#[derive(Component)]
pub struct ChooserConfirm;
#[derive(Component)]
pub struct ChooserQuestion;

/// A denomination as the chooser and the badge write it.
pub fn denom_text(cents: u64) -> String {
    if cents >= 100 {
        format!("${}", cents / 100)
    } else {
        format!("{cents}¢")
    }
}

/// Build the GAMES MENU (hidden until touched).
fn spawn_games_menu(
    c: &mut ChildSpawnerCommands,
    art: &ArtPack,
    fonts: &CabFonts,
    profile: Profile,
) {
    let top = JACKPOT_BAR_TOP + JACKPOT_BAR_W * 104.0 / 1225.0 + 4.0;
    let h = BAND_TOP - top;
    c.spawn((
        ChooserRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(top),
            width: Val::Px(CANVAS_W),
            height: Val::Px(h),
            overflow: Overflow::clip(),
            ..default()
        },
        GlobalZIndex(12),
        Visibility::Hidden,
    ))
    .with_children(|m| {
        m.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                image: art.ui.get("background").cloned().unwrap_or_default(),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        // The title (drawn: the cabinet's logo is not in the mobile build).
        m.spawn((
            Text::new("GOLDEN CENTURY"),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                font_size: FontSize::Px(150.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.82, 0.25)),
            TextLayout::justify(Justify::Center),
            TextShadow {
                offset: Vec2::new(4.0, 6.0),
                color: Color::srgba(0.35, 0.05, 0.0, 0.9),
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(h * 0.16),
                width: Val::Percent(100.0),
                ..default()
            },
        ));
        // The panel: header over the row of denominations.
        let (pw, ph) = (1360.0, 250.0);
        m.spawn((
            ChooserPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((CANVAS_W - pw) / 2.0),
                top: Val::Px(h * 0.6),
                width: Val::Px(pw),
                height: Val::Px(ph),
                border: UiRect::all(Val::Px(5.0)),
                border_radius: BorderRadius::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.02, 0.02, 0.86)),
            BorderColor::all(Color::srgb(0.9, 0.68, 0.22)),
        ))
        .with_children(|p| {
            p.spawn((
                ChooserHeader,
                Text::new("CHOOSE YOUR DENOMINATION"),
                TextFont {
                    font: bevy::text::FontSource::Handle(fonts.sans.clone()),
                    font_size: FontSize::Px(40.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.55)),
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(26.0),
                ..default()
            })
            .with_children(|row| {
                for &cents in profile.denoms() {
                    let face = if cents >= 100 {
                        Color::srgb(1.0, 0.8, 0.22)
                    } else {
                        Color::srgb(0.96, 0.5, 0.1)
                    };
                    row.spawn((
                        Node {
                            width: Val::Px(186.0),
                            height: Val::Px(112.0),
                            border: UiRect::all(Val::Px(3.0)),
                            border_radius: BorderRadius::all(Val::Px(16.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(face),
                        BorderColor::all(Color::srgb(0.55, 0.3, 0.05)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(denom_text(cents)),
                            TextFont {
                                font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                                font_size: FontSize::Px(68.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.95, 1.0)),
                            TextShadow {
                                offset: Vec2::new(2.0, 3.0),
                                color: Color::srgba(0.25, 0.08, 0.0, 0.85),
                            },
                        ));
                    })
                    .observe(
                        move |_press: On<PointerPress>,
                              mut input: ResMut<CabinetInput>,
                              mut menu: ResMut<GamesMenu>| {
                            if !menu.open || menu.pending.is_some() {
                                return;
                            }
                            if profile.confirms() {
                                menu.pending = Some(cents);
                            } else {
                                input.denom = Some(cents);
                            }
                        },
                    );
                }
            });
        });
        // Club: "<denomination> GAME / ARE YOU SURE?", centred over the row.
        let (cw, ch) = (330.0, 190.0);
        m.spawn((
            ChooserConfirm,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((CANVAS_W - cw) / 2.0),
                top: Val::Px(h * 0.6 - ch + 40.0),
                width: Val::Px(cw),
                height: Val::Px(ch),
                border: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.02, 0.02, 0.95)),
            BorderColor::all(Color::srgb(0.9, 0.68, 0.22)),
            GlobalZIndex(13),
            Visibility::Hidden,
        ))
        .with_children(|q| {
            q.spawn((
                ChooserQuestion,
                Text::new(""),
                TextFont {
                    font: bevy::text::FontSource::Handle(fonts.sans.clone()),
                    font_size: FontSize::Px(30.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.55)),
                TextLayout::justify(Justify::Center),
            ));
            q.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(30.0),
                ..default()
            })
            .with_children(|row| {
                for yes in [true, false] {
                    row.spawn((
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(62.0),
                            border: UiRect::all(Val::Px(3.0)),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.96, 0.5, 0.1)),
                        BorderColor::all(Color::srgb(0.55, 0.3, 0.05)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(if yes { "YES" } else { "NO" }),
                            TextFont {
                                font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                                font_size: FontSize::Px(40.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        ));
                    })
                    .observe(
                        move |_press: On<PointerPress>,
                              mut input: ResMut<CabinetInput>,
                              mut menu: ResMut<GamesMenu>| {
                            let chosen = menu.pending.take();
                            if yes {
                                input.denom = chosen;
                            }
                        },
                    );
                }
            });
        });
    });
    // BET, WIN and the tabs give way; CREDIT stays, and on the club machines
    // COLLECT too.
    let cover_right = if profile == Profile::Club {
        1740.0
    } else {
        CANVAS_W
    };
    c.spawn((
        ChooserCover,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(780.0),
            top: Val::Px(BAND_TOP),
            width: Val::Px(cover_right - 780.0),
            height: Val::Px(SCREEN_BOTTOM - BAND_TOP),
            ..default()
        },
        BackgroundColor(Color::srgb(0.015, 0.01, 0.03)),
        GlobalZIndex(12),
        Visibility::Hidden,
    ));
}

/// Run the GAMES MENU: show or hide it, alternate its header, end the
/// closing splash, and put back the reels a denomination last showed.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn games_menu(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut menu: ResMut<GamesMenu>,
    mut reels: ResMut<Reels>,
    mut cells: Query<(&CellLabel, &mut Text), (Without<ChooserHeader>, Without<ChooserQuestion>)>,
    mut header: Query<&mut Text, (With<ChooserHeader>, Without<ChooserQuestion>)>,
    mut question: Query<
        &mut Text,
        (
            With<ChooserQuestion>,
            Without<ChooserHeader>,
            Without<CellLabel>,
        ),
    >,
    mut confirm: Query<
        &mut Visibility,
        (
            With<ChooserConfirm>,
            Without<ChooserRoot>,
            Without<ChooserCover>,
            Without<ChooserPanel>,
        ),
    >,
    mut covers: Query<&mut Visibility, Or<(With<ChooserRoot>, With<ChooserCover>)>>,
    mut panel: Query<
        &mut Visibility,
        (
            With<ChooserPanel>,
            Without<ChooserRoot>,
            Without<ChooserCover>,
        ),
    >,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time).min(0.05);
    if menu.open {
        menu.since += dt;
    }
    menu.closing = (menu.closing - dt).max(0.0);
    let vis = |on: bool| {
        if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    let showing = menu.showing();
    for mut v in &mut covers {
        if *v != vis(showing) {
            *v = vis(showing);
        }
    }
    for mut v in &mut panel {
        if *v != vis(menu.open) {
            *v = vis(menu.open);
        }
    }
    let line = if ((menu.since / MENU_HEADER) as u32).is_multiple_of(2) {
        "CHOOSE YOUR DENOMINATION"
    } else {
        "TOUCH SCREEN TO MAKE SELECTION"
    };
    for mut t in &mut header {
        if t.0 != line {
            t.0 = line.to_string();
        }
    }
    let asking = menu.open && menu.pending.is_some();
    for mut v in &mut confirm {
        if *v != vis(asking) {
            *v = vis(asking);
        }
    }
    if let Some(cents) = menu.pending {
        let q = format!("{} GAME\nARE YOU SURE?", denom_text(cents));
        for mut t in &mut question {
            if t.0 != q {
                t.0.clone_from(&q);
            }
        }
    }
    // Each denomination comes back with the reels it left (footage).
    if let Some((old, new)) = menu.changed.take() {
        let mut window = vec![String::new(); cells.iter().count()];
        let rows = 3usize;
        for (label, text) in &cells {
            if let Some(slot) = window.get_mut(label.reel as usize * rows + label.row as usize) {
                *slot = text.0.clone();
            }
        }
        menu.windows.insert(old, window);
        if let Some(saved) = menu.windows.get(&new).cloned() {
            for (label, mut text) in &mut cells {
                if let Some(s) = saved.get(label.reel as usize * rows + label.row as usize)
                    && text.0 != *s
                {
                    text.0.clone_from(s);
                }
            }
            for reel in 0..saved.len() / rows {
                reels.show_window(reel, saved[reel * rows..(reel + 1) * rows].to_vec());
            }
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
/// the gamble (a loss): 1.0 s on the cabinet (footage, exactly 30 frames).
const GAMBLE_SHOW_WIN: f32 = 1.0;
const GAMBLE_SHOW_LOSS: f32 = 1.0;
/// Seconds each kind of gamble (Suit, then Red/Black) labels the boxes.
const GAMBLE_KIND_TURN: f32 = 2.0;

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
    /// Half the stake is gambled, the rest kept ("Half Gamble Selected").
    pub half: bool,
    /// Seconds since the gamble opened (the boxes' Suit / Red/Black clock).
    since: f32,
    /// The last gamble win, kept over the card into the next round.
    last_win: Option<u64>,
    /// How the last gamble ended (lost or not), shown until the next game.
    pub ended: Option<bool>,
    /// The win the gamble ended on, shown on the WIN meter until the next
    /// game.
    pub result: Option<u64>,
    /// Testing: the autoplay's choices, its next choice, its wait.
    auto: Option<String>,
    auto_step: usize,
    auto_wait: f32,
}

impl GambleShow {
    /// A gamble with a full history from the start: the cabinet always shows
    /// five past cards (from play before this session), then the real ones.
    /// The starting cards are drawn for display only, from `seed` by a
    /// splitmix generator of their own; they decide nothing and never touch
    /// the game's outcome source.
    pub fn with_history(seed: u64) -> GambleShow {
        let mut x = seed ^ 0x6a09_e667_f3bc_c909;
        let history = (0..HISTORY)
            .map(|_| {
                x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
                let mut z = x;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
                ((z ^ (z >> 31)) % 52) as u8
            })
            .collect();
        GambleShow {
            history,
            ..default()
        }
    }

    /// Testing: the gamble autoplay holds the unattended spin while a win can
    /// be gambled.
    pub fn holds_autospin(&self, can_gamble: bool) -> bool {
        can_gamble && self.auto.as_deref().is_some_and(|s| !s.is_empty())
    }
}

/// The most a gamble may win, in cents: machines show "Gamble not offered if
/// gamble win will exceed $10000.00" and "Gamble win limit is $10000.00".
pub const GAMBLE_WIN_LIMIT_CENTS: u64 = 1_000_000;

/// Whether a gamble that multiplies `stake` credits by `times` stays within
/// the gamble win limit at `cents` per credit.
pub fn within_gamble_limit(stake: u64, times: u64, cents: u64) -> bool {
    stake
        .saturating_mul(times)
        .saturating_mul(cents)
        .le(&GAMBLE_WIN_LIMIT_CENTS)
}

/// Whether the last win may be gambled now: the game allows it, no
/// progressive jackpot was won, and doubling it stays within the limit.
pub fn gamble_open(game: &Game, cents: u64, show: &FeatureShow) -> bool {
    game.can_gamble()
        && !show.no_gamble()
        && within_gamble_limit(game.state().gamble_stake, 2, cents)
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
    denom: Res<Denom>,
) {
    if !fidelity.0 {
        return;
    }
    let cents = denom.cents();
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
        if input.gamble && at_rest && gamble_open(&sim.game, cents, &show) {
            gamble.active = true;
            gamble.card = None;
            gamble.half = false;
            gamble.since = 0.0;
            gamble.last_win = None;
            gamble.ended = None;
            gamble.stake = sim.game.state().gamble_stake;
            input.skip_spin = true;
        }
        return;
    }
    gamble.since += dt;
    // GAMBLE again chooses between the full and the half gamble.
    if input.gamble && gamble.card.is_none() && gamble.stake >= 2 {
        gamble.half = !gamble.half;
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
        let done = if won {
            GAMBLE_SHOW_WIN
        } else {
            GAMBLE_SHOW_LOSS
        };
        if t >= done {
            // The history takes the card as the next round opens (footage).
            gamble.card = None;
            gamble.history.insert(0, card);
            gamble.history.truncate(HISTORY);
        }
        if !won && t >= done {
            gamble.active = false;
            gamble.result = Some(0);
            gamble.last_win = None;
            gamble.ended = Some(true);
        } else if won
            && t >= done
            && (!sim.game.can_gamble() || !within_gamble_limit(gamble.stake, 2, cents))
        {
            // A limit (gambles in a row, or the gamble win limit): the win is
            // kept.
            gamble.active = false;
            gamble.result = Some(gamble.stake);
            gamble.ended = Some(false);
        }
        return;
    }
    // A suit is not offered when four times the stake passes the limit.
    let pick = input
        .pick
        .take()
        .filter(|p| is_colour(*p) || within_gamble_limit(gamble.stake, 4, cents));
    let Sim { game, rng } = &mut *sim;
    if let Some(pick) = pick {
        let command = if gamble.half {
            Command::HalfGamble { pick }
        } else {
            Command::Gamble { pick }
        };
        if let Ok(events) = game.apply(command, rng) {
            // Each turn starts on the full gamble.
            gamble.half = false;
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
                        gamble.stake = *award;
                        if *won {
                            gamble.last_win = Some(*award);
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
        gamble.ended = Some(false);
    }
}

/// The gamble screen and its parts.
#[derive(Component)]
pub struct GambleRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GambleText {
    /// "Full Gamble Selected" or "Half Gamble Selected".
    Selected,
    /// "Red/Black Gamble / Gamble Amount / n / $x".
    Amount,
    /// "Red/Black Gamble / to win ..." alternating with the suit gamble.
    ToWin,
    /// "Gamble Win / n / $x" over a winning card.
    GambleWin,
    /// "Select a SUIT, RED/BLACK or TAKE WIN".
    Prompt,
}
/// A choice button on the gamble screen.
#[derive(Component)]
pub struct GambleButton(GamblePick);
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
        // Left: the tab, the amount, RED and the red suits.
        g.spawn((
            at(lx, 175.0, 400.0, 50.0),
            ImageNode {
                image: ui("pill"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                GambleText::Selected,
                gamble_text("Full Gamble Selected", italic, 25.0),
            ));
        });
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
            t.spawn((
                GambleText::Prompt,
                gamble_text("Select a SUIT, RED/BLACK or TAKE WIN", italic, 26.0),
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
        GambleButton(pick),
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
    mut buttons: Query<
        (&GambleButton, &mut ImageNode),
        (Without<GambleCard>, Without<GambleStamp>),
    >,
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
    let _ = &time;
    // Both boxes name the same kind: Suit, then Red/Black, 2 s each from the
    // gamble's opening; only Red/Black when a suit would pass the limit.
    let suits_open = within_gamble_limit(gamble.stake, 4, cents);
    let suit_turn = suits_open && ((gamble.since / GAMBLE_KIND_TURN) as u32).is_multiple_of(2);
    let (kind, staked, mult) = match (shown, gamble.last) {
        (Some(_), Some((pick, stake))) => (gamble_kind(Some(pick)), stake, pick.multiplier()),
        _ => {
            let stake = if gamble.half {
                gamble.stake / 2
            } else {
                gamble.stake
            };
            if suit_turn {
                ("Suit Gamble", stake, 4)
            } else {
                ("Red/Black Gamble", stake, 2)
            }
        }
    };
    for (b, mut image) in &mut buttons {
        let tint = if is_colour(b.0) || suits_open {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.35)
        };
        if image.color != tint {
            image.color = tint;
        }
    }
    for (which, mut t) in &mut texts {
        let s = match which {
            GambleText::Selected if gamble.half => "Half Gamble Selected".to_string(),
            GambleText::Selected => "Full Gamble Selected".to_string(),
            GambleText::Amount => format!("{kind}\nGamble Amount\n{}", amount(staked)),
            GambleText::ToWin => {
                format!("{kind}\nto win\n{}", amount(staked.saturating_mul(mult)))
            }
            // The last win's header stays into the next round (footage).
            GambleText::GambleWin => match (won, gamble.last_win) {
                (true, _) => format!("Gamble Win\n{}", amount(gamble.stake)),
                (false, Some(w)) if shown.is_none() => format!("Gamble Win\n{}", amount(w)),
                _ => String::new(),
            },
            GambleText::Prompt if suits_open => "Select a SUIT, RED/BLACK or TAKE WIN".to_string(),
            GambleText::Prompt => "Select RED/BLACK or TAKE WIN".to_string(),
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

/// Seconds each winning line shows (real game: 43-45 frames, each line in
/// turn, round and round until the next game). Within it the line's badges
/// light twice, and its winning symbols that do not animate blink out twice,
/// in between (measured frame by frame).
const WIN_LINE_SECONDS: f32 = 1.45;
const WIN_LINE_LIT: [(f32, f32); 2] = [(0.0, 0.33), (0.70, 0.93)];
const WIN_LINE_BLINK: [(f32, f32); 2] = [(0.33, 0.63), (0.97, 1.37)];

/// Whether `phase` (seconds into a line's showing) falls in one of `spans`.
fn in_spans(phase: f32, spans: &[(f32, f32)]) -> bool {
    spans.iter().any(|&(a, b)| phase >= a && phase < b)
}
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
    // The flags keep waving while the free games wait for START FEATURE
    // (cabinet footage: they wave until the feature starts).
    let flags_wait = matches!(show.awaiting, Some(Await::Start(Feature::FreeGames)));
    show.scatter_shown = (show.scatter_paid || show.fg_flags)
        && !gamble.active
        && (show.awaiting.is_none() || flags_wait)
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
    let lit = current.filter(|_| in_spans(show.win_t % WIN_LINE_SECONDS, &WIN_LINE_LIT));
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
// Two or three emperors stacked on a reel become one tall full-length
// emperor (the game's own Wild2UP / Wild3UP animations: the heads wipe into
// the figure, which then burns with a fire border) and stay so until the reel
// spins again. Real game (recorded with sound): only a stack on a winning
// line does so, when the spin's win would show; it plays SND_Wild_2 or
// SND_Wild_3 (the stack's height; 2.5 s) and the win waits for it. A stack on
// no winning line stays as heads.

/// The tiles of the game's stack animations: each 130 x 118 frame pixels,
/// the first at (43, 39).
const WILD_TILE: (f32, f32) = (130.0, 118.0);
const WILD_TILE_AT: (f32, f32) = (43.0, 39.0);
/// Frames per second of the stack animations.
const WILD_FPS: f32 = 20.0;

/// Emperor stacks: those standing tall (reel, top row, height, since when in
/// seconds of `Time`), and those landed (reel, top row, height).
#[derive(Resource, Default)]
pub struct WildStacks {
    list: Vec<(u8, u8, u8, f32)>,
    landed: Vec<(u8, u8, u8)>,
    /// The stacks of this spin have been found.
    found: bool,
}

/// Seconds the win waits for a stack to stand (the length of SND_Wild_N).
pub const WILD_HOLD: f32 = 2.5;

impl WildStacks {
    /// This spin's stacks have been found.
    pub fn found(&self) -> bool {
        self.found
    }

    /// The height of the tallest landed stack on one of these winning lines
    /// (line, symbols in a row from reel 1), if any.
    pub fn on_lines(
        &self,
        paylines: &[dragonlink_model::Payline],
        wins: impl Iterator<Item = (u16, u8)>,
    ) -> Option<u8> {
        let mut tallest = None;
        for (line, count) in wins {
            let Some(rows) = paylines.get(line as usize) else {
                continue;
            };
            for &(reel, top, height) in &self.landed {
                let on = reel < count
                    && rows
                        .0
                        .get(reel as usize)
                        .is_some_and(|&row| row >= top && row < top + height);
                if on {
                    tallest = tallest.max(Some(height));
                }
            }
        }
        tallest
    }
}

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
    show: Res<FeatureShow>,
    labels: Query<(&CellLabel, &Text)>,
    mut stacks: ResMut<WildStacks>,
) {
    if !fidelity.0 {
        return;
    }
    // A spin clears the stacks; they are found once everything has stopped
    // (and the doors have opened), and stand when a win on them shows.
    let moving = (0..reels.states.len()).any(|r| reels.moving(r as u8));
    if moving {
        *stacks = WildStacks::default();
        return;
    }
    if show.reveal.is_some() || show.hold_and_spin() {
        return;
    }
    let now = time.elapsed_secs();
    if show.wild.is_some() && stacks.list.is_empty() {
        stacks.list = stacks
            .landed
            .iter()
            .map(|&(reel, top, height)| (reel, top, height, now))
            .collect();
    }
    if stacks.found {
        return;
    }
    stacks.found = true;
    for reel in 0..reels.states.len() as u8 {
        let mut column = [false; 3];
        for (l, text) in &labels {
            if l.reel == reel && (l.row as usize) < 3 {
                let (key, dim) = label_symbol(&text.0);
                column[l.row as usize] = key == "emperor" && !dim;
            }
        }
        let mut row = 0;
        while row < 3 {
            if !column[row] {
                row += 1;
                continue;
            }
            let start = row;
            while row < 3 && column[row] {
                row += 1;
            }
            if row - start >= 2 {
                stacks.landed.push((reel, start as u8, (row - start) as u8));
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
        let stack = stacks.list.iter().find(|s| s.0 == tall.reel);
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
    let line_cells = cells.clone();
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
        let on = cells.contains(&(f.reel, f.row)) && !stacks.list.iter().any(|s| s.0 == f.reel);
        let key = labels
            .iter()
            .find(|(l, _)| l.reel == f.reel && l.row == f.row)
            .map(|(_, t)| label_symbol(&t.0).0.to_string())
            .unwrap_or_default();
        let frame = on
            .then(|| {
                // The flags wave at 20 frames a second (real game).
                let fps = if key == "pennant" {
                    20.0
                } else {
                    SYMBOL_FX_FPS
                };
                let n = (t * fps) as usize;
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
        // The still symbol gives way to its animation; a winning symbol that
        // does not animate blinks out with its line (real game).
        let blink = !playing
            && line_cells.contains(&(f.reel, f.row))
            && in_spans(show.win_t % WIN_LINE_SECONDS, &WIN_LINE_BLINK);
        if playing || blink {
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
    labels: Query<&Text, With<CellLabel>>,
    mut music: ResMut<Music>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    let held = labels.iter().filter(|t| t.0.starts_with("ORB")).count();
    let want = match show.stage {
        Stage::FreeGames { .. } => Some("SND_Fea_BGM"),
        Stage::HoldSpin { .. } if show.grid && show.intro.is_none() && held >= 13 => {
            Some("SND_FreeSpin_BGMFast")
        }
        Stage::HoldSpin { .. } if show.grid && show.intro.is_none() => Some("SND_FreeSpin_BGM"),
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

/// Play the sounds the presentation has cued.
pub fn feature_cues(
    fidelity: Res<Fidelity>,
    mut show: ResMut<FeatureShow>,
    sounds: Res<Sounds>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    for name in std::mem::take(&mut show.cues) {
        sounds.play(&mut commands, name);
    }
}

// ---------------------------------------------------------------------------
// Coin shower
// ---------------------------------------------------------------------------
//
// Cabinet footage of big free-game wins (34x and 36x the bet): while the win
// counts up, gold coins (the game's own 25-frame spinning coin, from its
// CoinShower scene) burst from the top of the screen and rain over it.
// Smaller wins showed none.

/// Coins in the shower at once.
const SHOWER_COINS: usize = 56;
/// A win at least this many times the bet counts up under a coin shower.
const SHOWER_RATIO: u64 = 20;

/// One coin of the shower.
#[derive(Component)]
pub struct ShowerCoin(usize);

/// The shower's coins, hidden until a big win.
pub fn spawn_coin_shower(c: &mut ChildSpawnerCommands) {
    for i in 0..SHOWER_COINS {
        c.spawn((
            ShowerCoin(i),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(26),
            Visibility::Hidden,
        ));
    }
}

/// A stable pseudo-random number in 0..1 for coin `i`, draw `k`.
fn shower_rand(i: usize, k: u64) -> f32 {
    let h = (i as u64 * 0x9E37_79B9 + k * 0x85EB_CA6B)
        .wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        .rotate_left(29);
    (h % 10_000) as f32 / 10_000.0
}

/// Rain coins while a big win counts up.
pub fn coin_shower(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    rollup: Res<Rollup>,
    show: Res<FeatureShow>,
    mut coins: Query<(&ShowerCoin, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    let bet = sim.game.total_bet().max(1);
    let on = (rollup.active() && rollup.to.saturating_sub(rollup.from) >= SHOWER_RATIO * bet)
        || show.pay_shower(bet);
    let frames = art.anims.get("coin_shower");
    let t = time.elapsed_secs();
    for (coin, mut node, mut image, mut vis) in &mut coins {
        let want = if on && frames.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if want == Visibility::Hidden {
            continue;
        }
        let i = coin.0;
        // Each coin is thrown from the top, falls across the screen and is
        // thrown again: its own period and phase.
        let period = 1.6 + shower_rand(i, 1) * 1.2;
        let phase = shower_rand(i, 2) * period;
        let cycle = ((t + phase) / period).floor() as u64;
        let u = (t + phase) % period;
        let x0 = shower_rand(i, 10 + cycle) * CANVAS_W;
        let vx = (shower_rand(i, 20 + cycle) - 0.5) * 500.0;
        let vy = -250.0 - shower_rand(i, 30 + cycle) * 450.0;
        let x = x0 + vx * u;
        let y = 40.0 + vy * u + 0.5 * 1500.0 * u * u;
        let size = 70.0 + shower_rand(i, 4) * 50.0;
        node.left = Val::Px(x - size / 2.0);
        node.top = Val::Px(y - size / 2.0);
        node.width = Val::Px(size);
        node.height = Val::Px(size);
        if let Some(f) = frames {
            let n = ((t * 30.0) as usize + i * 7) % f.len().max(1);
            if let Some(h) = f.get(n)
                && image.image != *h
            {
                image.image = h.clone();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The free games' trigger
// ---------------------------------------------------------------------------
//
// Real game (recorded with sound, frame by frame): the flags land with the
// reels' own clink and nothing else. 0.2 s after the last reel is still the
// feature bell sounds and plays out (2.55 s); as it ends the flags start
// waving (the game's own flag animation from its first frame, at 20 fps) with
// SND_Scatter (4.75 s); as that ends the scatter win counts on WIN to its
// tune; then SELECT SPIN TO START FEATURE. The press makes no sound; the flags
// wave on for about a second, then a hard cut to the free games: their music
// starts from its beginning, the first free game spins at once, and the
// banner's total counts up from 1, one a tick (SND_Fea_Inc), during that spin.

/// Seconds into the trigger sequence (it begins when the result is shown,
/// 0.17 s after the last reel is still): the bell, the flags and their sound,
/// and the win's count.
const FG_BELL: f32 = 0.045;
const FG_FLAGS: f32 = FG_BELL + 2.6;
pub const FG_COUNT: f32 = FG_FLAGS + 4.76;
/// Seconds from the START FEATURE press to the cut to the free games.
const FG_CUT: f32 = 1.07;
/// Seconds per step of the banner's total counting up after the cut.
const FG_TICK: f32 = 4.0 / 30.0;

impl FeatureShow {
    /// Seconds before a feature's next step (real game): a free game starts
    /// 1.25 s after its win has counted, or, with no win, about 2.5 s after
    /// its doors opened (0.85 s after the reveal's end); a respin's step 0.5 s.
    pub fn step_pause(&self) -> f32 {
        match self.stage {
            Stage::FreeGames { .. } if self.game_won => 1.25,
            Stage::FreeGames { .. } => 0.85,
            _ => 0.5,
        }
    }

    /// The trigger sequence is at least `after` seconds past the coins'
    /// burst.
    fn since_burst(&self, after: f32) -> bool {
        self.intro.is_some_and(|t| t >= INTRO_BURST + after)
    }

    /// A win through an emperor sounds one of the wild sounds, once a game;
    /// a stack of emperors on the winning line also stands, and the win
    /// waits for it (real game: a stack held the win for the sound's 2.5 s).
    /// On recordings the variant changes from win to win, the same stack
    /// sounding different ones; it is not the stack's height. Here the five
    /// take turns in a scrambled order.
    pub fn begin_wild(&mut self, stack: bool) {
        if self.wild_sounded {
            return;
        }
        self.wild_sounded = true;
        const WILD_SOUNDS: [&str; 5] = [
            "SND_Wild_1",
            "SND_Wild_2",
            "SND_Wild_3",
            "SND_Wild_4",
            "SND_Wild_5",
        ];
        self.cues
            .push(WILD_SOUNDS[(self.wild_turn * 3 + 2) % WILD_SOUNDS.len()]);
        self.wild_turn += 1;
        if stack {
            self.wild = Some(0.0);
        }
    }

    /// Whether a wild sound has played this game (the presenter asks once).
    pub fn wild_sounded(&self) -> bool {
        self.wild_sounded
    }

    /// Begin the free games' trigger sequence, if a base game's flags have
    /// won the free games and it has not begun; true when it begins.
    pub fn begin_free_games_trigger(&mut self) -> bool {
        if self.fg_trigger.is_some() || !matches!(self.stage, Stage::Base) {
            return false;
        }
        self.fg_trigger = Some(0.0);
        true
    }

    /// The free games' total as the banner shows it (counting up after the
    /// cut).
    fn counter_total(&self, total: u32) -> u32 {
        self.fg_count.map_or(total, |(_, n)| n.min(total))
    }
}

/// Run the free games' trigger sequence and the cut into the free games.
pub fn free_games_trigger(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut show: ResMut<FeatureShow>,
    mut presentation: ResMut<crate::Presentation>,
    sounds: Res<Sounds>,
    mut commands: Commands,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time);
    let crossed = |a: f32, b: f32, at: f32| a < at && b >= at;
    if let Some(t) = show.wild {
        show.wild = Some(t + dt);
    }
    if let Some(t0) = show.fg_trigger {
        let t = t0 + dt;
        show.fg_trigger = Some(t);
        if crossed(t0, t, FG_BELL) {
            sounds.play(&mut commands, "feature_bell");
        }
        if crossed(t0, t, FG_FLAGS) {
            sounds.play(&mut commands, "SND_Scatter");
            show.fg_flags = true;
            show.win_t = 0.0;
        }
    }
    if let Some(t0) = show.fg_cut {
        let t = t0 + dt;
        if t >= FG_CUT {
            // The cut: the free games, their music, the first game at once.
            show.fg_cut = None;
            show.fg_flags = false;
            show.scatter_paid = false;
            show.stage = Stage::FreeGames {
                number: 1,
                total: u32::from(show.games_won),
            };
            show.fg_count = Some((0.0, 1));
            presentation.step_timer = 0.0;
        } else {
            show.fg_cut = Some(t);
        }
    }
    if let Some((t0, n)) = show.fg_count {
        let total = match show.stage {
            Stage::FreeGames { total, .. } => total,
            _ => 0,
        };
        let t = t0 + dt;
        let mut n = n;
        // One tick a step, just before the banner's digit changes.
        while n < total && t >= n as f32 * FG_TICK {
            n += 1;
            sounds.play(&mut commands, "SND_Fea_Inc");
        }
        show.fg_count = if n >= total { None } else { Some((t, n)) };
    }
}

// ---------------------------------------------------------------------------
// The Hold & Spin medallion and the counter's burst
// ---------------------------------------------------------------------------

/// The HOLD & SPIN medallion: the burning gold disc over the reels with the
/// feature's title on it (real game: centred on the reels, about 2.3 reels
/// wide and the reels' full height).
#[derive(Component)]
pub struct HsMedallion;
/// The burst on the banner's counter as it reaches 3.
#[derive(Component)]
pub struct BannerBurst;

const MEDALLION: (f32, f32) = (2.33 * PITCH.0, 3.0 * PITCH.1);

/// Spawn the medallion (hidden) and the banner's burst.
pub fn spawn_medallion(c: &mut ChildSpawnerCommands, art: &ArtPack) {
    let (w, h) = MEDALLION;
    c.spawn((
        HsMedallion,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(GRID_CENTER.0 - w / 2.0),
            top: Val::Px(GRID_CENTER.1 - h / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(25),
        Visibility::Hidden,
    ))
    .with_children(|m| {
        m.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(w * 0.22),
                top: Val::Px(h * 0.21),
                width: Val::Px(w * 0.56),
                height: Val::Px(h * 0.58),
                ..default()
            },
            ImageNode {
                image: art.ui.get("hs_title").cloned().unwrap_or_default(),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
    });
    // The banner's counter digit sits near its left end (real game).
    let bw = BANNER_SIZE.0;
    let (sw, sh) = (1.2 * PITCH.0, 1.4 * PITCH.1);
    let (x, y) = (GRID_CENTER.0 - bw / 2.0 + bw * 0.083, BANNER_Y);
    c.spawn((
        BannerBurst,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x - sw / 2.0),
            top: Val::Px(y - sh / 2.0),
            width: Val::Px(sw),
            height: Val::Px(sh),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(26),
        Visibility::Hidden,
    ));
}

/// Show the medallion during the trigger sequence, and the banner's burst.
#[allow(clippy::type_complexity)]
pub fn hs_medallion(
    fidelity: Res<Fidelity>,
    art: Res<ArtPack>,
    show: Res<FeatureShow>,
    mut medallion: Query<
        (&mut ImageNode, &mut Visibility),
        (With<HsMedallion>, Without<BannerBurst>),
    >,
    mut burst: Query<(&mut ImageNode, &mut Visibility), (With<BannerBurst>, Without<HsMedallion>)>,
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
    let on = show
        .intro
        .is_some_and(|t| (INTRO_MEDALLION..INTRO_GRID).contains(&t));
    for (mut image, mut vis) in &mut medallion {
        let f = if on { frame("coin", clock, true) } else { None };
        let want = if f.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(h) = f
            && image.image != h
        {
            image.image = h;
        }
        if *vis != want {
            *vis = want;
        }
    }
    for (mut image, mut vis) in &mut burst {
        let f = show
            .banner_burst
            .and_then(|s| frame("burst", clock - s, false));
        let want = if f.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(h) = f
            && image.image != h
        {
            image.image = h;
        }
        if *vis != want {
            *vis = want;
        }
    }
}

/// A burst on one of the WIN banner's "WIN" letters.
#[derive(Component)]
pub struct WinFlare;

/// Where the WIN banner's letters sit, across its width (real game: 0.136
/// and 0.851).
const WIN_LETTERS: [f32; 2] = [0.136, 0.851];

/// Spawn the two WIN letter bursts (hidden).
pub fn spawn_win_flares(c: &mut ChildSpawnerCommands) {
    let bw = BANNER_SIZE.0;
    let s = 0.6 * PITCH.0;
    for k in WIN_LETTERS {
        let x = GRID_CENTER.0 - bw / 2.0 + bw * k;
        c.spawn((
            WinFlare,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x - s / 2.0),
                top: Val::Px(BANNER_Y - s / 2.0),
                width: Val::Px(s),
                height: Val::Px(s),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(26),
            Visibility::Hidden,
        ));
    }
}

/// Burst the WIN letters every 59 frames once the coins are added, and fade
/// the rumble out as the first coin is added.
pub fn payout_effects(
    fidelity: Res<Fidelity>,
    art: Res<ArtPack>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    mut flares: Query<(&mut ImageNode, &mut Visibility), With<WinFlare>>,
    mut sinks: Query<&mut AudioSink>,
    global: Res<bevy::audio::GlobalVolume>,
) {
    if !fidelity.0 {
        return;
    }
    let f = show.flares_at.filter(|_| !gamble.active).and_then(|s| {
        let frames = art.anims.get("burst")?;
        let t = (show.clock - s).max(0.0) % FLARE_EVERY;
        frames.get((t * ANIM_FPS) as usize).cloned()
    });
    for (mut image, mut vis) in &mut flares {
        let want = if f.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(h) = &f
            && image.image != *h
        {
            image.image = h.clone();
        }
        if *vis != want {
            *vis = want;
        }
    }
    if let (Some(id), Some(t)) = (show.rumble, show.collect_t())
        && let Ok(mut sink) = sinks.get_mut(id)
    {
        let k = ((t - COLLECT_FIRST) / RUMBLE_FADE).clamp(0.0, 1.0);
        // Scaled by the global volume, which a muted test run sets to 0.
        sink.set_volume(bevy::audio::Volume::Linear(
            (1.0 - k) * global.volume.to_linear(),
        ));
    }
}
