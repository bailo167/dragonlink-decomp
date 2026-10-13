# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-13.**

## In one paragraph

The repository has its foundation, a headless game model, a first evidence base and a mathematics
analysis subsystem. Nothing about the original Dragon Link games is reproduced: no original
mathematics, assets or behaviour measurements are held, and the simulation and analysis run only on
a synthetic fixture. Public research has established the titles' Australian approval identities and
platform generations and the player-visible feature rules, and has established that the mathematics
is not public. Work is pushed to GitHub and CI is green on Linux, Windows, macOS and wasm32. A placeholder application
shell runs the simulation on screen.

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

## Known limits of what exists

- Every rule in the simulation is a generic slot-game rule, written down so it can be checked. None
  has been checked on an Australian machine.
- The synthetic fixture is not Dragon Link: five lines, 24-stop strips, symbols `A` to `H`.
- No free games, no progressive meters, no weighted stops, no attract or idle states.
- The application shows nothing of Golden Century. Its timing constants are ours, not measured.
  It has not been run in a browser.
- The mathematics tooling has never seen a real configuration. Its model will need extending when
  one arrives ([reverse-engineering/math/CONFIGURATION.md](reverse-engineering/math/CONFIGURATION.md)).
- Victorian approvals and most marketing-page findings were read once and are not yet re-checked.

## Next

1. Replay an observation's outcomes through the simulation (scripted outcome source), so rules can
   be compared on the same outcomes once a recording is transcribed.
2. `dragonlink-assets`, `dragonlink-import`, `dragonlink-pack`: manifest, first importer stage,
   local package validation.
3. Bundle and run the web build in a browser.
4. Check existing third-party footage for rules, pay-table and information screens.
5. Re-read the once-read sources so their claims can be labelled VERIFIED.

## Waiting on the owner

See [BLOCKERS.md](BLOCKERS.md). Nothing stops the work above. Three things would unlock more:
official mobile software installed from the store (B-003), recordings of an Australian machine's
rules and information screens (B-004), and any lawfully held cabinet software (B-002). One decision
is open: whether results from a recovered configuration may be published (DECISIONS D-010).
