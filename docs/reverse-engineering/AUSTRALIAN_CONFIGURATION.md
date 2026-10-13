# The Australian release: configuration, as the manufacturer published it

What Aristocrat's own archived Australian pages, a regulator's records, venue pages and trade press
say about how the four titles were configured **in Australia**. Read 2026-10-13.

This page answers much of what had been listed as needing a recording of a machine. It describes
the original (Helix-era) Australian product as marketed from 2016 to 2024. **None of it is stated
for the MarsX reissue or the "Grand" builds.**

Labels: VERIFIED means read in the primary source in two independent passes. STRONG EVIDENCE means
read once, or several indicators agreeing without one source stating it.

## Sources

| Key | Source | Class |
|---|---|---|
| **ANZ pages** | Aristocrat's Australian title pages, `aristocrat.com/anz/games/<title>-link/`, in the Internet Archive: three or four captures per title between 2020 and 2023, with identical text. Page metadata: published 23 May 2018 | official media, Australia |
| **Microsite** | Aristocrat's earlier Australian microsite, `info.au.aristocratgaming.com/link`, captures from November 2016 to March 2018 | official media, Australia |
| **Stills** | Images on those pages: poster frames of machines marked "demonstration software only", and product renders | official media, Australia |
| **Deck** | An Aristocrat training deck on jackpots (Queensland, 2022), hosted by a clubs association. It does not name Dragon Link | official media, Australia |
| **Venues** | The Star Gold Coast, The Star Sydney and Treasury Brisbane pages, in the Internet Archive | third party (venue) |
| **Trade** | Inside Asian Gaming, 30 September 2024, quoting Aristocrat's product director for Australia and New Zealand | trade press |

## Configuration, identical for all four titles

From the specification block on each ANZ title page (re-checked on the Golden Century capture of
19 June 2021):

| Region | Line configurations | Cabinets | Denominations |
|---|---|---|---|
| NSW & ACT | 50 lines, 25 lines, 5 lines | Helix X, Helix+, Helix+ Slant, Helix, Helix Slant, Helix Slant Superscreen | 1c, 2c, 5c, 10c, $1, $2 |
| Queensland | 50, 25, 5 lines | Helix+, Helix+ Slant, Helix, Helix Slant, Helix Slant Superscreen | 1c, 2c, 5c, 10c, $1 |
| Victoria | 50, 25, 5 lines | Helix+, Helix | 1c, 2c, 5c, 10c, $1 |
| Northern Territory | as Victoria | as Victoria | as Victoria |

- **Label:** VERIFIED (official media, Australia), for the original release.
- The microsite's family entry agrees: "50L, 25L, 5L", denominations with "$2 (NSW only)", and the
  descriptive ratings "Volatility: medium", "Feature Frequency: Average", "Jackpot RTP: Medium".
- **This settles the launch cabinet** as far as the manufacturer's own listing goes: the Helix
  family. It was UNKNOWN in [PLATFORMS.md](PLATFORMS.md).
- **Which denomination uses which line count is not stated.** UNKNOWN.
- Dragon Link is listed for those four regions only. The stand-alone sibling, Dragon Cash, is
  additionally listed for South Australia and New Zealand.
- Trade: "Dragon Link is a 50-line game in Australia and an 88-line game in Asia".

**A cross-reference worth testing.** The mobile build's Golden Century carries a module named for
five paylines and a "high roller" configuration ([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md)). The
Australian release has a five-line configuration among its three. HYPOTHESIS: the mobile build
corresponds to that five-line configuration. Nothing states it.

## Free games, per title

| Title | What the ANZ page says | Label |
|---|---|---|
| Golden Century | In the free games "all special symbols will reveal the same symbol, including the chance for jackpot symbols, which provides players an additional way to trigger the Hold n Spin feature" | VERIFIED (official media, Australia) |
| Autumn Moon | The same wording | VERIFIED |
| Happy & Prosperous | "royals are removed. An extra 3 free games can be won with any scatter win during the feature" | VERIFIED |
| Panda Magic | "all royals are removed and 3 extra free games can be won with any scatter during the feature" | VERIFIED |

- So Hold & Spin can be entered from free games in Golden Century and Autumn Moon, through revealed
  jackpot symbols.
- **The initial number of free games is not stated officially.** One Australian still of Golden
  Century shows "FREE GAME 4 OF 6". PARTIAL.
- The trigger count on an Australian machine is not stated on these pages. The US family rule is
  three scatters ([FEATURES.md](FEATURES.md)).
