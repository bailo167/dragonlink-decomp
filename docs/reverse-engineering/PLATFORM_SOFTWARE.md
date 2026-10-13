# Platform software: what is public

What public regulatory lists, open-source emulator documentation and patents say about the software
platform under the cabinet games. Read 2026-10-13, once unless marked; so STRONG EVIDENCE at most
unless stated. It complements [PLATFORMS.md](PLATFORMS.md), which covers which titles ran on which
generation.

Sources: ACT notifiable instruments (about 940 approval instruments from 2009 to 2026 were read in
bulk); a US state regulator's certification lists (Maryland, 2018 to 2025); the open-source MAME
driver for the Gen7 board (source code and comments only); Aristocrat patents (embodiments).

## What is established

| Topic | Finding | Scope | Label |
|---|---|---|---|
| Operating system | The base software is Linux-based on every generation from Gen7 to Gen10: the approvals list "Linux OS" as a component with its own identifier | Australia, Gen7–Gen10 | VERIFIED for Gen8 ([REGULATORY.md](REGULATORY.md)); STRONG EVIDENCE for the others |
| One base across generations | A single base-software identifier covered Gen7 and Gen8 together from 2014 to 2018, later Gen8 with Gen9, and a 2025 instrument lists Gen10, Gen9 and Gen8 together. US lists show the same ("GEN7/GEN7X/GEN8/GEN9", "GEN8/GEN9/GEN10") | Australia and US | STRONG EVIDENCE |
| Game and base are separate | Game, system software and BIOS carry separate identifiers and hashes. US lists name the game a "Personality" program, the jackpot schedule a "Game Assets" item, and the base a "System Base" | both | STRONG EVIDENCE |
| Gen7 storage layout | One CompactFlash card: boot loader and MBR in the first cylinder; partition 1 the Linux OS; partition 2 the system software; partition 3 the game. BIOS in flash on the carrier board; an FPGA configuration image | Australia, **Gen7** | STRONG EVIDENCE (regulatory, with per-component HMAC-SHA1 values) |
| Software verification | Components are listed with HMAC-SHA1 values; signed BIOS images; an FPGA configuration component. The standard required HMAC-SHA1 verification for 2018 builds and HMAC-SHA256 later. A patent embodiment describes an FPGA monitoring first boot code, a hash chain and signed partitions; the emulator's comments mention the same hardware elements | Gen7; inherited by Gen8 only as a hypothesis | STRONG EVIDENCE for Gen7 |
| Gen7 hardware | An Intel Celeron-M class processor module on a carrier board | Gen7 | STRONG EVIDENCE (open-source emulator documentation) |
| Communication protocols | The NSW X-series protocol, the ticket standard and the NSW jackpot standard for the Lightning Link approvals; QCOM in Queensland | Australia | STRONG EVIDENCE |
| Jackpot system | A separate approved device: LPJS 1.YA002 (up to 504 machines) with a "Dragon Link (Standard)" theme added in its 2018 firmware; later 1.YA003, 1.YA018, and 1.YA038 for the "Grand" variants. Dragon Link jackpot setting identifiers D0010001 to D0010009 exist; their values are not published | Australia | STRONG EVIDENCE |
| Series lineage | Lightning Link is ACT specifications 1.HDG00 to 1.HDG09; Dragon Link continues from 1.HDG10, with one Lightning Link title numbered in between. Lightning Link ran on Gen7 and Gen8 under one game identifier | Australia | STRONG EVIDENCE. It does not show shared mathematics |
| Base software identifiers | Gen8: 50002030 (2014) through 51972000 (2023). Gen9: 60102007 (2019) onward. Gen10: 61812000, 62002006, 62022000 (2025) | Australia | STRONG EVIDENCE |

HYPOTHESIS (the research pass's): the Australian eight-digit identifiers encode the US version
scheme, for example 51302007 as 5.13.0-2.00.7.

## What is not public

For Gen8 as shipped in 2018, and for Gen9 and Gen10:

- the storage medium and the partition and file-system layout, from a primary source (Gen7's is
  known; resale listings suggest CFast cards for Gen8, which is a lead only);
- the game package or "personality" format, its compression and any encryption, and the actual
  verification chain;
- the game engine and middleware of the Dragon Link builds;
- the random number generator's algorithm, seeding and call sites;
- the Linux distribution and kernel version;
- any file listing or hash of a Dragon Link build. Only identifiers are public.

No public service manual, teardown, repository or research paper on Gen8 or later was found. The
open-source emulator has nothing beyond Gen7.

## Consequences for the project

1. The planning assumption in [PLATFORMS.md](PLATFORMS.md), that a cabinet image would hold Linux
   file systems and ELF programs, is supported for the operating system. The Gen7 layout is the
   best public model of what a Gen8 medium looks like, and only a model.
2. The mobile package's engine library identifies itself with the same framework names found in
   machine-side job descriptions and strings ([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md)). That link is
   STRONG EVIDENCE at most.
3. If the owner ever lawfully holds a cabinet medium, the regulator-published component hashes and
   identifiers are how it would be identified by generation and build before anything else is
   concluded from it.

## Material that exists and is not obtained

Recorded so that it is not rediscovered and mistaken for an oversight. None of it is held.

- Copies of Gen7 machine software circulate in the emulation scene. They are proprietary software
  copied without apparent licence, and of a different generation and game.
- Used game media for Gen8 are sold by equipment resellers in the US. Buying one is a purchase with
  licence and gaming-device questions attached, and US builds: the owner's decision.
- The complete mathematics of each approved build is held by the accredited testers and regulators,
  as the standard requires it to be submitted. Reaching it would take a formal information request.
- A machine's operator audit mode displays its variation, theoretical return and software
  signatures. Operator access only.
