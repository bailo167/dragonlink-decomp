# Published return figures and what regulation requires

Read 2026-10-13. This page **corrects an earlier statement in this knowledge base**: that no
regulator had published a return figure. That is true only of Australia, and only for Dragon Link.

Labels: VERIFIED means read in the primary document in two independent passes (the research pass
and a re-check). STRONG EVIDENCE means read once.

## Dragon Link in the United States: a published payout range

Maryland Lottery and Gaming Control Commission, *Software, Hardware, and Game Certifications
3/25/2024 – 4/22/2024*, table "Max Bet over $500":
<https://www.mdgaming.com/wp-content/uploads/2024/04/Commission-Spreadsheet-April-2024-Final.pdf>

| Game as listed | File number | Max bet | ID number | Payout percentage | Casino |
|---|---|---|---|---|---|
| DRAGON LINK GOLDEN CENTURY | MO-73-ARI-20-43 | $2,500.00 | 1.01-66921 | **89.80–91.70** | MGM |
| DRAGON LINK HAPPY AND PROSPEROUS | MO-73-ARI-20-41 | $2,500.00 | 1.01-66919 | **89.80–91.70** | MGM |
| DRAGON LINK GENGHIS KHAN | MO-73-ARI-23-91 | $2,500.00 | 1.01-68688 | **89.80–91.70** | MGM |

- **Label:** VERIFIED as printed. Source class regulatory. **Scope: US cabinet**, a high-limit
  approval.
- The same range for three titles is consistent with one return model across the family. It does
  not show it.
- What the range spans (denominations, variations, with or without the linked level) is **not
  defined** in the list. UNKNOWN.
- Nothing here is an Australian figure. No Australian return figure for Dragon Link has been found.
- Other US identifiers from the same regulator's lists, read once: Panda Magic 1.01-66920 and
  Autumn Moon 1.01-66922 (2020), and a 2025 rebuild 1.01-70184, 1.01-70204 to 1.01-70206.
- Only this one US regulator's lists were mined. Other states publish similar lists and are a lead.

## Lightning Link in Australia: return by denomination and variation

ACT Gambling and Racing Commission, notifiable instrument NI 2015-473 (10 August 2015):
<https://www.legislation.act.gov.au/DownloadFile/ni/2015-473/current/PDF/2015-473.PDF>

For each Lightning Link title the schedule prints:

| Field | Value |
|---|---|
| Machine type | multi-game, multi-denomination, with one stand-alone progressive ("iSAP") level whose wins go to the Total Wins meter, and one linked level |
| Maximum bet | $10.00 |
| Base credit value | 1c, 2c, 5c, 10c, selectable by the player |
| Jackpot system | LPJS 1.YA002 with the approved "Lightning Link" jackpot settings |

and a table of return by configuration. As reported for the full table (the 2c rows re-checked):

| Configuration | Stated CRP % | 1c | 2c | 5c | 10c |
|---|---|---|---|---|---|
| 6 | 90.01 | 89.43 | **89.81** | 90.20 | 90.58 |
| 7 | 90.02 | 89.44 | **89.82** | 90.21 | 90.59 |
| 8 | 90.03 | 89.45 | **89.83** | 90.22 | 90.60 |

- **Label:** VERIFIED for the fields and the 2c column; STRONG EVIDENCE for the other columns and
  for the statement that the table is the same for all ten Lightning Link titles.
- **Scope: Lightning Link, Australian cabinet, 2015.** Not Dragon Link.
- What it shows for Lightning Link: return rises with denomination in steps of about 0.38 points,
  and the three configurations differ by 0.01 point.
- **What the figures include** is an inference from two regulatory documents, STRONG EVIDENCE at
  most: under the NSW protocol they are the return of everything paid through the Total Wins meter,
  which would include the stand-alone level and exclude the linked level.
- The ACT stopped printing these fields after early 2017. None of the instruments naming Dragon
  Link has them.

**Why it matters for Dragon Link.** Dragon Link is filed in the ACT as the continuation of the same
series (Lightning Link specifications 1.HDG00 to 1.HDG09, Dragon Link from 1.HDG10) and first ran
with the same jackpot-system specification under different jackpot settings. The patent embodiment
names the same four denominations. HYPOTHESIS: the original Australian Dragon Link builds offered
the same denominations and a similar return structure. Nothing found states it.

