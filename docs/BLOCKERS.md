# Blockers

Genuine external blockers only: things that cannot be resolved by research, experiment or more
work, and need a person, a purchase, hardware or material nobody on the project holds. Ordinary
unfinished work is in `progress/progress.toml`, not here.

A blocker stops one lane, not the project. Each entry says what proceeds meanwhile.

## The rule for what counts

**Something is a blocker only after lawful public sources have been seriously exhausted for the
exact piece of information that is missing.** "Not on the manufacturer's website" is not "not
available anywhere publicly". Before an item asks anything of the owner, the project searches and
analyses, for each title separately and for each specific field:

- gameplay recordings, including long sessions frame by frame, looking specifically for the Help,
  Game Rules, Paytable and Game Information screens, denomination and bet selectors, line counts,
  displayed return figures, feature and jackpot rules, and version or specification identifiers;
- court filings and their exhibits, read in full rather than from summaries;
- regulatory approvals, standards and submissions;
- archived manufacturer pages, brochures, manuals and promotional material, including the
  Internet Archive;
- patents, academic and security research, preservation forums, public repositories, lawfully
  public service documentation, and listings or photographs that show media or identifiers;
- secondary sites, as leads to stronger evidence only.

A blocker is then narrowed to whatever specific information remains. An entry here names the
fields still missing and what was searched, not a general wish for a source.

This does not change what may be obtained: no pirated cabinet software, no leaked credentials, no
access to operating gaming systems ([LEGAL.md](LEGAL.md)).

## Open

### B-005 — The mobile game's protected tables are not read

- **Since:** 2026-10-13. Narrowed the same day.
- **Blocks:** the *numbers* of rung 2 of the mathematics ladder: the mobile build's reel strips,
  stop weights, pays, prize lists, weight tables and variation parameters, which are on the device
  in protected files. Also reading the mobile build's rules and pay tables from its help artwork.
- **What happened:** decrypting the protected tables, and then exporting the help artwork from the
  game's containers, were each stopped by the safety controls of the AI tooling the project is built
  with. The project does not work around those controls (DECISIONS D-014).
- **What the owner then supplied:** a static analysis of the app's code, done separately, which
  attempted no decryption. It recovers how the mobile game computes a round, and none of the
  numbers ([reverse-engineering/MOBILE_MATH_ARCHITECTURE.md](reverse-engineering/MOBILE_MATH_ARCHITECTURE.md),
  DECISIONS D-016). So the *algorithms* of rung 2 are no longer blocked.
- **Needs, for the numbers:** something that supplies the tables' contents without attacking their
  protection. The analysis names two: the game-load response of an ordinary session (which
  variation runs), and the definitions as a running game already holds them. Both are the owner's
  to arrange.
- **Meanwhile:** the cabinet side is pursued from public footage, which now gives pay entries,
  paylines and reel windows directly
  ([reverse-engineering/GOLDEN_CENTURY_SESSION.md](reverse-engineering/GOLDEN_CENTURY_SESSION.md)).

## Narrowed by the online evidence passes

Two entries were withdrawn as blockers on 2026-10-13 pending a serious search of public sources.
The passes over the litigation record, archives and regulators, and public platform sources have
reported. What they settled is no longer listed; what follows is what remains.

### B-004 (narrowed) — Player-visible facts of the Australian release

- **Settled from public sources** ([reverse-engineering/AUSTRALIAN_CONFIGURATION.md](reverse-engineering/AUSTRALIAN_CONFIGURATION.md)):
  the line configurations (50, 25 and 5 lines), the denominations per state, the launch cabinets,
  each title's free-games rule, the retrigger for two titles, the four jackpot levels and, at
  strong evidence, which are progressive; and venue start-up values for the Grand.