- "Win up to 250x your bet on a scatter" appears on the Australian pages of **all four** titles,
  not only the two recorded earlier.

## Jackpots

**Structure.** No single source says this of the Australian titles in so many words; five
indicators agree. Label: STRONG EVIDENCE for Australia. It was a HYPOTHESIS before this pass.

> The Grand is the linked progressive. The Major is a stand-alone progressive on each machine.
> The Minor and Mini are fixed "Bonus" prizes.

1. The ACT names the 2018 games "(1 Link + 1 iSAP)", and the NSW jackpot standard defines an iSAP
   machine as one with an integrated stand-alone progressive.
2. Australian screens label the levels GRAND JACKPOT, MAJOR JACKPOT, MINOR BONUS, MINI BONUS.
3. In a still of a bank of machines, every machine shows the same Grand while the Major differs
   between adjacent machines.
4. The training deck: "GRAND is Linked — MAJOR is SAP — MINOR and MINI are Bonus Prizes", beside a
   render with the same four-level layout.
5. Aristocrat's 2017 US release describes the Grand as a linked progressive with stand-alone
   jackpots below it.

Against it, and recorded: Aristocrat's Asia pages say "GRAND & MAJOR JACKPOT: Two exciting top
progressive prizes", and the label "(1 Link + 1 iSAP)" does not appear on most of the 2024 and
later specifications. Whether the structure changed is UNKNOWN.

**Grand start-up values** are set per venue or link, not per title:

| Value | Where | Label |
|---|---|---|
| $10,000, raised to $50,000 on 25 September 2019 | The Star Gold Coast, main floor | STRONG EVIDENCE (venue statement) |
| $25,000 | The Star Sydney | STRONG EVIDENCE (venue statement) |
| $50,000 from March 2020 | Treasury Brisbane | STRONG EVIDENCE (venue statement) |
| $100,000, with a stated maximum | The Star Gold Coast, high-limit area | STRONG EVIDENCE (venue statement) |
| $60,000 to $90,000 on MarsX, up from $50,000 | Australia | STRONG EVIDENCE (trade press) |

**Minor and Mini against the denomination shown on the same screen**, each from one official
still, read once:

| Denomination shown | Minor | Mini |
|---|---|---|
| 10c | $500 | $100 |
| $1 | cut off, begins "$1,2" | $250 |
| 5c (digit uncertain) | $250 | $50 |

PARTIAL. Whether these scale with the bet as well as the denomination is not shown. The figures on
Aristocrat's product renders are stock marketing figures shared with another product's render and
are **not** used.

**Not published anywhere found:** the Major's start-up and ceiling; increment and contribution
rates; a start-up table by denomination; the contents of the jackpot setting identifiers.

## Identifiers added by this pass

Regulatory, read once (the ACT register was swept for 2016 to 2026):

- Golden Century "Grand" 1.HD174 (NI 2026-418); Happy & Prosperous "Grand" 1.HD173 (NI 2026-417);
  Autumn Moon "Grand" 1.HD169 (NI 2026-416). Each title now has two "Grand" specifications a few
  weeks apart with no stated difference.
- **Autumn Moon 1.HD138** (NI 2026-420, 14 September 2026): a 2026 non-Grand build.
  **This corrects [REGULATORY.md](REGULATORY.md)**, which says none was found.
- The Dragon Cash siblings were first approved in the ACT on 27 July 2017, before Dragon Link.
- No ACT instrument of 2016 or 2017 names Dragon Link: 29 March 2018 is the ACT's earliest.

## First Australian release

A timeline from primary and official sources, which narrows the earlier conflict:

- The microsite does not list Dragon Link on 29 May 2016 and lists it, for Queensland and NSW, by
  13 November 2016.
- The Federal Court states it was launched in 2016 ([LITIGATION.md](LITIGATION.md)).
- The ANZ title pages were published on 23 May 2018, and the ACT approval is of March 2018.

So the product was marketed in Queensland and NSW from late 2016 and reached other jurisdictions
later. No NSW or Queensland approval record is public.

## Still not found for the Australian release

For all four titles:

- any return figure, per variation or denomination;
- the bet ladder, and which denominations run on 50, 25 or 5 lines;
- the pay table: symbol names and pays, the scatter scale behind "up to 250x", wild rules;
- the free-games trigger count and initial award as shown on an Australian machine;
- the Hold & Spin instruction text, and the prize values per denomination;
- the Major's parameters and all increment rates;
- whether the 2024 and later builds keep this configuration.

These are what the footage passes are looking for. Where these searches came up empty, and should
not be repeated, is recorded privately with the research notes.
