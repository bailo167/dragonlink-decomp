# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-21.**

## Later titles' free games: two implementations, one kept (2026-10-21)

Two sessions implemented Spring Festival's BIG reel and Golden Gong's monk at the same time:
feat/later-free-games (merged to main as 891c3e3) and the later titles' audit (feat/title-audit).
The two could not both stand, so the merge kept the audit's and dropped the other's systems
(`BigRun`, `big_column`, its `Expansion` and `wild_expand`, and their tests). Why: the kept one has
the same footage-verified behaviour and more, each re-measured by three checks and a recheck:
- the BIG free games' measured slower pace and the first free game's 2.60 s;
- SND_WindUp and the BIG wild's hold;
- the passing coins' club prizes;
- the BIG coin's backing, box, digits, ring, burst and board, and the gold line frames;
- the monk's SND_Wild and its line, count and next-game holds.

Before the merge, the two things only the other had were ported onto the kept one: the last BIG
picture leaving whole on the first base spin (VERIFIED) and the standing monk scrolling out whole
(HYPOTHESIS). Both have unit tests and muted captures. The other's claims stay in the claim files.
Its two disagreements with the kept one are recorded there: the expansion's sound pairing, and the
passing BIG coins' prizes. A side-by-side comparison is in the private source root at
`research/overnight-2026-10-18/big-reel-compare.md`.

## The seven later titles against their own footage (2026-10-20)

The owner asked that each title be similar to Golden Century but its own game, and that its sounds
be confirmed. Each of the seven later titles was compared, screen by screen, with its own club
footage (about 40 recordings at 1080p with sound, fetched into the private source root), every
difference re-measured by a second reader refute-first, and the confirmed ones implemented as
title data (the new one-table `apps/dragonlink/src/looks.rs`, the packs, the models), never as
title branches. Full account: [reverse-engineering/LATER_TITLES_AUDIT.md](reverse-engineering/LATER_TITLES_AUDIT.md);
claims dated 2026-10-20 in every title's claims file. Every run muted.

- **Shared, measured on several titles:** the trigger's wave lasts the title's own scatter clip
  + 0.12 s; a retrigger replays the trigger (WON status, bell, wave, the banner counting up from
  the old total, no press); a wild win shows its lines at once and holds only WIN's count for the
  title's wild clip; only winning stacks stand, and only where the pack has tall art; tall wilds
  take each title's own rect; animations play at their pack's rate (videos 20 fps); the coin
  shower from 25x with the free games' plaque burning; the counter bursts on every reset;
  per-title end stings (a clip a pack lacks comes from a sibling pack); no SND_EndMask at reveals.
- **Each title:** Autumn Moon's 30 fps moon gate and purple keys; Happy & Prosperous's layered
  Buddha and free-reel picture runs; Panda Magic's per-cell pandas and its own banner numbers;
  Genghis Khan's and Peace & Long Life's x2 wording, x2 wild animation and gold prompt; Golden
  Gong's growing monk, live line tabs and symbol sizes; Peace & Long Life's six-game retrigger,
  fitted sage and dimmed gamble; Spring Festival's BIG reel spinning as one reel at its own pace,
  its BIG coin's digits, ring, burst and board, its BIG animations and line frames, SND_WindUp.
- **Golden Century moved too** (its own club footage): pictures at 20 fps, the first free game's
  reels 1.79 s later, its FEATURE COMPLETED sting Genghis Khan's 2.2 s (one recording; a conflict
  with the official app recorded), no SND_EndMask, the wave 4.87 s.
- Checked on muted captures of every title: three independent fix-checks and a recheck
  re-measured each fix against the footage (the regressions they found are fixed and re-checked);
  unit tests for each new rule. Open, not blockers: the chooser splashes (no art in any pack),
  Spring Festival's leaving-block coins without prizes, the next free game about a second late
  after a retrigger (Peace & Long Life, Spring Festival), full-reel stacks in the later titles'
  synthetic base strips, and the unknowns listed in the audit document. No new external blocker.

## Markets as data (2026-10-20)

- **Markets.** The cabinet's market values are a table (`Market` in `apps/dragonlink/src/cabinet.rs`),
  chosen by `DRAGONLINK_MARKET`: `qld-club` (the default; 1c-$1), `nsw-club` (with $2, which the
  manufacturer lists for NSW and the ACT only) and `star` (The Star, Sydney: 5c 10c 25c 50c $1 $2,
  "Bet Multiplier", 50 fixed lines; `DRAGONLINK_PRESENTATION=casino` still selects it). Each row
  holds the denominations, every denomination's line configuration and deck keys, the
  presentation and the gamble's limits; nothing asks which denomination it is. A machine starts at
  its market's first denomination. The casino style used to play the probably US session's 1c-$2
  with 50/25/5 lines; the $2 club deck now offers one credit a line, as Dragon Cash's does.
- **Gamble limits.** The engine takes the gambles allowed in a row as configuration
  (`GameConfig`); the app takes the money limit from the market and writes it in the system
  messages. A limit that ends a won gamble keeps the win and shows "Gamble Completed - Gamble Limit
  Reached" (the platform's wording, never filmed: HYPOTHESIS), checked on a muted capture of a
  build allowing one gamble.
- **The chooser's PLAY cycle.** Already shown on every chooser since the attract work; it now
  steps through the market's denominations (checked on muted captures, The Star's six wrapping),
  and the Minor and Mini stand clear of the plaque (it covered a Minor of $1,250.00 or more). The
  brief's "about 3.5 s" a step was an early lower bound; the measured 4.0 s stays (conflict 15).
- **Open.** The casino's own Mini and Minor are not modelled, so at The Star's largest bets the
  synthetic return measured about 85% against 88-90% elsewhere; whether the club system message is
  a market's is in conflict on NSW footage.

## Overnight report (2026-10-18 to 19, autonomous, owner-approved plan)

The owner approved one plan for the night: Golden Century to our best-effort 100%, a machine
switcher, then each of the eight Dragon Link titles in the mobile build (scope widened by the
owner; docs/DECISIONS.md). Every run muted. Progress, newest last:

**Morning summary (read this first).** All on `main` (gate passed, pushed):

- All eight titles play in the cabinet from their own private packs, chosen from the machine
  menu on the yellow pill: Golden Century, Autumn Moon, Happy & Prosperous, Panda Magic, Genghis
  Khan, Golden Gong, Peace & Long Life, Spring Festival. Each has its own help-page pay table,
  symbols and free-games feature; the maths of every title is synthetic (85 to 90% at every bet).
- The owner's "two flags in one column" was the moving reel strips, not the game: fixed.
- A side-by-side audit of Golden Century against club footage found 85 differences; two rounds of
  adversarial verification confirmed 69, and all 69 are implemented and checked on muted captures
  (two regressions caught and fixed on the way). Six could not be reached by an unattended run
  ($1 play, a chosen denomination, the gamble's rarer phases) and rest on unit tests.
- Correction: recording y8c8lEdPW8Y is a public Queensland club recording, not the owner's own
  session; an earlier note called it the owner's, and the docs say so now.
- Open, not blockers (ordinary work): the later titles' sound placements are not checked against
  recordings with sound; retrigger counts for three later titles are
  unknown (three used, labelled); the club help pages of every title were never filmed, so GAME
  RULES shows the mobile pages. No new external blocker (docs/BLOCKERS.md unchanged).

- **Golden Century rules.** At most one flag per reel (every strip of every title checked by a
  test; the prepared runs' dressing had added a second flag); the emperor on reel 5 too (help
  screen; the y8c8lEdPW8Y transcription); one MAJOR coin at a time (help screen).
- **Flat return.** Each jackpot's chance is now the stake over its own prize (a progressive's
  restart), so its share of the return is the same at every bet and denomination; with the coin
  values retuned, Golden Century returns about 90% at every bet the decks allow, within about two
  points in 800,000-spin simulations (noise included). Synthetic, not Dragon Link's maths.
