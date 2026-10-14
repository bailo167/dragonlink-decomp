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
//! An ordinary start opens a launcher that lists the four titles; Up and Down
//! choose and Enter (or a number key) plays. A `--game` flag or a smoke run
//! skips the launcher and boots straight into a game.
//!
//! Controls in a game: Space or Enter spins, and steps a Hold & Spin feature;
//! Up and Down change the bet; Left and Right change the lines; Escape quits
//! (not in a browser, where there is no window to close).
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
use dragonlink_model::fixtures::{
    autumn_moon_synthetic, golden_century_synthetic, happy_prosperous_synthetic,
    panda_magic_synthetic, synthetic_dev_fixture,
};
use dragonlink_model::{JackpotTier, OrbValue, SymbolKind};

mod cabinet;

/// Testing only (`DRAGONLINK_FEATURE_TEST`): one orb inserted after every this
/// many stops of each base reel, and one scatter after every this many, so a
/// feature starts about one game in three. Synthetic either way; never a
/// claim about the original.
const FEATURE_TEST_ORB: usize = 3;
const FEATURE_TEST_SCATTER: usize = 6;

/// The model with extra orbs and scatters spread through its base reels (see
/// [`FEATURE_TEST_ORB`]).
fn feature_test(mut model: MathModel, orb_every: usize, scatter_every: usize) -> MathModel {
    let id = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .and_then(|i| u16::try_from(i).ok())
            .map(dragonlink_model::SymbolId)
    };
    let (Some(orb), Some(scatter)) = (id("orb"), id("pennant")) else {
        return model;
    };
    for strip in &mut model.reel_strips {
        let mut padded = Vec::with_capacity(strip.len() * 2);
        for (i, &s) in strip.iter().enumerate() {
            padded.push(s);
            if (i + 1) % orb_every == 0 {
                padded.push(orb);
            }
            if (i + 1) % scatter_every == 0 {
                padded.push(scatter);
            }
        }
        *strip = padded;
    }
    model
}

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
/// Seconds between symbol changes while a reel is spinning. Presentation only.
const REEL_CYCLE_SECONDS: f32 = 0.05;
/// Background of a cell on a winning line, and of a feature banner.
const WIN_COLOR: Color = Color::srgb(0.85, 0.62, 0.12);
/// Seconds a feature or jackpot banner stays on screen.
const BANNER_HOLD: f32 = 1.6;

const USAGE: &str = "usage: dragonlink \n[--game golden-century|autumn-moon|happy-prosperous|panda-magic|synthetic] \n[--smoke-frames N [--screenshot FILE]] [--seed N] [--credits N]";

/// Which math configuration the application runs. Both are synthetic: Golden
/// Century has the game's evidenced structure with invented numbers, the dev
/// fixture is deliberately unlike any title (`dragonlink_model::fixtures`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameChoice {
    GoldenCentury,
    AutumnMoon,
    HappyProsperous,
    PandaMagic,
    Synthetic,
}

impl GameChoice {
    fn model(self) -> MathModel {
        match self {
            GameChoice::GoldenCentury if std::env::var_os("DRAGONLINK_FEATURE_TEST").is_some() => {
                feature_test(
                    golden_century_synthetic(),
                    FEATURE_TEST_ORB,
                    FEATURE_TEST_SCATTER,
                )
            }
            GameChoice::GoldenCentury => golden_century_synthetic(),
            GameChoice::AutumnMoon => autumn_moon_synthetic(),
            GameChoice::HappyProsperous => happy_prosperous_synthetic(),
            GameChoice::PandaMagic => panda_magic_synthetic(),
            GameChoice::Synthetic => synthetic_dev_fixture(),
        }
    }

    /// The name shown on screen for this title.
    fn title(self) -> &'static str {
        match self {
            GameChoice::GoldenCentury => "GOLDEN CENTURY",
            GameChoice::HappyProsperous => "HAPPY & PROSPEROUS",
            GameChoice::PandaMagic => "PANDA MAGIC",
            GameChoice::AutumnMoon => "AUTUMN MOON",
            GameChoice::Synthetic => "DRAGONLINK (synthetic fixture)",
        }
    }
}

/// The four titles in scope, in the order the launcher menu lists them. The
/// synthetic development fixture is not a title and is offered only by flag.
const MENU_GAMES: [GameChoice; 4] = [
    GameChoice::GoldenCentury,
    GameChoice::HappyProsperous,
    GameChoice::PandaMagic,
    GameChoice::AutumnMoon,
];

/// Which screen the application is on: the title launcher, or a game.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum Screen {
    /// The four-title launcher. Shown when the application starts with no
    /// `--game` and no smoke run; the player picks a title to play.
    #[default]
    Menu,
    /// A game is running.
    Playing,
}

/// What the launcher needs to build the chosen game once the player picks one.
/// Present only while the menu is in use (a flag or smoke run skips it).
#[derive(Resource)]
struct Launch {
    seed: u64,
    credits: u64,
    /// The menu row the player is on, an index into [`MENU_GAMES`].
    selected: usize,
}

/// The launcher's root node, so it can be removed when a game begins.
#[derive(Component)]
struct MenuRoot;

/// A launcher row, carrying its index into [`MENU_GAMES`] so it can be
/// highlighted when it is the selected one.
#[derive(Component)]
struct MenuItem {
    index: usize,
}

/// The denomination, purely for display: the simulation works in credits, and
/// this says how much money one credit stands for. Changing it changes only
/// the dollar figures shown, never play. These are simulation dollars and
/// stand for no real money.
#[derive(Resource, Default)]
struct Denom {
    index: usize,
}

/// Cents per credit for each denomination, cheapest first.
const DENOM_CENTS: [u64; 5] = [1, 2, 5, 10, 100];

impl Denom {
    fn cents(&self) -> u64 {
        DENOM_CENTS.get(self.index).copied().unwrap_or(1)
    }

