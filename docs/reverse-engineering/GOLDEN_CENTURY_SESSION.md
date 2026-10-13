# Golden Century: one Australian session, transcribed

The first recording of an original machine read game by game. It gives Golden Century what the
footage passes could not find: pay-table entries, the shape of the first five paylines, and a
finding about how the game differs between denominations. Read 2026-10-13.

- **Recording:** YouTube `y8c8lEdPW8Y`, 53 minutes, a Queensland club machine whose top-screen line
  reads Golden Century, Dragon Link, © 2016. SHA-256 of the file held:
  `b4902dfbf78dcc53a4124295a25118021291fad9f00c4285d3839d1c0e93099e`.
- **Scope of everything here:** one session on one machine. Australian cabinet, Queensland club.
  Which build generation the machine runs cannot be told ([FOOTAGE.md](FOOTAGE.md)).
- The recording, the stills cut from it and the transcription are source material and are kept
  privately. This page holds what was read, in our words and figures.

## How it was read

1. `dragonlink-capture` found the moment each game came to rest: **583 games**
   ([../PARITY.md](../PARITY.md), "From a recording to an observation trace").
2. One still per game was cut and read into a window sheet: the fifteen positions, the
   denomination, the line count, the meters, and any win message.
3. **Two sets of stills were read by different readers.** The first set was cut a fifth of a second
   before each game started; a third of its stills caught the reels already creeping, and its
   readers said so. The picker was corrected, a second set was cut, and every game was read again
   from it.
4. Where both readings saw a reel at rest they were compared, position by position.

| Check | Result |
|---|---|
| Games with a reading | 583 of 583 |
| Kinds of screen | 543 base games, 23 free games, 11 Hold & Spin, 6 not a reel window |
| Reel windows read twice, by different readers from different frames | 1,760, and **none disagree** |
| Reel windows read once | 1,116 (the first set was moving, or one reading left a position unread) |
| Positions left unread in the final sheet | 9, in three games, all blanked by a win animation at the moment of the still |
| Prize-symbol values read | 664; every one is a whole multiple of the game's total bet |

What a single reader saw, and nobody has re-read: the win messages, and the meters.

## What the session shows about the game

Labels follow the claims database: a reading from footage is STRONG EVIDENCE at most until it has
been read twice. The windows have been; the messages and meters have not.

### Bets played

| Denomination | Lines | Credits per line | Bet | Games |
|---|---|---|---|---|
| $1 | 5 | 1 | $5.00 | 234 |
| 10c | 25 | 2 | $5.00 | 117 |
| 2c | 50 | 5 | $5.00 | 77 |
| 5c | 25 | 4 | $5.00 | 33 |
| $1 | 4 | 1 | $4.00 | 22 |
| 1c | 50 | 10 | $5.00 | 17 |
| 10c | 20 | 2 | $4.00 | 15 |

Fewer games each: 2c with 4 credits per line, and $1 with 3 lines and 2 lines. So the player
reduces lines one at a time, and ten credits per line is offered at 1c. The complete list of
options is still not shown.

### The first five paylines

A win message names a line and an amount ("Line 3 wins 5"). 64 such messages name lines 1 to 5.
Assuming the usual shapes, every one of them is explained by a run of one symbol from the left
reel on the named line, and none is left over:

| Line | Positions, reel 1 to reel 5 |
|---|---|
| 1 | middle, middle, middle, middle, middle |
| 2 | top, top, top, top, top |
| 3 | bottom, bottom, bottom, bottom, bottom |
| 4 | top, middle, bottom, middle, top |
| 5 | bottom, middle, top, middle, bottom |

STRONG EVIDENCE. Another 27 messages name lines between 9 and 25; their shapes are not known and
nothing is assumed about them.

### Pay-table entries

Pay per credit bet on the line, for a run from the left reel with the wild (the emperor) standing
in. Each entry is what a message on lines 1 to 5 showed; "n" is how many messages, with the time
of the first in the recording. An empty cell means no message was seen, not that nothing is paid.

| Symbol | Count | At $1 | At 1c to 10c |
|---|---|---|---|
| Two women | 5 | 150 (n=1, 18:53) | |
| Two women | 4 | 30 (n=1, 20:31) | |
| Two women | 3 | 5 (n=1, 31:57) | |
| Two women | 2 | 2 (n=8, 6:18) | |
| Chest | 5 | 100 (n=1, 8:21) | |
| Chest | 3 | 5 (n=3, 10:26) | 5 (n=1, 2c, 24:46) |
| Chest | 2 | 2 (n=9, 11:05) | |
| Vase | 3 | 5 (n=2, 33:01) | 5 (n=1, 10c, 18:20) |
| Vase | 2 | 2 (n=9, 6:50) | |
| Warriors | 2 | **3** (n=8, 7:36) | **2** (n=4; 1c, 5c, 10c; 1:19) |
| King | 3 | 5 (n=3, 43:16) | 5 (n=2; 2c, 10c; 5:43) |
| Queen | 3 | | 5 (n=1, 10c, 29:22) |
| Jack | 4 | 10 (n=1, 34:59) | |
| Jack | 3 | 5 (n=3, 10:14) | |
| Ten | 4 | 10 (n=2, 22:37) | |
| Ten | 3 | 5 (n=2, 10:05) | |

