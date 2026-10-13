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

### B-005 — The mobile package's protected content is not read by the project's tooling

- **Since:** 2026-10-13
- **Blocks:** rung 2 of the mathematics ladder (the mobile build's reel strips, weight tables and
  variation parameters, which are on the device in protected files), and reading the mobile
  build's rules and pay tables from its own help artwork.
- **What happened:** decrypting the protected tables, and then exporting the help artwork from the
  game's containers, were each stopped by the safety controls of the AI tooling the project is built
  with. The project does not work around those controls.
- **Needs:** a decision by the owner on whether and how that content is read by other means. Not
  something the project's tooling does.
- **Meanwhile:** rules, pay tables and bet structure are pursued from footage, the litigation
  record, archives and regulators. The mathematics is pursued from cabinet-side public sources.
  What was learned from the package without reading protected content stands
  ([reverse-engineering/MOBILE_PACKAGE.md](reverse-engineering/MOBILE_PACKAGE.md)): the engine, the
  formats, and that the four titles are two pairs sharing their tables.

## Under investigation, not yet blockers

These were listed as blockers on 2026-10-13 and were withdrawn the same day: a first search had
come up empty, which is not the same as the sources being exhausted. Each is now an active online
evidence pass. It returns here only as a narrowed entry naming exactly what could not be found.

### B-002 (withdrawn) — Cabinet mathematics and software

- **What is missing:** reel strips, weighting, RTP and its composition, feature and jackpot
  probabilities, the outcome generator, cabinet-specific assets and timing.
- **What is known:** the manufacturer asserts the mathematics as trade secrets and says it is not
  in its patents (`docs/reverse-engineering/math/EVIDENCE_STATE.md`). That is a statement about what
  the manufacturer published, not about what is public.
- **Being searched:** litigation discovery and exhibits; regulatory exhibits and submissions;
  patents; archived technical documents; academic and security research; preservation forums;
  public repositories; the Internet Archive; lawfully public service documentation; machine
  listings with media or storage photographs; publicly exposed package and file metadata; related
  Lightning Link and Dragon Link platform research.
- **Limit:** material that exists publicly but could only be obtained by taking pirated cabinet
  software is recorded as a lead and not obtained.
- **Path if that comes up empty:** the official mobile implementation, first analysed locally,
  then, if outcomes are decided server-side, observed through automated collection and
  reconstructed (DECISIONS D-013). So this lane is not blocked.
- **The only residue that can stay open:** proof that the hidden probabilities are those of the
  Australian cabinet. That needs genuine cabinet software or configuration and nothing else settles
  it. It is recorded as a known limit of the result, not as work waiting on anyone.

### B-004 (withdrawn) — Player-visible rules, pay tables and game information

- **What is missing:** per title and build, the number of lines, the bet ladder, denominations, the
  pay table, feature rules (free games count and retriggers), which jackpot levels are progressive,
  and the displayed return figure.
- **What is known:** these are shown to players on the machine's own screens. Footage of all four
  titles exists, including Australian footage, and the litigation record reproduces pay-table
  screens.
- **Being searched:** recordings of each title for the Help, Rules, Paytable and Game Information
  screens, cross-referenced between recordings to tell cabinet generations and configurations
  apart; the full litigation filings and exhibits; archived product pages, brochures and manuals.
- **Not asked of the owner:** a visit to a machine. That would be asked only for a specific field
  that the online pass shows cannot be recovered.

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