- **Packs for all eight titles.** A generic private exporter rebuilds Golden Century's pack byte
  for byte and exported the seven others into the private packs folder (nothing in Git).
- **Every title in the cabinet.** DRAGONLINK_ART may name a folder of packs; the cabinet boots
  any title with its own pack, its symbols' roles read from its model, and the yellow pill opens
  a machine chooser that restarts the app as another title with the same money.
- **Attract.** At zero credit the GAMES MENU opens after 30 s idle (the framework's default,
  HYPOTHESIS); the jackpot bar's PLAY plaque steps through the denominations with their Minor and
  Mini as club footage shows, with fire round the boxes.
- **Rules pages and sounds.** GAME RULES opens the title's own help pages from its pack (mobile
  layout; the club pages were never filmed); the Grand's bell, the fourteen-coin drum roll and the
  pictures' own win sounds placed from the sound hunt.
- **Autumn Moon.** In the mobile build it is Golden Century in other art, so its model is Golden
  Century's in its symbols, with its own pay table (club wins fit the mobile table, STRONG) and a
  six-game retrigger (three club recordings); it plays every feature in the cabinet with its own
  art, the prepared runs included (checked on a muted capture).
- **Happy & Prosperous and Panda Magic.** In the mobile build each is Golden Century's package
  less the mystery reveal and the tall wild; their models are Golden Century's in their symbols,
  without doors, with royal-free free reels (synthetic) and three-game retriggers (mobile and club
  footage) and the family's mobile pay table (the club table is unknown). Both play in the cabinet
  with their own art and return about 90% at every bet (checked on muted captures).
- **The four later titles.** Genghis Khan, Golden Gong, Peace & Long Life and Spring Festival
  share one base game read from their help pages: the family's fifty lines (Genghis Khan's and
  Golden Gong's pages, identical; Peace & Long Life and Spring Festival show the first 25 on
  mobile and sell up to 50 on club machines), one pay table, a wild with no pay of its own and a
  scatter paying 2, 15 and 100 times the bet. Each has its own free games: Genghis Khan's and
  Peace & Long Life's x2 wild (two make x4), Golden Gong's monk filling its reel and its own pay
  table, Spring Festival's BIG 3x3 reel over reels 2 to 4 with BIG ingot retriggers and a BIG coin
  that starts Hold & Spin with nine positions held. Coins in their free games start no Hold &
  Spin (three help pages say bought games only). The engine gained these as model data, tested on
  hand-made models. All four play in the cabinet with their own art and sounds (checked on muted
  captures); synthetic maths returning 85 to 88% at every bet.
- **The two flags in one column, found.** The engine never lands two scatters on a reel, but the
  moving reels scrolled the undressed base strips with the landed window spliced in anywhere, so
  a passing flag could show beside a landed one (the overnight audit traced the owner's sighting
  to this). The reels now scroll the strips of the model being played, the free strips in free
  games, and a reel whose window leaving or coming holds a scatter shows no other that spin.
- **Audit fixes: deck, meters and messages.** The button deck and bezel now sit above every
  screen effect (coin comets and the coin shower stop at the bezel line, as at the glass's edge).
  Line wins always read in credits ("Line 17 wins 10"; 306 club messages, none in dollars). The
  jackpot values are coloured by level, Grand crimson, Major green, Minor blue, Mini orange, in
  heavy digits with a dark edge (hue VERIFIED on cabinet and app; tints from camera colour). The
  idle status line's 2 s alternation now counts from the end of the game or a bet change, bet
  line first, with a 0.1 s "Game Over - Play Now" at each debit (VERIFIED). The message row's start
  prompt changes only at the first game start after a base game's win, never on a timer (STRONG;
  after a feature HYPOTHESIS). Unit tests cover each rule; checked on muted captures.
- **Audit fixes: lines boxes and line badges.** The club's "N LINES" boxes are now the large
  panels club footage shows (geometry STRONG, look VERIFIED on four machines): the game's own
  lines-box fill, a gold rim, the inner edge under the reel frame, "50" over "LINES" at the
  cabinet's glyph heights (the number in the game's own brush-serif numerals, Impact standing in
  for "LINES"). Golden Gong, whose reel art
  carries its own tabs, now has its reel panel fitted so the frame sits on the reels and its tabs
  stand outside it, where the cabinet's boxes are; it gets no panels of ours. Winning lines show
  the cabinet's pieces: badges sized to the denomination's line configuration (small at 50 lines,
  wide at 25 and 5), one badge per line at 50 and 25 lines on the side filmed for that line (both
  ends at 5 lines and for line 1 at 25), outside the frame with a stub across it (sizes and count
  STRONG; sides VERIFIED for the lines filmed, unfilmed lines by parity HYPOTHESIS). Unit tests
  cover the sizes, sides, crop and panel fit; checked on muted captures at 50, 25 and 5 lines.
- **Audit fixes: Hold & Spin.** The intro is the cabinet's: one fire straight into the Hold & Spin
  board (no medallion over the reels, no second fire), timed to two club recordings, the prompt
  3.5 s after the coins burst (STRONG; it was 4.6 s). Both features' prompt reads PRESS START
  FEATURE BUTTON, our own cream lettering over the banner's bar, the lettering alone blinking on a
  0.667 s cycle (VERIFIED). Every Hold & Spin's coins now fly to a flame-bordered WINNER box on a
  plate over the jackpot bar, with the live Grand and Major under it and nothing over the reels;
  the count's WIN banner counts up from nothing with WIN, and the plate goes 0.55 s after the count
  (VERIFIED on three club recordings; the plate's layout is ours, the line's colour PARTIAL). The
  official app's medallion, SELECT SPIN wording and banner tally are recorded as the app's. Unit
  tests cover the intro's timing, the blink, the collection and the comets; checked on muted
  captures (both features, the jackpots and doors runs, Golden Gong).
- **Audit fixes: free games.** The free-games trigger now runs as on club cabinets (VERIFIED on
  two recordings): "6 FREE GAMES WON!" on the status line from the result, "Scatter wins N" from
  the flags' wave, the PRESS START FEATURE prompt as the scatter win starts counting (the count
  goes on under it; it waited for the count before), the status alternating in 2.0 s slots from
  the prompt's start, Press first, and the blink restarting as the count completes. At the press
  the flags go still, both rows blank, and the first free game's reels, banner and counting total
  come 0.52 s later (1.2 s before, with the banner ahead of the reels). A free game's "WIN $x WIN"
  banner now shows only for a win of 10x the bet or more (the rule STRONG, the 10x limit
  HYPOTHESIS between the 6.0x and 11.0x the footage brackets), held 1.0 s after its count. After
  a Hold & Spin inside the free games the board stays dark with no banner for 0.43 s, then "FREE
  GAME N+1 OF 6" comes with the reels (VERIFIED, one machine), and the moving strips scroll an
  open position's dimmed symbol away still dimmed. Unit tests cover each rule (one runs the reel
  system on a world); checked on muted captures of the doors run.
