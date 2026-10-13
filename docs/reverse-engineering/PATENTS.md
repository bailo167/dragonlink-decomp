# Patents

Aristocrat patents covering the hold-and-respin mechanic. Bibliographic data and claim 1 read on
Google Patents on 2026-10-13, once. Source class: official media (patent). A patent describes
embodiments, not a shipped game: nothing here is a statement about Dragon Link's implementation.

The core family is by Scott Christopher Olive, assigned to Aristocrat Technologies Australia Pty
Ltd, priority AU 2014903132 (11 Aug 2014). The mechanic originated with Lightning Link; the patents
name neither game. The worked example is a 3×5 "Hold & Spin" feature using pearl symbols.

| Patent | Title | What claim 1 covers (summary in our words) |
|---|---|---|
| [US 9,928,691 B2](https://patents.google.com/patent/US9928691B2/en) (granted 2018) | System and method for providing a feature game | A feature of a variable number of free games triggered by a predefined count of special symbols; special symbols are held, a free-game counter ends the feature. Claim text read in part. |
| [US 11,756,383 B2](https://patents.google.com/patent/US11756383B2/en) (2023) | System and method for providing a feature game | A player-selected denomination drives selection of the value assigned to a configurable symbol; held symbols, replacement of other positions, award is the sum of prize values. |
| [US 11,302,148 B2](https://patents.google.com/patent/US11302148B2/en) (2022) | Gaming machine and method for providing a feature game | A free-games series with "reveal" symbols replaced by one revealed symbol, which can trigger a second series where configurable symbols are held; the count falls when none lands and resets when one does. |
| [AU 2016101967](https://patents.google.com/patent/AU2016101967A4/en) (innovation patent, expired) | A system and method for providing a feature game | Configurable symbols carrying prize values; a trigger sets a free-game counter; values are accumulated and paid when the feature ends. |
| [US 11,594,103 B2](https://patents.google.com/patent/US11594103B2/en) (2023) | Gaming machine and method with prize chance configurable symbol | A configurable symbol can carry a prize value or a "prize chance" which then randomly awards or does not award. |
| [US 11,551,508 B2](https://patents.google.com/patent/US11551508B2/en), [US 11,625,982 B2](https://patents.google.com/patent/US11625982B2/en) | Hold and spin with iterative awards; with reel expansion | Later variants, not the original mechanic. |

## Embodiment details worth testing against evidence

From the description of US 9,928,691 (an embodiment, read once):

- the feature triggers "when six configurable symbols appear"; "the preferred number of free games
  is three"; the counter resets on each new configurable symbol;
- fifteen symbols on a 5×3 matrix trigger the Grand jackpot;
- "four denominations available for selection, 1c, 2c, 5c and 10c";
- the Grand is a linked progressive, the Major a standalone progressive, Minor and Mini fixed
  amounts proportional to the bet;
- jackpots are paid immediately rather than accumulated.

These agree with the rules Aristocrat states for Dragon Link in its complaint
([FEATURES.md](FEATURES.md)) on trigger, count, reset and Grand. The progressive structure and the
immediate jackpot payment are **leads for Dragon Link, not findings**.

US 11,302,148's "reveal symbols replaced by one revealed symbol" matches the marketing description
of the Golden Century and Autumn Moon free games ("all special symbols will reveal the same
symbol"). HYPOTHESIS: that patent describes those titles' free-games mechanic.

The court order in *Aristocrat v. Light & Wonder* records Aristocrat's expert stating that the
mathematical details "are not disclosed in Aristocrat's patents".
