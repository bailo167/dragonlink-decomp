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
use dragonlink_core::{Command, CreditReason, DevRng, Event, Game, OutcomeRng};
use dragonlink_model::MathModel;
use dragonlink_model::fixtures::{
    autumn_moon_synthetic, golden_century_synthetic, happy_prosperous_synthetic,
    panda_magic_synthetic, synthetic_dev_fixture,
};
use dragonlink_model::{JackpotTier, OrbValue, SymbolKind};

mod cabinet;

/// Testing only (`DRAGONLINK_FEATURE_TEST`): one orb inserted after every this
/// many stops of each base reel, and one scatter after every this many, so a
/// feature starts about one game in three. `DRAGONLINK_FEATURE_TEST=flags`
/// adds the scatters only (free games often, Hold & Spin at its usual rate).
/// Synthetic either way; never a claim about the original.
const FEATURE_TEST_ORB: usize = 3;
const FEATURE_TEST_SCATTER: usize = 6;

/// Testing only (`DRAGONLINK_SCENARIO`, Golden Century): prepared runs whose
/// first spin lands three flags, and in most of them six coins too, so Hold &
/// Spin plays and the free games follow; the free games' reels carry the
/// mystery doors often.
/// The outcome source is left alone: its first spin's reel stops are worked
/// out from the seed and the synthetic strips are dressed at those stops.
/// Changes to that run's synthetic model only, never a claim about the
/// original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// `both`: coins land on most respins, so the grid fills for the Grand.
    Both,
    /// `jackpots`: Hold & Spin ends with one Major, four Minis and two Minors
    /// (and no Grand), at [`JACKPOTS_SEED`].
    Jackpots,
    /// `minors`: four Minis and two Minors, no Major or Grand, so the win
    /// may be gambled, at [`MINORS_SEED`].
    Minors,
    /// `doors`: three flags and no Hold & Spin first; in the free games the
    /// doors open onto coins that start Hold & Spin, and in another game onto
    /// the emperor; no Major or Grand, so the win may be gambled, at
    /// [`DOORS_SEED`].
    Doors,
}

/// The seeds the `jackpots` and `minors` runs play: found by
/// `find_jackpot_scenario_seeds` and held by
/// `the_jackpot_scenarios_win_what_they_promise`.
const JACKPOTS_SEED: u64 = 1;
const MINORS_SEED: u64 = 17;
const DOORS_SEED: u64 = 34;

impl Scenario {
    fn from_env() -> Option<Scenario> {
        match std::env::var("DRAGONLINK_SCENARIO").ok()?.as_str() {
            "both" => Some(Scenario::Both),
            "jackpots" => Some(Scenario::Jackpots),
            "minors" => Some(Scenario::Minors),
            "doors" => Some(Scenario::Doors),
            _ => None,
        }
    }

    /// The seed this run plays.
    fn seed(self, seed: u64) -> u64 {
        match self {
            Scenario::Both => seed,
            Scenario::Jackpots => JACKPOTS_SEED,
            Scenario::Minors => MINORS_SEED,
            Scenario::Doors => DOORS_SEED,
        }
    }

    /// This run's model, for the seed it plays. Its jackpots keep their
    /// full chance at every stake, so the run is the same whatever the bet.
    fn model(self, model: MathModel, seed: u64) -> MathModel {
        let mut model = dress_scenario(model, seed, self != Scenario::Doors);
        let orb = model.symbols.iter().position(|s| s.key == "orb");
        // The runs that promise their jackpots keep coins out of their free
        // games (off the doors and off the reels, a nine in each coin's
        // place), so those start no Hold & Spin of their own.
        let nine = model.symbols.iter().position(|s| s.key == "9");
        if self != Scenario::Doors
            && let Some(fg) = model.free_games.as_mut()
        {
            fg.reveal_values
                .retain(|v| Some(usize::from(v.symbol.0)) != orb);
            if let Some(nine) = nine.and_then(|n| u16::try_from(n).ok()) {
                for strip in &mut fg.free_reel_strips {
                    for s in strip.iter_mut() {
                        if Some(usize::from(s.0)) == orb {
                            *s = dragonlink_model::SymbolId(nine);
                        }
                    }
                }
            }
        }
        let Some(hs) = model.hold_and_spin.as_mut() else {
            return model;
        };
        for jackpot in &mut hs.jackpots {
            jackpot.full_chance_stake = None;
            jackpot.full_chance_share = None;
        }
        match self {
            // Coins on most respins, so the grid fills and the Grand is won.
            Scenario::Both => hs.orb_land_chance = dragonlink_model::Ratio { num: 3, den: 4 },
            Scenario::Doors => {}
            Scenario::Jackpots | Scenario::Minors => {
                // Fewer coins per respin, and jackpots on more of them.
                hs.orb_land_chance = dragonlink_model::Ratio { num: 1, den: 6 };
                let major = if self == Scenario::Jackpots { 8 } else { 0 };
                hs.orb_values = [
                    (14, OrbValue::BetMultiple(1)),
                    (12, OrbValue::BetMultiple(2)),
                    (10, OrbValue::BetMultiple(3)),
                    (8, OrbValue::BetMultiple(5)),
                    (6, OrbValue::BetMultiple(10)),
                    (30, OrbValue::Jackpot(JackpotTier::Mini)),
                    (15, OrbValue::Jackpot(JackpotTier::Minor)),
                    (major, OrbValue::Jackpot(JackpotTier::Major)),
                ]
                .into_iter()
                .filter(|(weight, _)| *weight > 0)
                .map(|(weight, value)| dragonlink_model::WeightedOrbValue { weight, value })
                .collect();
            }
        }
        model
    }
}

/// The model and seed `game` plays: as given, or a prepared run's (see
/// [`Scenario`]; Golden Century only).
fn scenario(game: GameChoice, model: MathModel, seed: u64) -> (MathModel, u64) {
    match Scenario::from_env() {
        Some(run) if game == GameChoice::GoldenCentury => {
            let seed = run.seed(seed);
            (run.model(model, seed), seed)
        }
        Some(_) => {
            eprintln!("DRAGONLINK_SCENARIO is for Golden Century only; ignored");
            (model, seed)
        }
        None => (model, seed),
    }
}