- **Audit fixes: system message and meters.** The club profile now fills the message row's left
  slot as Queensland club machines do (texts and place VERIFIED, rules STRONG on more than 30
  transitions): "Gamble win limit is $10000.00" while a base game's win is offered, "Credit
  reached $100.00, cash-in disabled" once after the credit rises to $100, and otherwise the gamble
  notice and a greeting in turn, switching 2.0 s after each start and, with no game, every 2.0 s
  (HYPOTHESIS). The greeting is a generic "Welcome" standing in for the venue's own line. After a
  denomination choice the club meters alternate dollars and credits each second until the first
  game, the clock counting from the YES (VERIFIED on five cases; the phase PARTIAL), and BET then
  shows the last bet in credits of the new denomination. Unit tests cover the rules; checked on
  muted captures (a choice of 5c, the base game with wins, a Hold & Spin, the casino profile).
- **Audit fixes verified.** Muted captures of Golden Century (both, doors and a plain base run)
  and of Golden Gong and Spring Festival with frequent features were checked against each fix.
  Two slips were mended: free games won on a Hold & Spin's spin started in the frame of its
  award, so the WIN meter counted the whole Hold & Spin total a second time, coin shower and all,
  under their prompt (the free games now start an event interval later, as before); and the
  WINNER plate stayed 0.95 s after the count, not 0.55 s (it counted from the payment, 0.4 s
  after the count). Golden Gong's monk and Spring Festival's BIG reel still play as before.
