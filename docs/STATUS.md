# Status

The current state of the project, what was last done, and what is next. Updated with every
meaningful increment. Item-level state is in `progress/progress.toml`; this page is the narrative.

**Last updated: 2026-10-13.**

## In one paragraph

The repository has its foundation and a first headless game model. Nothing about the original
Dragon Link games is reproduced: no original mathematics, assets or behaviour measurements are held,
and the simulation runs only on a synthetic fixture. Research into public sources has started.
Nothing has been pushed to GitHub yet because this machine has no working GitHub login
([BLOCKERS.md](BLOCKERS.md)); all work is committed locally.

## Done

### Foundation (2026-10-13)

- Rust workspace, toolchain pinned to 1.98.0, dual MIT/Apache-2.0 licence.
- `tools/repo-hygiene`, adapted from ASAMU-decomp and widened: refuses Android packages, cabinet and
  disk images, archives, engine bundles, audio, fonts, RE databases, key material (by extension and
  by content), the directory names of the private source root, credential shapes, oversized files,
  private paths; warns on decompiler fingerprints.
- `tools/progress-gen`, carried over from ASAMU-decomp; `progress/progress.toml` populated with the
  whole roadmap.
- The Windows gate (`tools/dev/gate.ps1`) and host setup script; CI workflow written.
- Policy documents: legal scope, evidence and provenance, source material, architecture, parity,
  building, decisions, blockers, prior art.

### Headless model (2026-10-13)

- `dragonlink-model`: confidence labels and source classes with their rules enforced in code;
  a claims database (one file for the family, one per title) that currently holds open questions
  only; runnable math models with provenance; the synthetic development fixture.
- `dragonlink-core`: command/event state machine (idle, Hold & Spin), bet and credit accounting,
  reel windows, line and scatter evaluation, a parameterised Hold & Spin respin loop with jackpots,
  transactional commands, save and resume, and three outcome sources (seeded, recording, scripted).
  Tests cover forced feature scenarios, determinism, exact replay from a draw log, and invariants
  over thousands of seeded spins.

## Known limits of what exists

- Every rule in the simulation is a generic slot-game rule, written down so it can be checked. None
  has been checked against the original.
- The synthetic fixture is not Dragon Link: five lines, 24-stop strips, symbols `A` to `H`.
- No free games, no progressive jackpot meters, no attract or idle states.
- CI has never run.

## In progress

- Public-source research for Golden Century and the family (regulatory listings, platform
  generations, published rules, patents, official mobile apps, footage sources).

## Next

1. Private source root: create it on the Windows host, and the tool that creates and checks it.
2. `dragonlink-formats` and `dragonlink-inventory`: identify files by content, hash and inventory a
   source folder, read ZIP-family packages safely.
3. `dragonlink-trace`: the trace format, a rewrite-trace producer and the comparison tool.
4. `dragonlink-assets`, `dragonlink-import`, `dragonlink-pack`: manifest, first importer stage,
   local package validation.
5. The Bevy application shell in placeholder mode, with the smoke launch and wasm build in the gate.
6. Enter the research findings as claims and research documents.
7. Acquire and inventory official mobile reference software, if it can be done without an account.

## Waiting on the owner

See [BLOCKERS.md](BLOCKERS.md). One item: a GitHub login on the Windows host so work can be pushed.