## What regulation requires of any compliant build

These are **constraints**, not observations of the software. A compliant machine must satisfy
them; whether a given build does is a separate question that needs the software. Clause numbers are
from the national standard (revisions 10.3, 2016 and 12.1), the NSW jackpot technical standard and
the Queensland protocol; read once unless marked.

| Question in [math/QUESTIONS.md](math/QUESTIONS.md) | What is required | Clause |
|---|---|---|
| Is the outcome decided before the reels are shown? | The outcome comes from the random number generator and a mapping. Video reels must be shown in the arrangement of the reel strips, without rearrangement | NS 2016 8.5, 4.8; 12.1 8.5, 4.8 (8.5 re-checked) |
| Can button timing affect the outcome? | For builds under the 2016 standard and earlier: the outcome may not be determined until the player's options are complete, and the generator "must be cycled continuously between plays". So the moment of the press selects which values are drawn, unpredictably; it cannot change their distribution. Later revisions drop continuous cycling and require a cryptographically strong generator | NS 10.3 3.14.2, 3.14.6; NS 2016 8.6, 8.9 (re-checked); 12.1 8.6, 8.8–8.10 |
| Do earlier spins influence later ones? | Outcomes must not depend on previous outcomes. Reels must be independent of each other and of their own past | NS 2016 3.3, 4.29; 12.1 3.3, 4.48 |
| Is every stop equally likely? | Each reel position must be equally probable unless the game rules say otherwise | NS 2016 4.29; 12.1 4.48 |
| Persistent state? | Declared metamorphic tokens and progressive pools are the only permitted carried state; cues that imitate one are prohibited | NS 2016 4.25–4.27; 12.1 4.44–4.46, 3.3 |
| Does the bet change more than scale? | Betting options within a variation may move the return by at most 0.20 points. The chance of winning a stand-alone or linked jackpot level "must be directly and linearly proportional to the amount bet" | NS 4.17; NSW Jackpot TS 1.3 cl 2.1.1 |
| How rare may a prize be? | A floor of 1 in 7,000,000 applies to non-linked prizes; linked progressives are ignored for it, so it does not bound the Grand | NS 2016 8.19–8.20; 12.1 8.20–8.21 |
| Volatility | A cap on a normalised standard deviation | NS 2016 8.21; 12.1 8.22–8.23 |
| Do two machines with the same title run the same mathematics? | Not necessarily, by design: a game may carry up to eight variations and up to eight configurations chosen at setup, and the minimum return differs by jurisdiction | NSW Appendix 2.1.3; QCOM 9.1 |
| What must the machine show a player? | The return, games per win, the odds of the five most and five least valuable combinations, and the linked-jackpot contribution | 12.1 4.50–4.52 |
| What is submitted to the tester? | Reel strips in exact sequence, the prize schedule, a return treatise and per-option calculations | NS 2016 / 12.1 9.44–9.49 |

So several questions the mathematics lane asks have a **regulatory answer** for a compliant
Australian machine: the outcome precedes the presentation, button timing cannot change the
distribution, spins are independent, and the same title can run different mathematics on different
machines. Each stays UNVERIFIED for the Dragon Link software itself.

## Patent embodiments that bear on structure

Embodiments, not the shipped game.

- US 11,594,103: one draw per reel mapped through a table of ranges to a stop; feature reels per
  position, or fewer reels dealt without replacement; and the remark that one feature reel "may be
  long … and only contain only one configurable symbol in order to control the odds of the Grand
  Jackpot". An illustrative prize table carries weights without values.
- The same family, on denomination: the denomination changes the value-table weightings, the size
  of the two lower levels and the jackpot contribution.
- No patent read contains a reel strip, weight, hit rate or return of the product.

A sworn statement on the Australian court record ties the product to the patent for Lightning Link
([LITIGATION.md](LITIGATION.md)): base symbols from one set of reels, feature symbols from a second.

## A precedent for what a court record can contain

*Guy v Crown Melbourne Ltd (No 2)* [2018] FCA 36 reproduces the full reel map of an older Aristocrat
game, Dolphin Treasure, from the manufacturer's approval document after confidentiality was waived,
and finds on the manufacturer's evidence that the generator fixes a stop per reel before the
animation shows it and that "the outcome of any given spin is independent of any other spin". An
older game on older hardware: evidence of how the manufacturer built games, not of Dragon Link.
