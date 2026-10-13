# Prior art

What this project took from elsewhere, what it only learned from, and what it left alone.
Reviewed 2026-10-13.

## ASAMU-decomp (sibling project, same owner)

<https://github.com/bailo167/ASAMU-decomp> — MIT OR Apache-2.0, the same licence as this project.
Read at commit `512e6d3`.

| System | What was done | Notes |
|---|---|---|
| Progress tracker (`progress/progress.toml`, six states, `evidence` required for implemented/verified, `blocker` required for blocked) | **Carried over.** | Same schema, so the two trackers read alike. |
| `tools/progress-gen` (validation, SVG matrix, README table, `--check`) | **Carried over unchanged**, with an origin note. | Generic already. |
| `tools/repo-hygiene` | **Adapted.** | Same design (extension, magic bytes, size, private paths, decompiler fingerprints, one showcase folder). Lists replaced for this project's material; added content checks for ZIP/DEX/ELF/Unity/SquashFS/audio/fonts, source-root directory names, and credential shapes. |
| Workspace layout, pinned toolchain, exactly pinned dependencies, `unsafe` forbidden, app not a default member, dev profile with optimised dependencies | **Carried over.** | |
| `CLAUDE.md` as an operating contract with a session checklist | **Adapted.** | Rules rewritten for this project's evidence problem. |
| Confidence labels on every research claim | **Adapted.** | Six labels here instead of four, plus a separate source class, because mobile-versus-cabinet provenance matters. Enforced in code as well as in prose. |
| Importer/runtime separation; converted data is user-local | **Carried over as a principle.** | No code shared; the formats are unrelated. |
| Deterministic headless simulation driven by pure step functions, parity by replaying traces | **Carried over as a principle.** | |
| Tests skip when original data is absent; synthetic fixtures written in test code | **Carried over as a principle.** | |
| CI: hygiene job first, then a three-OS matrix, pinned toolchain | **Adapted.** | A wasm32 job added; the app jobs will follow the app. |
| Release workflow that refuses game data in packages | **Deferred.** | To be adapted when there is something to release. |
| `tools/dev/win_host*` (build on a second Windows machine over SSH) and `desk_runner` | **Not ported.** | Here the development machine is the Windows host; see DECISIONS.md D-006. The gate's one-line-per-step, logs-kept style follows it. |
| `tools/asamu-inventory` (hash, sniff and categorise an install) | **Design reused**, code not. | The planned `dragonlink-inventory` follows the same shape with different signatures. |
| `tools/asamu-trace` (convert, replay, compare, report) | **Concept reused.** | That tool is built around recorded player physics; the trace here is a discrete event list. |
| `tools/ghidra-scripts` (headless import, sanitized summaries rather than decompiled bodies) | **Deferred.** | Relevant once there is a native binary to analyse; the "export summaries, not bodies" rule is adopted now. |
| Everything about Unreal Engine 3, Kismet, the game's levels and physics | **Not relevant.** | |

## rea — "Reverse Engineer Anything"

<https://github.com/morluto/rea> — MIT (Copyright (c) 2026 morluto). Read at commit `ded199a`.

**What it is.** A local MCP server and CLI in TypeScript that gives AI agents one interface over
existing reverse-engineering engines (Ghidra headless, JADX, Apktool, Binwalk, Unblob, IDA and
others) and returns evidence records with provenance and stated limitations. It is an orchestrator,
not a parser library.

**Relevant capabilities found by reading the code.** ZIP-family containers (including APK) as
inventories; hand-written ELF, PE resource, .NET metadata parsers; a small magic-byte classifier that
looks at bytes before file names and leaves unknown as unknown; a decompression budget and a safe
output tree for extraction; Ghidra driven through `analyzeHeadless` with a script.

**Absent.** Entropy analysis, a general signature database, IL2CPP and Unity asset bundles, OBB,
native DEX/AXML/resource-table parsers, any texture, audio, font or animation decoding, any
proprietary game container.

**Taken as ideas, reimplemented in Rust, no code copied:**

- classify by content before extension, and keep "unknown" as a first-class answer;
- a decompression budget (a ceiling on expansion ratio and absolute size) and a safe output tree for
  untrusted archives;
- records that carry their own limitations (what a tool could not see);
- content-addressed identity for artifacts (SHA-256), distinguishing a file from its occurrences.

**External tools it points to, to be run directly when needed:** Ghidra headless for native
libraries and cabinet binaries, JADX and Apktool for an Android package's Java layer and resources,
Binwalk and Unblob for disk images and unknown blobs.

**Not adopted.** The code itself (wrong language, tightly coupled); its commercial-tool providers;
runtime instrumentation, which is outside a static first phase.

## universal-modder

<https://github.com/rehan-remade/universal-modder> — MIT (Copyright (c) 2026 Rehan and
universal-modder contributors). Read at commit `5ba8150`.

**What it is.** A toolkit for agents that mod PC games: prompt "skills", a Python CLI, and a
knowledge base of notes. It is not an extraction or reverse-engineering framework.

**Relevant capabilities found by reading the code.** Engine detection by file-name patterns over a
desktop install; PE header, Godot and GameMaker header checks; example parsers for a few specific
game formats; a pre-publish lint for verbatim game files, decompiler fingerprints and secrets.

**Absent.** Android handling of any kind, a generic file identifier, entropy analysis, ELF parsing,
disk images, audio or font decoding.

**Taken as ideas:**

- its "Pattern 4 — reimplement" framing, which ASAMU-decomp already follows and this project follows;
- oracles: a format is understood when a round trip proves it, and behaviour is understood when a
  trace replays;
- scrub rules for publishing reverse-engineering notes, which match this project's hygiene rules;
- obtaining an official Android package from one's own device with the platform's own tools rather
  than from a mirror.

**Not adopted.** Its engine scanner (desktop only), its asset generation and automation commands
(the wrong direction for preservation), and its knowledge base as a source of facts: the notes are
unreviewed, and some draw on leaked material, which this project does not use.

## Attribution

No code from rea or universal-modder is included in this repository. The ideas listed above are
credited here. Code adapted from ASAMU-decomp is noted at the top of each file it appears in.