    fn label(&self) -> String {
        match self.cents() {
            100 => "$1".to_string(),
            c => format!("{c}c"),
        }
    }
}

/// `credits` of play money as dollars, given cents per credit.
fn money(credits: u64, cents: u64) -> String {
    let total = credits.saturating_mul(cents);
    format!("${}.{:02}", grouped(total / 100), total % 100)
}

/// A whole number with thousands separators, as the cabinet's meters show it.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
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
const CONTROLS_HINT: &str = "Space: spin   Up/Down: bet   Left/Right: lines   D: denomination   Esc: quit   (features play themselves)";
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
                    Some("autumn-moon") => GameChoice::AutumnMoon,
                    Some("happy-prosperous") => GameChoice::HappyProsperous,
                    Some("panda-magic") => GameChoice::PandaMagic,
                    Some("synthetic") => GameChoice::Synthetic,
                    _ => {
                        return Err("--game is golden-century, autumn-moon, \nhappy-prosperous, panda-magic or synthetic"
                            .to_string());
                    }
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
    /// A large banner announcing a feature or a jackpot, shown while
    /// `banner_timer` is positive and then cleared.
    banner: String,
    banner_timer: f32,
}

/// Which reels are spinning, for the reel animation.
#[derive(Resource, Default)]
struct ReelAnim {
    spinning: Vec<bool>,
    timer: f32,
    phase: usize,
}

#[derive(Resource)]
struct Smoke {
    frames_left: Option<u32>,
    screenshot: Option<String>,
    frame: u32,
    /// Frames for which the automatic key press is withheld.
    hold: u32,
    /// Run on the clock rather than a fixed step (the cabinet build, so an
    /// unattended run keeps the machine's real pace for recording).
    real_time: bool,
}

impl Smoke {
    /// Seconds this frame counts for.
    fn dt(&self, time: &Time) -> f32 {
        if self.frames_left.is_some() && !self.real_time {
            SMOKE_FRAME_SECONDS
        } else {
            time.delta_secs()
        }
    }
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

/// An overlay on a cell that shows the authentic symbol image when the asset
/// pack provides one.
#[derive(Component)]
struct CellImage {
    reel: u8,
    row: u8,
}

/// Authentic artwork loaded from a local asset pack. `symbols` is keyed by the
/// model's symbol key (e.g. "emperor", "warriors", "9"); `ui` by the UI image
/// stem ("jackpot_bar", "reel_background"). Empty when no pack is configured,
/// in which case the placeholder presentation shows instead.
#[derive(Resource, Default)]
struct ArtPack {
    symbols: std::collections::HashMap<String, Handle<Image>>,
    ui: std::collections::HashMap<String, Handle<Image>>,
    /// Pixel size of each symbol image, for drawing it at its own size.
    sizes: std::collections::HashMap<String, (f32, f32)>,
    /// The game's animations, as frame sequences (`anim/<name>/NNNN.png`).
    anims: std::collections::HashMap<String, Vec<Handle<Image>>>,
    /// Whether the pack has been read (loading happens once, on first use).
    loaded: bool,
}

#[derive(Component)]
struct JackpotMeter {
    tier: JackpotTier,
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct StatusText;

/// The large centered banner that announces a feature or a jackpot.
#[derive(Component)]
struct FeatureBanner;

/// Give each event of one command its delay. Reels stop one after another;
/// everything else follows at a steady pace.
#[cfg(test)]
fn schedule(events: Vec<Event>) -> Vec<(f32, Event)> {
    schedule_with(events, &Timing::default())
}

/// When the reels stop and events follow. The developer build keeps a quick
/// pace; the cabinet build uses the machine's pace.
#[derive(Resource, Clone, Copy)]
struct Timing {
    lead_in: f32,
    reel_interval: f32,
    event_interval: f32,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            lead_in: SPIN_LEAD_IN,
            reel_interval: REEL_STOP_INTERVAL,
            event_interval: EVENT_INTERVAL,
        }
    }
}

fn schedule_with(events: Vec<Event>, timing: &Timing) -> Vec<(f32, Event)> {
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
                    timing.lead_in
                }
                Event::ReelStopped { .. } => timing.reel_interval,
                // Line wins are lit together, not one by one.
                Event::OrbValueShown { .. } | Event::LineWin { .. } => 0.0,
                _ => timing.event_interval,
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

/// A generated colour for a symbol. Placeholder art: the colours are ours, and
/// stand in for artwork we do not have. They are not any game's colours.
fn cell_color(kind: Option<SymbolKind>, key: &str) -> Color {
    match kind {
        Some(SymbolKind::Wild) => Color::srgb(0.55, 0.25, 0.65),
        Some(SymbolKind::Scatter) => Color::srgb(0.15, 0.50, 0.55),
        Some(SymbolKind::Orb) => Color::srgb(0.80, 0.60, 0.10),
        _ => {
            // A distinct hue per symbol key, so the tiles read apart. Royals
            // (short keys) are cooler, pictures (words) warmer.
            let n = key
                .bytes()
                .fold(0u32, |a, b| a.wrapping_mul(131).wrapping_add(b.into()));
            let hue = (n % 360) as f32;
            let picture = key.len() > 2;
            let sat = if picture { 0.45 } else { 0.30 };
            let light = if picture { 0.32 } else { 0.26 };
            Color::hsl(hue, sat, light)
        }
    }
}

