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
use bevy::text::{LetterSpacing, LineHeight};
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
/// The screen's foot (the rows under the reels and the meter band are
/// placed per presentation in [`spawn_meter_band`]).
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

/// The clip a bet key sounds and its volume (footage: x1 G3, x2 C4, x5 G4,
/// x10 C5, the first two the game's own reduced beeps, the others the app's
/// shared bank, about 10 dB louder, so played at 0.32; x3's E4 is the bank's
/// beep between them, not heard on footage).
pub fn bet_beep(multiplier: u32) -> (&'static str, f32) {
    match multiplier {
        1 => ("betbeep1_reduced", 1.0),
        2 => ("betbeep3_reduced", 1.0),
        3 => ("bank_betbeep3", 0.32),
        5 => ("bank_betbeep4", 0.32),
        _ => ("bank_betbeep5", 0.32),
    }
}

/// A moment of the gamble that may sound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GambleCue {
    Open,
    Toggle,
    Pick,
    Won,
    Lost,
    TakeWin,
}

/// What the gamble plays, from public recordings of the gamble with sound on
/// about 20 Aristocrat machines (golden-century-sound-cues.md):
/// - Won: the platform's gamble-win tune, a short arpeggio into a chord, on
///   every win of every title with usable sound, Golden Century included
///   (VERIFIED). Lost: a low square-wave buzz, on four titles including
///   Golden Century (VERIFIED). Neither is in any bank we hold, so the pack
///   carries our own synthesis of each from the measured pitches and timing.
/// - Open, Toggle, Pick, TakeWin: nothing (STRONG: no cabinet sound at any of
///   13 openings, at any choice beyond the deck key's own click, at the
///   on-camera take wins). No loop while waiting.
pub fn gamble_cue(cue: GambleCue) -> Option<(&'static str, f32)> {
    match cue {
        GambleCue::Won => Some(("gamble_win", 1.0)),
        GambleCue::Lost => Some(("gamble_loss", 1.0)),
        GambleCue::Open | GambleCue::Toggle | GambleCue::Pick | GambleCue::TakeWin => None,
    }
}

/// Seconds after the card turns that its outcome sounds (footage: the win
/// tune 0.13-0.28 s, the loss buzz 0.15-0.30 s after the face).
const GAMBLE_SOUND_AFTER_FACE: f32 = 0.2;

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

/// The model's symbols by the part they play, so the presentation names no
/// title's own symbols: Golden Century's emperor, pennant and doors are
/// another title's wild, scatter and mystery symbol.
#[derive(Clone, Debug, Default)]
pub struct Roles {
    pub wild: String,
    pub scatter: String,
    pub coin: String,
    /// The free games' mystery symbol, if the title has one.
    pub mystery: Option<String>,
    /// What a respinning Hold & Spin position shows passing: every symbol
    /// but the mystery one, the coin last.
    pub respin: Vec<String>,
}

