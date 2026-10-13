# Features: Hold & Spin, jackpots, free games

What is known about how the features work. Read 2026-10-13. Claims are in
`crates/dragonlink-model/claims/family.toml` and the per-title files.

## Sources

| Key | Document | Class |
|---|---|---|
| **Complaint** | *Aristocrat Technologies, Inc. v. Light & Wonder, Inc.*, D. Nev. 2:24-cv-00382, ECF 1, 26 Feb 2024. <https://storage.courtlistener.com/recap/gov.uscourts.nvd.167218/gov.uscourts.nvd.167218.1.0_1.pdf> | court record (Aristocrat's own description of its game) |
| **Order** | Same case, ECF 125, order granting preliminary injunction, 20 Sep 2024. <https://storage.courtlistener.com/recap/gov.uscourts.nvd.167218/gov.uscourts.nvd.167218.125.0.pdf> | court record (the court's summary of the record) |
| **Pages** | Aristocrat's regional product pages, e.g. <https://www.aristocratgaming.com/us/games/us-dragon-link> and the per-title Asia pages | official media |

**Scope warning.** The complaint describes the **US** Dragon Link family generically. It is the
best public description of the rules, and it is Aristocrat's own, but it is not a description of
the Australian build of any one title. Rules below are VERIFIED *as statements about the US
family*; their application to the Australian titles is PARTIAL until seen on an Australian machine.

## Hold & Spin

| Rule | What the source says | Where | Label |
|---|---|---|---|
| Trigger | Six or more bonus symbols (golden orbs) on a single base-game spin | Complaint ¶29, ¶103; Order p. 2 | VERIFIED (US family) |
| Award | Three free games (the complaint also says "free spins") | Complaint ¶29, ¶103 | VERIFIED (US family) |
| Hold | The triggering orbs are held; any orb landing during the feature is also held | Complaint ¶103; Order p. 2 | VERIFIED (US family) |
| Reset | The count returns to three each time a new orb lands | Complaint ¶103; Order p. 2 | VERIFIED (US family) |
| Orb contents | Each orb shows a prize amount or a bonus/jackpot name. Credits at 1c to 10c, dollars at $1 and $2 | Complaint ¶29, ¶103 | VERIFIED (US family) |
| Progress display | A large golden orb in the upper display shows orbs collected and the number required for the Grand Jackpot | Complaint ¶103 | VERIFIED (US family) |
| Grand | Fifteen orbs win the Grand Jackpot | Complaint ¶103 | VERIFIED (US family) |
| End | The value of each held orb is counted into a win meter under the word "WINNER", a flame connecting each orb to the meter | Complaint ¶103 | VERIFIED (US family) |
| Where it can trigger | Panda Magic: "Hold & Spin in base or Free Games" | Pages (Asia), read once | STRONG EVIDENCE, regional |

The order calls the orb the "SCAT2" symbol and notes that it can appear "scattered among and in
place of the usual symbols" (Order p. 2, footnote 1).

### What our simulation does, and where it departs from evidence

`dragonlink-core` implements trigger count, initial respins, reset on a new orb, hold, a full-grid
award and summing held values, all as parameters. Those match the rules above in shape. What is
**ours and unevidenced**:

- one fixed chance per empty position per respin. The order groups "hold & spin reels and
  probabilities" as a category of its own (Order p. 7), which suggests the feature runs on reels of
  its own rather than a flat chance. PARTIAL, an inference;
- one value table for base game and feature;
- jackpot orbs paid at the end with the other orbs. A patent embodiment describes jackpots paid
  immediately ([PATENTS.md](PATENTS.md)); which the game does is UNKNOWN;
- the order in which outcomes are drawn.

## Jackpots and bonuses

| Claim | Source | Label |
|---|---|---|
| Four levels: Grand Jackpot, Major Jackpot, Minor Bonus, Mini Bonus | Complaint ¶103, ¶106 | VERIFIED (US family) |
| The original Australian approvals label each title "(1 Link + 1 iSAP)" and require a linked progressive jackpot system with the approved "Dragon Link" settings | ACT NI 2018-199, NI 2018-200 | VERIFIED (Australia) |
| The 2024 Australian builds operate with jackpot system 1.YA018 (N-Abler) or 1.YA003; two jackpot setting IDs are excluded | ACT NI 2024-704 | VERIFIED (Australia) |
| The "Grand" variants operate with jackpot system 1.YA038 and "Dragon & Phoenix Link Grand" settings | ACT NI 2026-389 | VERIFIED (Australia) |

**Which levels are progressive is not established for Australia**, and the sources disagree by
region:

- Aristocrat's Asia pages: "GRAND & MAJOR JACKPOT: Two exciting top progressive prizes."
- Its US and EMEA pages: "GRAND JACKPOT: Exciting top progressive prize."
- The complaint names Grand and Major "Jackpot" and Minor and Mini "Bonus".
- The ACT label "1 Link + 1 iSAP" reads as one linked level plus one further progressive level,
  but the instruments define neither term.

HYPOTHESIS: in Australia the Grand is the linked progressive and the Major a standalone
progressive, with Minor and Mini fixed. It fits every source above; none states it.

Start values, increments, contribution rates and odds: UNKNOWN.

## Free games

| Claim | Source | Label |
|---|---|---|
| Three or more scatter symbols trigger the free games feature | Complaint ¶105 | VERIFIED (US family) |
| Golden Century: in the free games "all special symbols will reveal the same symbol, including the chance for the jackpot"; tagged "Free Games with mystery symbols" | Pages (Asia, US), read once | STRONG EVIDENCE, regional |
| Autumn Moon: the same wording as Golden Century | Pages, read once | STRONG EVIDENCE, regional |
| Happy & Prosperous: "Free Games with royal symbols removed" | Pages, read once | STRONG EVIDENCE, regional |
| Panda Magic: "Free Games with royal symbols removed"; "great retriggers" | Pages, read once | STRONG EVIDENCE, regional |

Number of free games awarded, retrigger rules, and whether free games use different reels: UNKNOWN.
Free games are not implemented in the simulation.

## Other presentation

- A "Coin Shower" animation plays on any win of 25 times the bet or more (Complaint ¶104, read
  once). STRONG EVIDENCE (US family).
- "Win up to 250x your bet on a scatter" (Pages, read once): on the Australian pages of all four titles, not only Golden Century and Autumn Moon as first recorded. No screen read from footage shows a scatter paying 250: the US pay table of three titles gives 100 times the total bet for five. 250 is the largest prize-symbol multiple in the US Hold & Spin rules, and in Autumn Moon the wild's top line pay in free games ([FOOTAGE.md](FOOTAGE.md)). What the sentence refers to is unexplained.