- **Audit fixes, second pass: Hold & Spin.** A second verification of the audit, frame by frame at
  30 fps on four club recordings, corrected the board and the collection (all VERIFIED unless
  said). The first respin now starts 0.59 s after the counter's 3 (cabinets 0.55-0.63 s; the app
  had waited about 1.4 s, its hold meant to precede a step pause that never applied there). The
  burst on the 3 is a short blue-white flash, the pack's burst recoloured at load, 0.45 s (its size
  PARTIAL). The board's row lines are thin and pale under the gold dividers (the pack frame's own
  cleared at load; the hue PARTIAL). The status line follows the cabinet through the feature:
  blank through the intro and running respins, "Press START FEATURE button" through the count-up,
  "Press SPIN button" between respins, "Free Spins Completed" through the tally and the count
  (inside the free games "Free Game N of 6", STRONG). The WINNER box shows in the frame FREE SPINS
  COMPLETED goes, the first coin 0.38 s later, each comet 0.67 s and landing with its value (the
  bolt's frames stretched to its flight); the WIN banner's amount is white. Unit tests cover each;
  checked on muted captures (the jackpots run, Spring Festival's BIG coin feature).
- **Audit fixes, second pass: jackpots.** The Grand restarts at $10,000, as a Golden Century
  machine shows after its Grand (VERIFIED there, STRONG for Australian Golden Century), opening
  below $13,000 and stopping at $25,000 (synthetic; a club Grand was won at $24,122.63); the
  synthetic return stays flat at about 90% at every bet (the flat-return test re-run with the new
  meters). Both meters now climb a cent or two every few seconds, the club machines' order of rate
  (STRONG on five machines; it was 14-100 times faster), and a changed figure rolls up inside its
  box over 0.37 s (VERIFIED on one machine). Every meter holds while a resolved respin plays out,
  so a won meter no longer stops alone before its coin lands (HYPOTHESIS). Through Hold & Spin a
  plate over the jackpot bar shows the cabinet's disc, "N COLLECTED / 15 WIN / GRAND JACKPOT",
  counting each coin as it lands, with the four jackpots at its corners and the game's fire dragons
  (the disc VERIFIED, our layout HYPOTHESIS). A Major is awarded first on the pack's MAJOR JACKPOT
  plaque, its amount green, with its own count and coin shower, then the coins (STRONG on an Autumn
  Moon club recording, PARTIAL for Golden Century; WIN keeping it is the owner's call). A Grand
  comes under the pack's fire at 30 fps, the plaque switching on under full cover (the fire VERIFIED
  on one machine), with "GRAND JACKPOT Win $x" on the status line and "Call Attendant : JACKPOT HIT
  $x" on the club's system line (STRONG). After a base game's Hold & Spin the status line turns
  PLAY ON and "Free Spins Completed" while the win is offered (VERIFIED on a Happy & Prosperous club
  recording, PARTIAL for Golden Century). The feature banner moved 10 px down, clear of the Major's
  and Minor's values. Unit tests cover each; checked on muted captures (the jackpots and both runs,
  a base run for the roll).

- **Audit fixes, second pass: base game and free games.** Checked frame by frame on club
  recordings and followed (each label in [GOLDEN_CENTURY_PRESENTATION.md](reverse-engineering/GOLDEN_CENTURY_PRESENTATION.md)
  and [GOLDEN_CENTURY_CABINET_DECK.md](reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md)): a
  winning line blinks in two even 0.66 s cycles, its badges lit exactly while its symbols show
  (STRONG; the app's uneven 1.45 s was the mobile app's); the first free game's banner counts from
  "1 OF 0", a step every 5 frames, the status blank until a step after the total (VERIFIED); the
  doors open 0.30 s after the rest and faster, the wins at the rest + 1.5 s, a no-win free game's
  next reels at the rest + 2.0 s (STRONG; PARTIAL for the wins); FEATURE COMPLETED waits for the
  last count, "Feature Completed" then the offer, CREDIT no longer flickering to the total
  (STRONG); a free game's WIN banner no longer flashes the last game's amount (our glitch, fixed);
  no start prompt from a denomination choice to the first game (STRONG). The club's meter labels
  shade orange to yellow and its denomination yellow to orange (direction VERIFIED, tints PARTIAL),
  the status and message texts have a dark rim all round (STRONG), and the club's tall pill has our
  drawing of its signal-like icon on a yellow-to-orange pill (shapes VERIFIED). The prepared doors
  run lands its doors in short vertical runs, the other prepared runs' free reels the model's own
  (synthetic). Unit tests cover each; checked on muted captures (the doors run, Golden Gong, Spring
  Festival, the casino profile) and an event trace (doors at the rest + 0.30 s, wins + 1.50 s,
  no-win next game + 2.00 s).
- **Audit fixes, second pass: the button deck.** Checked frame by frame on club recordings
  ([GOLDEN_CENTURY_CABINET_DECK.md](reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md)): every Hold
  & Spin respin now spells GOOD LUCK, as each cabinet respin does (VERIFIED); as the reels start
  each key keeps its idle face until its letter arrives, and as they stop the idle faces wipe
  back from the left, 0.08 s a column (VERIFIED, one machine). The teardrop points up (four
  machines) and is dark and smoky with a pale-blue legend, lit blue only for START FEATURE, which
  stays lit on the club profile with its legend turning like a coin (STRONG for deck A; the
  casino profile keeps deck B's blink). GAMBLE and TAKE WIN are always lit, TAKE WIN pale khaki
  with blue letters; the selected lines key is light blue with white ink and the selected credits
  key pale with a red numeral; the credits keys a denomination does not offer are lit; on the
  flash's plain half the keys turn light blue with faint legends; in the gamble the black choices
  are dark with light-blue icons and the unused keys a white-silver lattice. The keys are an eighth
  of a key apart, the teardrop about 1.7 keys wide a quarter key clear (STRONG): each key's art had
  been drawn inside its legend padding, widening the gaps and squashing the teardrop. The teardrop
  and the lattice key are our own new drawings in the local packs (nothing in Git). Unit tests
  cover each rule; checked on muted captures (base, the minors run, the autogambler, Golden Gong,
  Spring Festival, the casino profile).
- **Audit fixes, second pass: the gamble and the GAMES MENU.** Checked on club recordings
  ([GOLDEN_CENTURY_CABINET_DECK.md](reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md)): nothing of
  the base screen stands over the gamble any more (FEATURE COMPLETED's plate and a tall wild's head
  showed above its top), and it comes back after a loss (VERIFIED, two machines); the club's COLLECT
  and GAMES MENU stay over the gamble screen (VERIFIED, three machines); the gamble prompt is the bet
  line's yellow, widely spaced (VERIFIED on two club machines; an NSW video-deck machine and the
  owner's Happy & Prosperous show it pale, a conflict recorded); the picked button fills with the
  card's face, not at the press (STRONG); the teardrop shows a card-back lattice through the gamble
  (VERIFIED, one machine; our own drawing, `spin_lattice`, in the local packs, the dark teardrop
  where a pack lacks it). The chooser is laid out as two recordings measure it by homography
  (STRONG): an opaque panel about 995 x 195 centred near y 755, bevelled red-orange and amber keys
  162 x 80 with white figures in a navy outline and a smaller cent sign, a larger cream header, ARE
  YOU SURE? wholly above the panel with yellow and cream lines and bevelled YES / NO (sizes
  PARTIAL), and the drawn title where the logo stands, about 1565 px wide, now each title's own
  name (a longer one set smaller); the club chooser keeps a COLLECT pill moved to the bottom right
  corner (STRONG, one machine). `DRAGONLINK_SHOW=menu` now waits for the first rest instead of
  testing one frame, and `DRAGONLINK_SHOW=confirm` also opens ARE YOU SURE?, for captures. Unit
  tests cover each (three spawn the cabinet into a world); checked on muted captures (the chooser
  with its confirmation, a base run's gamble).

## Project interpretation (owner reset, 2026-10-14)

**The target is to recreate the actual Dragon Link gaming-machine experience as closely as
technically possible — launching Golden Century should feel like sitting at a real Dragon Link
machine, recognisable against the reference footage.** The current builds do **not** meet that
standard yet. Golden Century now runs with the authentic art, animations and sound of the owner's
mobile build (from a local pack, never committed), and its gameplay follows timings and sounds
measured on a recording of the official app; it is still not "finished": side-by-side comparison
throughout, unplaced sounds, attract mode and the hidden maths remain.

**Priority is absolute: Golden Century cabinet experience first. Every other title is frozen**
(Autumn Moon, Happy & Prosperous, Panda Magic, and the further titles) until Golden Century reaches
the cabinet-experience standard, after which the proven systems propagate.

## In one paragraph

The repository has its foundation, a headless game model, an evidence base, a mathematics analysis
subsystem, and **functional-prototype** gameplay for four titles on synthetic maths behind a
launcher. Golden Century is being rebuilt as the authentic experience (P0): it plays with the game's
own art, animations and sound from a local pack, in a cabinet frame the owner chose, its gameplay
matched step by step to a recording of the official app
([reverse-engineering/GOLDEN_CENTURY_PRESENTATION.md](reverse-engineering/GOLDEN_CENTURY_PRESENTATION.md)). **A major enabler just landed: the
official mobile art and audio are now extractable.** The Mercury container parser plus a new raw-RGBA
and ETC1 texture decoder recover Golden Century's authentic artwork, fonts and 114 Ogg audio clips to
viewable files, and the game's own help/paytable screens were read to recover the **authentic
player-visible paytable** (docs/reverse-engineering/GOLDEN_CENTURY_PAYTABLE.md). The encrypted `.enc`
math tables and security-relevant native code remain out of scope (D-014/D-016); hidden probabilities
stay synthetic (D-017). Assets never enter Git; they build into a local, ignored runtime pack. Work
is pushed to GitHub and CI is green on Linux, Windows, macOS and wasm32.

## Done

### Foundation (2026-10-13)

- Rust workspace, toolchain pinned to 1.98.0, dual MIT/Apache-2.0 licence.
- `tools/repo-hygiene`, adapted from ASAMU-decomp and widened for this project's material.
- `tools/progress-gen`, carried over; `progress/progress.toml` populated with the whole roadmap.
- The Windows gate (`tools/dev/gate.ps1`) and host setup script; CI, green on first run.
- Policy documents: legal scope, evidence and provenance, source material, architecture, parity,
  building, decisions, blockers, prior art.

### Headless model (2026-10-13)

- `dragonlink-model`: confidence labels and source classes enforced in code; the claims database;
  runnable math models with provenance; the synthetic development fixture.
- `dragonlink-core`: command/event state machine (idle, Hold & Spin), bet and credit accounting,
  reel windows, line and scatter evaluation, a parameterised Hold & Spin loop with jackpots,
  transactional commands, save and resume, and seeded, recording and scripted outcome sources.

### First evidence pass (2026-10-13)

- 99 sourced claims across the family and the four titles, each scoped by market.
- ACT instruments (read twice) and Victorian approvals (read once): specification numbers for all
  four titles across three Australian generations, on Gen8 (Linux-based; Helix+), Gen9 Mars X and
  Gen10 Baron.
- The manufacturer's complaint in *Aristocrat v. Light & Wonder*: the Hold & Spin rules of the US
  family. The court's order: the mathematics is asserted as trade secrets and is not public.
- Research documents: regulatory, platforms, features, patents, mobile reference, reference media,
  open questions and conflicts, one page per title.

### Mathematics and analysis (2026-10-13)

- `dragonlink-math`: game configurations with per-constant provenance; exact line and scatter
  return; exact Hold & Spin analysis by recursion; hit frequency and base variance by full
  enumeration; seeded Monte Carlo with intervals; progressive expected value and break-even meters;
  configuration comparison; the per-game report.
- `dragonlink-analyze`: `analyze-game`, `compare-games`, `progressive-ev`, `simulate`,
  `export-fixture`.
- The exact analysis, full enumeration and simulation agree with each other in the tests.
- The synthetic fixture was retuned with the tool (`synthetic-dev-2`).
- The lane's documents under `docs/reverse-engineering/math/`.

### Source root and inventory (2026-10-13)

- `dragonlink-formats`: file identification by content (public formats only, unknown stays
  unknown), entropy, and a bounds-checked central-directory reader for ZIP-family packages, tested
  on hand-built archives, every truncation and every single-byte corruption.
- `dragonlink-inventory`: `init` and `check` for the private source root, `scan` to hash and
  classify a folder into an inventory with no absolute paths.
- The source root exists on the Windows host, outside every repository, and the boundary between
  it and Git was exercised end to end ([BUILDING.md](BUILDING.md), "Source root on the host").

### Application shell (2026-10-13)

- `apps/dragonlink`: a Bevy 0.20 application that sends commands to the simulation and presents
  its events: a 5×3 window of generated cells, staggered reel stops, orb values, Hold & Spin
  stepping, a credit meter that follows presented events, a status line, and a permanent
  placeholder banner. Keyboard input. `--smoke-frames` for unattended runs on a fixed step, with
  `--screenshot`.
- The gate now runs all nine steps on the Windows host: the app builds in debug and release,
  smoke-launches a real window, and compiles for wasm32.
- Looking at the first screenshots found and fixed three presentation mistakes: a glyph missing
  from the built-in font, the meter showing a win before the reels had landed, and the previous
  spin's result lingering on reels still in motion.

### Traces (2026-10-13)

- `dragonlink-trace`: one JSON Lines format for rewrite traces and transcribed observations,
  validation that refuses mislabelled headers, and an event-by-event comparison with timing
  statistics that says whether it was a determinism check, a structural comparison, parity, or a
  comparison between observations. The command-line tool records, validates and compares.
- Only determinism checks are possible so far: there is no observation of the original.

### Manifest, importer, packer (2026-10-13)

- `dragonlink-assets`: the manifest of a normalised asset set, its validation, verification of
  files against it, and discovery at run time with a plain-words reason when there is no asset set.
- `dragonlink-import`, stage 1: loose standard media copied into the asset set with provenance and
  hashes; idempotent; the source is never modified.
- `dragonlink-pack`: `verify`, and `assemble` for a local full package with a do-not-distribute
  notice, refused inside a Git work tree.
- The application reports at start-up why it is in placeholder mode.
- `dragonlink-inspect`: one unknown file measured (type, hash, entropy overall and by block,
  strings statistics, archive summary) without printing any of its content.
- The whole chain was run on the Windows host's source root with synthetic files: import, verify,
  discovery by the application, package assembly, tamper detection.

### First original material: the official mobile package (2026-10-13)

- Google's Android emulator with a Play Store image was installed on the Windows host; the owner
  signed in and installed Mighty Fu Casino 5.6.0 and opened Golden Century.
- The four installed APK files and the 113 files the app downloaded were copied into the private
  source root unmodified, hashed, given a provenance record and inventoried with the project's tools.
- Measured: a Unity IL2CPP app; each game delivered as a folder of unencrypted `.mercury`
  containers and `.enc` tables whose names include reel strips, weightings and four variations; and
  plain-text strings written for a gaming machine, which is strong evidence that the mobile build
  hosts a package built on the manufacturer's own game framework
  ([reverse-engineering/MOBILE_PACKAGE.md](reverse-engineering/MOBILE_PACKAGE.md)).
- A structural analysis identified the app's native game runtime, worked out the container format,
  and found, re-checked by hash, that the four titles in scope are two pairs sharing their table
  files: Golden Century with Autumn Moon, Happy & Prosperous with Panda Magic.
- Decrypting the protected tables, and exporting the help artwork, were stopped by the safety
  controls of the tooling. That is treated as a boundary (DECISIONS D-014, BLOCKERS B-005). Nothing
  has been decrypted, exported or imported.

### The litigation record (2026-10-13)

- The free public record of the US and Australian proceedings was read
  ([reverse-engineering/LITIGATION.md](reverse-engineering/LITIGATION.md)). It holds no number for the
  mathematics. It does give the Australian launch year (2016), an adverse party's admission of the
  window and denominations, sworn evidence that the feature draws from its own set of reels, and a
  description of base reel strips being rebuilt from public gameplay video.
- That last point sets the route for cabinet-side reel data: read reel windows from public footage
  and assemble the strips (DECISIONS D-015).
- An assertion in the filings that confidential PAR sheets were posted online is recorded and not
  followed.

### Online evidence passes: archives, regulators, platform (2026-10-13)

- **The Australian configuration is established** from the manufacturer's own archived pages: 50-,
  25- and 5-line configurations, denominations per state, the Helix launch cabinets, each title's
  free-games rule, and at strong evidence the jackpot structure
  ([reverse-engineering/AUSTRALIAN_CONFIGURATION.md](reverse-engineering/AUSTRALIAN_CONFIGURATION.md)).
- **Return figures exist publicly after all**: a US regulator prints a payout range for Golden
  Century and Happy & Prosperous, and the ACT printed Lightning Link's return per denomination in
  2015. No Australian figure for Dragon Link was found
  ([reverse-engineering/PUBLISHED_RETURNS.md](reverse-engineering/PUBLISHED_RETURNS.md)).
- **What regulation requires** on outcome selection, timing, independence and jackpots is recorded
  with clause numbers, answering several of the mathematics lane's questions as constraints.
- **Platform lineage** from Gen7 to Gen10 is documented; the internals of Gen8 and later are not
  public ([reverse-engineering/PLATFORM_SOFTWARE.md](reverse-engineering/PLATFORM_SOFTWARE.md)).
- The blocker entries are narrowed to what these passes could not find.

### Footage pass: Happy & Prosperous, Panda Magic, Autumn Moon (2026-10-13)

- About 150 to 170 public videos per title were scanned and the candidates read
  ([reverse-engineering/FOOTAGE.md](reverse-engineering/FOOTAGE.md)).
- The US pay table and rules screens of all three were transcribed. The three share one pay table,
  and it is the one quoted in the manufacturer's complaint.
- From Australian play: 50 lines at 1c and 2c, 25 at 5c and 10c, 5 at $1 on Queensland club
  machines; six free games with the retriggers; each title's free-games behaviour, confirming the
  manufacturer's pages; and that the upper two jackpot levels move while the lower two are fixed.
- No Australian rules, pay-table or information screen was found, and no return figure.

### Footage pass: Golden Century (2026-10-13)

- 175 videos triaged and 182 caption tracks searched. **No rules, pay-table or information page for
  Golden Century was found in any recording**, so the first title has no pay table from footage.
- Read from Australian play: lines are chosen by the player up to 50, 25 or 5 by denomination; six
  free games from three scatters; the mystery-symbol reveal; the Hold & Spin prompts and counters;
  the Mini and Minor fixed per denomination; the Grand and Major both incrementing.
- Much footage titled Dragon Link is the stand-alone Dragon Cash, and no recording can be tied to a
  build generation.
- Next for this gap: read pays from the machine's line-win messages; a denser scan.

### Three code lanes finished (2026-10-13)

Each was started by a delegated agent that stopped part-way; each was reviewed, completed, tested
and merged through the gate.

- **Reading archives safely.** `dragonlink-formats` reads an entry of a ZIP-family archive, stored
  or deflated, and returns only bytes whose size and CRC-32 verify and whose local header agrees
  with the directory, within a per-archive decompression budget. `dragonlink-import` uses it for a
  second stage: standard media inside archives. Tested on synthetic archives written byte by byte,
  including bombs, lying sizes and every single-byte corruption of a small archive. A dry run on
  the held mobile package read fourteen archives with no entry failing; nothing was imported.
- **Replay of observed outcomes.** `dragonlink-trace` scripts the outcomes a trace shows through
  the simulation and reports what the simulation then did differently
  ([PARITY.md](PARITY.md), "Replay"). It is the tool an observation transcribed from footage will
  be run through. It has only ever been given our own traces.
- **The browser.** `tools/dev/web.ps1` bundles the placeholder shell for the web and checks it in a
  headless browser: it starts, shows its placeholder banner, and spins on a key press
  ([BUILDING.md](BUILDING.md), "The web bundle").

### The passes' findings entered as claims (2026-10-13)

- The claims database went from 99 to 124 claims: 45 verified, 49 strong evidence, 9 partial,
  21 open questions. Seventeen questions that were open this morning now have sourced answers
  (lines, denominations and symbols for each title, the pay table of three titles as shown in the
  US, which jackpot levels are progressive, the launch cabinets), and new claims record the launch
  year, the free-games award and the fixed prize values.
- **Labelling of footage.** A reading from footage is entered as strong evidence at most, even
  where two recordings agree, because each was read in one pass by one reader. An independent
  second reading is what would make one verified.
- The knowledge base was corrected where the passes overtook it: an Autumn Moon 2026 build exists
  (1.HD138); return figures do exist publicly, in the United States; the launch cabinet is
  established; the "250x" sentence is on all four Australian pages and matches no screen.
  [reverse-engineering/OPEN_QUESTIONS.md](reverse-engineering/OPEN_QUESTIONS.md) now says which
  conflicts are settled and by what.

### From a recording to a trace (2026-10-13)

- `dragonlink-capture`: finds the moment each game in a recording came to rest and cuts one still
  per game. On the held 53-minute Golden Century session it found 583 games.
- `dragonlink-trace transcribe`: a sheet of transcribed windows becomes an observation trace of one
  configuration. Nothing is filled in, and the meters are checked against each other
  ([PARITY.md](PARITY.md), "From a recording to an observation trace").
- The 583 stills are being read into a window sheet. Stills and sheet are kept privately with the
  recording.

### The first transcribed session (2026-10-13)

- A 53-minute Australian session of Golden Century was read game by game: 583 games, every one
  read, 1,808 reel windows read twice by different readers with no disagreement
  ([reverse-engineering/GOLDEN_CENTURY_SESSION.md](reverse-engineering/GOLDEN_CENTURY_SESSION.md)).
- **Golden Century has pay-table entries at last**: sixteen, read from the machine's own win
  messages, with the shapes of lines 1 to 5.
- **The $1 game is a different configuration from the 1c to 10c games** on that machine: different
  reels, and one pay seen to differ. Changing the number of lines does not change the reels.
- Reel reconstruction was extended to windows that sit at more than one stop (recurring runs,
  stacks). With it two of the five $1 reels assemble as candidates; three still miss windows.
  More sessions are the remedy; the tooling takes them.
- The entries were checked against a separate reading of the credit meter: all 221 $1 games that
  could be checked paid exactly what the entries and line shapes give.

### The mobile game's engine, described (2026-10-13)

- The owner handed over a static analysis of the mobile app's code, done separately from this
  project's tooling: a report, reference routines and a function map, kept privately.
- It establishes how the mobile Golden Century computes a round: mathematics inside the app with
  backend economy and jackpot settlement; weighted cyclic reels with the stop in the middle row;
  prize-symbol values assigned over the whole strip before the window; one reveal symbol for all
  masked positions; a strip per position in Hold & Spin; prize codes looked up in per-bet lists
  ([reverse-engineering/MOBILE_MATH_ARCHITECTURE.md](reverse-engineering/MOBILE_MATH_ARCHITECTURE.md)).
- No protected table was read. Every reel, weight, pay and probability of the mobile game is
  still unknown, and B-005 narrows to exactly that.
- Details bearing on the security of the live app are not published or built on (DECISIONS D-016).

### Official mobile art and audio are extractable; authentic paytable recovered (2026-10-14)

- The Mercury container format was already parsed; a **raw-RGBA and ETC1 texture decoder** was added
  (dependency-free), so Golden Century's authentic textures (symbols, backgrounds, reel art, logo,
  the red/gold number glyph atlases), 18 embedded fonts and **114 Ogg/Vorbis audio clips** now
  decode to viewable PNG/Ogg files. Verified by eye on the decoded output.
- Reading the game's own help/paytable textures recovered the **authentic player-visible paytable**:
  5 fixed paylines and their shapes, a line cost of 5 coins, all symbol pays (emperor/wild 500/100/
  25/5, warriors 250/50/10/3, women 150/30/5/2, vase and chest 100/25/5/2, K 75/20/5, Q/J/10/9
  50/10/5), scatter 100/10/2 × total bet, wild on reels 2–5 only, 6 free games, orb prizes and
  jackpot rules (docs/reverse-engineering/GOLDEN_CENTURY_PAYTABLE.md). This supersedes the invented
  Golden Century pay values.
- **Boundary held:** only the unencrypted `.mercury` art/audio is touched. The `.enc` math tables
  and native code are not decrypted or analysed (D-014/D-016); hidden probabilities stay synthetic
  (D-017). Nothing from the source root is committed; assets build into a local, ignored pack.
- This is the enabler for the P0 rebuild: render the authentic cabinet experience instead of
  placeholder rectangles.

### Four functional-prototype titles, a launcher and hardening (2026-10-14)

*These are functional prototypes — working simulation under placeholder visuals, not the cabinet
experience. Golden Century is the one under active rebuild; the other three are frozen.*

- **Autumn Moon, Happy & Prosperous and Panda Magic** play end to end as prototypes on the Golden
  Century
  engine, each as its own synthetic configuration with the title's evidenced structure (its symbols,
  the US family pay table, its free-games rule) and invented strips, weights, land chance and jackpot
  amounts. Autumn Moon has the free-games mystery reveal; Happy & Prosperous and Panda Magic remove
  the royals inside the feature. Selected with `--game autumn-moon|happy-prosperous|panda-magic`.
  Synthetic returns about 94%, 89% and 89%; never to be quoted as Dragon Link's.
- **A four-title launcher**: an ordinary start lists the four titles and the player picks one; a
  `--game` flag or a smoke run boots straight into a game (so the gate and tests never wait on a
  menu). Implemented as a Bevy state split (`Screen::Menu` / `Screen::Playing`).
- **Hardening**: a cross-title soak test plays every title for 25,000 spins across two seeds, varying
  the stake, and checks a spin never fails, the game always returns to the base game, both features
  are reached, and each title is deterministic and replays event for event.
- **Feature and jackpot banners**: a large centered banner announces FREE GAMES, HOLD & SPIN,
  RETRIGGER, FULL GRID and the jackpot tiers, across all four titles.
- **Known fidelity gaps** (documented in the fixtures and the tracker): Panda Magic's Hold & Spin can
  start inside its free games on the real machine; Autumn Moon's free-games wild has its own line pay.
  Neither is modelled yet. No themed artwork or audio (assets are blocked, B-005). No authentic math.

### Golden Century gameplay matched to the official app (2026-10-14)

The owner recorded the official mobile app with sound (a 3:41 session and six excerpts). Every
topic was measured frame by frame and sound by sound, and re-measured by an independent verifier
([reverse-engineering/GOLDEN_CENTURY_PRESENTATION.md](reverse-engineering/GOLDEN_CENTURY_PRESENTATION.md)).
The app now follows it:

- **Reels.** Measured speed, cadence and landing (on past the stop, snap back, slide home); every
  reel lands within a frame of the real timing in a timed run. Results wait 0.17 s after the reels
  are still, and a spin is refused until then (this fixed wins showing while the last reel moved).
- **Wins.** The real tunes at the measured win sizes, counts lasting the tune's audible length, lines
  in turn with their symbols blinking, stacked emperors standing only on a winning line with
  SND_Wild_2/3. The synthetic strips now deal cards apart, so columns mix their cards.
- **Free games.** Bell, waving flags with SND_Scatter, the scatter win, the prompt, a silent press
  and a cut into the free games with their music and counting total; doors at once with
  SND_EndMask; measured pace; FEATURE COMPLETED with SND_Fea_End. Retriggers add three (owner).
- **Hold & Spin.** The trigger, fire and medallion (about 1 s, and no COLLECT screen: the owner's
  changes), the counter, respins stopping by reel with their sounds, and the payout: coins added to
  a WIN banner by fire comets, then a coin shower and the payout music, found in the app's shared
  sound bank (it also plays under the cabinet's big counts). Updated 2026-10-19: the intro, the
  prompt and the collection now follow club cabinets (one fire, no medallion, PRESS START FEATURE
  BUTTON, the coins to a WINNER box over the jackpot bar; see the overnight report).

### Button deck, GAMES MENU and gamble rebuilt from cabinet footage (2026-10-17)

Public cabinet footage of Golden Century, the owner's own recording of a Dragon Link cabinet,
the mobile build's scene files and the national standard were read again by eight readers, and
every finding re-checked by an independent verifier (262 confirmed, 61 corrected, 1 refuted;
[reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md](reverse-engineering/GOLDEN_CENTURY_CABINET_DECK.md)).
Two Australian presentations exist. The **club** one (credits per line, lines keys that play,
a chooser asking ARE YOU SURE?) is the owner's recorded cabinet and the only Golden Century deck
legible on footage. The **casino** one ("Bet Multiplier", fixed lines) rests mostly on a long
session that a reflection places at a US casino; no Australian casino deck is legible. The app
now holds the presentation as data, club by default (`DRAGONLINK_PRESENTATION=casino` for the
other):

- **Deck (club).** GAMBLE over TAKE WIN; credits keys along the top that only select, lines keys
  along the bottom that set the lines and play, the selected keys pale, keys not offered blank;
  BET stays stale until the next game. The teardrop reads PLAY / REPEAT BET at rest, START FEATURE
  (blinking) while a feature waits, SPIN in Hold & Spin, a lattice in the gamble. GOOD / LUCK
  wipes in as the reels start; CHOOSE on the menu.
- **One clock.** The keys flash at 1 Hz while a win is offered and in the gamble, and the meters
  switch to credits every other second then only (the standard forbids it during a play).
- **GAMES MENU.** The title splash with CHOOSE YOUR DENOMINATION / TOUCH SCREEN TO MAKE
  SELECTION every 2 s; club 1c-$1 with "<denomination> GAME / ARE YOU SURE?", casino 1c-$2 at
  once; each denomination's last reels return. A change keeps the money: the core recounts the
  credits and holds the cents left over (`ChangeDenomination`).
- **Gamble.** Golden Century's own key layout (suits on the top row, BLACK bottom right, a lattice
  on unused keys); cards shown 1.0 s; Suit and Red/Black boxes alternating every 2 s from entry;
  no gamble after a Major or Grand or past the $10,000 gamble win limit, suits off when four times
  the stake would pass it; Gamble Completed messages; GAMBLE completes a running count. A half
  gamble on the top-left key is kept as a labelled hypothesis.

Not seen on footage and so not claimed: RED's and TAKE WIN's keys in the gamble, the credits row
at 5c and 10c, an Australian casino deck, club deck sounds, and the cabinet's own splash art (not
in the mobile build; the backdrop carries a drawn title).

