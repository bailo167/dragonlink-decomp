# Mathematics and analysis

One of the project's end-states, alongside visual and gameplay parity:

> Understand how these specific Dragon Link implementations work mathematically, demonstrate which
> parts are proven, and explain quantitatively, from evidence, how the real Australian games
> behave — rather than relying on player folklore.

This folder is the home of that work. It is a research lane of its own: it does not block the
renderer, importer or runtime, and they do not block it.

**State on 2026-10-13: no original mathematics is held, and none is public**
([EVIDENCE_STATE.md](EVIDENCE_STATE.md)). What exists is the method, the provenance model and the
analysis tooling, exercised on the synthetic fixture so that it is ready when a real source arrives.

## What the lane sets out to determine

For each of the four titles, per version and configuration, as far as evidence permits. Nothing on
this list is assumed to exist; each is a question to be answered.

| Area | Questions |
|---|---|
| **Reel model** | Exact or reconstructed reel strips, virtual reels, symbol weighting. |
| **Pay model** | Pay-table mathematics; denomination-dependent and bet-dependent behaviour; whether selectable configurations change more than prize scaling. |
| **Return** | RTP and its composition (base game, features, jackpots); hit frequency; volatility. |
| **Features** | Trigger probability of Hold & Spin and of free games; the feature's internal mechanics; respin symbol probability and distribution; the distribution of feature outcomes; free-games mathematics. |
| **Jackpots** | Mini, Minor, Major and Grand probabilities; progressive contribution; expected jackpot frequency; whether meters affect expected value; expected value as a function of displayed meters; whether any level can cross break-even. |
| **State** | State transitions; any persistent state; whether previous spins influence later ones; any observable machine state that changes expected value. |
| **Outcome generation** | RNG call sites and usage; whether the outcome is chosen before the reels are presented and how; how reel-stop animation relates to the selected result; whether input timing can affect an outcome. |
| **Versions** | Whether cabinet and program versions use different mathematics; whether two machines showing the same title necessarily run the same mathematics; whether mobile implementations differ materially from cabinet ones. |

The aim is to sort every belief about these games into one of: **myth · mathematically irrelevant
observation · configuration difference · genuine state-dependent behaviour · genuine expected-value
difference · implementation quirk · defect**.

## The ladder

Where a configuration comes from, in order of preference (DECISIONS D-013):

| Rung | Source | Result | Provenance of its constants |
|---|---|---|---|
| 1 | Cabinet-side evidence: litigation exhibits, regulatory material, patents, research, lawfully held cabinet software | exact where found | `extracted`, `regulatory`, `derived` |
| 1b | **The cabinet game observed in public recordings**: reel windows read frame by frame from footage of machines | reel strips assembled exactly from stopped windows where a recording set is large enough; stop weighting tested; orb-landing rates counted. Not the prize weighting or jackpot rates | `observed` for what a frame shows, `inferred_statistically` for rates; scope is the market and build of the footage |
| 2 | The official mobile app, analysed on the device | exact for the mobile implementation | `official_mobile` |
| 3 | The official mobile app, observed over very many automated spins | reel strips assembled exactly from stopped windows where strips are fixed; weights and feature probabilities estimated with intervals | `inferred_statistically`, scope mobile |

Rung 1b is the only one that yields **cabinet-side** reel data without any software. The court
record describes a competitor rebuilding base reel strips from public gameplay video, and a court
finding that orb-landing chances can be found by playing and counting
([../LITIGATION.md](../LITIGATION.md)). Rungs 2 and 3 are not available through this project's
tooling ([../../BLOCKERS.md](../../BLOCKERS.md), B-005).

A rung is used only as far as the one above fails, and a configuration never hides which rung each
constant came from. A model from rung 2 or 3 is a mobile model: the closest authentic
implementation, compared with cabinet evidence wherever cabinet evidence exists.

**The one thing that may never be shown:** that the hidden probabilities are, number for number,
the Australian cabinet's. Only genuine cabinet software or configuration settles that.

## Method

1. **Exact over simulated.** Where a configuration is complete, quantities are computed exactly:
   full enumeration of reel stops for the base game, exact recursion for the feature. Simulation is
   used only where exact analysis is not tractable, always seeded and reproducible, always with a
   stated uncertainty.
