# Evidence and provenance

The project's value is that what it says about the original games can be trusted, including when it
says "unknown". This document is the policy. The same rules are enforced in code
(`crates/dragonlink-model/src/evidence.rs`, `claims.rs`) and checked by `cargo test`.

## Confidence labels

Every substantive statement about the original carries one.

| Label | Meaning | Typical basis |
|---|---|---|
| **VERIFIED** | Seen directly in a primary source, reproducibly. | Bytes parsed from held software; a figure printed in a regulator's document; a rule shown on the game's own rules screen in a recording we hold. |
| **STRONG EVIDENCE** | Several independent indicators agree. | The manufacturer's product page and a regulatory listing and footage agree. |
| **PARTIAL** | Supported in part; known to be incomplete. | A rule seen for one denomination only. |
| **HYPOTHESIS** | Our own conjecture, stated so it can be tested. | "The four titles share one feature engine." |
| **UNVERIFIED LEAD** | Something a source says that nobody has checked. | An identifier quoted on a forum; a figure on an affiliate page. |
| **UNKNOWN** | No usable evidence. | Reel strips, today. |

A label describes support for a statement about the **original**. It never describes our code.
Our code's state is tracked separately in `progress/progress.toml`.

## Source classes

Recorded separately from confidence, because the two answer different questions.

| Class | What it is | Can support VERIFIED? | Evidence of Australian cabinet mathematics? |
|---|---|---|---|
| **Cabinet** | Software or data from a gaming machine. | yes | yes |
| **Regulatory** | A regulator's document or register. | yes | yes |
| **Official mobile** | An official social-casino implementation. | yes, *about the mobile implementation* | **no** |
| **Official media** | The manufacturer's own pages, brochures, videos, patents. | yes | no |
| **Court record** | A filing or order in a court's public record. | yes, for what the document says; a party's allegation stays that party's statement | no |
| **Gameplay recording** | A recording of the original being played. | yes, for what is visible | no (a sample is not a distribution) |
| **Third party** | Somebody else's observation or write-up. | no | no |
| **Hypothesis** | Our own conjecture. | no | no |
| **Synthetic fixture** | Invented by this project for development. | n/a, it is not about the original | **never** |

## Rules

1. **Unknown means no value.** An open question is recorded as a question. It does not hold a
   "probably" value.
2. **Support needs a source and a citation.** Any label above UNKNOWN names its source class and
   cites a URL or document, with the date it was read. Our own conjecture needs no citation and
   cannot be labelled above HYPOTHESIS.
3. **Third-party pages cannot verify.** Slot-game facts online are heavily polluted by affiliate and
   machine-written pages. They are leads.
4. **Mobile is for presentation.** Official mobile software is valid evidence for artwork, symbols,
   typography, sound, animation, sequence timing, feature presentation, state transitions, UI layout,
   terminology and branding. It is **not** assumed to share reel strips, symbol weights, RTP, jackpot
   probabilities, feature frequency, RNG behaviour or prize tables with the Australian cabinet game.
   A finding from mobile is scoped `mobile` and stays so.
5. **Synthetic is labelled everywhere.** A math model carries its provenance; a trace carries the
   model's provenance in its header; the application shows a banner. Statistics of a synthetic model
   are never quoted as the original's.
6. **Conflicts are documented, not resolved by convenience.** When two sources disagree, both are
   recorded with the disagreement, and the claim's label reflects it.
7. **Corrections are visible.** When evidence overturns a claim, the claim is updated and the change
   noted. Old conclusions are not silently rewritten.
8. **Samples are not distributions.** Probabilities, RTP and frequencies are not inferred from a
   gameplay sample and called verified. A statistical observation is recorded with its sample size
   and stays PARTIAL at most.
9. **VERIFIED means read twice.** A statement taken from a document is VERIFIED only when the
   passage has been read in the primary document in two independent passes, or is reproduced by a
   committed tool. A single reading of a primary source is STRONG EVIDENCE until it is re-checked.
   A search-result summary is not a reading at all.
10. **Versions matter.** A finding is scoped to the market, platform and version it came from
   (`cabinet-au`, `cabinet-us`, `mobile`, …). Machine generations are not assumed to share software.

## Where evidence lives

| What | Where | In Git? |
|---|---|---|
| Claims, one statement each, machine-checked | `crates/dragonlink-model/claims/*.toml` | yes |
| Research notes, context, conflicts, methods | `docs/reverse-engineering/` | yes |
| Hashes, sizes, names, inventories (sanitized) | `docs/reverse-engineering/data/` | yes |
| The material itself, extractions, recordings, decompiler projects | the private source root | **no** |

Local evidence is referred to by content hash (SHA-256) and by its path relative to the source root,
never by a machine-specific absolute path.

## Recording a claim

```toml
[[claim]]
id = "hold-and-spin.trigger"
statement = "Six or more prize symbols in the window start Hold & Spin."
value = "6"
confidence = "strong_evidence"
source = "official_media"
citation = "https://…"
accessed = "2026-10-13"
scope = "cabinet-au"
note = "Agrees with …; not yet seen on a rules screen."
```

A claim file is rejected by the tests if a claim breaks the rules above.

## From claims to a runnable game

The simulation needs a complete math model: every strip, pay and chance. Claims become a model only
when the claims for a title are complete and come from cabinet or regulatory evidence. Until then
the simulation runs on the synthetic fixture, and that is said wherever its output appears.
A model assembled from official mobile software, if that ever becomes possible, is labelled
mobile-derived and is not cabinet parity.
