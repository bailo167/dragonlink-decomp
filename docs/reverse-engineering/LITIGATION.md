# The litigation record

What the public court record says about Dragon Link. Read 2026-10-13, from documents free on
CourtListener's RECAP archive and from the Federal Court of Australia's own site.

**Bottom line.** The public record contains **no number for the mathematics**: no RTP, hit rate,
reel strip, weight, prize-value distribution or jackpot probability. It does contain structural
statements about how the games are built, an adverse party's admissions, the first primary-source
Australian launch date, and a description of reel strips being reconstructed from gameplay video.

## How to read the labels

Every statement names **who says it**: a court's finding, a party's allegation, a sworn statement,
or third-party material attached as an exhibit. These are not interchangeable.

- **VERIFIED**: read in the document and re-checked for wording and page.
- **STRONG EVIDENCE**: read once.
- **UNVERIFIED LEAD**: third-party material that happens to be attached to a filing. An exhibit
  being in a court file does not make its contents reliable.

No page images were viewed in this pass: everything is from the documents' text layers. The
screenshots in the amended complaints were not seen.

## The proceedings

| Proceeding | Court | What was read |
|---|---|---|
| *Aristocrat Technologies, Inc. v. Light & Wonder, Inc.*, No. 2:24-cv-00382 (filed 26 Feb 2024; closed 26 Feb 2026 on settlement) | D. Nev. | The preliminary-injunction order (ECF 125, later vacated on settlement), the second amended complaint and its appendix (ECF 206, 206-1), the first amended complaint (ECF 72), Light & Wonder's answers and counterclaims (ECF 87, 220), its motion to strike (ECF 98), the redacted motion to enforce (ECF 159), discovery stipulations (ECF 217, 232), the dismissal order (ECF 344) and others in part. 92 documents are free on RECAP; the experts' declarations and the trade-secret identification are sealed or not there |
| *Aristocrat Technologies Australia Pty Ltd v Light & Wonder, Inc.* [2024] FCA 439; leave to appeal refused [2024] FCA 870 | Federal Court of Australia | Both judgments |
| *Aristocrat Technologies Australia Pty Ltd v Commissioner of Patents* [2020] FCA 778; [2021] FCAFC 202; (No 3) [2024] FCA 212; [2025] FCAFC 131 | Federal Court of Australia | All four. They concern the feature-game patent, with Lightning Link as the product |

Not opened: [2022] HCA 29 (could not be fetched); the February 2025 reasons refusing an Australian
injunction (reported as subject to a non-publication order).

## Player-visible facts

Scope is the US cabinet family unless stated.

| Fact | Where | Who says it | Label |
|---|---|---|---|
| Five reels, three positions each | ECF 125 pp. 1, 3; ECF 206 ¶ 103; ECF 220 p. 14 | the court, reciting sworn evidence; Aristocrat; **admitted by Light & Wonder** | VERIFIED |
| Denominations 1c, 2c, 5c, 10c, $1, $2, selectable by the player | ECF 206 ¶ 103; ECF 220 p. 14 | Aristocrat; **admitted by Light & Wonder** | VERIFIED |
| Symbol classes: five royals, four pictures, a scatter, a wild (reels 2 to 5 only), a Hold & Spin symbol | ECF 206 ¶¶ 104, 106, 108 | Aristocrat's allegation; not admitted | STRONG EVIDENCE |
| Hold & Spin: six or more orbs on one spin; three spins; orbs held; the count returns to three on a new orb | ECF 125 p. 2; ECF 206 ¶¶ 32, 109 | the court, reciting sworn evidence; Aristocrat | VERIFIED |
| Orbs carry values in the base game as well as in the feature | ECF 125 p. 7; ECF 206 ¶ 31 | the court, reciting sworn evidence; Aristocrat | VERIFIED |
| Fifteen orbs win the Grand; the display reads "15 WIN GRAND JACKPOT" | ECF 206 ¶¶ 33, 109 | Aristocrat | STRONG EVIDENCE |
| Free games on three or more scatters | ECF 125 p. 3; ECF 206 ¶ 111 | the court, reciting sworn evidence; Aristocrat | VERIFIED |
| Hold & Spin "may be triggered in base or free games" | ECF 206-1 Ex. H | Aristocrat's 2017 press material | STRONG EVIDENCE |
| The Grand "is a linked progressive, while standalone jackpots give players a chance to win smaller-sized progressives" | ECF 206-1 Ex. H, Ex. I | Aristocrat's 2017 press material | STRONG EVIDENCE (US, 2017) |
| Four scatters pay 15 in Dragon Link | ECF 125 p. 3 | the court, reciting a sworn comparison made at Dragon Train's Australian launch | STRONG EVIDENCE; the title, denomination and market of the Dragon Link compared are not stated |
| The family is ten titles, including the four in scope | ECF 220 p. 36; [2024] FCA 439, orders | Light & Wonder; the Australian court's orders | VERIFIED |
| Mobile "gameplay features and audiovisual elements … are materially the same" as the cabinets' | ECF 206 ¶ 37 | Aristocrat's allegation; it says nothing about mathematics | STRONG EVIDENCE |

