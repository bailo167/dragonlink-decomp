# Research knowledge base

Findings about the original games, the platforms they ran on, and the formats their material comes
in. Every substantive claim here carries a confidence label and a source
(see [../EVIDENCE.md](../EVIDENCE.md)). The machine-checked form of the claims is in
`crates/dragonlink-model/claims/`.

**State on 2026-10-13: research has just started.** Documents are added as findings exist. A topic
without a document has no findings yet; it is not hidden work.

## Index

| Topic | Document | State |
|---|---|---|
| Open questions, in one place | `crates/dragonlink-model/claims/family.toml` and the per-title claim files | open questions recorded |
| Cabinet and platform generations, version matrix | to be written | not started |
| Regulatory sources and identifiers | to be written | research under way |
| Official mobile reference software | to be written | not started |
| Golden Century | to be written | research under way |
| Happy & Prosperous · Panda Magic · Autumn Moon | to be written | not started |
| Asset formats (containers, textures, fonts, audio, animation) | to be written | not started; no material held |
| Mathematics (reel data, paytables, outcome generation) | to be written | no source held |
| Features (Hold & Spin, jackpots, free games) | to be written | research under way |
| Game flow (state machines, timing) | to be written | not started |

## How to add a finding

1. Read the source yourself. A search-result summary is not a reading.
2. Write the claim in the relevant claim file with its label, source class, citation and access date.
   `cargo test -p dragonlink-model` checks the rules.
3. Add context here: what the source is, what exactly it shows, what it does not show, and any
   conflict with another source.
4. For local material, cite it by SHA-256 and path relative to the source root. Publish facts about
   it, not the material.
5. Update `progress/progress.toml` if an item's state changed.

## What does not belong here

Original files, extracted assets, decompiler output, large raw dumps, recordings or screenshots of
the original, machine-specific paths. Those stay in the private source root
([../SOURCES.md](../SOURCES.md)).