fn setup(
    mut commands: Commands,
    sim: Res<Sim>,
    asset_server: Res<AssetServer>,
    mut art: ResMut<ArtPack>,
    fidelity: Res<cabinet::Fidelity>,
    mut fonts: ResMut<Assets<Font>>,
    mut sounds: ResMut<cabinet::Sounds>,
) {
    // The initial state's OnEnter runs before Startup systems, so the pack is
    // read here, on first use.
    load_art(&asset_server, &mut art);
    let model = sim.game.model();
    if fidelity.0 {
        let cab_fonts = cabinet::load_fonts(&mut fonts);
        if let Ok(dir) = std::env::var("DRAGONLINK_ART") {
            *sounds = cabinet::Sounds::load(&asset_server, &dir);
        }
        cabinet::spawn(&mut commands, model, &art, &cab_fonts);
        return;
    }
    let art = &*art;
    let title = if model.id.starts_with("golden-century") {
        "GOLDEN CENTURY"
    } else if model.id.starts_with("autumn-moon") {
        "AUTUMN MOON"
    } else if model.id.starts_with("happy-prosperous") {
        "HAPPY & PROSPEROUS"
    } else if model.id.starts_with("panda-magic") {
        "PANDA MAGIC"
    } else {
        "DRAGONLINK (synthetic fixture)"
    };
    let gold = Color::srgb(1.0, 0.80, 0.28);
    let muted = Color::srgb(0.62, 0.66, 0.72);
    let tiers = [
        (JackpotTier::Grand, Color::srgb(0.85, 0.20, 0.25)),
        (JackpotTier::Major, Color::srgb(0.25, 0.70, 0.35)),
        (JackpotTier::Minor, Color::srgb(0.30, 0.55, 0.90)),
        (JackpotTier::Mini, Color::srgb(0.90, 0.60, 0.20)),
    ];
    // Full-screen themed background (the Great Wall scene), behind everything,
    // when the pack supplies it.
    if let Some(bg) = art.ui.get("background") {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(bg.clone()),
            GlobalZIndex(-1),
        ));
    }
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|root| {
            // Title.
            root.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(34.0),
                    ..default()
                },
                TextColor(gold),
            ));
            // Jackpot bar: the authentic bar image with live amounts overlaid
            // when the pack supplies it, otherwise coloured tier boxes.
            if let Some(bar_img) = art.ui.get("jackpot_bar") {
                root.spawn((
                    Node {
                        width: Val::Px(760.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                    ImageNode::new(bar_img.clone()),
                ))
                .with_children(|bar| {
                    // One amount over each segment (Grand, Major, Minor, Mini).
                    for (tier, frac) in [
                        (JackpotTier::Grand, 0.16),
                        (JackpotTier::Major, 0.41),
                        (JackpotTier::Minor, 0.66),
                        (JackpotTier::Mini, 0.91),
                    ] {
                        bar.spawn((
                            JackpotMeter { tier },
                            Text::new("-"),
                            TextFont {
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::justify(Justify::Center),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(frac * 100.0 - 11.0),
                                top: Val::Px(36.0),
                                width: Val::Percent(22.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                        ));
                    }
                });
            } else {
                root.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|bar| {
                    for (tier, colour) in tiers {
                        bar.spawn((
                            Node {
                                width: Val::Px(150.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(2.0),
                                ..default()
                            },
                            BackgroundColor(colour.with_alpha(0.22)),
                        ))
                        .with_children(|box_| {
                            box_.spawn((
                                Text::new(format!("{tier:?}").to_uppercase()),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(colour),
                            ));
                            box_.spawn((
                                JackpotMeter { tier },
                                Text::new("-"),
                                TextFont {
                                    font_size: FontSize::Px(18.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });
            }
            // An initial reel window, drawn from the strips, so the machine
            // shows symbols the moment it opens instead of blanks.
            let initial = |reel: u8, row: u8| -> (String, Option<SymbolKind>) {
                let strip = &model.reel_strips[reel as usize];
                if strip.is_empty() {
                    return ("-".to_string(), None);
                }
                let stop = (reel as usize * 7 + 2) % strip.len();
                let id = strip[(stop + row as usize) % strip.len()];
                let def = &model.symbols[id.0 as usize];
                (def.key.clone(), Some(def.kind))
            };
            // Reels, on the authentic reel backdrop when the pack supplies it,
            // otherwise a plain dark panel.
            let mut reel_panel = root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.07, 0.10)),
            ));
            if let Some(backdrop) = art.ui.get("reel_background") {
                reel_panel.insert(ImageNode::new(backdrop.clone()));
            }
            reel_panel.with_children(|reels| {
                for reel in 0..model.grid.reels {
                    reels
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|column| {
                            for row in 0..model.grid.rows {
                                let (key0, kind0) = initial(reel, row);
                                column
                                    .spawn((
                                        Cell { reel, row },
                                        Node {
                                            width: Val::Px(116.0),
                                            height: Val::Px(100.0),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        // On the authentic reel panel the
                                        // symbol sits directly on the face, with
                                        // no coloured tile behind it.
                                        BackgroundColor(
                                            if art.ui.contains_key("reel_background") {
                                                Color::srgba(0.0, 0.0, 0.0, 0.0)
                                            } else {
                                                cell_color(kind0, &key0)
                                            },
                                        ),
                                    ))
                                    .with_children(|cell| {
                                        cell.spawn((
                                            CellLabel { reel, row },
                                            Text::new(key0.clone()),
                                            TextFont {
                                                font_size: FontSize::Px(20.0),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                            TextLayout::justify(Justify::Center),
                                        ));
                                        // Authentic symbol art, drawn over the
                                        // cell when the asset pack supplies it
                                        // (see `render_symbols`); hidden until
                                        // then, so placeholder text shows.
                                        cell.spawn((
                                            CellImage { reel, row },
                                            Node {
                                                position_type: PositionType::Absolute,
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            ImageNode::default(),
                                            Visibility::Hidden,
                                        ));
                                    });
                            }
                        });
                }
            });
            // Meters.
            root.spawn((
                HudText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // Status line.
            root.spawn((
                StatusText,
                Text::new(CONTROLS_HINT),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(muted),
            ));
            // Compact, persistent disclaimer (docs/LEGAL.md).
            root.spawn((
                Text::new(
                    "Synthetic reimplementation - generated art, invented maths. Not Dragon Link, \
not evidence of any real game, simulation credits only (no real money).",
                ),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.62, 0.20)),
                TextLayout::justify(Justify::Center),
            ));
        });
    // A full-screen overlay for the feature/jackpot banner, on top of the
    // game. Empty until an event raises it (see `present` and `banner`).
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|overlay| {
            overlay.spawn((
                FeatureBanner,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(72.0),
                    ..default()
                },
                TextColor(gold),
                TextLayout::justify(Justify::Center),
            ));
        });
}

/// Show and fade the feature/jackpot banner while its timer runs.
fn banner(
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut presentation: ResMut<Presentation>,
    mut banners: Query<(&mut Text, &mut TextColor), With<FeatureBanner>>,
) {
    if presentation.banner_timer > 0.0 {
        presentation.banner_timer -= smoke.dt(&time);
    }
    let showing = presentation.banner_timer > 0.0 && !presentation.banner.is_empty();
    // Fade out over the last part of the hold.
    let alpha = (presentation.banner_timer / (BANNER_HOLD * 0.5)).clamp(0.0, 1.0);
    for (mut text, mut color) in &mut banners {
        let wanted = if showing { &presentation.banner } else { "" };
        if text.0 != wanted {
            text.0 = wanted.to_string();
        }
        color.0 = Color::srgb(1.0, 0.80, 0.28).with_alpha(if showing { alpha } else { 0.0 });
    }
}

/// Turn key presses into commands. The simulation answers with events, which
/// are queued for presentation. A refused command only changes the status line.
#[allow(clippy::too_many_arguments)]
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mut smoke: ResMut<Smoke>,
    mut sim: ResMut<Sim>,
    mut presentation: ResMut<Presentation>,
    mut denom: ResMut<Denom>,
    mut exit: MessageWriter<AppExit>,
    timing: Res<Timing>,
    cabinet_input: Res<cabinet::CabinetInput>,
    feature: Res<cabinet::FeatureShow>,
    mut gamble: ResMut<cabinet::GambleShow>,
) {
    // Denomination (display only) cycles with D, while idle.
    let at_rest = sim.game.is_idle() && presentation.queue.is_empty() && !gamble.active;
    if keys.just_pressed(KeyCode::KeyD) && at_rest {
        denom.index = (denom.index + 1) % DENOM_CENTS.len();
    }
    if let Some(i) = cabinet_input.denom
        && at_rest
        && let Some(pos) = DENOM_CENTS
            .iter()
            .position(|c| *c == cabinet::DECK_DENOMS[i])
    {
        denom.index = pos;
    }
    // A denomination allows a number of lines; more than that are dropped.
    let max_lines = cabinet::max_lines(denom.cents());
    let lines_request = cabinet_input.lines.filter(|n| *n <= max_lines);
    let lines_clamp = (at_rest && sim.game.lines() > max_lines).then(|| {
        sim.game
            .model()
            .line_counts()
            .into_iter()
            .filter(|n| *n <= max_lines)
            .max()
            .unwrap_or(1)
    });
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
    let auto = auto && !gamble.holds_autospin(sim.game.can_gamble());
    let go = (auto
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || cabinet_input.spin)
        && !cabinet_input.skip_spin
        && sim.game.is_idle()
        // A feature still on screen (its count, its banner) finishes first.
        && presentation.queue.is_empty()
        && feature.awaiting.is_none()
        && !feature.blocks()
        && !gamble.active;
    if go {
        // A new game: the gamble's result leaves the WIN meter.
        gamble.result = None;
    }
    // In the cabinet build an unattended run pauses between games, so each
    // result and win can be seen (and captured) at rest.
    if go && auto && timing.lead_in > SPIN_LEAD_IN {
        smoke.hold = 330;
    }
    let bets = sim.game.model().bet.credits_per_line_options.len();
    let bet_index = sim.game.state().bet_index;
    let line_counts = sim.game.model().line_counts();
    let here = line_counts
        .iter()
        .position(|n| *n == sim.game.lines())
        .unwrap_or(0);
    let command = if go {
        Some(Command::Spin)
    } else if let Some(lines) = lines_clamp.or(lines_request) {
        Some(Command::SetLines { lines })
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
            presentation.queue.extend(schedule_with(events, &timing));
            presentation.wait = presentation.queue.front().map_or(0.0, |(delay, _)| *delay);
        }
        Err(refusal) => presentation.status = format!("{refusal}"),
    }
}

/// While a reel is spinning, run its cells through the symbols so the window
/// reads as moving. Presentation only; the outcome is already decided.
fn animate_reels(
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    mut reels_spin: ResMut<ReelAnim>,
    mut cells: Query<(&Cell, &mut BackgroundColor)>,
    mut labels: Query<(&CellLabel, &mut Text)>,
    fidelity: Res<cabinet::Fidelity>,
) {
    // The cabinet build draws real moving strips instead (cabinet::spin_reels).
    if fidelity.0 || reels_spin.spinning.iter().all(|s| !*s) {
        return;
    }
    let dt = smoke.dt(&time);
    reels_spin.timer -= dt;
    if reels_spin.timer > 0.0 {
        return;
    }
    reels_spin.timer = REEL_CYCLE_SECONDS;
    reels_spin.phase = reels_spin.phase.wrapping_add(1);
    let model = sim.game.model();
    // Symbols that read while spinning: everything but the orb and scatter, so
    // the blur does not look like a pending feature.
    let rolling: Vec<&dragonlink_model::SymbolDef> = model
        .symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Regular | SymbolKind::Wild))
        .collect();
    if rolling.is_empty() {
        return;
    }
    let spin_bg = Color::srgb(0.10, 0.11, 0.15);
    for reel in 0..model.grid.reels {
        if !reels_spin
            .spinning
            .get(reel as usize)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        for row in 0..model.grid.rows {
            let pick = (reels_spin.phase + reel as usize * 5 + row as usize * 3) % rolling.len();
            let key = rolling[pick].key.clone();
            for (cell, mut background) in &mut cells {
                if cell.reel == reel && cell.row == row {
                    background.0 = spin_bg;
                }
            }
            for (label, mut text) in &mut labels {
                if label.reel == reel && label.row == row {
                    text.0.clone_from(&key);
                }
            }
        }
    }
}

