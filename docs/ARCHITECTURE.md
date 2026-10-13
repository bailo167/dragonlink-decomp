# Architecture

## Shape

```
                 ┌──────────────────────────── private, outside Git ───────────────────────────┐
                 │  source root: official-mobile/ cabinet/ captures/ extracted/ decompiler/ …  │
                 └───────────────┬──────────────────────────────────────────────┬──────────────┘
                                 │ read only                                    │
                  inventory / inspect (tools)                            importer (tool)
                                 │                                              │
                   sanitized facts, claims                     normalised asset set + manifest
                                 │                                              │
┌────────────────────────────────▼───────────────┐              ┌───────────────▼──────────────┐
│ dragonlink-model                               │              │ dragonlink-assets (planned)  │
│  evidence labels · claims · symbols · layouts  │              │  manifest · asset ids ·      │
│  math models (+ provenance) · synthetic fixture│              │  placeholder mode            │
└───────────────┬────────────────────────────────┘              └───────────────┬──────────────┘
                │                                                               │
┌───────────────▼────────────────────────────────┐                              │
│ dragonlink-core                                │                              │
│  commands → events · state machine · pays ·    │                              │
│  Hold & Spin · credits · outcome sources       │                              │
└───────────────┬───────────────────┬────────────┘                              │
                │ events            │ events                                    │
┌───────────────▼──────────┐   ┌────▼───────────────────────────────────────────▼──────────────┐
│ dragonlink-trace (plan.) │   │ apps/dragonlink (planned) — Bevy                              │
│  trace format · compare  │   │  window · reels · UI · audio · VFX · input · platform         │
└──────────────────────────┘   └───────────────────────────────────────────────────────────────┘
```

## Workspace

| Path | State | Role |
|---|---|---|
| `crates/dragonlink-model` | exists | Evidence labels, claims database, symbols, layouts, runnable math models, the synthetic fixture. |
| `crates/dragonlink-core` | exists | Deterministic simulation. Commands in, events out. |
| `crates/dragonlink-math` | exists | Mathematics analysis of a game configuration: exact return and feature analysis, seeded simulation, progressive EV, comparison, reports. |
| `tools/dragonlink-analyze` | exists | `analyze-game`, `compare-games`, `progressive-ev`, `simulate`. |
| `crates/dragonlink-trace` | exists | Trace format shared by rewrite traces and transcribed observations; comparison. |
| `crates/dragonlink-formats` | exists | File identification by content, entropy, listing of ZIP-family archives. |
| `crates/dragonlink-assets` | exists | Asset manifest, logical asset ids, verification, discovery of the asset set with a stated reason when there is none. |
| `apps/dragonlink` | exists | The Bevy executable. Placeholder mode only: generated cells, synthetic fixture. |
| `tools/dragonlink-inventory` | exists | Create and check the private source root; hash and classify everything under a source folder. |
| `tools/dragonlink-inspect` | planned | Look at one unknown file: signature, entropy profile, structure. |
| `tools/dragonlink-import` | exists | Source material → normalised asset set + manifest. Stage 1: loose standard media only. |
| `tools/dragonlink-pack` | exists | Verify an asset set and assemble a local full package outside Git. |
| `tools/dragonlink-trace` | exists | Record, validate and compare traces from the command line. |
| `tools/progress-gen` | exists | Tracker validation and rendering. |
| `tools/repo-hygiene` | exists | Refuses proprietary material, dumps, secrets, private paths. |
| `tools/dev` | exists | Windows gate and host setup scripts. |

The Bevy executable is a workspace member but not a default member, so `cargo test` covers the
headless crates quickly; the gate builds the app explicitly.

## Boundaries, and why

**The simulation is headless and deterministic.** `dragonlink-core` has no Bevy, no file access, no
clock and no platform API. Every outcome comes from an injected `OutcomeRng`. That makes it testable
without a window, replayable from a draw log, runnable for millions of spins, and portable to WASM.
The order in which it requests draws is documented (`game.rs`) because a recording replays against it.

**The renderer does not do mathematics.** The application sends commands and presents the events and
state it gets back. Presentation timing (how long a reel takes to stop) belongs to the application
and is recorded with its own evidence; whether a reel stops on a given symbol does not.

**The screen shows what has been presented, not what the simulation knows.** A command resolves
at once in the simulation; the application then reveals its events over time. Anything on screen
(the credit meter, the window) follows the events as they are shown, so a result is never visible
before the reels that produced it. The first screenshot of the shell caught exactly this mistake.

**Claims and math models are different things.** A claim is one statement about the original with
its support. A math model is a complete, runnable configuration with a provenance. Nothing converts
an incomplete set of claims into a runnable model by guessing the rest. See
[EVIDENCE.md](EVIDENCE.md), "From claims to a runnable game".

**The outcome source is a test instrument.** `DevRng`, `RecordingRng` and `ScriptedRng` exist to
test our code. They model nothing about the original's generator, which is unknown.

**The runtime never reads source material.** An importer writes a normalised asset set with a
manifest; the runtime loads that, or runs in placeholder mode. Proprietary formats are understood by
tools, not by the game.

**Titles differ by data.** Common mechanics live once; a title contributes symbols, layout, pays,
feature parameters and asset mappings. A mechanic becomes shared and data-driven when evidence shows
the titles share it, not in anticipation.

**Parsers treat input as hostile.** Checked arithmetic, bounds checks, explicit errors, no panics on
malformed input, decompression budgets.

## Events

The simulation's output is a list of events (`dragonlink-core/src/event.rs`): bet placed, credits
changed, spin started, each reel stopped with its visible symbols, orb values, line and scatter wins,
feature triggered, each Hold & Spin respin, jackpots awarded, feature and spin completed.

The same vocabulary is meant to describe a recording of the original. Symbols are named by key, and
fields an observer cannot see (a reel's stop index) are optional, so a transcribed observation and a
rewrite trace can be compared event by event. See [PARITY.md](PARITY.md).

## Platforms

Windows, Linux, macOS and Web/WASM are all targets. The Windows host is the authoritative test
machine ([BUILDING.md](BUILDING.md)). Consequences for design: no Windows-only file APIs in shared
code, no assumption of native threads or an unrestricted file system in anything the web build uses,
assets addressed through the manifest rather than by walking directories.
