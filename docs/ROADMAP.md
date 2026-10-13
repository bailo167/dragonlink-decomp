# Roadmap

Golden Century is the first complete vertical slice. The systems proven on it are then generalised
to Happy & Prosperous, Panda Magic and Autumn Moon. Stages overlap: reusable tooling and the engine
do not wait for every research question to be answered.

Item-level state is in `progress/progress.toml`. Milestone ids match it.

| Milestone | Stage | Goal | Done when |
|---|---|---|---|
| **M0** | A — Foundation | The repository can be worked in safely. | It builds; the Windows environment is reproducible; the progress system, hygiene, docs, CI and the source root exist and work. |
| **M1** | B — Evidence | Know what can be known from lawful sources. | Official and regulatory sources are identified and catalogued; official mobile software is inventoried where lawfully available; files are hashed; package inventories exist; relevant formats are identified. |
| **M2** | C — Import | Source material becomes a usable local asset set. | A source scanner, a manifest, extraction of images, audio and resources where possible, a normalised layout, and a repeatable importer. |
| **M3** | D — Headless model | The game plays without a window. | Game state, spin requests, reel outcomes, award calculation, the feature state machine and deterministic traces exist, with unknown mathematics explicit. |
| **M4** | E — Bevy slice | Golden Century runs on screen. | The title boots; UI and reels render; spin works; imported local assets appear; audio plays; win and feature presentation work as far as evidence supports. |
| **M5** | F — Parity | Agreement is measured, not asserted. | Source-versus-rewrite traces, timing, screenshot, animation and audio-event comparisons exist, and discrepancies are tracked. |
| **M6** | G — Portability | It runs everywhere it should. | Windows verified; Linux and macOS build and test; Web/WASM builds and runs. |
| **M7** | Other titles | The family, not one game. | Happy & Prosperous, Panda Magic and Autumn Moon run on the shared systems with their own evidenced data. |

## Order of work within Golden Century

1. Evidence workflow and source inventory.
2. Importer and asset normalisation.
3. Game-family architecture, deterministic simulation.
4. Reel rendering, input, betting, spin lifecycle.
5. Pay evaluation, feature lifecycle, Hold & Spin, jackpot presentation.
6. Audio, animation.
7. Parity tooling against recordings.
8. Windows validation, web build, local packaging.

## What can change the order

- **A cabinet source appears.** Mathematics becomes recoverable; the cabinet workflow in
  [SOURCES.md](SOURCES.md) takes priority.
- **Official mobile software is obtained.** Presentation research and the importer move ahead.
- **A lane is blocked.** It is recorded in [BLOCKERS.md](BLOCKERS.md) and another lane proceeds.

## Not planned

Real-money play, connection to gaming equipment or jackpot networks, and any title outside the four
until those are done. See [LEGAL.md](LEGAL.md).
