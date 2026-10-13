# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-14.**

## In one paragraph

The repository has its foundation, a headless game model, a first evidence base and a mathematics
analysis subsystem. **All four titles in scope now play end to end** in the application on synthetic,
clearly-labelled maths — Golden Century, Autumn Moon, Happy & Prosperous and Panda Magic — chosen
from a four-title launcher, and a cross-title soak test keeps them robust. No original Dragon Link
mathematics is reproduced: the configurations are invented behind the math interface and never
presented as authentic (DECISIONS D-017). The first original material is held, the official mobile
app with Golden Century's content, and is being analysed. Public research has established the titles'
Australian approval identities and platform generations and the player-visible feature rules, and has
established that the mathematics is not public. Work is pushed to GitHub and CI is green on Linux,
Windows, macOS and wasm32.

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

### All four titles playable, a launcher and hardening (2026-10-14)

- **Autumn Moon, Happy & Prosperous and Panda Magic now play end to end** on the Golden Century
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

## Known limits of what exists

- Every rule in the simulation is a generic slot-game rule, written down so it can be checked. None
  has been checked on an Australian machine.
- The synthetic development fixture is not Dragon Link: five lines, 24-stop strips, symbols `A` to
  `H`. The four title configurations are also synthetic: their structure is evidenced but every
  strip, weight, chance and jackpot amount is invented and labelled so.
- The importer handles standard media that is loose or inside ZIP-family archives, and identifies
  nothing. No proprietary container is unpacked. The application cannot display imported assets.
- The application draws every title with generated coloured tiles and generic symbol names, no
  themed artwork and no audio. Its timing constants are ours, not measured. In a browser it has been
  checked on one host and one browser only, and the headless screenshot capture is unreliable on the
  current host (black frames), so recent presentation changes are verified by tests and code, not by
  eye.
- Replay has never been given an observation of an original game: none has been transcribed.
- The mathematics tooling has never seen a real configuration. Its model will need extending when
  one arrives ([reverse-engineering/math/CONFIGURATION.md](reverse-engineering/math/CONFIGURATION.md)).
- Victorian approvals and most marketing-page findings were read once and are not yet re-checked.

## Priorities

Set by the owner on 2026-10-13. **P0 is a complete, polished, playable Golden Century.** The
mathematics stays synthetic and configurable behind the existing interface
(`game/features -> math config -> synthetic now, authentic later`), clearly labelled and never
presented as authentic (DECISIONS D-017).

1. **Complete the playable Golden Century reimplementation.** The synthetic config exists and the
   app runs it; what remains is free games in the simulation, and turning the placeholder shell
   into a real game (reel-spin, win presentation, feature sequences, jackpot meters, bet/line/
   denomination controls, machine-like timing) on generated placeholder art.
2. **Fidelity and polish** until it behaves and feels like the original.
3. **Integrate authentic mobile or cabinet values** when recovered, by replacing the config.
4. **Further forensic and mathematics research** (cabinet footage, the mobile hidden-math gap):
   now secondary evidence streams, continued only when they do not slow P0.
5. **The other three titles**, after Golden Century is solid.

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

## Next

The four titles, the launcher and the first hardening and polish are done
(see 2026-10-14 above). What remains, in rough order of value:

1. **Fidelity and polish across all four titles**: themed placeholder symbol
   artwork (shapes we draw, no proprietary asset), audio, win and feature
   transitions beyond the banner, a jackpot-win flourish, and layout polish.
   Visual work is held back by the unreliable screenshot capture on this host;
   it wants an eyeball pass.
2. **Close the documented fidelity gaps**: Panda Magic's Hold & Spin inside the
   free games (a nested-feature change to the core state machine, verifiable by
   deterministic tests), and Autumn Moon's free-games wild line pay.
3. **Integrate authentic values** when recovered, by replacing a config.
4. **The further titles** (Genghis Khan, Golden Gong, Peace & Long Life,
   Spring Festival), once their player-visible structure is evidenced; today
   the evidence for them is thin.

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
