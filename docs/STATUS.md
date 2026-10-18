# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-18.**

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
  sound bank (it also plays under the cabinet's big counts).

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
  random between its restart and 80% of a ceiling (Grand $5,000 to $12,000 of $15,000, Major $500
  to $1,200 of $1,500, the owner's ranges), climbs it by a few cents at irregular moments with an
  occasional jump, through features too, and holds a won meter at the amount won until the win is
  counted. Start values, rates, restarts and ceilings are synthetic: no source shows them.
- **Chance follows the stake.** The national standard requires a jackpot's chance to be directly
  and linearly proportional to the bet (NS 4.17, PUBLISHED_RETURNS.md). Each jackpot now has its
  full chance from a stake up (synthetic: 50 credits for the Mini and Minor, $5 for the Major,
  $25 for the Grand); below it a jackpot coin, or a full grid's Grand, stands only in proportion
  to the stake. Without this a 1c bet could win a $5,000 Grand at the same chance as a $5 bet, and
  the synthetic return ran past 8,000% at the smallest stake.
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
  lists (VERIFIED), each about one reveal in eight (synthetic). Revealed coins each draw a prize
  (as the mobile build does) and open with it.
- **Hold & Spin inside the free games.** Six or more coins in a free game start Hold & Spin, which
  holds the free games and hands them back when it ends, its win in their total; on the last free
  game FEATURE COMPLETED follows. Presented as the owner's cabinet session shows it (STRONG,
  `free-games.hold-and-spin-inside`): the usual intro, "Free Game N of 6" kept on the status
  line, FREE SPINS COMPLETED and its count, WIN counting on from the free games' total, the next
  free game with no START FEATURE. In simulation about one free-games round in five has one.
- **Jackpot bar.** Each value centred in its box (they sat up to 58 bar pixels left; the Mini's
  $250.00 at $1 ran over its box), set smaller only if too wide.
- **Coin plates.** A coin landing on a Hold & Spin respin now carries its MINI / MINOR / MAJOR
  plate as it arrives; it used to appear only once the respin had settled.
- **Testing.** `DRAGONLINK_SCENARIO=doors` (coins open Hold & Spin by the third free game, the
  emperor in another, no progressive, so the gamble is offered), and `DRAGONLINK_CAPTURE` (with
  `_EVERY` and `_FROM`) saves an unattended run's frames from the renderer for checking by eye.
- **Evidence.** The wild lands on reels 2 to 5, never reel 1 (the owner's session transcription;
  the claims said reels 2 to 4). The synthetic base strips still carry it on reels 2 to 4 only,
  and the free reels carry no coins though the session shows some: both noted for the next pass.

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
  report says so. Simulated with the meters set, it runs from about 74% to 100% across stakes and
  denominations (`gc_synthetic_plays`), where the standard allows a bet to move the return by 0.20
  points at most: synthetic, and not tuned to that.
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
