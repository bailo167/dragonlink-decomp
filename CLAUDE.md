# CLAUDE.md — operating contract for agents working on dragonlink-decomp

dragonlink-decomp is an **evidence-driven preservation and reimplementation** of Aristocrat's classic
Australian *Dragon Link* games in **Rust + Bevy**. Scope: Golden Century (first, the vertical slice),
then Happy & Prosperous, Panda Magic, Autumn Moon. It is the sibling of ASAMU-decomp and follows the
same engineering approach.

It is **not** a "Dragon Link inspired" slot game. Behaviour that is not evidenced stays unknown.

```
legitimately obtained source material (private source root, outside Git)
        ──► inventory / inspection / static analysis / behavioural observation
        ──► importer (runs on the user's machine)
        ──► normalised local asset set + manifest ──► Rust/Bevy runtime
```

## The five rules that outrank everything else

1. **Preserve evidence and provenance.** Every claim about the original carries a confidence label,
   a source class and a citation (`docs/EVIDENCE.md`).
2. **Never fabricate parity.** No invented reel strips, weights, odds, RTP, RNG behaviour, feature
   frequency or prize tables presented as recovered. Synthetic values are labelled synthetic everywhere.
3. **Keep proprietary material out of Git.** The repository is PUBLIC.
4. **Mobile is evidence for presentation, not for cabinet mathematics.**
5. **Compilation is not verification.** Neither is a plausible implementation or a similar-looking result.

## Session start checklist (in order)

1. `git pull --ff-only`
2. Read this file.
3. Read `docs/STATUS.md` (current state, what was last done, next task) and `docs/BLOCKERS.md`.
4. Read the relevant parts of `docs/reverse-engineering/` and `crates/dragonlink-model/claims/` —
   **use existing evidence instead of restarting discovery.**
5. Inspect `progress/progress.toml`.
6. `git status` — understand any uncommitted work before touching anything.
7. Run the tests: `cargo test --workspace`.
8. Choose the **highest-value unblocked task**. If a task is blocked, record it and take another lane.
9. Work on a focused branch in small verified steps.
10. Update evidence docs and claim files (with labels) as facts are confirmed.
11. Update `progress/progress.toml` honestly, then `cargo run -p progress-gen`.
12. Run the Windows gate: `tools\dev\gate.ps1` (see "The Windows gate").
13. Hygiene: `cargo run -p repo-hygiene -- --staged` after staging; also `git status`, `git diff --cached --stat`.
14. Commit (small, logical, conventional-commit messages), merge to `main` when the gate passes, push.
15. Continue with the next task. Do not stop at a plan.

Commit early. Knowledge that is not committed does not exist.

## Source material hygiene

Never commit, paste or upload:

- Aristocrat or Product Madness binaries of any kind: APK/XAPK/OBB/AAB, `.so`, DEX, executables
- cabinet, CFast, disk, firmware or ROM images, or anything extracted from them
- textures, sprites, fonts, music, sound, voice, video, animation data, decrypted or unpacked bundles
- bulk decompiler output (Ghidra, JADX, Apktool, IL2CPP dumps), large `strings` dumps, verbatim proprietary code
- cryptographic keys, account tokens, credentials, cookies
- gameplay recordings or screenshots of the original
- a local full-game package

Safe to publish: independently written Rust, importers, parsers, schemas, file signatures, hashes,
sizes, counts, file names, structural descriptions, measurements, behavioural descriptions in our own
words, small synthetic fixtures written by us, citations, evidence manifests.

Rules of thumb:

- Source material lives under the private source root, **outside the work tree**
  (`DRAGONLINK_SOURCE_ROOT`, see `docs/SOURCES.md`). Never inside the repository, not even ignored.
- Original evidence is immutable input. Work on copies; never modify, patch or rename originals.
- Tests that need real material read `DRAGONLINK_SOURCE_ROOT` and **skip** when it is unset.
  Public tests use synthetic fixtures written by us, byte by byte, in test code.
- No machine-specific absolute paths in code or docs. No home-directory paths (the hygiene tool refuses them).
- Showcase images: only screenshots of **our runtime in placeholder mode**, only in `docs/images/`.
  Nothing rendered from imported original assets is committed (`docs/LEGAL.md`).

## Environment variables

