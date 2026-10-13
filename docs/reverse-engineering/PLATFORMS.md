# Cabinets, platforms and versions

Which hardware and software generations the four titles are recorded against. Read 2026-10-13.
The regulatory rows are from the ACT instruments in [REGULATORY.md](REGULATORY.md).

## Version and platform matrix

Golden Century shown; the other titles follow the same pattern with their own numbers.

| Generation | ACT specification | First ACT instrument | Platform evidence | Label |
|---|---|---|---|---|
| Original | 1.HDG11 (H&P 1.HDG10, PM 1.HDG12, AM 1.HDG14) | 29 Mar 2018 | Listed against "Gen8 BASE Software (System Software, Linux, OS, BIOS)" 51302007 (2019), 51332002 (2020), 51972000 (2023); regression tested on "Gen 8 Helix+ Upright Casino Top" with Gen8.9 carrier trays (2023) | VERIFIED (regulatory) |
| 2024 build | 1.HDG96 (H&P 1.HDG94, AM 1.HDG97; PM 1.HD049 in 2025) | 29 Nov 2024 | Regression tested on "Gen9 Mars X Slant" with Gen9 BASE Software; then on "Gen10 Baron" / "Gen 10 The Baron Upright" (program identifiers 62002006, 62022000) | VERIFIED (regulatory) |
| 2026 build | 1.HD129 (H&P 1.HD135) | 4 May 2026 | No platform named in the entry read | VERIFIED that it exists; what changed is UNKNOWN |
| "Grand" | 1.HD157 (H&P 1.HD161, PM 1.HD163, AM 1.HD159) | 31 Jul 2026 | Operates with jackpot system 1.YA038 and "Dragon & Phoenix Link Grand" settings | VERIFIED that it exists; how it differs is UNKNOWN |

Victoria lists "2024" and "2026" builds with its own identifiers. How those map to the ACT
specifications is **UNKNOWN**.

## What is established

| Claim | Source | Label |
|---|---|---|
| The Gen8 base software is Linux-based ("System Software, Linux, OS, BIOS"; the Dragon Cash entry names "Linux OS X507 and BL/MBR 0501") | ACT NI 2019-726, NI 2018-199 | VERIFIED |
| Aristocrat's Australian site presents Dragon Link "now on MarsX … with larger jackpots", naming eight games including all four titles | <https://www.aristocratgaming.com/aus/games/aus-dragon-link> (official media, read once) | STRONG EVIDENCE |
| Victoria has approved Aristocrat machine types named Gen9 MarsX Upright, Slant and Portrait, Gen8 Helix Upright Casino Top, Helix XT Portrait, Helix+, The Baron Upright and Gen10 The Baron Portrait | VGCCC report (read once) | STRONG EVIDENCE; machine types only, no link to a game |
| The European debut (ICE, February 2018) was on the "Arc Single" portrait-screen cabinet with the four base games | <https://asgam.com/2017/12/28/aristocrats-european-vacation/> (trade press, read once) | STRONG EVIDENCE, for Europe |
| The 2022 Asian version launched on the MarsX cabinet with the four base games | <https://asgam.com/2022/07/28/chasing-the-dragon/> (trade press, read once) | STRONG EVIDENCE, for Asia |

## What is not established

- **The cabinet of the original Australian release.** The 2018 ACT approvals name no cabinet, and
  Helix+ appears only in a 2023 regression entry. "Dragon Link shipped on Helix in Australia" is an
  inference. UNKNOWN.
- **Whether the generations share mathematics or presentation.** Three generations and a "Grand"
  variant exist for Golden Century alone. Nothing says what changed. Each finding about a cabinet
  build must therefore record which generation it came from.
- **The first Australian approval.** See [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md), conflict 1.
- **Relationship to official mobile software.** UNKNOWN; see [MOBILE_REFERENCE.md](MOBILE_REFERENCE.md).

## Consequences for the project

1. A cabinet source, if one is ever lawfully obtained, must be identified by generation before
   anything is concluded from it. The preservation target is the Australian release; the original
   (2018, Gen8) and the MarsX reissue are different software.
2. The Gen8 platform being Linux-based means a disk image would most likely hold standard Linux
   file systems and ELF binaries. The format-identification tooling is planned accordingly; this is
   a planning assumption, not a finding about any image.
3. The trace and claim formats carry a `scope` so that findings from different markets and
   generations are never merged silently.
