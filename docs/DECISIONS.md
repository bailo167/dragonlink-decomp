# Decisions

Architectural and project decisions, with the reasons. Newest last. A decision that is later
reversed stays here with a note saying what replaced it.

## D-001 — Mirror ASAMU-decomp's structure and conventions (2026-10-13)

**Decision.** Same workspace layout (`apps/`, `crates/`, `tools/`, `docs/`, `progress/`), same
tracker and hygiene tools, same pinned-toolchain and pinned-dependency policy, same licence
(MIT OR Apache-2.0), same "app is not a default member" arrangement.

**Why.** The two projects have one owner and one way of working; an agent or contributor moving
between them should find the same things in the same places. The generic tools were already proven.

**Not carried over.** Anything specific to that game or to Unreal Engine; its remote Windows build
scripts (see D-006).

## D-002 — Separate claims from runnable math models (2026-10-13)

**Decision.** Statements about the original live in a claims database, one statement at a time with
label, source and citation. The simulation runs on a `MathModel` that carries a single provenance
and is either recovered from complete cabinet or regulatory evidence, or labelled synthetic.
Nothing fills a gap in the claims so that a model can run.

**Why.** The failure this project must not have is a plausible game presented as Dragon Link. If
claims fed the simulation directly, every unknown would need a default, and defaults become
invisible. With the split, the engine can be developed on a fixture that cannot be mistaken for the
original (five lines, symbols `A` to `H`), and the claims stay honest.

**Alternative rejected.** One per-title configuration with a "confidence" on every field. It reads
well but makes a half-evidenced game runnable, which is the thing to avoid.

## D-003 — Events are the interface, and symbols are named by key (2026-10-13)

**Decision.** The simulation's output is a list of events. The same event vocabulary describes a
transcribed recording of the original. Symbols appear by key (a string), and fields an observer
cannot see are optional.

**Why.** Parity tooling can then be built before there is anything to compare, and both sides of a
comparison are produced in one format. Numeric symbol ids would tie a trace to one model's numbering.

## D-004 — Outcome sources are test instruments (2026-10-13)

**Decision.** The simulation asks an injected `OutcomeRng` for uniform integers below a bound, in a
documented order. Three implementations exist: seeded (SplitMix64), recording, scripted. None is
described as, or shaped to resemble, the original's generator.

**Why.** Determinism, replay and forced scenarios are needed to test our code. The original's
generator and its mapping to reel stops are unknown; pretending otherwise would be fabrication. If
it is ever established, it becomes another implementation with its provenance stated.

## D-005 — Commands are transactional (2026-10-13)

**Decision.** `Game::apply` works on a copy of the state and commits only on success.

**Why.** A scripted outcome source can run out part-way through a spin, and an amount can overflow.
A half-applied spin would corrupt a replay silently. The cost is one small state clone per command.

## D-006 — The Windows gate is a local script, not a remote-host wrapper (2026-10-13)

**Decision.** `tools/dev/gate.ps1` runs directly on the Windows host. ASAMU-decomp's
`win_host.sh` family (sync a checkout to a second machine over SSH and build there) is not ported.

**Why.** In this project the development machine is the Windows host. A remote wrapper would add a
few thousand lines of scripts and tests with nothing to wrap. If development later moves to another
machine, that tooling can be ported then.

## D-007 — The private source root is outside the work tree (2026-10-13)

**Decision.** Source material lives in a folder named by `DRAGONLINK_SOURCE_ROOT`, outside any Git
repository, not in an ignored folder inside this one. The hygiene tool additionally refuses files
under the source root's directory names anywhere in the tree.

**Why.** An ignored folder is one `git add -f`, one changed ignore rule or one editor plugin away
from a public leak. A folder outside the repository cannot be committed by accident.

## D-008 — No dependency on rea or universal-modder (2026-10-13)

**Decision.** Neither project is a dependency. A few ideas are reimplemented in Rust
([PRIOR_ART.md](PRIOR_ART.md)); external tools they point to (Ghidra, JADX, Apktool, Binwalk) are
run directly when needed.

