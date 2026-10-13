# Open questions, conflicts and leads

The unresolved state of the research in one place. Read 2026-10-13. Machine-readable open
questions are the `unknown` claims in `crates/dragonlink-model/claims/`.

## Conflicts between sources

Recorded as they stand. None is resolved by choosing the convenient reading. Several were narrowed
or settled by the evidence passes of 2026-10-13; what settled them is cited, and the earlier
wording is kept so the change can be seen.

1. **First release.** The court order says Dragon Link launched "in 2016". Aristocrat's complaint
   says "released in the United States in 2017" with trade dress used "in Australia since at least
   2016". A trade article says "2017 debut". An Aristocrat manager said in July 2022 it had been
   "over five years since Dragon Link launched in … ANZ". The earliest regulatory approval found is
   the ACT's of 29 March 2018, and New Zealand's list to July 2017 has none.
   **Settled for the launch year:** the Federal Court states a 2016 launch
   ([LITIGATION.md](LITIGATION.md)), Aristocrat's Australian microsite lists the product for
   Queensland and NSW by November 2016, and Australian machines carry a 2016 notice
   ([AUSTRALIAN_CONFIGURATION.md](AUSTRALIAN_CONFIGURATION.md), [FOOTAGE.md](FOOTAGE.md)). The first
   Australian *approval* is still not public.
2. **Which jackpot levels are progressive.** One on the US and EMEA pages (Grand), two on the Asia
   pages (Grand and Major); "Jackpot" versus "Bonus" naming in the complaint; "1 Link + 1 iSAP" in
   the ACT approvals. **Narrowed to STRONG EVIDENCE for Australia:** the Grand linked, the Major a
   stand-alone progressive, the Minor and Mini fixed. Five indicators agree
   ([AUSTRALIAN_CONFIGURATION.md](AUSTRALIAN_CONFIGURATION.md)), and footage shows the upper two
   meters moving and the lower two fixed per denomination. The Asia pages' wording and the absence
   of the label from later approvals stand against it.
3. **How many titles.** Four launch titles in Europe and Asia; eight on Aristocrat's Australian
   page; ten Dragon Link titles in the ACT register. Not a conflict about the four in scope.
4. **Scatter pay.** The complaint's figures give 100 for five scatters on both tables; Aristocrat's
   pages say "up to 250x your bet on a scatter", on the Australian pages of all four titles.
   **Explained in part:** the complaint's unnamed pay table is the US pay table of Happy &
   Prosperous, Panda Magic and Autumn Moon, read from their rules screens, where five scatters pay
   100 times the total bet. No screen shows a scatter paying 250; 250 is the largest prize-symbol
   multiple in Hold & Spin, and in Autumn Moon the wild's top line pay in free games. What the
   marketing sentence refers to remains unexplained ([FOOTAGE.md](FOOTAGE.md)).
5. **Pay tables differ by denomination** in the complaint's figures. One pay table cannot be
   assumed for all configurations. **Added:** line counts and the $1 values of the Minor and Mini
   differ between US machines, Australian club machines and Australian casino machines.
6. **Terminology.** "Free spins" and "free games" for the Hold & Spin count; "orbs", "coins",
   "SCAT2 symbols", and "pearls" in the patent for the prize symbol.
7. **Version identity.** Three generations of Golden Century plus a "Grand" variant in the ACT;
   "2024" and "2026" builds in Victoria. No source maps one register to the other or says whether
   mathematics changed.

## Looked for and not found

Found since this list was first written (2026-10-13), and no longer open: a US payout range for
two titles ([PUBLISHED_RETURNS.md](PUBLISHED_RETURNS.md)); the Australian line configurations and
denominations ([AUSTRALIAN_CONFIGURATION.md](AUSTRALIAN_CONFIGURATION.md)); the number of free
games awarded, and the symbol sets of three titles from their own rules screens
([FOOTAGE.md](FOOTAGE.md)).

Still not found:

- Any **Australian** return figure or range for Dragon Link, in any source.
- The complete bet ladder, and the payline shapes.
- An Australian rules, pay-table or information screen, in footage or in a filing; and any pay
  table at all for Golden Century.
- A title-level approvals register for NSW, Queensland, South Australia, Tasmania, NT or WA.
- A current New Zealand approved-software list.
- An Aristocrat sell sheet or brochure for Dragon Link.
- An official statement of the mobile apps' engine.
- Official long-form gameplay video of the four titles.

## Leads, in order of value

1. **The machine's own information screens.** The national standard requires them to show RTP or
   its range, the linked-jackpot contribution and bet limits. A recording of those screens on an
   Australian machine is the most direct lawful source for several unknowns. About 650 public
   videos were scanned and none shows one: the button is on screen and is never pressed
   ([FOOTAGE.md](FOOTAGE.md)). A denser scan and other video hosts come before anyone is asked to
   visit a venue ([../BLOCKERS.md](../BLOCKERS.md)).
2. **Line-win messages in existing footage.** The machine prints each line win. Read from a
   session with one line selected, or with the line map known, they give the Australian pay table
   without a rules screen. Not yet worked.
3. **Reel windows in existing footage**, assembled into strips with `dragonlink-analyze
   reconstruct-reels` ([math/METHODS.md](math/METHODS.md)), and then checked by replaying the
   transcribed session through the simulation ([../PARITY.md](../PARITY.md), "Replay").
4. **Further public filings in the court docket**: the free record has been read
   ([LITIGATION.md](LITIGATION.md)); redacted filings available for a fee are listed there and are
   the owner's decision.
5. **Official mobile software**: held and inventoried ([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md));
   its protected content is not read ([../BLOCKERS.md](../BLOCKERS.md), B-005).
6. **Other US regulators' certification lists**, for payout ranges of the other two titles; the
   rest of the patent sweep; forums and marketplaces that refused automated reading.

Done since this list was first written: the archived Aristocrat ANZ title pages were opened and
read ([AUSTRALIAN_CONFIGURATION.md](AUSTRALIAN_CONFIGURATION.md)); existing footage was scanned for
rules screens ([FOOTAGE.md](FOOTAGE.md)).

## Unverified leads that must not be used as facts

- "95.2% RTP" quoted by an affiliate-type page in a search result. Unsourced.
- Golden Century symbol names from third-party sites.

No longer leads, having been established: the mobile app is a Unity application
([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md), from the package itself); the original Australian release
was listed for the Helix family of cabinets
([AUSTRALIAN_CONFIGURATION.md](AUSTRALIAN_CONFIGURATION.md), from the manufacturer's pages).