The pay-table figures quoted in the complaint are in [math/EVIDENCE_STATE.md](math/EVIDENCE_STATE.md).

### Leads only

Third-party material attached as exhibits, recorded so it is not mistaken for evidence:

- a casino operator's blog saying Golden Century has 25 lines, or 50 at one cent, that different
  reels load with the denomination, and that the two lower levels are fixed (ECF 206-1 Ex. J);
- forum posts on the largest orb value, which disagree with each other (Ex. M).

## How the games are built

| Statement | Where | Who says it | Label |
|---|---|---|---|
| Orb values "are dynamically assigned on each spin", in base and bonus games, by a method with weightings and probabilities a player cannot discern | ECF 125 p. 7 | the court, reciting Aristocrat's expert (sworn) | VERIFIED |
| Lightning Link embodies a claim in which the base game draws from "a first set of reels" and the feature refills empty positions from "a second set of reels", with prize values assigned to the configurable symbols during play | [2020] FCA 778 [11], [69] | Aristocrat's patent attorney (sworn, unchallenged) and the claim text | VERIFIED for Lightning Link; for Dragon Link an inference |
| A dependent claim has that second set as "individual reels each corresponding to an individual display position" | [2024] FCA 212 [135]; [2025] FCAFC 131 [138] | claim text | VERIFIED as claim text; whether the product does this is UNKNOWN |
| Aristocrat's sealed material has sections on its method for assigning prize amounts to the Hold & Spin symbols, on the implementation of "Hold & Spin Reels", and on the probability of winning the Major | ECF 98 p. 5 | Light & Wonder, quoting Aristocrat's headings | VERIFIED |
| The Hold & Spin hit rate "is a metric calculated from the base game reel strips" | ECF 98 p. 5, fn. 1 | Light & Wonder | VERIFIED as its statement |
| The games use "prize pools" with weighting tables for their distributions | ECF 220 p. 30 | Light & Wonder's allegation | VERIFIED as an allegation |
| Hold & Spin calculations exist "for every combination of denomination played" | ECF 125 p. 6 | the court, reciting sworn evidence | STRONG EVIDENCE |
| The Major has "multiple ways it can be won" | ECF 125 pp. 6, 13 | the court, reciting sworn evidence | VERIFIED |
| Base-game mathematics is reel-strip composition and layout, symbol counts per reel and hit rates; the sheets carry "weight sums" and "reel lengths" | ECF 125 pp. 10, 14, 16 | the court, reciting Aristocrat's interrogatory response and Light & Wonder | VERIFIED |
| The titles of each family "each encompass multiple titles with a similar underlying mathematical design" | ECF 159 p. 13; ECF 206 ¶ 27 | Aristocrat | VERIFIED |
| There are "dozens of iterations of the games", each with a regulatory submission holding "a similar set of math files"; worksheets differ by version and by the amount bet | ECF 217 p. 31, fn. 11; ECF 232 p. 10 | Aristocrat | VERIFIED |
| Dragon Cash "is essentially the same mathematically as the Dragon Link games aside from the hit rate to the grand progressive jackpot" | ECF 220 p. 30 | Light & Wonder, on information and belief | VERIFIED as an allegation |
| The original Dragon Link mathematics file was created on 12 November 2013 | ECF 125 p. 14 | the court | VERIFIED |
| Developed by High Roller Gaming Pty Ltd for Aristocrat | [2024] FCA 439 [2] | the Federal Court | VERIFIED |