/// A prepared run's dressing (see [`Scenario`]): the first spin's window with
/// three flags, and six coins when `coins`, at the stops the seed draws; the
/// doors on every third stop of the free games' reels.
fn dress_scenario(mut model: MathModel, seed: u64, coins: bool) -> MathModel {
    let id = |key: &str| {
        model
            .symbols
            .iter()
            .position(|s| s.key == key)
            .and_then(|i| u16::try_from(i).ok())
            .map(dragonlink_model::SymbolId)
    };
    let (Some(orb), Some(flag), Some(doors)) = (id("orb"), id("pennant"), id("doors")) else {
        return model;
    };
    let nine = id("9");
    // The first spin's window, reel by reel, top row first: three flags and
    // six coins (or none: the strip's own symbols stay, never a coin); the
    // other cells keep the strip's own symbols.
    let coin = coins.then_some(orb);
    let window: [[Option<dragonlink_model::SymbolId>; 3]; 5] = [
        [Some(flag), coin, coin],
        [coin, None, coin],
        [Some(flag), coin, None],
        [coin, None, None],
        [Some(flag), None, None],
    ];
    // The stops the seeded source will draw for the first spin: one draw
    // below each base strip's length, reel by reel (dragonlink-core's spin).
    let mut rng = DevRng::new(seed);
    for (reel, cells) in window.iter().enumerate() {
        let Some(strip) = model.reel_strips.get_mut(reel) else {
            break;
        };
        let len = strip.len();
        let Ok(stop) = u32::try_from(len).map(|n| rng.below(n)) else {
            break;
        };
        let Ok(stop) = stop else { break };
        // A reel given a flag keeps only that one (a nine in place of its
        // own), so no window shows two flags on one reel.
        if cells.contains(&Some(flag))
            && let Some(nine) = nine
        {
            for s in strip.iter_mut() {
                if *s == flag {
                    *s = nine;
                }
            }
        }
        for (row, cell) in cells.iter().enumerate() {
            if let Some(symbol) = cell {
                strip[(stop as usize + row) % len] = *symbol;
            }
        }
    }
    // The doors on every third stop of the free games' reels.
    if let Some(fg) = model.free_games.as_mut() {
        for strip in &mut fg.free_reel_strips {
            for (i, s) in strip.iter_mut().enumerate() {
                if i % 3 == 1 {
                    *s = doors;
                }
            }
        }
    }
    model
}

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
    // Testing (`DRAGONLINK_GRAND_TEST`): coins land on most respins, so the
    // grid usually fills and the Grand is won.
    if std::env::var_os("DRAGONLINK_GRAND_TEST").is_some()
        && let Some(hs) = model.hold_and_spin.as_mut()
    {
        hs.orb_land_chance = dragonlink_model::Ratio { num: 2, den: 3 };
    }
    for strip in &mut model.reel_strips {
        let mut padded = Vec::with_capacity(strip.len() * 2);
        for (i, &s) in strip.iter().enumerate() {
            padded.push(s);
            if orb_every > 0 && (i + 1) % orb_every == 0 {
                padded.push(orb);
            }
            if scatter_every > 0 && (i + 1) % scatter_every == 0 {
                padded.push(scatter);
            }
        }
        *strip = spaced(padded, scatter);
    }
    model
}

