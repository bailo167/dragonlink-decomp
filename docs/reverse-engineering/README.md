# Research knowledge base

Findings about the original games, the platforms they ran on, and the formats their material comes
in. Every substantive claim here carries a confidence label and a source
(see [../EVIDENCE.md](../EVIDENCE.md)). The machine-checked form of the claims is in
`crates/dragonlink-model/claims/`.

**State on 2026-10-13.** A first pass over public sources is written up. It establishes the
titles' Australian approval identities and platform generations from regulators' records, and the
player-visible feature rules from the manufacturer's own court filing. It establishes **no
mathematics**: none is public. The first original material is now held: the official mobile app
and the content it downloads for Golden Century ([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md)).

## Index

| Topic | Document | State |
|---|---|---|
| Regulatory approvals, identifiers, the national standard | [REGULATORY.md](REGULATORY.md) | ACT and Victoria catalogued; other jurisdictions not found |
| Cabinets, platforms, version matrix | [PLATFORMS.md](PLATFORMS.md) | three Australian generations established; launch cabinet unknown |
| Hold & Spin, jackpots, free games | [FEATURES.md](FEATURES.md) | rules sourced for the US family; Australian builds unconfirmed |
| Mathematics and analysis (a research lane of its own) | [math/](math/README.md) | no original mathematics held; method and tooling |
| Official mobile reference software | [MOBILE_REFERENCE.md](MOBILE_REFERENCE.md) | apps identified from listings |
| The official mobile package, first inventory | [MOBILE_PACKAGE.md](MOBILE_PACKAGE.md) | Mighty Fu Casino 5.6.0 held, hashed and measured; formats under analysis |
| Patents | [PATENTS.md](PATENTS.md) | core family summarised |
| Reference footage | [REFERENCE_MEDIA.md](REFERENCE_MEDIA.md) | sources listed; nothing watched |
| Conflicts, things not found, leads | [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md) | current |
| Golden Century | [GOLDEN_CENTURY.md](GOLDEN_CENTURY.md) | identity established; game details mostly unknown |
| Happy & Prosperous | [HAPPY_AND_PROSPEROUS.md](HAPPY_AND_PROSPEROUS.md) | identity established |
| Panda Magic | [PANDA_MAGIC.md](PANDA_MAGIC.md) | identity established |
| Autumn Moon | [AUTUMN_MOON.md](AUTUMN_MOON.md) | identity established |
| Asset formats (containers, textures, fonts, audio, animation) | to be written | not started; no material held |
| Game flow (state machines, timing) | to be written | not started; needs recordings |

## How findings are labelled here

- **VERIFIED**: the passage was read in the primary document in two independent passes.
- **STRONG EVIDENCE**: read once in a primary source, or several independent sources agree.
- Tables say "twice" or "once" where it matters.
- A statement about the **US** family is not a statement about an Australian build. Scope is given.

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
the original, recovered mathematics, machine-specific paths. Those stay in the private source root
([../SOURCES.md](../SOURCES.md)).