- **Settled since, from public footage of all four titles**
  ([reverse-engineering/FOOTAGE.md](reverse-engineering/FOOTAGE.md)): that lines are chosen by the
  player up to a maximum set by the denomination on Queensland club machines; the free-games award
  of six; the retriggers of two titles; the fixed Mini and Minor values per denomination; that the
  Grand and Major both increment; and, for three titles, the US pay table and Hold & Spin rules
  text read from the machines' own screens.
- **Still not found**, for the Australian builds: the Game Rules and information screens themselves,
  so the pay table and line map as displayed in Australia; any pay table at all for Golden Century;
  any return figure; the full bet ladder; any version identifier tying a recording to a build; the
  Major's parameters and all increment rates.
- **Settled since, by transcribing one session**
  ([reverse-engineering/GOLDEN_CENTURY_SESSION.md](reverse-engineering/GOLDEN_CENTURY_SESSION.md)):
  sixteen pay-table entries for Golden Century and the shapes of lines 1 to 5, read from the
  machine's own win messages without any rules screen.
- **Not yet done, and next:** more sessions read the same way, for the rest of the pay table and
  the other titles; a denser scan of the two main Australian channels for a rules screen; hosts
  other than YouTube.
- **Not exhausted, recorded so they are not assumed done:** forums and marketplaces that refused
  automated reading; other US regulators' certification lists; open-data portals of NSW.
- **Asked of the owner:** nothing. A visit to a machine would be asked only for a field that
  footage turns out not to show.

### B-002 (narrowed) — Cabinet software and mathematics

- **Settled from public sources**
  ([reverse-engineering/PUBLISHED_RETURNS.md](reverse-engineering/PUBLISHED_RETURNS.md),
  [reverse-engineering/PLATFORM_SOFTWARE.md](reverse-engineering/PLATFORM_SOFTWARE.md),
  [reverse-engineering/LITIGATION.md](reverse-engineering/LITIGATION.md)): a published US payout
  range for Golden Century and Happy & Prosperous; Lightning Link's Australian denominations,
  maximum bet and return by denomination; the rules any compliant build must satisfy on outcome
  selection, timing, independence and jackpots; the base-software lineage and identifiers from
  Gen7 to Gen10; the Gen7 storage layout and boot-chain design; that the feature draws from its
  own reels in Lightning Link.
- **Not public**, after these passes:
  - *mathematics, every Australian build:* reel strips for base game, free games and the feature;
    symbol and prize weight tables; variations and their returns; return composition; hit, feature
    and free-games frequency; jackpot hit probabilities, start-up, increment and ceiling; what
    changed between generations;
  - *platform, Gen8 and later:* the storage and file-system layout from a primary source; the game
    package format and its protection; the engine; the random number generator's algorithm and
    call sites; any file listing or hash of a Dragon Link build.
- **Path that needs no software:** reel windows read from public footage of machines and assembled
  into strips (DECISIONS D-015). It will not give prize weighting or jackpot rates.
- **Not exhausted:** the patent sweep (several sources refused automated reading; some patents
  unread); regulators' disclosure logs; test laboratories' public reports.
- **Exists and is not obtained** (the owner's decisions, none taken): leaked or unlicensed copies of
  machine software and confidential sheets; used game media sold by equipment resellers; a formal
  information request to a regulator; redacted court filings available for a fee.

## Resolved

### B-003 — Official mobile software needed a store login (resolved 2026-10-13)

The project installed Google's Android emulator with a Play Store image on the Windows host; the
owner signed in and installed Mighty Fu Casino, and opened Golden Century so that its content
downloaded. The installed package and the downloaded content were copied out with the platform's
own tools, hashed and inventoried ([reverse-engineering/MOBILE_PACKAGE.md](reverse-engineering/MOBILE_PACKAGE.md)).
The other three titles' content downloads when each is first opened in the app.

### B-001 — No working GitHub login on the Windows host (resolved 2026-10-13)

The access token in the environment was rejected and no other login was stored, so nothing could be
pushed. The owner logged in with `gh auth login`. All commits made in the meantime were pushed
unchanged, and CI ran green on the first push.