**Why.** Both are MIT-licensed, so reuse would be permitted, but one is a TypeScript orchestrator
and the other a desktop-modding toolkit of prompts and Python; neither has code for Android asset
containers, entropy analysis or cabinet images, and neither fits a Rust workspace. Their value here
is method.

## D-009 — Archives and binaries are ignored and refused wholesale (2026-10-13)

**Decision.** `.gitignore` and the hygiene tool refuse every archive, executable, image-of-a-disk,
engine bundle, audio and font format, with no general allow-list. The only binary files accepted
are showcase images in `docs/images/`.

**Why.** Nothing this project writes needs to be one of those formats; test fixtures are built in
test code. Erring towards refusal costs nothing and removes a class of accident.

## D-010 — Recovered mathematics stays out of the repository by default (2026-10-13)

**Decision.** A math configuration recovered from proprietary software (reel strips, weights,
probabilities, value tables, RTP models) is treated as proprietary material: it lives in the private
source root and the tools read it from there. The repository holds the tools, the schema, the
method, the synthetic fixture, player-visible facts (rules, pay tables, denominations) with their
sources, and whatever further findings the owner explicitly decides to publish.

**Why.** The court's order in *Aristocrat v. Light & Wonder* records Aristocrat asserting exactly
this material as trade secrets, while noting that rules, denominations and pay tables are visible
to players (`docs/reverse-engineering/math/EVIDENCE_STATE.md`). The project's standing rule is to
err towards not publishing proprietary data. Publishing recovered mathematics would be a legal
decision with real exposure, and it is the owner's to make with advice, not a default.

**Consequence.** `dragonlink-analyze` and the runtime take a configuration by path. Nothing in the
public build depends on a recovered configuration. Reports generated from one are written next to
it, not into the work tree.

**Open for the owner.** Whether derived, aggregate results (an RTP figure, a feature frequency)
from a recovered configuration may be published is undecided. Until decided, they are not.

## D-011 — Mathematics analysis is its own crate, driven by configurations (2026-10-13)

**Decision.** `dragonlink-math` analyses a `GameConfiguration`: a complete math model plus one
provenance record per constant group and a set of sourced behaviour findings. It computes exactly
where it can (frequency sums, convolution, a finite recursion for the feature, full enumeration
when tractable) and simulates, seeded and with intervals, where it cannot. `dragonlink-analyze`
exposes `analyze-game`, `compare-games`, `progressive-ev` and `simulate`.

**Why a separate crate.** The simulation must stay minimal and deterministic; analysis needs
floating point, big enumerations and report formatting. Keeping them apart also gives two
independent routes to every quantity (distributions versus sampling the state machine), and the
tests require them to agree.

**Why a configuration and not the claims.** Same reason as D-002: claims are single statements with
gaps; analysis needs a complete model. The configuration adds per-constant provenance so that a
report can state how well each number is supported, and rejects mixtures of invented and recovered
constants.

**Why exact first.** A simulated RTP has an interval that rare jackpots make wide; the exact figure
has none. On the synthetic fixture the exact return is a single number where three million
simulated spins still leave a 3-point interval.

**The fixture was retuned with this tool** (`synthetic-dev-2`): the first version returned over
600%. It was adjusted to a sane shape for development, towards no real game's figures.

## D-012 — The application presents events; unattended runs use a fixed step (2026-10-13)

**Decision.** The Bevy application sends commands to the simulation, receives the events of the
whole command at once, and reveals them over time. Everything it draws follows the events it has
revealed so far, never the simulation's current state. In an unattended run (`--smoke-frames`) the
presentation advances a fixed sixtieth of a second per frame instead of real time.

**Why.** The simulation is instantaneous by design, so the application could read the final credit
total while the reels are still landing. The first screenshot showed it doing that. Making the
display follow presented events fixes it structurally and keeps the application honest about being
a presenter. The fixed step came from the same screenshot: on a high-refresh display a
frame-counted run covered a fraction of the time expected. With a fixed step, a seed and a frame
count define the picture, which is what a visual comparison will need.

