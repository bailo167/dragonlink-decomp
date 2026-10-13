# Golden Century — player-visible paytable and rules

**Source class: official mobile.** Read from the Golden Century help/paytable screens in the
mobile build (Mighty Fu Casino 5.6.0, `com.productmadness.fafafagold`), recovered from the game's
own in-app rules textures and read directly. These are **player-visible facts** shown to every
player on the machine's help screens, so they are publishable (CLAUDE.md, "Safe to publish";
docs/DECISIONS.md D-017). The hidden reel strips, weights and probabilities are **not** here and
stay private.

Label: **VERIFIED** (read directly from the game's own rules screens). Marked `© 2019 Aristocrat
Technologies Australia Pty Ltd` on the screen.

## Bet and lines

- **5 paylines**, fixed (the mobile game is `5x3_5Paylines`).
- "All lines are played for a **Line Cost of 5 coins**. Choose your Total Bet. Total Bet is the
  Line Cost multiplied by the Bet Multiplier."
- All wins on the paytable are **shown at Bet Multiplier 1**; a line win scales with the multiplier.
- Wins on different lines are added. Highest win only on each line. All wins begin with the leftmost
  reel and pay left to right on adjacent reels, **except** the scatter and the orb, which pay in any
  position.

### The 5 paylines (rows 0 = top, 1 = middle, 2 = bottom)

| Line | Shape | Rows across reels 1–5 |
|------|-------|-----------------------|
| 1 | middle | 1, 1, 1, 1, 1 |
| 2 | top | 0, 0, 0, 0, 0 |
| 3 | bottom | 2, 2, 2, 2, 2 |
| 4 | V | 0, 1, 2, 1, 0 |
| 5 | inverted V | 2, 1, 0, 1, 2 |

## Line pays (per line, at Bet Multiplier 1)

Pays are for 5 / 4 / 3 / 2 of a kind, left to right from reel 1. Picture symbols pay from 2 of a
kind; royals pay from 3.

| Symbol | 5 | 4 | 3 | 2 |
|--------|----|----|----|----|
| Emperor (acts as the top picture; also the **wild**) | 500 | 100 | 25 | 5 |
| Warriors (terracotta) | 250 | 50 | 10 | 3 |
| Women (two women) | 150 | 30 | 5 | 2 |
| Vase (blue-and-white) | 100 | 25 | 5 | 2 |
| Chest (red treasure) | 100 | 25 | 5 | 2 |
| K | 75 | 20 | 5 | — |
| Q | 50 | 10 | 5 | — |
| J | 50 | 10 | 5 | — |
| 10 | 50 | 10 | 5 | — |
| 9 | 50 | 10 | 5 | — |

### Wild (emperor)

Substitutes for all symbols **except the scatter and the orb**. Appears on **reels 2, 3, 4 and 5
only**.

### Scatter (pennant / flag)

Pays in any position, multiplied by the **Total Bet**:

| Count | 5 | 4 | 3 |
|-------|-----|-----|-----|
| Pays × Total Bet | 100 | 10 | 2 |

Three or more scatters also award the Free Games feature.

## Free Games

- **6 free games** are won with any 3 or more scattered pennants.
- A **doors** symbol appears on all reels during the feature; at the end of the reel spin all doors
  reveal one of the following symbols: emperor, warriors, women, vase, chest, K, Q, J, 10, 9, or orb.
- Highest win only on each line. The feature can retrigger during the feature. Total Bet is the same
  as the game that triggered it.

## Hold & Spin and the orb

- The **orb** (golden sun) is the Hold & Spin prize symbol; it and the scatter always appear with an
  orb prize on them. Orbs pay after the Hold & Spin feature, or after the Grand jackpot is awarded.
- **Orb prizes:** MAJOR, MINOR, MINI, or **250, 200, 100, 50, 20, 15, 10, 5, 4, 3, 2 or 1 × Total
  Bet**. 6 or more orbs win all prizes appearing on the orbs.
- Jackpots: **Grand** (linked), **Major** (linked), **Minor**, **Mini**. The Major is awarded when
  MAJOR appears on an orb at the end of Hold & Spin, when 15 orbs appear with MAJOR on one, or
  randomly after a bought game. Only one orb may contain MAJOR at any time.

## What stays private

The reel strips, symbol weightings, orb-value weightings, land chances, hidden probabilities and the
actual jackpot amounts are **not** player-visible and are not recorded here. The runtime uses
synthetic, clearly-labelled values for those (docs/DECISIONS.md D-017); this paytable replaces the
*invented* pay values previously used for Golden Century with the authentic player-visible ones.
