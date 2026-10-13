# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-13.**

## In one paragraph

The repository has its foundation, a headless game model, a first evidence base and a mathematics
analysis subsystem. Nothing about the original Dragon Link games is reproduced yet: the simulation and analysis run
only on a synthetic fixture. The first original material is held, the official mobile app with
Golden Century's content, and is being analysed. Public research has established the titles' Australian approval identities and
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

## Known limits of what exists

- Every rule in the simulation is a generic slot-game rule, written down so it can be checked. None
  has been checked on an Australian machine.
- The synthetic fixture is not Dragon Link: five lines, 24-stop strips, symbols `A` to `H`.
- No free games, no progressive meters, no weighted stops, no attract or idle states.
- The importer handles standard media that is loose or inside ZIP-family archives, and identifies
  nothing. No proprietary container is unpacked. The application cannot display imported assets.
- The application shows nothing of Golden Century. Its timing constants are ours, not measured.
  In a browser it has been checked on one host and one browser only.
- Replay has never been given an observation of an original game: none has been transcribed.
- The mathematics tooling has never seen a real configuration. Its model will need extending when
  one arrives ([reverse-engineering/math/CONFIGURATION.md](reverse-engineering/math/CONFIGURATION.md)).
- Victorian approvals and most marketing-page findings were read once and are not yet re-checked.

## Next

1. Replay an observation's outcomes through the simulation (scripted outcome source), so rules can
   be compared on the same outcomes once a recording is transcribed.
2. Unpack ZIP-family packages safely (decompression budget, safe output tree), so that a real
   package can be imported once one is held.
3. Bundle and run the web build in a browser.
4. Check existing third-party footage for rules, pay-table and information screens.
5. Re-read the once-read sources so their claims can be labelled VERIFIED.

## Waiting on the owner

Two decisions, neither of which stops other work:

- whether and how the mobile package's protected content is read, since the project's tooling
  does not do it (BLOCKERS B-005);
- whether results from a recovered configuration may be published (DECISIONS D-010).