### The owner's play test: reels, band, coins and the gamble (2026-10-18)

The owner played the build and reported: the reels clipping down as each landed, the feature
prompt half off the dark band, GAMES MENU and GAME RULES clipping, coins looking off, a silent and
off-looking gamble, and an empty gamble history. Five analysts diagnosed these against the footage
and the code, each re-checked by a verifier, and the fixes follow their verified numbers:

- **Reels.** The landing copied from the mobile app dipped a whole symbol and jumped back. The
  cabinet (60 fps footage) runs through its stop to about 0.65, is at 0.40 the next frame and
  slides home in 8 frames, at 20 symbols a second, reels 0.45 s apart, crisp. The app now does that
  (measured on a muted capture), and symbols outside the window no longer vanish as a reel stops.
- **Band.** The status line and the feature prompts sit on the backdrop above the band, which
  starts higher; the tabs are capsules whose gloss stays inside the rim; the GAMES MENU's splash
  runs over the band, leaving a CREDIT panel.
- **Coins.** Black tiles under every coin; plates at the measured size and no longer stretched
  while spinning; digits and "$" as measured.
- **Gamble.** Laid out from the footage, the base game's backdrop undimmed, upright captions,
  the card turning 0.15 s after a choice, the chosen button filling yellow, redrawn art; five
  history cards from the start. Its sounds were then found on about 20 machines' recordings: a
  win tune and a loss buzz (both VERIFIED, Golden Century among them), everything else silent;
  the app plays our own synthesis of the two (golden-century-sound-cues.md).