impl Roles {
    pub fn from_model(model: &MathModel) -> Roles {
        let key_of = |kind: SymbolKind| {
            model
                .symbols
                .iter()
                .find(|s| s.kind == kind)
                .map(|s| s.key.clone())
                .unwrap_or_default()
        };
        let mystery = model
            .free_games
            .as_ref()
            .and_then(|fg| fg.mystery)
            .and_then(|id| model.symbols.get(id.0 as usize))
            .map(|s| s.key.clone());
        let coin = model
            .hold_and_spin
            .as_ref()
            .and_then(|hs| model.symbols.get(hs.orb.0 as usize))
            .map(|s| s.key.clone())
            .unwrap_or_else(|| key_of(SymbolKind::Orb));
        let mut respin: Vec<String> = model
            .symbols
            .iter()
            .map(|s| s.key.clone())
            .filter(|k| Some(k) != mystery.as_ref() && *k != coin)
            .collect();
        respin.push(coin.clone());
        Roles {
            wild: key_of(SymbolKind::Wild),
            scatter: key_of(SymbolKind::Scatter),
            coin,
            mystery,
            respin,
        }
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
pub fn window(title: &str) -> Window {
    Window {
        title: title.into(),
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
    /// A heavy, wide face for the gamble's amounts (Arial Black).
    pub heavy: Handle<Font>,
    /// The status and message rows' face (Verdana Bold).
    pub msg: Handle<Font>,
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
        heavy: first(&[
            "/System/Library/Fonts/Supplemental/Arial Black.ttf",
            "C:/Windows/Fonts/ariblk.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
        ]),
        msg: first(&[
            "/System/Library/Fonts/Supplemental/Verdana Bold.ttf",
            "/Library/Fonts/Verdana Bold.ttf",
            "C:/Windows/Fonts/verdanab.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
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

/// The jackpot bar's four boxes: the left and right edges of each box's dark
/// panel, in the bar image's pixels (measured on `jackpot_bar`).
const JACKPOT_BOXES: [(JackpotTier, (f32, f32)); 4] = [
    (JackpotTier::Grand, (14.0, 351.0)),
    (JackpotTier::Major, (365.0, 667.0)),
    (JackpotTier::Minor, (681.0, 961.0)),
    (JackpotTier::Mini, (975.0, 1209.0)),
];

/// The jackpot values' type size, and the share of its box a value may fill
/// before it is set smaller.
const JACKPOT_VALUE_SIZE: f32 = 56.0;
const JACKPOT_VALUE_FILL: f32 = 0.9;

/// A jackpot value's box width, in canvas pixels.
#[derive(Component)]
pub struct JackpotFit {
    width: f32,
}

/// A value's width in ems in the meter face: figures and "$" about 0.46,
/// "," and "." about 0.25 (measured on the bar's "$5,434.05").
fn meter_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| if matches!(c, ',' | '.') { 0.25 } else { 0.46 })
        .sum()
}

/// The type size that keeps `text` inside a box `width` wide.
fn jackpot_value_size(text: &str, width: f32) -> f32 {
    let ems = meter_ems(text);
    if ems <= 0.0 {
        return JACKPOT_VALUE_SIZE;
    }
    JACKPOT_VALUE_SIZE.min(width * JACKPOT_VALUE_FILL / ems)
}

/// Set each jackpot value small enough to stay inside its box.
pub fn fit_jackpots(
    fidelity: Res<Fidelity>,
    mut values: Query<(&JackpotFit, &Text, &mut TextFont, &mut Node), Changed<Text>>,
) {
    if !fidelity.0 {
        return;
    }
    for (fit, text, mut font, mut node) in &mut values {
        let size = jackpot_value_size(&text.0, fit.width);
        if font.font_size != FontSize::Px(size) {
            let old = match font.font_size {
                FontSize::Px(px) => px,
                _ => JACKPOT_VALUE_SIZE,
            };
            font.font_size = FontSize::Px(size);
            // Keep the value on the same middle line.
            if let Val::Px(top) = node.top {
                node.top = Val::Px(top + (old - size) * 0.62);
            }
        }
    }
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
            // The black tile each coin sits on (cabinet footage and the app:
            // a coin's cell is black, not the blue reel face), from separator
            // to separator; under the symbols, so overhangs draw over it.
            let (_, ch) = cell_size();
            for reel in 0..model.grid.reels {
                let (left, width) = coin_tile_span(reel);
                for row in 0..model.grid.rows {
                    let (_, y) = cell_center(reel, row);
                    c.spawn((
                        CoinTile { reel, row },
                        coin_tile(
                            left,
                            y - ch / 2.0,
                            width,
                            (row > 0, row + 1 < model.grid.rows),
                        ),
                        Visibility::Hidden,
                    ));
                }
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
            // Each value centred in its box's dark panel.
            for (tier, (left, right)) in JACKPOT_BOXES {
                let width = (right - left) * k;
                c.spawn((
                    JackpotMeter { tier },
                    JackpotFit { width },
                    label(
                        "-",
                        font,
                        JACKPOT_VALUE_SIZE,
                        Color::srgb(1.0, 1.0, 0.96),
                        960.0 - bar_w / 2.0 + (left + right) / 2.0 * k,
                        bar_top + 70.0 * k,
                        width,
                    ),
                ));
            }

            spawn_meter_band(c, fonts, art, gold, pale, profile);

            spawn_deck(c, art, &fonts.sans, &fonts.symbols);
            spawn_gamble_screen(c, art, fonts, profile);
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

/// An on-screen tab's look: the club machines' yellow-to-orange capsule with
/// navy condensed lettering, or the owner's casino photo's two-tone khaki
/// and silver ones.
#[derive(Clone, Copy)]
enum TabLook {
    Club,
    CasinoKhaki,
    CasinoSilver,
}

/// A rounded on-screen tab (COLLECT, GAMES MENU, GAME RULES): a capsule with
/// a gloss across its upper half kept inside the rounded rim.
#[allow(clippy::too_many_arguments)]
fn pill(
    c: &mut ChildSpawnerCommands,
    fonts: &CabFonts,
    text: &str,
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    size: f32,
    look: TabLook,
) {
    let stop = |r: u8, g: u8, b: u8, at: f32| {
        bevy::ui::ColorStop::new(Color::srgb_u8(r, g, b), Val::Percent(at))
    };
    let (gradient, rim, ink, font) = match look {
        TabLook::Club => (
            bevy::ui::LinearGradient::to_right(vec![
                stop(250, 245, 200, 0.0),
                stop(250, 215, 45, 35.0),
                stop(238, 140, 30, 100.0),
            ]),
            Color::srgb_u8(110, 55, 10),
            Color::srgb_u8(30, 32, 100),
            &fonts.meter,
        ),
        TabLook::CasinoKhaki => (
            bevy::ui::LinearGradient::to_bottom(vec![
                stop(250, 250, 228, 0.0),
                stop(250, 250, 228, 50.0),
                stop(244, 231, 140, 50.0),
                stop(244, 231, 140, 100.0),
            ]),
            Color::srgb_u8(150, 90, 30),
            Color::srgb_u8(30, 24, 6),
            &fonts.sans,
        ),
        TabLook::CasinoSilver => (
            bevy::ui::LinearGradient::to_bottom(vec![
                stop(245, 248, 250, 0.0),
                stop(225, 230, 238, 100.0),
            ]),
            Color::srgb_u8(120, 125, 140),
            Color::srgb_u8(50, 52, 73),
            &fonts.sans,
        ),
    };
    let gloss_h = (h * 0.42).round();
    c.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(cy - h / 2.0),
            width: Val::Px(w),
            height: Val::Px(h),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(h / 2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui::BackgroundGradient(vec![bevy::ui::Gradient::Linear(gradient)]),
        BorderColor::all(rim),
    ))
    .with_children(|p| {
        // The gloss, inset so its rounded top stays within the rim.
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                top: Val::Px(2.0),
                height: Val::Px(gloss_h),
                border_radius: BorderRadius::top(Val::Px(gloss_h / 2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
        ));
        p.spawn((
            Text::new(text),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextLayout::no_wrap(),
            TextColor(ink),
        ));
    });
}

/// The status line's centre, and its height (the centre of its capitals) on
/// each presentation (cabinet footage mapped onto the canvas: club 897,
/// casino 889).
const STATUS_X: f32 = 956.0;
fn status_y(profile: Profile) -> f32 {
    match profile {
        Profile::Club => 897.0,
        Profile::Casino => 889.0,
    }
}

/// The message row under it (feature prompts, line wins), its top.
fn message_top(profile: Profile) -> f32 {
    match profile {
        Profile::Club => 904.0,
        Profile::Casino => 898.0,
    }
}

/// The dark meter band's top: club 940, casino 933 (owner's photo).
pub fn band_top(profile: Profile) -> f32 {
    match profile {
        Profile::Club => 940.0,
        Profile::Casino => 933.0,
    }
}

/// The meters' centre line.
fn meter_y(profile: Profile) -> f32 {
    match profile {
        Profile::Club => 990.0,
        Profile::Casino => 984.0,
    }
}

/// The status and message rows' text: Verdana Bold 23, pale blue-white,
/// with a dark edge (cabinet footage); the casino spaces its letters a
/// little.
fn status_text(s: &str, fonts: &CabFonts, profile: Profile, justify: Justify) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: bevy::text::FontSource::Handle(fonts.msg.clone()),
            font_size: FontSize::Px(23.0),
            ..default()
        },
        TextColor(Color::srgb_u8(210, 220, 245)),
        TextLayout::justify(justify),
        LetterSpacing::Px(if profile == Profile::Casino { 1.5 } else { 0.0 }),
        TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::srgba(0.03, 0.03, 0.05, 0.9),
        },
    )
}

/// The rotating prompt above the meters ("3 [scatter] Start FREE GAMES",
/// "6 [orb] Start HOLD & SPIN FEATURE").
#[derive(Component)]
pub struct StartPrompt {
    /// Which of the two prompts this is.
    which: u8,
}

/// The screen below the reels, laid out as the cabinet lays it out
/// (positions mapped from footage onto the canvas): the status line and the
/// message row on the backdrop under the reels; the dark meter band with
/// CREDIT, BET and WIN; the tabs and the denomination box; "N LINES" either
/// side of the reels.
fn spawn_meter_band(
    c: &mut ChildSpawnerCommands,
    fonts: &CabFonts,
    art: &ArtPack,
    gold: Color,
    pale: Color,
    profile: Profile,
) {
    let font = &fonts.meter;
    let club = profile == Profile::Club;
    let rect = |l: f32, t: f32, w: f32, h: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(l),
        top: Val::Px(t),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    };
    let top = band_top(profile);
    let my = meter_y(profile);
    // The band: on the club machines a rounded panel over a darker strip;
    // in the casino style one dark area from the band to the screen's foot.
    if club {
        c.spawn((
            rect(0.0, 1040.0, CANVAS_W, SCREEN_BOTTOM - 1040.0),
            BackgroundColor(Color::srgb_u8(6, 5, 10)),
        ));
        c.spawn((
            Node {
                border_radius: BorderRadius::all(Val::Px(16.0)),
                ..rect(200.0, top, 1513.0, 100.0)
            },
            BackgroundColor(Color::srgba_u8(18, 18, 34, 235)),
        ));
    } else {
        c.spawn((
            rect(0.0, top + 102.0, CANVAS_W, SCREEN_BOTTOM - top - 102.0),
            BackgroundColor(Color::srgb_u8(8, 8, 12)),
        ));
        c.spawn((
            Node {
                border_radius: BorderRadius::right(Val::Px(16.0)),
                ..rect(0.0, top, 1730.0, 102.0)
            },
            BackgroundColor(Color::srgba_u8(8, 8, 12, 245)),
        ));
    }