/// Play a feature's steps by itself, with a pause between them, so the player
/// does not drive Hold & Spin or the free games by hand.
fn drive_features(
    time: Res<Time>,
    smoke: Res<Smoke>,
    mut sim: ResMut<Sim>,
    mut presentation: ResMut<Presentation>,
    timing: Res<Timing>,
    feature: Res<cabinet::FeatureShow>,
    rollup: Res<cabinet::Rollup>,
) {
    // The cabinet waits for START FEATURE, for SPIN between respins, and for
    // a free game's win to count up before the next game.
    if feature.awaiting.is_some() || feature.blocks() || rollup.active() {
        presentation.step_timer = FEATURE_STEP_PAUSE;
        return;
    }
    if !presentation.queue.is_empty() {
        presentation.step_timer = FEATURE_STEP_PAUSE;
        return;
    }
    if sim.game.is_idle() {
        return;
    }
    let dt = smoke.dt(&time);
    presentation.step_timer -= dt;
    if presentation.step_timer > 0.0 {
        return;
    }
    presentation.step_timer = FEATURE_STEP_PAUSE;
    let Sim { game, rng } = &mut *sim;
    if let Ok(events) = game.apply(Command::Advance, rng) {
        presentation.queue.extend(schedule_with(events, &timing));
        presentation.wait = presentation.queue.front().map_or(0.0, |(delay, _)| *delay);
    }
}