- **Testing.** `DRAGONLINK_MUTE=1` silences every run.

### Jackpots as the club machines run them; prepared test runs (2026-10-18)

The owner's next play test asked for the jackpots to behave as on the machines:

- **Mini and Minor** are fixed for each denomination, as Queensland club footage shows them
  (STRONG, claims/family.toml `jackpots.minor-mini-au`): 1,000 and 5,000 credits from 1c to 10c,
  250 and 1,250 at $1, so $10 / $50 at 1c up to $250 / $1,250 at $1. The model carries them as
  fixed prizes by credit size (`FixedPrize`); the core pays them by the denomination in play.
- **Major and Grand** are progressive meters in money (that they climb: STRONG,
  `jackpots.meters-au`). The core pays the meter, in whole credits with the cents held over, and
  restarts it (`Command::SetProgressive`, `GameState::meters`). The app's link opens each meter at
  random between its restart and a mark below its ceiling (Grand $10,000 to $13,000 of $25,000,
  Major $500 to $1,200 of $1,500), climbs it by a cent or two every few seconds with an occasional
  jump, through features too, and holds a won meter at the amount won until the win is counted.
  Updated 2026-10-19: the Grand's $10,000 restart is filmed (VERIFIED on one machine, STRONG for
  Australian Golden Century) and the climb's order of rate too (STRONG); the openings, the exact
  rates, the Major's restart and the ceilings stay synthetic (it was $5,000 to $12,000 of $15,000,
  14-100 times faster).