- STRONG EVIDENCE for entries seen more than once, PARTIAL for those seen once. Sixteen
  combinations were seen; fifteen show one value every time.
- **All four pictures pay for two of a kind.** In the US pay table of the other three titles only
  the top picture does ([FOOTAGE.md](FOOTAGE.md)). So Golden Century's Australian pay table is not
  that table.
- Three scatters pay twice the total bet (one observation, 10c; [FOOTAGE.md](FOOTAGE.md)).
- Not seen: any pay for the nine, any five-of-a-kind for a royal, the scatter for four or five, and
  any pay made up of wilds alone.

### The pay table differs between $1 and the cent denominations

Two warriors pay 3 at $1, in all eight messages, and 2 at 1c, 5c and 10c, in all four. Nothing
else in the table above differs where both columns are filled. STRONG EVIDENCE, for that one entry.

This is the first direct observation, on an Australian machine, of something the litigation record
says of the family: that pay tables are not the same at every denomination
([LITIGATION.md](LITIGATION.md)).

### The reels differ between $1 and the cent denominations

For each reel, a window is the three symbols it stopped on. If two configurations use the same
reel strips, the windows seen in one will mostly have been seen in the other, as often as a
configuration's own second half repeats its first half.

| Comparison | Share of windows already seen in the other set, reels 1 to 5 |
|---|---|
| $1 with 5 lines: second half of the games against the first half (the yardstick) | 92%, 88%, 93%, 88%, 79% |
| $1 with 4 lines, against $1 with 5 lines | 100%, 100%, 100%, 96%, 96% |
| 5c against 10c | 97%, 97%, 92%, 95%, 84% |
| 2c against 10c | 96%, 88%, 92%, 88%, 79% |
| 1c against 10c | 100%, 94%, 89%, 100%, 78% |
| **10c against $1** | **64%, 54%, 76%, 42%, 72%** |
| **2c against $1** | **71%, 60%, 79%, 37%, 76%** |

- **Changing the number of lines does not change the reels.** The same windows appear.
- **1c, 2c, 5c and 10c are consistent with one set of reels.**
- **$1 is not that set.** The shortfall is far beyond the yardstick, and largest on the fourth
  reel, where the mix of symbols also differs visibly (the chest is about twice as common at the
  cent denominations as at $1).
- Label: STRONG EVIDENCE that the reels differ, class *inferred statistically*, from 266 games at
  $1 and 276 at the cent denominations. It is a comparison of what was seen, not a comparison of
  strips, and it has not been put through a formal test yet.

Together with the pay-table entry above, this is what the mathematics lane asked for under
"configuration differences": on this machine the $1 game is a different mathematical configuration
from the cent games, not the same game with bigger credits.

### Prize symbols

- Every one of 664 values read is a whole multiple of the total bet: 1, 2, 3, 4, 5, 10, 15, 20, 50
  or 200 times it, or a level's name (Mini, Minor, Major). All ten multiples are among the twelve
  the US rules list (250 and 100 were not seen).
- At $1 the value is printed in dollars, elsewhere in credits. STRONG EVIDENCE, read once.
- Prize symbols land in stacks: a reel can show three at once.
- How often each value appears is the kind of figure the project keeps out of the repository
  ([../DECISIONS.md](../DECISIONS.md), D-010); the set of values is what a player sees.

### Features seen

Hold & Spin was played to the end several times, twice inside free games, and free games were
triggered five times. Stills of features were read but features are not yet analysed: their reels
and rules are separate from the base game's.

## Reel reconstruction: not there yet

The trace of each configuration was put through `dragonlink-analyze reconstruct-reels`.

- At $1, 236 base games give nearly complete coverage of four reels: the tool estimates that under
  3% of further games would show a window not yet seen on those reels, and about 6% on the fifth.
- **No strip assembles yet.** Two reasons. Some windows are still missing. And symbols come in
  stacks, so the same window sits at more than one stop: assembling such a strip needs to know how
  many times, and a few hundred games cannot tell three times from four.
- The cent denominations have far fewer games each and are further off.
- Strips, and anything that would give them, stay private when they exist
  ([../DECISIONS.md](../DECISIONS.md), D-010 and D-015).

More sessions of the same machine type are the remedy, and the tooling now takes them: cut,
read twice, convert, reconstruct.

## Limits

- One session, one machine, one evening. Nothing here says the next machine is configured the same.
- The win messages and meters were read once. A message is one of several the machine cycles
  through, so a game's whole win is often more than the one message read.
- What each game paid in total has not been derived yet: the credit meter is being read at a fixed
  instant after each game for that, and the result will be checked against the entries above.
- The detector missed some games (the credit meter shows where), so the session is not a complete
  account and no return figure can be taken from it.
- The build generation is unknown.