/// Show queued events when their time comes.
#[allow(clippy::too_many_arguments)]
fn present(
    time: Res<Time>,
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    fidelity: Res<cabinet::Fidelity>,
    mut feature: ResMut<cabinet::FeatureShow>,
    mut presentation: ResMut<Presentation>,
    mut reels_spin: ResMut<ReelAnim>,
    mut cells: Query<(&Cell, &mut BackgroundColor)>,
    mut labels: Query<(&CellLabel, &mut Text)>,
) {
    let reels = sim.game.model().grid.reels as usize;
    if reels_spin.spinning.len() != reels {
        reels_spin.spinning = vec![false; reels];
    }
    presentation.wait -= smoke.dt(&time);
    while presentation.wait <= 0.0 {
        // A respin spinning or the coins being counted hold what follows.
        if fidelity.0 && feature.blocks() {
            presentation.wait = 0.0;
            break;
        }
        let Some((_, event)) = presentation.queue.pop_front() else {
            presentation.wait = 0.0;
            break;
        };
        if fidelity.0 {
            feature.on_event(&event);
        }
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
                // Every reel starts spinning; `animate_reels` fills the window
                // until each reel's stop arrives.
                for s in reels_spin.spinning.iter_mut() {
                    *s = true;
                }
                // A new base spin clears any lingering feature banner.
                presentation.banner.clear();
                presentation.banner_timer = 0.0;
                presentation.status = format!("Spin {spin}");
            }
            Event::ReelStopped { reel, symbols, .. } => {
                if let Some(s) = reels_spin.spinning.get_mut(*reel as usize) {
                    *s = false;
                }
                for (row, key) in symbols.iter().enumerate() {
                    let kind = model.symbols.iter().find(|s| &s.key == key).map(|s| s.kind);
                    set_cell(*reel, row as u8, key.clone(), cell_color(kind, key));
                }
                // The machine letters an orb's prize as its reel lands; the
                // engine reports the values after the last reel, so look ahead.
                let landed = *reel;
                let ahead: Vec<(u8, OrbValue)> = presentation
                    .queue
                    .iter()
                    .filter_map(|(_, e)| match e {
                        Event::OrbValueShown { reel, row, value } if *reel == landed => {
                            Some((*row, *value))
                        }
                        _ => None,
                    })
                    .collect();
                for (row, value) in ahead {
                    set_cell(landed, row, orb_text(value), orb_color);
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
                presentation.banner = "FREE GAMES".to_string();
                presentation.banner_timer = BANNER_HOLD;
                presentation.status = format!("FREE GAMES  -  {games} games won");
            }
            Event::FreeGameStarted { number, remaining } => {
                // Each free game spins the reels like a paid game.
                for s in reels_spin.spinning.iter_mut() {
                    *s = true;
                }
                presentation.status = format!("FREE GAME {number}  ({remaining} to come)");
            }
            Event::MysteryRevealed { symbol, count } => {
                presentation.status = format!("Mystery reveals {symbol} x{count}");
            }
            Event::FreeGamesRetriggered { added, remaining } => {
                presentation.banner = "RETRIGGER".to_string();
                presentation.banner_timer = BANNER_HOLD;
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
                // Everything that is not a held orb is cleared for the feature
                // (the cabinet dims it during its trigger sequence instead).
                if !fidelity.0 {
                    for reel in 0..model.grid.reels {
                        for row in 0..model.grid.rows {
                            set_cell(reel, row, String::new(), Color::srgb(0.07, 0.07, 0.09));
                        }
                    }
                }
                for orb in locked {
                    set_cell(orb.reel, orb.row, orb_text(orb.value), orb_color);
                }
                presentation.banner = "HOLD & SPIN".to_string();
                presentation.banner_timer = BANNER_HOLD;
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
                presentation.banner = format!("{tier:?} JACKPOT").to_uppercase();
                presentation.banner_timer = BANNER_HOLD;
                presentation.status = format!("{tier:?} awarded: {award}");
            }
            Event::HoldAndSpinCompleted {
                total_award,
                full_grid,
            } => {
                if *full_grid {
                    presentation.banner = "FULL GRID".to_string();
                    presentation.banner_timer = BANNER_HOLD;
                }
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

#[allow(clippy::type_complexity)]
fn hud(
    sim: Res<Sim>,
    presentation: Res<Presentation>,
    denom: Res<Denom>,
    mut hud: Single<&mut Text, (With<HudText>, Without<StatusText>)>,
    mut status: Single<&mut Text, (With<StatusText>, Without<HudText>)>,
    mut meters: Query<(&JackpotMeter, &mut Text), (Without<HudText>, Without<StatusText>)>,
) {
    let game = &sim.game;
    let cents = denom.cents();
    let state = if !presentation.queue.is_empty() {
        "SPINNING"
    } else if game.is_idle() {
        "READY"
    } else {
        "FEATURE"
    };
    hud.0 = format!(
        "CREDITS {} ({})     {}  BET {} ({} x {} lines)     WIN {}     {state}",
        presentation.shown_credits,
        money(presentation.shown_credits, cents),
        denom.label(),
        money(game.total_bet(), cents),
        game.credits_per_line(),
        game.lines(),
        money(presentation.shown_win, cents),
    );
    if !presentation.status.is_empty() {
        status.0.clone_from(&presentation.status);
    }
    // Jackpot meters: synthetic fixed amounts for the current bet.
    let total_bet = game.total_bet();
    for (meter, mut text) in &mut meters {
        let amount = game
            .model()
            .hold_and_spin
            .as_ref()
            .and_then(|hs| hs.jackpots.iter().find(|j| j.tier == meter.tier))
            .map(|j| u64::from(j.total_bet_multiple).saturating_mul(total_bet));
        text.0 = amount.map_or_else(|| "-".to_string(), |c| money(c, cents));
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

/// One camera for both screens, spawned once at startup.
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Load authentic symbol art from the asset pack, if one is configured through
/// the `DRAGONLINK_ART` environment variable (its `symbols/<key>.png` files).
/// Only files that exist are loaded; a symbol with no file keeps its
/// placeholder tile. Nothing proprietary ships in the repository: the pack is
/// a local, git-ignored set built from the owner's own source material.
fn load_art(asset_server: &AssetServer, pack: &mut ArtPack) {
    if pack.loaded {
        return;
    }
    pack.loaded = true;
    if let Ok(dir) = std::env::var("DRAGONLINK_ART") {
        let root = std::path::Path::new(&dir);
        load_pngs(asset_server, &root.join("symbols"), "symbols", pack, true);
        load_pngs(asset_server, &root.join("ui"), "ui", pack, false);
        load_anims(asset_server, &root.join("anim"), pack);
        eprintln!(
            "authentic art: {} symbols, {} ui images from {dir}",
            pack.symbols.len(),
            pack.ui.len()
        );
    }
}

/// Load each animation folder under `dir` as an ordered frame sequence.
fn load_anims(asset_server: &AssetServer, dir: &std::path::Path, pack: &mut ArtPack) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Ok(files) = std::fs::read_dir(entry.path()) else {
            continue;
        };
        let mut stems: Vec<String> = files
            .flatten()
            .filter_map(|f| {
                let p = f.path();
                (p.extension().and_then(|e| e.to_str()) == Some("png"))
                    .then(|| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
                    .flatten()
            })
            .collect();
        stems.sort();
        let frames = stems
            .iter()
            .map(|stem| asset_server.load(format!("anim/{name}/{stem}.png")))
            .collect();
        pack.anims.insert(name, frames);
    }
}

/// Load every `*.png` in `dir`, keyed by file stem, through the asset server
/// at `<sub>/<stem>.png`; symbol images also record their pixel size.
fn load_pngs(
    asset_server: &AssetServer,
    dir: &std::path::Path,
    sub: &str,
    pack: &mut ArtPack,
    symbols: bool,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let handle = asset_server.load(format!("{sub}/{stem}.png"));
                if let Some(size) = cabinet::png_size(&path) {
                    pack.sizes.insert(stem.to_string(), size);
                }
                if symbols {
                    pack.symbols.insert(stem.to_string(), handle);
                } else {
                    pack.ui.insert(stem.to_string(), handle);
                }
            }
        }
    }
}

/// Show the authentic symbol image on each cell whose current symbol has art,
/// over the placeholder tile (which still carries the cell background, so a
/// winning line still lights up behind the symbol). Cells whose symbol has no
/// art, or that are spinning on a non-symbol, keep the placeholder.
#[allow(clippy::type_complexity)]
fn render_symbols(
    time: Res<Time>,
    art: Res<ArtPack>,
    fidelity: Res<cabinet::Fidelity>,
    reels: Res<cabinet::Reels>,
    labels: Query<(&CellLabel, &Text)>,
    mut images: Query<(&CellImage, &mut ImageNode, &mut Visibility, &mut Node)>,
) {
    if art.symbols.is_empty() {
        return;
    }
    let (cw, ch) = cabinet::cell_size();
    for (cell, mut image, mut visibility, mut node) in &mut images {
        if fidelity.0 && reels.moving(cell.reel) {
            *visibility = Visibility::Hidden;
            continue;
        }
        let key = labels
            .iter()
            .find(|(l, _)| l.reel == cell.reel && l.row == cell.row)
            .map(|(_, text)| text.0.as_str());
        // A prize symbol's label carries its value ("ORB\nx4"); it is an orb.
        // An open Hold & Spin position shows its symbol dimmed ("DIM:K").
        let (key, dim) = key.map_or((None, false), |k| {
            let (k, dim) = cabinet::label_symbol(k);
            (Some(k), dim)
        });
        let tint = if dim { cabinet::DIM_TINT } else { Color::WHITE };
        if image.color != tint {
            image.color = tint;
        }
        match key.and_then(|k| art.symbols.get(k).map(|h| (k, h))) {
            Some((k, handle)) => {
                // The coin burns: its frames loop.
                let handle = if fidelity.0 {
                    cabinet::symbol_image(&art, k, time.elapsed_secs())
                        .unwrap_or_else(|| handle.clone())
                } else {
                    handle.clone()
                };
                if image.image != handle {
                    image.image = handle;
                }
                if fidelity.0 {
                    // The game draws each symbol at its own size, centred on
                    // the cell; large symbols overhang their neighbours.
                    let (w, h) = art
                        .sizes
                        .get(k)
                        .map_or((cw, ch), |s| cabinet::symbol_size(k, *s));
                    // A dimmed symbol on an open Hold & Spin position stays
                    // inside its position (large ones otherwise overhang).
                    let (w, h) = if dim {
                        let k = (cw / w).min(ch / h).min(1.0) * 0.96;
                        (w * k, h * k)
                    } else {
                        (w, h)
                    };
                    if node.width != Val::Px(w) || node.height != Val::Px(h) {
                        node.left = Val::Px((cw - w) / 2.0);
                        node.top = Val::Px((ch - h) / 2.0);
                        node.width = Val::Px(w);
                        node.height = Val::Px(h);
                    }
                }
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// The colour of a launcher row: gold when it is the selected one.
fn menu_row_color(selected: bool) -> Color {
    if selected {
        Color::srgb(1.0, 0.80, 0.28)
    } else {
        Color::srgb(0.78, 0.80, 0.84)
    }
}

/// Build the four-title launcher.
fn menu_setup(mut commands: Commands, launch: Res<Launch>) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("DRAGON LINK"),
                TextFont {
                    font_size: FontSize::Px(44.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.80, 0.28)),
            ));
            root.spawn((
                Text::new("choose a game"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.62, 0.66, 0.72)),
            ));
            for (index, game) in MENU_GAMES.iter().enumerate() {
                root.spawn((
                    MenuItem { index },
                    Text::new(format!("{}.  {}", index + 1, game.title())),
                    TextFont {
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(menu_row_color(index == launch.selected)),
                ));
            }
            root.spawn((
                Text::new("Up / Down to choose, Enter to play, or press 1-4"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.58, 0.64)),
            ));
            root.spawn((
                Text::new(
                    "Synthetic placeholder build: invented maths and generic symbols, not Dragon Link",
                ),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.45, 0.47, 0.52)),
            ));
        });
}