- **Chance follows the stake.** The national standard requires a jackpot's chance to be directly
  and linearly proportional to the bet (NS 4.17, PUBLISHED_RETURNS.md). A jackpot coin, or a full
  grid's Grand, stands only with chance stake / its prize (a progressive's restart value), so a
  1c bet cannot win a $10,000 Grand at a $5 bet's chance (the return once ran past 8,000% at the
  smallest stake). Updated 2026-10-19: first a set stake per jackpot, now its prize.
- **Prepared runs** (`DRAGONLINK_SCENARIO`): `both` (Hold & Spin then free games, the Grand),
  `jackpots` (a Major, four Minis and two Minors, then free games; no gamble after the Major) and
  `minors` (the same without the Major; the win can be gambled). Golden Century only. Each
  dresses its run's synthetic model only, its jackpots at full chance whatever the bet; tests hold
  each run to what it promises.
- "FREE SPINS COMPLETED" at the end of Hold & Spin stays: it is what the official app and the
  cabinet footage show (VERIFIED).

### Doors, Hold & Spin inside the free games, the jackpot bar and coin plates (2026-10-18)

From the owner's play test of the prepared runs:

- **Doors.** They now open onto the emperor (wild) and the coin too, as the game's help screen
  lists (VERIFIED): the emperor about one reveal in five and the coin one in eight (synthetic,
  near a tally of 60 filmed reveals: emperor 12, coin 7, a sample leaning to big wins). Revealed
  coins each draw a prize (as the mobile build does) and open with it. Coins also land on the free
  reels themselves (public recording y8c8lEdPW8Y, a Queensland club), and count with revealed
  ones. A run of revealed emperors from reel 1 pays the emperor's line (help screen).
- **Hold & Spin inside the free games.** Six or more coins in a free game start Hold & Spin, which
  holds the free games and hands them back when it ends, its win in their total; on the last free
  game FEATURE COMPLETED follows. Presented as the public Queensland club recording y8c8lEdPW8Y
  shows it (STRONG, `free-games.hold-and-spin-inside`): the usual intro, "Free Game N of 6" kept
  on the status line, FREE SPINS COMPLETED and its count, WIN counting on from the free games'
  total, CREDIT held until the feature ends, the next free game with no START FEATURE. The music
  and the ending after the last free game come from a probably US casino recording (HYPOTHESIS
  for the club machine). In simulation about one free-games round in five has one.
