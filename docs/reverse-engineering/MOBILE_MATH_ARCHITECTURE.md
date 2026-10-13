# The mobile game's mathematics engine: how it works

What a static analysis of the official mobile app's code established about how Golden Century
computes a game there. Received 2026-10-13.

- **Scope: the mobile implementation**, Mighty Fu Casino 5.6.0 for Android
  ([MOBILE_PACKAGE.md](MOBILE_PACKAGE.md)). Nothing here is a statement about cabinet mathematics
  ([../EVIDENCE.md](../EVIDENCE.md), rule 4). Where a cabinet observation agrees or disagrees, that
  is said separately.
- **Where it comes from.** The owner had the app's native code analysed separately from this
  project's tooling, and handed over a report, a set of small reference routines and a map of the
  functions examined. They are kept privately with the software. This page restates the findings
  that bear on the reimplementation, in our words.
- **What was not done.** No protected table was read, and no key recovery or decryption was
  attempted. Reel contents, stop weights, pays, prize arrays and every probability **remain
  unknown**. No game was instrumented and no network traffic was captured: conclusions about what
  runs where are from the code's paths, not from watching it run.
- **Labels.** The report labels its own findings. Here everything is STRONG EVIDENCE at most:
  one analyst, one pass, not re-derived by this project.
- **Left out on purpose.** Addresses and the function map (they belong with the binary). And two
  details that concern the security of a live product rather than how the game plays: how the
  generator is seeded, and the hooks that exist for supplying numbers from outside. They are in the
  private report.

## The architecture

- **The game's mathematics runs inside the app.** The app builds its game with a "local server"
  setting switched on, and the class called a game server is code in the app's own native library.
  Reel stops are drawn there from a generator constructed there.
- **The backend's part is the economy.** Rounds are reported to the operator's service as signed
  wager and win records, and progressive jackpot *amounts* come back from a jackpot service. No
  round record carries a symbol grid, stops or a seed.
- So: **local title mathematics, with backend economy and jackpot settlement.** Not a client that
  displays a grid the server chose.
- **The backend chooses which variation runs.** The response to loading a game carries a
  variation name, and the engine takes its last two characters as a folder: Golden Century has
  four, labelled 88, 90, 92 and 94. That choice is made once per session, not per game. Which one
  a given session gets, and what the labels mean, is unknown: **they are folder names and are not
  to be read as return percentages.**
- Between those four folders only two files differ: the configuration (two pairs of two) and the
  table of prize-symbol weights (all four differ). Reels, scenes and the reveal table are the
  same bytes in all four. So whatever the variations change, it is concentrated in how prize
  symbols are weighted.

## One engine, two titles

Golden Century and Autumn Moon are built on the **same game-server class**, and 45 of their 48
protected files are identical; the three that differ are variation configurations. Happy &
Prosperous and Panda Magic share 46 of 47. All four titles carry the same file of Hold & Spin
reels, byte for byte. That is shared material, not proof of equal behaviour: frequencies also
depend on tables that differ.

Four sub-games are set up for a title, each with its own definition: the base game, the free
games, Hold & Spin entered from the base game, and Hold & Spin entered from free games.

## Reels and windows

- Reels are **named cyclic strips**, with an optional weight on each stop. With weights, a stop is
  chosen in proportion to its weight; without, uniformly. Whether Golden Century uses weights is
  not known: the capability is in the code, the values are in protected files.
- **The chosen stop is the middle row.** A three-row window is the stop and its two neighbours.
- A definition can hold **several reel sets, selected by bet index and by line index.** The title
  ships separate strip files for the base game and the free games, each in a second version whose
  name suggests "no bonus". Which is used when is not established.

*Against the cabinet:* a Queensland machine was seen to use different reels at $1 than at the cent
denominations, and the same reels whatever number of lines was played
([GOLDEN_CENTURY_SESSION.md](GOLDEN_CENTURY_SESSION.md)). An engine that selects a reel set by bet
configuration is the kind of mechanism that would produce the first. This is a resemblance between
two implementations, not evidence that they share data.

## What happens in one base game, in order

1. A stop is drawn for each reel.
2. **Prize symbols get their values before the window is built, and over the whole strip.** Every
   prize-symbol placeholder on each strip is given an identity from a weighted table chosen by bet
   index, whether or not it will be visible. So values are drawn for prize symbols nobody sees.
3. The window is built from the strips as they now stand.
4. Line and scatter wins are evaluated, then feature entry.

Point 2 matters for any replay: an implementation that draws values only for visible prize symbols
consumes a different number of draws and falls out of step. Our synthetic model draws for visible
ones only; that is a difference to carry in configuration, not to hide.

## Wins

- Combinations are table data of three kinds: on a line, on a line in any position, and scattered.
  A line is read in a stated direction, and a substitute symbol stands in only where a table says
  it may. **Which symbols a wild replaces is data, not a rule in code.**
- A win is the combination's award times a bet quantity, and for some combinations times the cost
  per line as well. Which quantity each is in practice has not been traced through the bet
  manager, so this is not yet a formula to implement.
- Money is converted from fractional amounts by **truncation**, not rounding.
- The app's own readable text says three of one scatter start the free games and six of the other
  start Hold & Spin.