/// The strip with any scatter dropped that would show with another in one
/// reel window (three rows): the game never shows two flags on a reel.
fn spaced(
    strip: Vec<dragonlink_model::SymbolId>,
    scatter: dragonlink_model::SymbolId,
) -> Vec<dragonlink_model::SymbolId> {
    let mut out: Vec<dragonlink_model::SymbolId> = Vec::with_capacity(strip.len());
    for s in strip {
        let near = out.iter().rev().take(2).any(|&p| p == scatter);
        if s == scatter && near {
            continue;
        }
        out.push(s);
    }
    // Across the wrap: the last two stops and the first two show together.
    while out.len() > 4 {
        let n = out.len();
        let tail = [out[n - 2], out[n - 1]];
        let head = [out[0], out[1]];
        let clash = |a: &[dragonlink_model::SymbolId], b: &[dragonlink_model::SymbolId]| {
            (0..a.len()).any(|i| {
                (0..b.len()).any(|j| a[i] == scatter && b[j] == scatter && (a.len() - i) + j <= 2)
            })
        };
        if !clash(&tail, &head) {
            break;
        }
        // Drop the latest scatter at the end of the strip.
        match out.iter().rposition(|&p| p == scatter) {
            Some(i) if i >= n - 2 => {
                out.remove(i);
            }
            _ => break,
        }
    }
    out
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
                let flags = std::env::var("DRAGONLINK_FEATURE_TEST").is_ok_and(|v| v == "flags");
                feature_test(
                    golden_century_synthetic(),
                    if flags { 0 } else { FEATURE_TEST_ORB },
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

    /// The title's name in file names and on the command line.
    fn slug(self) -> &'static str {
        match self {
            GameChoice::GoldenCentury => "golden-century",
            GameChoice::AutumnMoon => "autumn-moon",
            GameChoice::HappyProsperous => "happy-prosperous",
            GameChoice::PandaMagic => "panda-magic",
            GameChoice::Synthetic => "synthetic",
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

/// The local authentic pack for `game`, never in the repository:
/// `DRAGONLINK_ART` names Golden Century's pack itself (as it always has) or
/// a folder of packs, one per title (`<folder>/<slug>`).
fn pack_dir(game: GameChoice) -> Option<String> {
    let dir = std::env::var("DRAGONLINK_ART")
        .ok()
        .filter(|d| !d.is_empty())?;
    let root = std::path::Path::new(&dir);
    if root.join("symbols").is_dir() {
        return (game == GameChoice::GoldenCentury).then_some(dir);
    }
    let own = root.join(game.slug());
    own.join("symbols")
        .is_dir()
        .then(|| own.display().to_string())
}

/// "GOLDEN CENTURY" as "Golden Century", for the window's title.
fn title_case(name: &str) -> String {
    name.split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_string() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Switch machines when the chooser picks another title: the app starts
/// again as that title, with the same money (`--credits` at 1c), and this
/// one closes. Only between games; a feature in progress keeps the machine.
fn machine_switch(
    smoke: Res<Smoke>,
    sim: Res<Sim>,
    denom: Res<Denom>,
    presentation: Res<Presentation>,
    mut menu: ResMut<cabinet::MachineMenu>,
    mut exit: MessageWriter<AppExit>,
) {
    // Testing (`DRAGONLINK_SHOW=machines`): the chooser opens two seconds
    // into an unattended run, so it can be captured.
    if smoke.frames_left.is_some()
        && smoke.frame == 120
        && std::env::var("DRAGONLINK_SHOW").as_deref() == Ok("machines")
    {
        menu.open = true;
    }
    let Some(slug) = menu.chosen.take() else {
        return;
    };
    menu.open = false;
    if slug == menu.current || !sim.game.is_idle() || !presentation.queue.is_empty() {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let money = sim
            .game
            .credits()
            .saturating_mul(denom.cents())
            .saturating_add(sim.game.state().residue);
        if let Ok(exe) = std::env::current_exe() {
            let spawned = std::process::Command::new(exe)
                .args(["--game", slug.as_str(), "--credits", &money.to_string()])
                .spawn();
            if spawned.is_ok() {
                exit.write(AppExit::Success);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (&denom, &mut exit);
}

/// The pack the running title uses, if any.
#[derive(Resource, Default, Clone)]
struct PackDir(Option<String>);

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

/// The denomination: how much money one credit stands for. The simulation
/// works in credits; a change recounts them so the money is unchanged
/// (`Command::ChangeDenomination`). These are simulation dollars and stand for no real
/// money.
#[derive(Resource, Default)]
struct Denom {
    index: usize,
}

/// Cents per credit for each denomination, cheapest first.
const DENOM_CENTS: [u64; 6] = cabinet::MENU_DENOMS;

impl Denom {
    fn cents(&self) -> u64 {
        DENOM_CENTS.get(self.index).copied().unwrap_or(1)
    }

    fn label(&self) -> String {
        match self.cents() {
            c if c >= 100 => format!("${}", c / 100),
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

/// The progressive jackpots' meters as the screen shows them. On Australian
/// machines the Major and the Grand are running meters that increase during
/// play (STRONG, claims/family.toml, jackpots.meters-au); the simulation is
/// told each new value (`Command::SetProgressive`), so a win pays what the
/// meter shows. How they climb here (small irregular steps at irregular
/// moments, an occasional jump), where they open and where they stop are
/// this project's synthetic choices ([`LINK_SPECS`]): no source shows an
/// increment rate or a ceiling. They draw on their own generator, never the
/// outcome source.
#[derive(Resource, Default)]
struct Link {
    rng: u64,
    meters: Vec<LinkMeter>,
}

/// One progressive meter of the [`Link`].
struct LinkMeter {
    spec: &'static LinkSpec,
    /// What the meter shows, in cents.
    shown: u64,
    /// The value last given to the simulation.
    told: u64,
    /// Seconds until it next climbs.
    wait: f32,
    /// Won, and the win not yet shown: the meter holds the amount won.
    held: bool,
}

/// How a progressive meter starts and climbs (synthetic, the owner's
/// ranges: the Grand from $5,000 to $15,000, the Major from $500 to $1,500,
/// opening anywhere up to 80% of the top).
struct LinkSpec {
    tier: JackpotTier,
    /// The meter stops climbing here, in cents.
    ceiling: u64,
    /// It opens at most here, in cents (and at least at its restart value).
    opens_below: u64,
    /// Seconds between climbs, at least and at most.
    every: (f32, f32),
    /// Cents a climb adds, at least and at most.
    climb: (u64, u64),
    /// One climb in [`LINK_JUMP_ONE_IN`] adds this many instead.
    jump: (u64, u64),
}

const LINK_SPECS: [LinkSpec; 2] = [
    LinkSpec {
        tier: JackpotTier::Grand,
        ceiling: 1_500_000,
        opens_below: 1_200_000,
        every: (0.15, 1.6),
        climb: (1, 12),
        jump: (25, 90),
    },
    LinkSpec {
        tier: JackpotTier::Major,
        ceiling: 150_000,
        opens_below: 120_000,
        every: (0.4, 3.0),
        climb: (1, 4),
        jump: (5, 20),
    },
];
const LINK_JUMP_ONE_IN: u64 = 15;

impl Link {
    /// The next number of the meters' own generator (splitmix64).
    fn next(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A whole number from `lo` to `hi`, both included.
    fn between(&mut self, (lo, hi): (u64, u64)) -> u64 {
        let span = hi.saturating_sub(lo).saturating_add(1);
        lo.saturating_add(self.next() % span)
    }

    /// Seconds from `lo` to `hi`.
    fn seconds(&mut self, (lo, hi): (f32, f32)) -> f32 {
        let unit = (self.next() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }

    /// The meter the screen shows for a progressive jackpot, in cents.
    fn shown(&self, tier: JackpotTier) -> Option<u64> {
        self.meters
            .iter()
            .find(|m| m.spec.tier == tier)
            .map(|m| m.shown)
    }
}

/// Tell the simulation a progressive meter's value. The command draws
/// nothing, so it is given an outcome source that has nothing to give.
fn set_progressive(game: &mut Game, tier: JackpotTier, value: u64) -> bool {
    let mut none = dragonlink_core::ScriptedRng::new(Vec::new());
    game.apply(Command::SetProgressive { tier, value }, &mut none)
        .is_ok()
}

/// Open the link for a new game: each progressive the model has starts
/// somewhere between its restart value and 80% of its ceiling.
fn open_link(game: &mut Game, seed: u64) -> Link {
    let mut link = Link {
        rng: seed ^ 0x6C69_6E6B_6D65_7465,
        meters: Vec::new(),
    };
    for spec in &LINK_SPECS {
        let reset = game
            .model()
            .hold_and_spin
            .as_ref()
            .and_then(|hs| hs.jackpots.iter().find(|j| j.tier == spec.tier))
            .and_then(|j| j.progressive_reset);
        let Some(reset) = reset else { continue };
        let value = link.between((reset, spec.opens_below.max(reset)));
        let wait = link.seconds(spec.every);
        if set_progressive(game, spec.tier, value) {
            link.meters.push(LinkMeter {
                spec,
                shown: value,
                told: value,
                wait,
                held: false,
            });
        }
    }
    link
}

/// The progressive meters climb, through features too. A meter the
/// simulation has paid out holds the amount won until the win is shown.
fn link_creep(
    time: Res<Time>,
    smoke: Res<Smoke>,
    presentation: Res<Presentation>,
    show: Res<cabinet::FeatureShow>,
    mut link: ResMut<Link>,
    mut sim: ResMut<Sim>,
) {
    // Everything shown, the feature's count included (in the placeholder
    // presentation, which has no count, once the events are shown).
    let shown_all = sim.game.is_idle()
        && presentation.queue.is_empty()
        && !show.in_feature()
        && !show.counting();
    link.tick(&mut sim.game, smoke.dt(&time), &|tier| {
        shown_all || show.link_won.contains(&tier)
    });
}

impl Link {
    /// `dt` seconds of the link: each meter climbs when its time comes, and
    /// one the simulation has paid out shows the amount won until `win_shown`
    /// says its win has been shown.
    fn tick(&mut self, game: &mut Game, dt: f32, win_shown: &dyn Fn(JackpotTier) -> bool) {
        for i in 0..self.meters.len() {
            let spec = self.meters[i].spec;
            let tier = spec.tier;
            let now = game.meter(tier).unwrap_or(self.meters[i].told);
            let mut wait = self.meters[i].wait - dt;
            let mut told = self.meters[i].told;
            let mut held = self.meters[i].held;
            if now != told {
                // Won: the simulation paid it and restarted it.
                told = now;
                held = true;
            }
            if held && win_shown(tier) {
                held = false;
            }
            if wait <= 0.0 {
                wait = self.seconds(spec.every);
                let add = if self.next().is_multiple_of(LINK_JUMP_ONE_IN) {
                    self.between(spec.jump)
                } else {
                    self.between(spec.climb)
                };
                let next = told.saturating_add(add).min(spec.ceiling.max(told));
                if next != told && set_progressive(game, tier, next) {
                    told = next;
                }
            }
            let meter = &mut self.meters[i];
            meter.wait = wait;
            meter.told = told;
            meter.held = held;
            if !held {
                meter.shown = told;
            }
        }
    }
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
    /// Testing (`DRAGONLINK_CAPTURE=<folder>`): every `n`th frame of an
    /// unattended run saved there (`DRAGONLINK_CAPTURE_EVERY`, default 6)
    /// from frame `DRAGONLINK_CAPTURE_FROM` (default 0), from the renderer,
    /// so other windows cannot spoil it.
    capture: Option<(String, u32, u32)>,
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
    /// Which of the title's symbols plays which part (wild, scatter, coin,
    /// mystery), from its model.
    roles: cabinet::Roles,
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

/// Events presented while the reels are moving: the spin's start, its bet,
/// and the reels' stops (with their coin values). Everything else waits for
/// the reels to come to rest.
fn during_reels(event: &Event) -> bool {
    matches!(
        event,
        Event::BetPlaced { .. }
            | Event::BetChanged { .. }
            | Event::SpinStarted { .. }
            | Event::ReelStopped { .. }
            | Event::OrbValueShown { .. }
            | Event::CreditsChanged {
                reason: CreditReason::Bet,
                ..
            }
    )
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

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    sim: Res<Sim>,
    asset_server: Res<AssetServer>,
    mut art: ResMut<ArtPack>,
    fidelity: Res<cabinet::Fidelity>,
    mut fonts: ResMut<Assets<Font>>,
    mut sounds: ResMut<cabinet::Sounds>,
    profile: Res<cabinet::Profile>,
    pack: Res<PackDir>,
    machines: Res<cabinet::MachineMenu>,
) {
    // The initial state's OnEnter runs before Startup systems, so the pack is
    // read here, on first use.
    load_art(&asset_server, &mut art, pack.0.as_deref());
    let model = sim.game.model();
    art.roles = cabinet::Roles::from_model(model);
    if fidelity.0 {
        let cab_fonts = cabinet::load_fonts(&mut fonts);
        if let Some(dir) = &pack.0 {
            *sounds = cabinet::Sounds::load(&asset_server, dir);
        }
        cabinet::spawn(&mut commands, model, &art, &cab_fonts, *profile);
        cabinet::spawn_machine_menu(&mut commands, &cab_fonts, &machines);
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
    reels: Res<cabinet::Reels>,
    fidelity: Res<cabinet::Fidelity>,
    mut menu: ResMut<cabinet::GamesMenu>,
    sounds: Res<cabinet::Sounds>,
    mut commands: Commands,
    profile: Res<cabinet::Profile>,
) {
    let profile = *profile;
    // Idle means the reels have come to rest on screen too: the last one runs
    // on and settles after the game has stopped it, and a spin started then
    // would throw it straight back into motion mid-settle.
    let at_rest = sim.game.is_idle()
        && presentation.queue.is_empty()
        && !gamble.active
        && reels.rested_for() >= cabinet::SPIN_AFTER_REST;
    // The cabinet's GAMES MENU opens on a touch at rest.
    // Testing (`DRAGONLINK_SHOW=menu`, an unattended run only): the GAMES
    // MENU opens by itself two seconds in, so it can be captured.
    let show_menu = smoke.frames_left.is_some()
        && smoke.frame == 120
        && std::env::var("DRAGONLINK_SHOW").as_deref() == Ok("menu");
    if fidelity.0 && (cabinet_input.menu || show_menu) && at_rest && !menu.showing() {
        menu.open_now();
    }
    // A denomination: touched on the GAMES MENU, or the next one with D.
    let offered = if fidelity.0 {
        profile.denoms()
    } else {
        &DENOM_CENTS[..]
    };
    let chosen = if menu.open {
        cabinet_input.denom
    } else {
        (keys.just_pressed(KeyCode::KeyD) && at_rest && !menu.showing()).then(|| {
            let here = offered.iter().position(|c| *c == denom.cents());
            offered[here.map_or(0, |i| (i + 1) % offered.len())]
        })
    };
    if let Some(to) = chosen
        && let Some(index) = DENOM_CENTS.iter().position(|c| *c == to)
    {
        if menu.open {
            menu.close();
        }
        let from = denom.cents();
        if from != to && sim.game.is_idle() {
            // The same money in the new credit, so the simulation's credit
            // size is always the one shown (fixed prizes and meters depend
            // on it). The cabinet then plays one credit per line (the
            // multiplier at 1) and the denomination's starting lines
            // (footage).
            let last_bet =
                (sim.game.state().spins > 0).then(|| sim.game.total_bet().saturating_mul(from));
            let Sim { game, rng } = &mut *sim;
            let changed = u32::try_from(from)
                .ok()
                .zip(u32::try_from(to).ok())
                .map(|(from, to)| game.apply(Command::ChangeDenomination { from, to }, rng));
            if let Some(Ok(_)) = changed {
                if fidelity.0 {
                    let _ = game.apply(
                        Command::SetStake {
                            index: 0,
                            lines: profile.lines_on_change(to),
                        },
                        rng,
                    );
                    menu.changed = Some((from, to));
                    menu.last_bet_cents = last_bet;
                }
                presentation.shown_credits = game.credits();
                denom.index = index;
            }
        }
    }
    // The lines a denomination allows: fixed in the casino style, the
    // denomination's ladder on the club machines.
    let lines_fix = (fidelity.0 && at_rest)
        .then(|| match profile {
            cabinet::Profile::Casino => cabinet::lines_for(denom.cents()),
            cabinet::Profile::Club => {
                let ladder = cabinet::line_keys(denom.cents());
                if ladder.contains(&sim.game.lines()) {
                    sim.game.lines()
                } else {
                    ladder[0]
                }
            }
        })
        .filter(|n| *n != sim.game.lines() && sim.game.model().line_counts().contains(n));
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
    let auto =
        auto && !gamble.holds_autospin(cabinet::gamble_open(&sim.game, denom.cents(), &feature));
    let options = sim.game.model().bet.credits_per_line_options.clone();
    let bet_index_of = |credits: u32| options.iter().position(|o| *o == credits);
    // Casino: a bet key sets its multiplier and plays in one press. Club: a
    // lines key sets the lines and plays (footage).
    let bet_press = cabinet_input
        .bet
        .filter(|_| fidelity.0)
        .and_then(|m| bet_index_of(m).map(|index| (m, index)));
    let lines_press = cabinet_input
        .lines
        .filter(|n| fidelity.0 && sim.game.model().line_counts().contains(n));
    let go = (auto
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || cabinet_input.spin
        || bet_press.is_some()
        || lines_press.is_some())
        && !cabinet_input.skip_spin
        && sim.game.is_idle()
        // A feature still on screen (its count, its banner) finishes first.
        && presentation.queue.is_empty()
        && reels.rested_for() >= cabinet::SPIN_AFTER_REST
        && feature.awaiting.is_none()
        && !feature.blocks()
        && !gamble.active
        && !menu.showing();
    if go {
        // A new game: the gamble's result and its end message go, BET shows
        // its own bet.
        gamble.result = None;
        gamble.ended = None;
        let first_here = menu.last_bet_cents.take().is_some();
        let Sim { game, rng } = &mut *sim;
        if let Some((m, index)) = bet_press {
            // The beep marks a new bet (repeat games carry none).
            if index != game.state().bet_index || first_here {
                let (clip, volume) = cabinet::bet_beep(m);
                sounds.play_at(&mut commands, clip, volume);
            }
            if index != game.state().bet_index
                && let Ok(events) = game.apply(Command::SetBet { index }, rng)
            {
                presentation.queue.extend(schedule_with(events, &timing));
            }
        }
        if let Some(lines) = lines_press
            && lines != game.lines()
            && let Ok(events) = game.apply(Command::SetLines { lines }, rng)
        {
            presentation.queue.extend(schedule_with(events, &timing));
        }
    }
    // Club: a credits-per-line key only selects.
    let credits_press = cabinet_input
        .credits
        .filter(|_| fidelity.0 && at_rest && !menu.showing())
        .and_then(bet_index_of)
        .filter(|i| *i != sim.game.state().bet_index);
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
    // BET does not follow a selection: it keeps the last game's bet until the
    // next game (footage).
    if credits_press.is_some() && sim.game.state().spins > 0 && menu.last_bet_cents.is_none() {
        menu.last_bet_cents = Some(sim.game.total_bet().saturating_mul(denom.cents()));
    }
    let command = if go {
        Some(Command::Spin)
    } else if let Some(index) = credits_press {
        Some(Command::SetBet { index })
    } else if let Some(lines) = lines_fix {
        Some(Command::SetLines { lines })
    } else if keys.just_pressed(KeyCode::ArrowUp) && bet_index + 1 < bets {
        Some(Command::SetBet {
            index: bet_index + 1,
        })
    } else if keys.just_pressed(KeyCode::ArrowDown) && bet_index > 0 {
        Some(Command::SetBet {
            index: bet_index - 1,
        })
    } else if fidelity.0 {
        // The cabinet's lines follow the denomination.
        None
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
        presentation.step_timer = feature.step_pause();
        return;
    }
    if !presentation.queue.is_empty() {
        presentation.step_timer = feature.step_pause();
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
    presentation.step_timer = feature.step_pause();
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
    reels_motion: Res<cabinet::Reels>,
    rollup: Res<cabinet::Rollup>,
    stacks: Res<cabinet::WildStacks>,
    mut cells: Query<(&Cell, &mut BackgroundColor)>,
    mut labels: Query<(&CellLabel, &mut Text)>,
) {
    let reels = sim.game.model().grid.reels as usize;
    if reels_spin.spinning.len() != reels {
        reels_spin.spinning = vec![false; reels];
    }
    // The title's wild and mystery symbols, from its model.
    let roles = cabinet::Roles::from_model(sim.game.model());
    let wild = roles.wild.as_str();
    let mystery = roles.mystery.as_deref();
    presentation.wait -= smoke.dt(&time);
    while presentation.wait <= 0.0 {
        // Flags that won the free games play their sequence (the bell, the
        // flags waving) before their win is shown; the feature then waits
        // for that win's count (real game).
        if fidelity.0
            && reels_motion.rested_for() >= cabinet::RESULT_AFTER_REST
            && matches!(
                presentation.queue.front(),
                Some((_, Event::ScatterWin { .. }))
            )
            && presentation
                .queue
                .iter()
                .any(|(_, e)| matches!(e, Event::FreeGamesStarted { .. }))
        {
            feature.begin_free_games_trigger();
        }
        if fidelity.0
            && rollup.active()
            && matches!(
                presentation.queue.front(),
                Some((
                    _,
                    Event::FeatureTriggered { .. }
                        | Event::FreeGamesStarted { .. }
                        | Event::FreeGamesResumed { .. }
                ))
            )
        {
            presentation.wait = 0.0;
            break;
        }
        // A stack of emperors on a winning line stands, with its sound, and
        // the win waits for it (real game).
        if fidelity.0
            && !feature.wild_sounded()
            && reels_motion.rested_for() >= cabinet::RESULT_AFTER_REST
            && matches!(presentation.queue.front(), Some((_, Event::LineWin { .. })))
        {
            // The stacks are found once the reels (and any doors) are still.
            if !stacks.found() {
                presentation.wait = 0.0;
                break;
            }
            let wins: Vec<(u16, u8)> = presentation
                .queue
                .iter()
                .filter_map(|(_, e)| match e {
                    Event::LineWin { line, count, .. } => Some((*line, *count)),
                    _ => None,
                })
                .collect();
            let paylines = &sim.game.model().paylines;
            let stack = stacks.on_lines(paylines, wins.iter().copied()).is_some();
            // A single emperor on a winning line sounds too.
            let emperor = wins.iter().any(|&(line, count)| {
                paylines.get(line as usize).is_some_and(|rows| {
                    (0..count).any(|reel| {
                        rows.0.get(reel as usize).is_some_and(|&row| {
                            labels
                                .iter()
                                .any(|(l, t)| l.reel == reel && l.row == row && t.0 == wild)
                        })
                    })
                })
            });
            if stack || emperor {
                feature.begin_wild(stack);
            }
        }
        // A respin spinning or the coins being counted hold what follows.
        if fidelity.0 && feature.blocks() {
            presentation.wait = 0.0;
            break;
        }
        // What a spin brings (its wins, its feature, the next game) waits
        // until the reels are at rest on screen: the last reel's stop is
        // presented as it starts to run on to its stop, and it overshoots
        // and settles after that.
        // The doors open sooner than a result shows (real game).
        let after_rest = match presentation.queue.front() {
            Some((_, Event::MysteryRevealed { .. })) => cabinet::REVEAL_AFTER_REST,
            _ => cabinet::RESULT_AFTER_REST,
        };
        if fidelity.0
            && reels_motion.rested_for() < after_rest
            && presentation
                .queue
                .front()
                .is_some_and(|(_, e)| !during_reels(e))
        {
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
                // Only coins that landed as coins: a free game's doors show
                // theirs as they open (the engine reports both after the
                // reveal).
                let landed = *reel;
                let is_coin = |row: u8| {
                    symbols.get(row as usize).is_some_and(|key| {
                        model
                            .symbols
                            .iter()
                            .any(|s| &s.key == key && s.kind == SymbolKind::Orb)
                    })
                };
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
                    if is_coin(row) {
                        set_cell(landed, row, orb_text(value), orb_color);
                    }
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
                // Free games after Hold & Spin: its dimmed positions light
                // again.
                for (_, mut text) in &mut labels {
                    if let Some(rest) = text.0.strip_prefix("DIM:") {
                        text.0 = rest.to_string();
                    }
                }
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
                // The coins the doors open onto, each with its prize (the
                // engine reports them straight after the reveal).
                let faces: Vec<(u8, u8, String)> = presentation
                    .queue
                    .iter()
                    .map_while(|(_, e)| match e {
                        Event::OrbValueShown { reel, row, value } => {
                            Some((*reel, *row, orb_text(*value)))
                        }
                        _ => None,
                    })
                    .collect();
                if fidelity.0 {
                    feature.reveal_faces = faces;
                } else {
                    // The placeholder opens the doors at once, in the
                    // revealed symbol's colour (a coin's for coins).
                    let kind = model
                        .symbols
                        .iter()
                        .find(|s| &s.key == symbol)
                        .map(|s| s.kind);
                    let mut opened = Vec::new();
                    for (label, mut text) in &mut labels {
                        if Some(text.0.as_str()) == mystery {
                            let face = faces
                                .iter()
                                .find(|(reel, row, _)| (*reel, *row) == (label.reel, label.row))
                                .map(|(_, _, face)| face.clone());
                            let color = if face.is_some() {
                                orb_color
                            } else {
                                cell_color(kind, symbol)
                            };
                            text.0 = face.unwrap_or_else(|| symbol.clone());
                            opened.push((label.reel, label.row, color));
                        }
                    }
                    for (cell, mut background) in &mut cells {
                        if let Some((_, _, color)) = opened
                            .iter()
                            .find(|(reel, row, _)| (*reel, *row) == (cell.reel, cell.row))
                        {
                            background.0 = *color;
                        }
                    }
                }
                presentation.status = format!("Mystery reveals {symbol} x{count}");
            }
            Event::FreeGamesResumed { remaining } => {
                // The free games carry on after the Hold & Spin one of them
                // started: its dimmed positions light again.
                for (_, mut text) in &mut labels {
                    if let Some(rest) = text.0.strip_prefix("DIM:") {
                        text.0 = rest.to_string();
                    }
                }
                presentation.status = format!("FREE GAMES resume  ({remaining} to come)");
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
                // After a Hold & Spin on the last free game: its dimmed
                // positions light again.
                for (_, mut text) in &mut labels {
                    if let Some(rest) = text.0.strip_prefix("DIM:") {
                        text.0 = rest.to_string();
                    }
                }
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
    link: Res<Link>,
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
    // Jackpot meters: a progressive's meter in money, any other its prize
    // now in credits.
    for (meter, mut text) in &mut meters {
        text.0 = match link.shown(meter.tier) {
            Some(meter_cents) => money(meter_cents, 1),
            None => game
                .jackpot_value(meter.tier)
                .map_or_else(|| "-".to_string(), |c| money(c, cents)),
        };
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
    // A capture of the run, frame by frame (testing only; the pictures go
    // to a folder outside the repository, see docs/LEGAL.md).
    #[cfg(not(target_arch = "wasm32"))]
    if let Some((dir, every, from)) = &smoke.capture
        && smoke.frame >= *from
        && smoke.frame.is_multiple_of(*every)
    {
        let path = format!("{dir}/f{:05}.png", smoke.frame);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
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
fn load_art(asset_server: &AssetServer, pack: &mut ArtPack, dir: Option<&str>) {
    if pack.loaded {
        return;
    }
    pack.loaded = true;
    if let Some(dir) = dir {
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
                    // A dimmed symbol on an open Hold & Spin position keeps
                    // its size but is cut off at its position's edges, as
                    // if behind the grid (large ones otherwise overhang).
                    let src = art.sizes.get(k).copied().unwrap_or((w, h));
                    let crop = if dim && (w > cw || h > ch) {
                        let (vw, vh) = (w.min(cw), h.min(ch));
                        let (sx, sy) = (src.0 * vw / w, src.1 * vh / h);
                        let (x0, y0) = ((src.0 - sx) / 2.0, (src.1 - sy) / 2.0);
                        Some((vw, vh, Rect::new(x0, y0, x0 + sx, y0 + sy)))
                    } else {
                        None
                    };
                    let rect = crop.map(|c| c.2);
                    if image.rect != rect {
                        image.rect = rect;
                    }
                    let (w, h) = crop.map_or((w, h), |c| (c.0, c.1));
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
    let (model, seed) = scenario(game, game.model(), launch.seed);
    match Game::new(model, launch.credits) {
        Ok(mut g) => {
            commands.insert_resource(open_link(&mut g, seed));
            commands.insert_resource(Sim {
                game: g,
                rng: DevRng::new(seed),
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
    // The fidelity build: an authentic pack is configured for the title. It
    // boots straight into the game, and refuses to start on an incomplete
    // pack rather than fall back to placeholder tiles.
    let art_dir = pack_dir(cli.game);
    if art_dir.is_none()
        && let Ok(dir) = std::env::var("DRAGONLINK_ART")
        && !dir.is_empty()
    {
        eprintln!("no authentic pack for {} under {dir}", cli.game.title());
        return AppExit::error();
    }
    let fidelity = art_dir.is_some();
    if let Some(dir) = &art_dir
        && let Err(missing) = cabinet::check_pack(dir, &cli.game.model())
    {
        eprintln!("authentic pack at {dir} is incomplete; missing:");
        for m in missing {
            eprintln!("  {m}");
        }
        return AppExit::error();
    }
    let explicit_game = args.iter().any(|a| a == "--game");
    let direct_boot = fidelity || explicit_game || cli.smoke_frames.is_some();
    let direct_sim = if direct_boot {
        let (model, seed) = scenario(cli.game, cli.game.model(), cli.seed);
        match Game::new(model, cli.credits) {
            Ok(mut game) => Some((
                open_link(&mut game, seed),
                Sim {
                    game,
                    rng: DevRng::new(seed),
                },
            )),
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
            cabinet::window(&title_case(cli.game.title()))
        } else {
            primary_window()
        }),
        ..default()
    });
    // When an authentic asset pack is configured, point the asset server at it
    // so `symbols/<key>.png` resolves there. The pack is never in the repo.
    let plugins = match art_dir.clone() {
        Some(dir) => plugins.set(bevy::asset::AssetPlugin {
            file_path: dir,
            ..default()
        }),
        None => plugins,
    };
    app.add_plugins(plugins);
    app.insert_resource(PackDir(art_dir.clone()));
    // The machine chooser lists every title with a pack.
    app.insert_resource(cabinet::MachineMenu {
        entries: MENU_GAMES
            .iter()
            .filter(|g| pack_dir(**g).is_some())
            .map(|g| (g.slug().to_string(), title_case(g.title())))
            .collect(),
        current: cli.game.slug().to_string(),
        ..default()
    });
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Startup, announce_placeholder_mode);
    // Testing (`DRAGONLINK_MUTE`): every sound plays at zero volume, so runs
    // can be watched and recorded without being heard.
    if std::env::var_os("DRAGONLINK_MUTE").is_some() {
        app.insert_resource(bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(
            0.0,
        )));
    }
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
    .insert_resource(cabinet::GambleShow::with_history(cli.seed))
    .insert_resource(cabinet::GamesMenu::default())
    .insert_resource(cabinet::AttractIdle::default())
    .insert_resource(cabinet::DeckState::default())
    .insert_resource(cabinet::Profile::from_env())
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
        capture: std::env::var("DRAGONLINK_CAPTURE")
            .ok()
            .filter(|d| !d.is_empty())
            .map(|dir| {
                let every = std::env::var("DRAGONLINK_CAPTURE_EVERY")
                    .ok()
                    .and_then(|n| n.parse().ok())
                    .filter(|n: &u32| *n > 0)
                    .unwrap_or(6);
                let from = std::env::var("DRAGONLINK_CAPTURE_FROM")
                    .ok()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                (dir, every, from)
            }),
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
                cabinet::free_games_trigger,
                cabinet::hs_medallion,
                cabinet::payout_effects,
                cabinet::gamble_control,
                cabinet::attract_timer,
                input,
                cabinet::clear_buttons,
                drive_features,
                present,
                link_creep,
                machine_switch,
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
                cabinet::feature_cues,
                cabinet::coin_shower,
            )
                .chain(),
            (
                cabinet::win_rollup,
                render_symbols,
                cabinet::orb_overlay,
                cabinet::symbol_fx,
                cabinet::games_menu,
                cabinet::light_deck,
                cabinet::feature_screen,
                cabinet::gamble_screen,
                cabinet::start_prompts,
                cabinet::clear_cells,
                cabinet::update_meters,
                cabinet::fit_jackpots,
                cabinet::attract_bar,
                cabinet::additive_fire,
                cabinet::machine_menu,
                cabinet::fit,
            )
                .chain(),
        )
            .chain()
            .run_if(in_state(Screen::Playing)),
    );

    match direct_sim {
        Some((link, sim)) => {
            app.insert_resource(link);
            app.insert_resource(sim);
            app.insert_state(Screen::Playing);
        }
        None => {
            app.insert_resource(Link::default());
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
    fn the_both_features_scenario_opens_with_hold_and_spin_then_free_games() {
        let seed = 1;
        let model = Scenario::Both.model(golden_century_synthetic(), seed);
        let mut game = Game::new(model, 100_000).unwrap();
        let mut rng = DevRng::new(seed);
        let events = game.play_spin(&mut rng).unwrap();
        let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
        let hs = at(&|e| {
            matches!(
                e,
                Event::FeatureTriggered {
                    feature: dragonlink_core::Feature::HoldAndSpin,
                    ..
                }
            )
        });
        let fg = at(&|e| matches!(e, Event::FreeGamesStarted { .. }));
        let grand = at(&|e| {
            matches!(
                e,
                Event::JackpotAwarded {
                    tier: JackpotTier::Grand,
                    ..
                }
            )
        });
        assert!(hs.is_some(), "Hold & Spin starts");
        assert!(fg.is_some(), "the free games start");
        assert!(hs < fg, "Hold & Spin first, then the free games");
        assert!(grand.is_some(), "the grid fills for the Grand");
        // The whole spin's win, Hold & Spin and free games together, can be
        // gambled afterwards.
        assert!(game.can_gamble());
    }

    /// What a prepared run's first spin does at `seed`: the jackpots it
    /// wins, whether the free games follow Hold & Spin, and whether its win
    /// can then be gambled (by the simulation; the app also refuses after a
    /// progressive win).
    fn first_spin(run: Scenario, seed: u64) -> (Vec<JackpotTier>, bool, bool) {
        let (events, gamble) = first_spin_events(run, seed);
        let mut won: Vec<JackpotTier> = events
            .iter()
            .filter_map(|e| match e {
                Event::JackpotAwarded { tier, .. } => Some(*tier),
                _ => None,
            })
            .collect();
        won.sort();
        let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
        let hs = at(&|e| {
            matches!(
                e,
                Event::FeatureTriggered {
                    feature: dragonlink_core::Feature::HoldAndSpin,
                    ..
                }
            )
        });
        let fg = at(&|e| matches!(e, Event::FreeGamesStarted { .. }));
        let both = hs.is_some() && fg.is_some() && hs < fg;
        (won, both, gamble)
    }

    /// A prepared run's first spin at `seed`, and whether its win can then
    /// be gambled (by the simulation).
    fn first_spin_events(run: Scenario, seed: u64) -> (Vec<Event>, bool) {
        let model = run.model(golden_century_synthetic(), seed);
        let mut game = Game::new(model, 100_000).unwrap();
        let mut rng = DevRng::new(seed);
        let events = game.play_spin(&mut rng).unwrap();
        (events, game.can_gamble())
    }

    /// Whether the `doors` run's first spin does what it promises: free games
    /// with no Hold & Spin first; by their third game, doors open onto coins
    /// that start Hold & Spin, which ends short of a full grid; in another
    /// game onto the emperor; no Major or Grand; the free games finish; the
    /// win may be gambled.
    fn doors_as_promised(seed: u64) -> bool {
        let (events, gamble) = first_spin_events(Scenario::Doors, seed);
        let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
        let Some(fg) = at(&|e| matches!(e, Event::FreeGamesStarted { .. })) else {
            return false;
        };
        let Some(hs) = at(&|e| matches!(e, Event::HoldAndSpinStarted { .. })) else {
            return false;
        };
        let game_at_hs = events[..hs]
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::FreeGameStarted { number, .. } => Some(*number),
                _ => None,
            })
            .unwrap_or(0);
        let coin_reveal = events[..hs]
            .iter()
            .any(|e| matches!(e, Event::MysteryRevealed { symbol, .. } if symbol == "orb"));
        let emperor = events
            .iter()
            .any(|e| matches!(e, Event::MysteryRevealed { symbol, .. } if symbol == "emperor"));
        let progressive = events.iter().any(|e| {
            matches!(
                e,
                Event::JackpotAwarded {
                    tier: JackpotTier::Major | JackpotTier::Grand,
                    ..
                }
            )
        });
        let full = events.iter().any(|e| {
            matches!(
                e,
                Event::HoldAndSpinCompleted {
                    full_grid: true,
                    ..
                }
            )
        });
        let finished =
            at(&|e| matches!(e, Event::FreeGamesCompleted { .. })).is_some_and(|end| end > hs);
        fg < hs
            && (1..=3).contains(&game_at_hs)
            && coin_reveal
            && emperor
            && !progressive
            && !full
            && finished
            && gamble
    }

    fn tally(tier: JackpotTier, count: usize) -> Vec<JackpotTier> {
        vec![tier; count]
    }

    /// The jackpots each prepared run promises, smallest tier first.
    fn promised(run: Scenario) -> Vec<JackpotTier> {
        let mut won = tally(JackpotTier::Mini, 4);
        won.extend(tally(JackpotTier::Minor, 2));
        if run == Scenario::Jackpots {
            won.push(JackpotTier::Major);
        }
        won.sort();
        won
    }

    /// Finds seeds for the `jackpots` and `minors` runs (run by hand:
    /// `cargo test -p dragonlink find_jackpot_scenario_seeds -- --ignored
    /// --nocapture`).
    #[test]
    #[ignore = "a search, run by hand"]
    fn find_jackpot_scenario_seeds() {
        for run in [Scenario::Jackpots, Scenario::Minors] {
            let want = promised(run);
            let found: Vec<u64> = (0..200_000u64)
                .filter(|&seed| {
                    let (won, both, gamble) = first_spin(run, seed);
                    both && won == want && gamble
                })
                .take(5)
                .collect();
            println!("{run:?}: {found:?}");
        }
        let doors: Vec<u64> = (0..200_000u64)
            .filter(|&seed| doors_as_promised(seed))
            .take(5)
            .collect();
        println!("Doors: {doors:?}");
    }

    #[test]
    fn the_jackpot_scenarios_win_what_they_promise() {
        for run in [Scenario::Jackpots, Scenario::Minors] {
            let (won, both, gamble) = first_spin(run, run.seed(0));
            assert!(both, "{run:?}: Hold & Spin, then the free games");
            assert_eq!(won, promised(run), "{run:?}: its jackpots");
            // The simulation offers the gamble either way; the app refuses
            // it after the Major (a progressive win).
            assert!(gamble, "{run:?}: the win may be gambled");
        }
        assert!(
            doors_as_promised(Scenario::Doors.seed(0)),
            "doors: coins open Hold & Spin in the free games, and the emperor"
        );
    }

    #[test]
    fn the_link_opens_below_eighty_percent_and_climbs_unevenly_to_its_ceiling() {
        for seed in 0..50 {
            let mut game = Game::new(golden_century_synthetic(), 1000).unwrap();
            let link = open_link(&mut game, seed);
            let grand = link.shown(JackpotTier::Grand).unwrap();
            let major = link.shown(JackpotTier::Major).unwrap();
            assert!((500_000..=1_200_000).contains(&grand), "{grand}");
            assert!((50_000..=120_000).contains(&major), "{major}");
            // The simulation pays what the meters show.
            assert_eq!(game.meter(JackpotTier::Grand), Some(grand));
            assert_eq!(game.meter(JackpotTier::Major), Some(major));
        }
        let mut game = Game::new(golden_century_synthetic(), 1000).unwrap();
        let mut link = open_link(&mut game, 7);
        let mut rises = std::collections::BTreeSet::new();
        let mut last = link.shown(JackpotTier::Grand).unwrap();
        // A minute in tenths of a second: it climbs, by differing amounts.
        for _ in 0..600 {
            link.tick(&mut game, 0.1, &|_| true);
            let now = link.shown(JackpotTier::Grand).unwrap();
            assert!(now >= last);
            if now > last {
                rises.insert(now - last);
            }
            last = now;
        }
        assert!(rises.len() > 3, "uneven rises: {rises:?}");
        assert_eq!(game.meter(JackpotTier::Grand), Some(last));
        // A day later both meters wait at their ceilings.
        for _ in 0..86_400 {
            link.tick(&mut game, 1.0, &|_| true);
        }
        assert_eq!(link.shown(JackpotTier::Grand), Some(1_500_000));
        assert_eq!(link.shown(JackpotTier::Major), Some(150_000));
    }

    #[test]
    fn a_won_meter_holds_the_win_until_it_is_shown() {
        let mut game = Game::new(golden_century_synthetic(), 1000).unwrap();
        let mut link = open_link(&mut game, 3);
        let won = link.shown(JackpotTier::Major).unwrap();
        // The simulation pays the Major and restarts its meter at $500.
        let mut state = game.state().clone();
        state.meters.insert(JackpotTier::Major, 50_000);
        game = Game::resume(game.model().clone(), state).unwrap();
        link.tick(&mut game, 0.0, &|_| false);
        assert_eq!(link.shown(JackpotTier::Major), Some(won));
        link.tick(&mut game, 0.0, &|_| true);
        assert_eq!(link.shown(JackpotTier::Major), Some(50_000));
    }

    /// A Hold & Spin a free game starts keeps that free game on the status
    /// line and hands back to the free games (cabinet footage, y8c8lEdPW8Y).
    #[test]
    fn a_hold_and_spin_inside_the_free_games_returns_to_them() {
        let mut show = cabinet::FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 3,
            remaining: 3,
        });
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        assert!(show.inside_free_games() && show.hold_and_spin());
        assert_eq!(show.caption(0.0).as_deref(), Some("Free Game 3 of 6"));
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        assert_eq!(show.caption(0.0).as_deref(), Some("Free Game 3 of 6"));
        show.on_event(&Event::FreeGamesResumed { remaining: 3 });
        assert!(!show.inside_free_games() && !show.hold_and_spin());
        assert_eq!(show.caption(0.0).as_deref(), Some("Free Game 3 of 6"));
        assert_eq!(show.grand_win(), None);
        // On the last free game, FEATURE COMPLETED follows its tally.
        let mut show = cabinet::FeatureShow::default();
        show.on_event(&Event::FreeGameStarted {
            number: 6,
            remaining: 0,
        });
        show.on_event(&Event::HoldAndSpinStarted {
            respins: 3,
            locked: Vec::new(),
        });
        show.on_event(&Event::HoldAndSpinCompleted {
            total_award: 100,
            full_grid: false,
        });
        // The engine is idle by now; CREDIT holds while its end is shown.
        assert!(show.counting());
        show.on_event(&Event::FreeGamesCompleted {
            games_played: 6,
            total_award: 150,
        });
        assert!(!show.inside_free_games() && !show.hold_and_spin());
    }

    #[test]
    fn the_gamble_sounds_only_where_chosen() {
        use cabinet::{GambleCue, gamble_cue};
        // A win and a loss sound (footage); opening, switching, choosing and
        // taking the win are silent, and nothing loops.
        assert_eq!(gamble_cue(GambleCue::Won), Some(("gamble_win", 1.0)));
        assert_eq!(gamble_cue(GambleCue::Lost), Some(("gamble_loss", 1.0)));
        assert_eq!(gamble_cue(GambleCue::Open), None);
        assert_eq!(gamble_cue(GambleCue::Toggle), None);
        assert_eq!(gamble_cue(GambleCue::Pick), None);
        assert_eq!(gamble_cue(GambleCue::TakeWin), None);
    }

    /// Every clip the gamble and the bet keys name is in the local pack
    /// (skipped when no pack is configured; plays nothing).
    #[test]
    fn the_named_clips_are_in_the_pack() {
        let Some(dir) = pack_dir(GameChoice::GoldenCentury) else {
            return;
        };
        use cabinet::{GambleCue, bet_beep, gamble_cue};
        let cues = [
            GambleCue::Open,
            GambleCue::Toggle,
            GambleCue::Pick,
            GambleCue::Won,
            GambleCue::Lost,
            GambleCue::TakeWin,
        ];
        let names = cues
            .into_iter()
            .filter_map(gamble_cue)
            .chain([1, 2, 3, 5, 10].map(bet_beep))
            .map(|(name, _)| name);
        for name in names {
            let path = std::path::Path::new(&dir).join(format!("audio/{name}.ogg"));
            assert!(path.is_file(), "{} is missing", path.display());
        }
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

    /// The tunes the real game played for these wins (recorded with sound).
    #[test]
    fn wins_play_the_real_games_tunes() {
        let bet = 200;
        assert_eq!(cabinet::win_tune(80, bet), 4);
        assert_eq!(cabinet::win_tune(120, bet), 5);
        assert_eq!(cabinet::win_tune(320, bet), 13);
        assert_eq!(cabinet::win_tune(400, bet), 14);
        // Rising with the win, never past the last tune.
        let mut last = 0;
        for win in (0..40_000).step_by(37) {
            let tune = cabinet::win_tune(win, bet);
            assert!(tune >= last && (1..=32).contains(&tune));
            last = tune;
        }
    }

    #[test]
    fn coins_are_lettered_in_credits_at_cents_and_dollars_at_a_dollar() {
        assert_eq!(cabinet::coin_text(20, 1), "20");
        assert_eq!(cabinet::coin_text(20, 100), "$20");
        assert_eq!(cabinet::coin_text(10, 200), "$20");
    }

    /// No reel shows two flags at once (any three consecutive stops, across
    /// the wrap), in the model or either feature test model.
    #[test]
    fn a_reel_never_shows_two_flags() {
        for model in [
            golden_century_synthetic(),
            feature_test(
                golden_century_synthetic(),
                FEATURE_TEST_ORB,
                FEATURE_TEST_SCATTER,
            ),
            feature_test(golden_century_synthetic(), 0, FEATURE_TEST_SCATTER),
        ] {
            let scatter = model
                .symbols
                .iter()
                .position(|s| s.key == "pennant")
                .expect("pennant");
            for (r, strip) in model
                .reel_strips
                .iter()
                .chain(model.free_games.iter().flat_map(|f| &f.free_reel_strips))
                .enumerate()
            {
                let n = strip.len();
                for i in 0..n {
                    let flags = (0..3)
                        .filter(|k| strip[(i + k) % n].0 as usize == scatter)
                        .count();
                    assert!(flags <= 1, "strip {r} stop {i}: {flags} flags");
                }
            }
        }
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