2. **Provenance for every constant.** Each number in a recovered configuration says how it was
   obtained ([PROVENANCE.md](PROVENANCE.md)). An estimate is never quietly promoted to a fact.
3. **Captures validate; they do not reveal.** Gameplay recordings are small samples. They are used
   to test a recovered model (does the model predict what was observed?), not to infer the
   underlying distribution. See "Role of gameplay captures" below.
4. **Version identity first.** A configuration names the title, market, platform and program
   version it describes. Results are never pooled across versions.
5. **Unknown stays unknown.** A report prints its unknowns. A quantity that depends on an unknown
   constant is reported as unknown, not computed from a placeholder.

## Tooling

The `dragonlink-math` crate and the `dragonlink-analyze` command.

| Command | What it does | State |
|---|---|---|
| `dragonlink-analyze analyze-game <configuration>` | Reproducible quantitative breakdown: RTP and its composition, hit frequency, volatility, feature frequency and value, jackpot model, per-constant provenance, unknowns. | see `progress/progress.toml`, category "Mathematics and analysis" |
| `dragonlink-analyze compare-games <a> <b>` | Structural and quantitative diff of two configurations or versions. | same |
| `dragonlink-analyze progressive-ev <configuration> <meter values>` | Expected value as a function of displayed progressive meters; break-even meter values. | same |
| `dragonlink-analyze simulate <configuration>` | Seeded Monte Carlo with confidence intervals, to cross-check the exact results and to cover what cannot be enumerated. | same |

A **configuration** is a machine-readable file: a complete math model plus the provenance of every
constant in it ([CONFIGURATION.md](CONFIGURATION.md)). The only configuration that exists is the
synthetic fixture, and every report produced from it says so on its first line.

## Where recovered configurations live

**Not in this repository, by default.** Aristocrat asserts the mathematics as trade secrets
([EVIDENCE_STATE.md](EVIDENCE_STATE.md)). A configuration recovered from proprietary software is
proprietary material like any asset: it lives in the private source root under `math/`, and the
tools read it from there. The repository holds the tools, the schema, the method, the synthetic
fixture, and whatever findings the owner decides are publishable. See
[../../DECISIONS.md](../../DECISIONS.md), D-010.

## Role of gameplay captures

A capture can:

- confirm player-visible facts exactly (rules, pay tables, the displayed RTP range, meter values);
- validate a recovered model: observed frequencies should fall inside the model's predicted range,
  and a result far outside it shows the model, or the version identification, is wrong;
- bound a quantity loosely, with the sample size and interval stated.

A capture cannot establish a probability, an RTP or a reel strip. Hold & Spin and jackpot events
are rare; the Grand is rare enough that no realistic amount of footage constrains it.

## The report

For each recovered game, version and platform the tooling produces one technical report
([REPORT_TEMPLATE.md](REPORT_TEMPLATE.md)): configuration identity, evidence confidence, reel
model, pay table, RTP, hit frequency, feature frequency and value, jackpot model, progressive
contribution, overall expected value, denomination and bet effects, state dependence, RNG
architecture, outcome-selection timing, known version differences, unknowns, provenance and
validation results.

## Defects

If analysis of lawfully held software finds an implementation defect, something that departs from
the game's intended mathematics:

- it is recorded with enough evidence to understand and reproduce it locally, in the private
  source root;
- it is kept clearly apart from the intended mathematics in every report;
- it is not published as a method for use against operating machines. Whether and how to disclose
  it (to the manufacturer or a regulator) is the owner's decision.

This project studies software it lawfully holds. It does not test, probe or interact with
operating gaming machines ([../../LEGAL.md](../../LEGAL.md)).

## Documents

| Document | Content |
|---|---|
| [EVIDENCE_STATE.md](EVIDENCE_STATE.md) | What is and is not known today, and why the mathematics is not public. |
| [PROVENANCE.md](PROVENANCE.md) | The provenance classes for numeric constants and their rules. |
| [CONFIGURATION.md](CONFIGURATION.md) | The configuration file format. |
| [METHODS.md](METHODS.md) | How each quantity is computed, with its assumptions. |
| [REPORT_TEMPLATE.md](REPORT_TEMPLATE.md) | The per-game report. |
| [QUESTIONS.md](QUESTIONS.md) | The questions the lane must be able to answer, and what each needs. |