/// Keep the launcher's highlight on the selected row.
fn menu_highlight(launch: Res<Launch>, mut items: Query<(&MenuItem, &mut TextColor)>) {
    for (item, mut color) in &mut items {
        color.0 = menu_row_color(item.index == launch.selected);
    }
}

/// Move through the launcher and start the chosen game.
fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut launch: ResMut<Launch>,
    mut commands: Commands,
    mut presentation: ResMut<Presentation>,
    mut next: ResMut<NextState<Screen>>,
) {
    let count = MENU_GAMES.len();
    if keys.just_pressed(KeyCode::ArrowDown) {
        launch.selected = (launch.selected + 1) % count;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        launch.selected = (launch.selected + count - 1) % count;
    }
    let digit = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .iter()
    .position(|k| keys.just_pressed(*k))
    .filter(|i| *i < count);
    if let Some(i) = digit {
        launch.selected = i;
    }
    let go =
        digit.is_some() || keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    if !go {
        return;
    }
    let game = MENU_GAMES[launch.selected];
    match Game::new(game.model(), launch.credits) {
        Ok(g) => {
            commands.insert_resource(Sim {
                game: g,
                rng: DevRng::new(launch.seed),
            });
            presentation.shown_credits = launch.credits;
            next.set(Screen::Playing);
        }
        Err(e) => eprintln!("cannot start {}: {e}", game.title()),
    }
}