**Timing constants are ours.** Reel-stop and event intervals in the application were chosen so the
sequence can be followed. They were not measured on an original machine and are labelled so.

## D-013 — The mathematics is pursued down a ladder; only one thing can stay unproven (2026-10-13)

**Decision (the owner's).** No lane waits on cabinet software. The mathematics is pursued in this
order, each rung used only as far as the one above it fails:

1. **Cabinet-side evidence from lawful public sources**: litigation exhibits, regulatory material,
   patents, research, and any cabinet software the owner lawfully comes to hold. Exact where found.
2. **The official mobile implementation, analysed locally**: whatever the installed app implements
   on the device (tables, reels, pay logic, assets) is recovered from it directly. It is the
   closest authentic implementation available.
3. **The official mobile implementation, observed**: if outcomes are decided on a server, spins are
   collected automatically in large numbers and the observable model is reconstructed. Reel strips
   are assembled exactly from overlapping stopped windows where the game uses fixed strips; stop
   weighting and feature probabilities are estimated, with intervals.

**What can remain unproven forever:** that the hidden probabilities of the result are, number for
number, those of the Australian cabinet. Only genuine cabinet software or configuration can show
that. Everything else has a path.

**What does not change.**
- A model from rung 2 or 3 is a *mobile* model. Its provenance is `official_mobile` (rung 2) or
  `inferred_statistically` with scope `mobile` (rung 3) on every constant, its reports say so, and
  it is compared with cabinet evidence, never presented as it (EVIDENCE.md rule 4, D-002, D-010).
- Estimates carry their sample size and interval and keep the class `inferred_statistically`
  until replaced by an extracted value. Rare events (the top jackpot) are reported with the width
  their rarity gives them, not with false precision.
- Player-visible facts about the cabinet game (rules, pay tables, bet structure) come from cabinet
  footage and documents, and are used to test a mobile-derived model: where the mobile game's
  visible rules differ from the cabinet's, that difference is a finding.

**Risk noted.** Automated play of the mobile app is very likely against its terms of service and
may cost the account used. That is the owner's decision; an account kept for the purpose is used.
Collection runs on the owner's own machines against the owner's own account and nothing else.

## D-014 — Safety controls on the tooling are a boundary (2026-10-13)

**Decision.** When the safety controls of the AI tooling stop a step, the project does not attempt
the same step by rewording it, by another technical route, or through another agent. It records
what was stopped, continues on the lanes that are unaffected, and leaves the decision about that
step to the owner.

**What this applied to.** Recovering the key for, and decrypting, the mobile package's protected
tables; and exporting the help and pay-table artwork from the game's containers. Both were stopped
on 2026-10-13.

**Effect on D-013.** Rung 2 of the mathematics ladder (the mobile implementation analysed on the
device) is not available through the project's tooling beyond what inventory and structural
comparison give. Rung 3 (observing many automated spins) may meet the same kind of limit and is
not assumed to be available. Rung 1, cabinet-side evidence from lawful public sources, and the
online evidence passes for player-visible facts, are unaffected.

## D-015 — Reel data is pursued from public footage of the cabinet game (2026-10-13)

**Decision.** The route to cabinet-side reel data is rung 1b: read the stopped reel windows frame
by frame from public recordings of machines, per title and per identified build, and assemble the
strips with the project's reconstruction tooling. Each recording set is tied to a market and, where
it can be told, a cabinet generation; sets are never pooled across builds.

**Why.** It needs no software, so it is unaffected by B-005. It yields cabinet data, not mobile
data. The method is described in the court record as something the industry itself did, and the
court found that orb-landing chances follow from what can be observed
(`docs/reverse-engineering/LITIGATION.md`). The tooling exists and is tested on simulated play.

**What it will not give.** The prize-value weighting and the jackpot rates, which the same court
treated as secret and not observable; and anything about the random number generator. Those stay
unknown unless cabinet-side documents surface lawfully.

**Honesty conditions.** A window is recorded only when legible in a frame. Completeness is
estimated, and a strip is not asserted while unseen windows are still expected. A reconstruction
says which footage it rests on and how many stops were observed.
