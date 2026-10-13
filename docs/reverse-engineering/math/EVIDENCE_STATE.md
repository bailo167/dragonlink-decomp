# Mathematics: what the evidence says today

Read 2026-10-13. **Short version: none of the original mathematics is known, and the reason is
structural, not a gap in searching.**

## The mathematics is not public

The preliminary-injunction order in *Aristocrat v. Light & Wonder* (D. Nev. 2:24-cv-00382, ECF 125,
20 Sep 2024, pp. 2 and 6–7) records Aristocrat's position and its witnesses' declarations:

- Aristocrat attributes the games' success to "the complex mathematical design of the games, which
  it considers a valuable trade secret";
- the asserted secrets include "the mathematical parameters and associated calculations, including
  reel strip layouts, hit rates, probabilities and return percentages", hit rates and average prizes
  for the Grand and Major jackpots, the Hold & Spin calculations for every combination of
  denomination played, and the model that determines RTP and feature hit rates;
- "while certain game elements such as rules, denominations, and pay tables are visible to players,
  the underlying mathematical algorithms are not publicly available";
- these details "are not disclosed in Aristocrat's patents".

The court grouped the asserted secrets into four categories (p. 7): the method of assigning prizes
to the Hold & Spin symbols, Hold & Spin reels and probabilities, Major and Grand jackpot
information, and base game math. Label: VERIFIED as what the order records.

### What this means for the project

1. **Rules, denominations and pay tables are player-visible** and are legitimate research targets
   from official media, rules screens and recordings.
2. **Reel strips, weights, probabilities and RTP models cannot be obtained from public sources.**
   Only cabinet software or a regulator's mathematics document could establish them, and the
   project holds neither.
3. **They cannot be recovered from watching play either.** A sample of spins is not a distribution
   ([../EVIDENCE.md](../../EVIDENCE.md), rule 8).
4. **If mathematics is ever recovered from lawfully held software, it is not committed to this
   repository by default.** It is treated like any other proprietary material: kept in the private
   source root and loaded locally. See [../DECISIONS.md](../../DECISIONS.md), D-010.

## Open questions

All UNKNOWN, recorded as claims in `claims/family.toml`:

| Question | Claim id |
|---|---|
| Reel strips: symbol order and length per reel, per title and bet configuration | `math.reel-strips` |
| Theoretical RTP per title, denomination and jurisdiction | `math.rtp` |
| The random number generator and its mapping to reel stops | `math.rng` |
| The chance of a prize symbol landing on a Hold & Spin respin | `math.hold-and-spin.orb-chance` |
| The distribution of prize-symbol values and jackpot labels | `math.hold-and-spin.value-distribution` |
| Jackpot odds, increments and resets | `math.jackpot-odds` |

Also not established for any of the four titles: the number of lines or ways, the bet ladder, the
Australian denomination values, and the full pay table.

## Pay-table figures quoted in the complaint

The complaint (¶100, ¶102) quotes figures from a "10-cent pay table" and a "$1 pay table" of
Dragon Link. They are reproduced here as a **lead only**:

- the complaint does not say which title they come from;
- the unit (credits or multiple of the bet) is not stated in the text;
- they are US figures;
- they were read once and the complaint's pay-table screenshots were not viewed.

| Symbol | "10-cent pay table" | "$1 pay table" |
|---|---|---|
| Scatter, 5 / 4 / 3 | 100 / 15 / 2 | 100 / 10 / 2 |
| Top picture, 5 / 4 / 3 / 2 | 150 / 50 / 10 / 2 | 250 / 50 / 10 / 3 |
| Second picture, 5 / 4 / 3 (/ 2) | 100 / 30 / 5 | 150 / 30 / 5 / 2 |
| Two other pictures, 5 / 4 / 3 (/ 2) | 100 / 25 / 5 | 100 / 25 / 5 / 2 |
| One royal, 5 / 4 / 3 | 75 / 20 / 5 | 75 / 20 / 5 |
| Four royals, 5 / 4 / 3 | 50 / 10 / 5 | 50 / 10 / 5 |

Label: UNVERIFIED LEAD for any Australian title. Two things they do establish, at STRONG EVIDENCE
for the US family: pay tables differ between denominations, and some picture symbols pay on two of
a kind. **These figures are not used by the simulation.**

## What the simulation runs on

The synthetic development fixture (`crates/dragonlink-model/src/fixtures.rs`): five paylines,
24-stop reel strips, symbols `A` to `H`, invented pays, chances and values. It is not derived from
anything above, deliberately. Its statistics are its own and are never to be quoted as Dragon
Link's. See [../EVIDENCE.md](../../EVIDENCE.md), "From claims to a runnable game".

## Outcome generation

Nothing is known about the original's random number generator, its seeding, or how a random number
becomes a reel stop. The generators in `dragonlink-core` (`DevRng`, `RecordingRng`, `ScriptedRng`)
are test instruments for our own code.
