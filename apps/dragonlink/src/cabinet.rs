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
/// (A title whose lines tabs are part of its reel art, Golden Gong, has no
/// `payline_tab`; it is not needed.)
pub const REQUIRED_UI: [&str; 22] = [
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
    /// The "50" of "50 LINES" either side of the reels.
    Lines,
    /// Its "LINES" ("LINE" for one).
    LinesWord,
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

/// This app's machine chooser, not a cabinet feature: every title with a
/// local pack, opened from the yellow pill at the band's left. Choosing
/// another title restarts the app as that machine, the money carried over.
#[derive(Resource, Default)]
pub struct MachineMenu {
    pub open: bool,
    /// (slug, name) of each title with a pack, in the launcher's order.
    pub entries: Vec<(String, String)>,
    /// The running title's slug.
    pub current: String,
    /// A title chosen, for `main` to switch to.
    pub chosen: Option<String>,
}

/// The chooser's overlay.
#[derive(Component)]
pub struct MachineMenuRoot;

/// Build the machine chooser, hidden until the pill is touched.
pub fn spawn_machine_menu(commands: &mut Commands, fonts: &CabFonts, menu: &MachineMenu) {
    let gold = Color::srgb(1.0, 0.86, 0.12);
    commands
        .spawn((
            CanvasRoot,
            MachineMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(CANVAS_W),
                height: Val::Px(CANVAS_H),
                ..default()
            },
            GlobalZIndex(60),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
            ));
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(440.0),
                    top: Val::Px(150.0),
                    width: Val::Px(1040.0),
                    border: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(30.0)),
                    padding: UiRect::all(Val::Px(32.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(28.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.05, 0.06, 0.16)),
                BorderColor::all(gold),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("CHOOSE YOUR MACHINE"),
                    TextFont {
                        font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                        font_size: FontSize::Px(52.0),
                        ..default()
                    },
                    TextColor(gold),
                ));
                p.spawn(Node {
                    width: Val::Px(960.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(28.0),
                    row_gap: Val::Px(24.0),
                    ..default()
                })
                .with_children(|g| {
                    for (slug, name) in &menu.entries {
                        let here = *slug == menu.current;
                        let slug = slug.clone();
                        g.spawn((
                            Node {
                                width: Val::Px(440.0),
                                height: Val::Px(92.0),
                                border: UiRect::all(Val::Px(3.0)),
                                border_radius: BorderRadius::all(Val::Px(46.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(if here {
                                Color::srgb(0.55, 0.38, 0.04)
                            } else {
                                Color::srgb(0.1, 0.2, 0.55)
                            }),
                            BorderColor::all(if here {
                                gold
                            } else {
                                Color::srgb(0.6, 0.7, 1.0)
                            }),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(name.to_uppercase()),
                                TextFont {
                                    font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                                    font_size: FontSize::Px(34.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        })
                        .observe(
                            move |_press: On<PointerPress>, mut menu: ResMut<MachineMenu>| {
                                if menu.open {
                                    menu.chosen = Some(slug.clone());
                                }
                            },
                        );
                    }
                });
                p.spawn((
                    Node {
                        width: Val::Px(260.0),
                        height: Val::Px(70.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(35.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
                    BorderColor::all(Color::srgb(0.75, 0.75, 0.8)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("BACK"),
                        TextFont {
                            font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                })
                .observe(
                    |_press: On<PointerPress>, mut menu: ResMut<MachineMenu>| {
                        menu.open = false;
                    },
                );
            });
        });
}

/// Show or hide the machine chooser.
pub fn machine_menu(
    menu: Res<MachineMenu>,
    mut roots: Query<&mut Visibility, With<MachineMenuRoot>>,
) {
    let want = if menu.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut roots {
        if *vis != want {
            *vis = want;
        }
    }
}

/// The game's rules pages: the title's own help pages from its local pack
/// (`ui/help_NN`, in the help scene's order), over its background, with the
/// mobile layout's BACK / EXIT / NEXT buttons. On the cabinet GAME RULES
/// opens them and each further press turns the page; any other press closes
/// them; they close themselves after 120 s (the shared framework, VERIFIED
/// as code). No club recording shows the pages, so their order and layout
/// follow the mobile build (HYPOTHESIS for the cabinet). Keyboard (this
/// app's own): I or F1 opens, the arrows or Page Up/Down turn, Esc closes.
#[derive(Resource, Default)]
pub struct GameRules {
    pub open: bool,
    pub page: usize,
    idle: f32,
    /// The GAME RULES pill was touched.
    pub pressed: bool,
    /// A press closed the rules and must do nothing else.
    pub swallow: bool,
    /// A page button was touched: -1 back, 0 exit, 1 next.
    pub button: Option<i8>,
}

const RULES_IDLE_SECONDS: f32 = 120.0;

/// The rules overlay and its page.
#[derive(Component)]
pub struct RulesRoot;
#[derive(Component)]
pub struct RulesPage;

/// The title's help pages, in order.
pub fn help_pages(art: &ArtPack) -> Vec<String> {
    let mut pages: Vec<String> = art
        .ui
        .keys()
        .filter(|k| {
            k.strip_prefix("help_")
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .cloned()
        .collect();
    pages.sort();
    pages
}

/// The next page after `page` of `count`, `step` pages on, wrapping both ways.
pub fn turn_page(page: usize, count: usize, step: i64) -> usize {
    if count == 0 {
        return 0;
    }
    (page as i64 + step).rem_euclid(count as i64) as usize
}

/// Build the rules overlay, hidden.
pub fn spawn_rules(commands: &mut Commands, art: &ArtPack) {
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    commands
        .spawn((
            CanvasRoot,
            RulesRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(CANVAS_W),
                height: Val::Px(SCREEN_BOTTOM),
                overflow: Overflow::clip(),
                ..default()
            },
            GlobalZIndex(50),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode {
                    image: ui("background"),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            c.spawn((
                RulesPage,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(960.0 - 870.0),
                    top: Val::Px(120.0),
                    width: Val::Px(1740.0),
                    height: Val::Px(777.0),
                    ..default()
                },
                ImageNode {
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            for (step, x, key) in [
                (-1i8, 713.0, "help_back"),
                (0, 960.0, "help_exit"),
                (1, 1207.0, "help_next"),
            ] {
                c.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x - 80.0),
                        top: Val::Px(1000.0 - 63.0),
                        width: Val::Px(160.0),
                        height: Val::Px(126.0),
                        ..default()
                    },
                    ImageNode {
                        image: ui(key),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .observe(
                    move |_press: On<PointerPress>, mut rules: ResMut<GameRules>| {
                        rules.button = Some(step);
                    },
                );
            }
        });
}

/// Open, turn and close the rules pages.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn game_rules(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    keys: Res<ButtonInput<KeyCode>>,
    input: Res<CabinetInput>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    menu: Res<GamesMenu>,
    sounds: Res<Sounds>,
    mut commands: Commands,
    mut rules: ResMut<GameRules>,
    mut roots: Query<&mut Visibility, With<RulesRoot>>,
    mut page: Query<&mut ImageNode, With<RulesPage>>,
) {
    if !fidelity.0 {
        return;
    }
    let pages = help_pages(&art);
    let at_rest = sim.game.is_idle()
        && presentation.queue.is_empty()
        && !show.in_feature()
        && !gamble.active
        && !menu.showing();
    let open_key = keys.just_pressed(KeyCode::KeyI) || keys.just_pressed(KeyCode::F1);
    // Testing (`DRAGONLINK_SHOW=rules`, an unattended run only): the rules
    // open two seconds in and turn a page three seconds later.
    let shown = smoke.frames_left.is_some()
        && matches!(smoke.frame, 120 | 300)
        && std::env::var("DRAGONLINK_SHOW").as_deref() == Ok("rules");
    let open_key = open_key || shown;
    let pill = std::mem::take(&mut rules.pressed) || open_key;
    let button = rules.button.take();
    if !rules.open {
        if pill && (at_rest || shown) && !pages.is_empty() {
            rules.open = true;
            rules.page = 0;
            rules.idle = 0.0;
            sounds.play(&mut commands, "ButtonClick_reduced");
        }
    } else {
        let mut step = None;
        if pill {
            step = Some(1);
        }
        if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::PageDown) {
            step = Some(1);
        }
        if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::PageUp) {
            step = Some(-1);
        }
        let mut close = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace);
        match button {
            Some(0) => close = true,
            Some(b) => step = Some(i64::from(b)),
            None => {}
        }
        // Any other press closes the rules and does nothing else.
        let other = input.spin
            || input.menu
            || input.gamble
            || input.take_win
            || input.bet.is_some()
            || input.credits.is_some()
            || input.lines.is_some()
            || keys.just_pressed(KeyCode::Space)
            || keys.just_pressed(KeyCode::Enter);
        if other {
            close = true;
            rules.swallow = true;
        }
        if keys.just_pressed(KeyCode::Escape) {
            rules.swallow = true;
        }
        if let Some(step) = step {
            rules.page = turn_page(rules.page, pages.len(), step);
            rules.idle = 0.0;
            sounds.play(&mut commands, "GenericButton");
        }
        rules.idle += smoke.dt(&time);
        if close || rules.idle >= RULES_IDLE_SECONDS || pages.is_empty() {
            if button == Some(0) {
                sounds.play(&mut commands, "GenericButton");
            }
            rules.open = false;
        }
    }
    let want = if rules.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in &mut roots {
        if *v != want {
            *v = want;
        }
    }
    if rules.open
        && let Some(h) = pages.get(rules.page).and_then(|k| art.ui.get(k))
    {
        for mut image in &mut page {
            if image.image != *h {
                image.image = h.clone();
            }
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

/// Attract and the chooser's top screen, from cabinet footage of club
/// machines (research notes, GOLDEN_CENTURY_CABINET_DECK.md "Attract and
/// idle"):
/// - An idle machine at zero credit is found on the denomination chooser
///   (VERIFIED, two recordings); the delay before it opens is not filmed, so
///   the shared framework's 30 s default is used (HYPOTHESIS), or
///   `DRAGONLINK_ATTRACT_SECONDS`. No demo spins, no attract sound.
/// - While the chooser shows, a PLAY plaque between the Minor and Mini steps
///   1c, 2c, 5c, 10c, $1 and wraps, with that denomination's Minor and Mini:
///   the first step 1.1 s after it opens, then every 4.0 s (VERIFIED: 14
///   openings and one period); at a step the label changes at +0.10 s, the
///   values at +0.30 s, a white flare lasts 0.25 s. Fire runs round the boxes
///   the whole time (STRONG; the exact route is not followed, HYPOTHESIS).
/// - After a choice the Minor, Mini and plaque go blank for about 0.25 s.
pub const ATTRACT_SECONDS: f32 = 30.0;
const ATTRACT_FIRST_STEP: f32 = 1.1;
const ATTRACT_STEP: f32 = 4.0;
const ATTRACT_LABEL_LAG: f32 = 0.10;
const ATTRACT_VALUE_LAG: f32 = 0.30;
const ATTRACT_FLARE: f32 = 0.25;

/// The step a chooser `t` seconds open shows (0 from the opening, 1 from the
/// first step, and so on), `lag` seconds after each step begins.
pub fn attract_step(t: f32, lag: f32) -> usize {
    let t = t - lag;
    if t < ATTRACT_FIRST_STEP {
        0
    } else {
        1 + ((t - ATTRACT_FIRST_STEP) / ATTRACT_STEP) as usize
    }
}

/// How bright the step flare is `t` seconds after the chooser opened (0 to 1).
pub fn attract_flare(t: f32) -> f32 {
    if t < ATTRACT_FIRST_STEP {
        return 0.0;
    }
    let since = (t - ATTRACT_FIRST_STEP) % ATTRACT_STEP;
    if since < ATTRACT_FLARE {
        1.0 - since / ATTRACT_FLARE
    } else {
        0.0
    }
}

/// The idle timer of attract: the seconds idle so far after `dt` more, and
/// whether the chooser opens now. It runs only while `armed` (the game at
/// rest with no credit) and starts again on any `input`.
pub fn attract_tick(idle: f32, dt: f32, timeout: f32, armed: bool, input: bool) -> (f32, bool) {
    if !armed || input {
        return (0.0, false);
    }
    let idle = idle + dt;
    if idle >= timeout {
        (0.0, true)
    } else {
        (idle, false)
    }
}

/// Seconds the machine has sat idle at zero credit.
#[derive(Resource, Default)]
pub struct AttractIdle(f32);

/// The PLAY plaque and its parts, the flare over it, and the fire round the
/// Minor and Mini boxes.
#[derive(Component)]
pub struct PlayPlaque;
#[derive(Component)]
pub struct PlayPlaqueDenom;
#[derive(Component)]
pub struct PlayPlaqueFlare;
#[derive(Component)]
pub struct AttractFire;

/// Spawn the chooser's additions to the jackpot bar, hidden.
fn spawn_attract_bar(c: &mut ChildSpawnerCommands, font: &Handle<Font>, bar_top: f32, k: f32) {
    let left = 960.0 - JACKPOT_BAR_W / 2.0;
    // The fire over the Minor and Mini boxes.
    for (tier, (l, r)) in JACKPOT_BOXES {
        if !matches!(tier, JackpotTier::Minor | JackpotTier::Mini) {
            continue;
        }
        // A little larger than the box, so the flames run round it and
        // clear its title and value.
        let (w, h) = ((r - l + 36.0) * k, 132.0 * k);
        c.spawn((
            AttractFire,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left + (l - 18.0) * k),
                top: Val::Px(bar_top - 12.0 * k),
                width: Val::Px(w),
                height: Val::Px(h),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            ZIndex(3),
            Visibility::Hidden,
        ));
    }
    // The plaque, centred on the seam between the Minor and Mini boxes.
    let (pw, ph) = (118.0, 76.0);
    let cx = left + 968.0 * k;
    let cy = bar_top + 62.0 * k;
    c.spawn((
        PlayPlaque,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - pw / 2.0),
            top: Val::Px(cy - ph / 2.0),
            width: Val::Px(pw),
            height: Val::Px(ph),
            border: UiRect::all(Val::Px(4.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.03, 0.02)),
        BorderColor::all(Color::srgb(0.95, 0.74, 0.2)),
        ZIndex(4),
        Visibility::Hidden,
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("PLAY"),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.4)),
        ));
        p.spawn((
            Node {
                width: Val::Px(92.0),
                height: Val::Px(36.0),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.72, 0.06, 0.05)),
        ))
        .with_children(|b| {
            b.spawn((
                PlayPlaqueDenom,
                Text::new("1c"),
                TextFont {
                    font: bevy::text::FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(30.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
        // The step's white flare, over the whole plaque.
        p.spawn((
            PlayPlaqueFlare,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-14.0),
                top: Val::Px(-14.0),
                width: Val::Px(pw + 20.0),
                height: Val::Px(ph + 20.0),
                border_radius: BorderRadius::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 0.95, 0.0)),
        ));
    });
}

/// Open the chooser after the machine has sat idle at zero credit.
#[allow(clippy::too_many_arguments)]
pub fn attract_timer(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    input: Res<CabinetInput>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    gamble: Res<GambleShow>,
    show: Res<FeatureShow>,
    mut menu: ResMut<GamesMenu>,
    mut idle: ResMut<AttractIdle>,
) {
    if !fidelity.0 {
        return;
    }
    let timeout = std::env::var("DRAGONLINK_ATTRACT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|s| *s > 0.0)
        .unwrap_or(ATTRACT_SECONDS);
    let armed = sim.game.is_idle()
        && presentation.queue.is_empty()
        && !gamble.active
        && !show.in_feature()
        && !menu.showing()
        && sim.game.credits() == 0
        && sim.game.state().residue == 0;
    let pressed = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || input.spin
        || input.menu
        || input.gamble
        || input.take_win
        || input.bet.is_some()
        || input.credits.is_some()
        || input.lines.is_some()
        || input.pick.is_some();
    let (left, open) = attract_tick(idle.0, smoke.dt(&time), timeout, armed, pressed);
    idle.0 = left;
    if open {
        menu.open_now();
    }
}

/// The jackpot bar while the chooser shows: the PLAY plaque steps through
/// the denominations with each one's Minor and Mini, and fire runs round
/// their boxes; after a choice they go blank a moment.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn attract_bar(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    menu: Res<GamesMenu>,
    deck: Res<DeckState>,
    sim: Res<Sim>,
    art: Res<ArtPack>,
    mut values: Query<(&JackpotMeter, &mut Text), Without<PlayPlaqueDenom>>,
    mut plaque: Query<&mut Visibility, (With<PlayPlaque>, Without<AttractFire>)>,
    mut denom_text: Query<&mut Text, (With<PlayPlaqueDenom>, Without<JackpotMeter>)>,
    mut flare: Query<&mut BackgroundColor, With<PlayPlaqueFlare>>,
    mut fire: Query<(&mut ImageNode, &mut Visibility), (With<AttractFire>, Without<PlayPlaque>)>,
) {
    if !fidelity.0 {
        return;
    }
    let open = menu.open;
    let want = if open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut plaque {
        if *v != want {
            *v = want;
        }
    }
    let frames = art.anims.get("flameborder_sm");
    let frame = frames.and_then(|f| {
        let n = (time.elapsed_secs() * ANIM_FPS) as usize;
        f.get(n % f.len().max(1)).cloned()
    });
    for (mut image, mut v) in &mut fire {
        let show = open && frame.is_some();
        if let (true, Some(h)) = (show, &frame)
            && image.image != *h
        {
            image.image = h.clone();
        }
        let w = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != w {
            *v = w;
        }
    }
    if menu.closing() {
        // After a choice: blank for a moment, then the plain bar.
        for (m, mut text) in &mut values {
            if matches!(m.tier, JackpotTier::Minor | JackpotTier::Mini) && !text.0.is_empty() {
                text.0.clear();
            }
        }
        return;
    }
    if !open {
        return;
    }
    let denoms = deck.profile.denoms();
    if denoms.is_empty() {
        return;
    }
    let t = menu.since();
    let label = denoms[attract_step(t, ATTRACT_LABEL_LAG) % denoms.len()];
    let shown = denoms[attract_step(t, ATTRACT_VALUE_LAG) % denoms.len()];
    for mut text in &mut denom_text {
        let want = denom_text_short(label);
        if text.0 != want {
            text.0 = want;
        }
    }
    let alpha = attract_flare(t) * 0.85;
    for mut bg in &mut flare {
        bg.0 = Color::srgba(1.0, 1.0, 0.95, alpha);
    }
    let model = sim.game.model();
    for (m, mut text) in &mut values {
        if !matches!(m.tier, JackpotTier::Minor | JackpotTier::Mini) {
            continue;
        }
        let want = fixed_prize(model, m.tier, shown)
            .map_or_else(|| "-".to_string(), |credits| money(credits, shown));
        if text.0 != want {
            text.0 = want;
        }
    }
}

/// The fire round the jackpot boxes is drawn by the game additively: its
/// frames are flames on opaque black. Once loaded, each frame's brightness
/// becomes its alpha (and its colour is brightened to match), so the flames
/// lie over the bar with the black gone. The pack itself is untouched.
pub fn additive_fire(art: Res<ArtPack>, mut images: ResMut<Assets<Image>>, mut done: Local<usize>) {
    let Some(frames) = art.anims.get("flameborder_sm") else {
        return;
    };
    if *done >= frames.len() {
        return;
    }
    let mut converted = 0;
    for handle in frames {
        let Some(mut image) = images.get_mut(handle) else {
            continue;
        };
        if image.texture_descriptor.format
            != bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
        {
            converted += 1;
            continue;
        }
        if let Some(data) = image.data.as_mut()
            && data.len() % 4 == 0
            && data
                .as_chunks::<4>()
                .0
                .iter()
                .any(|px| px[3] == 255 && px[0].max(px[1]).max(px[2]) < 8)
        {
            for px in data.as_chunks_mut::<4>().0 {
                let peak = px[0].max(px[1]).max(px[2]);
                let alpha = (u16::from(peak) * u16::from(px[3]) / 255) as u8;
                if peak > 0 {
                    for c in &mut px[..3] {
                        *c = (u16::from(*c) * 255 / u16::from(peak)) as u8;
                    }
                }
                px[3] = alpha;
            }
        }
        converted += 1;
    }
    *done = converted;
}

/// "1c" or "$1", as the plaque letters a denomination.
fn denom_text_short(cents: u64) -> String {
    if cents >= 100 {
        format!("${}", cents / 100)
    } else {
        format!("{cents}c")
    }
}

/// A fixed jackpot's prize in credits at a credit of `cents`.
fn fixed_prize(model: &MathModel, tier: JackpotTier, cents: u64) -> Option<u64> {
    let unit = u32::try_from(cents).ok()?;
    model
        .hold_and_spin
        .as_ref()?
        .jackpots
        .iter()
        .find(|j| j.tier == tier)?
        .fixed_credits
        .iter()
        .filter(|f| f.from_unit <= unit)
        .max_by_key(|f| f.from_unit)
        .map(|f| f.credits)
}

/// The jackpot values' type size, and the share of its box a value may fill
/// before it is set smaller. In the heavy face (Arial Black) a figure is
/// about 0.74 of the type size, so 62 keeps the digits about 46 px tall.
const JACKPOT_VALUE_SIZE: f32 = 62.0;
const JACKPOT_VALUE_FILL: f32 = 0.9;

/// A jackpot value's colour, by level. Both the cabinet's jackpot display
/// and the official mobile app colour each level's value: the Grand crimson,
/// the Major green, the Minor blue, the Mini orange (p6OesyvM5vA 5:59,
/// y8c8lEdPW8Y 0:05-0:18, the app at 20 s; Happy & Prosperous bUbSjAGwqs8
/// agrees). The hue family is VERIFIED; these tints sit between two
/// cameras' readings and the bar art's own label hues (STRONG for hue only:
/// camera colour is shifted).
fn jackpot_value_colour(tier: JackpotTier) -> Color {
    match tier {
        JackpotTier::Grand => Color::srgb(0.89, 0.10, 0.24),
        JackpotTier::Major => Color::srgb(0.12, 0.98, 0.42),
        JackpotTier::Minor => Color::srgb(0.10, 0.40, 0.97),
        JackpotTier::Mini => Color::srgb(0.95, 0.46, 0.25),
    }
}

/// A jackpot value's box width, in canvas pixels, and its largest type
/// size.
#[derive(Component)]
pub struct JackpotFit {
    width: f32,
    max: f32,
}

/// A value's width in ems in the jackpot values' heavy face (Arial Black's
/// advances): figures and "$" 0.667, "," and "." 0.333.
fn meter_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| if matches!(c, ',' | '.') { 0.333 } else { 0.667 })
        .sum()
}

/// The type size, at most `max`, that keeps `text` inside a box `width`
/// wide.
fn jackpot_value_size(text: &str, width: f32, max: f32) -> f32 {
    let ems = meter_ems(text);
    if ems <= 0.0 {
        return max;
    }
    max.min(width * JACKPOT_VALUE_FILL / ems)
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
        let size = jackpot_value_size(&text.0, fit.width, fit.max);
        if font.font_size != FontSize::Px(size) {
            let old = match font.font_size {
                FontSize::Px(px) => px,
                _ => fit.max,
            };
            font.font_size = FontSize::Px(size);
            // Keep the value on the same middle line.
            if let Val::Px(top) = node.top {
                node.top = Val::Px(top + (old - size) * 0.62);
            }
        }
    }
}

/// How long a changed figure of a progressive's value takes to roll, and
/// how many figures a value may have. On a club cabinet the changed figure
/// rolls up inside its box: the old one rises out of sight, the new one
/// rises from below, clipped to the box, easing at both ends, over about
/// 0.35-0.40 s; one-cent steps follow each other at once (y8c8lEdPW8Y,
/// m_top1.jpg and two rolls at 30 fps in its 1105-1122 s section: .71 to
/// .72 over 11-12 frames, .72 to .73 over 10-11; VERIFIED there). What a
/// carry does was not seen: every changed figure rolls (HYPOTHESIS).
const ROLL_SECONDS: f32 = 0.37;
const ROLL_SLOTS: u8 = 12;
/// The value box the figures roll inside: the dark panel under each
/// level's title, rows 40-90 of the jackpot bar's 104 (every pack's bar).
const ROLL_CLIP: (f32, f32) = (40.0 / 104.0, 90.0 / 104.0);

/// A progressive's value as it rolls from one string to the next.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Roll {
    from: String,
    to: String,
    t: f32,
}

/// Where one figure of a rolling value is: its character and its offset in
/// line heights (0 in place, positive below), and the character it replaces
/// with that one's offset, while it rolls.
#[derive(Clone, Debug, PartialEq)]
pub struct RollPose {
    pub now: char,
    pub now_y: f32,
    pub old: Option<(char, f32)>,
}

impl Roll {
    /// Follow the value the meter shows: a change starts a roll from the
    /// value last shown in full (a roll still running ends at once).
    pub fn follow(&mut self, text: &str) {
        if self.to == text {
            return;
        }
        if self.to.is_empty() || self.to == "-" {
            // The first value (after the placeholder) shows as it is.
            self.from = text.to_string();
            self.t = ROLL_SECONDS;
        } else {
            self.from = std::mem::take(&mut self.to);
            self.t = 0.0;
        }
        self.to = text.to_string();
    }

    pub fn advance(&mut self, dt: f32) {
        self.t = (self.t + dt).min(ROLL_SECONDS);
    }

    /// Each figure's pose now. The old value lines up with the new from the
    /// right (the cents stay in place when a figure is added).
    pub fn poses(&self) -> Vec<RollPose> {
        let x = (self.t / ROLL_SECONDS).clamp(0.0, 1.0);
        let e = x * x * (3.0 - 2.0 * x);
        let old: Vec<char> = self.from.chars().collect();
        let new: Vec<char> = self.to.chars().collect();
        let shift = new.len() as isize - old.len() as isize;
        new.iter()
            .enumerate()
            .map(|(i, &c)| {
                let was = usize::try_from(i as isize - shift)
                    .ok()
                    .and_then(|j| old.get(j).copied());
                let rolling = x < 1.0 && was != Some(c);
                RollPose {
                    now: c,
                    now_y: if rolling { 1.0 - e } else { 0.0 },
                    old: if rolling { was.map(|w| (w, -e)) } else { None },
                }
            })
            .collect()
    }
}

/// A progressive's value drawn as rolling figures over its meter, whose
/// own text stays hidden (its size still fitted to the box).
#[derive(Component)]
pub struct JackpotRoll {
    source: Entity,
    roll: Roll,
    /// The box's top and height on the canvas.
    clip: (f32, f32),
}
/// One figure of a rolling value: its slot, and whether it is the figure
/// leaving.
#[derive(Component)]
pub struct RollGlyph {
    slot: u8,
    old: bool,
}

/// The rolling figures for the meter `source`: a row across the value's
/// box (`cx`, `w`; `clip` its top and height), one clipped slot a figure,
/// each holding the figure coming (in the row's flow) and the figure
/// leaving.
fn spawn_roll(
    c: &mut ChildSpawnerCommands,
    source: Entity,
    font: &Handle<Font>,
    colour: Color,
    (cx, w): (f32, f32),
    clip: (f32, f32),
) {
    let glyph = |slot: u8, old: bool| {
        (
            RollGlyph { slot, old },
            Text::new(""),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(JACKPOT_VALUE_SIZE),
                ..default()
            },
            TextColor(colour),
            TextShadow {
                offset: Vec2::new(2.0, 2.0),
                color: Color::srgba(0.03, 0.03, 0.05, 0.9),
            },
            Node {
                position_type: if old {
                    PositionType::Absolute
                } else {
                    PositionType::Relative
                },
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
        )
    };
    c.spawn((
        JackpotRoll {
            source,
            roll: Roll::default(),
            clip,
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cx - w / 2.0),
            top: Val::Px(clip.0),
            width: Val::Px(w),
            height: Val::Px(clip.1),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexStart,
            ..default()
        },
    ))
    .with_children(|r| {
        for slot in 0..ROLL_SLOTS {
            r.spawn(Node {
                height: Val::Percent(100.0),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|s| {
                s.spawn(glyph(slot, false));
                s.spawn(glyph(slot, true));
            });
        }
    });
}

/// Roll each progressive's changed figures (after the meters are written
/// and fitted): each figure at its meter's size, where the meter's own
/// text would draw it, offset by its pose (a box's height a step).
#[allow(clippy::type_complexity)]
pub fn roll_jackpots(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sources: Query<(&Text, &TextFont, &Node), (With<JackpotFit>, Without<RollGlyph>)>,
    mut rows: Query<
        (Entity, &mut JackpotRoll, &mut Node),
        (Without<JackpotFit>, Without<RollGlyph>),
    >,
    parents: Query<&ChildOf>,
    mut glyphs: Query<
        (
            Entity,
            &RollGlyph,
            &mut Text,
            &mut TextFont,
            &mut Node,
            &mut Visibility,
        ),
        (Without<JackpotFit>, Without<JackpotRoll>),
    >,
) {
    if !fidelity.0 {
        return;
    }
    let dt = smoke.dt(&time);
    for (row_id, mut row, mut node) in &mut rows {
        let Ok((text, font, source_node)) = sources.get(row.source) else {
            continue;
        };
        row.roll.follow(&text.0);
        row.roll.advance(dt);
        let size = match font.font_size {
            FontSize::Px(px) => px,
            _ => JACKPOT_VALUE_SIZE,
        };
        let (clip_top, clip_h) = row.clip;
        let base = match source_node.top {
            Val::Px(top) => top - clip_top,
            _ => 0.0,
        };
        if node.top != Val::Px(clip_top) {
            node.top = Val::Px(clip_top);
        }
        let poses = row.roll.poses();
        for (id, g, mut t, mut f, mut n, mut vis) in &mut glyphs {
            // This row's figures only (glyph, slot, row).
            let row_of = parents
                .get(id)
                .and_then(|slot| parents.get(slot.parent()))
                .map(|r| r.parent());
            if row_of != Ok(row_id) {
                continue;
            }
            let pose = poses.get(usize::from(g.slot));
            let (ch, y, show) = match (pose, g.old) {
                (Some(p), false) => (Some(p.now), p.now_y, true),
                (
                    Some(RollPose {
                        old: Some((c, y)), ..
                    }),
                    true,
                ) => (Some(*c), *y, true),
                _ => (None, 0.0, false),
            };
            let want: String = ch.map(String::from).unwrap_or_default();
            if t.0 != want {
                t.0 = want;
            }
            if f.font_size != FontSize::Px(size) {
                f.font_size = FontSize::Px(size);
            }
            let top = Val::Px(base + y * clip_h);
            if n.top != top {
                n.top = top;
            }
            let v = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *vis != v {
                *vis = v;
            }
        }
    }
}

/// Whether the model's `tier` is a running (progressive) meter.
fn is_progressive(model: &MathModel, tier: JackpotTier) -> bool {
    model
        .hold_and_spin
        .as_ref()
        .and_then(|hs| hs.jackpots.iter().find(|j| j.tier == tier))
        .is_some_and(|j| j.progressive_reset.is_some())
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

/// The name a title's model shows on screen (its chooser's drawn title, the
/// window's title), from the model's id.
pub fn title_name(model_id: &str) -> &'static str {
    const NAMES: [(&str, &str); 8] = [
        ("golden-century", "GOLDEN CENTURY"),
        ("autumn-moon", "AUTUMN MOON"),
        ("happy-prosperous", "HAPPY & PROSPEROUS"),
        ("panda-magic", "PANDA MAGIC"),
        ("genghis-khan", "GENGHIS KHAN"),
        ("golden-gong", "GOLDEN GONG"),
        ("peace-long-life", "PEACE & LONG LIFE"),
        ("spring-festival", "SPRING FESTIVAL"),
    ];
    NAMES
        .iter()
        .find(|(slug, _)| model_id.starts_with(slug))
        .map_or("DRAGONLINK (synthetic fixture)", |(_, name)| name)
}

/// Build the cabinet: the game screen, the meter strip and the button deck.
pub fn spawn(
    commands: &mut Commands,
    model: &MathModel,
    art: &ArtPack,
    fonts: &CabFonts,
    profile: Profile,
) {
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

            // Club: the "N LINES" panels either side, under the reel frame.
            if profile == Profile::Club {
                spawn_lines_panels(c, art);
            }

            // The reel panel (with the lines tabs, where its art has them).
            let (gx, gy) = GRID_CENTER;
            c.spawn((
                ReelPanel { fitted: false },
                sprite(
                    ui("reel_background"),
                    gx,
                    gy + 4.5 * PANEL_SCALE.1,
                    1240.0 * PANEL_SCALE.0,
                    629.0 * PANEL_SCALE.1,
                ),
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
            spawn_big_art(c, &fonts.meter);
            spawn_feature_layer(
                c,
                art,
                &fonts.meter,
                &fonts.heavy,
                &crate::looks::title_look(&model.id),
            );
            spawn_hs_overlays(c, &fonts.meter);
            spawn_win_lines(c, &fonts.sans);
            spawn_coin_shower(c);
            spawn_counter_burst(c);
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
            // While the chooser shows: fire round the Minor and Mini boxes
            // and the PLAY plaque between them (spawn_attract_bar).
            spawn_attract_bar(c, &fonts.meter, bar_top, k);
            // Each value centred in its box's dark panel, coloured by its
            // level in heavy, wide digits with a dark edge (cabinet and app).
            // The heavy face sits lower in its line than the meter face did,
            // so the middle line is raised to keep the digits at about canvas
            // y 73-117 (measured on a capture; the app's centre is 93.4).
            for (tier, (left, right)) in JACKPOT_BOXES {
                let width = (right - left) * k;
                let (cx, cy) = (
                    960.0 - bar_w / 2.0 + (left + right) / 2.0 * k,
                    bar_top + 64.5 * k,
                );
                let mut value = c.spawn((
                    JackpotMeter { tier },
                    JackpotFit {
                        width,
                        max: JACKPOT_VALUE_SIZE,
                    },
                    label(
                        "-",
                        &fonts.heavy,
                        JACKPOT_VALUE_SIZE,
                        jackpot_value_colour(tier),
                        cx,
                        cy,
                        width,
                    ),
                    TextShadow {
                        offset: Vec2::new(2.0, 2.0),
                        color: Color::srgba(0.03, 0.03, 0.05, 0.9),
                    },
                ));
                // A running meter's figures roll as they change (drawn by
                // `roll_jackpots` over its hidden text).
                if is_progressive(model, tier) {
                    value.insert(Visibility::Hidden);
                    let source = value.id();
                    spawn_roll(
                        c,
                        source,
                        &fonts.heavy,
                        jackpot_value_colour(tier),
                        (cx, width),
                        (
                            bar_top + bar_h * ROLL_CLIP.0,
                            bar_h * (ROLL_CLIP.1 - ROLL_CLIP.0),
                        ),
                    );
                }
            }

            // Through Hold & Spin, the collected-coin disc over the bar.
            spawn_hs_collected(c, art, fonts);

            spawn_meter_band(c, fonts, art, gold, pale, profile);

            spawn_deck(c, art, &fonts.sans, &fonts.symbols);
            spawn_gamble_screen(c, art, fonts, profile);
            spawn_games_menu(
                c,
                art,
                fonts,
                profile,
                title_name(&model.id),
                &crate::looks::title_look(&model.id),
            );

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

/// The club band's tabs that stay on screen through the gamble, above its
/// screen (club cabinets: _XHUzg4qDXI 0:07.60 and 1:13.00, LwC3hVqFRMg
/// 18:30-18:33, the owner's Happy & Prosperous 00:13.53; VERIFIED), and give
/// way on the chooser (whose own COLLECT moves to the corner).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BandTab {
    Collect,
    GamesMenu,
}

/// The club tabs' layer: over the gamble screen and the chooser's splash
/// (12), as CREDIT and the tall pill are; the GAMES MENU's touch area one
/// above it.
const BAND_TAB_LAYER: i32 = 14;

/// A rounded on-screen tab (COLLECT, GAMES MENU, GAME RULES): a capsule with
/// a gloss across its upper half kept inside the rounded rim. Returns the
/// tab.
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
) -> Entity {
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
    })
    .id()
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

/// The status and message rows' text: Verdana Bold 23, pale blue-white
/// (cabinet footage); the casino spaces its letters a little. Its dark edge
/// is drawn by copies (`spawn_edged_text`).
fn status_text(s: &str, fonts: &CabFonts, profile: Profile, justify: Justify) -> impl Bundle {
    status_text_in(
        s,
        fonts,
        profile,
        TextLayout::justify(justify),
        Color::srgb_u8(210, 220, 245),
    )
}

/// A status-row text in `colour`, laid out by `layout`.
fn status_text_in(
    s: &str,
    fonts: &CabFonts,
    profile: Profile,
    layout: TextLayout,
    colour: Color,
) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: bevy::text::FontSource::Handle(fonts.msg.clone()),
            font_size: FontSize::Px(23.0),
            ..default()
        },
        TextColor(colour),
        layout,
        LetterSpacing::Px(if profile == Profile::Casino { 1.5 } else { 0.0 }),
    )
}

/// The status and message rows' dark edge, all round the letters (club
/// footage, a straight-on zoom: T49m2COe0Ho; STRONG; an offset shadow left
/// the letters' top and left edges light). Bevy draws one shadow a text and
/// no stroke, so the edge is eight dark copies of the text, this far off
/// each way, behind it.
const EDGE_OFFSETS: [(f32, f32); 8] = [
    (-1.5, -1.5),
    (0.0, -1.5),
    (1.5, -1.5),
    (-1.5, 0.0),
    (1.5, 0.0),
    (-1.5, 1.5),
    (0.0, 1.5),
    (1.5, 1.5),
];
const EDGE_COLOUR: Color = Color::srgba(0.03, 0.03, 0.05, 0.9);

/// A copy of another text (a dark edge, a shaded band) that follows its
/// words, letter spacing and visibility (`echo_texts`).
#[derive(Component)]
pub struct Echo(Entity);

/// Keep each copy's words, spacing and visibility those of its text.
pub fn echo_texts(
    owners: Query<(&Text, Option<&LetterSpacing>, &Visibility), Without<Echo>>,
    mut echoes: Query<(
        &Echo,
        &mut Text,
        Option<&mut LetterSpacing>,
        &mut Visibility,
    )>,
) {
    for (echo, mut text, spacing, mut vis) in &mut echoes {
        let Ok((theirs, their_spacing, their_vis)) = owners.get(echo.0) else {
            continue;
        };
        if text.0 != theirs.0 {
            text.0.clone_from(&theirs.0);
        }
        if let (Some(mut mine), Some(their_spacing)) = (spacing, their_spacing)
            && *mine != *their_spacing
        {
            *mine = *their_spacing;
        }
        if *vis != *their_vis {
            *vis = *their_vis;
        }
    }
}

/// Move a length by `d` pixels, if it is in pixels.
fn nudge(v: Val, d: f32) -> Val {
    match v {
        Val::Px(p) => Val::Px(p + d),
        other => other,
    }
}

/// Spawn a status-row text placed by `node`, with `extra` on it and its dark
/// edge behind it (following it), all at `z` if given; returns the text.
#[allow(clippy::too_many_arguments)]
fn spawn_edged_text(
    c: &mut ChildSpawnerCommands,
    s: &str,
    fonts: &CabFonts,
    profile: Profile,
    justify: Justify,
    node: Node,
    z: Option<GlobalZIndex>,
    extra: impl Bundle,
) -> Entity {
    let mut copies = Vec::with_capacity(EDGE_OFFSETS.len());
    for (dx, dy) in EDGE_OFFSETS {
        let mut at = node.clone();
        at.left = nudge(at.left, dx);
        at.top = nudge(at.top, dy);
        let mut copy = c.spawn((
            status_text_in(s, fonts, profile, TextLayout::justify(justify), EDGE_COLOUR),
            at,
        ));
        if let Some(z) = z {
            copy.insert(z);
        }
        copies.push(copy.id());
    }
    let mut text = c.spawn((status_text(s, fonts, profile, justify), node, extra));
    if let Some(z) = z {
        text.insert(z);
    }
    let owner = text.id();
    for copy in copies {
        c.commands().entity(copy).insert(Echo(owner));
    }
    owner
}

/// A fixed status-row text in a row (the start prompts' words), with its
/// dark edge behind it.
fn spawn_edged_inline(c: &mut ChildSpawnerCommands, s: &str, fonts: &CabFonts, profile: Profile) {
    c.spawn(Node::default()).with_children(|w| {
        for (dx, dy) in EDGE_OFFSETS {
            w.spawn((
                status_text_in(s, fonts, profile, TextLayout::no_wrap(), EDGE_COLOUR),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(dx),
                    top: Val::Px(dy),
                    ..default()
                },
            ));
        }
        w.spawn(status_text(s, fonts, profile, Justify::Left));
    });
}

/// The club meters' labels (CREDIT, BET, WIN) are orange at the top of their
/// capitals and yellow in the lower half; the denomination is the other way
/// up, yellow at the top and orange at the foot (club footage, row profiles
/// of the label pixels on three machines: CAM8vranI9Y, T49m2COe0Ho,
/// y8c8lEdPW8Y; the direction VERIFIED, the white digits beside them flat,
/// so the shading is the art's; the tints from camera colour, PARTIAL). Bevy
/// fills text with one colour, so a label is drawn in its lower colour with
/// copies in steps of the shading over it, each clipped to the top of the
/// capitals (`shade_bands`): (top colour, lower colour, the share of the
/// capitals' height from their top where the lower colour is reached, steps).
const LABEL_SHADE: ([u8; 3], [u8; 3], f32, u32) = ([245, 155, 45], [245, 214, 40], 0.6, 5);
const DENOM_SHADE: ([u8; 3], [u8; 3], f32, u32) = ([245, 212, 62], [235, 125, 45], 0.85, 6);

/// The shading's lower colour, the label's own.
fn shade_base((_, lower, _, _): ([u8; 3], [u8; 3], f32, u32)) -> Color {
    Color::srgb_u8(lower[0], lower[1], lower[2])
}

/// The copies that shade a label from `top` at the capitals' top to `lower`
/// at `until` of their height, in `steps`: (colour, the share of the
/// capitals' height each is clipped to), the largest first, so each smaller
/// one lies over the last and each shows its colour over its own step.
fn shade_bands((top, lower, until, steps): ([u8; 3], [u8; 3], f32, u32)) -> Vec<(Color, f32)> {
    let steps = steps.max(1);
    (0..steps)
        .rev()
        .map(|k| {
            // Step k covers until * k / steps .. until * (k + 1) / steps, in
            // the shading's colour at its middle.
            let mid = (k as f32 + 0.5) / steps as f32;
            let channel = |i: usize| {
                let (a, b) = (f32::from(top[i]), f32::from(lower[i]));
                (a + (b - a) * mid) / 255.0
            };
            (
                Color::srgb(channel(0), channel(1), channel(2)),
                until * (k + 1) as f32 / steps as f32,
            )
        })
        .collect()
}
/// Where the meters' face (Impact) puts its capitals in a `label` box: their
/// top below the box's top, and their height, as shares of the font size
/// (from the face's metrics: cap height 0.79 of the size, the line box 1.2).
const IMPACT_CAPS: (f32, f32) = (0.21, 0.79);

/// Copies of a `label` in the colours of `bands`, each clipped to the top of
/// its capitals, drawn over it (see `LABEL_SHADE`); `of` makes them follow a
/// label whose words change.
#[allow(clippy::too_many_arguments)]
fn spawn_label_bands(
    c: &mut ChildSpawnerCommands,
    text: &str,
    font: &Handle<Font>,
    size: f32,
    (cx, cy, w): (f32, f32, f32),
    bands: &[(Color, f32)],
    transform: UiTransform,
    z: Option<GlobalZIndex>,
    of: Option<Entity>,
) {
    for &(colour, share) in bands {
        let mut clip = c.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(cx - w / 2.0),
                top: Val::Px(cy - size * 0.62),
                width: Val::Px(w),
                height: Val::Px(size * (IMPACT_CAPS.0 + IMPACT_CAPS.1 * share)),
                overflow: Overflow::clip(),
                ..default()
            },
            transform,
        ));
        if let Some(z) = z {
            clip.insert(z);
        }
        clip.with_children(|k| {
            let mut copy = k.spawn((
                Text::new(text),
                TextFont {
                    font: bevy::text::FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(colour),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(w),
                    ..default()
                },
            ));
            if let Some(of) = of {
                copy.insert(Echo(of));
            }
        });
    }
}

/// The key under which the app's own drawing of the club tall pill's icon
/// joins the pack's ui images (made by `club_pill_icon`, never from a pack).
pub const PILL_ICON_KEY: &str = "_club_pill_icon";
/// The club tall pill's icon on the canvas, width and height.
const PILL_ICON: (f32, f32) = (36.0, 56.0);

/// Whether the club tall pill's icon covers (`x`, `y`), in canvas pixels from
/// its box's top left (`PILL_ICON`): from the top, two arcs and a half-disc
/// about one centre, like a signal sign, then a trapezoid wider at its top
/// and a short bar (club cabinets, straight on at 1080p: CAM8vranI9Y and
/// T49m2COe0Ho at 13.0 s; the shapes VERIFIED, their sizes measured on the
/// zoom and scaled to our pill, PARTIAL). Our own drawing.
fn pill_icon_covers(x: f32, y: f32) -> bool {
    const CX: f32 = PILL_ICON.0 / 2.0;
    // The arcs' and the half-disc's centre, on the half-disc's flat foot.
    const CY: f32 = 28.4;
    // How far the arcs reach either side of upright, in degrees.
    const ARC_HALF_ANGLE: f32 = 40.0;
    let dx = x - CX;
    let up = CY - y;
    let r = dx.hypot(up);
    let within = up > 0.0 && dx.abs().atan2(up).to_degrees() <= ARC_HALF_ANGLE;
    let arc = |outer: f32, inner: f32| within && r <= outer && r >= inner;
    let half_disc = up >= 0.0 && r <= 6.6;
    // The trapezoid, 33 px across at its top and 20 at its foot.
    let trapezoid = (32.0..=46.0).contains(&y) && {
        let k = (y - 32.0) / 14.0;
        dx.abs() <= 16.5 + (10.0 - 16.5) * k
    };
    let bar = (48.5..=56.0).contains(&y) && dx.abs() <= 9.5;
    arc(28.4, 21.2) || arc(18.0, 10.6) || half_disc || trapezoid || bar
}

/// The club tall pill's icon as an image of ours, blue on clear, drawn at
/// twice the canvas size and smoothed (4 x 4 samples a pixel).
pub fn club_pill_icon() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const SCALE: u32 = 2;
    const SAMPLES: u32 = 4;
    let (w, h) = (PILL_ICON.0 as u32 * SCALE, PILL_ICON.1 as u32 * SCALE);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for py in 0..h {
        for px in 0..w {
            let mut hits = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = (px as f32 + (sx as f32 + 0.5) / SAMPLES as f32) / SCALE as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / SAMPLES as f32) / SCALE as f32;
                    if pill_icon_covers(x, y) {
                        hits += 1;
                    }
                }
            }
            let alpha = (hits * 255 / (SAMPLES * SAMPLES)) as u8;
            // The cabinet's blue (camera colour about 25, 45, 160).
            data.extend_from_slice(&[28, 42, 150, alpha]);
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    )
}

/// The rotating prompt above the meters ("3 [scatter] Start FREE GAMES",
/// "6 [orb] Start HOLD & SPIN FEATURE").
#[derive(Component)]
pub struct StartPrompt {
    /// Which of the two prompts this is.
    which: u8,
}

/// The club's "N LINES" panels, left and right: (left edge, width) on the
/// canvas, both from y 407, 204 tall. Club footage (CAM8vranI9Y,
/// T49m2COe0Ho, y8c8lEdPW8Y, 1bAevGz0K2k, rectified onto our reel frame,
/// within 3 px): with the outline, x 12-13 and 1901-1903, y 405-613; the
/// inner edge runs under the frame, with no corner showing there.
const LINES_PANELS: [(f32, f32); 2] = [(14.0, 136.0), (1762.0, 138.0)];
const LINES_PANEL_TOP: f32 = 407.0;
const LINES_PANEL_H: f32 = 204.0;
/// The centres of the panels' text: "50" and "LINES" (footage: x 84 and
/// 1831; y 478 and 550).
const LINES_TEXT_X: [f32; 2] = [84.0, 1831.0];
const LINES_TEXT_Y: (f32, f32) = (478.0, 550.0);

/// The part of the game's own lines-box art (`payline_tab`) that fills a
/// panel: its fill without the gold rim, with the bright band centred
/// (measured on Golden Century's 111 x 225 image: x 14-78, y 14-211, the
/// band 0.41 of the fill as on the cabinet's 0.40). The image's own
/// proportions are taller and narrower than the cabinet's box, so only its
/// fill is used. Another title's art is cropped in the same proportion.
fn lines_panel_crop(size: (f32, f32)) -> Rect {
    let (w, h) = size;
    Rect::new(
        w * 14.0 / 111.0,
        h * 14.0 / 225.0,
        w * 78.0 / 111.0,
        h * 211.0 / 225.0,
    )
}

/// Club: a large panel either side of the reels for "N LINES" (cabinet
/// footage, four machines): the reels' own blue lattice fill (the game's
/// lines-box art), a gold rim of about 10 px, gold outside and orange
/// inside, in a thin dark outline. Spawned before the reel panel, so the
/// frame covers its inner edge as the cabinet's does. The rim colours are
/// our frame art's gold (the camera shifts the cabinet's). A title whose
/// reel art carries its own lines tabs has no `payline_tab`, and no panels.
fn spawn_lines_panels(c: &mut ChildSpawnerCommands, art: &ArtPack) {
    let Some(fill) = art.ui.get("payline_tab") else {
        return;
    };
    let crop = art.sizes.get("payline_tab").copied().map(lines_panel_crop);
    for (left, width) in LINES_PANELS {
        c.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(LINES_PANEL_TOP),
                width: Val::Px(width),
                height: Val::Px(LINES_PANEL_H),
                border: UiRect::all(Val::Px(5.0)),
                border_radius: BorderRadius::all(Val::Px(14.0)),
                ..default()
            },
            BorderColor::all(Color::srgb_u8(240, 200, 60)),
            Outline::new(Val::Px(2.0), Val::ZERO, Color::srgb_u8(45, 15, 0)),
        ))
        .with_children(|b| {
            b.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    border_radius: BorderRadius::all(Val::Px(9.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb_u8(170, 105, 20)),
            ))
            .with_children(|f| {
                f.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        image: fill.clone(),
                        rect: crop,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            });
        });
    }
}

/// The reel panel's sprite, fitted once its image has loaded.
#[derive(Component)]
pub struct ReelPanel {
    fitted: bool,
}

/// Where the opaque frame of a reel panel lies across its art, as fractions
/// of the image's width (Golden Century's 749-pixel panel: opaque from x 2
/// to 746; the four later titles' 1229-pixel panels agree within a pixel).
const PANEL_FRAME: (f32, f32) = (2.0 / 749.0, 747.0 / 749.0);

/// The (left, width) on the canvas of a reel panel whose art is `w` pixels
/// wide with its frame opaque from `first` to `last` across a row above the
/// lines tabs, given where every title's panel goes (`left0`, `width0`).
/// None when the frame fills the image (no tabs: it stays where it is). A
/// panel whose art carries the lines tabs either side (Golden Gong's) is
/// drawn wider, so that its frame lands on the reels as every other title's
/// does, and its tabs stand outside the frame.
fn reel_panel_fit(w: f32, first: f32, last: f32, left0: f32, width0: f32) -> Option<(f32, f32)> {
    let span = last + 1.0 - first;
    if w <= 0.0 || span <= 0.0 || span >= w * (PANEL_FRAME.1 - PANEL_FRAME.0) * 0.98 {
        return None;
    }
    let scale = width0 * (PANEL_FRAME.1 - PANEL_FRAME.0) / span;
    Some((left0 + width0 * PANEL_FRAME.0 - first * scale, w * scale))
}

/// Fit the reel panel to its art once the image has loaded: a panel whose
/// art carries the lines tabs is widened (see [`reel_panel_fit`]). The
/// frame is read from the row at 15% of the image's height, above the tabs.
pub fn fit_reel_panel(
    images: Res<Assets<Image>>,
    mut panels: Query<(&mut ReelPanel, &ImageNode, &mut Node)>,
) {
    for (mut panel, image_node, mut node) in &mut panels {
        if panel.fitted {
            continue;
        }
        let Some(image) = images.get(&image_node.image) else {
            continue;
        };
        panel.fitted = true;
        let Some((first, last)) = frame_span(image) else {
            continue;
        };
        let (Val::Px(left0), Val::Px(width0)) = (node.left, node.width) else {
            continue;
        };
        if let Some((left, width)) = reel_panel_fit(
            image.width() as f32,
            first as f32,
            last as f32,
            left0,
            width0,
        ) {
            node.left = Val::Px(left);
            node.width = Val::Px(width);
        }
    }
}

/// The first and last opaque pixel of a reel panel's frame, across the row
/// at 15% of the image's height (above any lines tabs).
fn frame_span(image: &Image) -> Option<(u32, u32)> {
    let (w, h) = (image.width(), image.height());
    let row = h * 3 / 20;
    let opaque = |x: &u32| image.get_color_at(*x, row).is_ok_and(|c| c.alpha() > 0.5);
    Some(((0..w).find(opaque)?, (0..w).rev().find(opaque)?))
}

/// The "N LINES" text for `lines` played: the number, and the word.
fn lines_text(lines: u16) -> (String, &'static str) {
    (lines.to_string(), if lines == 1 { "LINE" } else { "LINES" })
}

/// The height of a lines panel's numeral image: the banner numerals' gold
/// is 31.5 px of each 45 px image, and the cabinet's "50" is 47 px tall.
const LINES_DIGIT_H: f32 = 45.0 * 47.0 / 31.5;

/// One numeral of a lines panel's number (club).
#[derive(Component)]
pub struct LinesDigit {
    index: u8,
}

/// Club: the lines panels' number in the game's own banner numerals.
pub fn lines_digits(
    fidelity: Res<Fidelity>,
    sim: Res<Sim>,
    art: Res<ArtPack>,
    mut digits: Query<(&LinesDigit, &mut ImageNode, &mut Node)>,
) {
    if !fidelity.0 {
        return;
    }
    let number = lines_text(sim.game.lines()).0;
    for (d, mut image, mut node) in &mut digits {
        let glyph = number.chars().nth(d.index as usize).and_then(|c| {
            let key = format!("bdigit_{c}");
            art.ui.get(&key).zip(art.sizes.get(&key))
        });
        let Some((h, &(w, gh))) = glyph else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        if image.image != *h {
            image.image = h.clone();
        }
        let k = LINES_DIGIT_H / gh;
        let (want_w, want_h) = (Val::Px(w * k), Val::Px(gh * k));
        if node.width != want_w || node.height != want_h || node.display != Display::Flex {
            node.width = want_w;
            node.height = want_h;
            node.display = Display::Flex;
        }
    }
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
    spawn_edged_text(
        c,
        "",
        fonts,
        profile,
        Justify::Center,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(STATUS_X - 600.0),
            top: Val::Px(status_y(profile) - 14.0),
            width: Val::Px(1200.0),
            ..default()
        },
        None,
        CabMeter::Multiplier,
    );

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
            spawn_edged_inline(row, count, fonts, profile);
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
            spawn_edged_inline(row, text, fonts, profile);
        });
    }

    // A feature's message ("Line 3 wins 20", "Scatter wins 100") takes the
    // prompts' place on the same row.
    spawn_edged_text(
        c,
        "",
        fonts,
        profile,
        Justify::Right,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(1145.0),
            top: Val::Px(message_top(profile) + 5.0),
            width: Val::Px(540.0),
            ..default()
        },
        None,
        FeatureMessage,
    );

    // The club's system message at the row's left (see `SystemMessage`),
    // over the chooser's splash and the gamble screen (12) as CREDIT is:
    // it stays on the chooser (VERIFIED). The longest text is 728 px.
    spawn_edged_text(
        c,
        "",
        fonts,
        profile,
        Justify::Left,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(SYSTEM_MESSAGE_X),
            top: Val::Px(message_top(profile) + 5.0),
            width: Val::Px(800.0),
            ..default()
        },
        Some(GlobalZIndex(13)),
        SystemMessage,
    );

    // The tall yellow pill at the far left (its function is not shown on
    // footage). In this app it opens the machine chooser (MachineMenu), this
    // app's own feature. The casino's has stacked bars on a flat yellow
    // (GOLDEN_CENTURY_CABINET_DECK.md, "Casino left side"); the club's
    // shades from pale yellow at the top to orange at the foot and carries
    // our drawing of its icon (`club_pill_icon`), the bars where it is
    // missing.
    let (pill_top, pill_h) = if club { (937.0, 92.0) } else { (945.0, 90.0) };
    let icon = art.ui.get(PILL_ICON_KEY).filter(|_| club).cloned();
    let mut pill_node = c.spawn((
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
    ));
    if club {
        // Pale yellow at the top to orange at the foot (club cabinets; the
        // tints from camera colour, PARTIAL).
        let stop = |r: u8, g: u8, b: u8, at: f32| {
            bevy::ui::ColorStop::new(Color::srgb_u8(r, g, b), Val::Percent(at))
        };
        pill_node.insert(bevy::ui::BackgroundGradient(vec![
            bevy::ui::Gradient::Linear(bevy::ui::LinearGradient::to_bottom(vec![
                stop(250, 228, 120, 0.0),
                stop(246, 200, 50, 45.0),
                stop(238, 150, 40, 100.0),
            ])),
        ]));
    }
    pill_node
        .with_children(|b| {
            if let Some(image) = icon {
                b.spawn((
                    Node {
                        width: Val::Px(PILL_ICON.0),
                        height: Val::Px(PILL_ICON.1),
                        ..default()
                    },
                    ImageNode {
                        image,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
                return;
            }
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
        })
        .observe(|_press: On<PointerPress>, mut menu: ResMut<MachineMenu>| {
            menu.open = !menu.open;
        });
    // The tabs. Club: GAMES MENU above the band at the far left, GAME RULES
    // on it; COLLECT over the denomination box; GAMES MENU and COLLECT stay
    // over the gamble screen (`BandTab`). Casino: GAME RULES on the band at
    // the left, GAMES MENU over the denomination box (no casino gamble is
    // filmed).
    if club {
        let menu_tab = pill(
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
        c.commands()
            .entity(menu_tab)
            .insert((BandTab::GamesMenu, GlobalZIndex(BAND_TAB_LAYER)));
        // Its touch area over the raised tab (inert in the gamble and on the
        // chooser: `input` opens the menu only at rest).
        c.spawn((
            rect(3.0, 880.0, 112.0, 40.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(BAND_TAB_LAYER + 1),
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
        c.spawn((
            rect(81.0, 988.0, 110.0, 42.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(|_press: On<PointerPress>, mut rules: ResMut<GameRules>| {
            rules.pressed = true;
        });
        let collect = pill(
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
        c.commands()
            .entity(collect)
            .insert((BandTab::Collect, GlobalZIndex(BAND_TAB_LAYER)));
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
        c.spawn((
            rect(77.0, 999.0, 158.0, 44.0),
            BackgroundColor(Color::NONE),
            GlobalZIndex(1),
        ))
        .observe(|_press: On<PointerPress>, mut rules: ResMut<GameRules>| {
            rules.pressed = true;
        });
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
        let z = (i == 0).then_some(GlobalZIndex(14));
        let base = if club { shade_base(LABEL_SHADE) } else { gold };
        let mut e = c.spawn((label(text, font, 68.0, base, x, my, 260.0), narrow));
        if let Some(z) = z {
            e.insert(z);
        }
        // Club: orange at the top of the capitals (see `LABEL_SHADE`).
        if club {
            spawn_label_bands(
                c,
                text,
                font,
                68.0,
                (x, my, 260.0),
                &shade_bands(LABEL_SHADE),
                narrow,
                z,
                None,
            );
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
    // Club: yellow at the top, orange at the foot (see `LABEL_SHADE`).
    let denom = c
        .spawn((
            CabMeter::Denom,
            label(
                "1¢",
                font,
                dsize,
                if club {
                    shade_base(DENOM_SHADE)
                } else {
                    denom_colour
                },
                dx,
                dy,
                bw,
            ),
        ))
        .id();
    if club {
        spawn_label_bands(
            c,
            "1¢",
            font,
            dsize,
            (dx, dy, bw),
            &shade_bands(DENOM_SHADE),
            UiTransform::IDENTITY,
            None,
            Some(denom),
        );
    }
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

    // "N LINES" either side of the reels. A title whose reel art carries
    // its own lines tabs (no `payline_tab`: Golden Gong's, "50 LINES" in its
    // art) has neither box nor text of ours.
    if !art.ui.contains_key("payline_tab") {
        return;
    }
    if club {
        // On the panels (`spawn_lines_panels`), in the cabinet's sizes:
        // "50" 47 px tall, "LINES" 34 px tall and about 95 wide (footage,
        // three machines within 3 px). The cabinet's face is a gold brush
        // serif; Impact stands in for it in "LINES".
        let shadow = TextShadow {
            offset: Vec2::new(2.0, 2.0),
            color: Color::srgba(0.12, 0.05, 0.0, 0.9),
        };
        // The number in the game's own banner numerals where the pack has
        // them (the cabinet's brush serif), else in Impact.
        let numerals = (0..10).all(|d| art.ui.contains_key(&format!("bdigit_{d}")));
        for cx in LINES_TEXT_X {
            if numerals {
                c.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(cx - 60.0),
                    top: Val::Px(LINES_TEXT_Y.0 - LINES_DIGIT_H / 2.0),
                    width: Val::Px(120.0),
                    height: Val::Px(LINES_DIGIT_H),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for index in 0..2 {
                        row.spawn((
                            LinesDigit { index },
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
            } else {
                c.spawn((
                    CabMeter::Lines,
                    label("50", font, 58.0, gold, cx, LINES_TEXT_Y.0, 120.0),
                    shadow,
                ));
            }
            c.spawn((
                CabMeter::LinesWord,
                label("LINES", font, 42.0, gold, cx, LINES_TEXT_Y.1, 120.0),
                shadow,
            ));
        }
        return;
    }
    // The casino style keeps its small dark box (its footage is not
    // measured for this).
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
            label("50", font, 30.0, gold, x, GRID_CENTER.1 - 14.0, 104.0),
        ));
        c.spawn((
            CabMeter::LinesWord,
            label("LINES", font, 30.0, gold, x, GRID_CENTER.1 + 22.0, 104.0),
        ));
    }
}

/// Show one of the two start prompts: the one `FeatureShow::prompt` names,
/// which changes to the other at the first game start after a win message
/// has used the slot, never on a timer (cabinet footage).
pub fn start_prompts(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    menu: Res<GamesMenu>,
    mut prompts: Query<(&StartPrompt, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    // Blank from START FEATURE to the first free game too (club cabinet),
    // and from a denomination choice until the first game starts (club
    // cabinets: y8c8lEdPW8Y 11.8-15.4 s, CAM8vranI9Y 11.4-13.8 s, the prompt
    // back with the game's debit; STRONG; after a visit with no change
    // UNKNOWN, treated alike).
    let quiet = show.message().is_some()
        || show.fg_cut.is_some()
        || show.stage != Stage::Base
        || gamble.active
        || menu.selecting;
    for (p, mut vis) in &mut prompts {
        let want = if p.which == show.prompt && !quiet {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

// ---------------------------------------------------------------------------
// The system message (club)
// ---------------------------------------------------------------------------

/// The club machines' system message at the left of the message row
/// ("Gamble not offered if gamble win will exceed $10000.00"): left-aligned
/// at x 225 with its capitals centred near y 923, in the status line's face
/// and colour, always showing a message (Queensland club footage mapped
/// onto the canvas: CAM8vranI9Y, T49m2COe0Ho, y8c8lEdPW8Y; text and place
/// VERIFIED). The casino profile leaves it empty: an NSW club and a US
/// casino show only one-off lines there, so the always-filled slot is the
/// Queensland club's configuration.
#[derive(Component)]
pub struct SystemMessage;

/// The system message's left edge on the canvas (footage: x 225-227).
const SYSTEM_MESSAGE_X: f32 = 225.0;
/// The club's cash-in limit: notes are refused once CREDIT reaches it
/// ("Credit reached $100.00, cash-in disabled"; VERIFIED).
const CASH_IN_LIMIT_CENTS: u64 = 10_000;
/// The venue's greeting. A SUBSTITUTE: the venue sets this line, and club
/// footage shows "Welcome To <the club's name>"; no real venue's name is
/// used here.
const VENUE_GREETING: &str = "Welcome";
/// The system message switches two seconds after a game starts (club
/// footage: 13 of 13 changes 2.0-2.25 s after a start; STRONG) and, with
/// no game, every two seconds (HYPOTHESIS: one idle toggle seen, 2.2 s).
const SYSTEM_SWITCH: f32 = 2.0;
/// Seconds the cash-in notice shows (club footage: 1.0-1.5 s, 2.4 s once;
/// PARTIAL).
const CASH_IN_SHOWS: f32 = 1.2;

/// A limit as the system messages write it: "$10000.00", never grouped
/// (every frame; `money` would write "$10,000.00").
fn plain_dollars(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// The club's system messages (texts VERIFIED on Queensland club footage).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SystemText {
    /// "Gamble not offered if gamble win will exceed $10000.00".
    GambleNotice,
    /// "Gamble win limit is $10000.00", while a base game's win is offered.
    WinLimit,
    /// "Credit reached $100.00, cash-in disabled", once after CREDIT rises
    /// to the limit.
    CashIn,
    /// The venue's greeting (a substitute: [`VENUE_GREETING`]).
    Greeting,
}

impl SystemText {
    /// The message as the slot writes it.
    pub fn text(self) -> String {
        match self {
            SystemText::GambleNotice => format!(
                "Gamble not offered if gamble win will exceed {}",
                plain_dollars(GAMBLE_WIN_LIMIT_CENTS)
            ),
            SystemText::WinLimit => format!(
                "Gamble win limit is {}",
                plain_dollars(GAMBLE_WIN_LIMIT_CENTS)
            ),
            SystemText::CashIn => format!(
                "Credit reached {}, cash-in disabled",
                plain_dollars(CASH_IN_LIMIT_CENTS)
            ),
            SystemText::Greeting => VENUE_GREETING.to_string(),
        }
    }
}

/// One frame of what the system message follows.
#[derive(Clone, Copy, Debug, Default)]
pub struct SlotFrame {
    /// Seconds since the last frame.
    pub dt: f32,
    /// A game started this frame (a base game, a free game or a respin):
    /// the credit before its bet, in cents.
    pub start: Option<u64>,
    /// A base game's win is offered ("PLAY ON, GAMBLE or TAKE WIN"; not a
    /// feature's total).
    pub offer: bool,
    /// CREDIT as shown, in cents.
    pub credit: u64,
}

/// The club's system message, as Queensland club footage shows it
/// (y8c8lEdPW8Y at 4 fps over 14-65, 236-244, 2860-2885 and 3044-3085 s;
/// CAM8vranI9Y; T49m2COe0Ho):
/// - a base game's win offered shows the win limit, until the next start
///   (STRONG; not during the count, not after a feature);
/// - CREDIT rising to the cash-in limit or above (a win paid, credits
///   loaded) leaves the cash-in notice due (STRONG);
/// - a start ends the win limit or the cash-in notice with the gamble
///   notice (a cut-off cash-in notice stays due); otherwise, with the
///   credit before its bet at the limit or above, it brings the greeting;
///   otherwise it changes nothing (STRONG);
/// - two seconds after a start it switches: to the cash-in notice if due
///   (1.2 s, then the greeting), else to the gamble notice with the credit
///   at the limit or above, else to the other of the gamble notice and the
///   greeting (STRONG: more than 30 transitions);
/// - with no start and no offer it switches every two seconds the same
///   way, the cash-in notice included when due (HYPOTHESIS: one idle
///   toggle seen; the cash-in notice once at an idle switch, y8c8lEdPW8Y
///   14.0 s);
/// - in the gamble and on the chooser it carries on, the win limit not
///   forced (the owner's recording shows the greeting in the gamble;
///   PARTIAL).
#[derive(Resource, Debug, Default)]
pub struct SystemSlot {
    /// The greeting (true) or the gamble notice (false), between the
    /// others.
    greeting: bool,
    /// The win limit shows.
    limit: bool,
    /// Seconds left on the cash-in notice, while it shows.
    cash: Option<f32>,
    /// CREDIT rose to the cash-in limit: the notice is due at the next
    /// switch.
    cash_due: bool,
    /// Seconds since the last start, switch or offer.
    clock: f32,
    /// The next switch is the first since a start.
    after_start: bool,
    /// A win was offered last frame.
    offer: bool,
    /// CREDIT last frame, in cents (none before the first).
    credit: Option<u64>,
}

impl SystemSlot {
    /// Follow one frame.
    pub fn step(&mut self, f: SlotFrame) {
        // CREDIT rose to the limit; the credit loaded at the start counts.
        if self.credit.is_none_or(|c| f.credit > c) && f.credit >= CASH_IN_LIMIT_CENTS {
            self.cash_due = true;
        }
        self.credit = Some(f.credit);
        if f.offer {
            // A cash-in notice the offer cuts off stays due.
            self.limit = true;
            self.cash = None;
            self.offer = true;
            self.clock = 0.0;
            return;
        }
        if std::mem::take(&mut self.offer) {
            self.clock = 0.0;
        }
        if let Some(before) = f.start {
            if self.limit || self.cash.is_some() {
                self.greeting = false;
            } else if before >= CASH_IN_LIMIT_CENTS {
                self.greeting = true;
            }
            self.limit = false;
            self.cash = None;
            self.clock = 0.0;
            self.after_start = true;
            return;
        }
        self.clock += f.dt;
        if let Some(left) = self.cash {
            if left > f.dt {
                self.cash = Some(left - f.dt);
            } else {
                // The greeting follows, the next switch two seconds on.
                self.cash = None;
                self.cash_due = false;
                self.greeting = true;
                self.clock = 0.0;
            }
            return;
        }
        if self.clock < SYSTEM_SWITCH {
            return;
        }
        self.clock = 0.0;
        self.limit = false;
        if self.cash_due {
            self.cash = Some(CASH_IN_SHOWS);
        } else if self.after_start && f.credit >= CASH_IN_LIMIT_CENTS {
            self.greeting = false;
        } else {
            self.greeting = !self.greeting;
        }
        self.after_start = false;
    }

    /// The message showing.
    pub fn showing(&self) -> SystemText {
        if self.limit {
            SystemText::WinLimit
        } else if self.cash.is_some() {
            SystemText::CashIn
        } else if self.greeting {
            SystemText::Greeting
        } else {
            SystemText::GambleNotice
        }
    }
}

/// What the system message saw last frame: the reels moving, a respin
/// under way, and the games paid for at the last start.
#[derive(Default)]
pub struct SlotSeen {
    spinning: bool,
    respinning: bool,
    spins: u64,
}

/// Keep the club's system message (see [`SystemSlot`]); the casino
/// profile's stays empty.
#[allow(clippy::too_many_arguments)]
pub fn system_message(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    profile: Res<Profile>,
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    denom: Res<Denom>,
    rollup: Res<Rollup>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    deck: Res<DeckState>,
    reels: Res<Reels>,
    mut slot: ResMut<SystemSlot>,
    mut seen: Local<SlotSeen>,
    mut texts: Query<&mut Text, With<SystemMessage>>,
) {
    if !fidelity.0 {
        return;
    }
    let want = if *profile == Profile::Club {
        let cents = denom.cents();
        let credit = credit_shown(&sim, &presentation, &rollup, &show, &gamble)
            .saturating_mul(cents)
            .saturating_add(sim.game.state().residue);
        // A start: the reels set off (free games' too) or a respin begins
        // (the deck's spinning mode covers both, so the reels are read).
        let spinning = reels.any_moving();
        let respinning = show.respinning();
        let started = (spinning && !seen.spinning) || (respinning && !seen.respinning);
        seen.spinning = spinning;
        seen.respinning = respinning;
        let start = started.then(|| {
            // A paid game's bet is off CREDIT by the time its reels move.
            let spins = sim.game.state().spins;
            let paid = spins > seen.spins;
            seen.spins = spins;
            if paid {
                credit.saturating_add(sim.game.total_bet().saturating_mul(cents))
            } else {
                credit
            }
        });
        slot.step(SlotFrame {
            dt: smoke.dt(&time).min(0.05),
            start,
            offer: deck.offer && show.stage == Stage::Base,
            credit,
        });
        // A Grand calls the attendant from its award until the next game.
        show.grand_call(cents)
            .unwrap_or_else(|| slot.showing().text())
    } else {
        String::new()
    };
    for mut text in &mut texts {
        if text.0 != want {
            text.0.clone_from(&want);
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
    // The black bezel under the screen, and the deck panel. The deck is
    // hardware in front of the glass: nothing the screen draws (coin comets,
    // the coin shower, flares) may paint over it, so the bezel, the panel and
    // the keys form one stacking layer above every screen effect, in a fixed
    // order inside it: bezel (DECK_LAYER, 0) < panel (DECK_LAYER, 1) < keys
    // (DECK_LAYER, 2).
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
        GlobalZIndex(DECK_LAYER),
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
        GlobalZIndex(DECK_LAYER),
        ZIndex(1),
    ));
    // Near-square keys, as on the cabinet, in two rows, and the teardrop
    // beside them spanning both, the whole centred on what shows: the
    // teardrop's art fills about [`TEARDROP_VISIBLE`] of its node from
    // [`TEARDROP_INSET`] in, and it stands a quarter of a key clear of the
    // keys (cabinet footage).
    let (kw, kh, gx, gy) = DECK_KEY;
    let cluster = 6.0 * kw + 5.0 * gx;
    let (sw, sh) = DECK_TEARDROP;
    let gap = TEARDROP_GAP * kw;
    let left = (CANVAS_W - (cluster + gap + TEARDROP_VISIBLE * sw)) / 2.0;
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
        left + cluster + gap - TEARDROP_INSET * sw,
        top + kh + gy / 2.0 - sh / 2.0,
        sw,
        sh,
    );
}

/// A key's width and height and the gaps between keys (cabinet footage:
/// near-square keys about a tenth of a key apart along a row, 0.09-0.14 on
/// three machines, STRONG; the key art's bezel runs to its edge, so the
/// node gap is the gap seen). Truly square keys would need a taller deck
/// strip.
const DECK_KEY: (f32, f32, f32, f32) = (96.0, 80.0, 12.0, 10.0);

/// The teardrop's node: its art shows about 1.7 keys wide (cabinet 1.6-1.74
/// keys) and as tall as both rows.
const DECK_TEARDROP: (f32, f32) = (204.0, 170.0);

/// The share of the teardrop's node its art fills across, and how far in
/// from the node's left edge it starts (the pack's `spin_*` art).
const TEARDROP_VISIBLE: f32 = 0.8;
const TEARDROP_INSET: f32 = 0.1;

/// The gap between the keys and the teardrop, in keys (cabinet footage:
/// 0.2-0.4 of a key).
const TEARDROP_GAP: f32 = 0.25;

/// The deck's stacking layer: above every screen effect (the Hold & Spin
/// transition 30, the coin shower, bolt bursts and win flares 26, the bolts
/// 22, the WINNER box and its splash 20-21, the CREDIT pill 14, the chooser
/// and the gamble 12-13, the WINNER plate 4) and below the rules overlay (50)
/// and the machine menu (60), which still cover the deck.
const DECK_LAYER: i32 = 40;

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
    /// A denomination was chosen and no game has started since, at base
    /// (see [`GamesMenu::selecting`]).
    pub selecting: bool,
    /// Seconds since the mode or the offer began (after a denomination
    /// choice, since the YES): the one clock of the deck's flash, the
    /// teardrop's legends and the meters' switch.
    pub since: f32,
    /// The mode before this one, and seconds since the mode changed (not
    /// restarted by the offer): the clock of the wipe back to the idle
    /// faces as the reels stop.
    pub prev: DeckMode,
    pub mode_t: f32,
    /// Seconds START FEATURE has been awaited (0 while it is not).
    pub wait_t: f32,
}

impl DeckState {
    /// The club machines' 1 Hz flash: in its second half-second the
    /// flashing keys show their plain face (owner's recording).
    pub fn plain(&self) -> bool {
        ((self.since * 2.0) as u32) % 2 == 1
    }

    /// The meters switch to credits every other second while a win is
    /// offered, through the gamble, and from a denomination choice until
    /// the first game, on the club machines only (club footage: 1.00 s
    /// phases, five cases on four recordings after a choice).
    pub fn in_credits(&self) -> bool {
        self.profile == Profile::Club
            && (self.offer || self.mode == DeckMode::Gamble || self.selecting)
            && (self.since as u32) % 2 == 1
    }
}

/// Where the deck's clock restarts when the mode or the offer changes:
/// from nothing, except as the game returns from a denomination choice,
/// when it counts from the YES (the splash's [`MENU_CLOSE`] earlier). The
/// meters then show dollars 0.7 s more and switch to credits 1.0 s after
/// the YES, the first switch that two of the club recordings place 0.87
/// and 0.93 s after it (the phase is PARTIAL: 0.26 to 1.04 s across five
/// cases).
fn deck_clock_restart(from: DeckMode, selecting: bool) -> f32 {
    if from == DeckMode::Menu && selecting {
        MENU_CLOSE
    } else {
        0.0
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
    // The teardrop points up, as on every cabinet filmed; its legend sits in
    // its ring, centred 0.55 of the way down (cabinet 0.52-0.57 from the
    // apex).
    let pad = if button == CabButton::Spin {
        UiRect::top(Val::Px(h * TEARDROP_LEGEND_PAD))
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
        // The art fills the whole key: an image node draws in its content
        // box by default, and the legend's padding would shrink every key
        // (to 84 x 68, widening the gaps) and squash the teardrop.
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            visual_box: bevy::ui::VisualBox::BorderBox,
            ..default()
        },
        // Above the bezel and the panel inside the deck's layer; the legend
        // children inherit the key's place in the stack.
        GlobalZIndex(DECK_LAYER),
        ZIndex(2),
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

/// The teardrop legend's top padding, in its height: the legend centres on
/// the art's ring, 0.55 of the way down.
const TEARDROP_LEGEND_PAD: f32 = 0.10;

/// GOOD LUCK's wipe as the reels start: a letter every 0.1 s from the left,
/// each key keeping its idle face and legend until its letter arrives, the
/// bottom row a frame behind, the right column dark 0.1 s after the D
/// (cabinet footage: four wipes on one machine, two of them respins,
/// VERIFIED).
const WIPE_IN_STEP: f32 = 0.1;
const WIPE_ROW_LAG: f32 = 1.0 / 30.0;

/// The wipe back to the idle faces as the reels stop: from the left, a
/// column every 0.08 s (two stops on one machine, VERIFIED).
const WIPE_BACK_STEP: f32 = 0.08;

/// The legend's alpha on the plain face of the flash: faint (PARTIAL).
const PLAIN_LEGEND_ALPHA: f32 = 0.35;

/// The club teardrop's START FEATURE turns edge-on and back like a coin,
/// over [`LEGEND_TURN`] s at the end of every [`LEGEND_TURN_PERIOD`] s from
/// the wait's start (a Golden Century Dragon Cash deck A, STRONG; the phase
/// ours).
const LEGEND_TURN_PERIOD: f32 = 1.28;
const LEGEND_TURN: f32 = 0.3;

/// Faces a pack may lack, drawn as another of its images under a tint:
/// (face, stand-in, tint). Tints multiply, so the coloured stand-ins tint
/// the white key.
const STAND_IN_FACES: [(&str, &str, Color); 7] = [
    // The selected lines key, the chosen gamble key and the plain flash.
    ("key_lightblue", "key_white", Color::srgb(0.62, 0.80, 1.0)),
    // The red gamble choices' plain flash.
    ("key_lavender", "key_white", Color::srgb(0.86, 0.84, 1.0)),
    // The selected credits key: pale lavender-white.
    ("key_pale", "key_white", Color::srgb(0.94, 0.92, 1.0)),
    // TAKE WIN: pale khaki.
    ("key_khaki", "key_white", Color::srgb(1.0, 0.88, 0.60)),
    // A pack built before the purple key draws GAMBLE blue.
    ("key_purple", "key_blue", Color::WHITE),
    // The gamble's unused keys: the card back if the pack has no lattice key.
    ("key_lattice", "card_back", Color::WHITE),
    // The teardrop's gamble lattice: the dark teardrop if the pack has none.
    ("spin_lattice", "spin_dim", Color::WHITE),
];

/// The pack image and tint that draw a face: its own image if the pack has
/// one, else its stand-in (see [`STAND_IN_FACES`]).
fn resolve_face(face: &'static str, has: impl Fn(&str) -> bool) -> (&'static str, Color) {
    if has(face) {
        return (face, Color::WHITE);
    }
    STAND_IN_FACES
        .iter()
        .find(|(f, ..)| *f == face)
        .map_or((face, Color::WHITE), |&(_, image, tint)| (image, tint))
}

/// What the deck shows this frame: its state and the facts of the game its
/// faces, legends and inks follow (see [`light_deck`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct DeckLook {
    pub deck: DeckState,
    /// At rest: the game idle with nothing queued.
    pub idle: bool,
    /// START FEATURE is awaited.
    pub start_wait: bool,
    /// The Hold & Spin board shows.
    pub hold_and_spin: bool,
    /// The gamble's suits may be chosen (within the gamble win limit).
    pub suits_open: bool,
    /// The gamble choice, once its card is turned.
    pub chosen: Option<GamblePick>,
    pub lines: u16,
    pub bet: u32,
}

impl DeckLook {
    /// The role a key shows. It trails the deck's own while the deck changes
    /// over: as the reels start, GOOD LUCK wipes in from the left, each key
    /// keeping its idle face and legend until its letter arrives; as they
    /// stop, the idle faces wipe back from the left.
    pub fn shown(&self, cap: CabButton) -> Role {
        let d = self.deck;
        if let CabButton::Slot { row, col } = cap {
            let c = f32::from(col.min(5));
            match d.mode {
                DeckMode::Spinning if col > 0 => {
                    let lag = if row == 1 { WIPE_ROW_LAG } else { 0.0 };
                    if d.since < WIPE_IN_STEP * c + lag {
                        let idle = DeckState {
                            mode: DeckMode::Base,
                            offer: false,
                            ..d
                        };
                        return role(cap, &idle);
                    }
                }
                DeckMode::Base if d.prev == DeckMode::Spinning && d.mode_t < WIPE_BACK_STEP * c => {
                    let spinning = DeckState {
                        mode: DeckMode::Spinning,
                        ..d
                    };
                    return role(cap, &spinning);
                }
                _ => {}
            }
        }
        role(cap, &d)
    }

    /// Whether a key shows the plain face of the club machines' 1 Hz flash:
    /// the keys that can be pressed while a win is offered (top keys 1-2 and
    /// the bottom row), and the gamble's open choices until a card is turned
    /// (suits past the gamble limit stay dark and still).
    pub fn flashing(&self, cap: CabButton, shown: Role) -> bool {
        let d = self.deck;
        if d.profile != Profile::Club || !d.plain() {
            return false;
        }
        match (d.mode, cap, shown) {
            (_, _, Role::Letter(_) | Role::Off | Role::Blank | Role::Lattice) => false,
            (DeckMode::Base, CabButton::Slot { row, col }, _) => d.offer && (row == 1 || col <= 1),
            (DeckMode::Gamble, _, Role::Pick(p)) => {
                self.chosen.is_none() && (is_colour(p) || self.suits_open)
            }
            _ => false,
        }
    }

    /// A key's face: a pack image, or a stand-in face (see
    /// [`resolve_face`]). Every key is lit at full brightness, GAMBLE and
    /// TAKE WIN whether or not a gamble is offered and the credits keys a
    /// denomination does not offer too (club footage).
    pub fn face(&self, cap: CabButton) -> &'static str {
        let d = self.deck;
        let r = self.shown(cap);
        if cap == CabButton::Spin {
            // A dark smoky teardrop at rest, while the reels spin and through
            // Hold & Spin; lit blue only while START FEATURE waits: steadily
            // on the club deck (deck A), blinking 0.5 s on, 0.5 s off on the
            // casino's (US deck B).
            let blink_off = d.profile == Profile::Casino && ((d.wait_t * 2.0) as u32) % 2 == 1;
            return match r {
                // The gamble: a card-back lattice filling the face, with no
                // legend (club footage, _XHUzg4qDXI 0:06.80-0:08.57 and
                // 1:12.00-1:14.50; VERIFIED on one machine).
                Role::Lattice => "spin_lattice",
                Role::Off => "spin_dim",
                _ if self.start_wait && !blink_off => "spin_lit",
                _ => "spin_dim",
            };
        }
        if self.flashing(cap, r) {
            return match r {
                Role::Pick(p) if is_red(p) => "key_lavender",
                _ => "key_lightblue",
            };
        }
        match r {
            Role::Pick(p) if self.chosen == Some(p) => "key_lightblue",
            Role::Credits(m) if m == self.bet => "key_pale",
            Role::Bet(m) if m == self.bet => "key_white",
            Role::Bet(_) | Role::Credits(_) | Role::Blank => "key_red",
            Role::Lines(n) if n == self.lines => "key_lightblue",
            Role::Lines(_) | Role::Play => "key_blue",
            Role::Gamble => "key_purple",
            Role::TakeWin => "key_khaki",
            // Hearts, diamonds and RED orange; clubs, spades and BLACK dark
            // (club footage, the vivid phase); a suit past the limit dark.
            Role::Pick(p) if is_red(p) && (is_colour(p) || self.suits_open) => "key_red",
            Role::Pick(_) | Role::Letter(_) | Role::Off => "key_dark",
            Role::Lattice => "key_lattice",
        }
    }

    /// A key's legend colour: faint on the plain face of the flash.
    pub fn ink(&self, cap: CabButton) -> Color {
        let r = self.shown(cap);
        let ink = match (r, self.face(cap)) {
            // Spelled letters glow blue on a dark key.
            (Role::Letter(_), _) => Color::srgb(0.35, 0.65, 1.0),
            // The dark teardrop's legend pale blue; white on the lit one.
            (_, "spin_dim") => Color::srgb(0.45, 0.62, 1.0),
            (_, "key_lightblue" | "key_lavender" | "spin_lit") => Color::WHITE,
            // TAKE WIN's letters blue on its pale face.
            (Role::TakeWin, _) => Color::srgb(0.20, 0.42, 0.95),
            // The selected credits key's numeral pink-red, the others' light
            // blue on orange.
            (Role::Credits(_), "key_pale") => Color::srgb(0.85, 0.35, 0.45),
            (Role::Credits(_), _) => Color::srgb(0.7, 0.85, 1.0),
            // The gamble's choices: light-blue icons on orange and on dark.
            (Role::Pick(_), "key_red") => Color::srgb(0.75, 0.88, 1.0),
            (Role::Pick(_), "key_dark") => Color::srgb(0.6, 0.8, 1.0),
            (_, "key_white" | "key_pale" | "key_khaki" | "key_gold") => {
                Color::srgb(0.06, 0.08, 0.2)
            }
            (_, "key_dark") => Color::srgb(0.45, 0.5, 0.62),
            _ => Color::WHITE,
        };
        if self.flashing(cap, r) {
            ink.with_alpha(PLAIN_LEGEND_ALPHA)
        } else {
            ink
        }
    }

    /// A key's legend, line by line with sizes.
    pub fn words(&self, cap: CabButton) -> Vec<(String, f32)> {
        let d = self.deck;
        let r = self.shown(cap);
        let lines_of = |v: &[(&str, f32)]| -> Vec<(String, f32)> {
            v.iter().map(|(w, z)| ((*w).to_string(), *z)).collect()
        };
        match (cap, r) {
            (CabButton::Spin, Role::Off | Role::Lattice) => Vec::new(),
            // Footage: START FEATURE while a feature waits, SPIN through Hold
            // & Spin, PLAY and REPEAT BET in turn at rest (club); sized to
            // sit inside the teardrop's ring.
            (CabButton::Spin, _) if self.start_wait => {
                lines_of(&[("START", 17.0), ("FEATURE", 15.0)])
            }
            (CabButton::Spin, _) if self.hold_and_spin => lines_of(&[("SPIN", 25.0)]),
            (CabButton::Spin, _)
                if d.profile == Profile::Club
                    && d.mode == DeckMode::Base
                    && self.idle
                    && (d.since as u32) % 2 == 1 =>
            {
                lines_of(&[("REPEAT", 16.0), ("BET", 16.0)])
            }
            (CabButton::Spin, _) => lines_of(&[("PLAY", 25.0)]),
            // Top left is blank purple in the gamble.
            (_, Role::Gamble) if d.mode == DeckMode::Gamble => Vec::new(),
            (_, Role::Pick(p)) if !is_colour(p) && !self.suits_open => Vec::new(),
            _ => legend(r, self.lines, self.bet),
        }
    }

    /// The teardrop legend's width as a share of its own: 1, but while the
    /// club's START FEATURE turns like a coin.
    pub fn legend_turn(&self) -> f32 {
        if !self.start_wait || self.deck.profile != Profile::Club {
            return 1.0;
        }
        let into = self.deck.wait_t % LEGEND_TURN_PERIOD - (LEGEND_TURN_PERIOD - LEGEND_TURN);
        if into < 0.0 {
            1.0
        } else {
            (std::f32::consts::PI * into / LEGEND_TURN).cos().abs()
        }
    }
}

/// The deck's mode: the GAMES MENU, the gamble, a spin, or the game. A Hold
/// & Spin respin spins the deck as the reels do (club footage: GOOD LUCK on
/// every respin, the teardrop keeping SPIN; VERIFIED).
fn deck_mode(menu: bool, gambling: bool, reels_moving: bool, respinning: bool) -> DeckMode {
    if menu {
        DeckMode::Menu
    } else if gambling {
        DeckMode::Gamble
    } else if reels_moving || respinning {
        DeckMode::Spinning
    } else {
        DeckMode::Base
    }
}

/// Light the deck and set its legends, as the cabinet does (see
/// [`DeckLook`]): the selected credits key pale and the selected lines key
/// light blue among orange credits keys and blue lines keys; GAMBLE purple
/// and TAKE WIN pale khaki, always lit; the teardrop dark with a pale-blue
/// PLAY / REPEAT BET at rest, PLAY while the reels spin and SPIN in Hold &
/// Spin, lit for START FEATURE, a lattice in the gamble; GOOD LUCK wiping in
/// from the left as the reels start or a Hold & Spin respin turns, and the
/// idle faces wiping back as they stop; on the club machines the 1 Hz flash
/// of the keys that can be pressed while a win is offered and in the gamble,
/// with the chosen key steady light blue once a card is turned.
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
        &mut UiTransform,
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
    let mode = deck_mode(
        menu.showing(),
        gambling,
        reels.any_moving(),
        show.respinning(),
    );
    let offer = mode == DeckMode::Base && can_gamble;
    // After a denomination choice, until the first game (club footage).
    let selecting = menu.selecting && mode == DeckMode::Base;
    let start_wait = matches!(show.awaiting, Some(Await::Start(_)));
    let dt = smoke.dt(&time).min(0.05);
    if deck.mode != mode || deck.offer != offer {
        deck.since = deck_clock_restart(deck.mode, selecting);
    } else {
        deck.since += dt;
    }
    if deck.mode != mode {
        deck.prev = deck.mode;
        deck.mode_t = 0.0;
    } else {
        deck.mode_t += dt;
    }
    deck.wait_t = if start_wait { deck.wait_t + dt } else { 0.0 };
    deck.mode = mode;
    deck.offer = offer;
    deck.selecting = selecting;
    deck.profile = *profile;
    deck.cents = cents;
    let look = DeckLook {
        deck: *deck,
        idle,
        start_wait,
        hold_and_spin: show.hold_and_spin(),
        // A suit's four times the stake may pass the gamble win limit.
        suits_open: within_gamble_limit(gamble.stake, 4, cents),
        // Once a card is turned the flash stops; the chosen key stays light
        // blue.
        chosen: gamble.card.and(gamble.last.map(|(p, _)| p)),
        lines: sim.game.lines(),
        bet: sim.game.credits_per_line(),
    };
    for (cap, mut image) in &mut caps {
        let (key, tint) = resolve_face(look.face(cap.0), |k| art.ui.contains_key(k));
        if let Some(h) = art.ui.get(key)
            && image.image != *h
        {
            image.image = h.clone();
        }
        if image.color != tint {
            image.color = tint;
        }
    }
    let turn = look.legend_turn();
    for (label, fonts, mut text, mut font, mut color, mut node, mut transform) in &mut labels {
        let words = look.words(label.0);
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
        let suit = matches!(look.shown(label.0), Role::Pick(p) if !is_colour(p));
        let want = if suit { &fonts.symbols } else { &fonts.text };
        let source = bevy::text::FontSource::Handle(want.clone());
        if font.font != source {
            font.font = source;
        }
        let ink = look.ink(label.0);
        if color.0 != ink {
            color.0 = ink;
        }
        let sx = if label.0 == CabButton::Spin {
            turn
        } else {
            1.0
        };
        if transform.scale.x != sx {
            transform.scale.x = sx;
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

/// Seconds each half of the idle status line's alternation shows: the bet
/// line, then "Game Over - Play Now" (cabinet footage).
const REST_TURN: f32 = 2.0;
/// How long "Game Over - Play Now" flashes as a game is debited (cabinet
/// footage: 0.07-0.13 s on the club machines, 2-4 frames at 30 fps).
const DEBIT_FLASH: f32 = 0.10;

/// The idle status line's phase. The bet line and "Game Over - Play Now"
/// alternate, two seconds each, counted from the moment the machine came to
/// rest (the end of a game) or the bet last changed, bet line first: on the
/// cabinet a credits-per-line key restarts the cycle (y8c8lEdPW8Y 12.967 to
/// 14.967 s is exactly 2.000 s), and after a game the bet line shows first.
#[derive(Default)]
pub struct RestPhase {
    at_rest: bool,
    since: f32,
    /// The bet as last seen: credits per line, lines, cents a credit.
    bet: (u32, u16, u64),
    /// Games started so far as last seen, and when the debit flash ends.
    spins: Option<u64>,
    flash_until: f32,
}

impl RestPhase {
    /// Follow the machine at time `t`.
    fn update(&mut self, t: f32, at_rest: bool, bet: (u32, u16, u64), spins: u64) {
        if at_rest && (!self.at_rest || bet != self.bet) {
            self.since = t;
        }
        if self.spins.and_then(|s| s.checked_add(1)) == Some(spins) {
            self.flash_until = t + DEBIT_FLASH;
        }
        self.at_rest = at_rest;
        self.bet = bet;
        self.spins = Some(spins);
    }

    /// At rest, whether the bet line shows (rather than "Game Over - Play
    /// Now"): [0, 2) s of rest, then [4, 6) s, and so on.
    fn bet_line_turn(&self, t: f32) -> bool {
        (((t - self.since).max(0.0) / REST_TURN) as u32).is_multiple_of(2)
    }

    /// A game was debited within the last [`DEBIT_FLASH`] seconds.
    fn flashing(&self, t: f32) -> bool {
        t < self.flash_until
    }
}

/// A feature under way, from its trigger spin on (the game is in it
/// before the screen shows it), and its end (completion, count and
/// payment: on the last free game the engine is idle before they are
/// shown): CREDIT holds still while the win counts on WIN (cabinet
/// footage).
fn credit_held(sim: &Sim, presentation: &Presentation, show: &FeatureShow) -> bool {
    show.in_feature() || show.counting() || !sim.game.is_idle() || fg_end_queued(presentation, show)
}

/// FEATURE COMPLETED is showing and the free games' last events are still
/// queued: CREDIT still holds the feature's total off (club cabinets: CREDIT
/// rises only when the win is taken, y8c8lEdPW8Y 1045.933, 1bAevGz0K2k
/// 368.300; STRONG), until the gamble's offer holds it.
fn fg_end_queued(presentation: &Presentation, show: &FeatureShow) -> bool {
    show.stage == Stage::FreeGamesDone && !presentation.queue.is_empty()
}

/// The credits CREDIT shows (before any cents a change to a larger
/// denomination left over): held through a feature (see [`credit_held`]);
/// otherwise counting up with the win meter, less a win that may still be
/// gambled.
fn credit_shown(
    sim: &Sim,
    presentation: &Presentation,
    rollup: &Rollup,
    show: &FeatureShow,
    gamble: &GambleShow,
) -> u64 {
    if credit_held(sim, presentation, show) {
        return presentation
            .shown_credits
            .saturating_sub(presentation.shown_win);
    }
    let pending = rollup.to.saturating_sub(rollup.shown());
    let at_rest = sim.game.is_idle() && presentation.queue.is_empty();
    let held = if at_rest && (sim.game.can_gamble() || gamble.active) {
        sim.game.state().gamble_stake
    } else {
        0
    };
    presentation.shown_credits.saturating_sub(pending.max(held))
}

/// What BET shows: after a denomination change (or a credits key's
/// selection) the last game's bet until the next game (footage), and in a
/// credits phase that bet in credits of the denomination now played (club
/// footage: "BET 50" against "$1.00" at 2c, CAM8vranI9Y 12.1 s and
/// 1bAevGz0K2k 15.8 s; VERIFIED); otherwise the bet.
fn bet_meter(last_bet_cents: Option<u64>, in_credits: bool, total_bet: u64, cents: u64) -> String {
    match last_bet_cents {
        Some(c) if in_credits => grouped(c / cents.max(1)),
        Some(c) => money(c, 1),
        None if in_credits => grouped(total_bet),
        None => money(total_bet, cents),
    }
}

/// The status line a base game's Hold & Spin leaves once it is counted and
/// the machine is at rest, before any gamble: with its win `offered`, PLAY
/// ON and "Free Spins Completed" in turn (`first_turn`: the first two
/// seconds of each four from the offer's start), PLAY ON first (the owner's
/// Happy & Prosperous club recording: 60 frames each, VERIFIED there; two
/// Golden Century club recordings show PLAY ON first, then the win taken
/// within 0.1-0.2 s: 1bAevGz0K2k 165.3-165.7, f5sMl9a5jds 282.5; PARTIAL
/// for Golden Century). A win that `stands` unoffered (a progressive won,
/// or past the gamble limit) keeps "Free Spins Completed" until the next
/// game (HYPOTHESIS: the line the offer turns with). A win taken leaves the
/// bet line (f5sMl9a5jds 282.6-283.9; `None`).
fn hold_and_spin_end_line(offered: bool, stands: bool, first_turn: bool) -> Option<&'static str> {
    match (offered, stands) {
        (true, _) if first_turn => Some(PLAY_ON),
        (true, _) | (false, true) => Some(FREE_SPINS_COMPLETED),
        (false, false) => None,
    }
}

/// The offer while a win may be gambled.
const PLAY_ON: &str = "PLAY ON, GAMBLE or TAKE WIN";
/// The status line at Hold & Spin's end.
const FREE_SPINS_COMPLETED: &str = "Free Spins Completed";
/// The status line as the free games end.
const FEATURE_COMPLETED: &str = "Feature Completed";

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
    mut rest: Local<RestPhase>,
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
    let t = time.elapsed_secs();
    let at_rest =
        sim.game.is_idle() && presentation.queue.is_empty() && !rollup.active() && !gamble.active;
    rest.update(
        t,
        at_rest,
        (sim.game.credits_per_line(), sim.game.lines(), cents),
        sim.game.state().spins,
    );
    // Cents a change to a larger denomination left over: money on CREDIT.
    let residue = sim.game.state().residue;
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
    // A feature's own status line, if it has one now: a Grand's, the end of
    // a base game's Hold & Spin at rest, or the feature's caption.
    let hs_end = (at_rest && show.hold_and_spin_over() && gamble.ended.is_none())
        .then(|| {
            hold_and_spin_end_line(
                gamble_open(&sim.game, cents, &show),
                sim.game.can_gamble(),
                rest.bet_line_turn(t),
            )
        })
        .flatten()
        .map(str::to_string);
    // FEATURE COMPLETED: "Feature Completed" on the status line a moment,
    // then the offer (club cabinets: one frame, y8c8lEdPW8Y 1044.267, then
    // PLAY ON; a one-frame crossfade, 1bAevGz0K2k 367.567; VERIFIED on
    // y8c8). Here while the free games' last events are queued (an event
    // interval).
    let fg_end = fg_end_queued(&presentation, &show).then(|| FEATURE_COMPLETED.to_string());
    let feature_caption = show
        .grand_caption(cents)
        .or(hs_end)
        .or(fg_end)
        .or_else(|| show.caption());
    for (meter, mut text, mut colour, spacing) in &mut meters {
        let value = match meter {
            CabMeter::Credit => {
                let credits = credit_shown(&sim, &presentation, &rollup, &show, &gamble);
                if in_credits || residue == 0 || credit_held(&sim, &presentation, &show) {
                    amount(credits, cents)
                } else {
                    money(credits.saturating_mul(cents).saturating_add(residue), 1)
                }
            }
            CabMeter::Bet => {
                bet_meter(menu.last_bet_cents, in_credits, sim.game.total_bet(), cents)
            }
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
            CabMeter::Lines => lines_text(sim.game.lines()).0,
            CabMeter::LinesWord => lines_text(sim.game.lines()).1.to_string(),
            CabMeter::Multiplier => feature_caption.clone().unwrap_or_else(|| {
                let per_line = sim.game.credits_per_line();
                let bet_line = match deck.profile {
                    Profile::Club if per_line == 1 => "1 credit per line".to_string(),
                    Profile::Club => format!("{per_line} credits per line"),
                    Profile::Casino => format!("Bet Multiplier {per_line}"),
                };
                let turn = ((t / 2.0) as u32).is_multiple_of(2);
                if !at_rest && rest.flashing(t) {
                    // The debit of a game: a flash of "Game Over - Play Now"
                    // (footage: y8c8lEdPW8Y 15.0-15.3, 61.8, 3076.0 s).
                    "Game Over - Play Now".to_string()
                } else if !at_rest {
                    bet_line
                } else if gamble_open(&sim.game, cents, &show) {
                    // The offer while a win can be gambled.
                    PLAY_ON.to_string()
                } else if let Some(lost) = gamble.ended {
                    // Footage: the gamble's end message in turn with the bet.
                    match (turn, lost) {
                        (true, true) => "Gamble Completed - Gamble Feature Lost".to_string(),
                        (true, false) => "Gamble Completed".to_string(),
                        (false, _) => bet_line,
                    }
                } else if sim.game.credits() < sim.game.total_bet() {
                    "Insert More Credits or Change Selection".to_string()
                } else if rest.bet_line_turn(t) {
                    // Idle: the bet line and "Game Over - Play Now", two
                    // seconds each, bet line first from the end of each game
                    // and from a bet change (footage).
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
        // A feature's status texts ("6 FREE GAMES WON!", "Press START
        // FEATURE button", "Free Game 1 of 6") are the bet line's yellow and
        // spacing too (club cabinet: 1bAevGz0K2k, their brightest pixels
        // blue/red 0.64-0.70 like the bet line's; STRONG).
        // So are "Free Spins Completed" and a Grand's line (club cabinets:
        // the same yellow, 1bAevGz0K2k, f5sMl9a5jds, Wc7QU1C2jWo; VERIFIED
        // for the first, STRONG for the second). The offer keeps its own
        // look here.
        let feature_line = feature_caption
            .as_deref()
            .is_some_and(|c| !c.is_empty() && c != PLAY_ON);
        let bet_line =
            deck.profile == Profile::Club && (text.0.ends_with("per line") || feature_line);
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
    /// This spin scrolls the free reels' strip.
    free: bool,
    /// The window leaving or the one coming holds a scatter, so no other
    /// passes on this reel this spin: the machine never shows two on a reel.
    quiet: bool,
}

/// All reels' motion.
#[derive(Resource, Default)]
pub struct Reels {
    states: Vec<ReelState>,
    /// The symbol keys the reels scroll through while spinning: the base
    /// game's strips and the free games' (the engine's own, NS 4.8: reels
    /// are shown as their strips are arranged).
    base: Vec<Vec<String>>,
    free: Vec<Vec<String>>,
    /// The title's scatter.
    scatter: String,
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
        let strips = if s.free { &self.free } else { &self.base };
        let Some(strip) = strips.get(reel).filter(|s| !s.is_empty()) else {
            return "";
        };
        let len = strip.len();
        let mut i = n.rem_euclid(len as i64) as usize;
        if s.quiet {
            for _ in 0..len {
                if strip[i] != self.scatter {
                    break;
                }
                i = (i + 1) % len;
            }
        }
        &strip[i]
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

/// The strips the reels scroll through, as symbol keys: the model's base
/// strips and its free games' (the base ones again when it has none).
pub fn reel_strips(model: &MathModel) -> Reels {
    let keys = |strips: &[Vec<dragonlink_model::SymbolId>]| {
        strips
            .iter()
            .map(|strip| {
                strip
                    .iter()
                    .map(|id| {
                        model
                            .symbols
                            .get(id.0 as usize)
                            .map(|s| s.key.clone())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect::<Vec<Vec<String>>>()
    };
    let base = keys(&model.reel_strips);
    let free = model
        .free_games
        .as_ref()
        .map(|fg| keys(&fg.free_reel_strips))
        .filter(|f| f.len() == base.len())
        .unwrap_or_else(|| base.clone());
    let states = (0..base.len())
        .map(|reel| ReelState {
            pos: ((reel * 7 + 2) % base[reel].len().max(1)) as f32,
            ..default()
        })
        .collect();
    Reels {
        states,
        base,
        free,
        scatter: Roles::from_model(model).scatter,
        rested: SPIN_AFTER_REST,
    }
}

/// The labels of `reel`'s window, top row first, as on screen: a dimmed
/// open position keeps its "DIM:" and scrolls away still dimmed (club
/// cabinet, y8c8lEdPW8Y 1020.43; VERIFIED).
fn window_labels(labels: &Query<(&CellLabel, &Text)>, reel: usize) -> Vec<String> {
    let mut window = vec![String::new(); 3];
    for (label, text) in labels {
        if label.reel as usize == reel && (label.row as usize) < 3 {
            window[label.row as usize] = text.0.clone();
        }
    }
    window
}

/// The tint a moving strip draws a label in: dimmed for an open Hold &
/// Spin position's symbol, as at rest.
fn strip_tint(dim: bool) -> Color {
    if dim { DIM_TINT } else { Color::WHITE }
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
        // A spin starting: the window on screen scrolls away; when it or the
        // window coming holds a scatter, no other passes on this reel.
        let starting = (reels.states[reel].motion == Motion::Idle && spinning).then(|| {
            let leaving = window_labels(&labels, reel);
            let quiet = leaving
                .iter()
                .chain(anim.coming.get(reel).into_iter().flatten())
                .any(|l| label_symbol(l).0 == reels.scatter);
            (leaving, quiet)
        });
        let state = &mut reels.states[reel];
        state.stopped_now = false;
        match state.motion {
            Motion::Idle if spinning => {
                // Start from what is on screen: the window stays on the strip
                // and scrolls away.
                state.pos = state.pos.round();
                if let Some((leaving, quiet)) = starting {
                    state.landing_at = state.pos as i64;
                    state.landing = leaving;
                    state.quiet = quiet;
                }
                state.free = anim.free;
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
        let (key, dim) = label_symbol(raw);
        let key = key.to_string();
        let tint = strip_tint(dim);
        if image.color != tint {
            image.color = tint;
        }
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
        let coin = reels.moving(tile.reel) && label_symbol(raw).0 == "orb";
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
        let (key, dim) = label_symbol(raw);
        let key = key.to_string();
        let tint = strip_tint(dim).with_alpha(BLUR_ALPHA);
        if image.color != tint {
            image.color = tint;
        }
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

    /// The clip for `name`, or the title's own stand-in for it: a title with
    /// one wild sound (Happy & Prosperous, Panda Magic, Golden Gong:
    /// `SND_Wild`) plays it where Golden Century plays one of its five.
    fn clip(&self, name: &str) -> Option<&(Handle<AudioSource>, f32)> {
        self.clips.get(name).or_else(|| {
            // The free games' own wild sound (SND_WildFG) has no stand-in:
            // Peace & Long Life's x2 wild is silent on club cabinets (two
            // recordings, seven rounds; STRONG).
            (name.starts_with("SND_Wild") && name != "SND_WildFG")
                .then(|| {
                    self.clips
                        .get("SND_Wild")
                        .or_else(|| self.clips.get("SND_Wild_1"))
                })
                .flatten()
        })
    }

    /// Play a clip once; returns the playing entity.
    pub fn play(&self, commands: &mut Commands, name: &str) -> Option<Entity> {
        let (clip, _) = self.clip(name)?;
        Some(
            commands
                .spawn((AudioPlayer::new(clip.clone()), PlaybackSettings::DESPAWN))
                .id(),
        )
    }

    /// Play a clip once at `volume` (scaled by the global volume, which a
    /// muted run sets to 0).
    pub fn play_at(&self, commands: &mut Commands, name: &str, volume: f32) -> Option<Entity> {
        let (clip, _) = self.clip(name)?;
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
        self.clip(name).map(|(_, s)| *s)
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
    /// Seconds since the count completed.
    after: f32,
}

impl Rollup {
    pub fn active(&self) -> bool {
        self.t < self.duration
    }

    /// The whole win this count is counting (credits).
    fn win(&self) -> u64 {
        self.to.saturating_sub(self.from)
    }

    /// Done with the last count as a free game starts: it counts nothing
    /// more (`win` is 0) until this game's win arrives, which then counts
    /// from what WIN shows. Without it the banner showed the last game's
    /// amount for the event interval between this game's line wins and its
    /// credit
    /// (our recording: "WIN $1.00 WIN" for 4 frames before $0.01), and the
    /// first free game the scatter's; club cabinets never do (y8c8lEdPW8Y
    /// 1027.4). What WIN shows and the total seen stay; no tune plays in
    /// the free games, so nothing stops.
    pub fn settle(&mut self) {
        self.from = self.to;
        self.t = self.t.max(self.duration);
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
        rollup.after = 0.0;
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
        rollup.after = 0.0;
    }
    rollup.seen = target;
    if rollup.active() {
        rollup.t += smoke.dt(&time);
    } else {
        rollup.after += smoke.dt(&time);
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
    show: Res<FeatureShow>,
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
        if reels.moving(reel) || show.big_covers(reel) {
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
            && !show.big_covers(t.reel)
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

/// The face's prize in credits, for a bet-multiple face ("x5"). A
/// position under a BIG coin has no prize of its own ("x0") and shows none.
fn face_credits(face: &str, total_bet: u64) -> Option<u64> {
    face.strip_prefix('x')
        .and_then(|m| m.parse::<u64>().ok())
        .filter(|m| *m > 0)
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
    /// When (`clock`) the start prompt showed, while it shows.
    prompt_at: Option<f32>,
    /// The scatter win was still counting under the start prompt when last
    /// seen (the lettering's blink restarts as the count completes).
    prompt_count: bool,
    /// The free games are back after a Hold & Spin one of them started,
    /// until the next free game starts: the board stays dark, with no
    /// banner (club cabinet, y8c8lEdPW8Y 1019.93-1020.37).
    resuming: bool,
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
    /// A respin of this Hold & Spin has been presented (the start prompt's
    /// status line lasts until the first).
    respun: bool,
    /// Flame bursts: position and start time on `clock`.
    bursts: Vec<(u8, u8, f32)>,
    /// Seconds of feature presentation, for the animations.
    clock: f32,
    /// The transition fire: when it began (`clock`) and its frame rate.
    transition_at: Option<(f32, f32)>,
    /// The WINNER total while the coins are counted.
    pub winner: Option<u64>,
    /// After the count: until when (`clock`) the WINNER box stays, and the
    /// total it holds meanwhile (whatever follows, the free games too).
    winner_hold: Option<(f32, u64)>,
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
    /// The Major this Hold & Spin won (credits), shown on its own plaque
    /// before the coins; its own count while it runs; and what that count
    /// has put on WIN.
    major: Option<u64>,
    major_pay: Option<Pay>,
    major_paid: u64,
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
    /// A picture's own sound has played this game (one a game).
    pic_sounded: bool,
    /// The drum roll of a respin that starts with fourteen coins held.
    drum: Option<Entity>,
    /// When the Grand's bell rings (`clock`), once the grid is full.
    grand_bell_at: Option<f32>,
    /// This free game's BIG symbol and the reels it covers (key, first,
    /// width), from the game's start: Spring Festival's BIG reel.
    pub big: Option<(String, u8, u8)>,
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
    /// was pressed (until the first free game starts; the cut comes at
    /// `FG_CUT`), and the banner's total counting up as the first free game
    /// spins (seconds, total shown).
    fg_trigger: Option<f32>,
    fg_flags: bool,
    /// Seconds since a stack on a winning line began to stand (its win waits).
    pub wild: Option<f32>,
    /// A wild sound has played this game, and how many have played.
    wild_sounded: bool,
    wild_turn: usize,
    fg_cut: Option<f32>,
    fg_count: Option<(f32, u32)>,
    /// Which start prompt the message row shows: 0 = "3 [flag] Start FREE
    /// GAMES", 1 = "6 [coin] Start HOLD & SPIN FEATURE". On the cabinet it
    /// changes only at the first game start after a win message, never on a
    /// timer (y8c8lEdPW8Y 15-84 s, 7 of 7; CAM8vranI9Y 18-44 s, 2 of 2).
    /// Starting on FREE GAMES is our choice (one recording opens on each);
    /// a denomination change leaves it as it is.
    pub prompt: u8,
    /// The title's own clips' lengths in seconds (0 until the pack's sounds
    /// are known: Golden Century's then): SND_Scatter, which the scatters'
    /// wave lasts, and its wild sound, which a standing stack holds the win
    /// for (see [`FeatureShow::wave_seconds`]).
    pub scatter_clip: f32,
    pub wild_clip: f32,
    /// The free games' own wild sound (Genghis Khan's x2 wild, SND_WildFG),
    /// 0 if the title has none.
    pub wild_fg_clip: f32,
    /// How long this game's wild win holds the count (the clip that played).
    wild_for: f32,
    /// A filled reel of this game's expansions is in a win.
    expand_won: bool,
    /// Seconds since this game's wilds began to fill their reels, and the
    /// title's SND_ExpandWin length.
    pub expand: Option<f32>,
    pub expand_clip: f32,
    /// A retrigger in the free games while its sequence plays (the totals
    /// before and after it): the base game's trigger again, with no START
    /// FEATURE, the banner's total counting up from the old total.
    retrigger: Option<(u32, u32)>,
}

impl FeatureShow {
    /// Follow an event as it is presented.
    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::SpinStarted { .. } => {
                // The other start prompt follows a base game's win message,
                // once, at the next game start (the frame WIN resets). Not
                // after free games or Hold & Spin, whose stage is not Base:
                // that part is unobserved (HYPOTHESIS).
                if self.stage == Stage::Base && !self.win_lines.is_empty() {
                    self.prompt ^= 1;
                }
                self.big = None;
                self.pic_sounded = false;
                self.grand_bell_at = None;
                self.flares_at = None;
                self.wild = None;
                self.wild_sounded = false;
                self.expand = None;
                self.expand_won = false;
                self.fg_trigger = None;
                self.fg_flags = false;
                self.fg_cut = None;
                self.resuming = false;
                self.grand = None;
                self.grand_done = false;
                self.major = None;
                self.major_pay = None;
                self.major_paid = 0;
                self.progressive = false;
                self.link_won.clear();
                self.win_lines.clear();
                self.scatter_paid = false;
                self.stage = Stage::Base;
                self.scatter_award = 0;
                self.collected = None;
                self.grid = false;
                self.winner = None;
                self.winner_hold = None;
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
                // The board stays a moment, dark, with no banner, until the
                // next free game spins (club cabinet).
                if let Some((number, total)) = self.fg_return.take() {
                    self.stage = Stage::FreeGames { number, total };
                    self.resuming = true;
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
                // A retrigger's sequence ends with the next free game (the
                // base trigger's clock runs on through the free games).
                if self.retrigger.take().is_some() {
                    self.fg_trigger = None;
                    self.fg_flags = false;
                }
                self.pic_sounded = false;
                // Each free game is a game of its own for the wild.
                self.wild = None;
                self.wild_sounded = false;
                self.expand = None;
                self.expand_won = false;
                self.game_won = false;
                self.win_lines.clear();
                self.resuming = false;
                self.stage = Stage::FreeGames {
                    number: *number,
                    total: number + remaining,
                };
                // The first free game: the banner, its total counting up from
                // 0 and the music come with the reels (club cabinet: the
                // banner 2 frames after the first motion, "FREE GAME 1 OF 0"
                // for 5 frames).
                if *number == 1 {
                    self.fg_count = Some((0.0, 0));
                }
                self.fg_cut = None;
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
                self.resuming = false;
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
                self.respun = false;
                self.bursts.clear();
                self.winner = None;
                self.winner_hold = None;
                self.transition_at = None;
                self.bolt = None;
                self.splash_at = None;
                self.major = None;
                self.major_pay = None;
                self.major_paid = 0;
            }
            Event::HoldAndSpinRespin {
                respins_left,
                landed,
                ..
            } => {
                self.respin_landed = Some(landed.iter().map(|o| (o.reel, o.row)).collect());
                self.respun = true;
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
        // The start prompt's lettering blinks from the moment it shows.
        self.prompt_at = matches!(what, Await::Start(_)).then_some(self.clock);
        self.prompt_count = false;
    }

    /// Follow the win's count under the start prompt: the cabinet restarts
    /// the lettering's blink, lit, the moment the scatter win's count
    /// completes (club cabinets: y8c8lEdPW8Y 948.50 and 793.80, 1bAevGz0K2k
    /// 327.77; VERIFIED).
    pub fn follow_count(&mut self, counting: bool) {
        if !matches!(self.awaiting, Some(Await::Start(_))) {
            self.prompt_count = false;
            return;
        }
        if self.prompt_count && !counting {
            self.prompt_at = Some(self.clock);
        }
        self.prompt_count = counting;
    }

    /// Whether the start prompt's lettering is lit now, while a feature
    /// waits for START FEATURE (`None` while none does).
    pub fn prompt_lettering(&self) -> Option<bool> {
        if !matches!(self.awaiting, Some(Await::Start(_))) {
            return None;
        }
        Some(prompt_lit(
            self.clock - self.prompt_at.unwrap_or(self.clock),
        ))
    }

    /// A Hold & Spin respin is under way (its positions turning).
    pub fn respinning(&self) -> bool {
        self.respin.is_some()
    }

    /// A Hold & Spin respin has been presented and is not over: about to
    /// turn, or turning.
    pub fn respin_in_flight(&self) -> bool {
        self.respin.is_some() || self.respin_landed.is_some()
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

    /// Whether this free game's BIG symbol covers `reel`.
    pub fn big_covers(&self, reel: u8) -> bool {
        self.big
            .as_ref()
            .is_some_and(|(_, first, width)| (*first..first + width).contains(&reel))
    }

    /// A picture's own sound for a win of four or five of it, with no wild on
    /// its line: SND_Pic1 to SND_Pic4, the pictures highest first (the
    /// mobile engine's win-sound rule, 4 of a kind and up; silent on 2 of a
    /// kind in the official app recording). Once a game. STRONG for the
    /// engine, PARTIAL for the cabinet.
    pub fn picture_win(&mut self, picture: usize) {
        const NAMES: [&str; 4] = ["SND_Pic1", "SND_Pic2", "SND_Pic3", "SND_Pic4"];
        if self.pic_sounded {
            return;
        }
        if let Some(name) = NAMES.get(picture) {
            self.cues.push(name);
            self.pic_sounded = true;
        }
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
        self.major = None;
        self.major_pay = None;
        self.major_paid = 0;
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
    pub fn caption(&self) -> Option<String> {
        // Club cabinet: "N FREE GAMES WON!" steady from the result (0.14 s
        // after the reels rest) until START FEATURE is awaited; blank from
        // the press until the first free game (y8c8lEdPW8Y 938.57-946.0 and
        // 949.87; 1bAevGz0K2k 319.3-326.7 and 328.6; VERIFIED). The base
        // stage only: the trigger's clock runs on through the free games.
        if matches!(self.stage, Stage::Base) {
            if self.fg_cut.is_some() {
                return Some(String::new());
            }
            if self.fg_trigger.is_some() && self.awaiting.is_none() {
                return Some(format!("{} FREE GAMES WON!", self.games_won));
            }
        }
        // A retrigger: "N FREE GAMES WON!" from the result for 2.0 s, then the
        // free game on its old total; from the scatters' wave the two take
        // 2.0 s slots (WON at the wave + 2.0 s); the free game on its old
        // total while the banner counts; one step after the last its new
        // total, the slots starting again from it (club cabinets: Autumn Moon
        // nYM0NGR3px0 and TsTj-sxJ6Yc, Happy & Prosperous uXp_uWK4OH8, Panda
        // Magic tg4nMylHxhI, frame by frame; VERIFIED).
        if let (Some((old, new)), Stage::FreeGames { number, .. }, Some(t)) =
            (self.retrigger, self.stage, self.fg_trigger)
        {
            let won = format!("{} FREE GAMES WON!", self.games_won);
            let game = |total: u32| format!("Free Game {number} of {total}");
            let first = self.fg_count_at();
            let done = first + new.saturating_sub(old) as f32 * FG_TICK;
            let slot = |from: f32| ((t - from) / PROMPT_TURN) as u32;
            return Some(if t < PROMPT_TURN {
                won
            } else if t < FG_FLAGS + PROMPT_TURN {
                game(old)
            } else if t < first {
                if slot(FG_FLAGS) % 2 == 1 {
                    won
                } else {
                    game(old)
                }
            } else if t < done {
                game(old)
            } else if slot(done) % 2 == 0 {
                game(new)
            } else {
                won
            });
        }
        // While START FEATURE is awaited, the status alternates in 2.0 s
        // slots from the prompt's start, "Press START FEATURE button" first
        // (club cabinets: y8c8lEdPW8Y 791.67-793.70 and 946.10-948.08,
        // 1bAevGz0K2k 326.77; VERIFIED).
        let won_turn = ((self.waited / PROMPT_TURN) as u32) % 2 == 1;
        // Hold & Spin's end: "Free Spins Completed" from FREE SPINS
        // COMPLETED through the tally and the count, inside the free games
        // too (club cabinets: 1bAevGz0K2k 142.6-164.3, f5sMl9a5jds
        // 256.5-281.1, rwatCLs3QO0; inside the free games y8c8lEdPW8Y 999.5,
        // 1006.0, 1018.7; VERIFIED). Then the base game's bet line, or the
        // free game again.
        if self.stage == Stage::HoldSpinDone && self.hold_and_spin_ending() {
            return Some(FREE_SPINS_COMPLETED.into());
        }
        match (self.awaiting, self.stage) {
            (Some(Await::Start(Feature::FreeGames)), _) if won_turn => {
                Some(format!("{} FREE GAMES WON!", self.games_won))
            }
            (Some(Await::Start(_)), _) => Some("Press START FEATURE button".into()),
            // In the base game the prompt's line stays through the counter's
            // count-up, until the first respin starts (club cabinets:
            // 1bAevGz0K2k 113.9-116.30, gone at 116.37 as the reels move;
            // f5sMl9a5jds 212.1-213.20; VERIFIED). Inside the free games the
            // free game shows from the press (below).
            (None, Stage::HoldSpin { .. })
                if self.fg_return.is_none()
                    && self.started
                    && !self.respun
                    && self.respin.is_none() =>
            {
                Some("Press START FEATURE button".into())
            }
            (Some(Await::Respin), _) if self.respin.is_none() => Some("Press SPIN button".into()),
            // Blank while the banner's total counts up, until one step after
            // it reaches the total (club cabinets: y8c8lEdPW8Y 951.567,
            // 1bAevGz0K2k 330.333; VERIFIED).
            (_, Stage::FreeGames { .. }) if self.fg_count.is_some() => Some(String::new()),
            (_, Stage::FreeGames { number, total }) => {
                Some(format!("Free Game {number} of {total}"))
            }
            // A Hold & Spin inside the free games keeps the free game on the
            // status line through its intro, count-up and respins (cabinet
            // footage: "Free Game 3 of 6" over its board, y8c8lEdPW8Y 962.0-
            // 968.6, 970.8, 971.9; STRONG, one recording).
            (_, Stage::HoldSpin { .. } | Stage::HoldSpinDone) if self.fg_return.is_some() => self
                .fg_return
                .map(|(number, total)| format!("Free Game {number} of {total}")),
            // In the base game the line is blank through the intro (bell,
            // burst, fire and board) and while a respin runs (club cabinets:
            // 1bAevGz0K2k 107.6-113.0, 116.6, 120.0; f5sMl9a5jds 206.5-211.5,
            // 213.6, 214.5; VERIFIED).
            (_, Stage::HoldSpin { .. }) => Some(String::new()),
            _ => None,
        }
    }

    /// Hold & Spin's end is showing: FREE SPINS COMPLETED, the tally on the
    /// WINNER box, or the count still counting.
    fn hold_and_spin_ending(&self) -> bool {
        self.completed_wait.is_some()
            || self.collect_start
            || self.collect.is_some()
            || self.pay.as_ref().is_some_and(|p| p.t < p.seconds)
    }

    /// A base game's Hold & Spin has been counted and the next game has not
    /// begun (free games that follow it clear it first).
    pub fn hold_and_spin_over(&self) -> bool {
        self.stage == Stage::HoldSpinDone
            && self.fg_return.is_none()
            && !self.hold_and_spin_ending()
    }

    /// The Grand this game won, in credits: from its award on the WINNER
    /// display until the next game (or the free games it started).
    pub fn grand_awarded(&self) -> Option<u64> {
        self.grand.filter(|_| {
            self.grand_done
                || self.pay.is_some()
                || self.collect.as_ref().is_some_and(|c| c.grand_up)
        })
    }

    /// The status line after a Grand, at `cents` a credit: "GRAND JACKPOT
    /// Win $24122.63", centred in the bet line's yellow, the dollars never
    /// grouped, from the award until the next game, over the coins' tally
    /// and the count (club cabinet: Wc7QU1C2jWo 3.6 s, one legible frame;
    /// STRONG). The amount is the award in whole credits (the meter's odd
    /// cent at 2c is held over). A casino build filmed other words
    /// ("GRAND JACKPOT $13566.04", OL6Pu1AI-cQ 161-163.5 s; a conflict
    /// recorded); the club's are used in both profiles.
    pub fn grand_caption(&self, cents: u64) -> Option<String> {
        self.grand_awarded().map(|g| {
            format!(
                "GRAND JACKPOT Win {}",
                plain_dollars(g.saturating_mul(cents))
            )
        })
    }

    /// The system message after a Grand, at `cents` a credit: "Call
    /// Attendant : JACKPOT HIT $24122.63" (club cabinet: Wc7QU1C2jWo 3.6 s,
    /// STRONG; the casino build's "Call Attendant - You have won a ..." is
    /// recorded as a conflict). It implies a hand pay; the app still
    /// credits the Grand (the owner's call), and shows the line as filmed.
    pub fn grand_call(&self, cents: u64) -> Option<String> {
        self.grand_awarded().map(|g| {
            format!(
                "Call Attendant : JACKPOT HIT {}",
                plain_dollars(g.saturating_mul(cents))
            )
        })
    }

    /// The message at the right, in place of the start prompts.
    pub fn message(&self) -> Option<String> {
        // Club cabinet: blank from the START FEATURE press until the first
        // free game (line wins included).
        if self.fg_cut.is_some() {
            return None;
        }
        if let Some(note) = &self.win_note {
            return Some(note.clone());
        }
        // Club cabinet: "Scatter wins N" from the flags' wave (the result
        // + 2.6 s) through the count and the wait, until START FEATURE is
        // pressed (y8c8lEdPW8Y 941.20, 1bAevGz0K2k 321.9; VERIFIED); blank
        // through the free games. The official app shows it only while it
        // waits.
        let waiting = matches!(self.awaiting, Some(Await::Start(Feature::FreeGames)));
        ((self.fg_flags || waiting) && self.scatter_award > 0)
            .then(|| format!("Scatter wins {}", self.scatter_award))
    }

    /// A base game's free-games trigger has reached its count: the cabinet
    /// asks for START FEATURE as the win counts (y8c8lEdPW8Y 946.07,
    /// 1bAevGz0K2k 326.77-326.80; VERIFIED).
    pub fn trigger_counting(&self) -> bool {
        self.fg_trigger.is_some_and(|t| t >= self.fg_count_at())
            && matches!(self.stage, Stage::Base)
    }

    /// How long the scatters wave before the scatter win counts: as long as
    /// the title's own SND_Scatter plays, and `WAVE_AFTER_CLIP` (club
    /// cabinets: Genghis Khan, Peace & Long Life, Golden Century, Happy &
    /// Prosperous, Panda Magic and Golden Gong measured; STRONG).
    pub fn wave_seconds(&self) -> f32 {
        if self.scatter_clip > 0.0 {
            self.scatter_clip + WAVE_AFTER_CLIP
        } else {
            FG_COUNT - FG_FLAGS
        }
    }

    /// Seconds into the trigger sequence at which the scatter win counts.
    fn fg_count_at(&self) -> f32 {
        FG_FLAGS + self.wave_seconds()
    }

    /// How long a win through a wild holds WIN's count, from its first line:
    /// the title's wild sound (Golden Century's 2.5 s), and `WILD_COUNT_LAG`.
    pub fn wild_hold(&self) -> f32 {
        let clip = if self.wild_clip > 0.0 {
            self.wild_clip
        } else {
            WILD_HOLD
        };
        clip + WILD_COUNT_LAG
    }

    /// A win through a wild is holding WIN's count: the lines show, the
    /// count waits for the wild's sound (club cabinets: Panda Magic 3.33-3.43 s
    /// from the first line on three cases against its 3.19 s clip, a win
    /// with no panda counting 0.20 s after its line; Golden Century's
    /// standing emperor showing its lines while WIN waits, y8c8lEdPW8Y
    /// 732-733; STRONG).
    pub fn wild_holds(&self) -> bool {
        self.wild.is_some_and(|t| t < self.wild_for)
    }

    /// Whether `front`, the next event to show, waits for a win's count to
    /// finish: a feature's trigger, the free games' prompt, their return
    /// after a Hold & Spin and their end do. Not the free games' own trigger
    /// once its count has begun: the cabinet shows that prompt as the
    /// scatter win starts counting, and the count goes on under it. FEATURE
    /// COMPLETED comes the frame after the last free game's count reaches
    /// the feature's total (club cabinets: y8c8lEdPW8Y 1044.233-1044.267,
    /// 1bAevGz0K2k 367.567; STRONG), not while it counts.
    pub fn waits_for_count(&self, front: Option<&Event>) -> bool {
        let waits = matches!(
            front,
            Some(
                Event::FeatureTriggered { .. }
                    | Event::FreeGamesStarted { .. }
                    | Event::FreeGamesResumed { .. }
                    | Event::FreeGamesCompleted { .. }
            )
        );
        let prompt_now = self.trigger_counting()
            && matches!(
                front,
                Some(
                    Event::FeatureTriggered {
                        feature: Feature::FreeGames,
                        ..
                    } | Event::FreeGamesStarted { .. }
                )
            );
        waits && !prompt_now
    }

    /// START FEATURE pressed for the free games: the flags go still and the
    /// screen holds until the cut (club cabinets: the symbols still within
    /// 0.05 s of the press, nothing moving until the reels start; VERIFIED).
    fn press_free_games(&mut self) {
        self.fg_cut = Some(0.0);
        self.fg_flags = false;
        self.scatter_paid = false;
    }

    /// Run the time from the START FEATURE press on; true on the frame of
    /// the cut, when the first free game is to start.
    fn advance_cut(&mut self, dt: f32) -> bool {
        let Some(t0) = self.fg_cut else {
            return false;
        };
        let t = t0 + dt;
        self.fg_cut = Some(t);
        crossed(t0, t, FG_CUT)
    }
}

/// The status line's turns while START FEATURE is awaited, in seconds.
const PROMPT_TURN: f32 = 2.0;

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
    mut commands: Commands,
) {
    if !fidelity.0 || show.awaiting.is_none() || show.blocks() {
        return;
    }
    show.waited += smoke.dt(&time);
    // The free games' prompt shows as the scatter win starts counting; its
    // lettering's blink restarts as the count completes.
    show.follow_count(rollup.active());
    let pressed = (input.spin && !input.skip_spin)
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter);
    let auto = smoke.frames_left.is_some() && show.waited > 2.5;
    if pressed || auto {
        // START FEATURE goes straight into the free games (cabinet footage:
        // the banner reads FREE GAME 1 OF n as the first game spins; the base
        // screen does not come back in between).
        if matches!(show.awaiting, Some(Await::Start(Feature::FreeGames))) {
            // The press is silent and completes a count still running (its
            // tune stops); the flags go still and nothing moves until the
            // first free game spins (club cabinet: 0.50-0.53 s after the
            // prompt goes).
            rollup.t = rollup.duration;
            if let Some(id) = rollup.tune.take() {
                commands.entity(id).try_despawn();
            }
            show.press_free_games();
        }
        show.awaiting = None;
        show.prompt_at = None;
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
/// The Hold & Spin grid lines over the reels: our row lines under the
/// pack's frame.
#[derive(Component)]
pub struct HsFrame;
/// One of the Hold & Spin grid's two row lines.
#[derive(Component)]
pub struct HsRowLine;

/// The Hold & Spin grid's row lines as club cabinets show them: about
/// 2.5-3 px on our canvas, far paler and less saturated than the gold
/// dividers (15-16 px, the base frame's own, as ours are; VERIFIED on two
/// venues, f5sMl9a5jds 212.8 and y8c8lEdPW8Y 973.0); a pale cream-gold
/// (PARTIAL: the hue depends on the camera, estimated after removing one
/// camera's blue cast; the 'lavender' read on that camera alone is its
/// cast, HYPOTHESIS). The pack's frame (the official app's) draws them
/// 2 px of its art, stretched to 5-6.5 px of saturated gold; they are
/// cleared from it (`clear_row_lines`) and drawn here under its dividers.
const HS_ROW_LINE: Color = Color::srgb(0.94, 0.82, 0.58);
const HS_ROW_LINE_H: f32 = 3.0;
/// The message at the right of the meter band during a feature.
#[derive(Component)]
pub struct FeatureMessage;

const BANNER_SIZE: (f32, f32) = (760.0, 96.0);
/// The banner's middle line: its gold rim (the node's top, canvas y 124.5)
/// clear of the Major's and Minor's values above it, whose ink ends at
/// about y 122 (shadow included). At + 6 the rim hid their bottom 7-8 px
/// whenever a banner showed (measured on a capture). The cabinet has no
/// overlap (separate screens); the official app's banner sits wholly below
/// its bar (VERIFIED, official mobile app).
const BANNER_Y: f32 = GRID_CENTER.1 - 1.5 * PITCH.1 + 16.0;
/// The counter's digits' height.
const COUNTER_DIGIT_H: f32 = 52.0;

/// The start prompt's lettering: PRESS START FEATURE BUTTON, drawn by us
/// over the banner's blue bar (the pack has only the official app's SELECT
/// SPIN TO START FEATURE art; the cabinet shows the same banner for both
/// features).
#[derive(Component)]
pub struct PromptLettering;

/// The banner's blue field on our 760 x 96 banner (left, top, width,
/// height; measured on the bar's art).
const PROMPT_FIELD: (f32, f32, f32, f32) = (15.0, 11.0, 730.0, 64.0);
/// The lettering as club footage shows it (the same within a pixel on four
/// frames of two recordings; VERIFIED): one line, centred, across 0.96 of
/// the field; START FEATURE's caps 0.42 of the field's height, PRESS and
/// BUTTON small caps 0.81 of theirs on the same baseline; pale cream (judged
/// against white text on the same screen, not measured as a colour) with a
/// dark shadow. Arial Black (caps 0.716 of its size) stands in for the
/// game's brush face: its letters run narrow of the cabinet's, so the line
/// is widened by `PROMPT_WIDEN`, and doubled spaces give the cabinet's gaps
/// (0.03-0.045 of the line each); its caps sit a little low in their line,
/// so the line is raised by `PROMPT_RAISE` to centre them (both checked on a
/// capture: the line 0.96 of the field, centred within a pixel).
const PROMPT_LARGE: f32 = 37.0;
const PROMPT_SMALL: f32 = 30.0;
const PROMPT_WIDEN: f32 = 1.09;
const PROMPT_RAISE: f32 = 2.0;
const PROMPT_CREAM: Color = Color::srgb(0.929, 0.898, 0.827);
/// The lettering blinks while the bar stays: a 20-video-frame (0.667 s)
/// cycle in every cycle measured, about 0.35 s lit, and the first lit phase
/// about 0.25 s as the banner pops in (VERIFIED: three Hold & Spin waits and
/// a free games wait, y8c8lEdPW8Y 969.10-970.53 and 2519.57-2521.77,
/// 1bAevGz0K2k 113.77-115.37, y8c8lEdPW8Y 945.8-949.8; the free games wait
/// measured 0.367 s lit and a 0.30 s first phase, within a camera frame).
const PROMPT_BLINK_PERIOD: f32 = 2.0 / 3.0;
const PROMPT_BLINK_ON: f32 = 0.35;
const PROMPT_BLINK_PHASE: f32 = 0.10;

/// Whether the start prompt's lettering is lit `since` seconds after the
/// prompt showed.
pub fn prompt_lit(since: f32) -> bool {
    (since + PROMPT_BLINK_PHASE).rem_euclid(PROMPT_BLINK_PERIOD) < PROMPT_BLINK_ON
}

/// The banner, its counter digits, the Hold & Spin grid lines.
pub fn spawn_feature_layer(
    c: &mut ChildSpawnerCommands,
    art: &ArtPack,
    font: &Handle<Font>,
    heavy: &Handle<Font>,
    look: &crate::looks::TitleLook,
) {
    // The prompt's lettering: cream, or the title's own (Genghis Khan's and
    // Peace & Long Life's gold).
    let lettering = look.prompt.map_or(PROMPT_CREAM, crate::looks::rgb);
    let ui = |k: &str| art.ui.get(k).cloned().unwrap_or_default();
    let (gx, gy) = GRID_CENTER;
    c.spawn((
        HsFrame,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        Visibility::Hidden,
    ))
    .with_children(|f| {
        // The row lines between the rows, under the frame's dividers and
        // border.
        for k in [-0.5, 0.5] {
            f.spawn((
                HsRowLine,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(gx - 2.5 * PITCH.0),
                    top: Val::Px(gy + k * PITCH.1 - HS_ROW_LINE_H / 2.0),
                    width: Val::Px(5.0 * PITCH.0),
                    height: Val::Px(HS_ROW_LINE_H),
                    ..default()
                },
                BackgroundColor(HS_ROW_LINE),
            ));
        }
        // The grid art is the reel panel's own size (749 x 380): laid exactly
        // over the panel, so its border is the panel's border.
        f.spawn(sprite(
            ui("hs_frame"),
            gx,
            gy + 4.5 * PANEL_SCALE.1,
            1240.0 * PANEL_SCALE.0,
            629.0 * PANEL_SCALE.1,
        ));
    });
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
        // The start prompt's lettering, centred in the blue field.
        let (fx, fy, fw, fh) = PROMPT_FIELD;
        let face = |size: f32| TextFont {
            font: bevy::text::FontSource::Handle(heavy.clone()),
            font_size: FontSize::Px(size),
            ..default()
        };
        b.spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(fx * bw / 760.0),
            top: Val::Px(fy * bh / 96.0 - PROMPT_RAISE),
            width: Val::Px(fw * bw / 760.0),
            height: Val::Px(fh * bh / 96.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|f| {
            f.spawn((
                PromptLettering,
                Text::new("PRESS  "),
                face(PROMPT_SMALL),
                TextColor(lettering),
                TextShadow {
                    offset: Vec2::new(2.0, 2.0),
                    color: Color::srgba(0.02, 0.02, 0.06, 0.85),
                },
                TextLayout::default().with_no_wrap(),
                UiTransform {
                    scale: Vec2::new(PROMPT_WIDEN, 1.0),
                    ..UiTransform::IDENTITY
                },
                Visibility::Hidden,
            ))
            .with_children(|t| {
                t.spawn((
                    TextSpan::new("START  FEATURE  "),
                    face(PROMPT_LARGE),
                    TextColor(lettering),
                ));
                t.spawn((
                    TextSpan::new("BUTTON"),
                    face(PROMPT_SMALL),
                    TextColor(lettering),
                ));
            });
        });
        // The game number ends a little before OF and the total starts a
        // little after it, so two-digit numbers grow away from OF (club
        // cabinets; the anchors from each pack's own banner lettering:
        // crate::looks).
        let (number_end, total_start) = look.counter;
        for number in 0..2u8 {
            let (left, justify) = if number == 0 {
                (bw * number_end - 120.0, JustifyContent::FlexEnd)
            } else {
                (bw * total_start, JustifyContent::FlexStart)
            };
            b.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(0.0),
                width: Val::Px(120.0),
                height: Val::Px(bh),
                flex_direction: FlexDirection::Row,
                justify_content: justify,
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

/// The banner's key for the start prompt, which has no art of its own.
const PROMPT_KEY: &str = "prompt";

/// A free game's win shows "WIN $x WIN" over the reels only when it is at
/// least this many times the total bet; a smaller win counts on the WIN
/// meter under "FREE GAME n OF 6" and its counter. Club cabinet footage
/// (1bAevGz0K2k, y8c8lEdPW8Y, f5sMl9a5jds, kVjwYtVwhkk; bets $2.50-$5): shown
/// at 11.0x, 12.8x, 16x, 20x, 22x (twice), 40x and 44x; not shown at 18 wins
/// from 0.1x to 6.0x. The rule STRONG (26 paid free games, five features);
/// the limit lies above 6.0x and at most 11.0x, and 10x is our choice
/// (HYPOTHESIS). A credit limit is ruled out; a fixed dollar limit ($25.50 to
/// $32) also fits the footage.
const FG_WIN_BANNER_X: u64 = 10;
/// A free game's big win stays on the banner this long after its count,
/// then "FREE GAME n OF 6" comes back until the next game (club cabinet:
/// 1.0 s in three cases ended by a press; STRONG).
const FG_WIN_HOLD: f32 = 1.0;

/// What the WIN banner over the reels shows: a free game's big win while it
/// counts and a moment after, or a Hold & Spin's payment (always shown:
/// "WIN $0.29 WIN" at 1bAevGz0K2k 164.3). The count is this free game's win
/// alone: each free game's start settles the last count (`Rollup::settle`)
/// and its credit arrives in one step.
fn banner_win(show: &FeatureShow, rollup: &Rollup, total_bet: u64) -> Option<u64> {
    let big = rollup.win() >= FG_WIN_BANNER_X.saturating_mul(total_bet.max(1));
    let held = rollup.active() || rollup.after < FG_WIN_HOLD;
    (matches!(show.stage, Stage::FreeGames { .. })
        && show.game_won
        && show.reveal.is_none()
        && rollup.win() > 0
        && big
        && held)
        .then(|| rollup.shown().saturating_sub(rollup.from))
        .or(show.pay_banner())
}

impl FeatureShow {
    /// The banner over the reels: its art's key (`PROMPT_KEY` for the start
    /// prompt), or none; `game_win` is what the WIN banner shows, if it does.
    fn banner_key(&self, game_win: Option<u64>) -> Option<String> {
        match (self.awaiting, self.stage) {
            (_, Stage::FreeGames { .. } | Stage::HoldSpinDone) if game_win.is_some() => {
                Some("banner_win".to_string())
            }
            (_, Stage::HoldSpin { .. }) if self.intro.is_some() => None,
            // Between FREE SPINS COMPLETED and the WIN banner: no banner.
            (_, Stage::HoldSpinDone) if self.collect.is_some() => None,
            // Both features' start prompt: the bar with our lettering.
            (Some(Await::Start(_)), _) => Some(PROMPT_KEY.to_string()),
            // Back from a Hold & Spin: no banner until the next free game
            // (club cabinet, y8c8lEdPW8Y 1019.93-1020.37).
            (_, Stage::FreeGames { .. }) if self.resuming => None,
            (_, Stage::FreeGames { .. }) => Some("banner_free_game".to_string()),
            (_, Stage::FreeGamesDone) => Some("banner_feature_completed".to_string()),
            (_, Stage::HoldSpin { .. }) if self.counter > 0 => {
                Some(format!("banner_spins_{}", self.counter.min(3)))
            }
            (_, Stage::HoldSpin { .. }) => None,
            (_, Stage::HoldSpinDone) => Some("banner_spins_completed".to_string()),
            _ => None,
        }
    }

    /// The numbers of "FREE GAME n OF m" on the banner, while it shows them.
    fn counter_numbers(&self, game_win: Option<u64>) -> Option<[String; 2]> {
        match (self.awaiting, self.stage) {
            (None | Some(Await::Respin), Stage::FreeGames { number, total })
                if game_win.is_none() && !self.resuming =>
            {
                Some([number.to_string(), self.counter_total(total).to_string()])
            }
            _ => None,
        }
    }
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
    mut lettering: Query<
        &mut Visibility,
        (
            With<PromptLettering>,
            Without<FeatureBanner>,
            Without<CounterDigit>,
            Without<HsFrame>,
            Without<HsCell>,
            Without<BannerAmount>,
        ),
    >,
    sim: Res<Sim>,
    gamble: Res<GambleShow>,
) {
    if !fidelity.0 {
        return;
    }
    let game_win = banner_win(&show, &rollup, sim.game.total_bet());
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
    let text = show.banner_key(game_win);
    let prompt = text.as_deref() == Some(PROMPT_KEY);
    let handle = text
        .as_deref()
        .filter(|_| !prompt)
        .and_then(|k| art.ui.get(k));
    // Nothing of the base screen shows over the gamble: its banner (FEATURE
    // COMPLETED) stands above the gamble screen's top and comes back by
    // itself after a loss (club cabinets: LwC3hVqFRMg 18:30.4-18:33.7,
    // _XHUzg4qDXI 1:13.00; VERIFIED).
    for mut vis in &mut banner {
        let want = if !gamble.active && (handle.is_some() || prompt) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for mut image in &mut banner_text {
        if let Some(h) = handle
            && image.image != *h
        {
            image.image = h.clone();
        }
        // The prompt shows the bar alone under our lettering.
        let tint = if prompt { Color::NONE } else { Color::WHITE };
        if image.color != tint {
            image.color = tint;
        }
    }
    // Only the lettering blinks; the bar stays.
    let lit = prompt && show.prompt_lettering() == Some(true);
    for mut vis in &mut lettering {
        let want = if lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    // The counter's numbers, in the banner's own gold digits.
    let counter = show.counter_numbers(game_win);
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
    // Back in the free games after a Hold & Spin, they stay dark until the
    // next free game spins (club cabinet: the row lines go with the plaque,
    // the open cells stay dark).
    let dark =
        (show.hold_and_spin() && (show.grid || show.since_burst(1.0 / 30.0))) || show.resuming;
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
// The sequence follows club cabinet footage of the feature: the feature
// bell; the other symbols dim and every coin bursts into flame with the
// trigger sound; one fire (dragons, then a white-out) sweeps the screen and
// clears straight to the Hold & Spin grid; "PRESS START FEATURE BUTTON",
// its lettering blinking. After the press the respin counter counts up 1, 2,
// 3, and the first respin starts. Held coins burn with a fire ring; open
// positions show dimmed symbols and spin as small reels of their own,
// stopping one by one; a coin that lands bursts in with the Hold & Spin
// "doonk". The counter drops as a respin starts and returns to 3 when a coin
// has landed. At the end each coin's comet flies to a flame-bordered WINNER
// total over the jackpot bar.

/// Seconds into the trigger sequence for each step (it begins when the
/// result shows, 0.17 s after the last reel is still). The bell's place is
/// the official app's (recorded with sound; the cabinet copies have none);
/// the rest is two club recordings matched frame by frame against the fire's
/// own 54 frames (y8c8lEdPW8Y 965.60-969.10, 1bAevGz0K2k 110.17-113.67;
/// STRONG): the coins burst 2.70 s after the reels are still, to
/// SND_FeatureTrigger; the fire's first dragon head 0.90-0.93 s after the
/// burst; the screen white about 1.8-2.3 s after it, the board already the
/// Hold & Spin board under it; the fire gone (only its top band left) by
/// 2.70 s; the prompt and the Hold & Spin music 3.50 s after the burst. The
/// cabinet shows no medallion over the reels, no COLLECT screen and no
/// second fire: those are the official app's, and the app follows the
/// cabinet (owner, 2026-10-18), which replaces the owner's 2026-10-14 choice
/// of a 1 s medallion.
const INTRO_BELL: f32 = 0.01;
const INTRO_BURST: f32 = 2.53;
/// The fire's frame 0, so its frame 1 (the first dragon head) shows 0.91 s
/// after the burst.
const INTRO_TRANSITION: f32 = INTRO_BURST + 0.87;
/// The fire (54 frames) runs at 22.3 fps: a linear fit of its key frames on
/// both recordings (21.5-23), which puts frame 41 (the top band) at the
/// burst + 2.71 s and its embers' end at + 3.07 s.
const FIRE_FPS: f32 = 22.3;
/// The board and its backdrop come up under the fire's full white (frames
/// 22-33), half way in.
const INTRO_GRID: f32 = INTRO_TRANSITION + 27.0 / FIRE_FPS;
/// The prompt (the music starts with it; its onset on the cabinet is not
/// measurable, and with the prompt is the official app's order).
const INTRO_END: f32 = INTRO_BURST + 3.5;
/// The counter's count-up after START FEATURE (the official app: a
/// SND_Fea_Inc tick a step, the first 0.27 s after the press, then every
/// 0.2 s; a burst on the banner's 3), and how long the count holds the 3.
/// The START FEATURE press plays the first respin at once (`feature_press`
/// zeroes the step timer), and its start, queued, shows the frame the count
/// ends: so the hold is the time from the 3 to the first respin. Club
/// cabinets start it by itself 0.55-0.63 s after the 3, mean 0.59 s
/// (VERIFIED, four features on three recordings at two venues: y8c8lEdPW8Y
/// 970.93 to 971.53 and 2522.15 to 2522.70, 1bAevGz0K2k 115.78 to 116.37,
/// f5sMl9a5jds 212.67 to 213.30; the counter drops to 2 a frame or two
/// before the reels move). The official app waits about 1.9 s; the cabinet
/// outranks it. (The earlier hold of 1.4 s was thought to be followed by the
/// 0.5 s step pause; it was not, and gave about 1.4 s.)
const COUNT_FIRST: f32 = 0.27;
const COUNT_STEP: f32 = 0.2;
const COUNT_HOLD: f32 = 0.59;
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
/// Seconds from the end of the respin that fills the grid to the Grand's
/// bell (3.55 s after the fifteenth coin lands, less the respin's settle).
const GRAND_BELL_AFTER: f32 = 3.2;
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

/// A coin of the collection: its position, amount, whether it is a jackpot
/// and which.
type CollectCoin = (u8, u8, u64, bool, Option<JackpotTier>);

/// The coins being added to the WINNER box, then the Grand.
#[derive(Clone, Debug, Default)]
pub struct Collect {
    t: f32,
    /// Each step's coin, in order; the Grand, last, has no position
    /// (`GRAND_STEP`).
    coins: Vec<CollectCoin>,
    /// Steps begun, and amounts added to the banner.
    launched: usize,
    counted: usize,
    /// The Grand's step has begun (its plaque shows), and its fire.
    grand_up: bool,
    grand_fire: bool,
    /// A Major's step has begun and its plaque shows, until its own count
    /// is over.
    major_up: bool,
}

/// The position of the Grand's step (it has no coin).
const GRAND_STEP: u8 = u8::MAX;

/// The collection's order: a Major first, on its own plaque with its own
/// count, then the other coins reel by reel, top to bottom (club cabinet:
/// Autumn Moon 5Z1-hFzpyh0 525-590 s, the Major's amount green on WINNER
/// with a coin shower, then the coins tallied without it; its Hold & Spin
/// scenes are Golden Century's in the mobile build: STRONG, PARTIAL for
/// Golden Century itself). The owner chose coins first for the Grand; for
/// the Major the cabinet's order is followed.
fn majors_first(coins: Vec<CollectCoin>) -> Vec<CollectCoin> {
    let (mut first, rest): (Vec<CollectCoin>, Vec<CollectCoin>) = coins
        .into_iter()
        .partition(|coin| coin.4 == Some(JackpotTier::Major));
    first.extend(rest);
    first
}

/// The Grand's fire. On the cabinet a fire wipes both screens, from the
/// bottom up, and clears, bottom first, to reveal WINNER / GRAND JACKPOT
/// (OL6Pu1AI-cQ 3.4-4.4 s at 0.1 s steps: first flames 3.4-3.5 s, the main
/// screen covered 3.6-3.7, the top 3.8-4.1, clear at 4.2 and 4.3-4.4; one
/// Golden Century machine, VERIFIED there). The pack's transition fire
/// (the intro's) has the same shape: frames 15-20 fill from the bottom,
/// 21-35 cover, 36-41 clear (alpha coverage measured), 26 frames from fill
/// to clear, 0.87 s at 30 fps against the cabinet's 0.9-1.0 s (STRONG that
/// it is the same fire). It starts `GRAND_FIRE_LEAD` before the Grand's
/// step, so the plaque switches on under full cover (frame 27) and shows as
/// the fire clears (our coins-first order: its timing HYPOTHESIS).
const GRAND_FIRE_FPS: f32 = ANIM_FPS;
const GRAND_FIRE_LEAD: f32 = 27.0 / GRAND_FIRE_FPS;

/// The end of Hold & Spin. "FREE SPINS COMPLETED" with SND_Fea_End (3 s),
/// then the collection, as club cabinets show it (VERIFIED: 1bAevGz0K2k
/// 2:25-2:45, y8c8lEdPW8Y 1001.9-1019.9, f5sMl9a5jds 4:18-4:38, rwatCLs3QO0):
/// in the frame the banner goes (3.07-3.10 s after it showed), the jackpot
/// meters give way to an empty flame-bordered WINNER box, with a line of the
/// live Grand and Major under it, and nothing shows over the reels; each
/// coin in turn, reel by reel and top to bottom, 1.33-1.37 s apart (the
/// first 0.33-0.43 s after the box), flares and its fire comet (the game's
/// bolt animations) flies up to the box, which adds its value 0.57-0.73 s
/// later (mean 0.67; the first coin's 0.70-0.73) with a splash, to
/// SND_Small_5 (SND_Medium_1 for a big coin); after the last value the count
/// begins: the WIN banner over the reels, counting up from nothing with WIN,
/// its letters bursting from its first moment (again every 59 frames until
/// the next game), the payout music and a coin shower, WIN counting the total
/// up over the music's length (SPIN completes it; the music stops); the box
/// keeps the total and gives way to the meters 0.55 s after the count is
/// complete. Measured at 30 fps on four club recordings (1bAevGz0K2k 145.200
/// box, 145.533 first flare, 146.267 value; y8c8lEdPW8Y 998.867 COMPLETED,
/// 1001.933, 1002.267, 1003.000; f5sMl9a5jds 255.700, 258.800, 259.233,
/// 259.933; rwatCLs3QO0 clip 1.95, 5.033, 5.467, 6.167). The official app
/// instead tallies onto an empty WIN banner over the reels (2.86 s after FREE
/// SPINS COMPLETED, its first coin a frame later, 0.45 s comets, the letters
/// 0.8 s after the last value); the app follows the cabinet. The rumble
/// (SND_Rumble, 0.37 s in) and the pause before the count (0.87 s) are the
/// official app's: the cabinet copies have no sound (whether the cabinet
/// rumbles there is UNKNOWN; its place here HYPOTHESIS), and the cabinets
/// disagree on the pause (1.6 s and 3.8 s). Our cabinet keeps its own
/// meters; a Grand is added to the box after the coins, its plaque on the
/// box while it is (owner: coins first).
const COMPLETED_SECONDS: f32 = 3.0;
const COLLECT_RUMBLE: f32 = 0.37;
const COLLECT_FIRST: f32 = 0.38;
const COLLECT_STEP: f32 = 1.347;
/// From a coin's flare to its value on the box (the comet's flight).
const BOLT_FLIGHT: f32 = 0.67;
/// The frame at which each of the game's five bolt animations (bolt0 to
/// bolt4, 30 fps) reaches the box: its comet's first opaque pixels in the
/// frame's top row (measured on the frames, the same in every title's pack;
/// bolt2 grazes the top at frame 16). The frames up to it are stretched over
/// `BOLT_FLIGHT`, so the comet lands as the value does; the rest play at
/// 30 fps.
const BOLT_ARRIVE: [f32; 5] = [16.0, 13.0, 17.0, 12.0, 12.0];

/// The frame of bolt animation `k` to show `since` seconds after its coin
/// flared.
fn bolt_frame(k: usize, since: f32) -> usize {
    let arrive = BOLT_ARRIVE
        .get(k)
        .copied()
        .unwrap_or(BOLT_FLIGHT * ANIM_FPS);
    let since = since.max(0.0);
    let n = if since < BOLT_FLIGHT {
        since * arrive / BOLT_FLIGHT
    } else {
        arrive + (since - BOLT_FLIGHT) * ANIM_FPS
    };
    n as usize
}
/// From the last value to the count, and the WIN letters' bursts' period.
const PAY_DELAY: f32 = 0.87;
const FLARE_EVERY: f32 = 59.0 / 30.0;
/// The WINNER box gives way to the jackpot meters this long after the count
/// is complete (club cabinets 0.53-0.60 s, through one dark frame; the
/// owner's cabinet about 0.5 s).
const WINNER_HOLD: f32 = 0.55;
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

/// What a moment of the collection did: whether the rumble starts, the
/// coins' sounds due, and the total once the count is due.
#[derive(Debug, Default)]
struct CollectStep {
    rumble: bool,
    sounds: Vec<&'static str>,
    done: Option<u64>,
    /// A Major's amount has shown on its plaque: its own count begins.
    major: Option<u64>,
}

impl FeatureShow {
    /// Begin the collection of `coins` (in order; the Grand last), WIN
    /// standing at `pay_base`: the empty WINNER box shows from this frame,
    /// the one FREE SPINS COMPLETED goes (club cabinets).
    fn begin_collect(&mut self, coins: Vec<CollectCoin>, pay_base: u64) {
        self.winner = Some(0);
        self.paid = 0;
        self.pay_base = pay_base;
        self.flares_at = None;
        self.collect = Some(Collect { coins, ..default() });
    }

    /// Run the collection on by `dt` seconds (at `self.clock`): each coin's
    /// comet and its value on the box with a splash, a Major's plaque and
    /// its own count, the Grand's fire and plaque, then the count.
    fn advance_collect(&mut self, dt: f32, total_bet: u64) -> CollectStep {
        let mut step = CollectStep::default();
        // A Major's own count holds the collection until it is over.
        if self.major_pay.is_some() {
            return step;
        }
        let Some(mut c) = self.collect.take() else {
            return step;
        };
        let clock = self.clock;
        let t0 = c.t;
        c.t += dt;
        step.rumble = crossed(t0, c.t, COLLECT_RUMBLE);
        // The Grand's fire starts ahead of its plaque.
        let next_at = COLLECT_FIRST + c.launched as f32 * COLLECT_STEP;
        if !c.grand_fire
            && c.coins
                .get(c.launched)
                .is_some_and(|coin| coin.0 == GRAND_STEP)
            && c.t >= next_at - GRAND_FIRE_LEAD
        {
            c.grand_fire = true;
            self.transition_at = Some((clock, GRAND_FIRE_FPS));
        }
        // Each coin in turn flares and sends its comet to the box; a Major
        // and the Grand show their plaques instead.
        while c.launched < c.coins.len() && c.t >= COLLECT_FIRST + c.launched as f32 * COLLECT_STEP
        {
            let (reel, row, amount, jackpot, tier) = c.coins[c.launched];
            if reel == GRAND_STEP {
                c.grand_up = true;
            } else if tier == Some(JackpotTier::Major) {
                c.major_up = true;
            } else {
                self.bolt = Some((reel, row, clock));
            }
            step.sounds.push(if jackpot {
                "SND_Credit1"
            } else if amount >= BIG_COIN_RATIO * total_bet {
                "SND_Medium_1"
            } else {
                "SND_Small_5"
            });
            c.launched += 1;
        }
        // Its value on the box as the comet arrives, with a splash; a
        // Major's amount on its plaque, where its own count begins.
        while c.counted < c.launched
            && c.t >= COLLECT_FIRST + c.counted as f32 * COLLECT_STEP + BOLT_FLIGHT
        {
            let (_, _, amount, _, tier) = c.coins[c.counted];
            c.counted += 1;
            // A progressive's meter restarts as its amount is shown.
            if let Some(tier @ (JackpotTier::Major | JackpotTier::Grand)) = tier {
                self.link_won.push(tier);
            }
            if tier == Some(JackpotTier::Major) {
                self.major = Some(amount);
                step.major = Some(amount);
                self.collect = Some(c);
                return step;
            }
            self.winner = Some(self.winner.unwrap_or(0) + amount);
            self.splash_at = Some(clock);
        }
        let last =
            COLLECT_FIRST + c.coins.len().saturating_sub(1) as f32 * COLLECT_STEP + BOLT_FLIGHT;
        if c.counted >= c.coins.len() && c.t >= last + PAY_DELAY {
            step.done = self.winner;
        } else {
            self.collect = Some(c);
        }
        step
    }

    /// A Major's own count is over: its plaque gives way to the empty
    /// WINNER box and the tally goes on, the next coin `COLLECT_FIRST`
    /// after (club cabinet: the box empty again as a comet leaves the reels,
    /// Autumn Moon 5Z1-hFzpyh0 560 s).
    fn end_major(&mut self, amount: u64) {
        self.major_paid += amount;
        self.winner = Some(0);
        if let Some(c) = self.collect.as_mut() {
            c.major_up = false;
            c.t = c.counted as f32 * COLLECT_STEP;
        }
    }

    /// A Major's plaque shows (from its step until its own count is over),
    /// and its amount on it once that count has begun.
    pub fn major_showing(&self) -> bool {
        self.collect.as_ref().is_some_and(|c| c.major_up)
    }

    fn major_amount(&self) -> Option<u64> {
        self.major
            .filter(|_| self.major_showing() && self.major_pay.is_some())
    }

    /// What a Major's own count has put on WIN so far.
    fn major_counted(&self) -> Option<u64> {
        match &self.major_pay {
            Some(p) => {
                let k = (p.t / p.seconds).clamp(0.0, 1.0);
                Some(self.major_paid + (p.amount as f32 * k) as u64)
            }
            None => (self.major_paid > 0).then_some(self.major_paid),
        }
    }

    /// The Grand's plaque shows (from its step until the count is over).
    pub fn grand_showing(&self) -> bool {
        self.grand.is_some()
            && (self.collect.as_ref().is_some_and(|c| c.grand_up) || self.pay.is_some())
    }

    /// What WIN shows while the feature's total is counted onto it: a
    /// Major's own count, then the coins' (and the Grand's) on top.
    pub fn grand_win(&self) -> Option<u64> {
        let major = self.major_counted();
        match self.pay_banner() {
            Some(w) => Some(self.pay_base + major.unwrap_or(0) + w),
            None => major.map(|m| self.pay_base + m),
        }
    }

    /// What the WIN banner over the reels shows: nothing while the coins
    /// go to the WINNER box; then the amount counted so far, from nothing,
    /// in step with WIN (club cabinets: the banner at $1.08 with WIN at
    /// $11.08 from $10.00); then the total until the next game.
    pub fn pay_banner(&self) -> Option<u64> {
        if let Some(p) = &self.pay {
            let k = (p.t / p.seconds).clamp(0.0, 1.0);
            return Some(self.paid + (p.amount as f32 * k) as u64);
        }
        if self.collect.is_some() {
            return None;
        }
        // Once paid, it holds the sum until the game moves on.
        (self.stage == Stage::HoldSpinDone && self.collected.is_some()).then_some(self.paid)
    }

    /// The WINNER box shows over the jackpot bar: from its arrival during
    /// the collection, through the count, until a moment after the count is
    /// complete.
    pub fn winner_showing(&self) -> bool {
        self.winner.is_some() || self.pay.is_some() || self.held_winner().is_some()
    }

    /// The total the WINNER box holds after the count, while it stays.
    fn held_winner(&self) -> Option<u64> {
        self.winner_hold
            .filter(|(until, _)| self.clock < *until)
            .map(|(_, total)| total)
    }

    /// The amount in the WINNER box: empty until the first value lands,
    /// then the values added so far, then the total.
    fn winner_amount(&self) -> Option<u64> {
        self.winner.or(self.held_winner()).filter(|w| *w > 0)
    }

    /// A big payment is counting (coins rain): the count, or a Major's own.
    pub fn pay_shower(&self, bet: u64) -> bool {
        let big = |p: &Pay| p.t < p.seconds && p.amount >= PAY_SHOWER_RATIO * bet;
        self.pay.as_ref().is_some_and(big) || self.major_pay.as_ref().is_some_and(big)
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
    // The comet flies up to the middle of the WINNER box.
    let (tx, ty) = WINNER_CENTRE;
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
            // Straight above the coin no bolt ends exactly over its ring:
            // the one ending nearest the middle is taken (it ends inside the
            // box all the same).
            let (sx, miss) = if (tx - cx).abs() < 1.0 {
                (sy * 0.4, (dx * sy * 0.4).abs())
            } else {
                ((tx - cx) / dx, 0.0)
            };
            if sx <= 0.0 || sy <= 0.0 {
                continue;
            }
            let cost = (sx / sy).ln().abs() + miss;
            if cost < best.4 {
                best = (k, flip, sx, sy, cost);
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

/// The top of the WINNER total's flame box (y 33-113, above the WIN
/// banner's top at 124.5).
const WINNER_TOP: f32 = 33.0;
/// The WINNER total's flame box: the pack's own flame-bordered box (536 x
/// 128), its inner aspect about 5.6 as on the cabinet.
const WINNER_BOX: (f32, f32) = (440.0, 80.0);
/// The middle of the WINNER box, where the comets end.
const WINNER_CENTRE: (f32, f32) = (960.0, WINNER_TOP + WINNER_BOX.1 * 0.5);
/// Where the splash animation (512 x 256) draws its box: x 40..472,
/// y 145..223 (measured on its frames).
const SPLASH_BOX: (f32, f32, f32, f32) = (40.0, 145.0, 432.0, 78.0);

/// The WINNER plate covers the whole jackpot bar (its bottom at canvas y
/// 147.2), so none of the meters' digits show under it; the WIN banner
/// (from y 124.5) draws over its foot while the count runs. Our single
/// canvas has no top screen: the plate standing in for it is our layout
/// (the owner's call; the suggested default).
const WINNER_PLATE_H: f32 = 148.0;
/// The line of the live Grand and Major under the box: its top and size,
/// and its colour (yellow on one club camera, green-tinted on another:
/// PARTIAL).
const WINNER_LINE_TOP: f32 = 118.0;
const WINNER_LINE_SIZE: f32 = 16.0;
const WINNER_GREEN: Color = Color::srgb(0.86, 0.93, 0.36);
/// The total's size in the box: the box's inner black is about 36 px tall,
/// and on the cabinet the amount's caps are about 0.6 of it.
const WINNER_AMOUNT_SIZE: f32 = 40.0;

/// The Hold & Spin top screen's stand-in over the jackpot bar.
#[derive(Component)]
pub struct HsCollectedPlate;
/// The number on the collected-coin disc.
#[derive(Component)]
pub struct HsCollectedCount;
/// A fire dragon beside the disc (mirrored on the right).
#[derive(Component)]
pub struct HsDragon;

/// The disc's diameter (its lettering clear of the feature banner's rim,
/// canvas y 124.5) and the corner boxes' height on the plate.
const HS_DISC: f32 = 140.0;
const HS_BOX_H: f32 = 68.0;
/// Each corner box's place: (tier, right-hand side, bottom row).
const HS_CORNERS: [(JackpotTier, bool, bool); 4] = [
    (JackpotTier::Grand, false, false),
    (JackpotTier::Major, true, false),
    (JackpotTier::Minor, false, true),
    (JackpotTier::Mini, true, true),
];
/// The corner boxes' largest value size.
const HS_BOX_VALUE: f32 = 34.0;
/// The fire dragons' size and their centres' distance from the disc's.
const HS_DRAGON: (f32, f32) = (70.0, 140.0);
const HS_DRAGON_OFFSET: f32 = 330.0;

/// Through every Hold & Spin the cabinet's top screen shows a sun disc
/// reading "N COLLECTED / 15 WIN / GRAND JACKPOT", the count including the
/// coins held (it steps as each coin lands), with the four jackpots in
/// small boxes at its corners, Grand and Minor at the left, Major and Mini
/// at the right, and fire dragons either side (VERIFIED: 1bAevGz0K2k
/// 141.5 s, OL6Pu1AI-cQ 0.5 s and 130 s, Happy & Prosperous bUbSjAGwqs8
/// 2.5-3.0 s; Autumn Moon 5Z1-hFzpyh0 by the research notes). Our single
/// canvas has no top screen: as for the WINNER display, a plate over the
/// jackpot bar stands in for it (our layout, the owner's call: HYPOTHESIS),
/// with the game's own coin, its "GRAND JACKPOT" lettering (the bottom band
/// of its COLLECT 15 TO WIN art), each box cropped from its jackpot bar
/// with the live value, and its fire dragon. The wording is the cabinet's
/// in a substitute face. The official app shows instead an "N COLLECTED"
/// badge (VERIFIED for the app only).
fn spawn_hs_collected(c: &mut ChildSpawnerCommands, art: &ArtPack, fonts: &CabFonts) {
    let plate_left = 960.0 - JACKPOT_BAR_W / 2.0;
    let disc_left = JACKPOT_BAR_W / 2.0 - HS_DISC / 2.0;
    let ink = Color::srgb(0.12, 0.02, 0.02);
    let red = Color::srgba(0.85, 0.05, 0.02, 0.9);
    let heavy = |text: &str, size: f32, top: f32| {
        (
            Text::new(text),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.heavy.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(ink),
            TextShadow {
                offset: Vec2::new(1.5, 1.5),
                color: red,
            },
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(top),
                width: Val::Px(HS_DISC),
                ..default()
            },
        )
    };
    let bar = art.ui.get("jackpot_bar").cloned();
    let bar_size = art
        .sizes
        .get("jackpot_bar")
        .copied()
        .unwrap_or((1224.0, 104.0));
    c.spawn((
        HsCollectedPlate,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(plate_left),
            top: Val::Px(0.0),
            width: Val::Px(JACKPOT_BAR_W),
            height: Val::Px(WINNER_PLATE_H),
            border_radius: BorderRadius::bottom(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.05, 0.20)),
        GlobalZIndex(4),
        Visibility::Hidden,
    ))
    .with_children(|p| {
        // The fire dragons, facing the disc.
        for right in [false, true] {
            let side = if right { 1.0 } else { -1.0 };
            let cx = JACKPOT_BAR_W / 2.0 + side * HS_DRAGON_OFFSET;
            p.spawn((
                HsDragon,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(cx - HS_DRAGON.0 / 2.0),
                    top: Val::Px((WINNER_PLATE_H - HS_DRAGON.1) / 2.0),
                    width: Val::Px(HS_DRAGON.0),
                    height: Val::Px(HS_DRAGON.1),
                    ..default()
                },
                ImageNode {
                    flip_x: right,
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
        }
        // The four jackpots in boxes cropped from the bar, the live values
        // in their panels.
        let k = bar_size.0 / 1225.0;
        let scale = HS_BOX_H / bar_size.1.max(1.0);
        for (tier, right, bottom) in HS_CORNERS {
            let Some(&(_, (left, right_edge))) = JACKPOT_BOXES.iter().find(|(t, _)| *t == tier)
            else {
                continue;
            };
            let (x0, x1) = (
                (left - 7.0).max(0.0) * k,
                (right_edge + 7.0).min(1225.0) * k,
            );
            let w = (x1 - x0) * scale;
            let x = if right {
                JACKPOT_BAR_W - 24.0 - w
            } else {
                24.0
            };
            let y = if bottom {
                WINNER_PLATE_H - 4.0 - HS_BOX_H
            } else {
                4.0
            };
            if let Some(bar) = &bar {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x),
                        top: Val::Px(y),
                        width: Val::Px(w),
                        height: Val::Px(HS_BOX_H),
                        ..default()
                    },
                    ImageNode {
                        image: bar.clone(),
                        rect: Some(Rect::new(x0, 0.0, x1, bar_size.1)),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            }
            // The value in the box's black under its title (the bar art's
            // y 40-95 of 104).
            let width = (right_edge - left) * k * scale;
            p.spawn((
                JackpotMeter { tier },
                JackpotFit {
                    width,
                    max: HS_BOX_VALUE,
                },
                label(
                    "-",
                    &fonts.heavy,
                    HS_BOX_VALUE,
                    jackpot_value_colour(tier),
                    x + w / 2.0,
                    y + HS_BOX_H * 67.0 / 104.0,
                    width,
                ),
                TextShadow {
                    offset: Vec2::new(1.5, 1.5),
                    color: Color::srgba(0.03, 0.03, 0.05, 0.9),
                },
            ));
        }
        // The disc: the game's coin with the count, COLLECTED, 15 WIN and
        // the GRAND JACKPOT lettering.
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(disc_left),
                top: Val::Px(0.0),
                width: Val::Px(HS_DISC),
                height: Val::Px(HS_DISC),
                ..default()
            },
            ImageNode {
                image: art
                    .symbols
                    .get(&art.roles.coin)
                    .cloned()
                    .unwrap_or_default(),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_children(|d| {
            d.spawn((HsCollectedCount, heavy("", 52.0, 2.0)));
            d.spawn(heavy("COLLECTED", 16.0, 62.0));
            d.spawn(heavy("15 WIN", 12.0, 82.0));
            if let Some(lettering) = art.ui.get("collect_grand") {
                let (lw, lh) = art
                    .sizes
                    .get("collect_grand")
                    .copied()
                    .unwrap_or((510.0, 255.0));
                let band = Rect::new(0.0, lh * 200.0 / 255.0, lw, lh);
                let bw = HS_DISC * 0.72;
                let bh = bw * band.height() / band.width().max(1.0);
                d.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px((HS_DISC - bw) / 2.0),
                        top: Val::Px(99.0),
                        width: Val::Px(bw),
                        height: Val::Px(bh),
                        ..default()
                    },
                    ImageNode {
                        image: lettering.clone(),
                        rect: Some(band),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            }
        });
    });
}

impl FeatureShow {
    /// Coins presented but still to land on the screen: a respin's coins
    /// not yet at their stops (the board's labels already carry them).
    fn coins_to_land(&self) -> Vec<(u8, u8)> {
        if let Some(landed) = &self.respin_landed {
            return landed.clone();
        }
        self.respin
            .as_ref()
            .map(|a| {
                a.stops
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| s.3 && *i >= a.done)
                    .map(|(_, s)| (s.0, s.1))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The collected-coin disc shows: the Hold & Spin board is up, until
    /// the WINNER display takes the top over.
    pub fn collected_showing(&self) -> bool {
        self.grid && self.hold_and_spin() && !self.winner_showing()
    }
}

/// The disc's count: the coins on the board (`coins`, held and landed),
/// less those still to land.
fn coins_collected(coins: &[(u8, u8)], to_land: &[(u8, u8)]) -> usize {
    coins.iter().filter(|c| !to_land.contains(c)).count()
}

/// Show the collected-coin disc through Hold & Spin, its count stepping as
/// each coin lands, its dragons' fire running.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn hs_collected(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    art: Res<ArtPack>,
    labels: Query<(&CellLabel, &Text), Without<HsCollectedCount>>,
    mut plate: Query<&mut Visibility, With<HsCollectedPlate>>,
    mut count: Query<&mut Text, (With<HsCollectedCount>, Without<CellLabel>)>,
    mut dragons: Query<&mut ImageNode, With<HsDragon>>,
) {
    if !fidelity.0 {
        return;
    }
    let on = show.collected_showing() && !gamble.active;
    for mut v in &mut plate {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    if !on {
        return;
    }
    let coins: Vec<(u8, u8)> = labels
        .iter()
        .filter(|(_, t)| t.0.starts_with("ORB\n"))
        .map(|(l, _)| (l.reel, l.row))
        .collect();
    let n = coins_collected(&coins, &show.coins_to_land()).to_string();
    for mut t in &mut count {
        if t.0 != n {
            t.0.clone_from(&n);
        }
    }
    if let Some(frames) = art.anims.get("dragon") {
        let i = (time.elapsed_secs() * ANIM_FPS) as usize % frames.len().max(1);
        if let Some(h) = frames.get(i) {
            for mut image in &mut dragons {
                if image.image != *h {
                    image.image = h.clone();
                }
            }
        }
    }
}

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
/// The line of the live Grand and Major under the WINNER box.
#[derive(Component)]
pub struct WinnerJackpots;

/// The line under the WINNER box, from the Grand's and Major's values as
/// the meters show them.
pub fn winner_jackpots_line(grand: &str, major: &str) -> String {
    format!("GRAND JACKPOT: {grand}   MAJOR JACKPOT: {major}")
}

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
        let want = if show.winner_showing() && !gamble.active {
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
    // the screen through every Hold & Spin's collection and count (the
    // cabinet shows it on its top screen): a navy plate over the whole bar,
    // "WINNER" over the flame-bordered total, the live Grand and Major under
    // it. Under the WIN banner (5), which counts over its foot.
    c.spawn((
        WinnerPlate,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(960.0 - JACKPOT_BAR_W / 2.0),
            top: Val::Px(0.0),
            width: Val::Px(JACKPOT_BAR_W),
            height: Val::Px(WINNER_PLATE_H),
            border_radius: BorderRadius::bottom(Val::Px(14.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.05, 0.20)),
        GlobalZIndex(4),
        Visibility::Hidden,
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("WINNER"),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(26.0),
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.32, 0.12)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(-2.0),
                ..default()
            },
        ));
        p.spawn((
            WinnerJackpots,
            Text::new(""),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(WINNER_LINE_SIZE),
                ..default()
            },
            TextColor(WINNER_GREEN),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(WINNER_LINE_TOP),
                width: Val::Percent(100.0),
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
                font_size: FontSize::Px(WINNER_AMOUNT_SIZE),
                ..default()
            },
            TextColor(WINNER_GREEN),
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
            || self.fg_trigger.is_some_and(|t| t < self.fg_count_at())
            || self
                .retrigger_ends_at()
                .is_some_and(|end| self.fg_trigger.is_some_and(|t| t < end))
            || self.fg_cut.is_some_and(|t| t < FG_CUT)
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
        if crossed(t0, t, INTRO_TRANSITION) {
            show.transition_at = Some((clock, FIRE_FPS));
        }
        if crossed(t0, t, INTRO_GRID) {
            show.grid = true;
        }
        if t >= INTRO_END {
            show.intro = None;
            show.wait_for(Await::Start(Feature::HoldAndSpin));
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
    for _ in 0..show.advance_count_up(dt) {
        sounds.play(&mut commands, "SND_Fea_Inc");
    }

    // A respin was presented: every open position spins and stops in turn.
    if let Some(landed) = show.respin_landed.take() {
        show.counter = show.counter.saturating_sub(1);
        // Fourteen coins held: a drum roll from the respin's start, ended at
        // its last stop (the mobile engine starts it on a respin with
        // fourteen held, STRONG; its opening matched on a Happy & Prosperous
        // cabinet, VERIFIED there).
        let held = (0..reels)
            .flat_map(|reel| (0..rows).map(move |row| (reel, row)))
            .filter(|&(reel, row)| {
                !landed.contains(&(reel, row)) && !open(&label_of(&labels, reel, row))
            })
            .count();
        let drum = held + 1 == usize::from(reels) * usize::from(rows);
        if drum {
            show.drum = sounds.play(&mut commands, "SND_DrumRoll_extended");
        }
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
        let last = stops.iter().map(|s| s.2).fold(0.0, f32::max);
        let mut anim = RespinAnim {
            stops,
            base: show.respins_run * 1009,
            ..default()
        };
        if drum {
            anim.sounds.push((last + RESPIN_CLINK, "SND_DrumRoll_term"));
        }
        show.respin = Some(anim);
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
            if *name == "SND_DrumRoll_term"
                && let Some(e) = show.drum.take()
            {
                commands.entity(e).try_despawn();
            }
            sounds.play(&mut commands, name);
        }
        anim.sounds.retain(|s| s.0 > now);
        if anim.resets.iter().any(|&r| r <= now) {
            anim.resets.retain(|&r| r > now);
            // The counter's burst plays on every return to 3 (club cabinets:
            // Panda Magic 8V086h-IJ6I four resets and nYM0NGR3px0 five,
            // Golden Century y8c8lEdPW8Y 973.57, 977.43, 979.70; VERIFIED).
            if show.pending_left > show.counter {
                show.banner_burst = Some(clock);
            }
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
        if show.pending_left > show.counter {
            show.banner_burst = Some(show.clock);
        }
        show.counter = show.pending_left;
        // The grid full: the Grand's bell about 3.5 s after the fifteenth
        // coin lands (a Happy & Prosperous cabinet, VERIFIED there; the clip
        // is shared by the family).
        let full = (0..reels)
            .flat_map(|reel| (0..rows).map(move |row| (reel, row)))
            .all(|(reel, row)| !open(&label_of(&labels, reel, row)));
        if full && show.grand_bell_at.is_none() {
            show.grand_bell_at = Some(clock + GRAND_BELL_AFTER);
        }
    }
    if let Some(at) = show.grand_bell_at
        && clock >= at
        && at > 0.0
    {
        sounds.play(&mut commands, "SND_GrandBell");
        show.grand_bell_at = Some(0.0);
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
        let mut coins = majors_first(coins);
        // A full grid's Grand jackpot is added after the coins.
        let grand = jackpots
            .iter()
            .position(|(tier, _)| *tier == JackpotTier::Grand)
            .map(|i| jackpots.remove(i).1);
        if let Some(g) = grand {
            coins.push((GRAND_STEP, GRAND_STEP, g, true, Some(JackpotTier::Grand)));
        }
        show.grand = grand;
        show.begin_collect(coins, rollup.to);
    }
    let step = show.advance_collect(dt, total_bet);
    if step.rumble {
        show.rumble = sounds.play(&mut commands, "SND_Rumble");
    }
    for name in step.sounds {
        sounds.play(&mut commands, name);
    }
    // A Major's own count: the payout music and a coin shower for a big
    // one (or the long win tune), WIN counting it up over the music's
    // length; SPIN completes it (the cabinet held the plaque about 20 s).
    if let Some(amount) = step.major {
        let name = if amount >= PAY_SHOWER_RATIO * total_bet {
            let n = PAYOUT_MUSIC[show.payout_track % PAYOUT_MUSIC.len()];
            show.payout_track += 1;
            n
        } else {
            PAY_TUNE
        };
        show.pay_sound = sounds.play(&mut commands, name);
        show.major_pay = Some(Pay {
            amount,
            t: 0.0,
            seconds: sounds.seconds(name).unwrap_or(20.0).max(1.0),
        });
    }
    let mut done = step.done;
    if done.is_some()
        && let Some(id) = show.rumble.take()
    {
        commands.entity(id).try_despawn();
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
        // The WIN banner's letters burst as it appears (club cabinet).
        show.flares_at = Some(clock);
        show.pay = Some(Pay {
            amount: total,
            t: 0.0,
            seconds: sounds.seconds(name).unwrap_or(60.0).max(1.0),
        });
        done = None;
    }
    let unattended = |p: &Option<Pay>| {
        smoke.frames_left.is_some() && p.as_ref().is_some_and(|p| p.t > 3.0 && p.t < p.seconds)
    };
    let slam = input.spin
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || unattended(&show.pay)
        || unattended(&show.major_pay);
    if (show.pay.is_some() || show.major_pay.is_some()) && slam {
        input.skip_spin = true;
    }
    if let Some(mut p) = show.major_pay.take() {
        p.t += dt;
        if p.t < p.seconds && slam {
            p.t = p.seconds;
        }
        if p.t >= p.seconds
            && let Some(id) = show.pay_sound.take()
        {
            commands.entity(id).try_despawn();
        }
        if p.t >= p.seconds + PAY_AFTER {
            show.end_major(p.amount);
        } else {
            show.major_pay = Some(p);
        }
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
        show.hold_winner(total);
    }
}

impl FeatureShow {
    /// Run the counter's count-up on by `dt` seconds (at `self.clock`): its
    /// steps (each a tick), the burst on its last, and its end `COUNT_HOLD`
    /// after that. Returns the steps taken.
    fn advance_count_up(&mut self, dt: f32) -> u32 {
        let Some(t0) = self.count_up else {
            return 0;
        };
        let t = t0 + dt;
        self.count_up = Some(t);
        let target = self.pending_left;
        let step_at = |n: u8| COUNT_FIRST + f32::from(n) * COUNT_STEP;
        let mut steps = 0;
        while self.counter < target && t >= step_at(self.counter) {
            self.counter += 1;
            steps += 1;
            if self.counter == target {
                self.banner_burst = Some(self.clock);
            }
        }
        if self.counter >= target && t >= step_at(target.saturating_sub(1)) + COUNT_HOLD {
            self.count_up = None;
        }
        steps
    }

    /// The count's total has been paid (`PAY_AFTER` after the count was
    /// complete): the WINNER box stays until `WINNER_HOLD` after the count
    /// was complete.
    fn hold_winner(&mut self, total: u64) {
        let until = self.clock + (WINNER_HOLD - PAY_AFTER).max(0.0);
        self.winner_hold = self.winner.take().map(|w| (until, w));
        self.collected = Some(total);
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
    // 0.2 s after it), through the fire and on the board.
    let hs = show.hold_and_spin() && (show.grid || show.since_burst(0.2));
    for (r, mut image, mut vis) in &mut rings {
        let held = hs
            && !show.big_covers(r.reel)
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
            .and_then(|(s, fps)| frame("transition", (clock - s) * fps / ANIM_FPS, false))
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

    // The collection bolt and the splash it makes on the WINNER box. Its
    // comet reaches the box as the value lands (`bolt_frame`).
    for (mut image, mut vis, mut node) in &mut bolt {
        let f = show.bolt.and_then(|(reel, row, s)| {
            let (k, flip, left, top, w, h) = bolt_for(reel, row);
            let frames = art.anims.get(&format!("bolt{k}"))?;
            let f = frames.get(bolt_frame(k, clock - s))?.clone();
            Some((f, flip, left, top, w, h))
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

    // The WINNER total in the pack's flame-bordered box (its flames looped);
    // on a Grand, the GRAND JACKPOT plaque (its own 729 x 108 shape, the
    // title across its top, the amount below), on a Major the MAJOR JACKPOT
    // plaque, its amount green (club cabinet: Autumn Moon 5Z1-hFzpyh0
    // 530-555 s; a pack without the plaque keeps the flame box). The gamble
    // shows the jackpots, not the box.
    let grand = show.grand_showing();
    let major = !grand && show.major_showing();
    let plaque = if grand {
        art.ui
            .get("grand_plaque")
            .map(|h| (h.clone(), "grand_plaque"))
    } else if major {
        art.ui
            .get("major_plaque")
            .map(|h| (h.clone(), "major_plaque"))
    } else {
        None
    };
    let boxed = show.winner_showing() && !gamble.active;
    let (w, h) = WINNER_BOX;
    let plaque_w = plaque.as_ref().map(|(_, key)| {
        let (pw, ph) = art.sizes.get(*key).copied().unwrap_or((729.0, 108.0));
        h * pw / ph.max(1.0)
    });
    for (mut image, mut node, mut vis) in &mut winner {
        let key = match &plaque {
            Some((handle, _)) => Some(handle.clone()),
            None => frame("flameborder_sm", clock, true),
        };
        let w = plaque_w.unwrap_or(w);
        if node.width != Val::Px(w) {
            node.width = Val::Px(w);
            node.left = Val::Px(960.0 - w / 2.0);
        }
        match key.filter(|_| boxed) {
            Some(h) => {
                if image.image != h {
                    image.image = h;
                }
                set(&mut vis, true);
            }
            None => set(&mut vis, false),
        }
    }
    let amount = if grand {
        show.grand
    } else if major {
        show.major_amount()
    } else {
        show.winner_amount()
    }
    .filter(|_| boxed)
    .map(|w| money(w, denom.cents()))
    .unwrap_or_default();
    for (mut t, mut color, mut font, mut node) in &mut winner_text {
        if t.0 != amount {
            t.0 = amount.clone();
        }
        // On a plaque the amount sits in the band under its title.
        let (size, drop) = if plaque.is_some() {
            (WINNER_BOX.1 * 0.5, WINNER_BOX.1 * 0.17)
        } else {
            (WINNER_AMOUNT_SIZE, 0.0)
        };
        // (The plaque's size equals the box's, so the drop is set apart.)
        if font.font_size != FontSize::Px(size) {
            font.font_size = FontSize::Px(size);
        }
        if node.top != Val::Px(drop) {
            node.top = Val::Px(drop);
        }
        let want = if grand {
            Color::srgb(0.95, 0.12, 0.1)
        } else if major {
            MAJOR_AMOUNT_GREEN
        } else {
            WINNER_GREEN
        };
        if color.0 != want {
            color.0 = want;
        }
    }
}

/// A Major's amount on its plaque: green (club cabinet, Autumn Moon
/// 5Z1-hFzpyh0 540-555 s; the tint from a 160 x 90 storyboard, PARTIAL).
const MAJOR_AMOUNT_GREEN: Color = Color::srgb(0.30, 0.95, 0.35);

// ---------------------------------------------------------------------------
// Free games: the doors reveal and the win banner
// ---------------------------------------------------------------------------
//
// Club cabinets (motion per door cell at 30 fps, 4 fine and 6 coarse reveals
// on 1bAevGz0K2k and y8c8lEdPW8Y): the doors land with the reels; about
// 0.30 s after the last reel is still every door swings open together on the
// symbol they reveal, to SND_EndMask (the sound placed from the app), their
// motion visible for 0.60-0.73 s and over by the rest + 0.93-0.97 s (STRONG);
// the game's wins show at the rest + 1.5 s (PARTIAL: 3 cases, one machine);
// with no win the next free game spins at the rest + 2.0 s (STRONG: 4 cases,
// two machines). The app follows the cabinet. The official app, recorded with
// sound (all six free games of a feature), opens them 3 frames after the
// rest, the 42-frame sequence at 30 fps (1.4 s), and shows the wins 47
// frames after they begin to open (VERIFIED for the mobile app only).

/// Seconds after the reveal is presented that the doors open (it is
/// presented when they open), their frame rate (the game's own 42-frame
/// sequence then lasts 1.0 s; its motion, frames 1-29, 0.69 s, over by the
/// rest + 0.99 s as on the cabinet; frames 30-41 a faint fading remnant), and
/// the seconds from their opening to the game's wins (the rest + 1.5 s).
const REVEAL_OPEN: f32 = 0.0;
const REVEAL_FPS: f32 = 42.0;
const REVEAL_WINS: f32 = 1.2;
/// The doors' reveal is presented this long after the last reel is still
/// (club cabinets: 0.22-0.37 s, three of four reveals about 0.30 s).
pub const REVEAL_AFTER_REST: f32 = 0.30;
/// A wild filling its reel (Golden Gong's monk) is presented as the last
/// reel is still: its sound and its sequence start then (club cabinets,
/// twelve expansions; VERIFIED).
pub const EXPAND_AFTER_REST: f32 = 0.0;

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

/// The WIN banner's amount: white with a faint cool tint, between its gold
/// WIN words, for Hold & Spin's count and a free game's win alike (club
/// cabinets: the digits' brightest pixels as bright and as neutral as white
/// text on the same screen, the WIN words gold, the WINNER total yellow in
/// the same frames; f5sMl9a5jds 278.6, 281.1, 282.6, 795.8; y8c8lEdPW8Y
/// 1018.7, 1019.5, 1029.8; 1bAevGz0K2k 164.3, 165.5; VERIFIED). The official
/// app's is yellow.
const BANNER_AMOUNT_WHITE: Color = Color::srgb(0.93, 0.94, 1.0);

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
        TextColor(BANNER_AMOUNT_WHITE),
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
    /// A denomination was chosen and no game has started since: the club
    /// meters alternate dollars and credits each second meanwhile (club
    /// footage: y8c8lEdPW8Y 11.77-15.30 and 171.77-173.73, CAM8vranI9Y,
    /// 1bAevGz0K2k, etaH-vbYBKY; VERIFIED). Set on every choice, the
    /// current denomination's included (untested on the cabinet).
    pub selecting: bool,
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

    /// Seconds since the chooser opened.
    pub fn since(&self) -> f32 {
        self.since
    }

    /// The splash still shows after a choice.
    pub fn closing(&self) -> bool {
        !self.open && self.closing > 0.0
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

/// The chooser's layout on the canvas, by homography from the reel frame on
/// two club recordings of one Queensland venue (1bAevGz0K2k 0:13.0,
/// y8c8lEdPW8Y 0:09.0; agreeing within about 30 px; STRONG): the panel about
/// 980-1010 x 190-200 with its centre near y 755 (the width for five keys;
/// six widen it by their own share), opaque black-brown in a thin gold frame
/// with small corners.
const CHOOSER_PANEL: (f32, f32, f32) = (995.0, 195.0, 657.0);
/// The header's capitals: their centre line (cabinet about 700-707) and the
/// type size that gives them a cap of about 34 px (the cabinet's about 36:
/// "CHOOSE YOUR DENOMINATION" about 751 px wide, "TOUCH SCREEN TO MAKE
/// SELECTION" about 913) in our bold sans, a substitute for its decorative
/// cream serif.
const CHOOSER_HEADER: (f32, f32) = (703.0, 48.0);
const CHOOSER_HEADER_COLOUR: Color = Color::srgb(0.90, 0.80, 0.64);
/// The denomination keys: width, height, gap and top (cabinet about 157-162
/// x 80-81, 25-29 apart, y 738-820).
const CHOOSER_KEY: (f32, f32, f32, f32) = (162.0, 80.0, 26.0, 739.0);
/// The panel's margin either side of its row of keys (995 against a row of
/// five, 914).
const CHOOSER_MARGIN: f32 = (CHOOSER_PANEL.0 - (5.0 * CHOOSER_KEY.0 + 4.0 * CHOOSER_KEY.2)) / 2.0;
/// ARE YOU SURE?: width, height and the gap left above the panel (cabinet:
/// wholly above it, about 10 px clear; 1bAevGz0K2k 0:14.4, y8c8lEdPW8Y
/// 0:11.30; about 325 x 180 from one weak homography, PARTIAL).
const CHOOSER_CONFIRM: (f32, f32, f32) = (325.0, 180.0, 10.0);
/// Its YES and NO keys: width, height, gap (about 128 x 49, 23 apart;
/// PARTIAL).
const CHOOSER_ANSWER: (f32, f32, f32) = (128.0, 48.0, 24.0);
/// Club: the chooser keeps COLLECT, moved to the screen's bottom right
/// corner, the denomination box's place (y8c8lEdPW8Y 0:08-0:11; STRONG for
/// this layout, one machine; a 2018 layout, etaH-vbYBKY, keeps it at the
/// left).
const CHOOSER_COLLECT: (f32, f32) = (1830.0, 1016.0);
/// The drawn title standing in for the cabinet's logo: the centre of its
/// capitals (cabinet logo y about 485-640, directly above the panel), its
/// largest size and letter spacing, and the widest it may run (cabinet logo
/// about 1560 px wide; 1bAevGz0K2k 0:13.0, y8c8lEdPW8Y 0:09.0; STRONG).
const CHOOSER_TITLE: (f32, f32, f32, f32) = (562.0, 195.0, 20.0, 1600.0);
/// The meters' face's (Impact's) mean advance over capitals and spaces, in
/// ems (the title "GOLDEN CENTURY" at 150 px measured 1000 px on a capture).
const IMPACT_CAPS_ADVANCE: f32 = 0.476;

/// The chooser title's type size: the largest up to the cabinet logo's
/// width, smaller for a longer name.
fn chooser_title_size(name: &str) -> f32 {
    let (_, max, gap, widest) = CHOOSER_TITLE;
    let n = name.chars().count().max(1) as f32;
    max.min((widest - n * gap) / (n * IMPACT_CAPS_ADVANCE))
}

/// A chooser key's colours: its face, the lighter inner face of its bevel
/// and its rim (cabinet camera colour, PARTIAL: cent keys red-orange about
/// (247, 85, 44) with a darker red-brown rim, the dollar keys amber).
#[derive(Clone, Copy)]
enum ChooserFace {
    Cents,
    Dollars,
}

impl ChooserFace {
    /// Its colours in a title's look: the title's own cent keys where it
    /// has them (crate::looks), Golden Century's otherwise.
    fn colours(self, look: &crate::looks::TitleLook) -> (Color, Color, Color, Color) {
        match self {
            ChooserFace::Cents if look.cents_key.is_some() => {
                crate::looks::key_shades(look.cents_key.unwrap_or_default())
            }
            ChooserFace::Cents => (
                Color::srgb_u8(245, 95, 45),
                Color::srgb_u8(252, 140, 92),
                Color::srgb_u8(247, 104, 54),
                Color::srgb_u8(120, 35, 15),
            ),
            ChooserFace::Dollars if look.dollar_key.is_some() => {
                crate::looks::key_shades(look.dollar_key.unwrap_or_default())
            }
            ChooserFace::Dollars => (
                Color::srgb_u8(246, 205, 75),
                Color::srgb_u8(252, 230, 140),
                Color::srgb_u8(247, 210, 88),
                Color::srgb_u8(150, 95, 20),
            ),
        }
    }
}

/// The chooser keys' figures: white with a dark navy outline all round
/// (eight copies 2 px off each way, as the status texts' edge).
const KEY_FIGURE: Color = Color::srgb(0.97, 0.97, 1.0);
const KEY_OUTLINE: Color = Color::srgb(0.05, 0.07, 0.22);
const KEY_OUTLINE_OFFSETS: [(f32, f32); 8] = [
    (-2.0, -2.0),
    (0.0, -2.0),
    (2.0, -2.0),
    (-2.0, 0.0),
    (2.0, 0.0),
    (-2.0, 2.0),
    (0.0, 2.0),
    (2.0, 2.0),
];
/// The cent sign's size as a share of the digits' (the cabinet's is visibly
/// smaller than the figures).
const CENT_SIGN: f32 = 0.7;

/// A denomination as the chooser's keys write it: the figures, and a cent
/// sign drawn smaller after them.
fn chooser_figure(cents: u64) -> (String, Option<&'static str>) {
    if cents >= 100 {
        (format!("${}", cents / 100), None)
    } else {
        (cents.to_string(), Some("¢"))
    }
}

/// A bevelled chooser key, `w` x `h`: a dark rim, the face, and a lighter
/// inner face shading to it.
fn chooser_key<'a>(
    row: &'a mut ChildSpawnerCommands,
    w: f32,
    h: f32,
    face: ChooserFace,
    look: &crate::looks::TitleLook,
) -> EntityCommands<'a> {
    let (base, light, low, rim) = face.colours(look);
    let mut key = row.spawn((
        Node {
            width: Val::Px(w),
            height: Val::Px(h),
            border: UiRect::all(Val::Px(4.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(base),
        BorderColor::all(rim),
    ));
    key.with_children(|k| {
        k.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(4.0),
                right: Val::Px(4.0),
                top: Val::Px(3.0),
                bottom: Val::Px(5.0),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            bevy::ui::BackgroundGradient(vec![bevy::ui::Gradient::Linear(
                bevy::ui::LinearGradient::to_bottom(vec![
                    bevy::ui::ColorStop::new(light, Val::Percent(0.0)),
                    bevy::ui::ColorStop::new(low, Val::Percent(100.0)),
                ]),
            )]),
        ));
    });
    key
}

/// A key's figures, white in a navy outline, with an optional smaller tail
/// (the cent sign) on the same baseline.
fn spawn_key_figure(
    k: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    size: f32,
    (main, tail): (String, Option<&'static str>),
) {
    let face = |px: f32| TextFont {
        font: bevy::text::FontSource::Handle(font.clone()),
        font_size: FontSize::Px(px),
        ..default()
    };
    k.spawn(Node::default()).with_children(|w| {
        let mut one = |at: Option<(f32, f32)>, colour: Color| {
            let node = match at {
                Some((dx, dy)) => Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(dx),
                    top: Val::Px(dy),
                    ..default()
                },
                None => Node::default(),
            };
            w.spawn((
                Text::new(main.clone()),
                face(size),
                TextColor(colour),
                TextLayout::no_wrap(),
                node,
            ))
            .with_children(|t| {
                if let Some(tail) = tail {
                    t.spawn((
                        TextSpan::new(tail),
                        face(size * CENT_SIGN),
                        TextColor(colour),
                    ));
                }
            });
        };
        for at in KEY_OUTLINE_OFFSETS {
            one(Some(at), KEY_OUTLINE);
        }
        one(None, KEY_FIGURE);
    });
}

/// Build the GAMES MENU (hidden until touched), with `title` drawn on its
/// splash.
fn spawn_games_menu(
    c: &mut ChildSpawnerCommands,
    art: &ArtPack,
    fonts: &CabFonts,
    profile: Profile,
    title: &str,
    look: &crate::looks::TitleLook,
) {
    let header = look
        .chooser_header
        .map_or(CHOOSER_HEADER_COLOUR, crate::looks::rgb);
    let top = JACKPOT_BAR_TOP + JACKPOT_BAR_W * 104.0 / 1225.0 + 4.0;
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
        // The title (drawn: the cabinet's logo is not in the mobile build),
        // its capitals where the logo stands, right over the panel.
        let (title_y, _, gap, _) = CHOOSER_TITLE;
        let size = chooser_title_size(title);
        let (caps_top, caps_h) = IMPACT_CAPS;
        m.spawn((
            Text::new(title),
            TextFont {
                font: bevy::text::FontSource::Handle(fonts.meter.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.82, 0.25)),
            TextLayout::justify(Justify::Center),
            LetterSpacing::Px(gap),
            TextShadow {
                offset: Vec2::new(4.0, 6.0),
                color: Color::srgba(0.35, 0.05, 0.0, 0.9),
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(title_y - (caps_top + caps_h / 2.0) * size - top),
                width: Val::Percent(100.0),
                ..default()
            },
        ));
        // The panel: header over the row of denominations, each placed on
        // the canvas (a frame drawn under them, so no border shifts them).
        let denoms = profile.denoms();
        let (kw, kh, kgap, ktop) = CHOOSER_KEY;
        let n = denoms.len() as f32;
        let row_w = n * kw + (n - 1.0).max(0.0) * kgap;
        let (pw0, ph, ptop) = CHOOSER_PANEL;
        let pw = pw0.max(row_w + 2.0 * CHOOSER_MARGIN);
        m.spawn((
            ChooserPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((CANVAS_W - pw) / 2.0),
                top: Val::Px(ptop - top),
                width: Val::Px(pw),
                height: Val::Px(ph),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.08, 0.08)),
                BorderColor::all(Color::srgb(0.9, 0.68, 0.22)),
            ));
            // Arial Bold's capitals are centred 0.59 of the size below its
            // line's top.
            let (header_y, header_size) = CHOOSER_HEADER;
            p.spawn((
                ChooserHeader,
                Text::new("CHOOSE YOUR DENOMINATION"),
                TextFont {
                    font: bevy::text::FontSource::Handle(fonts.sans.clone()),
                    font_size: FontSize::Px(header_size),
                    ..default()
                },
                TextColor(header),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(header_y - 0.59 * header_size - ptop),
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
            p.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(ktop - ptop),
                width: Val::Percent(100.0),
                height: Val::Px(kh),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(kgap),
                ..default()
            })
            .with_children(|row| {
                for &cents in denoms {
                    let face = if cents >= 100 {
                        ChooserFace::Dollars
                    } else {
                        ChooserFace::Cents
                    };
                    chooser_key(row, kw, kh, face, look)
                        .with_children(|b| {
                            spawn_key_figure(b, &fonts.meter, 56.0, chooser_figure(cents));
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
        // Club: "<denomination> GAME / ARE YOU SURE?", wholly above the panel.
        let (cw, ch, clear) = CHOOSER_CONFIRM;
        m.spawn((
            ChooserConfirm,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((CANVAS_W - cw) / 2.0),
                top: Val::Px(ptop - clear - ch - top),
                width: Val::Px(cw),
                height: Val::Px(ch),
                border: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.02, 0.02)),
            BorderColor::all(Color::srgb(0.9, 0.68, 0.22)),
            GlobalZIndex(13),
            Visibility::Hidden,
        ))
        .with_children(|q| {
            let face = |px: f32| TextFont {
                font: bevy::text::FontSource::Handle(fonts.sans.clone()),
                font_size: FontSize::Px(px),
                ..default()
            };
            // "<denomination> GAME" in bold yellow, "ARE YOU SURE?" in the
            // header's cream.
            q.spawn((
                ChooserQuestion,
                Text::new(""),
                face(30.0),
                TextColor(Color::srgb(1.0, 0.86, 0.2)),
                TextLayout::justify(Justify::Center),
            ))
            .with_children(|t| {
                t.spawn((
                    TextSpan::new("ARE YOU SURE?"),
                    face(30.0),
                    TextColor(CHOOSER_HEADER_COLOUR),
                ));
            });
            let (aw, ah, agap) = CHOOSER_ANSWER;
            q.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(agap),
                ..default()
            })
            .with_children(|row| {
                for yes in [true, false] {
                    let word = if yes { "YES" } else { "NO" };
                    chooser_key(row, aw, ah, ChooserFace::Cents, look)
                        .with_children(|b| {
                            spawn_key_figure(b, &fonts.sans, 30.0, (word.to_string(), None));
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
        // Club: COLLECT stays, moved to the corner (inert, as on the band).
        if profile == Profile::Club {
            let (x, y) = CHOOSER_COLLECT;
            pill(
                m,
                fonts,
                "COLLECT",
                x,
                y - top,
                176.0,
                42.0,
                32.0,
                TabLook::Club,
            );
        }
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
    mut tabs: Query<
        &mut Visibility,
        (
            With<BandTab>,
            Without<ChooserRoot>,
            Without<ChooserCredit>,
            Without<ChooserPanel>,
            Without<ChooserConfirm>,
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
    // The band's raised tabs give way to the splash: the chooser shows no
    // GAMES MENU, and its COLLECT stands in the corner (spawn_games_menu).
    for mut v in &mut tabs {
        if *v != vis(!showing) {
            *v = vis(!showing);
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
        // "ARE YOU SURE?" is the question's own second span.
        let q = format!("{} GAME\n", denom_text(cents));
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

    /// The choice whose screen button shows filled: from the frame its card
    /// turns face up until the card goes (club cabinets: the fill comes with
    /// the face, once a frame early; _XHUzg4qDXI 0:07.87 and 1:16.800,
    /// iHDxUc1dlKs 0:12.433; STRONG), not at the press.
    pub fn picked(&self) -> Option<GamblePick> {
        self.card
            .filter(|(_, _, t)| *t >= GAMBLE_FACE_DELAY)
            .and(self.last.map(|(p, _)| p))
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
        // The prompt takes the status line's place and style; on the club
        // machines the bet line's yellow, spaced wider (see
        // `GAMBLE_PROMPT_CLUB`).
        let prompt = spawn_edged_text(
            g,
            "",
            fonts,
            profile,
            Justify::Center,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(STATUS_X - 600.0),
                top: Val::Px(status_y(profile) - top - 14.0),
                width: Val::Px(1200.0),
                ..default()
            },
            None,
            GambleText::Prompt,
        );
        if profile == Profile::Club {
            let (colour, gap) = GAMBLE_PROMPT_CLUB;
            g.commands()
                .entity(prompt)
                .insert((TextColor(colour), LetterSpacing::Px(gap)));
        }
    });
}

/// The club gamble prompt's colour and letter spacing: yellow like the bet
/// line and spaced so "Select a SUIT, RED/BLACK or TAKE WIN" spans about 730
/// canvas px (club cabinets: a steady yellow, never white, on _XHUzg4qDXI
/// 1:11.3-1:15.4 spanning x 589-1320, and on LwC3hVqFRMg 18:30-18:33;
/// VERIFIED per machine). An NSW machine with a video deck (iHDxUc1dlKs
/// 0:12.43) and the owner's Happy & Prosperous (00:13.53) show it white: a
/// conflict, recorded in GOLDEN_CENTURY_CABINET_DECK.md.
const GAMBLE_PROMPT_CLUB: (Color, f32) =
    (Color::srgb(225.0 / 255.0, 225.0 / 255.0, 80.0 / 255.0), 6.0);

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
    let chosen = gamble.picked();
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
// each gap between the reels, at the line's rows; it blinks with its winning
// symbols in turn (each lit about a third of a second), each winning line
// for two blinks in turn, and the message at the right reads "Line 1 wins 2".

/// Seconds each winning line shows, each line in turn, round and round
/// until the next game: two even cycles of about 0.66 s, each starting lit.
/// The line's badges and dashes are lit exactly while its winning symbols
/// show; the symbols that do not animate blink out exactly while the badges
/// are off, with no gap and no overlap (club cabinets, frame by frame:
/// y8c8lEdPW8Y 21.93-24.23 s, cycles of 19-20 frames; CAM8vranI9Y, a
/// multi-line win, its message moving to the next line every 1.333 s;
/// STRONG). The official app's lines are uneven, 43-45 frames each with
/// gaps (VERIFIED for the mobile app only).
const WIN_LINE_SECONDS: f32 = 1.32;
const WIN_LINE_LIT: [(f32, f32); 2] = [(0.0, 0.34), (0.66, 0.99)];
const WIN_LINE_BLINK: [(f32, f32); 2] = [(0.34, 0.66), (0.99, 1.32)];

/// Whether `phase` (seconds into a line's showing) falls in one of `spans`.
fn in_spans(phase: f32, spans: &[(f32, f32)]) -> bool {
    spans.iter().any(|&(a, b)| phase >= a && phase < b)
}
/// How a winning line is drawn in a line configuration (club footage,
/// rectified onto our canvas: CAM8vranI9Y, y8c8lEdPW8Y, f5sMl9a5jds). Node
/// sizes allow for the art's shaded rim (visible about 0.95 x 0.83 of the
/// badge node, 0.88 x 0.78 of the dash's).
#[derive(Clone, Copy, PartialEq, Debug)]
struct LineLook {
    /// The badge node: at 50 lines visible about 37 x 22 (three machines);
    /// at 25 lines about 97 x 43 (two), and at 5 lines about 100 x 44 (one).
    badge: (f32, f32),
    /// The number's font size (digits about 14 px tall at 50 lines, 26 at
    /// 25 and 5).
    number: f32,
    /// The dash in each gap (visible about 32 x 11 at 50 lines).
    dash: (f32, f32),
    /// The stub from the badge across the frame into the reel.
    stub: (f32, f32),
    /// Lines 1 to 5 show the art's own digit. Where the badge is far wider
    /// than the art's proportions (25 and 5 lines) the digit would stretch,
    /// so every line is the blank badge with its number set on it, and the
    /// badge's rounded ends keep their shape (sliced).
    own_digits: bool,
}

/// The line configuration a denomination plays, which the line pieces
/// follow: 50 lines at 1c and 2c, 25 at 5c and 10c, 5 above. Club footage
/// at 10c with 20 lines selected shows the 25-line badge (one case:
/// HYPOTHESIS that the size follows the denomination, not the lines chosen).
fn line_config(cents: u64) -> u16 {
    lines_for(cents)
}

/// The line pieces' look in line configuration `config`.
fn line_look(config: u16) -> LineLook {
    if config >= 50 {
        LineLook {
            badge: (39.0, 27.0),
            number: 20.0,
            dash: (36.0, 14.0),
            stub: (24.0, 8.0),
            own_digits: true,
        }
    } else {
        LineLook {
            badge: (102.0, 52.0),
            number: 38.0,
            dash: (52.0, 26.0),
            stub: (34.0, 16.0),
            own_digits: false,
        }
    }
}

/// The badge's visible share of its node's width.
const LINE_BADGE_VISIBLE: f32 = 0.95;
/// The reel frame's outer gold on the canvas, left and right: every badge
/// sits outside it, touching it (footage: the left badge's right edge at
/// x 141-144, the right badge's left edge at x 1771-1775).
const LINE_FRAME_OUTER: (f32, f32) = (142.0, 1771.0);

/// The centre x of a badge `width` wide at the left or right end.
fn line_badge_x(width: f32, right: bool) -> f32 {
    let half = width * LINE_BADGE_VISIBLE / 2.0;
    if right {
        LINE_FRAME_OUTER.1 + half
    } else {
        LINE_FRAME_OUTER.0 - half
    }
}

/// Which end of a line carries its one badge at 50 and 25 lines, by 1-based
/// line number, as filmed (gameplay recordings y8c8lEdPW8Y, f5sMl9a5jds,
/// CAM8vranI9Y: 40 line wins on three club machines; the side of a line
/// agrees across machines). The two configurations have their own maps
/// (lines 3, 7, 10 and 19 change sides). No simple rule fits: "left if the
/// line is in the first half" fails 6 of 13 lines at 50 and 9 of 18 at 25.
const SIDE_50_LEFT: [u16; 6] = [1, 3, 7, 17, 21, 23];
const SIDE_50_RIGHT: [u16; 7] = [5, 10, 19, 20, 24, 25, 28];
const SIDE_25_LEFT: [u16; 9] = [4, 9, 10, 11, 16, 17, 18, 19, 21];
const SIDE_25_RIGHT: [u16; 8] = [2, 3, 7, 12, 13, 20, 22, 25];

/// The ends of 1-based line `line` that carry a badge, (left, right), in
/// line configuration `config`: both at 5 lines (seen 7 of 7 times); one at
/// 25 lines, but both for line 1; one at 50 lines, line 1 included (no
/// frame at 25 or 50 lines shows two badges for any other line).
fn line_badge_ends(config: u16, line: u16) -> (bool, bool) {
    let (left, right): (&[u16], &[u16]) = if config >= 50 {
        (&SIDE_50_LEFT, &SIDE_50_RIGHT)
    } else if config >= 25 && line != 1 {
        (&SIDE_25_LEFT, &SIDE_25_RIGHT)
    } else {
        return (true, true);
    };
    if left.contains(&line) {
        (true, false)
    } else if right.contains(&line) {
        (false, true)
    } else {
        // HYPOTHESIS: a line never filmed takes a side by its parity, odd
        // left, which shares the badges between the two sides; nothing
        // shows which side it really takes.
        let odd = line % 2 == 1;
        (odd, !odd)
    }
}

/// The blank badge's rounded end, as a share of the art's width (Golden
/// Century's 44 x 34 badge: about 14 px).
const LINE_BADGE_END: f32 = 14.0 / 44.0;

/// How a badge image of `width` art pixels is drawn: stretched where the
/// art's digit is shown, otherwise sliced, its rounded ends scaled with
/// the badge's height and only its middle stretched.
fn line_badge_mode(look: &LineLook, width: Option<f32>) -> NodeImageMode {
    match width {
        Some(w) if !look.own_digits => NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::axes(w * LINE_BADGE_END, 0.0),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: f32::MAX,
        }),
        _ => NodeImageMode::Stretch,
    }
}

/// A badge at one end of the shown line.
#[derive(Component)]
pub struct LineBadge {
    right: bool,
}
/// The stub from a badge across the reel frame into the reel (footage: a
/// short piece of the line's colour, at both ends where both are badged).
#[derive(Component)]
pub struct LineStub {
    right: bool,
}
/// The number on a badge (at 50 lines, lines above five, the first five
/// badges carrying their own digit; at 25 and 5 lines, every line).
#[derive(Component)]
pub struct LineBadgeNumber;
/// The dash in one gap between reels.
#[derive(Component)]
pub struct LineDash {
    gap: u8,
}

/// The pieces of the shown winning line, hidden until a line wins.
pub fn spawn_win_lines(c: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    let look = line_look(50);
    // The stubs first, so the badges cover their outer ends.
    for right in [false, true] {
        c.spawn((
            LineStub { right },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(look.stub.0),
                height: Val::Px(look.stub.1),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            GlobalZIndex(6),
            Visibility::Hidden,
        ));
    }
    for right in [false, true] {
        c.spawn((
            LineBadge { right },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(look.badge.0),
                height: Val::Px(look.badge.1),
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
                    font_size: FontSize::Px(look.number),
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
                width: Val::Px(look.dash.0),
                height: Val::Px(look.dash.1),
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

/// A line win's message as the cabinet gives it: the 1-based line and the
/// award in credits, plain digits with no grouping comma ("Line 13 wins 20",
/// "Scatter wins 1000": y8c8lEdPW8Y). `line` is the 0-based payline index.
fn line_win_note(line: u16, award: u64) -> String {
    format!("Line {} wins {award}", u32::from(line) + 1)
}

/// A line win multiplied by its wilds (a free game's x2 wilds: two make x4)
/// gives the line's own pay, the multiplier and the award: "Line 8 wins
/// 50 x 2 = 100", "Line 4 wins 50 x 4 = 200" (Genghis Khan club cabinets,
/// four recordings; VERIFIED as displayed).
fn line_win_note_times(line: u16, award: u64, multiplier: u64) -> String {
    if multiplier <= 1 || award % multiplier != 0 {
        return line_win_note(line, award);
    }
    format!(
        "Line {} wins {} x {multiplier} = {award}",
        u32::from(line) + 1,
        award / multiplier
    )
}

/// What a free game's wilds multiply a line win by: the model's wild
/// multiplier once for each wild in the line's run (1 outside the free
/// games, or with no multiplier).
fn line_multiplier(
    model: &MathModel,
    free_game: bool,
    labels: &Query<(&CellLabel, &Text), Without<LineBadgeNumber>>,
    line: u16,
    count: u8,
) -> u64 {
    let Some(m) = model
        .free_games
        .as_ref()
        .and_then(|fg| fg.wild_multiplier)
        .filter(|m| free_game && *m > 1)
    else {
        return 1;
    };
    let Some(rows) = model.paylines.get(line as usize) else {
        return 1;
    };
    let wilds = (0..count)
        .filter(|&reel| {
            rows.0.get(reel as usize).is_some_and(|&row| {
                labels.iter().any(|(l, t)| {
                    l.reel == reel
                        && l.row == row
                        && model
                            .symbols
                            .iter()
                            .any(|s| s.key == label_symbol(&t.0).0 && s.kind == SymbolKind::Wild)
                })
            })
        })
        .count();
    u64::from(m).saturating_pow(wilds as u32)
}

/// Show the winning lines in turn, and their message.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn win_lines(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    art: Res<ArtPack>,
    gamble: Res<GambleShow>,
    mut show: ResMut<FeatureShow>,
    mut badges: Query<
        (&LineBadge, &mut Node, &mut ImageNode, &mut Visibility),
        (Without<LineDash>, Without<LineStub>),
    >,
    mut stubs: Query<
        (&LineStub, &mut Node, &mut ImageNode, &mut Visibility),
        (Without<LineDash>, Without<LineBadge>),
    >,
    mut numbers: Query<(&mut Text, &mut TextFont), With<LineBadgeNumber>>,
    mut dashes: Query<
        (
            &LineDash,
            &mut Node,
            &mut ImageNode,
            &mut UiTransform,
            &mut Visibility,
        ),
        (Without<LineBadge>, Without<LineStub>),
    >,
    cell_labels: Query<(&CellLabel, &Text), Without<LineBadgeNumber>>,
) {
    if !fidelity.0 {
        return;
    }
    show.win_t += smoke.dt(&time);
    // Nothing moves from START FEATURE to the first free game (club cabinet).
    let showing = !show.win_lines.is_empty()
        && !gamble.active
        && show.awaiting.is_none()
        && show.fg_cut.is_none()
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
    // The cabinet always gives a line win in credits, in plain digits
    // ("Line 13 wins 20", "Line 4 wins 100": y8c8lEdPW8Y, 306 messages at
    // every denomination, none in dollars), while WIN counts in dollars.
    let free_game = matches!(show.stage, Stage::FreeGames { .. });
    show.win_note = current.map(|(line, count, award)| {
        let m = line_multiplier(sim.game.model(), free_game, &cell_labels, line, count);
        line_win_note_times(line, award, m)
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
        for (_, _, _, mut v) in &mut stubs {
            set(&mut v, false);
        }
        for (_, _, _, _, mut v) in &mut dashes {
            set(&mut v, false);
        }
        return;
    };
    // The pieces follow the denomination's line configuration: their size,
    // and how many badges a line has, and at which end.
    let config = line_config(denom.cents());
    let look = line_look(config);
    let ends = line_badge_ends(config, line + 1);
    // The game's five line colours, by line number.
    let group = (line as usize % 5) + 1;
    let row_y = |reel: usize| {
        let row = rows.0.get(reel).copied().unwrap_or(1);
        cell_center(reel as u8, row).1
    };
    let end_reel = |right: bool| {
        if right {
            rows.0.len().saturating_sub(1)
        } else {
            0
        }
    };
    let shown = |right: bool| if right { ends.1 } else { ends.0 };
    let digit = look.own_digits && line < 5;
    let badge_key = if digit {
        format!("line{group}")
    } else {
        format!("line_blank{group}")
    };
    let mode = line_badge_mode(&look, art.sizes.get(&badge_key).map(|s| s.0));
    for (b, mut node, mut image, mut vis) in &mut badges {
        let x = line_badge_x(look.badge.0, b.right);
        node.width = Val::Px(look.badge.0);
        node.height = Val::Px(look.badge.1);
        node.left = Val::Px(x - look.badge.0 / 2.0);
        node.top = Val::Px(row_y(end_reel(b.right)) - look.badge.1 / 2.0);
        if let Some(h) = art.ui.get(&badge_key)
            && image.image != *h
        {
            image.image = h.clone();
        }
        if image.image_mode != mode {
            image.image_mode = mode.clone();
        }
        set(&mut vis, shown(b.right));
    }
    let number = if digit {
        String::new()
    } else {
        (line + 1).to_string()
    };
    for (mut t, mut font) in &mut numbers {
        if t.0 != number {
            t.0 = number.clone();
        }
        if font.font_size != FontSize::Px(look.number) {
            font.font_size = FontSize::Px(look.number);
        }
    }
    let dash_key = format!("line{group}_s");
    // A stub from each badge across the frame into the reel, tucked 4 px
    // under the badge.
    for (s, mut node, mut image, mut vis) in &mut stubs {
        let (w, h) = look.stub;
        let left = if s.right {
            LINE_FRAME_OUTER.1 + 4.0 - w
        } else {
            LINE_FRAME_OUTER.0 - 4.0
        };
        node.width = Val::Px(w);
        node.height = Val::Px(h);
        node.left = Val::Px(left);
        node.top = Val::Px(row_y(end_reel(s.right)) - h / 2.0);
        if let Some(handle) = art.ui.get(&dash_key)
            && image.image != *handle
        {
            image.image = handle.clone();
        }
        set(&mut vis, shown(s.right));
    }
    for (d, mut node, mut image, mut transform, mut vis) in &mut dashes {
        let a = d.gap as usize;
        let (y0, y1) = (row_y(a), row_y(a + 1));
        let x = (cell_center(d.gap, 0).0 + cell_center(d.gap + 1, 0).0) / 2.0;
        let y = (y0 + y1) / 2.0;
        node.width = Val::Px(look.dash.0);
        node.height = Val::Px(look.dash.1);
        node.left = Val::Px(x - look.dash.0 / 2.0);
        node.top = Val::Px(y - look.dash.1 / 2.0);
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

/// Frames per second of the stack animations (their frames' size and
/// place: each title's `tall3` / `tall2`, crate::looks).
const WILD_FPS: f32 = 20.0;

/// Emperor stacks: those standing tall (reel, top row, height, since when in
/// seconds of `Time`), and those landed (reel, top row, height).
#[derive(Resource, Default)]
pub struct WildStacks {
    list: Vec<(u8, u8, u8, f32)>,
    landed: Vec<(u8, u8, u8)>,
    /// The stacks of this spin have been found.
    found: bool,
    /// Wilds filling their reels this game (Golden Gong's monk), until the
    /// next spin.
    expanded: Vec<Expansion>,
}

/// A wild filling its reel: the landed wild grows into the reel's standing
/// figure, the title's own sequence for the row it landed on, its win
/// version when the reel is in a win (then the standing loop), its no-win
/// version held on its last frame otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Expansion {
    pub reel: u8,
    /// The row the wild landed on (0 top).
    pub row: u8,
    pub win: bool,
    /// The wild's key, which every position of the reel takes once the
    /// figure covers it.
    pub key: String,
    /// When the sequence began (seconds of `Time`).
    pub t0: f32,
    /// The reel's positions show the wild now.
    pub relabelled: bool,
}

/// A wild filling its reel plays its sequence at 20 frames a second from the
/// moment the reels are still, two frames in (Golden Gong club cabinets: the
/// figure visibly grows from rest + 0.33-0.37 s and fills the reel at rest +
/// 0.67-0.70 s on twelve expansions in three recordings, the pack's
/// sequences showing the landed head for frames 0-7 and growing over 8-13;
/// VERIFIED).
pub const EXPAND_FPS: f32 = 20.0;
pub const EXPAND_LEAD: f32 = 2.0 / EXPAND_FPS;
/// The reel's positions take the wild once the figure covers them (frame
/// 14 of the sequence).
pub const EXPAND_FULL: f32 = 14.0 / EXPAND_FPS;
/// After an expansion the first line shows at rest + 1.5 s and WIN counts
/// 0.25 s after the expansion's sound (rest + 2.45-2.60 s against the
/// 2.3 s SND_ExpandWin; club cabinets, four events; STRONG / VERIFIED).
pub const EXPAND_LINES_AFTER: f32 = 1.5;
pub const EXPAND_COUNT_LAG: f32 = 0.25;

/// Seconds the win waits for a stack to stand: Golden Century's SND_Wild_N
/// (a title with its own wild sound holds for its length instead).
pub const WILD_HOLD: f32 = 2.5;
/// WIN counts this long after the wild's sound ends, as a win with no wild
/// counts 0.20 s after its first line (Panda Magic club cabinets; STRONG).
pub const WILD_COUNT_LAG: f32 = 0.2;

impl WildStacks {
    /// Look for the stacks again (a wild has filled its reel); the reels
    /// filling stay.
    pub fn refind(&mut self) {
        let expanded = std::mem::take(&mut self.expanded);
        *self = WildStacks {
            expanded,
            ..WildStacks::default()
        };
    }

    /// A wild begins to fill its reel.
    pub fn expand(&mut self, expansion: Expansion) {
        self.expanded.retain(|e| e.reel != expansion.reel);
        self.expanded.push(expansion);
    }

    /// The reels filling whose positions should take the wild now (`now` in
    /// seconds of `Time`): marked as done.
    pub fn due_relabels(&mut self, now: f32) -> Vec<(u8, String)> {
        let mut due = Vec::new();
        for e in &mut self.expanded {
            if !e.relabelled && now - e.t0 >= EXPAND_FULL {
                e.relabelled = true;
                due.push((e.reel, e.key.clone()));
            }
        }
        due
    }

    /// Whether `reel` is filling (or filled) with a wild this game.
    pub fn expanding(&self, reel: u8) -> bool {
        self.expanded.iter().any(|e| e.reel == reel)
    }

    /// This spin's stacks have been found.
    pub fn found(&self) -> bool {
        self.found
    }

    /// The height of the tallest landed stack on one of these winning lines
    /// (line, symbols in a row from reel 1), if any.
    pub fn on_lines(
        &self,
        paylines: &[dragonlink_model::Payline],
        wins: impl Iterator<Item = (u16, u8)> + Clone,
    ) -> Option<u8> {
        self.landed
            .iter()
            .filter(|&&stack| stack_on_lines(stack, paylines, wins.clone()))
            .map(|&(_, _, height)| height)
            .max()
    }

    /// Stand the landed stacks that are on a winning line, from `now`
    /// (seconds of `Time`); a stack on no winning line stays as heads
    /// (Golden Century's rule; Genghis Khan club cabinet 3ggOSOerdhc
    /// 2:22.5-2:26, a 2-high stack on a win standing beside a 3-high one on
    /// none; STRONG).
    pub fn stand(
        &mut self,
        paylines: &[dragonlink_model::Payline],
        wins: impl Iterator<Item = (u16, u8)> + Clone,
        now: f32,
    ) {
        self.list = self
            .landed
            .iter()
            .filter(|&&stack| stack_on_lines(stack, paylines, wins.clone()))
            .map(|&(reel, top, height)| (reel, top, height, now))
            .collect();
    }
}

/// Whether a landed stack (reel, top row, height) is in the run of one of
/// these winning lines.
fn stack_on_lines(
    (reel, top, height): (u8, u8, u8),
    paylines: &[dragonlink_model::Payline],
    mut wins: impl Iterator<Item = (u16, u8)>,
) -> bool {
    wins.any(|(line, count)| {
        reel < count
            && paylines
                .get(line as usize)
                .and_then(|rows| rows.0.get(reel as usize))
                .is_some_and(|&row| row >= top && row < top + height)
    })
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

/// A free game's BIG symbol, drawn once over the reels it covers.
#[derive(Component)]
pub struct BigArt;
/// A BIG coin's prize.
#[derive(Component)]
pub struct BigValue;

/// The BIG symbol (hidden until one lands).
pub fn spawn_big_art(c: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    c.spawn((
        BigArt,
        Node {
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        GlobalZIndex(3),
        Visibility::Hidden,
    ))
    .with_children(|b| {
        b.spawn((
            BigValue,
            Text::new(""),
            TextFont {
                font: bevy::text::FontSource::Handle(font.clone()),
                font_size: FontSize::Px(110.0),
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.05, 0.02)),
            TextLayout::justify(Justify::Center),
        ));
    });
}

/// Show the free game's BIG symbol once the reels it covers are still: one
/// 3x3 picture (the title's own Mega art, `big_<key>`), a BIG coin with its
/// prize on it, through any Hold & Spin it starts.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn big_art(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    reels: Res<Reels>,
    art: Res<ArtPack>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    labels: Query<(&CellLabel, &Text), Without<BigValue>>,
    mut big: Query<(&mut Node, &mut ImageNode, &mut Visibility), With<BigArt>>,
    mut value: Query<&mut Text, With<BigValue>>,
    gamble: Res<GambleShow>,
) {
    if !fidelity.0 {
        return;
    }
    // Not over the gamble (see `feature_screen`).
    let shown = show
        .big
        .as_ref()
        .filter(|_| !gamble.active)
        .filter(|(_, first, width)| !(*first..first + width).any(|reel| reels.moving(reel)));
    let image = shown.and_then(|(key, _, _)| art.symbols.get(&format!("big_{key}")));
    for (mut node, mut img, mut vis) in &mut big {
        let (Some((key, first, width)), Some(handle)) = (shown, image) else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        if img.image != *handle {
            img.image = handle.clone();
        }
        let (left, _) = cell_center(*first, 0);
        let (right, _) = cell_center(first + width - 1, 0);
        let (_, top) = cell_center(*first, 0);
        let w = right - left + PITCH.0;
        // The BIG coin stands a little over the block, as in the scene
        // (718 against 660).
        let h = 3.0 * PITCH.1 * if key == "orb" { 718.0 / 660.0 } else { 1.0 };
        node.left = Val::Px(left - PITCH.0 / 2.0);
        node.top = Val::Px(top - PITCH.1 / 2.0 - (h - 3.0 * PITCH.1) / 2.0);
        node.width = Val::Px(w);
        node.height = Val::Px(h);
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
    }
    // A BIG coin's prize: its centre position's label.
    let prize = shown
        .filter(|(key, _, _)| key == "orb")
        .and_then(|(_, first, width)| {
            let centre = first + width / 2;
            labels
                .iter()
                .find(|(l, _)| l.reel == centre && l.row == 1)
                .and_then(|(_, t)| t.0.strip_prefix("ORB\n").map(str::to_string))
        })
        .map(|face| match face_credits(&face, sim.game.total_bet()) {
            Some(credits) => coin_text(credits, denom.cents()),
            None => face.to_uppercase(),
        })
        .unwrap_or_default();
    for mut text in &mut value {
        if text.0 != prize {
            text.0.clone_from(&prize);
        }
    }
}

/// Find emperor stacks as reels land; forget them as reels spin.
pub fn wild_stacks(
    fidelity: Res<Fidelity>,
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
    if stacks.found {
        return;
    }
    stacks.found = true;
    for reel in 0..reels.states.len() as u8 {
        // A reel the wild fills stands as its own figure, not as a stack.
        if stacks.expanding(reel) {
            continue;
        }
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
            // A run is a stack only where the title has tall art for it:
            // Panda Magic's and Happy & Prosperous's stacked wilds each
            // animate in their own cell (club cabinets: tg4nMylHxhI,
            // 8V086h-IJ6I, every panda of a stack; VERIFIED).
            let height = row - start;
            if height >= 2 && art.anims.contains_key(&format!("wild{height}_intro")) {
                stacks.landed.push((reel, start as u8, height as u8));
            }
        }
    }
}

/// Draw the tall emperors: the wipe, then the burning figure.
#[allow(clippy::too_many_arguments)]
pub fn wild_talls(
    fidelity: Res<Fidelity>,
    time: Res<Time>,
    art: Res<ArtPack>,
    look: Res<crate::looks::TitleLook>,
    images: Res<Assets<Image>>,
    stacks: Res<WildStacks>,
    show: Res<FeatureShow>,
    gamble: Res<GambleShow>,
    mut talls: Query<(&WildTall, &mut Node, &mut ImageNode, &mut Visibility)>,
) {
    if !fidelity.0 {
        return;
    }
    // Not over a Hold & Spin board, nor over the gamble (its head stands
    // above the gamble screen's top; see `feature_screen`).
    if show.hold_and_spin() || gamble.active {
        for (_, _, _, mut vis) in &mut talls {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    }
    let now = time.elapsed_secs();
    for (tall, mut node, mut image, mut vis) in &mut talls {
        // A reel the wild fills: its sequence for the landed row, then the
        // standing loop (in a win) or its last frame (no win).
        let filling = stacks
            .expanded
            .iter()
            .find(|e| e.reel == tall.reel)
            .and_then(|e| {
                let row = ["top", "middle", "bottom"].get(e.row as usize)?;
                let name = if e.win {
                    format!("wild3_expand_{row}")
                } else {
                    format!("wild3_expand_{row}_nowin")
                };
                let frames = art.anims.get(&name).filter(|f| !f.is_empty())?;
                let n = ((now - e.t0).max(0.0) * EXPAND_FPS) as usize;
                let frame = if n < frames.len() {
                    frames[n].clone()
                } else if e.win {
                    let lp = art.anims.get("wild3_loop").filter(|f| !f.is_empty())?;
                    lp[(n - frames.len()) % lp.len()].clone()
                } else {
                    frames[frames.len() - 1].clone()
                };
                Some((e.reel, 0u8, 3u8, frame, None))
            });
        // A stack standing on a winning line: its intro, then its loop.
        let standing = || {
            let &(reel, top, len, t0) = stacks.list.iter().find(|s| s.0 == tall.reel)?;
            let n = ((now - t0) * WILD_FPS) as usize;
            let intro = art.anims.get(&format!("wild{len}_intro"))?;
            let frame = if n < intro.len() {
                intro.get(n).cloned()
            } else {
                let lp = art.anims.get(&format!("wild{len}_loop"))?;
                lp.get((n - intro.len()) % lp.len().max(1)).cloned()
            }?;
            let (w, h, dy) = if len >= 3 {
                look.tall3
            } else if top == 0 {
                (look.tall2.0, look.tall2.1, look.tall2.2)
            } else {
                (look.tall2.0, look.tall2.1, look.tall2.3)
            };
            Some((reel, top, len, frame, Some((w, h, dy))))
        };
        let want = match filling.or_else(standing) {
            Some((reel, top, len, h, rect)) => {
                // A stack's figure in its title's own rect; a filling reel's
                // frames at its title's frame pixels a cell (crate::looks).
                let size = images.get(&h).map(|i| i.size_f32());
                if image.image != h {
                    image.image = h;
                }
                let (w, hh, dy) = match rect {
                    Some((w, h, dy)) => (w * PITCH.0, h * PITCH.1, dy),
                    None => {
                        let (px_x, px_y) = look.expand_px;
                        let (w, h) = size.map_or((PITCH.0, f32::from(len) * PITCH.1), |s| {
                            (s.x / px_x * PITCH.0, s.y / px_y * PITCH.1)
                        });
                        (w, h, look.expand_dy)
                    }
                };
                let (cx, cy) = cell_center(reel, top);
                let cy = cy + (f32::from(len) - 1.0) / 2.0 * PITCH.1 + dy * PITCH.1;
                node.left = Val::Px(cx - w / 2.0);
                node.top = Val::Px(cy - hh / 2.0);
                node.width = Val::Px(w);
                node.height = Val::Px(hh);
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

/// Frames per second of a symbol win animation the pack gives no rate for:
/// a video texture's 20 (Golden Century's warriors 120 frames over a 180-frame
/// range, the official app and the club cabinet both at 20 fps; Happy &
/// Prosperous's jade turning in 4.00 s on two recordings; STRONG).
const SYMBOL_FX_FPS: f32 = 20.0;

/// The win animation over one cell (hidden until it wins).
pub fn spawn_symbol_fx(cell: &mut ChildSpawnerCommands, reel: u8, row: u8) {
    cell.spawn((
        SymbolUnder { reel, row },
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
    cell.spawn((
        SymbolBadge { reel, row },
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

/// A free game's x2 badge pulsing over a winning x2 wild's animation.
#[derive(Component)]
pub struct SymbolBadge {
    reel: u8,
    row: u8,
}

/// A layered wild's back layer under its figure's animation (Happy &
/// Prosperous: the red lotus tile with gold coins falling, `wild_bg_*`, under
/// the Buddha, `buddha_*`; its mobile scene's WildBG and WildFG sprites, one
/// rect; club cabinets show the tile and the coins through every win, STRONG).
#[derive(Component)]
pub struct SymbolUnder {
    reel: u8,
    row: u8,
}

/// The wild's win animation (and a free game's x2 badge) runs at 20 frames a
/// second: the badge's 40 frames pulse every 1.97 s on Genghis Khan club
/// cabinets (GfJgVpPiNbw 27:29-27:35, 1mhLqnuUGTw 9:52; STRONG), as Golden
/// Century's tall emperor does (VERIFIED).
const WILD_FX_FPS: f32 = 20.0;

/// The frame of a symbol's win animation `n` frames in: its intro once, then
/// its loop.
fn fx_frame(art: &ArtPack, key: &str, n: usize) -> Option<Handle<Image>> {
    let intro = art.anims.get(&format!("{key}_intro"));
    let lp = art.anims.get(&format!("{key}_loop"))?;
    match intro {
        Some(i) if n < i.len() => i.get(n).cloned(),
        Some(i) => lp.get((n - i.len()) % lp.len().max(1)).cloned(),
        None => lp.get(n % lp.len().max(1)).cloned(),
    }
}

/// Animate the symbols of the winning line being shown.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn symbol_fx(
    fidelity: Res<Fidelity>,
    show: Res<FeatureShow>,
    sim: Res<Sim>,
    art: Res<ArtPack>,
    look: Res<crate::looks::TitleLook>,
    stacks: Res<WildStacks>,
    gamble: Res<GambleShow>,
    labels: Query<(&CellLabel, &Text)>,
    mut fx: Query<
        (&SymbolFx, &mut Node, &mut ImageNode, &mut Visibility),
        (
            Without<CellImage>,
            Without<SymbolBadge>,
            Without<SymbolUnder>,
        ),
    >,
    mut badges: Query<
        (&SymbolBadge, &mut Node, &mut ImageNode, &mut Visibility),
        (Without<CellImage>, Without<SymbolFx>, Without<SymbolUnder>),
    >,
    mut unders: Query<
        (&SymbolUnder, &mut Node, &mut ImageNode, &mut Visibility),
        (Without<CellImage>, Without<SymbolFx>, Without<SymbolBadge>),
    >,
    mut images: Query<
        (&CellImage, &mut Visibility),
        (
            Without<SymbolFx>,
            Without<SymbolBadge>,
            Without<SymbolUnder>,
        ),
    >,
) {
    if !fidelity.0 {
        return;
    }
    let hide = |vis: &mut Visibility| {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    };
    // Over the gamble no symbol shows, still or animated: the tall ones
    // overhang the gamble screen's top (see `feature_screen`); they come back
    // with the last line win after a loss.
    if gamble.active {
        for (_, _, _, mut vis) in &mut fx {
            hide(&mut vis);
        }
        for (_, _, _, mut vis) in &mut badges {
            hide(&mut vis);
        }
        for (_, _, _, mut vis) in &mut unders {
            hide(&mut vis);
        }
        for (_, mut vis) in &mut images {
            hide(&mut vis);
        }
        return;
    }
    let model = sim.game.model();
    let is_wild = |key: &str| {
        model
            .symbols
            .iter()
            .any(|s| s.key == key && s.kind == SymbolKind::Wild)
    };
    let key_at = |reel: u8, row: u8| {
        labels
            .iter()
            .find(|(l, _)| l.reel == reel && l.row == row)
            .map(|(_, t)| label_symbol(&t.0).0.to_string())
            .unwrap_or_default()
    };
    let run = |line: u16, count: u8| -> Vec<(u8, u8)> {
        model
            .paylines
            .get(line as usize)
            .map(|rows| {
                (0..count)
                    .filter_map(|reel| rows.0.get(reel as usize).map(|&row| (reel, row)))
                    .collect()
            })
            .unwrap_or_default()
    };
    // The positions animating now: the shown line's symbols, and every wild
    // in any of the game's wins, which start together with the first line
    // and keep on through all of them (club cabinets: Genghis Khan GfJgVpPiNbw
    // 27:29-27:35 and 1mhLqnuUGTw 9:52, Panda Magic tg4nMylHxhI; STRONG).
    let line_cells = show
        .win_current
        .map(|(line, count)| run(line, count))
        .unwrap_or_default();
    let mut cells = line_cells.clone();
    if show.win_current.is_some() {
        for &(line, count, _) in &show.win_lines {
            for (reel, row) in run(line, count) {
                if is_wild(&key_at(reel, row)) && !cells.contains(&(reel, row)) {
                    cells.push((reel, row));
                }
            }
        }
    }
    if show.scatter_shown {
        for (l, t) in &labels {
            if t.0 == art.roles.scatter {
                cells.push((l.reel, l.row));
            }
        }
    }
    let t = show.win_t;
    let (cw, ch) = cell_size();
    // A free game's x2 badge: its frames and its place over the wild's box.
    let badge_frames = look
        .x2_badge
        .and_then(|place| art.anims.get("wild_x2").map(|f| (place, f)));
    let mut badge_at: Vec<((u8, u8), Handle<Image>, (f32, f32, f32, f32))> = Vec::new();
    let mut under_at: Vec<((u8, u8), Handle<Image>, (f32, f32, f32, f32))> = Vec::new();
    let under_fps = art
        .anim_fps
        .get("wild_bg_loop")
        .copied()
        .unwrap_or(SYMBOL_FX_FPS);
    for (f, mut node, mut image, mut vis) in &mut fx {
        let on = cells.contains(&(f.reel, f.row))
            && !stacks.list.iter().any(|s| s.0 == f.reel)
            && !stacks.expanding(f.reel);
        let key = key_at(f.reel, f.row);
        let wild = is_wild(&key);
        // A free game's own wild (an x2 wild) has no animation of its own:
        // it plays the wild's, with its badge over it (the title's mobile
        // scene, WildFG).
        let own = art.anims.contains_key(&format!("{key}_loop"));
        let anim_key = if !own && wild {
            art.roles.wild.clone()
        } else {
            key.clone()
        };
        // Each animation at its own rate where the pack records it (most
        // are video textures drawn at 20 frames a second: the official app's
        // and club cabinets' pictures, flags and wilds; image sequences 30),
        // a video's 20 otherwise.
        let fps = art
            .anim_fps
            .get(&format!("{anim_key}_loop"))
            .copied()
            .unwrap_or(SYMBOL_FX_FPS);
        let frame = on
            .then(|| fx_frame(&art, &anim_key, (t * fps) as usize))
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
                if wild && let Some(bg) = fx_frame(&art, "wild_bg", (t * under_fps) as usize) {
                    under_at.push((
                        (f.reel, f.row),
                        bg,
                        ((cw - w) / 2.0, (ch - hh) / 2.0, w, hh),
                    ));
                }
                if anim_key != key
                    && let Some(((dx, dy, bw, bh), frames)) = badge_frames
                    && !frames.is_empty()
                {
                    let n = (t * WILD_FX_FPS) as usize % frames.len();
                    let (bw, bh) = (bw * w, bh * hh);
                    let (cx, cy) = (cw / 2.0 + dx * w, ch / 2.0 + dy * hh);
                    badge_at.push((
                        (f.reel, f.row),
                        frames[n].clone(),
                        (cx - bw / 2.0, cy - bh / 2.0, bw, bh),
                    ));
                }
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
                if ci.reel == f.reel && ci.row == f.row {
                    hide(&mut v);
                }
            }
        }
    }
    for (u, mut node, mut image, mut vis) in &mut unders {
        match under_at.iter().find(|(at, ..)| *at == (u.reel, u.row)) {
            Some((_, frame, (left, top, w, h))) => {
                if image.image != *frame {
                    image.image = frame.clone();
                }
                node.left = Val::Px(*left);
                node.top = Val::Px(*top);
                node.width = Val::Px(*w);
                node.height = Val::Px(*h);
                if *vis != Visibility::Inherited {
                    *vis = Visibility::Inherited;
                }
            }
            None => hide(&mut vis),
        }
    }
    for (b, mut node, mut image, mut vis) in &mut badges {
        match badge_at.iter().find(|(at, ..)| *at == (b.reel, b.row)) {
            Some((_, frame, (left, top, w, h))) => {
                if image.image != *frame {
                    image.image = frame.clone();
                }
                node.left = Val::Px(*left);
                node.top = Val::Px(*top);
                node.width = Val::Px(*w);
                node.height = Val::Px(*h);
                if *vis != Visibility::Inherited {
                    *vis = Visibility::Inherited;
                }
            }
            None => hide(&mut vis),
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
// tune; the official app asks SELECT SPIN TO START FEATURE after the count, and
// its flags wave on for about a second after the press. The club cabinet
// (which the app follows) differs there: "N FREE GAMES WON!" on the status
// line from the result, "Scatter wins N" from the wave, PRESS START FEATURE
// BUTTON as the scatter win starts counting (the count goes on under it); the
// press makes no sound, the flags go still at once, and nothing moves until
// the first free game spins 0.52 s after the press: the free games' music
// from its beginning, the banner's total counting up from 1, one a tick
// (SND_Fea_Inc), during that spin.

/// Seconds into the trigger sequence (it begins when the result is shown,
/// 0.17 s after the last reel is still): the bell, the flags and their sound,
/// and the win's count.
const FG_BELL: f32 = 0.045;
const FG_FLAGS: f32 = FG_BELL + 2.6;
pub const FG_COUNT: f32 = FG_FLAGS + 4.87;
/// The scatter win counts this long after the title's SND_Scatter ends:
/// Genghis Khan 6.20-6.25 s on ten triggers (five recordings, each start
/// placed by the clip itself) against its 6.11 s clip, Peace & Long Life
/// 4.15-4.18 s against 4.005 s, Golden Century's club audit 4.87 s against
/// 4.746 s (STRONG; the other titles HYPOTHESIS, by the same rule).
const WAVE_AFTER_CLIP: f32 = 0.12;
/// Seconds from the START FEATURE press (the frame its prompt goes) to the
/// first free game's reels moving: club cabinets 0.53 s (y8c8lEdPW8Y
/// 949.80-950.33) and 0.50 s (1bAevGz0K2k 328.60-329.10), VERIFIED; the
/// official app about 1.1 s.
const FG_FIRST_SPIN: f32 = 0.52;
/// The cut: the first free game is played then, and its start is shown one
/// event interval later, with its reels.
const FG_CUT: f32 = FG_FIRST_SPIN - TIMING.event_interval;
/// Seconds per step of the banner's total counting up after the cut: it
/// shows "FREE GAME 1 OF 0" for one step, then 1 to the total a step each,
/// and the status line stays blank until one step after the total (club
/// cabinets, frame by frame: y8c8lEdPW8Y 950.400-951.567, 1bAevGz0K2k
/// 329.167-330.333, 5 video frames a step on both; VERIFIED). The official
/// app counts from 1, 4 frames a step.
const FG_TICK: f32 = 5.0 / 30.0;

/// From the WIN plaque going to the next free game, after a Hold & Spin
/// inside the free games: 0.37-0.43 s on a club cabinet (y8c8lEdPW8Y
/// 1019.93-1020.37, 2562.93-2563.37, 3046.57-3046.93; VERIFIED, one
/// machine), the banner "FREE GAME N+1 OF 6" and the reels together. Less
/// the event interval the next free game's start waits.
const FG_RESUME: f32 = 0.43 - TIMING.event_interval;

/// A free game with no win: the pause before the next one, so that its reels
/// start 0.5 s after the doors' reveal ends, at the rest + 2.0 s as on club
/// cabinets (1bAevGz0K2k 333.67-335.63, 346.57-348.57, 359.60-361.60;
/// y8c8lEdPW8Y 954.90-956.87; STRONG). The game's last event shows as the
/// reveal ends (its interval runs out under the reveal), and the next
/// game's start shows an event interval after the pause. With no doors the
/// next game spins about 0.67 s after the rest (the cabinet's one case
/// 0.5 s; HYPOTHESIS). The official app waited 0.85 s (about 2.5 s after
/// its doors opened).
const FG_NO_WIN_PAUSE: f32 = 0.5 - TIMING.event_interval;

impl FeatureShow {
    /// Seconds before a feature's next step: a free game starts 1.25 s
    /// after its win has counted (the official app), or, with no win, 0.5 s
    /// after its doors' reveal ends (club cabinets, `FG_NO_WIN_PAUSE`); a
    /// respin's step 0.5 s.
    pub fn step_pause(&self) -> f32 {
        match self.stage {
            // Back from a Hold & Spin inside the free games: the next free
            // game soon after the plaque goes (both arms: the timer is armed
            // in the last frame before the return shows).
            Stage::HoldSpinDone if self.fg_return.is_some() => FG_RESUME,
            Stage::FreeGames { .. } if self.resuming => FG_RESUME,
            Stage::FreeGames { .. } if self.game_won => 1.25,
            Stage::FreeGames { .. } => FG_NO_WIN_PAUSE,
            _ => 0.5,
        }
    }

    /// The trigger sequence is at least `after` seconds past the coins'
    /// burst.
    fn since_burst(&self, after: f32) -> bool {
        self.intro.is_some_and(|t| t >= INTRO_BURST + after)
    }

    /// A win through an emperor sounds one of the wild sounds, once a game,
    /// and WIN's count waits for it while the lines show (`wild_holds`); a
    /// stack of emperors on the winning line also stands (real game: a stack
    /// held the win for the sound's 2.5 s).
    /// On recordings the variant changes from win to win, the same stack
    /// sounding different ones; it is not the stack's height. Here the five
    /// take turns in a scrambled order.
    /// A free game's x2 wild (Genghis Khan, Peace & Long Life) sounds its own
    /// clip, SND_WildFG, where the title has one (names only; HYPOTHESIS).
    pub fn begin_wild(&mut self, x2: bool) {
        if self.wild_sounded {
            return;
        }
        self.wild_sounded = true;
        self.wild = Some(0.0);
        if x2 {
            self.cues.push("SND_WildFG");
            // A title with no free-games wild sound plays none and holds
            // nothing (Peace & Long Life's x2 wild is silent on club
            // cabinets).
            self.wild_for = if self.wild_fg_clip > 0.0 {
                self.wild_fg_clip + WILD_COUNT_LAG
            } else {
                0.0
            };
            return;
        }
        self.wild_for = self.wild_hold();
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
    }

    /// Wilds fill their reels: Golden Gong's expansion sound, once a game,
    /// with the sequence (the win version when a filled reel is in a win;
    /// the pairing HYPOTHESIS, the two clips nearly alike), which is the
    /// game's wild sound: no SND_Wild follows (club cabinets; STRONG).
    pub fn begin_expansion(&mut self, win: bool) {
        if self.expand.is_some() {
            if win {
                self.expand_won = true;
            }
            return;
        }
        self.expand = Some(0.0);
        self.expand_won = win;
        self.wild_sounded = true;
        self.cues.push(if win {
            "SND_ExpandWin"
        } else {
            "SND_ExpandNoWin"
        });
    }

    /// Whether this game's expansion holds `front`: the lines until rest +
    /// 1.5 s, WIN's count until 0.25 s after the expansion's sound.
    pub fn expansion_holds(&self, front: Option<&Event>) -> bool {
        let Some(t) = self.expand else {
            return false;
        };
        let clip = if self.expand_clip > 0.0 {
            self.expand_clip
        } else {
            2.3
        };
        match front {
            Some(Event::LineWin { .. }) => t < EXPAND_LINES_AFTER,
            Some(Event::CreditsChanged { .. }) => self.expand_won && t < clip + EXPAND_COUNT_LAG,
            _ => false,
        }
    }

    /// Whether a wild sound has played this game (the presenter asks once).
    pub fn wild_sounded(&self) -> bool {
        self.wild_sounded
    }

    /// Begin the free games' trigger sequence, if a base game's flags have
    /// won the free games and it has not begun; true when it begins. The
    /// games won and the scatter's pay are known from the start: the status
    /// and message rows show them before those events are presented.
    pub fn begin_free_games_trigger(&mut self, games: u8, scatter: u64) -> bool {
        if self.fg_trigger.is_some() || !matches!(self.stage, Stage::Base) {
            return false;
        }
        self.fg_trigger = Some(0.0);
        self.games_won = games;
        self.scatter_award = scatter;
        true
    }

    /// Begin a retrigger's sequence, if a free game's scatters have added
    /// games (`added`, `remaining` after them) and it has not begun; true
    /// when it begins. As club cabinets show it (Panda Magic tg4nMylHxhI and
    /// 6b5UuR0po4g, three retriggers frame by frame; Autumn Moon, Happy &
    /// Prosperous and Peace & Long Life the same; VERIFIED): "N FREE GAMES
    /// WON!" on the status line from the result, the bell, the scatters'
    /// wave with SND_Scatter and "Scatter wins N" as on a base trigger; where
    /// a base trigger would ask for START FEATURE, the banner's total counts
    /// up from the old total, one `FG_TICK` a step; the game's win counts two
    /// steps after the last; no press.
    pub fn begin_retrigger(&mut self, added: u8, remaining: u32, scatter: u64) -> bool {
        let Stage::FreeGames { number, total } = self.stage else {
            return false;
        };
        if self.retrigger.is_some() {
            return false;
        }
        self.fg_trigger = Some(0.0);
        self.games_won = added;
        self.scatter_award = scatter;
        self.retrigger = Some((total, (number + remaining).max(total)));
        true
    }

    /// The trigger clock at which a retrigger's sequence lets the game's win
    /// count: two steps after the banner's last (0.33-0.37 s measured).
    fn retrigger_ends_at(&self) -> Option<f32> {
        self.retrigger
            .map(|(old, new)| self.fg_count_at() + (new.saturating_sub(old) + 1) as f32 * FG_TICK)
    }

    /// The free games' total as the banner shows it (counting up after the
    /// cut).
    fn counter_total(&self, total: u32) -> u32 {
        self.fg_count.map_or(total, |(_, n)| n.min(total))
    }
}

/// The banner's total counting up to `total`, `t` seconds after it began
/// from 0, having shown `n`: the count-up's state now (none once it is over,
/// one step after the total) and how many steps it took since.
fn fg_count_step(n: u32, total: u32, t: f32) -> (Option<(f32, u32)>, u32) {
    let mut shown = n;
    while shown < total && t >= (shown + 1) as f32 * FG_TICK {
        shown += 1;
    }
    let next = (shown < total || t < (total + 1) as f32 * FG_TICK).then_some((t, shown));
    (next, shown - n)
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
    if let Some(t) = show.expand {
        show.expand = Some(t + dt);
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
        // A retrigger: one step before the moment a base trigger asks for
        // START FEATURE, the banner's total starts counting from the old
        // total (its first step comes one tick later, at that moment).
        if let (Some((old, new)), Stage::FreeGames { number, .. }) = (show.retrigger, show.stage) {
            let arm = show.fg_count_at() - FG_TICK;
            if crossed(t0, t, arm) {
                show.stage = Stage::FreeGames { number, total: new };
                show.fg_count = Some((old as f32 * FG_TICK + (t - arm), old));
            }
        }
    }
    if show.advance_cut(dt) {
        // The cut: the first free game is played at once; its start (the
        // banner, its total counting up, the music) shows with its reels.
        show.fg_flags = false;
        show.scatter_paid = false;
        presentation.step_timer = 0.0;
    }
    if let Some((t0, n)) = show.fg_count {
        let total = match show.stage {
            Stage::FreeGames { total, .. } => total,
            _ => 0,
        };
        let (next, ticks) = fg_count_step(n, total, t0 + dt);
        for _ in 0..ticks {
            // One tick a step, as the banner's digit changes (one a step is
            // HYPOTHESIS: the cabinet's sound was not measured).
            sounds.play(&mut commands, "SND_Fea_Inc");
        }
        show.fg_count = next;
    }
}

// ---------------------------------------------------------------------------
// The counter's burst
// ---------------------------------------------------------------------------
//
// The official app stands a HOLD & SPIN medallion over the reels between two
// fires; the club cabinet shows none over the reels (its medallion is on the
// top screen), so the app has none (see the trigger sequence above).

/// The burst on the banner's counter as it reaches 3.
#[derive(Component)]
pub struct BannerBurst;

/// The counter's burst as club cabinets show it: a white to blue-white star
/// flash with sparks on the digit for about 0.23 s, then a thin ring round
/// it, gone by about 0.37-0.43 s (VERIFIED, four features on three
/// recordings at two venues: y8c8lEdPW8Y 970.967-971.30 and 2522.167-
/// 2522.567, 1bAevGz0K2k 115.867-116.233, f5sMl9a5jds 212.700-213.133), about
/// 1.9 banner heights across with its sparks (PARTIAL, camera-measured). The
/// pack has no white flash, so its orange burst is drawn recoloured
/// blue-white (`BannerBurstFrames`), its visible frames (0-17) at 40 fps, on
/// a square whose ring (0.81 of the frame) spans about 1.9 banner heights.
/// The official app's is the orange burst at 30 fps, larger.
const COUNTER_BURST_FPS: f32 = 40.0;
const COUNTER_BURST_LAST: usize = 17;
const COUNTER_BURST_SIZE: f32 = 230.0;

/// The frame of the counter's burst `since` seconds after the 3 showed,
/// while it shows.
fn counter_burst_frame(since: f32) -> Option<usize> {
    let n = (since.max(0.0) * COUNTER_BURST_FPS) as usize;
    (n <= COUNTER_BURST_LAST).then_some(n)
}

/// The pack's burst frames recoloured blue-white for the counter's flash,
/// built once they have loaded (the pack itself is untouched; empty until
/// then, or when the pack has no burst).
#[derive(Resource, Default)]
pub struct BannerBurstFrames(pub Vec<Handle<Image>>);

/// A burst pixel made blue-white: its alpha kept, its colour from pale blue
/// (200, 215, 255) to white by its brightness.
fn blue_white(px: [u8; 4]) -> [u8; 4] {
    let k = f32::from(px[0].max(px[1]).max(px[2])) / 255.0;
    let lerp = |from: f32| (from + (255.0 - from) * k).round().clamp(0.0, 255.0) as u8;
    [lerp(200.0), lerp(215.0), 255, px[3]]
}

/// Prepare the feature art once it has loaded: the counter's blue-white
/// burst frames (copies), and the Hold & Spin frame without its own row
/// lines (`clear_row_lines`; ours are drawn thinner and paler).
pub fn prepare_feature_art(
    fidelity: Res<Fidelity>,
    art: Res<ArtPack>,
    mut images: ResMut<Assets<Image>>,
    mut flash: ResMut<BannerBurstFrames>,
    mut frame_done: Local<bool>,
) {
    if !fidelity.0 {
        return;
    }
    if flash.0.is_empty()
        && let Some(src) = art.anims.get("burst")
        && !src.is_empty()
        && src.iter().all(|h| images.get(h).is_some())
    {
        let copies: Vec<Image> = src
            .iter()
            .filter_map(|h| images.get(h).cloned())
            .map(|mut image| {
                if image.texture_descriptor.format
                    == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                    && let Some(data) = image.data.as_mut()
                    && data.len() % 4 == 0
                {
                    for px in data.as_chunks_mut::<4>().0 {
                        *px = blue_white(*px);
                    }
                }
                image
            })
            .collect();
        flash.0 = copies.into_iter().map(|i| images.add(i)).collect();
    }
    if !*frame_done && let Some(handle) = art.ui.get("hs_frame") {
        if let Some(image) = images.get(handle)
            && image.texture_descriptor.format
                != bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
        {
            *frame_done = true;
        } else if let Some(mut image) = images.get_mut(handle) {
            let (w, h) = (image.width() as usize, image.height() as usize);
            if let Some(data) = image.data.as_mut() {
                clear_row_lines(data, w, h);
            }
            *frame_done = true;
        }
    }
}

/// Clear an RGBA image's inner row lines: rows opaque across most of its
/// width, away from its top and bottom tenths (its border), with two pixels'
/// margin for anti-aliasing each side. Where an upright line (a divider or a
/// side border) runs through, shown by pixels just above and below the
/// cleared band, its pixels stay. Returns the number of lines cleared.
fn clear_row_lines(data: &mut [u8], w: usize, h: usize) -> usize {
    const MARGIN: usize = 2;
    if w == 0 || h == 0 || data.len() != w.saturating_mul(h).saturating_mul(4) {
        return 0;
    }
    let alpha = |data: &[u8], x: usize, y: usize| data[(y * w + x) * 4 + 3];
    let line = |data: &[u8], y: usize| {
        let opaque = (0..w).filter(|&x| alpha(data, x, y) > 128).count();
        opaque * 10 > w * 6
    };
    let mut bands = Vec::new();
    let mut y = h / 10;
    while y < h - h / 10 {
        if line(data, y) {
            let start = y;
            while y + 1 < h - h / 10 && line(data, y + 1) {
                y += 1;
            }
            bands.push((start.saturating_sub(MARGIN), (y + MARGIN).min(h - 1)));
        }
        y += 1;
    }
    for &(top, bottom) in &bands {
        for x in 0..w {
            let above = top.checked_sub(1).map_or(0, |y| alpha(data, x, y));
            let below = if bottom + 1 < h {
                alpha(data, x, bottom + 1)
            } else {
                0
            };
            let keep = if above > 0 && below > 0 {
                above.max(below)
            } else {
                0
            };
            for y in top..=bottom {
                let a = &mut data[(y * w + x) * 4 + 3];
                *a = (*a).min(keep);
            }
        }
    }
    bands.len()
}

/// Spawn the banner's burst (hidden).
pub fn spawn_counter_burst(c: &mut ChildSpawnerCommands) {
    // The banner's counter digit sits near its left end (real game).
    let bw = BANNER_SIZE.0;
    let s = COUNTER_BURST_SIZE;
    let (x, y) = (GRID_CENTER.0 - bw / 2.0 + bw * 0.083, BANNER_Y);
    c.spawn((
        BannerBurst,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x - s / 2.0),
            top: Val::Px(y - s / 2.0),
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

/// Show the banner's burst as the counter reaches 3: the blue-white flash
/// (the pack's orange burst until its copies are made).
pub fn counter_burst(
    fidelity: Res<Fidelity>,
    art: Res<ArtPack>,
    flash: Res<BannerBurstFrames>,
    show: Res<FeatureShow>,
    mut burst: Query<(&mut ImageNode, &mut Visibility), With<BannerBurst>>,
) {
    if !fidelity.0 {
        return;
    }
    let f = show.banner_burst.and_then(|s| {
        let n = counter_burst_frame(show.clock - s)?;
        let frames = if flash.0.is_empty() {
            art.anims.get("burst")?
        } else {
            &flash.0
        };
        frames.get(n).cloned()
    });
    for (mut image, mut vis) in &mut burst {
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
    fn attract_opens_only_at_rest_with_no_credit_and_any_press_restarts_it() {
        // Armed: 29 s idle, then the 30th second opens the chooser.
        let (idle, open) = attract_tick(29.0, 0.5, 30.0, true, false);
        assert!(!open && (idle - 29.5).abs() < 1e-4);
        let (idle, open) = attract_tick(idle, 0.6, 30.0, true, false);
        assert!(open && idle == 0.0);
        // A press, or credit on the meter (not armed), starts it again.
        assert_eq!(attract_tick(29.9, 0.5, 30.0, true, true), (0.0, false));
        assert_eq!(attract_tick(29.9, 0.5, 30.0, false, false), (0.0, false));
    }

    #[test]
    fn the_play_plaque_steps_as_the_cabinet_does() {
        // 1c from the opening; the label moves at 1.2 s, the values at 1.4 s.
        assert_eq!(attract_step(0.0, ATTRACT_LABEL_LAG), 0);
        assert_eq!(attract_step(1.15, ATTRACT_LABEL_LAG), 0);
        assert_eq!(attract_step(1.25, ATTRACT_LABEL_LAG), 1);
        assert_eq!(attract_step(1.25, ATTRACT_VALUE_LAG), 0);
        assert_eq!(attract_step(1.45, ATTRACT_VALUE_LAG), 1);
        // Every 4.0 s after that, wrapping round five club denominations.
        assert_eq!(attract_step(5.25, ATTRACT_LABEL_LAG), 2);
        assert_eq!(attract_step(17.25, ATTRACT_LABEL_LAG) % 5, 0);
        // The flare at each step, gone 0.25 s later.
        assert!(attract_flare(1.1) > 0.99);
        assert_eq!(attract_flare(1.4), 0.0);
        assert!(attract_flare(5.15) > 0.5);
    }

    #[test]
    fn the_chooser_shows_each_denominations_minor_and_mini() {
        let model = dragonlink_model::fixtures::golden_century_synthetic();
        let prizes: Vec<(u64, u64)> = [1u64, 2, 5, 10, 100]
            .iter()
            .map(|&c| {
                (
                    fixed_prize(&model, JackpotTier::Minor, c).unwrap() * c,
                    fixed_prize(&model, JackpotTier::Mini, c).unwrap() * c,
                )
            })
            .collect();
        // Footage: $50/$10, $100/$20, $250/$50, $500/$100, $1,250/$250.
        assert_eq!(
            prizes,
            vec![
                (5_000, 1_000),
                (10_000, 2_000),
                (25_000, 5_000),
                (50_000, 10_000),
                (125_000, 25_000)
            ]
        );
    }

    /// Golden Century with a flag every fourth stop of every strip, base and
    /// free, so flags pass a moving reel all the time.
    fn flag_heavy_reels() -> Reels {
        let mut model = dragonlink_model::fixtures::golden_century_synthetic();
        let flag = model
            .symbols
            .iter()
            .position(|s| s.kind == dragonlink_model::SymbolKind::Scatter)
            .map(|i| dragonlink_model::SymbolId(i as u16))
            .unwrap();
        let fg = model.free_games.as_mut().unwrap();
        for strip in model
            .reel_strips
            .iter_mut()
            .chain(fg.free_reel_strips.iter_mut())
        {
            for (i, s) in strip.iter_mut().enumerate() {
                if i % 4 == 0 {
                    *s = flag;
                }
            }
        }
        reel_strips(&model)
    }

    /// The flags a reel shows with its top at `pos`: every strip index in
    /// view (partly or wholly).
    fn flags_in_view(reels: &Reels, reel: usize, pos: f32) -> usize {
        let first = pos.floor() as i64;
        let last = (pos + 3.0).ceil() as i64 - 1;
        (first..=last)
            .filter(|n| symbol_of(reels.key_at(reel, *n)) == reels.scatter)
            .count()
    }

    #[test]
    fn a_moving_reel_never_shows_two_flags() {
        let mut reels = flag_heavy_reels();
        let flag = reels.scatter.clone();
        for free in [false, true] {
            for reel in 0..5 {
                for row in 0..3 {
                    let mut window = vec!["K".to_string(); 3];
                    window[row] = flag.clone();
                    // A spin starting from a window with a flag: it scrolls
                    // away (spin_reels marks the reel quiet).
                    let at = 40i64;
                    {
                        let s = &mut reels.states[reel];
                        s.free = free;
                        s.quiet = true;
                        s.landing = window.clone();
                        s.landing_at = at;
                    }
                    let mut pos = at as f32;
                    while pos >= at as f32 - 5.0 {
                        assert!(
                            flags_in_view(&reels, reel, pos) <= 1,
                            "start, reel {reel} row {row} pos {pos}"
                        );
                        pos -= 0.02;
                    }
                    // Landing on a window with a flag: it runs on past its
                    // stop and back.
                    let target = 20i64;
                    reels.states[reel].landing_at = target;
                    let mut pos = target as f32 + 4.0;
                    while pos >= target as f32 - DEEP {
                        assert!(
                            flags_in_view(&reels, reel, pos) <= 1,
                            "landing, reel {reel} row {row} pos {pos}"
                        );
                        pos -= 0.02;
                    }
                }
            }
        }
    }

    #[test]
    fn a_reel_without_a_flag_still_passes_flags() {
        let mut reels = flag_heavy_reels();
        let s = &mut reels.states[0];
        s.quiet = false;
        s.landing = vec!["K".to_string(); 3];
        s.landing_at = 1000;
        let flags = (0..40)
            .filter(|n| reels.key_at(0, *n) == reels.scatter)
            .count();
        assert!(flags >= 8, "{flags}");
    }

    #[test]
    fn the_rules_pages_turn_both_ways_round() {
        assert_eq!(turn_page(0, 8, 1), 1);
        assert_eq!(turn_page(7, 8, 1), 0);
        assert_eq!(turn_page(0, 8, -1), 7);
        assert_eq!(turn_page(3, 0, 1), 0);
    }

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
                let size = jackpot_value_size(text, width, JACKPOT_VALUE_SIZE);
                assert!(size <= JACKPOT_VALUE_SIZE);
                assert!(
                    meter_ems(text) * size <= width * JACKPOT_VALUE_FILL + 0.01,
                    "{tier:?} {text}"
                );
            }
        }
        // A value too wide for its box is set smaller, to fit.
        let mini = (1209.0 - 975.0) * k;
        let size = jackpot_value_size("$1,234,567.89", mini, JACKPOT_VALUE_SIZE);
        assert!(size < JACKPOT_VALUE_SIZE);
        assert!(meter_ems("$1,234,567.89") * size <= mini * JACKPOT_VALUE_FILL + 0.01);
        // The values the bar shows today keep the full size.
        assert_eq!(
            jackpot_value_size("$250.00", (1209.0 - 975.0) * k, JACKPOT_VALUE_SIZE),
            JACKPOT_VALUE_SIZE
        );
        assert_eq!(
            jackpot_value_size("$15,000.00", (351.0 - 14.0) * k, JACKPOT_VALUE_SIZE),
            JACKPOT_VALUE_SIZE
        );
    }

    /// Each level's value has its own hue, as on the cabinet and in the app:
    /// the Grand red, the Major green, the Minor blue, the Mini orange.
    #[test]
    fn each_jackpot_value_has_its_levels_colour() {
        let hue = |tier| {
            let c = jackpot_value_colour(tier).to_srgba();
            (c.red, c.green, c.blue)
        };
        let (r, g, b) = hue(JackpotTier::Grand);
        assert!(r > 0.8 && g < 0.3 && b < 0.3);
        let (r, g, b) = hue(JackpotTier::Major);
        assert!(g > 0.8 && r < 0.3 && b < 0.6);
        let (r, g, b) = hue(JackpotTier::Minor);
        assert!(b > 0.8 && r < 0.3 && g < 0.6);
        let (r, g, b) = hue(JackpotTier::Mini);
        assert!(r > 0.8 && g > 0.3 && g < 0.6 && b < 0.4);
    }

    /// After a denomination choice the club meters alternate dollars and
    /// credits each second until the first game, the clock counting from
    /// the YES (club footage); the casino style never switches.
    #[test]
    fn club_meters_alternate_from_a_denomination_choice_until_the_first_game() {
        let mut deck = DeckState {
            profile: Profile::Club,
            mode: DeckMode::Base,
            selecting: true,
            ..Default::default()
        };
        // The game returns 0.3 s after the YES: dollars until 1.0 s after
        // it, then credits for a second, dollars for a second, and so on.
        deck.since = deck_clock_restart(DeckMode::Menu, true);
        assert!((deck.since - MENU_CLOSE).abs() < 1e-6);
        for (since, credits) in [(0.3, false), (0.95, false), (1.05, true), (1.95, true)] {
            deck.since = since;
            assert_eq!(deck.in_credits(), credits, "at {since} s");
        }
        for (since, credits) in [(2.05, false), (3.5, true), (4.2, false)] {
            deck.since = since;
            assert_eq!(deck.in_credits(), credits, "at {since} s");
        }
        // With no choice pending, base idle stays in dollars.
        deck.selecting = false;
        deck.since = 1.5;
        assert!(!deck.in_credits());
        // A win offered and the gamble still switch.
        deck.offer = true;
        assert!(deck.in_credits());
        // The casino style never switches.
        let casino = DeckState {
            profile: Profile::Casino,
            selecting: true,
            ..deck
        };
        assert!(!casino.in_credits());
        // Every other restart counts from nothing.
        assert_eq!(deck_clock_restart(DeckMode::Menu, false), 0.0);
        assert_eq!(deck_clock_restart(DeckMode::Spinning, true), 0.0);
        assert_eq!(deck_clock_restart(DeckMode::Base, true), 0.0);
    }

    /// In a credits phase BET shows the last game's bet in credits of the
    /// denomination now played (club footage: "BET 50" against "$1.00" at
    /// 2c), in dollars otherwise.
    #[test]
    fn bet_shows_the_last_bet_in_credits_of_the_new_denomination() {
        assert_eq!(bet_meter(Some(100), true, 1, 2), "50");
        assert_eq!(bet_meter(Some(100), false, 1, 2), "$1.00");
        assert_eq!(bet_meter(Some(1_234_500), true, 1, 1), "1,234,500");
        // No change pending: the bet itself.
        assert_eq!(bet_meter(None, true, 50, 2), "50");
        assert_eq!(bet_meter(None, false, 50, 2), "$1.00");
    }

    /// A club deck at 1c, 1 credit a line on 10 lines (credits keys 1, 2, 3,
    /// 5, 10; lines keys 1, 5, 10, 30, 50).
    fn club_look(mode: DeckMode) -> DeckLook {
        DeckLook {
            deck: DeckState {
                mode,
                profile: Profile::Club,
                cents: 1,
                ..Default::default()
            },
            idle: true,
            suits_open: true,
            lines: 10,
            bet: 1,
            ..Default::default()
        }
    }

    fn key(row: u8, col: u8) -> CabButton {
        CabButton::Slot { row, col }
    }

    /// Club footage: GOOD LUCK on every Hold & Spin respin, the teardrop
    /// keeping SPIN on its dark face.
    #[test]
    fn a_hold_and_spin_respin_spins_the_deck() {
        assert_eq!(deck_mode(false, false, false, true), DeckMode::Spinning);
        assert_eq!(deck_mode(false, false, true, false), DeckMode::Spinning);
        assert_eq!(deck_mode(false, false, false, false), DeckMode::Base);
        assert_eq!(deck_mode(false, true, false, true), DeckMode::Gamble);
        assert_eq!(deck_mode(true, false, true, false), DeckMode::Menu);
        let mut look = club_look(DeckMode::Spinning);
        look.hold_and_spin = true;
        look.deck.since = 0.6;
        assert_eq!(look.shown(key(0, 1)), Role::Letter('G'));
        assert_eq!(look.shown(key(1, 4)), Role::Letter('K'));
        assert_eq!(look.words(CabButton::Spin)[0].0, "SPIN");
        assert_eq!(look.face(CabButton::Spin), "spin_dim");
    }

    /// Club footage, four wipes: GAMBLE and TAKE WIN go dark first; every
    /// other key keeps its idle face and legend until its letter arrives, a
    /// letter every 0.1 s, the bottom row a frame behind, the right column
    /// dark 0.1 s after the D.
    #[test]
    fn good_luck_wipes_in_over_the_idle_faces() {
        let mut look = club_look(DeckMode::Spinning);
        look.deck.since = 0.0;
        assert_eq!(look.shown(key(0, 0)), Role::Off);
        assert_eq!(look.shown(key(1, 0)), Role::Off);
        assert_eq!(look.shown(key(0, 1)), Role::Credits(1));
        assert_eq!(look.face(key(0, 1)), "key_pale");
        assert_eq!(look.shown(key(1, 3)), Role::Lines(10));
        assert!(!look.words(key(1, 3)).is_empty());
        look.deck.since = 0.105;
        assert_eq!(look.shown(key(0, 1)), Role::Letter('G'));
        assert_eq!(look.face(key(0, 1)), "key_dark");
        assert_eq!(look.shown(key(1, 1)), Role::Lines(1));
        assert_eq!(look.shown(key(0, 2)), Role::Credits(2));
        look.deck.since = 0.14;
        assert_eq!(look.shown(key(1, 1)), Role::Letter('L'));
        look.deck.since = 0.41;
        assert_eq!(look.shown(key(0, 4)), Role::Letter('D'));
        assert_eq!(look.shown(key(0, 5)), Role::Credits(10));
        look.deck.since = 0.51;
        assert_eq!(look.shown(key(0, 5)), Role::Off);
        assert_eq!(look.shown(key(1, 5)), Role::Lines(50));
        look.deck.since = 0.54;
        assert_eq!(look.shown(key(1, 5)), Role::Off);
    }

    /// Club footage, two stops: the idle faces come back from the left, a
    /// column every 0.08 s, on the mode's own clock (the offer does not
    /// restart it).
    #[test]
    fn the_idle_faces_wipe_back_from_the_left_as_the_reels_stop() {
        let mut look = club_look(DeckMode::Base);
        look.deck.prev = DeckMode::Spinning;
        look.deck.mode_t = 0.0;
        assert_eq!(look.shown(key(0, 0)), Role::Gamble);
        assert_eq!(look.shown(key(0, 1)), Role::Letter('G'));
        assert_eq!(look.shown(key(1, 5)), Role::Off);
        look.deck.mode_t = 0.17;
        assert_eq!(look.shown(key(0, 2)), Role::Credits(2));
        assert_eq!(look.shown(key(1, 3)), Role::Letter('C'));
        look.deck.offer = true;
        look.deck.since = 0.0;
        assert_eq!(look.shown(key(1, 3)), Role::Letter('C'));
        look.deck.mode_t = 0.41;
        assert_eq!(look.shown(key(1, 3)), Role::Lines(10));
        assert_eq!(look.shown(key(1, 5)), Role::Lines(50));
        // After the gamble or the menu nothing wipes.
        look.deck.prev = DeckMode::Gamble;
        look.deck.mode_t = 0.0;
        assert_eq!(look.shown(key(1, 3)), Role::Lines(10));
    }

    /// Club footage: a dark smoky teardrop with a pale-blue legend at rest
    /// and while the reels spin, lit steadily for START FEATURE with the
    /// legend turning like a coin; the casino's deck B blinks.
    #[test]
    fn the_teardrop_is_dark_but_for_start_feature() {
        let mut look = club_look(DeckMode::Base);
        assert_eq!(look.face(CabButton::Spin), "spin_dim");
        assert_eq!(look.words(CabButton::Spin)[0].0, "PLAY");
        let pale = look.ink(CabButton::Spin).to_srgba();
        assert!(pale.blue > 0.9 && pale.red < 0.6);
        look.deck.mode = DeckMode::Spinning;
        assert_eq!(look.face(CabButton::Spin), "spin_dim");
        look.deck.mode = DeckMode::Base;
        look.start_wait = true;
        for wait in [0.1, 0.7, 1.6, 2.3] {
            look.deck.wait_t = wait;
            assert_eq!(look.face(CabButton::Spin), "spin_lit", "at {wait} s");
        }
        assert_eq!(look.ink(CabButton::Spin), Color::WHITE);
        assert_eq!(look.words(CabButton::Spin)[0].0, "START");
        look.deck.wait_t = 0.5;
        assert_eq!(look.legend_turn(), 1.0);
        look.deck.wait_t = 1.13;
        assert!(look.legend_turn() < 0.05);
        look.deck.wait_t = 1.27;
        assert!(look.legend_turn() > 0.9);
        look.deck.profile = Profile::Casino;
        look.deck.wait_t = 0.2;
        assert_eq!(look.face(CabButton::Spin), "spin_lit");
        look.deck.wait_t = 0.7;
        assert_eq!(look.face(CabButton::Spin), "spin_dim");
        look.deck.wait_t = 1.13;
        assert_eq!(look.legend_turn(), 1.0);
        // The legend sits on the ring, 0.55 of the way down (cabinet
        // 0.52-0.57 from the apex).
        assert!((0.52..=0.57).contains(&((1.0 + TEARDROP_LEGEND_PAD) / 2.0)));
    }

    /// Club footage: every key lit at full brightness; GAMBLE purple and
    /// TAKE WIN pale khaki with blue letters with no gamble offered; the
    /// selected lines key light blue with white ink, the selected credits key
    /// pale with a pink-red numeral, unoffered credits keys lit orange.
    #[test]
    fn every_key_is_lit_with_the_cabinets_faces() {
        let look = club_look(DeckMode::Base);
        let pack = |k: &str| {
            matches!(
                k,
                "key_white" | "key_blue" | "key_red" | "key_purple" | "key_dark" | "card_back"
            )
        };
        assert!(!look.deck.offer);
        assert_eq!(
            resolve_face(look.face(key(0, 0)), pack),
            ("key_purple", Color::WHITE)
        );
        let (image, tint) = resolve_face(look.face(key(1, 0)), pack);
        assert_eq!(image, "key_white");
        let khaki = tint.to_srgba();
        assert!(khaki.red > khaki.green && khaki.green > khaki.blue && khaki.blue > 0.5);
        let ink = look.ink(key(1, 0)).to_srgba();
        assert!(ink.blue > 0.9 && ink.red < 0.3);
        assert_eq!(look.face(key(1, 3)), "key_lightblue");
        assert_eq!(look.ink(key(1, 3)), Color::WHITE);
        let (image, tint) = resolve_face("key_lightblue", pack);
        assert_eq!(image, "key_white");
        assert!(tint.to_srgba().red < 0.7);
        assert_eq!(
            resolve_face("key_lightblue", |_| true),
            ("key_lightblue", Color::WHITE)
        );
        assert_eq!(look.face(key(0, 1)), "key_pale");
        let numeral = look.ink(key(0, 1)).to_srgba();
        assert!(numeral.red > 0.8 && numeral.green < 0.4);
        assert_eq!(look.face(key(0, 2)), "key_red");
        assert!(look.ink(key(0, 2)).to_srgba().blue > 0.9);
        let mut dollar = look;
        dollar.deck.cents = 100;
        assert_eq!(dollar.shown(key(0, 5)), Role::Blank);
        assert_eq!(
            resolve_face(dollar.face(key(0, 5)), pack),
            ("key_red", Color::WHITE)
        );
        assert!(dollar.words(key(0, 5)).is_empty());
    }

    /// Club footage: on the flash's plain half the pressable keys turn light
    /// blue (the gamble's red choices lavender) with faint legends; on the
    /// vivid half the gamble's red choices are orange and the black ones
    /// dark, light-blue icons on both; a chosen key is steady light blue;
    /// suits past the limit stay dark and still.
    #[test]
    fn the_flash_shows_light_blue_faces_with_faint_legends() {
        let mut look = club_look(DeckMode::Base);
        look.deck.offer = true;
        look.deck.since = 0.2;
        assert_eq!(look.face(key(1, 2)), "key_blue");
        look.deck.since = 0.7;
        assert_eq!(look.face(key(1, 2)), "key_lightblue");
        assert_eq!(look.face(key(0, 0)), "key_lightblue");
        assert_eq!(look.face(key(1, 0)), "key_lightblue");
        assert!(!look.words(key(1, 2)).is_empty());
        assert!((look.ink(key(1, 2)).alpha() - PLAIN_LEGEND_ALPHA).abs() < 1e-6);
        assert_eq!(look.face(key(0, 3)), "key_red");
        let mut g = club_look(DeckMode::Gamble);
        let (hearts, clubs, black) = (key(0, 1), key(0, 4), key(1, 5));
        g.deck.since = 0.2;
        assert_eq!(g.face(hearts), "key_red");
        assert_eq!(g.face(clubs), "key_dark");
        assert_eq!(g.face(black), "key_dark");
        let icon = g.ink(clubs).to_srgba();
        assert!(icon.blue > 0.9 && icon.red > 0.5);
        g.deck.since = 0.7;
        assert_eq!(g.face(hearts), "key_lavender");
        assert_eq!(g.face(clubs), "key_lightblue");
        assert_eq!(g.face(key(0, 3)), "key_lattice");
        g.chosen = Some(GamblePick::Clubs);
        assert_eq!(g.face(clubs), "key_lightblue");
        assert_eq!(g.face(hearts), "key_red");
        g.chosen = None;
        g.suits_open = false;
        assert_eq!(g.face(hearts), "key_dark");
        assert!(g.words(hearts).is_empty());
        assert_eq!(g.face(key(1, 1)), "key_lavender");
        // A pack without the lattice key shows the card back.
        assert_eq!(
            resolve_face("key_lattice", |k| k == "card_back"),
            ("card_back", Color::WHITE)
        );
    }

    /// Cabinet footage: keys about a tenth of a key apart, the teardrop
    /// about 1.7 keys wide a quarter of a key clear of them.
    #[test]
    fn the_deck_keys_and_teardrop_are_spaced_as_on_the_cabinet() {
        let (kw, _, gx, _) = DECK_KEY;
        assert!((0.09..=0.14).contains(&(gx / kw)));
        let shown = TEARDROP_VISIBLE * DECK_TEARDROP.0 / kw;
        assert!((1.6..=1.75).contains(&shown));
        assert!((0.2..=0.4).contains(&TEARDROP_GAP));
        // Its node starts right of the keys.
        assert!(TEARDROP_GAP * kw >= TEARDROP_INSET * DECK_TEARDROP.0);
    }

    /// Step a system message `secs` seconds in frames of 0.05 s.
    fn slot_run(slot: &mut SystemSlot, secs: f32, offer: bool, credit: u64) {
        let frames = (secs / 0.05).round() as u32;
        for _ in 0..frames {
            slot.step(SlotFrame {
                dt: 0.05,
                start: None,
                offer,
                credit,
            });
        }
    }

    /// A game starting, the credit `before` its bet and `credit` after.
    fn slot_start(slot: &mut SystemSlot, before: u64, credit: u64) {
        slot.step(SlotFrame {
            dt: 0.05,
            start: Some(before),
            offer: false,
            credit,
        });
    }

    /// The system messages' texts, as club footage writes them: the limits
    /// in plain dollars with no grouping, the greeting a substitute.
    #[test]
    fn the_system_messages_read_as_on_the_club_machines() {
        assert_eq!(plain_dollars(GAMBLE_WIN_LIMIT_CENTS), "$10000.00");
        assert_eq!(
            SystemText::GambleNotice.text(),
            "Gamble not offered if gamble win will exceed $10000.00"
        );
        assert_eq!(SystemText::WinLimit.text(), "Gamble win limit is $10000.00");
        assert_eq!(
            SystemText::CashIn.text(),
            "Credit reached $100.00, cash-in disabled"
        );
        assert_eq!(SystemText::Greeting.text(), "Welcome");
    }

    /// Below the cash-in limit a start changes nothing; the message
    /// switches between the gamble notice and the greeting two seconds
    /// after each start, and every two seconds with no game.
    #[test]
    fn below_the_cash_in_limit_the_message_switches_two_seconds_after_a_start() {
        let mut slot = SystemSlot::default();
        slot_run(&mut slot, 0.1, false, 5_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_start(&mut slot, 5_050, 5_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_run(&mut slot, 1.9, false, 5_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_run(&mut slot, 0.2, false, 5_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
        // The next start leaves it; two seconds on it switches back.
        slot_start(&mut slot, 5_000, 4_950);
        assert_eq!(slot.showing(), SystemText::Greeting);
        slot_run(&mut slot, 2.1, false, 4_950);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        // No game: every two seconds (HYPOTHESIS).
        slot_run(&mut slot, 2.0, false, 4_950);
        assert_eq!(slot.showing(), SystemText::Greeting);
    }

    /// Credit loaded to the cash-in limit shows the cash-in notice at the
    /// next switch, 1.2 s, then the greeting; at or above the limit a start
    /// brings the greeting and the gamble notice follows two seconds later.
    #[test]
    fn at_the_cash_in_limit_a_start_brings_the_greeting() {
        let mut slot = SystemSlot::default();
        // The session's credit, loaded at $100.00.
        slot_run(&mut slot, 1.9, false, 10_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_run(&mut slot, 0.2, false, 10_000);
        assert_eq!(slot.showing(), SystemText::CashIn);
        slot_run(&mut slot, 1.1, false, 10_000);
        assert_eq!(slot.showing(), SystemText::CashIn);
        slot_run(&mut slot, 0.2, false, 10_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
        // Once only: the next switch is the gamble notice.
        slot_run(&mut slot, 2.1, false, 10_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        // A start from $100.00 or more: the greeting, then the notice.
        slot_start(&mut slot, 10_000, 9_950);
        assert_eq!(slot.showing(), SystemText::Greeting);
        slot_run(&mut slot, 2.1, false, 9_950);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        // The same with the credit still at the limit (a free game's start).
        let mut slot = SystemSlot::default();
        slot_run(&mut slot, 3.4, false, 10_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
        slot_start(&mut slot, 10_000, 10_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
        slot_run(&mut slot, 2.1, false, 10_000);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_start(&mut slot, 10_000, 10_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
    }

    /// A base game's win offered shows the win limit until the next start,
    /// which brings the gamble notice; a win paid to the cash-in limit
    /// shows the cash-in notice two seconds after the start, and a start
    /// that cuts it off leaves it due.
    #[test]
    fn a_win_offered_shows_the_win_limit_and_a_win_paid_the_cash_in_notice() {
        let mut slot = SystemSlot::default();
        slot_run(&mut slot, 0.1, false, 9_000);
        slot_run(&mut slot, 3.0, true, 9_000);
        assert_eq!(slot.showing(), SystemText::WinLimit);
        // PLAY ON: the win paid takes CREDIT to $100.40 as the game starts.
        slot_start(&mut slot, 10_450, 10_400);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_run(&mut slot, 2.1, false, 10_400);
        assert_eq!(slot.showing(), SystemText::CashIn);
        // Cut off by the next start: the gamble notice, the cash-in notice
        // again at the next switch, then the greeting.
        slot_start(&mut slot, 10_400, 10_350);
        assert_eq!(slot.showing(), SystemText::GambleNotice);
        slot_run(&mut slot, 2.1, false, 10_350);
        assert_eq!(slot.showing(), SystemText::CashIn);
        slot_run(&mut slot, 1.3, false, 10_350);
        assert_eq!(slot.showing(), SystemText::Greeting);
    }

    /// Into the gamble from the offer, the win limit is not forced: the
    /// message carries on switching two seconds after the offer ends.
    #[test]
    fn the_win_limit_gives_way_in_the_gamble() {
        let mut slot = SystemSlot::default();
        slot_run(&mut slot, 0.1, false, 5_000);
        slot_run(&mut slot, 1.0, true, 5_000);
        assert_eq!(slot.showing(), SystemText::WinLimit);
        slot_run(&mut slot, 1.9, false, 5_000);
        assert_eq!(slot.showing(), SystemText::WinLimit);
        slot_run(&mut slot, 0.2, false, 5_000);
        assert_eq!(slot.showing(), SystemText::Greeting);
    }

    /// A line win's message is always in credits, in plain digits (cabinet
    /// footage: "Line 13 wins 20", "Line 4 wins 100"; never dollars).
    #[test]
    fn a_line_win_is_given_in_credits() {
        assert_eq!(line_win_note(12, 20), "Line 13 wins 20");
        assert_eq!(line_win_note(3, 100), "Line 4 wins 100");
        assert_eq!(line_win_note(0, 1000), "Line 1 wins 1000");
        // A free game's x2 wilds (Genghis Khan club cabinets).
        assert_eq!(line_win_note_times(7, 100, 2), "Line 8 wins 50 x 2 = 100");
        assert_eq!(line_win_note_times(3, 200, 4), "Line 4 wins 50 x 4 = 200");
        assert_eq!(line_win_note_times(18, 20, 4), "Line 19 wins 5 x 4 = 20");
        assert_eq!(line_win_note_times(0, 40, 1), "Line 1 wins 40");
    }

    /// The line pieces follow the denomination's line configuration (50 at
    /// 1c and 2c, 25 at 5c and 10c, 5 at $1), in the sizes measured on club
    /// footage: a small badge at 50 lines, a wide one at 25 and 5.
    #[test]
    fn line_pieces_are_sized_to_the_denominations_line_configuration() {
        assert_eq!(line_config(1), 50);
        assert_eq!(line_config(2), 50);
        assert_eq!(line_config(5), 25);
        assert_eq!(line_config(10), 25);
        assert_eq!(line_config(100), 5);
        let fifty = line_look(50);
        assert_eq!(fifty.badge, (39.0, 27.0));
        assert_eq!(fifty.number, 20.0);
        assert_eq!(fifty.dash, (36.0, 14.0));
        assert!(fifty.own_digits);
        let wide = line_look(25);
        assert_eq!(wide.badge, (102.0, 52.0));
        assert_eq!(wide.number, 38.0);
        assert_eq!(wide.dash, (52.0, 26.0));
        assert!(!wide.own_digits);
        assert_eq!(line_look(5), wide);
        // The badge's visible part touches the frame's outer gold from
        // outside: x 142 on the left, 1771 on the right.
        for look in [fifty, wide] {
            let half = look.badge.0 * LINE_BADGE_VISIBLE / 2.0;
            assert!((line_badge_x(look.badge.0, false) + half - 142.0).abs() < 1e-3);
            assert!((line_badge_x(look.badge.0, true) - half - 1771.0).abs() < 1e-3);
        }
        // 50 lines: centres about 123 and 1790; 25 and 5: about 93 and 1820.
        assert!((line_badge_x(39.0, false) - 123.0).abs() < 1.0);
        assert!((line_badge_x(39.0, true) - 1790.0).abs() < 1.0);
        assert!((line_badge_x(102.0, false) - 93.0).abs() < 1.0);
        assert!((line_badge_x(102.0, true) - 1820.0).abs() < 1.0);
    }

    /// One badge per line at 50 and 25 lines, on the side footage shows for
    /// each filmed line; both ends at 5 lines, and for line 1 at 25.
    #[test]
    fn a_line_has_one_badge_at_50_and_25_lines_on_its_filmed_side() {
        for line in SIDE_50_LEFT {
            assert_eq!(line_badge_ends(50, line), (true, false), "line {line}");
        }
        for line in SIDE_50_RIGHT {
            assert_eq!(line_badge_ends(50, line), (false, true), "line {line}");
        }
        for line in SIDE_25_LEFT {
            assert_eq!(line_badge_ends(25, line), (true, false), "line {line}");
        }
        for line in SIDE_25_RIGHT {
            assert_eq!(line_badge_ends(25, line), (false, true), "line {line}");
        }
        // The filmed cases named in the evidence.
        assert_eq!(line_badge_ends(50, 23), (true, false));
        assert_eq!(line_badge_ends(50, 28), (false, true));
        assert_eq!(line_badge_ends(25, 13), (false, true));
        // Line 1: one badge at 50 lines, both at 25.
        assert_eq!(line_badge_ends(50, 1), (true, false));
        assert_eq!(line_badge_ends(25, 1), (true, true));
        // Exactly one end for every other line, filmed or not.
        for line in 1..=50 {
            let (l, r) = line_badge_ends(50, line);
            assert!(l != r, "50 lines, line {line}");
        }
        for line in 2..=25 {
            let (l, r) = line_badge_ends(25, line);
            assert!(l != r, "25 lines, line {line}");
        }
        // 5 lines: both ends, every line.
        for line in 1..=5 {
            assert_eq!(line_badge_ends(5, line), (true, true));
        }
        // No line is on both lists of a configuration.
        assert!(SIDE_50_LEFT.iter().all(|l| !SIDE_50_RIGHT.contains(l)));
        assert!(SIDE_25_LEFT.iter().all(|l| !SIDE_25_RIGHT.contains(l)));
    }

    /// The wide badges are the blank badge, sliced so its rounded ends keep
    /// their shape; the small one is the art stretched, with its own digit.
    #[test]
    fn wide_line_badges_are_sliced() {
        match line_badge_mode(&line_look(25), Some(44.0)) {
            NodeImageMode::Sliced(s) => {
                assert_eq!(s.border, BorderRect::axes(14.0, 0.0));
                assert!(s.max_corner_scale > 2.0);
            }
            other => panic!("expected sliced, got {other:?}"),
        }
        assert_eq!(
            line_badge_mode(&line_look(50), Some(44.0)),
            NodeImageMode::Stretch
        );
        assert_eq!(
            line_badge_mode(&line_look(25), None),
            NodeImageMode::Stretch
        );
    }

    /// The club's lines panels: large, from y 407 to 611 about the middle
    /// row, their inner edges under the reel frame (outer gold at x 141 and
    /// 1772), outlined out to x 12 and 1902.
    #[test]
    fn the_lines_panels_run_under_the_reel_frame() {
        let [(l0, w0), (l1, w1)] = LINES_PANELS;
        assert_eq!(l0 - 2.0, 12.0);
        assert!(l0 + w0 > 141.0 + 5.0);
        assert!(l1 < 1772.0 - 5.0);
        assert_eq!(l1 + w1 + 2.0, 1902.0);
        assert_eq!(LINES_PANEL_TOP + LINES_PANEL_H / 2.0, 509.0);
        // The text sits on the panels' visible parts.
        assert!(LINES_TEXT_X[0] > l0 && LINES_TEXT_X[0] < 141.0);
        assert!(LINES_TEXT_X[1] > 1772.0 && LINES_TEXT_X[1] < l1 + w1);
        assert!(
            LINES_TEXT_Y.0 > LINES_PANEL_TOP && LINES_TEXT_Y.1 < LINES_PANEL_TOP + LINES_PANEL_H
        );
        // The numerals' gold (31.5 px of each 45 px image) comes out at the
        // cabinet's 47 px.
        assert!((LINES_DIGIT_H * 31.5 / 45.0 - 47.0).abs() < 1e-3);
        assert_eq!(lines_text(50), ("50".to_string(), "LINES"));
        assert_eq!(lines_text(1), ("1".to_string(), "LINE"));
    }

    /// The panel fill is the lines-box art's fill, cropped the same way at
    /// any size.
    #[test]
    fn the_lines_panel_fill_is_cropped_in_proportion() {
        assert_eq!(
            lines_panel_crop((111.0, 225.0)),
            Rect::new(14.0, 14.0, 78.0, 211.0)
        );
        let small = lines_panel_crop((67.0, 137.0));
        assert!((small.min.x - 8.45).abs() < 0.01 && (small.max.y - 128.48).abs() < 0.01);
    }

    /// A reel panel whose art is the frame alone stays put; one that also
    /// carries the lines tabs is drawn wider so its frame lands where every
    /// other title's frame lands.
    #[test]
    fn a_reel_panel_with_its_own_lines_tabs_is_widened_to_fit_the_reels() {
        let (left0, width0) = (960.0 - 620.0 * PANEL_SCALE.0, 1240.0 * PANEL_SCALE.0);
        // Golden Century's frame-only art, and the later titles'.
        assert_eq!(reel_panel_fit(749.0, 2.0, 746.0, left0, width0), None);
        assert_eq!(reel_panel_fit(1229.0, 3.0, 1224.0, left0, width0), None);
        // Golden Gong's: 1369 wide, the frame opaque from 70 to 1298.
        let (left, width) = reel_panel_fit(1369.0, 70.0, 1298.0, left0, width0).unwrap();
        let scale = width / 1369.0;
        let frame_left = left + 70.0 * scale;
        let frame_right = left + 1299.0 * scale;
        assert!((frame_left - (left0 + width0 * PANEL_FRAME.0)).abs() < 0.01);
        assert!((frame_right - (left0 + width0 * PANEL_FRAME.1)).abs() < 0.01);
        // Its tabs stand outside the frame, out to x 48 or so.
        assert!(left > 40.0 && left < 60.0, "left {left}");
        // Degenerate input changes nothing.
        assert_eq!(reel_panel_fit(0.0, 0.0, 0.0, left0, width0), None);
        assert_eq!(reel_panel_fit(100.0, 50.0, 40.0, left0, width0), None);
    }

    /// The frame is found across a row above the tabs: in a synthetic panel
    /// with tabs lower down, the row at 15% of the height spans the frame.
    #[test]
    fn the_frame_span_is_read_above_the_tabs() {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let (w, h) = (40u32, 20u32);
        let mut image = Image::new_fill(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        for y in 0..h {
            // The frame from x 6 to 33; tabs out to the edges from y 8 to 12.
            let (a, b) = if (8..=12).contains(&y) {
                (0, w - 1)
            } else {
                (6, 33)
            };
            for x in a..=b {
                image.set_color_at(x, y, Color::WHITE).unwrap();
            }
        }
        assert_eq!(frame_span(&image), Some((6, 33)));
        let empty = Image::new_fill(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        assert_eq!(frame_span(&empty), None);
    }

    /// At rest the status line shows the bet line for two seconds, then
    /// "Game Over - Play Now" for two, counted from the end of the game or
    /// the last bet change; a debit flashes "Game Over - Play Now" briefly.
    #[test]
    fn the_idle_status_line_counts_from_rest() {
        let bet = (2, 50, 1);
        let mut rest = RestPhase::default();
        rest.update(0.0, false, bet, 7);
        rest.update(10.3, false, bet, 7);
        // The game ends at 10.5 s: the bet line first, whatever the clock.
        rest.update(10.5, true, bet, 7);
        assert!(rest.bet_line_turn(10.5));
        assert!(rest.bet_line_turn(12.4));
        assert!(!rest.bet_line_turn(12.6));
        assert!(!rest.bet_line_turn(14.4));
        assert!(rest.bet_line_turn(14.6));
        // A credits-per-line key restarts the cycle.
        rest.update(13.0, true, (3, 50, 1), 7);
        assert!(rest.bet_line_turn(13.0));
        assert!(rest.bet_line_turn(14.9));
        assert!(!rest.bet_line_turn(15.1));
        // So does a denomination change.
        rest.update(16.0, true, (3, 50, 2), 7);
        assert!(rest.bet_line_turn(17.5));
        // Staying at rest with the same bet keeps the phase.
        rest.update(18.5, true, (3, 50, 2), 7);
        assert!(!rest.bet_line_turn(18.5));
        assert!(!rest.flashing(18.5));
        // A game is debited: the flash, for a tenth of a second.
        rest.update(19.0, false, (3, 50, 2), 8);
        assert!(rest.flashing(19.0));
        assert!(rest.flashing(19.09));
        assert!(!rest.flashing(19.11));
        // The first frame seen raises no flash.
        let mut fresh = RestPhase::default();
        fresh.update(1.0, true, bet, 40);
        assert!(!fresh.flashing(1.0));
    }

    /// The Hold & Spin intro as two club recordings show it: one fire, the
    /// board coming up under its white, the prompt 3.5 s after the burst
    /// (y8c8lEdPW8Y 965.60-969.10, 1bAevGz0K2k 110.17-113.67).
    #[test]
    fn the_hold_and_spin_intro_is_one_fire_into_the_board() {
        // The coins burst 2.67-2.70 s after the reels are still (the sequence
        // starts 0.17 s after).
        assert!((2.66..=2.71).contains(&(INTRO_BURST + 0.17)));
        // The fire's key frames, seconds after the burst, against the cabinet.
        let frame_at = |n: f32| INTRO_TRANSITION + n / FIRE_FPS - INTRO_BURST;
        assert!((0.88..=0.95).contains(&frame_at(1.0)), "first dragon head");
        assert!((1.70..=1.86).contains(&frame_at(20.0)), "full cover");
        assert!((2.54..=2.63).contains(&frame_at(38.0)), "lower half clear");
        assert!((2.67..=2.75).contains(&frame_at(41.0)), "top band only");
        // The board and its backdrop come up under the full white (22-33).
        let grid = (INTRO_GRID - INTRO_TRANSITION) * FIRE_FPS;
        assert!((22.0..=33.0).contains(&grid), "{grid}");
        // The prompt 3.50 s after the burst, the fire's embers gone.
        assert!((INTRO_END - INTRO_BURST - 3.5).abs() < 1e-4);
        assert!(frame_at(49.0) < INTRO_END - INTRO_BURST);
    }

    /// PRESS START FEATURE BUTTON's lettering blinks on a 0.667 s cycle,
    /// about 0.35 s lit, lit for about 0.25 s as it first shows, phased from
    /// the prompt's start, for both features (club footage).
    #[test]
    fn the_start_prompt_lettering_blinks_from_its_start() {
        assert!(prompt_lit(0.0) && prompt_lit(0.24));
        assert!(!prompt_lit(0.26) && !prompt_lit(0.56));
        assert!(prompt_lit(0.58) && prompt_lit(0.90));
        assert!(!prompt_lit(0.94) && prompt_lit(1.25));
        // Lit a little over half the time.
        let lit = (0..6000).filter(|k| prompt_lit(*k as f32 / 1000.0)).count();
        assert!((3100..=3200).contains(&lit), "{lit}");

        let mut show = FeatureShow::default();
        assert_eq!(show.prompt_lettering(), None);
        // The free games' prompt, phased from the moment it shows.
        show.clock = 10.0;
        show.on_event(&Event::FreeGamesStarted { games: 6 });
        assert_eq!(show.prompt_lettering(), Some(true));
        show.clock = 10.3;
        assert_eq!(show.prompt_lettering(), Some(false));
        show.clock = 10.6;
        assert_eq!(show.prompt_lettering(), Some(true));
        // The Hold & Spin prompt shows at another moment, on its own phase.
        show.awaiting = None;
        assert_eq!(show.prompt_lettering(), None);
        show.clock = 20.13;
        show.wait_for(Await::Start(Feature::HoldAndSpin));
        assert_eq!(show.prompt_lettering(), Some(true));
        show.clock = 20.43;
        assert_eq!(show.prompt_lettering(), Some(false));
        // A respin is awaited with no prompt.
        show.wait_for(Await::Respin);
        assert_eq!(show.prompt_lettering(), None);
    }

    /// Every Hold & Spin's coins go to the WINNER box over the jackpot bar,
    /// with nothing over the reels; the WIN banner shows only for the count,
    /// counting from nothing with WIN; the box goes 0.55 s after the count
    /// (club cabinets: 1bAevGz0K2k 2:25-2:45, y8c8lEdPW8Y 1001.9-1019.9).
    #[test]
    fn every_hold_and_spin_tallies_on_the_winner_box() {
        let mut show = FeatureShow::default();
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        assert!(!show.winner_showing());
        assert_eq!(show.caption().as_deref(), Some("Free Spins Completed"));
        show.completed_wait = None;
        // The box shows from the collection's first frame, empty.
        show.begin_collect(
            vec![(0, 0, 60, false, None), (1, 1, 40, false, None)],
            1_000,
        );
        assert!(show.winner_showing());
        assert_eq!(show.winner_amount(), None);
        let dt = 1.0 / 60.0;
        let mut t = 0.0;
        let mut first_value = None;
        let mut sounds = Vec::new();
        let total = loop {
            show.clock += dt;
            t += dt;
            let step = show.advance_collect(dt, 50);
            sounds.extend(step.sounds);
            if let Some(total) = step.done {
                break total;
            }
            // No banner over the reels, and WIN unchanged, while the coins
            // are collected.
            assert_eq!(show.pay_banner(), None);
            assert_eq!(show.grand_win(), None);
            // The box stays, empty until the first value.
            assert!(show.winner_showing(), "{t}");
            assert_eq!(show.caption().as_deref(), Some("Free Spins Completed"));
            if show.winner_amount().is_some() && first_value.is_none() {
                first_value = Some(t);
                assert!(show.splash_at.is_some(), "a splash with each value");
                assert_eq!(show.winner_amount(), Some(60));
            }
            assert!(show.flares_at.is_none());
            assert!(t < 10.0);
        };
        // The first value 0.38 s after the box plus the comet's 0.67 s (club
        // cabinets: 1.03-1.13 s).
        let first = first_value.unwrap_or_default();
        assert!((first - 1.05).abs() < 0.03, "{first}");
        assert!((1.03..=1.13).contains(&(COLLECT_FIRST + BOLT_FLIGHT)));
        assert_eq!(total, 100);
        assert_eq!(sounds, ["SND_Small_5", "SND_Small_5"]);
        // The count: the banner from nothing in step with WIN; the box keeps
        // the total.
        show.pay = Some(Pay {
            amount: total,
            t: 0.0,
            seconds: 10.0,
        });
        assert!(show.winner_showing());
        assert_eq!(show.pay_banner(), Some(0));
        assert_eq!(show.grand_win(), Some(1_000));
        assert_eq!(show.caption().as_deref(), Some("Free Spins Completed"));
        if let Some(p) = show.pay.as_mut() {
            p.t = 2.5;
        }
        assert_eq!(show.pay_banner(), Some(25));
        assert_eq!(show.grand_win(), Some(1_025));
        assert_eq!(show.winner_amount(), Some(100));
        // Complete: the count's total is paid PAY_AFTER after it completed;
        // the banner and WIN keep the total, and the box gives way to the
        // meters WINNER_HOLD after the count completed (not after the
        // payment).
        let completed = show.clock;
        show.clock += PAY_AFTER;
        show.pay = None;
        show.paid = total;
        show.hold_winner(total);
        assert_eq!(show.winner, None);
        assert_eq!(show.collected, Some(total));
        // The base game's bet line returns after the count.
        assert_eq!(show.caption(), None);
        assert_eq!(show.pay_banner(), Some(100));
        assert_eq!(show.grand_win(), Some(1_100));
        assert!(show.winner_showing());
        assert_eq!(show.winner_amount(), Some(100));
        // Free games that follow do not cut it short.
        show.on_event(&Event::FreeGamesStarted { games: 6 });
        assert!(show.winner_showing());
        assert_eq!(show.winner_amount(), Some(100));
        show.clock = completed + WINNER_HOLD - 0.02;
        assert!(show.winner_showing());
        show.clock = completed + WINNER_HOLD + 0.02;
        assert!(!show.winner_showing());
        assert_eq!(show.winner_amount(), None);
        // The next game clears it all.
        show.on_event(&Event::SpinStarted { spin: 2 });
        assert!(!show.winner_showing());
        assert_eq!(show.pay_banner(), None);
    }

    /// The comets end in the middle of the WINNER box, from every position.
    #[test]
    fn every_comet_flies_to_the_winner_box() {
        for reel in 0..5u8 {
            for row in 0..3u8 {
                let (k, flip, left, top, w, h) = bolt_for(reel, row);
                let (sx, sy) = (w / BOLT_FRAME.0, h / BOLT_FRAME.1);
                let end = if flip {
                    BOLT_FRAME.0 - BOLT_ENDS[k]
                } else {
                    BOLT_ENDS[k]
                };
                let (ex, ey) = (left + end * sx, top);
                // Straight above a coin, inside the box's flames.
                let near = if reel == 2 { WINNER_BOX.0 * 0.4 } else { 0.5 };
                assert!((ex - WINNER_CENTRE.0).abs() < near, "{reel},{row}: x {ex}");
                assert!((ey - WINNER_CENTRE.1).abs() < 0.5, "{reel},{row}: y {ey}");
                assert!(sy > 0.0 && sx > 0.0);
            }
        }
        // The box sits above the WIN banner, inside the plate, which covers
        // the whole jackpot bar.
        let bar_bottom = JACKPOT_BAR_TOP + JACKPOT_BAR_W * 104.0 / 1225.0;
        assert!(WINNER_PLATE_H >= bar_bottom);
        assert!(WINNER_TOP + WINNER_BOX.1 <= BANNER_Y - BANNER_SIZE.1 / 2.0);
        assert_eq!(
            winner_jackpots_line("$12,018.71", "$714.89"),
            "GRAND JACKPOT: $12,018.71   MAJOR JACKPOT: $714.89"
        );
    }

    /// Each comet reaches the box as its value lands: its animation's frames
    /// up to its arrival stretched over the flight, the rest at 30 fps (club
    /// cabinets: flare to value 0.57-0.73 s, the first flare 0.33-0.43 s after
    /// the box).
    #[test]
    fn each_comet_lands_with_its_value() {
        assert!((0.33..=0.43).contains(&COLLECT_FIRST));
        assert!((0.57..=0.73).contains(&BOLT_FLIGHT));
        assert!((1.33..=1.37).contains(&COLLECT_STEP));
        for (k, &arrive) in BOLT_ARRIVE.iter().enumerate() {
            let arrive = arrive as usize;
            assert_eq!(bolt_frame(k, 0.0), 0);
            assert_eq!(bolt_frame(k, -0.1), 0);
            assert!(bolt_frame(k, BOLT_FLIGHT - 0.01) < arrive, "bolt{k}");
            assert_eq!(bolt_frame(k, BOLT_FLIGHT + 0.001), arrive, "bolt{k}");
            // After the arrival, one frame each 30th of a second.
            assert_eq!(bolt_frame(k, BOLT_FLIGHT + 0.25), arrive + 7, "bolt{k}");
        }
        // An unknown animation falls back to 30 fps.
        assert_eq!(bolt_frame(9, 0.5), 15);
    }

    /// The first respin starts by itself 0.59 s after the counter's 3 (club
    /// cabinets 0.55-0.63 s, four features): the count holds the 3 that
    /// long (blocking what follows), and the respin the press played shows
    /// as it ends.
    #[test]
    fn the_first_respin_follows_the_count_in_0_59_s() {
        let mut show = FeatureShow::default();
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.intro = None;
        show.started = true;
        show.count_up = Some(0.0);
        let dt = 1.0 / 60.0;
        let (mut ticks, mut three, mut end) = (0, None, None);
        for _ in 0..240 {
            show.clock += dt;
            ticks += show.advance_count_up(dt);
            if show.counter == 3 && three.is_none() {
                three = Some(show.clock);
                assert_eq!(show.banner_burst, Some(show.clock), "the burst on the 3");
            }
            if show.count_up.is_none() {
                end = Some(show.clock);
                break;
            }
            assert!(show.blocks());
        }
        assert_eq!(ticks, 3);
        let (three, end) = (three.unwrap_or_default(), end.unwrap_or_default());
        assert!(
            (three - (COUNT_FIRST + 2.0 * COUNT_STEP)).abs() < dt,
            "{three}"
        );
        assert!((0.55..=0.63).contains(&(end - three)), "{}", end - three);
        assert!(!show.blocks());
    }

    /// The status line through a base game's Hold & Spin: blank through the
    /// intro, the prompt from its start through the count-up until the
    /// first respin, blank while a respin runs, "Press SPIN button" between,
    /// "Free Spins Completed" from the end through the count, then the bet
    /// line (club cabinets: 1bAevGz0K2k 107.6-165.5, f5sMl9a5jds 204-282.6).
    #[test]
    fn the_status_line_follows_a_hold_and_spin() {
        let press = Some("Press START FEATURE button".to_string());
        let blank = Some(String::new());
        let mut show = FeatureShow::default();
        assert_eq!(show.caption(), None, "the bet line while the reels spin");
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        assert_eq!(show.caption(), blank, "the intro");
        show.intro = None;
        show.grid = true;
        show.wait_for(Await::Start(Feature::HoldAndSpin));
        assert_eq!(show.caption(), press);
        // The press: the counter counts up under the same line.
        show.awaiting = None;
        show.started = true;
        show.count_up = Some(0.0);
        assert_eq!(show.caption(), press);
        show.count_up = None;
        assert_eq!(show.caption(), press, "until the first respin");
        // The first respin starts: blank while it runs.
        show.on_event(&Event::HoldAndSpinRespin {
            respin: 1,
            landed: Vec::new(),
            respins_left: 2,
            locked_total: 6,
        });
        show.respin_landed = None;
        show.respin = Some(RespinAnim::default());
        assert_eq!(show.caption(), blank);
        show.respin = None;
        assert_eq!(show.caption().as_deref(), Some("Press SPIN button"));
        // The last respin, then the end.
        show.awaiting = None;
        show.stage = Stage::HoldSpin { respins_left: 0 };
        show.respin = Some(RespinAnim::default());
        assert_eq!(show.caption(), blank);
        show.respin = None;
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        assert_eq!(show.caption().as_deref(), Some("Free Spins Completed"));
        // Inside the free games the free game shows from the press on, the
        // end still "Free Spins Completed", and the free game after it.
        let game = Some("Free Game 3 of 6".to_string());
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 3,
            remaining: 3,
        });
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        assert_eq!(show.caption(), game, "the intro");
        show.intro = None;
        show.grid = true;
        show.wait_for(Await::Start(Feature::HoldAndSpin));
        assert_eq!(show.caption(), press);
        show.awaiting = None;
        show.started = true;
        show.count_up = Some(0.0);
        assert_eq!(show.caption(), game, "the count-up");
        show.count_up = None;
        show.respin = Some(RespinAnim::default());
        assert_eq!(show.caption(), game, "a respin");
        show.respin = None;
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        assert_eq!(show.caption().as_deref(), Some("Free Spins Completed"));
        show.completed_wait = None;
        show.collected = Some(100);
        assert_eq!(show.caption(), game, "after the count");
    }

    /// The counter's burst is a blue-white flash of the pack's burst frames
    /// 0-17 at 40 fps (gone after about 0.45 s; club cabinets 0.37-0.43 s),
    /// on a square about 1.9 banner heights across its ring.
    #[test]
    fn the_counter_burst_is_a_short_blue_white_flash() {
        assert_eq!(counter_burst_frame(0.0), Some(0));
        assert_eq!(counter_burst_frame(0.2), Some(8));
        assert_eq!(counter_burst_frame(0.44), Some(17));
        assert_eq!(counter_burst_frame(0.46), None);
        let ring = COUNTER_BURST_SIZE * 0.81 / BANNER_SIZE.1;
        assert!((1.8..=2.1).contains(&ring), "{ring}");
        // The burst's orange becomes near white; its alpha is kept; a dark
        // pixel pale blue.
        let [r, g, b, a] = blue_white([249, 100, 35, 77]);
        assert!(r >= 250 && g >= 250 && b == 255 && a == 77, "{r} {g} {b}");
        assert_eq!(blue_white([0, 0, 0, 255]), [200, 215, 255, 255]);
        assert_eq!(blue_white([10, 255, 0, 0]), [255, 255, 255, 0]);
    }

    /// The pack frame's own row lines are cleared, anti-aliasing included,
    /// leaving its border and the dividers that cross them.
    #[test]
    fn the_hold_and_spin_frame_loses_its_row_lines() {
        let (w, h) = (40usize, 30usize);
        let mut data = vec![0u8; w * h * 4];
        let mut put = |x: usize, y: usize, a: u8| data[(y * w + x) * 4 + 3] = a;
        for x in 0..w {
            // The border, top and bottom, and a row line with soft edges.
            for y in [0, 1, 28, 29] {
                put(x, y, 255);
            }
            put(x, 13, 60);
            put(x, 14, 255);
            put(x, 15, 255);
            put(x, 16, 40);
        }
        for y in 0..h {
            // The side borders and a divider.
            for x in [0, 1, 19, 20, 38, 39] {
                put(x, y, 255);
            }
        }
        let alpha = |data: &[u8], x: usize, y: usize| data[(y * w + x) * 4 + 3];
        assert_eq!(clear_row_lines(&mut data, w, h), 1);
        for y in 12..=17 {
            for x in [2, 10, 18, 21, 37] {
                assert_eq!(alpha(&data, x, y), 0, "{x},{y}");
            }
            for x in [0, 1, 19, 20, 38, 39] {
                assert_eq!(alpha(&data, x, y), 255, "{x},{y}");
            }
        }
        // The border rows stay; a second pass finds nothing.
        assert_eq!(alpha(&data, 10, 0), 255);
        assert_eq!(alpha(&data, 10, 29), 255);
        assert_eq!(clear_row_lines(&mut data, w, h), 0);
        // A buffer of the wrong size is left alone.
        assert_eq!(clear_row_lines(&mut data, w + 1, h), 0);
        // Our row lines sit on the grid's row boundaries, thin.
        const { assert!(HS_ROW_LINE_H <= 3.0) };
    }

    /// START FEATURE to the first free game: the flags go still at the press,
    /// nothing moves and nothing shows until the first free game's reels
    /// start 0.52 s after it, with its banner, its total counting up and the
    /// status line (club cabinets: y8c8lEdPW8Y 949.80-950.40, 1bAevGz0K2k
    /// 328.60-329.17).
    #[test]
    fn the_free_games_start_half_a_second_after_the_press() {
        let mut show = FeatureShow::default();
        assert!(show.begin_free_games_trigger(6, 100));
        // The trigger sequence has played; the scatter win counts.
        show.fg_trigger = Some(FG_COUNT + 2.0);
        show.on_event(&Event::FreeGamesStarted { games: 6 });
        assert_eq!(show.awaiting, Some(Await::Start(Feature::FreeGames)));
        show.fg_flags = true;
        show.scatter_paid = true;
        // The press (feature_press).
        show.awaiting = None;
        show.press_free_games();
        assert!(!show.fg_flags && !show.scatter_paid, "the flags go still");
        assert!(show.blocks());
        assert_eq!(show.banner_key(None), None);
        assert!(!show.advance_cut(FG_CUT - 0.01));
        assert!(show.blocks());
        // The cut: the first free game is played; it shows one event
        // interval later, with its reels.
        assert!(show.advance_cut(0.02));
        assert!(!show.blocks());
        assert!((FG_CUT + TIMING.event_interval - 0.52).abs() < 1e-6);
        assert_eq!(show.caption().as_deref(), Some(""));
        assert_eq!(show.message(), None);
        assert_eq!(show.banner_key(None), None);
        assert!(!show.advance_cut(TIMING.event_interval));
        show.on_event(&Event::FreeGameStarted {
            number: 1,
            remaining: 5,
        });
        assert_eq!(show.fg_cut, None);
        assert!(show.fg_count.is_some(), "the banner's total counts up");
        assert_eq!(show.banner_key(None).as_deref(), Some("banner_free_game"));
        assert_eq!(
            show.counter_numbers(None),
            Some(["1".to_string(), "0".to_string()])
        );
    }

    /// The first free game's banner counts its total up from "1 OF 0", one
    /// step every 5 video frames, and the status line stays blank until one
    /// step after the total (club cabinets, frame by frame: y8c8lEdPW8Y
    /// 950.400-951.567, 1bAevGz0K2k 329.167-330.333).
    #[test]
    fn the_free_games_counter_counts_from_nought_and_the_status_waits() {
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 1,
            remaining: 5,
        });
        assert_eq!(show.fg_count, Some((0.0, 0)));
        let numbers = |show: &FeatureShow| show.counter_numbers(None).map(|[_, of]| of);
        assert_eq!(numbers(&show).as_deref(), Some("0"));
        assert_eq!(show.caption().as_deref(), Some(""));
        let mut ticks = 0;
        let mut at = |show: &mut FeatureShow, t: f32| {
            let n = show.fg_count.map_or(6, |(_, n)| n);
            let (next, steps) = fg_count_step(n, 6, t);
            show.fg_count = next;
            ticks += steps;
        };
        // "1 OF 0" for one step (0.167 s), then a step each 0.167 s.
        for (t, want) in [
            (0.15, "0"),
            (0.17, "1"),
            (0.34, "2"),
            (0.99, "5"),
            (1.01, "6"),
        ] {
            at(&mut show, t);
            assert_eq!(numbers(&show).as_deref(), Some(want), "at {t} s");
            assert_eq!(show.caption().as_deref(), Some(""), "status at {t} s");
        }
        // One step after the total the status shows the free game.
        at(&mut show, 1.16);
        assert!(show.fg_count.is_some());
        assert_eq!(show.caption().as_deref(), Some(""));
        at(&mut show, 1.17);
        assert_eq!(show.fg_count, None);
        assert_eq!(show.caption().as_deref(), Some("Free Game 1 of 6"));
        assert_eq!(numbers(&show).as_deref(), Some("6"));
        // One tick a step (HYPOTHESIS).
        assert_eq!(ticks, 6);
        assert!((FG_TICK - 5.0 / 30.0).abs() < 1e-6);
    }

    /// A free game's WIN banner never shows the last game's amount: as a
    /// game starts the last count is settled, so nothing is counting until
    /// this game's win arrives, which counts from what WIN shows (our
    /// recording showed "WIN $1.00 WIN" for 4 frames; club cabinets never).
    #[test]
    fn a_free_game_starts_with_the_last_count_settled() {
        let mut rollup = Rollup {
            from: 100,
            to: 2_000,
            t: 2.0,
            duration: 2.0,
            seen: 2_000,
            after: 0.5,
            ..Rollup::default()
        };
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 3,
            remaining: 3,
        });
        // This game's lines are presented an event interval before its
        // credit: the banner must not show the last game's 1,900.
        show.on_event(&Event::LineWin {
            line: 4,
            count: 3,
            symbol: "J".into(),
            award: 60,
        });
        assert_eq!(banner_win(&show, &rollup, 50), Some(1_900), "unsettled");
        rollup.settle();
        assert_eq!(rollup.win(), 0);
        assert_eq!(rollup.shown(), 2_000, "WIN keeps what it shows");
        assert!(!rollup.active());
        assert_eq!(banner_win(&show, &rollup, 50), None);
        assert_eq!(show.banner_key(None).as_deref(), Some("banner_free_game"));
        // The credit arrives: the count runs from what WIN shows (as
        // `win_rollup` takes it) to the new total, this game's win alone.
        let from = rollup.shown();
        assert_eq!(from, 2_000);
        rollup.from = from;
        rollup.to = 2_060;
        assert_eq!(rollup.win(), 60);
    }

    /// FEATURE COMPLETED waits for the last free game's count, and while the
    /// free games' last events are queued CREDIT still holds their total
    /// off and the status reads "Feature Completed" (club cabinets:
    /// y8c8lEdPW8Y 1044.233-1044.300, 1bAevGz0K2k 367.567).
    #[test]
    fn feature_completed_follows_the_last_count() {
        let completed = Event::FreeGamesCompleted {
            games_played: 6,
            total_award: 900,
        };
        let playing = FeatureShow {
            stage: Stage::FreeGames {
                number: 6,
                total: 6,
            },
            ..FeatureShow::default()
        };
        assert!(playing.waits_for_count(Some(&completed)));
        let mut done = FeatureShow::default();
        done.on_event(&completed);
        assert_eq!(done.stage, Stage::FreeGamesDone);
        let mut presentation = Presentation::default();
        presentation.queue.push_back((
            TIMING.event_interval,
            Event::SpinCompleted {
                spin: 1,
                total_award: 900,
            },
        ));
        assert!(fg_end_queued(&presentation, &done));
        assert!(!fg_end_queued(&presentation, &playing));
        presentation.queue.clear();
        assert!(!fg_end_queued(&presentation, &done), "then the offer");
    }

    /// The doors open 0.30 s after the reels rest, their 42 frames over a
    /// second, the wins at the rest + 1.5 s; with no win the next free game
    /// spins at the rest + 2.0 s (club cabinets: 1bAevGz0K2k, y8c8lEdPW8Y).
    #[test]
    fn the_doors_and_the_no_win_pace_follow_the_cabinet() {
        assert!((REVEAL_AFTER_REST - 0.30).abs() < 1e-6);
        assert!((REVEAL_AFTER_REST + 42.0 / REVEAL_FPS - 1.30).abs() < 1e-6);
        // The doors' motion (frames 1-29) is over by the rest + 0.99 s.
        assert!((REVEAL_AFTER_REST + 29.0 / REVEAL_FPS - 0.99).abs() < 0.01);
        let wins_at = REVEAL_AFTER_REST + REVEAL_OPEN + REVEAL_WINS;
        assert!((wins_at - 1.5).abs() < 1e-6);
        let no_win = FeatureShow {
            stage: Stage::FreeGames {
                number: 2,
                total: 6,
            },
            ..FeatureShow::default()
        };
        // The game's last event shows as the reveal ends; the pause; the
        // next game's start an event interval later, with its reels.
        let next_spin = wins_at + no_win.step_pause() + TIMING.event_interval;
        assert!((next_spin - 2.0).abs() < 1e-6, "{next_spin}");
        let won = FeatureShow {
            game_won: true,
            ..no_win
        };
        assert!((won.step_pause() - 1.25).abs() < 1e-6);
        // Golden Gong's monk keeps its own (unmeasured) delay.
        const { assert!(EXPAND_AFTER_REST < REVEAL_AFTER_REST) };
    }

    /// The winning line's badges and symbols take turns in two even cycles
    /// of about 0.66 s, lit first, with no gap and no overlap (club
    /// cabinets: y8c8lEdPW8Y 21.93-24.23 s, CAM8vranI9Y).
    #[test]
    fn a_winning_line_blinks_in_two_even_cycles() {
        let mut t = 0.0;
        let mut lit_frames = 0;
        while t < WIN_LINE_SECONDS {
            let lit = in_spans(t, &WIN_LINE_LIT);
            let blink = in_spans(t, &WIN_LINE_BLINK);
            assert!(lit != blink, "exactly one of lit and blanked at {t}");
            if lit {
                lit_frames += 1;
            }
            t += 1.0 / 300.0;
        }
        // Lit about half the time.
        let share = lit_frames as f32 / (WIN_LINE_SECONDS * 300.0);
        assert!((share - 0.5).abs() < 0.03, "{share}");
        assert!(in_spans(0.0, &WIN_LINE_LIT), "each line starts lit");
        let cycle = WIN_LINE_LIT[1].0 - WIN_LINE_LIT[0].0;
        assert!((cycle - 0.66).abs() < 0.02, "{cycle}");
        assert!((WIN_LINE_SECONDS - 2.0 * cycle).abs() < 0.02);
    }

    /// The club tall pill's icon: two arcs and a half-disc over a trapezoid
    /// and a bar, each apart, symmetric about the middle.
    #[test]
    fn the_club_pill_icon_has_its_five_parts() {
        let cx = PILL_ICON.0 / 2.0;
        // Down the middle: the top arc, a gap, the second arc, a gap, the
        // half-disc, a gap, the trapezoid, a gap, the bar.
        let column: Vec<bool> = (0..56)
            .map(|y| pill_icon_covers(cx, y as f32 + 0.5))
            .collect();
        let mut parts = 0;
        let mut was = false;
        for &on in &column {
            if on && !was {
                parts += 1;
            }
            was = on;
        }
        assert_eq!(parts, 5, "{column:?}");
        for y in 0..56 {
            for x in 0..18 {
                let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
                assert_eq!(
                    pill_icon_covers(x, y),
                    pill_icon_covers(PILL_ICON.0 - x, y),
                    "{x},{y}"
                );
            }
        }
        // Outside its box nothing.
        assert!(!pill_icon_covers(1.0, 40.0) && !pill_icon_covers(cx, 57.0));
        let image = club_pill_icon();
        assert_eq!(image.width(), PILL_ICON.0 as u32 * 2);
        assert_eq!(image.height(), PILL_ICON.1 as u32 * 2);
    }

    /// The free games' prompt shows as the scatter win starts counting, the
    /// count going on under it; a Hold & Spin's trigger and the free games
    /// after a Hold & Spin still wait for the count (club cabinets:
    /// y8c8lEdPW8Y 946.07, 1bAevGz0K2k 326.80).
    #[test]
    fn the_free_games_prompt_comes_with_the_scatter_count() {
        let triggered = Event::FeatureTriggered {
            feature: Feature::FreeGames,
            count: 3,
        };
        let started = Event::FreeGamesStarted { games: 6 };
        let mut show = FeatureShow::default();
        assert!(show.begin_free_games_trigger(6, 100));
        show.fg_trigger = Some(FG_COUNT - 0.1);
        assert!(show.waits_for_count(Some(&triggered)));
        show.fg_trigger = Some(FG_COUNT + 0.2);
        assert!(show.trigger_counting());
        assert!(!show.waits_for_count(Some(&triggered)));
        assert!(!show.waits_for_count(Some(&started)));
        show.clock = 10.0;
        show.on_event(&triggered);
        show.on_event(&started);
        assert_eq!(show.awaiting, Some(Await::Start(Feature::FreeGames)));
        // The lettering's blink restarts, lit, as the count completes.
        show.clock = 10.1;
        show.follow_count(true);
        show.clock = 10.3;
        show.follow_count(true);
        assert_eq!(show.prompt_lettering(), Some(false));
        show.follow_count(false);
        assert_eq!(show.prompt_lettering(), Some(true));
        show.clock = 10.6;
        show.follow_count(false);
        assert_eq!(show.prompt_lettering(), Some(false));

        let hold_and_spin = Event::FeatureTriggered {
            feature: Feature::HoldAndSpin,
            count: 6,
        };
        let after = FeatureShow {
            stage: Stage::HoldSpinDone,
            ..FeatureShow::default()
        };
        assert!(after.waits_for_count(Some(&started)));
        assert!(after.waits_for_count(Some(&hold_and_spin)));
        assert!(after.waits_for_count(Some(&Event::FreeGamesResumed { remaining: 3 })));
        assert!(after.waits_for_count(Some(&Event::FreeGamesCompleted {
            games_played: 6,
            total_award: 10,
        })));
        assert!(!after.waits_for_count(Some(&Event::SpinStarted { spin: 2 })));
        assert!(!after.waits_for_count(None));
    }

    /// The status and message rows through a free-games trigger: "N FREE
    /// GAMES WON!" from the result, "Scatter wins N" from the flags' wave,
    /// both blank from the press until the first free game (club cabinets:
    /// y8c8lEdPW8Y 938.57, 941.20, 949.87; 1bAevGz0K2k 319.3, 321.9, 328.6).
    #[test]
    fn a_free_games_trigger_shows_the_games_won_and_the_scatter_win() {
        let mut show = FeatureShow::default();
        assert!(show.begin_free_games_trigger(6, 100));
        assert_eq!(show.caption().as_deref(), Some("6 FREE GAMES WON!"));
        assert_eq!(show.message(), None);
        show.fg_flags = true;
        assert_eq!(show.message().as_deref(), Some("Scatter wins 100"));
        show.awaiting = None;
        show.press_free_games();
        assert_eq!(show.caption().as_deref(), Some(""));
        assert_eq!(show.message(), None);
        show.win_note = Some("Line 3 wins 20".to_string());
        assert_eq!(show.message(), None, "line wins too");
        show.win_note = None;
        show.on_event(&Event::FreeGameStarted {
            number: 1,
            remaining: 5,
        });
        show.fg_count = None;
        assert!(show.fg_trigger.is_some());
        assert_eq!(show.caption().as_deref(), Some("Free Game 1 of 6"));
        assert_eq!(show.message(), None);
    }

    /// While START FEATURE is awaited the free games' status alternates in
    /// 2.0 s slots from the prompt's start, "Press START FEATURE button"
    /// first (club cabinets: y8c8lEdPW8Y 791.67-793.70, 1bAevGz0K2k 326.77).
    #[test]
    fn the_prompt_status_alternates_every_two_seconds_press_first() {
        let press = Some("Press START FEATURE button".to_string());
        let won = Some("6 FREE GAMES WON!".to_string());
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGamesStarted { games: 6 });
        for (waited, want) in [
            (0.0, &press),
            (1.9, &press),
            (2.1, &won),
            (3.9, &won),
            (4.1, &press),
        ] {
            show.waited = waited;
            assert_eq!(&show.caption(), want, "at {waited} s");
        }
        // Hold & Spin's prompt does not alternate.
        show.wait_for(Await::Start(Feature::HoldAndSpin));
        show.waited = 2.5;
        assert_eq!(show.caption(), press);
    }

    /// A free game's win shows "WIN $x WIN" only from 10x the bet (the
    /// limit HYPOTHESIS; the rule STRONG), for a second after its count; a
    /// smaller win counts under "FREE GAME n OF 6" and its counter. A Hold &
    /// Spin's payment always shows.
    #[test]
    fn only_a_big_free_game_win_takes_the_banner() {
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 2,
            remaining: 4,
        });
        show.game_won = true;
        let counting = |win: u64| Rollup {
            from: 1_000,
            to: 1_000 + win,
            t: 0.5,
            duration: 2.0,
            ..Rollup::default()
        };
        let small = banner_win(&show, &counting(499), 50);
        assert_eq!(small, None);
        assert_eq!(show.banner_key(small).as_deref(), Some("banner_free_game"));
        assert_eq!(
            show.counter_numbers(small),
            Some(["2".to_string(), "6".to_string()])
        );
        let big = banner_win(&show, &counting(500), 50);
        assert_eq!(big, Some(125));
        assert_eq!(show.banner_key(big).as_deref(), Some("banner_win"));
        assert_eq!(show.counter_numbers(big), None);
        // A second after the count the free game's banner comes back.
        let done = |after: f32| Rollup {
            from: 1_000,
            to: 1_500,
            t: 2.0,
            duration: 2.0,
            after,
            ..Rollup::default()
        };
        assert_eq!(banner_win(&show, &done(0.9), 50), Some(500));
        assert_eq!(banner_win(&show, &done(1.1), 50), None);
        // A Hold & Spin's payment, however small.
        let paid = FeatureShow {
            stage: Stage::HoldSpinDone,
            collected: Some(5),
            paid: 5,
            ..FeatureShow::default()
        };
        assert_eq!(banner_win(&paid, &Rollup::default(), 50), Some(5));
        assert_eq!(paid.banner_key(Some(5)).as_deref(), Some("banner_win"));
    }

    /// Back in the free games after a Hold & Spin one of them started: the
    /// board stays dark with no banner and no counter, the status keeping
    /// the interrupted game, until "FREE GAME N+1 OF 6" comes with the reels
    /// 0.43 s after the plaque goes (club cabinet, y8c8lEdPW8Y 1019.93-1020.40).
    #[test]
    fn the_free_games_resume_dark_after_a_hold_and_spin() {
        let mut show = FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 3,
            remaining: 3,
        });
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        // The timer is armed in the last frame before the return shows.
        assert!((show.step_pause() - FG_RESUME).abs() < 1e-6);
        show.on_event(&Event::FreeGamesResumed { remaining: 3 });
        assert!(show.resuming);
        assert_eq!(show.caption().as_deref(), Some("Free Game 3 of 6"));
        assert!((show.step_pause() - 0.29).abs() < 1e-6);
        assert!((FG_RESUME + TIMING.event_interval - 0.43).abs() < 1e-6);
        assert_eq!(show.banner_key(None), None);
        assert_eq!(show.counter_numbers(None), None);
        show.on_event(&Event::FreeGameStarted {
            number: 4,
            remaining: 2,
        });
        assert!(!show.resuming);
        assert_eq!(show.banner_key(None).as_deref(), Some("banner_free_game"));
        assert_eq!(
            show.counter_numbers(None),
            Some(["4".to_string(), "6".to_string()])
        );
        assert_eq!(show.caption().as_deref(), Some("Free Game 4 of 6"));
    }

    /// After a denomination choice the message row shows no start prompt
    /// until the first game starts (club cabinets: y8c8lEdPW8Y 11.8-15.4 s,
    /// CAM8vranI9Y 11.4-13.8 s).
    #[test]
    fn no_start_prompt_from_a_denomination_choice_to_the_first_game() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(Fidelity(true));
        world.insert_resource(FeatureShow::default());
        world.insert_resource(GambleShow::default());
        world.insert_resource(GamesMenu {
            selecting: true,
            ..GamesMenu::default()
        });
        let prompt = world
            .spawn((StartPrompt { which: 0 }, Visibility::Inherited))
            .id();
        world.run_system_once(start_prompts).unwrap();
        assert_eq!(world.get::<Visibility>(prompt), Some(&Visibility::Hidden));
        // The first game starts: the prompt is back.
        world.resource_mut::<GamesMenu>().selecting = false;
        world.run_system_once(start_prompts).unwrap();
        assert_eq!(
            world.get::<Visibility>(prompt),
            Some(&Visibility::Inherited)
        );
    }

    /// A text's copies (its dark edge, a shaded band) follow its words,
    /// letter spacing and visibility, and keep their own colour.
    #[test]
    fn copies_follow_their_text() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let owner = world
            .spawn((
                Text::new("Free Game 1 of 6"),
                LetterSpacing::Px(4.8),
                Visibility::Hidden,
                TextColor(Color::WHITE),
            ))
            .id();
        let copy = world
            .spawn((
                Echo(owner),
                Text::new(""),
                LetterSpacing::Px(0.0),
                Visibility::Inherited,
                TextColor(EDGE_COLOUR),
            ))
            .id();
        world.run_system_once(echo_texts).unwrap();
        assert_eq!(
            world.get::<Text>(copy).map(|t| t.0.as_str()),
            Some("Free Game 1 of 6")
        );
        assert_eq!(
            world.get::<LetterSpacing>(copy),
            Some(&LetterSpacing::Px(4.8))
        );
        assert_eq!(world.get::<Visibility>(copy), Some(&Visibility::Hidden));
        assert_eq!(world.get::<TextColor>(copy), Some(&TextColor(EDGE_COLOUR)));
        // The edge goes all round the letters, evenly.
        let (sx, sy) = EDGE_OFFSETS
            .iter()
            .fold((0.0, 0.0), |(x, y), (dx, dy)| (x + dx, y + dy));
        assert!(sx == 0.0 && sy == 0.0);
        // The labels' shading: smaller shares over larger, orange at the
        // top of CREDIT, BET and WIN, yellow at the top of the denomination.
        for shade in [LABEL_SHADE, DENOM_SHADE] {
            let bands = shade_bands(shade);
            assert_eq!(bands.len() as u32, shade.3);
            assert!(bands.windows(2).all(|w| w[0].1 > w[1].1));
            assert!((bands[0].1 - shade.2).abs() < 1e-6);
        }
        let green = |c: Color| c.to_srgba().green;
        let top = shade_bands(LABEL_SHADE).last().map(|b| b.0).unwrap();
        assert!(
            green(top) < green(shade_base(LABEL_SHADE)),
            "orange over yellow"
        );
        let top = shade_bands(DENOM_SHADE).last().map(|b| b.0).unwrap();
        assert!(
            green(top) > green(shade_base(DENOM_SHADE)),
            "yellow over orange"
        );
    }

    /// A reel starting scrolls the window on screen away, not its stale
    /// landing, and an open position's dimmed symbol keeps its dimming (club
    /// cabinet, y8c8lEdPW8Y 1020.43); a dimmed flag still quiets its reel.
    #[test]
    fn a_reel_starting_scrolls_the_window_on_screen_still_dimmed() {
        use bevy::ecs::system::RunSystemOnce;
        let mut reels = reel_strips(&dragonlink_model::fixtures::golden_century_synthetic());
        let scatter = reels.scatter.clone();
        for reel in 0..2 {
            let s = &mut reels.states[reel];
            s.pos = 40.0;
            s.landing_at = 40;
            s.landing = vec!["door".to_string(); 3];
        }
        let mut world = World::new();
        world.insert_resource(reels);
        world.insert_resource(Fidelity(true));
        world.insert_resource(Time::<()>::default());
        world.insert_resource(Smoke {
            frames_left: Some(10),
            screenshot: None,
            frame: 0,
            hold: 0,
            real_time: false,
            capture: None,
        });
        world.insert_resource(ReelAnim {
            spinning: vec![true, true, false, false, false],
            ..ReelAnim::default()
        });
        world.insert_resource(ArtPack::default());
        world.insert_resource(Assets::<Image>::default());
        // On screen: a dimmed open position, a coin and a queen on reel 1;
        // a dimmed flag on reel 2.
        let dim_flag = format!("{DIM}{scatter}");
        for (reel, texts) in [
            (0u8, ["DIM:K", "ORB\nx5", "Q"]),
            (1u8, ["J", dim_flag.as_str(), "10"]),
        ] {
            for (row, text) in texts.iter().enumerate() {
                world.spawn((
                    CellLabel {
                        reel,
                        row: row as u8,
                    },
                    Text::new(*text),
                ));
            }
        }
        world.run_system_once(spin_reels).unwrap();
        let reels = world.resource::<Reels>();
        let first = reels.states[0].pos.round() as i64;
        assert!(matches!(reels.states[0].motion, Motion::Spinning { .. }));
        assert_eq!(reels.key_at(0, first), "DIM:K");
        assert_eq!(reels.key_at(0, first + 1), "ORB\nx5");
        assert_eq!(reels.key_at(0, first + 2), "Q");
        assert_eq!(label_symbol(reels.key_at(0, first)), ("K", true));
        assert_eq!(label_symbol(reels.key_at(0, first + 1)), ("orb", false));
        assert_eq!(strip_tint(true), DIM_TINT);
        assert_eq!(strip_tint(false), Color::WHITE);
        assert!(!reels.states[0].quiet);
        assert!(
            reels.states[1].quiet,
            "a dimmed flag leaving quiets its reel"
        );
    }

    /// A Major is awarded first, on its own plaque: no comet, SND_Credit1,
    /// its amount on the plaque and its meter restarting as it shows, its
    /// own count on WIN while the coins wait, then the empty box and the
    /// coins' tally, which leaves it out (club cabinet, Autumn Moon
    /// 5Z1-hFzpyh0 525-590 s).
    #[test]
    fn a_major_is_awarded_first_on_its_own_plaque() {
        let coins = majors_first(vec![
            (0, 0, 60, false, None),
            (2, 1, 30_000, true, Some(JackpotTier::Major)),
            (4, 2, 40, false, None),
        ]);
        assert_eq!(coins[0].4, Some(JackpotTier::Major));
        assert_eq!((coins[1].0, coins[2].0), (0, 4), "the rest in grid order");
        let mut show = FeatureShow::default();
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 30_100,
            full_grid: false,
        });
        show.completed_wait = None;
        show.begin_collect(coins, 1_000);
        let dt = 1.0 / 60.0;
        let mut sounds = Vec::new();
        let mut major = None;
        for _ in 0..600 {
            show.clock += dt;
            let step = show.advance_collect(dt, 50);
            sounds.extend(step.sounds);
            assert!(show.bolt.is_none(), "no comet for the Major");
            assert_eq!(show.winner_amount(), None, "the box stays empty");
            if let Some(m) = step.major {
                major = Some(m);
                break;
            }
        }
        assert_eq!(major, Some(30_000));
        assert_eq!(sounds, ["SND_Credit1"]);
        assert!(show.major_showing());
        assert!(show.link_won.contains(&JackpotTier::Major));
        // Its own count (the motion system starts it with the music).
        assert_eq!(show.major_amount(), None);
        show.major_pay = Some(Pay {
            amount: 30_000,
            t: 0.0,
            seconds: 10.0,
        });
        assert_eq!(show.major_amount(), Some(30_000));
        assert_eq!(show.grand_win(), Some(1_000));
        assert!(show.pay_shower(50), "a big Major rains coins");
        assert_eq!(show.pay_banner(), None, "no banner over the reels");
        // The collection waits for it.
        let t = show.collect_t();
        let step = show.advance_collect(1.0, 50);
        assert!(step.sounds.is_empty());
        assert_eq!(show.collect_t(), t);
        if let Some(p) = show.major_pay.as_mut() {
            p.t = 5.0;
        }
        assert_eq!(show.grand_win(), Some(16_000));
        // Over: the box empty again, the next coin COLLECT_FIRST later.
        show.major_pay = None;
        show.end_major(30_000);
        assert!(!show.major_showing());
        assert_eq!(show.winner, Some(0));
        assert_eq!(show.grand_win(), Some(31_000));
        let mut first_coin = None;
        let mut t = 0.0;
        let total = loop {
            show.clock += dt;
            t += dt;
            let step = show.advance_collect(dt, 50);
            if show.bolt.is_some() && first_coin.is_none() {
                first_coin = Some(t);
            }
            if let Some(total) = step.done {
                break total;
            }
            assert!(t < 10.0);
        };
        let first = first_coin.unwrap_or_default();
        assert!((first - COLLECT_FIRST).abs() < 0.03, "{first}");
        // The tally leaves the Major out; WIN keeps it.
        assert_eq!(total, 100);
        show.pay = Some(Pay {
            amount: total,
            t: 10.0,
            seconds: 10.0,
        });
        assert_eq!(show.pay_banner(), Some(100));
        assert_eq!(show.grand_win(), Some(31_100));
    }

    /// The Grand's fire covers the screens as its plaque switches on: it
    /// starts so that the plaque comes at its frame 27, under full cover
    /// (frames 21-35), at the cabinet's 30 fps (OL6Pu1AI-cQ 3.4-4.4 s).
    #[test]
    fn the_grand_fire_covers_the_screens_as_its_plaque_switches_on() {
        let mut show = FeatureShow {
            grand: Some(1_000_000),
            ..default()
        };
        show.begin_collect(
            vec![
                (0, 0, 60, false, None),
                (1, 0, 40, false, None),
                (
                    GRAND_STEP,
                    GRAND_STEP,
                    1_000_000,
                    true,
                    Some(JackpotTier::Grand),
                ),
            ],
            0,
        );
        let dt = 1.0 / 120.0;
        let (mut fire_at, mut plaque_at) = (None, None);
        for _ in 0..2_000 {
            show.clock += dt;
            let step = show.advance_collect(dt, 50);
            if fire_at.is_none()
                && let Some((s, fps)) = show.transition_at
            {
                assert_eq!(fps, GRAND_FIRE_FPS);
                fire_at = Some(s);
            }
            if plaque_at.is_none() && show.grand_showing() {
                plaque_at = Some(show.clock);
            }
            if step.done.is_some() {
                break;
            }
        }
        let (Some(fire), Some(plaque)) = (fire_at, plaque_at) else {
            panic!("a fire and a plaque");
        };
        let frame = (plaque - fire) * GRAND_FIRE_FPS;
        assert!(
            (21.0..=35.0).contains(&frame) && (frame - 27.0).abs() < 0.6,
            "{frame}"
        );
        // After the last coin's comet has left.
        assert!(fire > COLLECT_FIRST + COLLECT_STEP);
        // The intro's fire keeps its own rate.
        assert!((FIRE_FPS - 22.3).abs() < 1e-3);
    }

    /// After a base game's Hold & Spin, at rest, the status line alternates
    /// PLAY ON and "Free Spins Completed" while the win is offered, keeps
    /// "Free Spins Completed" for a win never offered, and gives way to the
    /// bet line once the win is taken; a Grand has its own line and its
    /// call to the attendant, the dollars ungrouped (club cabinets).
    #[test]
    fn a_hold_and_spin_end_and_a_grand_have_their_own_status_lines() {
        assert_eq!(hold_and_spin_end_line(true, true, true), Some(PLAY_ON));
        assert_eq!(
            hold_and_spin_end_line(true, true, false),
            Some(FREE_SPINS_COMPLETED)
        );
        assert_eq!(
            hold_and_spin_end_line(false, true, true),
            Some(FREE_SPINS_COMPLETED)
        );
        assert_eq!(hold_and_spin_end_line(false, false, true), None);
        let mut show = FeatureShow::default();
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: true,
        });
        assert!(!show.hold_and_spin_over(), "FREE SPINS COMPLETED shows");
        show.completed_wait = None;
        show.collected = Some(100);
        assert!(show.hold_and_spin_over());
        show.fg_return = Some((3, 6));
        assert!(!show.hold_and_spin_over(), "inside the free games");
        show.fg_return = None;
        // The Grand's line, from its award until the next game.
        assert_eq!(show.grand_caption(2), None);
        show.grand = Some(1_206_131);
        assert_eq!(show.grand_caption(2), None, "not before its award");
        show.grand_done = true;
        assert_eq!(
            show.grand_caption(2).as_deref(),
            Some("GRAND JACKPOT Win $24122.62")
        );
        assert_eq!(
            show.grand_call(2).as_deref(),
            Some("Call Attendant : JACKPOT HIT $24122.62")
        );
        show.on_event(&Event::SpinStarted { spin: 2 });
        assert_eq!(show.grand_caption(2), None);
        assert_eq!(show.grand_call(2), None);
    }

    /// The collected-coin disc counts the coins held and landed, each as it
    /// lands, and shows with the board until the WINNER display.
    #[test]
    fn the_collected_disc_counts_each_coin_as_it_lands() {
        let coins = [(0, 0), (1, 0), (2, 0), (3, 1)];
        let mut show = FeatureShow::default();
        assert_eq!(coins_collected(&coins, &show.coins_to_land()), 4);
        // Presented, not yet turning: its coins are still to land.
        show.respin_landed = Some(vec![(3, 1)]);
        assert_eq!(coins_collected(&coins, &show.coins_to_land()), 3);
        show.respin_landed = None;
        show.respin = Some(RespinAnim {
            stops: vec![(3, 1, 0.5, true), (4, 2, 0.9, false)],
            ..default()
        });
        assert_eq!(coins_collected(&coins, &show.coins_to_land()), 3);
        if let Some(a) = show.respin.as_mut() {
            a.done = 1;
        }
        assert_eq!(coins_collected(&coins, &show.coins_to_land()), 4);
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        assert!(!show.collected_showing(), "not before the board");
        show.grid = true;
        assert!(show.collected_showing());
        show.winner = Some(0);
        assert!(!show.collected_showing(), "the WINNER display takes over");
    }

    /// The feature banner's rim clears the jackpot values above it.
    #[test]
    fn the_feature_banner_clears_the_jackpot_values() {
        let k = JACKPOT_BAR_W / 1225.0;
        let middle = JACKPOT_BAR_TOP + 64.5 * k;
        // The figures reach about 0.36 of the type size below the middle
        // line (a capture: digits y 73-117 about 95), their edge 2 px more.
        let ink_bottom = middle + 0.36 * JACKPOT_VALUE_SIZE + 2.0;
        let banner_top = BANNER_Y - BANNER_SIZE.1 / 2.0;
        assert!(banner_top >= ink_bottom + 2.0, "{banner_top} {ink_bottom}");
        // It still stands across the reels' top edge.
        assert!(BANNER_Y + BANNER_SIZE.1 / 2.0 > GRID_CENTER.1 - 1.5 * PITCH.1);
    }

    /// A progressive's changed figures roll up inside the box: the old one
    /// rises out, the new one rises from below, eased, over 0.37 s; a
    /// change mid-roll ends it and rolls on (y8c8lEdPW8Y).
    #[test]
    fn a_changed_figure_rolls_up_inside_its_box() {
        assert!((0.35..=0.40).contains(&ROLL_SECONDS));
        let still = |r: &Roll| r.poses().iter().all(|p| p.now_y == 0.0 && p.old.is_none());
        let mut roll = Roll::default();
        roll.follow("-");
        roll.follow("$603.71");
        assert!(still(&roll), "the first value shows as it is");
        roll.follow("$603.72");
        let p = roll.poses();
        assert_eq!(p[6].now, '2');
        assert!((p[6].now_y - 1.0).abs() < 1e-6);
        assert_eq!(p[6].old, Some(('1', 0.0)));
        assert!(
            p[..6].iter().all(|p| p.old.is_none() && p.now_y == 0.0),
            "only the changed figure rolls"
        );
        roll.advance(ROLL_SECONDS / 2.0);
        let p = roll.poses();
        assert!((p[6].now_y - 0.5).abs() < 1e-4);
        assert!(p[6].old.is_some_and(|(_, y)| (y + 0.5).abs() < 1e-4));
        roll.follow("$603.73");
        let p = roll.poses();
        assert_eq!(
            p[6].old,
            Some(('2', 0.0)),
            "on from the value it was rolling to"
        );
        roll.advance(1.0);
        assert!(still(&roll));
        // Every changed figure rolls; a longer value lines up from the right.
        roll.follow("$1,000.00");
        let p = roll.poses();
        assert_eq!(p[8].old.map(|o| o.0), Some('3'));
        assert_eq!(p[0].old, None);
        assert!(p[0].now_y > 0.0);
        roll.advance(1.0);
        assert!(still(&roll));
    }

    /// The cabinet spawned into a world with no pack, for layout tests.
    fn spawned_cabinet(profile: Profile) -> World {
        let mut world = World::new();
        let model = dragonlink_model::fixtures::golden_century_synthetic();
        let art = ArtPack::default();
        let fonts = CabFonts {
            meter: Handle::default(),
            sans: Handle::default(),
            symbols: Handle::default(),
            heavy: Handle::default(),
            msg: Handle::default(),
        };
        {
            let mut commands = world.commands();
            spawn(&mut commands, &model, &art, &fonts, profile);
        }
        world.flush();
        world
    }

    fn px(v: Val) -> f32 {
        match v {
            Val::Px(p) => p,
            other => panic!("not pixels: {other:?}"),
        }
    }

    /// Club cabinets: COLLECT and GAMES MENU stay over the gamble screen
    /// and give way on the chooser, whose own COLLECT stands in the bottom
    /// right corner (the casino profile unchanged).
    #[test]
    fn the_club_tabs_stay_over_the_gamble_and_collect_moves_on_the_chooser() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = spawned_cabinet(Profile::Club);
        let mut tabs = world.query::<(&BandTab, &GlobalZIndex)>();
        let found: Vec<(BandTab, i32)> = tabs.iter(&world).map(|(t, z)| (*t, z.0)).collect();
        assert_eq!(found.len(), 2);
        for kind in [BandTab::Collect, BandTab::GamesMenu] {
            assert!(found.contains(&(kind, BAND_TAB_LAYER)), "{kind:?}");
        }
        // Over the gamble screen and the chooser's splash (12).
        const { assert!(BAND_TAB_LAYER > 12) };
        // The chooser keeps a COLLECT of its own, in the corner.
        let mut roots = world.query_filtered::<Entity, With<ChooserRoot>>();
        let root = roots.single(&world).unwrap();
        let mut texts = world.query::<(Entity, &Text)>();
        let collects: Vec<Entity> = texts
            .iter(&world)
            .filter(|(_, t)| t.0 == "COLLECT")
            .map(|(e, _)| e)
            .collect();
        assert_eq!(collects.len(), 2, "the band's and the chooser's");
        let in_chooser = |world: &World, mut e: Entity| {
            while let Some(parent) = world.get::<ChildOf>(e) {
                e = parent.parent();
                if e == root {
                    return true;
                }
            }
            false
        };
        let chooser_pill = collects.iter().filter(|e| in_chooser(&world, **e)).count();
        assert_eq!(chooser_pill, 1);
        // The tabs hide while the chooser shows, and come back after.
        world.insert_resource(Fidelity(true));
        world.insert_resource(Time::<()>::default());
        world.insert_resource(Smoke {
            frames_left: None,
            screenshot: None,
            frame: 0,
            hold: 0,
            real_time: false,
            capture: None,
        });
        world.insert_resource(Reels::default());
        let mut menu = GamesMenu::default();
        menu.open_now();
        world.insert_resource(menu);
        world.run_system_once(games_menu).unwrap();
        let mut vis = world.query_filtered::<&Visibility, With<BandTab>>();
        assert!(vis.iter(&world).all(|v| *v == Visibility::Hidden));
        world.resource_mut::<GamesMenu>().open = false;
        world.run_system_once(games_menu).unwrap();
        assert!(vis.iter(&world).all(|v| *v == Visibility::Inherited));
        // The casino keeps its tabs as they were.
        let mut world = spawned_cabinet(Profile::Casino);
        let mut tabs = world.query::<&BandTab>();
        assert_eq!(tabs.iter(&world).count(), 0);
    }

    /// The club gamble prompt is the bet line's yellow, spaced wider (club
    /// cabinets: _XHUzg4qDXI, LwC3hVqFRMg); the casino's keeps the status
    /// line's look.
    #[test]
    fn the_club_gamble_prompt_is_yellow_and_spaced() {
        let (yellow, gap) = GAMBLE_PROMPT_CLUB;
        let c = yellow.to_srgba();
        assert!(c.red > 0.8 && c.green > 0.8 && c.blue < 0.4);
        // Verdana Bold 23 writes the 36 letters about 518 px wide; spaced,
        // about 728 against the cabinet's about 731.
        let wide = 518.0 + 35.0 * gap;
        assert!((700.0..=760.0).contains(&wide), "{wide}");
        for (profile, want) in [
            (Profile::Club, (yellow, LetterSpacing::Px(gap))),
            (
                Profile::Casino,
                (Color::srgb_u8(210, 220, 245), LetterSpacing::Px(1.5)),
            ),
        ] {
            let mut world = spawned_cabinet(profile);
            let mut q = world.query::<(&GambleText, &TextColor, &LetterSpacing)>();
            let prompt: Vec<(Color, LetterSpacing)> = q
                .iter(&world)
                .filter(|(t, ..)| **t == GambleText::Prompt)
                .map(|(_, c, s)| (c.0, *s))
                .collect();
            assert_eq!(prompt, vec![want], "{profile:?}");
        }
    }

    /// The chooser as two club recordings show it (homography): the panel
    /// about 995 x 195 centred near y 755 with its keys inside it, the
    /// header's capitals near y 703, ARE YOU SURE? wholly above the panel
    /// with a small gap, the drawn title's capitals about y 485-640 and
    /// about 1560 wide.
    #[test]
    fn the_chooser_is_laid_out_as_on_the_club_cabinet() {
        let (pw, ph, ptop) = CHOOSER_PANEL;
        assert!((980.0..=1010.0).contains(&pw));
        assert!((190.0..=200.0).contains(&ph));
        assert!((750.0..=760.0).contains(&(ptop + ph / 2.0)));
        let (kw, kh, gap, ktop) = CHOOSER_KEY;
        assert!((155.0..=165.0).contains(&kw) && (78.0..=82.0).contains(&kh));
        assert!((25.0..=29.0).contains(&gap));
        assert!((775.0..=785.0).contains(&(ktop + kh / 2.0)));
        assert!(ktop > CHOOSER_HEADER.0 && ktop + kh < ptop + ph);
        const { assert!(CHOOSER_MARGIN > 30.0) };
        // The title fits the logo's place, and every title's name fits the
        // canvas.
        assert_eq!(chooser_title_size("GOLDEN CENTURY"), CHOOSER_TITLE.1);
        let (title_y, _, spacing, _) = CHOOSER_TITLE;
        let size = CHOOSER_TITLE.1;
        let caps = (
            title_y - IMPACT_CAPS.1 * size / 2.0,
            title_y + IMPACT_CAPS.1 * size / 2.0,
        );
        assert!((475.0..=495.0).contains(&caps.0) && (630.0..=650.0).contains(&caps.1));
        assert!(caps.1 < ptop, "the title stands over the panel");
        let wide = |name: &str| {
            let n = name.chars().count() as f32;
            n * IMPACT_CAPS_ADVANCE * chooser_title_size(name) + n * spacing
        };
        assert!((1500.0..=1620.0).contains(&wide("GOLDEN CENTURY")));
        for slug in [
            "golden-century",
            "autumn-moon",
            "happy-prosperous",
            "panda-magic",
            "genghis-khan",
            "golden-gong",
            "peace-long-life",
            "spring-festival",
        ] {
            let name = title_name(slug);
            assert_ne!(name, title_name("synthetic"), "{slug}");
            assert!(wide(name) <= CHOOSER_TITLE.3 + 1.0, "{name}");
        }
        // The spawned chooser: the confirmation wholly above the panel, a
        // small gap between; six keys widen the panel to hold them.
        for profile in [Profile::Club, Profile::Casino] {
            let mut world = spawned_cabinet(profile);
            let mut q = world.query_filtered::<&Node, With<ChooserPanel>>();
            let panel = q.single(&world).unwrap().clone();
            let mut q = world.query_filtered::<&Node, With<ChooserConfirm>>();
            let confirm = q.single(&world).unwrap().clone();
            let gap = px(panel.top) - (px(confirm.top) + px(confirm.height));
            assert!((5.0..=15.0).contains(&gap), "{gap}");
            let n = profile.denoms().len() as f32;
            assert!(px(panel.width) >= n * kw + (n - 1.0) * CHOOSER_KEY.2);
            assert_eq!(px(panel.width) == pw, n == 5.0, "{profile:?}");
        }
        // The cent sign is drawn apart, smaller; dollars whole.
        assert_eq!(chooser_figure(5), ("5".to_string(), Some("¢")));
        assert_eq!(chooser_figure(100), ("$1".to_string(), None));
        const { assert!(CENT_SIGN < 0.8) };
    }

    /// Club footage: the teardrop shows a card-back lattice through the
    /// gamble with no legend (a pack without that art keeps the dark
    /// teardrop), and stays dark on the chooser.
    #[test]
    fn the_teardrop_shows_a_lattice_in_the_gamble() {
        let mut look = club_look(DeckMode::Gamble);
        assert_eq!(look.face(CabButton::Spin), "spin_lattice");
        assert!(look.words(CabButton::Spin).is_empty());
        assert_eq!(
            resolve_face("spin_lattice", |k| k == "spin_dim"),
            ("spin_dim", Color::WHITE)
        );
        assert_eq!(
            resolve_face("spin_lattice", |_| true),
            ("spin_lattice", Color::WHITE)
        );
        look.deck.mode = DeckMode::Menu;
        assert_eq!(look.face(CabButton::Spin), "spin_dim");
    }

    /// The picked button fills with the card's face, not at the press (club
    /// cabinets: _XHUzg4qDXI 0:07.87 and 1:16.800, iHDxUc1dlKs 0:12.433).
    #[test]
    fn the_picked_button_fills_with_the_card_face() {
        let mut gamble = GambleShow {
            active: true,
            last: Some((GamblePick::Red, 20)),
            card: Some((3, true, 0.0)),
            ..GambleShow::default()
        };
        assert_eq!(gamble.picked(), None, "the card still face down");
        gamble.card = Some((3, true, GAMBLE_FACE_DELAY - 0.01));
        assert_eq!(gamble.picked(), None);
        gamble.card = Some((3, true, GAMBLE_FACE_DELAY));
        assert_eq!(gamble.picked(), Some(GamblePick::Red));
        gamble.card = Some((3, false, GAMBLE_FACE_DELAY + GAMBLE_SHOW_LOSS - 0.01));
        assert_eq!(gamble.picked(), Some(GamblePick::Red));
        gamble.card = None;
        assert_eq!(gamble.picked(), None, "gone with the card");
    }

    /// Nothing of the base screen stands over the gamble: a tall wild hides
    /// while it shows and stands again after (club cabinets: LwC3hVqFRMg
    /// 18:30.4-18:33.7, _XHUzg4qDXI 1:13.00).
    #[test]
    fn a_tall_wild_gives_way_to_the_gamble() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let mut art = ArtPack::default();
        art.anims
            .insert("wild3_intro".to_string(), vec![Handle::default()]);
        world.insert_resource(art);
        world.insert_resource(Fidelity(true));
        world.insert_resource(Time::<()>::default());
        world.insert_resource(FeatureShow::default());
        world.insert_resource(crate::looks::TitleLook::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(WildStacks {
            list: vec![(1, 0, 3, 0.0)],
            ..WildStacks::default()
        });
        world.insert_resource(GambleShow {
            active: true,
            ..GambleShow::default()
        });
        let tall = world
            .spawn((
                WildTall { reel: 1 },
                Node::default(),
                ImageNode::default(),
                Visibility::Inherited,
            ))
            .id();
        world.run_system_once(wild_talls).unwrap();
        assert_eq!(world.get::<Visibility>(tall), Some(&Visibility::Hidden));
        world.resource_mut::<GambleShow>().active = false;
        world.run_system_once(wild_talls).unwrap();
        assert_eq!(world.get::<Visibility>(tall), Some(&Visibility::Inherited));
    }
}
