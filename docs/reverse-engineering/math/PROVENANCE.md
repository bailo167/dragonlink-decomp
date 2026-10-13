# Provenance of numeric constants

Every constant in a game configuration records how it was obtained. This is separate from the
confidence labels on prose claims ([../../EVIDENCE.md](../../EVIDENCE.md)): a claim says how well a
statement is supported; a provenance class says **by what route a number got into the model**.

Enforced in code: `crates/dragonlink-math/src/config.rs`.

## Classes

Strongest first. A result computed from a configuration is no better supported than its weakest
constant, and every report prints the tally.

| Class | Meaning | Needs a source? |
|---|---|---|
| `extracted` | Read directly out of the game's own software or data. | yes: the evidence file's SHA-256 and where in it |
| `regulatory` | Stated in regulatory material. | yes: the document and page |
| `derived` | Computed from other constants by a stated derivation. | yes: the derivation and its inputs |
| `official_mobile` | Taken from an official mobile implementation. | yes: package, version, SHA-256 |
| `observed` | Read off the game as a player sees it: a rules screen, a pay table, a meter. | yes: the capture's SHA-256 and timestamp |
| `inferred_statistically` | Estimated from a sample of observed play. | yes: the sample, its size and the interval |
| `hypothesised` | Our conjecture. | no |
| `synthetic` | Invented by this project. Only in a synthetic configuration. | no |
| `unknown` | Not known. | never valid for a constant that has a value |

## Rules

1. **Every constant group has exactly one record.** A configuration missing one is rejected.
2. **A valued constant cannot be `unknown`.** If a constant is not known, the configuration is not
   complete and cannot be analysed as if it were. There is no placeholder value.
3. **Synthetic and real do not mix.** A synthetic constant inside a model presented as recovered is
   rejected, and so is a recovered-looking constant inside the synthetic fixture.
4. **A class that claims an origin names it.** `extracted`, `regulatory`, `derived`,
   `official_mobile`, `observed` and `inferred_statistically` require a non-empty source.
5. **Estimates stay estimates.** An `inferred_statistically` constant keeps that class until the
   value is extracted or stated in a document. A larger sample narrows its interval; it does not
   change its class. Captures are used chiefly to *validate* a recovered model, not to build one
   ([README.md](README.md), "Role of gameplay captures").
6. **Mobile is mobile.** An `official_mobile` constant describes the mobile implementation. A
   configuration built from it is a mobile configuration and is compared with a cabinet one, never
   merged into it.
7. **Replace, do not keep.** When better evidence arrives (a binary, a configuration file), the
   affected constants are re-extracted and their records replaced. Earlier estimates are not left
   in place beside them.

## Constant groups

One record each (`required_paths` in the code):

`grid` · `symbols` · `reel_strips` · `paylines` · `line_pays` · `scatter_pays` · `bet` ·
`hold_and_spin.trigger_count` · `hold_and_spin.initial_respins` ·
`hold_and_spin.reset_respins_on_new_orb` · `hold_and_spin.orb_land_chance` ·
`hold_and_spin.orb_values` · `hold_and_spin.full_grid_award` · `hold_and_spin.jackpots`

Groups are coarse today (all reel strips share one record). They will be split (per reel, per pay)
when a real configuration needs constants of mixed provenance within a group.

## Behaviour findings

Things that are not numbers in the model (RNG architecture, when the outcome is selected, how reel
stops relate to it, whether input timing or earlier spins matter, any persistent state) are
recorded in the configuration's `behaviour` section as sourced findings with the ordinary
confidence labels. Each starts UNKNOWN and is printed as unknown in every report until there is
evidence.