    // The status line under the reels.
    c.spawn((
        CabMeter::Multiplier,
        status_text("", fonts, profile, Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STATUS_X - 600.0),
            top: Val::Px(status_y(profile) - 14.0),
            width: Val::Px(1200.0),
            ..default()
        },
    ));

    // The message row: the start prompts at its right, right-aligned to
    // x 1685, wholly above the band.
    for (which, count, key, text) in [
        (0u8, "3", art.roles.scatter.as_str(), "Start FREE GAMES"),
        (
            1u8,
            "6",
            art.roles.coin.as_str(),
            "Start HOLD & SPIN FEATURE",
        ),
    ] {
        c.spawn((
            StartPrompt { which },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1145.0),
                top: Val::Px(message_top(profile)),
                width: Val::Px(540.0),
                height: Val::Px(36.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(7.0),
                ..default()
            },
            if which == 0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ))
        .with_children(|row| {
            row.spawn(status_text(count, fonts, profile, Justify::Left));
            // The symbol as a small tile, on its own background (a coin on
            // black).
            row.spawn((
                Node {
                    width: Val::Px(34.0),
                    height: Val::Px(28.0),
                    ..default()
                },
                BackgroundColor(if key == "orb" {
                    Color::BLACK
                } else {
                    Color::NONE
                }),
            ))
            .with_children(|t| {
                t.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        image: art.symbols.get(key).cloned().unwrap_or_default(),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            });
            row.spawn(status_text(text, fonts, profile, Justify::Left));
        });
    }

    // A feature's message ("Line 3 wins 20", "Scatter wins 100") takes the
    // prompts' place on the same row.
    c.spawn((
        FeatureMessage,
        status_text("", fonts, profile, Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(1145.0),
            top: Val::Px(message_top(profile) + 5.0),
            width: Val::Px(540.0),
            ..default()
        },
    ));

    // The tall yellow pill at the far left, with its stacked bars (its
    // function is not shown on footage).
    let (pill_top, pill_h) = if club { (937.0, 92.0) } else { (945.0, 90.0) };
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(20.0)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(7.0),
            ..rect(12.0, pill_top, 52.0, pill_h)
        },
        BackgroundColor(Color::srgb(0.98, 0.8, 0.14)),
        BorderColor::all(Color::srgb(0.75, 0.45, 0.05)),
        GlobalZIndex(14),
    ))
    .with_children(|b| {
        for w in [28.0, 22.0, 14.0] {
            b.spawn((
                Node {
                    width: Val::Px(w),
                    height: Val::Px(5.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.14, 0.4)),
            ));
        }
    });
    // The tabs. Club: GAMES MENU above the band at the far left, GAME RULES
    // on it; COLLECT over the denomination box. Casino: GAME RULES on the
    // band at the left, GAMES MENU over the denomination box.
    if club {
        pill(
            c,
            fonts,
            "GAMES MENU",
            59.0,
            900.0,
            112.0,
            40.0,
            20.0,
            TabLook::Club,
        );
        c.spawn((
            rect(3.0, 880.0, 112.0, 40.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(
            |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| {
                input.menu = true;
            },
        );
        pill(
            c,
            fonts,
            "GAME RULES",
            136.0,
            1009.0,
            110.0,
            42.0,
            19.0,
            TabLook::Club,
        );
        pill(
            c,
            fonts,
            "COLLECT",
            1817.0,
            908.0,
            176.0,
            42.0,
            32.0,
            TabLook::Club,
        );
    } else {
        pill(
            c,
            fonts,
            "GAME RULES",
            156.0,
            1021.0,
            158.0,
            44.0,
            20.0,
            TabLook::CasinoSilver,
        );
        pill(
            c,
            fonts,
            "GAMES MENU",
            1819.0,
            896.0,
            162.0,
            42.0,
            20.0,
            TabLook::CasinoKhaki,
        );
        c.spawn((
            rect(1738.0, 875.0, 164.0, 150.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(
            |_press: On<PointerPress>, mut input: ResMut<CabinetInput>| {
                input.menu = true;
            },
        );
    }

    // CREDIT / BET / WIN and their values (Impact, drawn a little narrow as
    // the cabinet's is); CREDIT stays above the GAMES MENU's splash.
    let (labels, values) = if club {
        (
            [("CREDIT", 310.0), ("BET", 850.0), ("WIN", 1217.0)],
            [
                (CabMeter::Credit, 602.0),
                (CabMeter::Bet, 1009.0),
                (CabMeter::Win, 1484.0),
            ],
        )
    } else {
        (
            [("CREDIT", 311.0), ("BET", 805.0), ("WIN", 1240.0)],
            [
                (CabMeter::Credit, 568.0),
                (CabMeter::Bet, 1005.0),
                (CabMeter::Win, 1496.0),
            ],
        )
    };
    let narrow = UiTransform {
        scale: Vec2::new(0.9, 1.0),
        ..UiTransform::IDENTITY
    };
    for (i, (text, x)) in labels.into_iter().enumerate() {
        let mut e = c.spawn((label(text, font, 68.0, gold, x, my, 260.0), narrow));
        if i == 0 {
            e.insert(GlobalZIndex(14));
        }
    }
    for (meter, x) in values {
        let mut e = c.spawn((meter, label("-", font, 68.0, pale, x, my, 420.0), narrow));
        if matches!(meter, CabMeter::Credit) {
            e.insert(GlobalZIndex(14));
        }
    }

    // The denomination box at the right, under COLLECT (club) or GAMES MENU
    // (casino); its small line is the credit conversion or MIN BET.
    let (bx, by, bw, bh, dx, dy, dsize, note_y, note_colour, denom_colour) = if club {
        (
            1726.0,
            941.0,
            178.0,
            97.0,
            1815.0,
            978.0,
            78.0,
            1022.0,
            Color::srgb_u8(240, 170, 60),
            gold,
        )
    } else {
        (
            1740.0,
            926.0,
            162.0,
            98.0,
            1821.0,
            967.0,
            76.0,
            1010.0,
            Color::srgb_u8(215, 225, 250),
            Color::srgb_u8(245, 235, 140),
        )
    };
    c.spawn((
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..rect(bx, by, bw, bh)
        },
        BackgroundColor(Color::srgba_u8(12, 10, 12, 242)),
        BorderColor::all(Color::srgb_u8(60, 58, 60)),
    ));
    c.spawn((
        CabMeter::Denom,
        label("1¢", font, dsize, denom_colour, dx, dy, bw),
    ));
    c.spawn((
        CabMeter::MinBet,
        label(
            "MIN BET 50",
            &fonts.sans,
            if club { 19.0 } else { 18.0 },
            note_colour,
            dx,
            note_y,
            bw,
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
    // Square keys, as on the cabinet (its keys are about as tall as wide),
    // in two rows, and the teardrop beside them spanning both.
    let (kw, kh, gx, gy) = DECK_KEY;
    let cluster = 6.0 * kw + 5.0 * gx;
    let (sw, sh) = DECK_TEARDROP;
    let left = (CANVAS_W - (cluster + 60.0 + sw)) / 2.0;
    let top = DECK_TOP + 8.0;
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
        left + cluster + 60.0,
        top + kh + gy / 2.0 - sh / 2.0,
        sw,
        sh,
    );
}

/// A key's width and height and the gaps between keys, and the teardrop's
/// size (cabinet footage: square keys about a third of a key apart, the
/// teardrop about 1.75 keys wide and as tall as both rows).
const DECK_KEY: (f32, f32, f32, f32) = (96.0, 80.0, 30.0, 10.0);
const DECK_TEARDROP: (f32, f32) = (170.0, 170.0);

/// A key's legend as the cabinet stacks it: lines of their own sizes (a lines
/// key reads a small PLAY, its count large, LINES, then the credits it bets),
/// sized to sit inside the key's bezel.
fn legend(role: Role, lines: u16, bet: u32) -> Vec<(String, f32)> {
    let plural = |n: u64, word: &str| {
        if n == 1 {
            format!("{n} {word}")
        } else {
            format!("{n} {word}S")
        }
    };
    let play = |n: u16, per: u32| {
        let credits = u64::from(n) * u64::from(per);
        vec![
            ("PLAY".to_string(), 10.0),
            (n.to_string(), 26.0),
            ((if n == 1 { "LINE" } else { "LINES" }).to_string(), 9.5),
            (
                plural(credits, "CREDIT"),
                if credits >= 100 { 8.5 } else { 9.0 },
            ),
        ]
    };
    match role {
        Role::Gamble => vec![("GAMBLE".into(), 16.0)],
        Role::TakeWin => vec![("TAKE".into(), 17.0), ("WIN".into(), 17.0)],
        Role::Bet(m) => vec![
            (format!("BET x {m}"), 13.0),
            ((u64::from(m) * u64::from(lines)).to_string(), 26.0),
        ],
        Role::Credits(n) => vec![(n.to_string(), 38.0)],
        Role::Lines(n) => play(n, bet),
        Role::Play => play(lines, bet),
        Role::Pick(p) if is_colour(p) => vec![(pick_text(p).into(), 17.0)],
        Role::Pick(p) => vec![(pick_text(p).into(), 36.0)],
        Role::Letter(c) => vec![(c.to_string(), 38.0)],
        Role::Blank | Role::Lattice | Role::Off => Vec::new(),
    }
}

/// Lines a key's legend may have.
const LEGEND_LINES: u8 = 4;

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

/// A line of a deck button's legend.
#[derive(Component)]
pub struct DeckLabel(pub CabButton, pub u8);

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
    // The play button's legend sits in the teardrop's wide upper part.
    let pad = if button == CabButton::Spin {
        UiRect::bottom(Val::Px(h * 0.22))
    } else {
        UiRect::all(Val::Px(6.0))
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
        b.spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|col| {
            for line in 0..LEGEND_LINES {
                col.spawn((
                    DeckLabel(button, line),
                    Text::new(""),
                    TextFont {
                        font: bevy::text::FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    LineHeight::RelativeToFont(1.0),
                    DeckFonts {
                        text: font.clone(),
                        symbols: symbols.clone(),
                    },
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ));
            }
        });
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
        &mut Node,
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
    for (label, fonts, mut text, mut font, mut color, mut node) in &mut labels {
        let r = role(label.0, &d);
        let lines_of = |v: &[(&str, f32)]| -> Vec<(String, f32)> {
            v.iter().map(|(w, z)| ((*w).to_string(), *z)).collect()
        };
        let words: Vec<(String, f32)> = match (label.0, r) {
            (CabButton::Spin, Role::Off | Role::Lattice) => Vec::new(),
            // Footage: START FEATURE while a feature waits, SPIN through Hold
            // & Spin, PLAY and REPEAT BET in turn at rest (club); sized to sit
            // inside the teardrop's face.
            (CabButton::Spin, _) if start_wait => lines_of(&[("START", 17.0), ("FEATURE", 15.0)]),
            (CabButton::Spin, _) if show.hold_and_spin() => lines_of(&[("SPIN", 25.0)]),
            (CabButton::Spin, _)
                if d.profile == Profile::Club
                    && d.mode == DeckMode::Base
                    && idle
                    && (d.since as u32) % 2 == 1 =>
            {
                lines_of(&[("REPEAT", 16.0), ("BET", 16.0)])
            }
            (CabButton::Spin, _) => lines_of(&[("PLAY", 25.0)]),
            // The plain face of the flash carries no legend.
            (cap, r) if flashing(cap, r) => Vec::new(),
            // Top left is blank purple in the gamble.
            (_, Role::Gamble) if d.mode == DeckMode::Gamble => Vec::new(),
            (_, Role::Pick(p)) if !is_colour(p) && !suits_open => Vec::new(),
            // The letters wipe in from the left, one every 0.1 s.
            (CabButton::Slot { col, .. }, Role::Letter(_))
                if d.mode == DeckMode::Spinning && d.since < 0.1 * f32::from(col) =>
            {
                Vec::new()
            }
            _ => legend(r, lines, bet),
        };
        let (word, size) = words
            .get(label.1 as usize)
            .cloned()
            .unwrap_or((String::new(), 12.0));
        let display = if word.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
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
    mut meters: Query<(
        &CabMeter,
        &mut Text,
        &mut TextColor,
        Option<&mut LetterSpacing>,
    )>,
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
    for (meter, mut text, mut colour, spacing) in &mut meters {
        let value = match meter {
            // A feature under way, from its trigger spin on (the game is in it
            // before the screen shows it): CREDIT holds still while the
            // trigger's win counts on WIN (cabinet footage).
            // Its end too (completion, count and payment): on the last free
            // game the engine is idle before they are shown.
            CabMeter::Credit if show.in_feature() || show.counting() || !sim.game.is_idle() => {
                amount(
                    presentation
                        .shown_credits
                        .saturating_sub(presentation.shown_win),
                    cents,
                )
            }
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
            CabMeter::Win if gamble.active => amount(gamble.shown_stake(), cents),
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
        // The club machines write the bet line ("3 credits per line") in
        // yellow, widely spaced; every other status text is pale blue-white
        // (footage).
        let (CabMeter::Multiplier, Some(mut spacing)) = (meter, spacing) else {
            continue;
        };
        let bet_line = deck.profile == Profile::Club && text.0.ends_with("per line");
        let (want, gap) = match (bet_line, deck.profile) {
            (true, _) => (Color::srgb_u8(225, 225, 80), 4.8),
            (false, Profile::Club) => (Color::srgb_u8(210, 220, 245), 0.0),
            (false, Profile::Casino) => (Color::srgb_u8(210, 220, 245), 1.5),
        };
        if colour.0 != want {
            colour.0 = want;
        }
        if *spacing != LetterSpacing::Px(gap) {
            *spacing = LetterSpacing::Px(gap);
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
            // The black tiles under passing coins, behind everything.
            let (left, width) = coin_tile_span(reel);
            for index in 0..STRIP_SLOTS as u8 {
                col.spawn((
                    StripTile { reel, index },
                    coin_tile(left - (x - column_width() / 2.0), 0.0, width, (true, true)),
                    Visibility::Hidden,
                ));
            }
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
    mut tiles: Query<
        (&StripTile, &mut Node, &mut Visibility),
        (
            Without<StripColumn>,
            Without<StripSlot>,
            Without<StripGhost>,
        ),
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
    for (tile, mut node, mut vis) in &mut tiles {
        let reel = tile.reel as usize;
        let (n, raw) = reels.slot_entry(reel, tile.index);
        let coin = reels.moving(tile.reel) && symbol_of(raw) == "orb";
        let want = if coin {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if coin {
            let cy = p / 2.0 + (n as f32 - reels.states[reel].pos) * p;
            node.top = Val::Px(cy - p / 2.0 - COIN_TILE_FADE);
        }
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

    /// Play a clip once at `volume` (scaled by the global volume, which a
    /// muted run sets to 0).
    pub fn play_at(&self, commands: &mut Commands, name: &str, volume: f32) -> Option<Entity> {
        let (clip, _) = self.clips.get(name)?;
        Some(
            commands
                .spawn((
                    AudioPlayer::new(clip.clone()),
                    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume)),
                ))
                .id(),
        )
    }

    /// Play a cue from a table: a clip and its volume.
    pub fn cue(&self, commands: &mut Commands, cue: Option<(&str, f32)>) {
        if let Some((name, volume)) = cue {
            self.play_at(commands, name, volume);
        }
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

/// The gold separators' cores between the reels on the reel panel (canvas
/// x, measured on the drawn panel): a coin's black tile runs between them,
/// so no sliver of the blue reel face shows beside it.
const SEPARATOR_CORES: [(f32, f32); 6] = [
    (141.0, 149.0),
    (469.0, 474.0),
    (792.0, 797.0),
    (1116.0, 1121.0),
    (1441.0, 1445.0),
    (1766.0, 1771.0),
];
/// How far a coin's tile reaches past its cell, above and below, fading out:
/// the coin's flames run a little past the cell and sit on darkness, not on
/// the blue face (on the cabinet they never show over blue).
const COIN_TILE_FADE: f32 = 14.0;

/// A reel's coin tile: its left edge and width on the canvas.
fn coin_tile_span(reel: u8) -> (f32, f32) {
    let i = (reel as usize).min(SEPARATOR_CORES.len() - 2);
    let left = SEPARATOR_CORES[i].1;
    let right = SEPARATOR_CORES[i + 1].0;
    (left, right - left)
}

/// A coin's black tile at `left`, its cell's `top`, `width` wide: black
/// over the cell, fading out just above and below it, except at the window's
/// own top and bottom edges (`fade`: above, below), where the frame is.
fn coin_tile(left: f32, top: f32, width: f32, fade: (bool, bool)) -> impl Bundle {
    let up = if fade.0 { COIN_TILE_FADE } else { 0.0 };
    let down = if fade.1 { COIN_TILE_FADE } else { 0.0 };
    let h = pitch() + up + down;
    let stop = |color: Color, at: f32| bevy::ui::ColorStop::new(color, Val::Px(at));
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top - up),
            width: Val::Px(width),
            height: Val::Px(h),
            ..default()
        },
        bevy::ui::BackgroundGradient(vec![bevy::ui::Gradient::Linear(
            bevy::ui::LinearGradient::to_bottom(vec![
                stop(if fade.0 { Color::NONE } else { Color::BLACK }, 0.0),
                stop(Color::BLACK, up),
                stop(Color::BLACK, h - down),
                stop(if fade.1 { Color::NONE } else { Color::BLACK }, h),
            ]),
        )]),
    )
}

/// The black tile under a landed coin.
#[derive(Component)]
pub struct CoinTile {
    reel: u8,
    row: u8,
}

/// The black tile under a coin passing on a moving strip.
#[derive(Component)]
pub struct StripTile {
    reel: u8,
    index: u8,
}

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
/// the ink stands about 0.276 of a row tall; the glyph images are tight
/// crops), and the digits drawn a little wider than their images, as the
/// cabinet's are.
const COIN_DIGIT_SHARE: f32 = 0.295;
const COIN_DIGIT_XSTRETCH: f32 = 1.04;
/// "$" stands 0.69 of a digit's height and sits a little low: its centre
/// about 0.045 of the digit height below theirs (cabinet footage, $5 / $10 /
/// $100 coins).
const COIN_DOLLAR_SHARE: f32 = 0.69;
const COIN_DOLLAR_DROP: f32 = 0.09;

/// A coin character's drawn size at `scale`, and its drop below the line's
/// centre (the "$" only).
fn coin_glyph(c: char, w: f32, h: f32, scale: f32) -> (f32, f32, f32) {
    if c == '$' {
        (
            w * scale * COIN_DIGIT_XSTRETCH * COIN_DOLLAR_SHARE,
            h * scale * COIN_DOLLAR_SHARE,
            73.0 * scale * COIN_DOLLAR_DROP,
        )
    } else {
        (w * scale * COIN_DIGIT_XSTRETCH, h * scale, 0.0)
    }
}

/// The scale for a prize's digit images: their own height to `digit_h`, or
/// smaller so the whole prize fits across the coin.
fn coin_digit_scale(text: &str, art: &ArtPack, digit_h: f32) -> f32 {
    let scale = digit_h / 73.0;
    let width: f32 = text
        .chars()
        .filter_map(|c| art.sizes.get(&coin_key(c)).map(|s| (c, s)))
        .map(|(c, (w, h))| coin_glyph(c, *w, *h, scale).0)
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
    mut tiles: Query<(&CoinTile, &mut Visibility), (Without<CoinGlyph>, Without<OrbTier>)>,
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
                art.ui.get(&key).zip(art.sizes.get(&key)).map(|g| (c, g))
            });
        match glyph {
            Some((c, (handle, (w, h)))) => {
                if image.image != *handle {
                    image.image = handle.clone();
                }
                let (gw, gh, drop) = coin_glyph(c, *w, *h, scale);
                node.width = Val::Px(gw);
                node.height = Val::Px(gh);
                node.margin.top = Val::Px(drop);
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
    for (t, mut vis) in &mut tiles {
        let coin = !reels.moving(t.reel)
            && labels
                .iter()
                .any(|(l, text)| l.reel == t.reel && l.row == t.row && symbol_of(&text.0) == "orb");
        let want = if coin {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    // The tier plate sits on the coin a little below its centre, at the
    // size measured on cabinet footage (TIER_PLATE).
    let (cw, ch) = cell_size();
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
        let _ = key;
        let (pw, ph) = TIER_PLATE;
        node.width = Val::Px(pw);
        node.height = Val::Px(ph);
        node.left = Val::Px((cw - pw) / 2.0);
        node.top = Val::Px((ch - ph) / 2.0 + TIER_DROP);
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

/// A MINI / MINOR / MAJOR plate's drawn size, and how far below the coin's
/// centre it sits (cabinet footage: the MINI word 0.62 of the reel's width
/// and 0.23 of a row tall). The same on a landed coin and a moving one, so a
/// plate keeps its size as its reel lands.
const TIER_PLATE: (f32, f32) = (280.0, 219.0);
const TIER_DROP: f32 = 8.0;

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
    mut plates: Query<(&StripTier, &mut ImageNode, &mut Visibility, &mut Node)>,
) {
    if !fidelity.0 {
        return;
    }
    // Centred in the coin's slot (the coin image's rect), at the plate's
    // own size.
    let (sw, sh) = art
        .sizes
        .get("orb")
        .map_or(cell_size(), |s| symbol_size("orb", *s));
    let (pw, ph) = TIER_PLATE;
    for (p, mut image, mut vis, mut node) in &mut plates {
        node.width = Val::Px(pw);
        node.height = Val::Px(ph);
        node.left = Val::Px((sw - pw) / 2.0);
        node.top = Val::Px((sh - ph) / 2.0 + TIER_DROP);
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
                art.ui
                    .get(&key)
                    .cloned()
                    .zip(art.sizes.get(&key).copied())
                    .map(|g| (c, g))
            });
        match glyph {
            Some((c, (handle, (w, h)))) => {
                if image.image != handle {
                    image.image = handle;
                }
                let (gw, gh, drop) = coin_glyph(c, w, h, scale);
                node.width = Val::Px(gw);
                node.height = Val::Px(gh);
                node.margin.top = Val::Px(drop);
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
    /// The progressive jackpots this game won whose amounts the count has
    /// reached (their meters restart then).
    pub link_won: Vec<JackpotTier>,
    /// The free game (number, of how many) a Hold & Spin started inside the
    /// free games interrupted; the status line keeps showing it, and the
    /// free games carry on from it (cabinet footage, y8c8lEdPW8Y).
    fg_return: Option<(u32, u32)>,
    /// The revealed coins' faces ("ORB\nx5", "ORB\nMini"), by position, so
    /// the doors open onto coins with their prizes.
    pub reveal_faces: Vec<(u8, u8, String)>,
    /// What WIN showed when a Hold & Spin's count began: its total counts on
    /// from there (cabinet footage: inside the free games WIN runs on from
    /// the free games' total).
    pay_base: u64,
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
                self.link_won.clear();
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
                if matches!(tier, JackpotTier::Major | JackpotTier::Grand) {
                    self.progressive = true;
                }
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
                    self.clear_hold_and_spin();
                    self.stage = Stage::Base;
                }
                // The bell and the flags' sound have played already (the
                // trigger sequence, before the win was shown).
                self.games_won = *games;
                self.wait_for(Await::Start(Feature::FreeGames));
            }
            Event::MysteryRevealed { symbol, .. } => {
                self.reveal = Some((0.0, symbol.clone(), Vec::new()));
                // The presentation adds the coins' faces, if coins.
                self.reveal_faces.clear();
            }
            Event::FreeGamesResumed { .. } => {
                // Back to the free games after a Hold & Spin one of them
                // started: its board clears, the next free game follows
                // with no START FEATURE (cabinet footage).
                self.clear_hold_and_spin();
                self.game_won = false;
                self.win_lines.clear();
                self.wild = None;
                self.wild_sounded = false;
                if let Some((number, total)) = self.fg_return.take() {
                    self.stage = Stage::FreeGames { number, total };
                }
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
                // A Hold & Spin on the last free game: its board clears for
                // FEATURE COMPLETED (cabinet footage: it follows the tally).
                if matches!(self.stage, Stage::HoldSpinDone) {
                    self.clear_hold_and_spin();
                    self.fg_return = None;
                }
                // Real game: "FEATURE COMPLETED" with SND_Fea_End, the music
                // cut at the same moment.
                self.cues.push("SND_Fea_End");
                self.stage = Stage::FreeGamesDone;
            }
            Event::HoldAndSpinStarted { respins, .. } => {
                // Inside the free games: the free game it interrupts is
                // remembered, and that game's lines are done.
                if let Stage::FreeGames { number, total } = self.stage {
                    self.fg_return = Some((number, total));
                    self.win_lines.clear();
                    self.game_won = false;
                }
                // An earlier Hold & Spin's progressive meters restarted at
                // its count; this one's restart at its own.
                self.link_won.clear();
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

    /// Clear a finished Hold & Spin's board, total and banner (the free
    /// games follow it, or carry on after it). The refusal of the gamble
    /// after a progressive stays.
    fn clear_hold_and_spin(&mut self) {
        self.grid = false;
        self.collected = None;
        self.winner = None;
        self.flares_at = None;
        self.grand = None;
        self.jackpots.clear();
        self.counter = 0;
        self.scatter_paid = false;
        self.paid = 0;
        self.pay_base = 0;
    }

    /// A Hold & Spin started inside the free games is under way.
    #[cfg(test)]
    pub fn inside_free_games(&self) -> bool {
        self.fg_return.is_some()
    }

    /// A feature's end is being shown: its completion message, the count of
    /// its coins or the payment of its total.
    pub fn counting(&self) -> bool {
        self.completed_wait.is_some()
            || self.collect_start
            || self.collect.is_some()
            || self.pay.is_some()
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
            // A Hold & Spin inside the free games keeps the free game on the
            // status line (cabinet footage: "Free Game 3 of 6" over its board).
            (_, Stage::HoldSpin { .. } | Stage::HoldSpinDone) if self.fg_return.is_some() => self
                .fg_return
                .map(|(number, total)| format!("Free Game {number} of {total}")),
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
    /// Each step's coin position, amount, whether it is a jackpot and which,
    /// in order; the Grand, last, has no position (`GRAND_STEP`).
    coins: Vec<(u8, u8, u64, bool, Option<JackpotTier>)>,
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
        if let Some(p) = self.pay.as_ref() {
            let k = (p.t / p.seconds).clamp(0.0, 1.0);
            return Some(self.pay_base + self.paid + (p.amount as f32 * k) as u64);
        }
        // Once paid, WIN holds the sum until the game moves on.
        (self.stage == Stage::HoldSpinDone && self.collected.is_some())
            .then_some(self.pay_base + self.paid)
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

/// A symbol for an open position, from a stable hash (`Roles::respin`:
/// the title's symbols, the coin last).
fn respin_symbol(roles: &Roles, reel: u8, row: u8, n: i64) -> &str {
    let symbols = &roles.respin;
    if symbols.len() < 2 {
        return roles.coin.as_str();
    }
    let h = (n as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(23)
        ^ (u64::from(reel) * 7 + u64::from(row)).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    // Coins pass less often than other symbols.
    let i = (h % (symbols.len() as u64 + 11)) as usize;
    if i >= symbols.len() + 10 {
        roles.coin.as_str()
    } else {
        symbols[i % (symbols.len() - 1)].as_str()
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
fn respin_key(roles: &Roles, reel: u8, row: u8, n: i64, end: i64, coin: bool) -> &str {
    let key = respin_symbol(roles, reel, row, n);
    let is_coin = key == roles.coin;
    if n == end {
        if coin {
            roles.coin.as_str()
        } else if is_coin {
            "Q"
        } else {
            key
        }
    } else if (n - end).abs() == 1 && is_coin {
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
/// The MINI / MINOR / MAJOR plate of a coin landing on a respinning
/// position, so it shows as the coin arrives, as its prize digits do.
#[derive(Component)]
pub struct HsSpinTier;

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
    mut digits: Query<(&HsSpinDigit, &mut ImageNode, &mut Node), Without<HsSpinTier>>,
    mut plates: Query<(&mut ImageNode, &mut Node, &mut Visibility), With<HsSpinTier>>,
) {
    if !fidelity.0 {
        return;
    }
    let Some(anim) = show.respin.as_ref() else {
        return;
    };
    // The plate sits on the coin image as on a landed coin (TIER_PLATE).
    let (ow, oh) = art
        .sizes
        .get("orb")
        .map_or(cell_size(), |s| symbol_size("orb", *s));
    let (pw, ph) = TIER_PLATE;
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
            let plate = (coin && n == end)
                .then(|| landing.as_deref().and_then(tier_image))
                .flatten()
                .and_then(|k| art.ui.get(k));
            for child in children.iter() {
                let Ok((mut image, mut node, mut vis)) = plates.get_mut(child) else {
                    continue;
                };
                match plate {
                    Some(handle) => {
                        if image.image != *handle {
                            image.image = handle.clone();
                        }
                        node.width = Val::Px(pw);
                        node.height = Val::Px(ph);
                        node.left = Val::Px((ow - pw) / 2.0);
                        node.top = Val::Px((oh - ph) / 2.0 + TIER_DROP);
                        *vis = Visibility::Inherited;
                    }
                    None => *vis = Visibility::Hidden,
                }
            }
            if coin && n == end {
                let text = landing
                    .as_deref()
                    .and_then(|f| face_credits(f, total_bet))
                    .map(|c| coin_text(c, cents));
                letter(&art, ch, text.as_deref(), children, &mut digits);
                continue;
            }
            let text = (respin_key(&art.roles, spin.reel, spin.row, n, end, coin)
                == art.roles.coin)
                .then(|| {
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
    digits: &mut Query<(&HsSpinDigit, &mut ImageNode, &mut Node), Without<HsSpinTier>>,
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
                art.ui
                    .get(&key)
                    .cloned()
                    .zip(art.sizes.get(&key).copied())
                    .map(|g| (c, g))
            });
        match glyph {
            Some((c, (h, (w, gh)))) => {
                if image.image != h {
                    image.image = h;
                }
                let (gw, ghh, drop) = coin_glyph(c, w, gh, scale);
                node.width = Val::Px(gw);
                node.height = Val::Px(ghh);
                node.margin.top = Val::Px(drop);
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
                // A landing coin's tier plate.
                img.spawn((
                    HsSpinTier,
                    ZIndex(1),
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
                .any(|(r, w, ..)| *r == reel && *w == row),
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
    rollup: Res<Rollup>,
    art: Res<ArtPack>,
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
                        text.0 = format!(
                            "{DIM}{}",
                            respin_key(&art.roles, reel, row, end, end, false)
                        );
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
                let (amount, tier) = match face_credits(face, total_bet) {
                    Some(amount) => (amount, None),
                    None => jackpots
                        .iter()
                        .position(|(tier, _)| format!("{tier:?}") == face)
                        .map(|i| {
                            let (tier, award) = jackpots.remove(i);
                            (award, Some(tier))
                        })
                        .unwrap_or((0, None)),
                };
                coins.push((
                    reel,
                    row,
                    amount,
                    face_credits(face, total_bet).is_none(),
                    tier,
                ));
            }
        }
        // A full grid's Grand jackpot is added after the coins.
        let grand = jackpots
            .iter()
            .position(|(tier, _)| *tier == JackpotTier::Grand)
            .map(|i| jackpots.remove(i).1);
        if let Some(g) = grand {
            coins.push((GRAND_STEP, GRAND_STEP, g, true, Some(JackpotTier::Grand)));
        }
        show.grand = grand;
        show.winner = None;
        show.paid = 0;
        show.pay_base = rollup.to;
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
            let (reel, row, amount, jackpot, _) = c.coins[c.launched];
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
            let (_, _, amount, _, tier) = c.coins[c.counted];
            show.winner = Some(show.winner.unwrap_or(0) + amount);
            // A progressive's meter restarts as its amount is counted.
            if let Some(tier @ (JackpotTier::Major | JackpotTier::Grand)) = tier {
                show.link_won.push(tier);
            }
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
            let key = respin_key(&art.roles, spin.reel, spin.row, n, end, coin);
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
    let mystery = art.roles.mystery.as_deref().unwrap_or("doors");
    let (w, h) = art
        .sizes
        .get(mystery)
        .map_or((cw, ch), |s| symbol_size(mystery, *s));
    cell.spawn((
        DoorFx { reel, row },
        // Over the coin's prize and plate (ZIndex 1) while the doors open.
        ZIndex(2),
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
#[allow(clippy::too_many_arguments)]
pub fn door_reveal(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut show: ResMut<FeatureShow>,
    sounds: Res<Sounds>,
    art: Res<ArtPack>,
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
            .filter(|(_, text)| Some(text.0.as_str()) == art.roles.mystery.as_deref())
            .map(|(l, _)| (l.reel, l.row))
            .collect()
    } else {
        cells
    };
    if t0 <= REVEAL_OPEN && t > REVEAL_OPEN {
        sounds.play(&mut commands, "SND_EndMask");
        for (l, mut text) in &mut labels {
            if cells.contains(&(l.reel, l.row)) {
                // A coin opens with its own prize; anything else is the
                // one symbol all the doors reveal.
                text.0 = show
                    .reveal_faces
                    .iter()
                    .find(|(reel, row, _)| (*reel, *row) == (l.reel, l.row))
                    .map_or_else(|| symbol.clone(), |(_, _, face)| face.clone());
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
/// The rounded panel that keeps CREDIT on screen over the chooser's splash.
#[derive(Component)]
pub struct ChooserCredit;
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
    // The title and panel are placed on the screen above the band; the
    // splash itself runs down over the band to the screen's foot (footage).
    let h = band_top(profile) - top;
    c.spawn((
        ChooserRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(top),
            width: Val::Px(CANVAS_W),
            height: Val::Px(SCREEN_BOTTOM - top),
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
    // Over the splash only a rounded panel holding CREDIT stays (footage:
    // BET, WIN, the tabs and the denomination box give way).
    c.spawn((
        ChooserCredit,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(200.0),
            top: Val::Px(band_top(profile)),
            width: Val::Px(545.0),
            height: Val::Px(100.0),
            border_radius: BorderRadius::all(Val::Px(16.0)),
            ..default()
        },
        BackgroundColor(Color::srgba_u8(18, 18, 34, 245)),
        GlobalZIndex(13),
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
            Without<ChooserCredit>,
            Without<ChooserPanel>,
        ),
    >,
    mut covers: Query<&mut Visibility, Or<(With<ChooserRoot>, With<ChooserCredit>)>>,
    mut panel: Query<
        &mut Visibility,
        (
            With<ChooserPanel>,
            Without<ChooserRoot>,
            Without<ChooserCredit>,
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
    /// What was at stake before the card now showing: WIN shows it until the
    /// card turns.
    before: u64,
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
    /// What WIN shows in the gamble: the stake, or before the card has
    /// turned what was at stake when it was chosen.
    pub fn shown_stake(&self) -> u64 {
        match self.card {
            Some((_, _, t)) if t < GAMBLE_FACE_DELAY => self.before,
            _ => self.stake,
        }
    }

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
    sounds: Res<Sounds>,
    mut commands: Commands,
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
            sounds.cue(&mut commands, gamble_cue(GambleCue::Open));
        }
        return;
    }
    gamble.since += dt;
    // GAMBLE again chooses between the full and the half gamble.
    if input.gamble && gamble.card.is_none() && gamble.stake >= 2 {
        gamble.half = !gamble.half;
        sounds.cue(&mut commands, gamble_cue(GambleCue::Toggle));
    }
    // Every press while gambling belongs to the gamble.
    let take = input.take_win
        || input.spin
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter);
    input.skip_spin = true;
    if let Some((card, won, t0)) = gamble.card {
        let t = t0 + dt;
        gamble.card = Some((card, won, t));
        // The card turns a moment after the choice; its outcome sounds a
        // moment after that.
        let at = GAMBLE_FACE_DELAY + GAMBLE_SOUND_AFTER_FACE;
        if t0 < at && t >= at {
            let cue = if won { GambleCue::Won } else { GambleCue::Lost };
            sounds.cue(&mut commands, gamble_cue(cue));
        }
        let done = GAMBLE_FACE_DELAY
            + if won {
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
                        gamble.before = gamble.stake;
                        gamble.stake = *award;
                        sounds.cue(&mut commands, gamble_cue(GambleCue::Pick));
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
        sounds.cue(&mut commands, gamble_cue(GambleCue::TakeWin));
    }
}

/// The gamble screen and its parts.
#[derive(Component)]
pub struct GambleRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GambleText {
    /// "Full Gamble Selected" or "Half Gamble Selected".
    Selected,
    /// "Red/Black Gamble" / "Gamble Amount" over the amount.
    AmountCaption,
    AmountValue,
    /// "Red/Black Gamble" / "to win" over what it would win.
    ToWinCaption,
    ToWinValue,
    /// "Gamble Win" over the last win, from the card's turn into the next
    /// round.
    WinCaption,
    WinValue,
    /// "Select a SUIT, RED/BLACK or TAKE WIN", in the status line.
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

/// The gamble screen's layout on the canvas, measured on cabinet footage by
/// homography from the Hold & Spin coins just before each gamble
/// (_XHUzg4qDXI, iHDxUc1dlKs; GOLDEN_CENTURY_CABINET_DECK.md): the columns'
/// centres, then each part's centre and size.
const GAMBLE_LX: f32 = 519.0;
const GAMBLE_RX: f32 = 1401.0;
const GAMBLE_PILL: (f32, f32, f32) = (351.0, 510.0, 55.0);
const GAMBLE_COLOUR: (f32, f32, f32) = (600.0, 502.0, 80.0);
const GAMBLE_SUIT: (f32, f32, f32) = (720.0, 198.0, 100.0);
/// The suit buttons' centres either side of their column.
const GAMBLE_SUIT_DX: f32 = 116.0;
const GAMBLE_CARD: (f32, f32, f32) = (655.0, 303.0, 361.0);
const GAMBLE_STAMP: (f32, f32, f32) = (731.0, 415.0, 125.0);
/// The side blocks' caption and value line pairs, and the Gamble Win
/// header's caption and values (line centres, 35 px apart).
const GAMBLE_CAPTION_Y: f32 = 431.5;
const GAMBLE_VALUE_Y: f32 = 501.5;
const GAMBLE_WIN_CAPTION_Y: f32 = 363.0;
const GAMBLE_WIN_VALUE_Y: f32 = 417.5;
const GAMBLE_LINE: f32 = 35.0;
/// Seconds from a choice to the card turning (footage: 0.10-0.17 s after the
/// finger lifts, 0.36-0.40 s after it first touches; a click here registers
/// as it is pressed, so the shorter delay; 0.35 s felt late).
pub const GAMBLE_FACE_DELAY: f32 = 0.15;

/// The gamble screen's captions: upright bold, pale blue-white, with a dark
/// navy edge (cabinet footage).
fn gamble_caption(s: &str, font: &Handle<Font>, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: bevy::text::FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
        TextLayout::justify(Justify::Center),
        LineHeight::Px(GAMBLE_LINE),
        TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::srgba(0.06, 0.08, 0.2, 0.9),
        },
    )
}

/// The gamble's amounts: a heavier, wider face in cream.
fn gamble_value(font: &Handle<Font>) -> impl Bundle {
    gamble_caption("", font, 39.0, Color::srgb(0.98, 0.965, 0.784))
}

/// Build the gamble screen (hidden until a gamble): the game's backdrop, the
/// banners, the two columns either side of the card, the history.
pub fn spawn_gamble_screen(
    c: &mut ChildSpawnerCommands,
    art: &ArtPack,
    fonts: &CabFonts,
    profile: Profile,
) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    // The whole screen between the jackpots and the meter band; every y
    // below is a canvas y, made relative to this top.
    let top = JACKPOT_BAR_TOP + JACKPOT_BAR_W * 104.0 / 1225.0 + 4.0;
    let h = band_top(profile) - top;
    let at = |cx: f32, cy: f32, bw: f32, bh: f32| Node {
        position_type: PositionType::Absolute,
        left: Val::Px(cx - bw / 2.0),
        top: Val::Px(cy - top - bh / 2.0),
        width: Val::Px(bw),
        height: Val::Px(bh),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let caption = Color::srgb(0.91, 0.957, 1.0);
    let banner = Color::srgb(0.843, 0.929, 1.0);
    c.spawn((
        GambleRoot,
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
    .with_children(|g| {
        // The game's own backdrop, undimmed and placed as in the base game.
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(-30.0 - top),
                width: Val::Px(1920.0),
                height: Val::Px(1242.0),
                ..default()
            },
            ImageNode {
                image: ui("background"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        // The thin silver rule between the two banners, at their top edge.
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(764.0),
                top: Val::Px(324.0 - top - 2.5),
                width: Val::Px(392.0),
                height: Val::Px(5.0),
                border: UiRect::vertical(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.96, 0.98, 1.0)),
            BorderColor::all(Color::srgb(0.1, 0.13, 0.19)),
        ));
        let (py, pw, ph) = GAMBLE_PILL;
        // Left banner: "Full Gamble Selected".
        g.spawn((
            at(GAMBLE_LX, py, pw, ph),
            ImageNode {
                image: ui("pill"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                GambleText::Selected,
                gamble_caption("Full Gamble Selected", &fonts.sans, 36.0, banner),
            ));
        });
        // Right banner: HISTORY and the last five cards' suits, newest left.
        g.spawn((
            at(GAMBLE_RX, py, pw, ph),
            ImageNode {
                image: ui("pill"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1212.0),
                top: Val::Px(py - top - GAMBLE_LINE / 2.0),
                ..default()
            },
            gamble_caption("HISTORY", &fonts.sans, 36.0, banner),
        ));
        for i in 0..HISTORY {
            g.spawn((
                GambleHistory(i),
                Text::new(""),
                TextFont {
                    font: bevy::text::FontSource::Handle(fonts.symbols.clone()),
                    font_size: FontSize::Px(31.0),
                    ..default()
                },
                TextColor(Color::BLACK),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1404.0 + 44.0 * i as f32 - 20.0),
                    top: Val::Px(py + 2.0 - top - 20.0),
                    width: Val::Px(40.0),
                    ..default()
                },
            ));
        }
        // The two columns: caption, amount, colour button, suit buttons.
        for (cx, caption_kind, value_kind, colour, suits) in [
            (
                GAMBLE_LX,
                GambleText::AmountCaption,
                GambleText::AmountValue,
                GamblePick::Red,
                [GamblePick::Hearts, GamblePick::Diamonds],
            ),
            (
                GAMBLE_RX,
                GambleText::ToWinCaption,
                GambleText::ToWinValue,
                GamblePick::Black,
                [GamblePick::Clubs, GamblePick::Spades],
            ),
        ] {
            g.spawn(at(cx, GAMBLE_CAPTION_Y, 520.0, 2.0 * GAMBLE_LINE))
                .with_children(|t| {
                    t.spawn((caption_kind, gamble_caption("", &fonts.sans, 36.0, caption)));
                });
            g.spawn(at(cx, GAMBLE_VALUE_Y, 520.0, 2.0 * GAMBLE_LINE))
                .with_children(|t| {
                    t.spawn((value_kind, gamble_value(&fonts.heavy)));
                });
            let (y, w, bh) = GAMBLE_COLOUR;
            gamble_button(g, art, colour, cx, y - top, w, bh);
            let (y, w, bh) = GAMBLE_SUIT;
            gamble_button(g, art, suits[0], cx - GAMBLE_SUIT_DX, y - top, w, bh);
            gamble_button(g, art, suits[1], cx + GAMBLE_SUIT_DX, y - top, w, bh);
        }
        // Middle: "Gamble Win" over the card, "WIN" across it.
        g.spawn(at(960.0, GAMBLE_WIN_CAPTION_Y, 400.0, GAMBLE_LINE))
            .with_children(|t| {
                t.spawn((
                    GambleText::WinCaption,
                    gamble_caption("", &fonts.sans, 36.0, caption),
                ));
            });
        g.spawn(at(960.0, GAMBLE_WIN_VALUE_Y, 400.0, 2.0 * GAMBLE_LINE))
            .with_children(|t| {
                t.spawn((GambleText::WinValue, gamble_value(&fonts.heavy)));
            });
        let (y, w, ch) = GAMBLE_CARD;
        g.spawn((
            GambleCard,
            at(960.0, y, w, ch),
            ImageNode {
                image: ui("card_back"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ));
        let (y, w, sh) = GAMBLE_STAMP;
        g.spawn((
            GambleStamp,
            at(960.0, y, w, sh),
            ImageNode {
                image: ui("win_stamp"),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(13),
            Visibility::Hidden,
        ));
        // The prompt takes the status line's place and style.
        g.spawn((
            GambleText::Prompt,
            status_text("", fonts, profile, Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(STATUS_X - 600.0),
                top: Val::Px(status_y(profile) - top - 14.0),
                width: Val::Px(1200.0),
                ..default()
            },
        ));
    });
}

/// A gamble button on the screen: the silver RED / BLACK capsules and the
/// suit ovals, gold-capped (`cy` is relative to the gamble screen's top).
fn gamble_button(
    g: &mut ChildSpawnerCommands,
    art: &ArtPack,
    pick: GamblePick,
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
) {
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
            image: art.ui.get(button_art(pick)).cloned().unwrap_or_default(),
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

/// A gamble button's art.
fn button_art(pick: GamblePick) -> &'static str {
    match pick {
        GamblePick::Red => "btn_red",
        GamblePick::Black => "btn_black",
        GamblePick::Hearts => "suit_hearts",
        GamblePick::Diamonds => "suit_diamonds",
        GamblePick::Clubs => "suit_clubs",
        GamblePick::Spades => "suit_spades",
    }
}

/// A suit button with no suit, for when suits are not offered: the gold cap
/// at its outer end (hearts and clubs at the left).
fn blank_suit_art(pick: GamblePick) -> &'static str {
    match pick {
        GamblePick::Hearts | GamblePick::Clubs => "suit_blank_left",
        _ => "suit_blank_right",
    }
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
    // The card turns a moment after the choice; until then its back shows.
    let turned = shown.filter(|(_, _, t)| *t >= GAMBLE_FACE_DELAY);
    let key = turned.map_or("card_back".to_string(), |(c, _, _)| format!("card_{c}"));
    if let Some(h) = art.ui.get(&key) {
        for mut image in &mut card {
            if image.image != *h {
                image.image = h.clone();
            }
        }
    }
    let won = turned.is_some_and(|(_, w, _)| w);
    for mut v in &mut stamp {
        set(&mut v, won);
    }
    for (h, mut t, mut c) in &mut history {
        let s = gamble.history.get(h.0).map(|card| card_suit(*card));
        let txt = s.map(|(s, _)| s.to_string()).unwrap_or_default();
        if t.0 != txt {
            t.0 = txt;
        }
        let want = if s.is_some_and(|(_, red)| red) {
            Color::srgb(0.88, 0.08, 0.1)
        } else {
            Color::srgb(0.06, 0.06, 0.08)
        };
        if c.0 != want {
            c.0 = want;
        }
    }
    // Both boxes name the same kind: Suit, then Red/Black, 2 s each from the
    // gamble's opening; only Red/Black when a suit would pass the limit.
    // While a card is up the screen shows the gamble just made.
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
    // The chosen button fills pale yellow while its card shows; suits not
    // offered show their blank capsule.
    let chosen = shown.and(gamble.last.map(|(p, _)| p));
    for (b, mut image) in &mut buttons {
        let base = button_art(b.0);
        let want = if !is_colour(b.0) && !suits_open {
            art.ui.get(blank_suit_art(b.0))
        } else if chosen == Some(b.0) {
            art.ui.get(&format!("{base}_picked"))
        } else {
            None
        }
        .or_else(|| art.ui.get(base));
        if let Some(h) = want
            && image.image != *h
        {
            image.image = h.clone();
        }
        let tint = if !is_colour(b.0) && !suits_open && !art.ui.contains_key(blank_suit_art(b.0)) {
            Color::srgba(1.0, 1.0, 1.0, 0.35)
        } else {
            Color::WHITE
        };
        if image.color != tint {
            image.color = tint;
        }
    }
    let amount = |credits: u64| format!("{credits}\n{}", money(credits, cents));
    let last_win = if won {
        Some(gamble.stake)
    } else {
        gamble.last_win
    };
    for (which, mut t) in &mut texts {
        let s = match which {
            GambleText::Selected if gamble.half => "Half Gamble Selected".to_string(),
            GambleText::Selected => "Full Gamble Selected".to_string(),
            GambleText::AmountCaption => format!("{kind}\nGamble Amount"),
            GambleText::AmountValue => amount(staked),
            GambleText::ToWinCaption => format!("{kind}\nto win"),
            GambleText::ToWinValue => amount(staked.saturating_mul(mult)),
            // The last win's header stays into the next round (footage); it
            // waits for the card to turn.
            GambleText::WinCaption
                if last_win.is_some() && (won || turned.is_none() && shown.is_none()) =>
            {
                "Gamble Win".to_string()
            }
            GambleText::WinValue
                if last_win.is_some() && (won || turned.is_none() && shown.is_none()) =>
            {
                amount(last_win.unwrap_or(0))
            }
            GambleText::WinCaption | GambleText::WinValue => String::new(),
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
    art: Res<ArtPack>,
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
    if show.hold_and_spin() {
        // The trigger game's stacks are over once Hold & Spin starts; they
        // must not stand again after it.
        if !stacks.list.is_empty() || !stacks.landed.is_empty() {
            *stacks = WildStacks {
                found: true,
                ..default()
            };
        }
        return;
    }
    if show.reveal.is_some() {
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
                column[l.row as usize] = key == art.roles.wild && !dim;
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
    show: Res<FeatureShow>,
    mut talls: Query<(&WildTall, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    // Not over a Hold & Spin board.
    if show.hold_and_spin() {
        for (_, _, _, mut vis) in &mut talls {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
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
            if t.0 == art.roles.scatter {
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
                let fps = if key == art.roles.scatter {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_jackpot_value_fits_its_box() {
        let k = JACKPOT_BAR_W / 1225.0;
        for (tier, (left, right)) in JACKPOT_BOXES {
            let width = (right - left) * k;
            for text in [
                "$10.00",
                "$250.00",
                "$1,250.00",
                "$1,500.00",
                "$15,000.00",
                "$150,000.00",
            ] {
                let size = jackpot_value_size(text, width);
                assert!(size <= JACKPOT_VALUE_SIZE);
                assert!(
                    meter_ems(text) * size <= width * JACKPOT_VALUE_FILL + 0.01,
                    "{tier:?} {text}"
                );
            }
        }
        // A value too wide for its box is set smaller, to fit.
        let mini = (1209.0 - 975.0) * k;
        let size = jackpot_value_size("$1,234,567.89", mini);
        assert!(size < JACKPOT_VALUE_SIZE);
        assert!(meter_ems("$1,234,567.89") * size <= mini * JACKPOT_VALUE_FILL + 0.01);
        // The values the bar shows today keep the full size.
        assert_eq!(
            jackpot_value_size("$250.00", (1209.0 - 975.0) * k),
            JACKPOT_VALUE_SIZE
        );
        assert_eq!(
            jackpot_value_size("$15,000.00", (351.0 - 14.0) * k),
            JACKPOT_VALUE_SIZE
        );
    }
}