## Free games: the reveal

Before pays are evaluated in a free game:

1. The masked positions in the window are found.
2. **One** symbol is drawn for all of them from a weighted table, with a different table when all
   fifteen positions are masked.
3. If it is an ordinary symbol, every masked position becomes that symbol.
4. If it is the prize symbol, each masked position draws its own value.

*Against the cabinet:* this is what the manufacturer's Australian page describes ("all special
symbols reveal the same symbol, including the chance of jackpot symbols"), and what footage shows
([FOOTAGE.md](FOOTAGE.md)).

The engine's default for extra free games on a retrigger is five, which a title's configuration
can override; Golden Century's value is in a protected file. On Australian machines the award is
six and the retrigger was three for two other titles: the default is not the game's value.

## Hold & Spin

- **Entry.** Prize symbols in the window are counted. Six or more start the feature with three
  spins. Fifteen award the Grand, **if the Grand is enabled for the current bet bracket**.
- **Below six, the engine has further paths** that can award the Grand or the Major at random,
  from weighted yes-or-no tables. Whether they are enabled, and how likely, is protected. So in
  this implementation "the Grand is won only by filling the screen" is not established.
- **State carried.** Which positions hold a prize symbol and which symbol each holds; the spins
  remaining; and whether one specially chosen position has hit yet. Held symbols are not drawn
  again.
- **Each respin.** Only empty positions spin, **each on a strip of its own**, taken from a
  separate file of Hold & Spin strips. One empty position is chosen uniformly at random and, until
  it lands a prize symbol once, spins on a strip picked from a weighted table by bet index; the
  others spin on one of two standard strips, one for the first column and one for the rest.
- **A different draw.** A respin position's stop is the random number reduced by the strip's
  interval directly. The base game's weighted mapping is not used. Whether that makes a difference
  depends on whether those strips carry weights, which is not known.
- **Counter.** After a respin the count goes down by one; if a new prize symbol landed it returns
  to three; if all fifteen positions are then full the feature ends at once and the Grand is
  queued where enabled. **A full screen takes precedence over the reset.** At zero, prizes are
  awarded.
- **Prizes.** A prize symbol's identity is a code. Three codes mean Major, Minor and Mini; any
  higher code is an index into a list of prize amounts for the bet, with a second list for an
  enlarged kind of prize symbol. So the figure on a prize symbol is looked up, not computed from
  its name. Minor and Mini are amounts from the bet bracket. Grand and Major are jackpots whose
  amount comes from the jackpot service.

*Against the cabinet:* six to start, three spins, back to three on a new prize symbol, fifteen for
the Grand: the same as the US rules text and the Australian prompts ([FOOTAGE.md](FOOTAGE.md)).
Individual strips per position is what the US rules call "individual reels" and what the
litigation record says of Lightning Link ([LITIGATION.md](LITIGATION.md)). The enabling of the
Grand by bet bracket, and the random paths below six, have no cabinet evidence either way.

## Jackpots

- The game decides locally that a jackpot is hit. The amount is requested from the jackpot
  service and returned with an identifier, a level and an amount, and is paid as returned.
- A hit is not finished when its animation plays: the engine keeps a queue of pending jackpots and
  completes the round only when it is empty.
- The numbering of levels differs between the game's own records, the request sent out and the
  response. They must not be assumed equal.
- Contribution, reset and increment rules were not found in the code examined.

## Random numbers

- A standard generator of the Mersenne Twister family supplies 32-bit words.
- A number below a bound is taken by **dividing the range into equal parts and redrawing when the
  word falls in the remainder**. It is not the remainder of a division, so it has no bias towards
  low numbers.
- How the generator is seeded, and what can supply numbers in its place, are recorded privately.

This is the mobile app's generator. The regulatory requirements that bind a cabinet's generator
are in [PUBLISHED_RETURNS.md](PUBLISHED_RETURNS.md); nothing here says how a cabinet meets them.

## What this changes for the project

1. **Rung 2 of the ladder has its algorithms and none of its numbers.** The mobile game's rules can
   now be implemented as a configuration of our simulation. Running it as the mobile game still
   needs the tables.
2. **The simulation should grow these mechanisms**, each as configuration, none presented as
   cabinet behaviour: reel sets chosen by bet and line configuration; the stop as the middle row;
   prize-symbol values assigned over the whole strip before the window; one reveal symbol for all
   masked positions; a strip per position in Hold & Spin with one specially weighted position;
   the full-screen-before-reset rule; prize codes looked up in lists; truncating conversion.
3. **Replay must know which draw order it is replaying.** An observation of the mobile game needs
   the mobile order; our synthetic fixture keeps its own.
4. **The protected tables stay unread** ([../BLOCKERS.md](../BLOCKERS.md), B-005). The report names
   what would close each gap without touching them: the game-load response of an ordinary session,
   and definitions as the running game already holds them.

## Still unknown in the mobile implementation

All reel stops and weights; which reel set is selected when; the paylines; substitutions; every
award; the free-games award and retrigger; every weight in the prize-symbol, reveal and feature
tables; the random Grand and Major chances; bet brackets; Mini, Minor and the prize lists; the
variation a session runs; jackpot contribution and reset; and some rarely taken branches where
several awards coincide.