Nothing product-specific was found on RTP, on the random number generator, or on outcome selection
against reel presentation.

## Reconstruction from observation, as described in the record

| Statement | Where | Who | Label |
|---|---|---|---|
| Light & Wonder says it rebuilt Lightning Link's base reel strips, RTP and hit rates **from public gameplay video**, in 2014 and 2016 | ECF 220 p. 29 | Light & Wonder's allegation. The court noted the reconstruction itself was never produced (ECF 125 p. 10) | VERIFIED as an allegation |
| The chance of landing an orb can be found by playing and counting, and follows from the number of orb symbols; it was not shown to be a trade secret | ECF 125 p. 9 | **the court's finding**, accepting Light & Wonder's expert | STRONG EVIDENCE |
| The prize weights and the jackpot hit rates were treated as shown to be secret | ECF 125 | the court | STRONG EVIDENCE |

**Why this matters here.** It describes, from inside the industry, the method this project's
reel-reconstruction tooling implements ([math/METHODS.md](math/METHODS.md)): assembling reel
strips from windows seen in recordings. Applied to public footage of Australian machines, it is a
route to cabinet-side reel data that needs no software at all. What it cannot give is what the
court treated as secret and unobservable: the prize weighting and the jackpot rates.

## Bearing on Australia

- **Launch.** Lightning Link and Dragon Link "were launched in 2014 and 2016 respectively"
  ([2024] FCA 439 [2]; repeated in [2024] FCA 870 [3]). VERIFIED. This is the first primary-source
  Australian date and settles the launch-year question for Australia.
- **The feature's own reels.** The sworn evidence about a second set of reels concerns Lightning
  Link as sold in Australia. No Australian judgment names Dragon Link.
- **US against Australian mathematics.** Nothing in the record compares them. Market-specific
  builds are confirmed only in general terms.

## Conflicts recorded

1. **Which levels are progressive.** Aristocrat's 2017 press material (linked Grand, the others
   standalone smaller progressives), the "Jackpot" and "Bonus" naming in the complaint, and a
   third-party blog (two lower levels fixed) do not agree, and none states the Australian
   configuration.
2. **Four-scatter pay.** 15 in one pay table and 10 in another: a denomination difference.
3. **Secrecy of the feature mathematics.** Aristocrat says it cannot be reverse engineered; Light &
   Wonder says it was, in 2017; the court treated prize weights and jackpot rates as secret and the
   orb-landing probabilities as not shown to be.
4. **Title name.** "Happy & Prosperous" and "Prosperous" both appear.

## A lead that is recorded and not followed

Filings by Light & Wonder (ECF 232 p. 7; ECF 257 p. 8; ECF 275) assert that PAR sheets of the
manufacturer, including one for Dragon Link Golden Century, had been posted on a document-sharing
site marked as the manufacturer's trade secret, and that the manufacturer asked for their removal.
These are one party's assertions; no court ruled on them, and the exhibits are not public.

**This project has not looked for those documents and holds none.** A confidential document of the
manufacturer posted by a third party is leaked material, which the project does not obtain
([../LEGAL.md](../LEGAL.md)). The assertion is recorded because it qualifies the statement that
the mathematics "is not public": it may not have stayed secret, which is a different thing from
being lawfully available.

## Still worth obtaining

Public **redacted** versions that exist on the US courts' paid system and are not on the free
archive, chiefly the experts' declarations describing how the games work and what a player can
observe (ECF 53-3, 53-4), Light & Wonder's opposition and its declarations on reconstructing
mathematics from footage (ECF 77 and attachments, 106), and Aristocrat's answer to the
counterclaims (ECF 221). Buying them is a purchase, so it is the owner's to approve. Also the
specification of AU 2016101967 and its parent from the patent office, which may describe how prize
values are assigned in the embodiment.