| Variable | Meaning |
|---|---|
| `DRAGONLINK_SOURCE_ROOT` | The private source root: original material, extractions, captures, local research. Outside Git. |
| `DRAGONLINK_DATA_DIR` | The normalised, imported runtime asset set. Defaults to `imported/` under the source root. |
| `GHIDRA_INSTALL_DIR` | Ghidra installation used by headless scripts. |

## The Windows gate

The Windows host is the authoritative test machine. Nothing meaningful merges to `main` without:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\gate.ps1 -Release
```

It runs fmt, clippy (warnings are errors), all tests, hygiene, the progress check, the app debug
build, a smoke launch, the wasm32 build and the release build, one line per step, logs in
`target\gate\`. Linux, macOS and web CI are a second opinion; they do not replace this gate.

Rules for that machine: install only tooling the project needs and record it in `docs/BUILDING.md`;
never reboot it without the owner's permission; never stop processes you did not start.

## Confidence labels (every substantive claim carries one)

| Label | Meaning |
|---|---|
| **VERIFIED** | Seen directly in a primary source, reproducibly. |
| **STRONG EVIDENCE** | Several independent indicators agree. |
| **PARTIAL** | Supported in part; known to be incomplete. |
| **HYPOTHESIS** | Our own conjecture, stated so it can be tested. |
| **UNVERIFIED LEAD** | Something a source says that nobody has checked. |
| **UNKNOWN** | No usable evidence. |

Source classes: cabinet · regulatory · official mobile · official media · gameplay recording ·
third party · hypothesis · synthetic fixture. A third-party page can never make a claim VERIFIED.
When two sources conflict, document the conflict; do not pick the convenient one.
When evidence disproves an earlier claim, update the doc and say what changed.

## Progress tracking — no fake progress

`progress/progress.toml` is canonical; `progress/progress.svg` and the README table are generated.

| Status | Meaning |
|---|---|
| `not_started` | No meaningful work. |
| `investigating` | Active research or reverse engineering. |
| `partial` | Real progress, materially incomplete. Also the ceiling for anything about the original whose mathematics or behaviour is still unevidenced. |
| `implemented` | Our implementation or tooling exists and passes appropriate tests; agreement with the original not proven. For pure research items: documented with reproducible sources. |
| `verified` | Evidence shows agreement with the original for the tracked scope, reproducibly. |
| `blocked` | A specific external blocker (`blocker = "..."` required, and an entry in `docs/BLOCKERS.md`). |

Never mark a stub implemented. Never mark something verified because it compiles, passes our own
tests, looks right, or seems probable. `implemented`/`verified` need an `evidence` field; the
generator rejects them otherwise. Update the tracker in the same commit as the work.

## Architecture rules

- `dragonlink-core` is deterministic and render-free: no Bevy, no files, no clock, no platform APIs.
  Outcomes come only from an injected `OutcomeRng`. It must compile for `wasm32-unknown-unknown`.
- The renderer reflects simulation state and events. It never decides an outcome or computes an award.
- Claims (`crates/dragonlink-model/claims/`) and runnable math models are separate. A math model is
  recovered from complete evidence or labelled synthetic; never a mixture presented as recovered.
- Title differences are data and configuration, not `if title == …` scattered through the code.
  Abstract after evidence shows commonality, not before.
- The runtime loads the normalised asset set through the manifest. It never rummages through the
  source root.
- Platform-specific code is isolated. Windows, Linux, macOS and Web/WASM are all targets.
- No `unsafe` (the workspace forbids it).

## Code quality for parsers

Hostile-input discipline: checked arithmetic, bounds checks, explicit error types, no blind offsets or
serialized lengths, no `unwrap()` in parser code, no panics on malformed input, decompression budgets
for archives. Fuzz-style tests on truncated and corrupted synthetic data.

## Safety boundary

Local study of lawfully held software and an independent simulator. Never: pirated cabinet software,
leaked credentials, access to operational gaming systems, tampering with real machines, jackpot-network
communication, or anything that helps cheat regulated equipment.

## Hard gates — ask the owner first

MFA, CAPTCHA, account login, purchases, physical hardware, manual gameplay capture, an operating-system
dialog that cannot be automated, material only the owner has, an irreversible or high-risk action, a
reboot of the Windows host. Ask in the `NEED FROM BAILEY / WHY / DO THIS / I AM CONTINUING WITH`
format, and keep working on everything else. Never ask for passwords, tokens or keys in chat.