- **Jackpot bar.** Each value centred in its box (they sat up to 58 bar pixels left; the Mini's
  $250.00 at $1 ran over its box), set smaller only if too wide.
- **Coin plates.** A coin landing on a Hold & Spin respin now carries its MINI / MINOR / MAJOR
  plate as it arrives; it used to appear only once the respin had settled.
- **Testing.** `DRAGONLINK_SCENARIO=doors` (coins open Hold & Spin by the third free game, the
  emperor in another, no progressive, so the gamble is offered), and `DRAGONLINK_CAPTURE` (with
  `_EVERY` and `_FROM`) saves an unattended run's frames from the renderer for checking by eye.
- **Evidence.** In the base game the wild lands on reels 2 to 5, never reel 1 (the project's
  transcription of y8c8lEdPW8Y; the claims said reels 2 to 4); doors can put it on reel 1 in the
  free games (filmed). The synthetic base strips still carry it on reels 2 to 4 only: noted for
  the next pass.

## Known limits of what exists

- Every rule in the simulation is a generic slot-game rule, written down so it can be checked. None
  has been checked on an Australian machine.
- The synthetic development fixture is not Dragon Link: five lines, 24-stop strips, symbols `A` to
  `H`. The four title configurations are also synthetic: their structure is evidenced but every
  strip, weight and chance is invented and labelled so, and so is every jackpot amount except
  Golden Century's Mini and Minor, which take the club footage's values (STRONG,
  `jackpots.minor-mini-au`).
- The importer handles standard media that is loose or inside ZIP-family archives, and identifies
  nothing. No proprietary container is unpacked. The application cannot display imported assets.
- Golden Century uses the authentic art and sound only when the local pack is present; the other
  titles still draw generated tiles. About 55 of the game's sound clips are not placed yet, and some
  rules are unknown (the win-tune rule between measured sizes, the big-coin sound threshold). The
  build is checked by eye from screen recordings of unattended runs on the Windows host.
- The exact Hold & Spin analysis values every jackpot at its bet multiple, at full chance.
  Golden Century's game pays fixed Mini and Minor prizes and progressive Major and Grand meters,
  each with a chance that follows the stake, so its played return is not the analysed figure; the
  report says so. Simulated with the meters set, it returns about 90% at every bet the decks
  allow, the configurations within about two points of each other in 800,000-spin runs, noise
  included (`gc_synthetic_plays`); the standard allows 0.20 points between bets. Synthetic.
- Replay has never been given an observation of an original game: none has been transcribed.
- The mathematics tooling has never seen a real configuration. Its model will need extending when
  one arrives ([reverse-engineering/math/CONFIGURATION.md](reverse-engineering/math/CONFIGURATION.md)).
- Victorian approvals and most marketing-page findings were read once and are not yet re-checked.

## Priorities

Reset by the owner on 2026-10-14. **P0 is the Golden Century cabinet experience**: launching it must
be recognisable as a reconstruction of Golden Century on a Dragon Link machine — full-screen cabinet
presentation, authentic symbol artwork, reel frame/geometry, logo, jackpot/topper area, real audio,
machine-like timing, and proper Free Games / mystery-reveal / Hold & Spin / jackpot presentation with
clean transitions, all driven from reference. It must not look like a Rust demo, a generic slot, a
debug UI or coloured boxes. The maths stays synthetic and configurable behind the interface, using
the exact known player-visible paytable/rules, never presented as authentic where hidden (D-017).

**Done means:** a side-by-side check against reference footage would make it reasonable to say "this
is recognisably Golden Century on Dragon Link" — not merely "the rules work". Verified by inspecting
rendered output, not unit tests alone.

**Every other title is frozen** until Golden Century meets this standard.

## Golden Century: playable

As of 2026-10-14 Golden Century plays end to end in the app
(`cargo run -p dragonlink -- --game golden-century`), from a normal player
flow with no debug or manual steps:

- denomination (1c to $1, display only), line (5/25/50) and bet (credits per
  line) selection; spin as long as you like;
- reel-spin animation; line and scatter wins with the winning payline lit and
  a win meter; a jackpot bar;
- the free games (scatter trigger, six games, the mystery reveal, retriggers)
  and Hold & Spin (six orbs, three respins resetting, orb values, full grid),
  both playing themselves with machine-like timing and a clean return;
- synthetic maths behind the existing math interface (about 93% RTP), clearly
  labelled, replaceable with authentic values without the architecture
  changing (DECISIONS D-017). Long automated sessions run without panics.

## Next — the Golden Century cabinet rebuild (P0)

Steps 1 to 3 below are largely done (art, sound, base screen, features), and the button deck,
GAMES MENU and gamble were rebuilt from cabinet footage on 2026-10-17. The remaining work: place
the unplaced sounds where recordings show them, check every screen side by side with the
recordings, attract/idle (the top screen's denomination cycle on the chooser), the gamble limits
as market data rather than constants, and bring the exporter into the repository's importer.
Owner to confirm: the club presentation as the default (the owner's photo is casino style, the
owner's recording club).

1. **Runtime asset pack.** Finish the exporter (ETC1 RGB + the second ETC1 block as **alpha**, so
   symbols are cut out; raw RGBA; Ogg audio; fonts) and build the authentic textures/audio into a
   local, git-ignored asset pack via the import pipeline. Clean-room generation only where no
   authentic asset exists.
2. **Authentic base-game screen.** Replace the placeholder reels with the real reel frame, symbol
   artwork, background and logo, laid out from the mobile scene graph (`main.mercury`,
   `5x3_5Paylines_LL.mercury`), at correct proportions. Get this visually close first.
3. **Outward from there:** reel-spin animation and sound, win presentation, the jackpot/topper area,
   Free Games (doors + mystery reveal), Hold & Spin, orb/prize and Mini/Minor/Major/Grand
   presentation, feature transitions, attract/idle — each checked against reference.
4. **Authentic paytable into the config** (GOLDEN_CENTURY_PAYTABLE.md): 5 lines, the real pays,
   scatter and wild rules; hidden probabilities stay synthetic and re-tuned for a sane return.
5. **Verification loop.** Inspect rendered output, not unit tests alone. The headless screenshot
   capture on the Windows host returns black frames; fix it or capture interactively (the owner's
   Mac runs the build). Resolve how the asset pack reaches the machine the build runs on.

Reference checklist to drive and check each screen: idle/base, reels pre-spin, reels spinning,
losing result, line win, larger win, scatter/free-game trigger, Free Games intro, Free Games play,
mystery reveal, Hold & Spin trigger, Hold & Spin play, orb/prize presentation, Mini/Minor/Major/Grand
award, feature exit to base.

## Not now (owner's direction, 2026-10-14)

Hidden-probability research (exact RTP, reel weights, protected tables,
active MathVariant, cabinet statistical reconstruction) is tabled. Hidden
values stay synthetic and configurable. The existing cabinet-footage readers
may finish in the background; only high-value player-visible findings are
integrated opportunistically.


## Waiting on the owner

One thing, which stops no other work:

- how the mobile game's table contents might be supplied without attacking their protection: its
  algorithms are described, its numbers are not (BLOCKERS B-005). A separate specialist pass on
  that gap is under way outside this project's tooling.

Decided on 2026-10-13 (DECISIONS D-017): reconstructed strips and prize frequencies stay private;
cabinet and mobile are separate configurations.
