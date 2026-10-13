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