/// Remove the launcher once a game has begun.
fn menu_teardown(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
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
    // The launcher is shown only for an ordinary interactive start. A `--game`
    // flag or a smoke run boots straight into the game, so every automated
    // path (the gate, the tests) stays on the direct path and never waits on a
    // menu.
    // The fidelity build: an authentic pack is configured. It is Golden
    // Century only, boots straight into the game, and refuses to start on an
    // incomplete pack rather than fall back to placeholder tiles.
    let art_dir = std::env::var("DRAGONLINK_ART")
        .ok()
        .filter(|d| !d.is_empty());
    let fidelity = art_dir.is_some();
    if let Some(dir) = &art_dir {
        if cli.game != GameChoice::GoldenCentury {
            eprintln!("the authentic pack is Golden Century's; run with --game golden-century");
            return AppExit::error();
        }
        if let Err(missing) = cabinet::check_pack(dir, &cli.game.model()) {
            eprintln!("authentic pack at {dir} is incomplete; missing:");
            for m in missing {
                eprintln!("  {m}");
            }
            return AppExit::error();
        }
    }
    let explicit_game = args.iter().any(|a| a == "--game");
    let direct_boot = fidelity || explicit_game || cli.smoke_frames.is_some();
    let direct_sim = if direct_boot {
        match Game::new(cli.game.model(), cli.credits) {
            Ok(game) => Some(Sim {
                game,
                rng: DevRng::new(cli.seed),
            }),
            Err(e) => {
                eprintln!("cannot start the game: {e}");
                return AppExit::error();
            }
        }
    } else {
        None
    };

    let mut app = App::new();
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(if fidelity {
            cabinet::window()
        } else {
            primary_window()
        }),
        ..default()
    });
    // When an authentic asset pack is configured, point the asset server at it
    // so `symbols/<key>.png` resolves there. The pack is never in the repo.
    let plugins = match std::env::var("DRAGONLINK_ART") {
        Ok(dir) if !dir.is_empty() => plugins.set(bevy::asset::AssetPlugin {
            file_path: dir,
            ..default()
        }),
        _ => plugins,
    };
    app.add_plugins(plugins);
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Startup, announce_placeholder_mode);
    app.insert_resource(ClearColor(if fidelity {
        Color::BLACK
    } else {
        Color::srgb(0.03, 0.03, 0.05)
    }))
    .insert_resource(cabinet::Fidelity(fidelity))
    .insert_resource(if fidelity {
        cabinet::TIMING
    } else {
        Timing::default()
    })
    .insert_resource(cabinet::CabinetInput::default())
    .insert_resource(cabinet::Sounds::default())
    .insert_resource(cabinet::Rollup::default())
    .insert_resource(cabinet::FeatureShow::default())
    .insert_resource(cabinet::GambleShow::default())
    .insert_resource(cabinet::WildStacks::default())
    .insert_resource(cabinet::Music::default())
    .insert_resource(if fidelity {
        cabinet::reel_strips(&cli.game.model())
    } else {
        cabinet::Reels::default()
    })
    .insert_resource(Presentation {
        shown_credits: cli.credits,
        ..default()
    })
    .insert_resource(Denom::default())
    .insert_resource(ArtPack::default())
    .insert_resource(ReelAnim::default())
    .insert_resource(Smoke {
        frames_left: cli.smoke_frames,
        screenshot: cli.screenshot,
        frame: 0,
        hold: 0,
        real_time: fidelity,
    })
    .add_systems(Startup, spawn_camera)
    .add_systems(OnEnter(Screen::Menu), menu_setup)
    .add_systems(OnExit(Screen::Menu), menu_teardown)
    .add_systems(
        Update,
        (menu_input, menu_highlight).run_if(in_state(Screen::Menu)),
    )
    .add_systems(OnEnter(Screen::Playing), setup)
    .add_systems(
        Update,
        (
            (
                smoke,
                cabinet::rollup_skip,
                cabinet::feature_press,
                cabinet::gamble_control,
                input,
                cabinet::clear_buttons,
                drive_features,
                present,
                animate_reels,
                hud,
                banner,
            )
                .chain(),
            (
                cabinet::spin_reels,
                cabinet::strip_coins,
                cabinet::strip_tiers,
                cabinet::reel_audio,
                cabinet::hold_and_spin_motion,
                cabinet::hold_and_spin_draw,
                cabinet::hs_spin_digits,
                cabinet::winner_plate,
                cabinet::hs_backdrop,
                cabinet::door_reveal,
                cabinet::door_draw,
                cabinet::win_lines,
                cabinet::wild_stacks,
                cabinet::wild_talls,
                cabinet::feature_music,
            )
                .chain(),
            (
                cabinet::win_rollup,
                render_symbols,
                cabinet::orb_overlay,
                cabinet::symbol_fx,
                cabinet::light_deck,
                cabinet::feature_screen,
                cabinet::gamble_screen,
                cabinet::start_prompts,
                cabinet::clear_cells,
                cabinet::update_meters,
                cabinet::fit,
            )
                .chain(),
        )
            .chain()
            .run_if(in_state(Screen::Playing)),
    );

    match direct_sim {
        Some(sim) => {
            app.insert_resource(sim);
            app.insert_state(Screen::Playing);
        }
        None => {
            app.insert_resource(Launch {
                seed: cli.seed,
                credits: cli.credits,
                selected: 0,
            });
            app.init_state::<Screen>();
        }
    }
    app.run()
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
        for (name, choice) in [
            ("golden-century", GameChoice::GoldenCentury),
            ("autumn-moon", GameChoice::AutumnMoon),
            ("happy-prosperous", GameChoice::HappyProsperous),
            ("panda-magic", GameChoice::PandaMagic),
            ("synthetic", GameChoice::Synthetic),
        ] {
            assert_eq!(
                parse_args(&args(&["--game", name])).unwrap().game,
                choice,
                "--game {name} did not parse"
            );
        }
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
    fn meters_group_thousands() {
        assert_eq!(money(102_150, 10), "$10,215.00");
        assert_eq!(money(585, 200), "$1,170.00");
        assert_eq!(money(5, 1), "$0.05");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }

    #[test]
    fn coins_are_lettered_in_credits_at_cents_and_dollars_at_a_dollar() {
        assert_eq!(cabinet::coin_text(20, 1), "20");
        assert_eq!(cabinet::coin_text(20, 100), "$20");
        assert_eq!(cabinet::coin_text(10, 200), "$20");
    }

    #[test]
    fn the_feature_test_model_starts_a_feature_about_one_game_in_three() {
        let model = feature_test(
            golden_century_synthetic(),
            FEATURE_TEST_ORB,
            FEATURE_TEST_SCATTER,
        );
        model.validate().expect("valid");
        let mut game = Game::new(model, 1_000_000_000).unwrap();
        let mut rng = DevRng::new(9);
        let (mut free, mut hold) = (0u32, 0u32);
        let n = 20_000;
        for _ in 0..n {
            for e in &game.play_spin(&mut rng).unwrap() {
                match e {
                    Event::FreeGamesStarted { .. } => free += 1,
                    Event::HoldAndSpinStarted { .. } => hold += 1,
                    _ => {}
                }
            }
        }
        let rate = (free + hold) as f64 / n as f64;
        eprintln!(
            "feature test: free {free}, hold {hold}, rate 1 in {:.1}",
            1.0 / rate
        );
        assert!((0.25..0.45).contains(&rate), "rate {rate}");
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
    fn every_launcher_game_builds_and_has_a_distinct_title() {
        use std::collections::HashSet;
        assert_eq!(MENU_GAMES.len(), 4, "the launcher offers the four titles");
        let mut titles = HashSet::new();
        for game in MENU_GAMES {
            assert!(
                Game::new(game.model(), 1_000).is_ok(),
                "{} does not build",
                game.title()
            );
            assert!(
                titles.insert(game.title()),
                "two launcher rows share the title {}",
                game.title()
            );
        }
        // The selected row is highlighted differently from the rest.
        assert_ne!(menu_row_color(true), menu_row_color(false));
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
