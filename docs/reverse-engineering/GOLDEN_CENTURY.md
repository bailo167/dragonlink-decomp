# Dragon Link — Golden Century

**The current target and first vertical slice.** Read 2026-10-13. Claims: `crates/dragonlink-model/claims/golden-century.toml`.
Family-level rules that are assumed to apply but not yet confirmed on this title's Australian build
are in [FEATURES.md](FEATURES.md).

## Identity and versions (Australia)

ACT specifications ([REGULATORY.md](REGULATORY.md) has instruments, application numbers and how
often each entry was read):

| Generation | ACT specification | First instrument | Platform evidence |
|---|---|---|---|
| Original | 1.HDG11 | 29 Mar 2018 | Gen8; Helix+ (2023 regression entry) |
| 2024 build | 1.HDG96 | 29 Nov 2024 | Gen9 Mars X Slant; Gen10 Baron |
| 2026 build | 1.HD129 | 4 May 2026 | not named |
| Grand | 1.HD157 | 31 Jul 2026 | jackpot system 1.YA038 |

Victoria: 2024: approval 28563, identifier 10169609. 2026: identifier 10170972. (read once).
Standalone sibling in the separate Dragon Cash family: 1.DG032 (2018), 1.DG232 (2026).

Whether the generations differ in mathematics or presentation is UNKNOWN. Every future finding
about this title records which generation it came from.

## What is known about the game

| Topic | Finding | Label |
|---|---|---|
| Window | Five reels, three rows | PARTIAL: stated for the US family, not this build |
| Symbol classes | Five royals, four pictures, scatter, wild (reels 2 to 5), Hold & Spin bonus symbol | PARTIAL: US family |
| Hold & Spin | Six or more orbs trigger; three respins, reset on each new orb; fifteen orbs win the Grand | PARTIAL: US family |
| Free games trigger | Three or more scatters | PARTIAL: US family |
| Free games rule | In the free games "all special symbols will reveal the same symbol, including the chance for the jackpot"; tagged "Free Games with mystery symbols". | STRONG EVIDENCE: Aristocrat's regional pages (Asia, US), read once |
| Other | "Win up to 250x your bet on a scatter." | STRONG EVIDENCE: the same pages |
| Mobile | Named in the Mighty Fu Casino listing and in Lightning Link Casino's App Store listing. | STRONG EVIDENCE: store listings, not checked in-app |

## Unknown

Lines or ways · bet ladder and Australian denominations · symbol names and artwork · the pay table ·
number of free games and retrigger rules · which jackpot levels are progressive and their values ·
reel strips, weights, RTP and every probability ([math/EVIDENCE_STATE.md](math/EVIDENCE_STATE.md)) ·
sound, animation and timing · attract and idle behaviour.

## Evidence that would move this title forward

1. A recording of the rules, pay-table and game-information screens on an Australian machine:
   settles lines, bets, pay table, feature rules and the displayed RTP range for that build.
2. Official mobile software: artwork, symbols, sound and presentation
   ([MOBILE_REFERENCE.md](MOBILE_REFERENCE.md)).
3. Lawfully held cabinet software: everything, mathematics included.

## State in our implementation

Nothing title-specific exists. The simulation runs on the synthetic fixture, which is not
Golden Century in any respect.
