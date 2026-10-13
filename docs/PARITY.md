# Parity

How agreement with the original games will be measured. Today **nothing is measured**: there is no
recording of the original, no original mathematics and no application. This document describes the
method so that it is built before the game "looks finished", and states plainly what is and is not
claimed.

## What is claimed today

Nothing about the original. Specifically:

- The simulation is deterministic and replays its own recorded games exactly. That is a property of
  our code (`deterministic-replay` in the tracker). It is not parity.
- Every run uses the synthetic development fixture. Its hit rates, returns and feature frequency
  are properties of the fixture.

## What parity means here

Observable behaviour, compared in layers, each with its own evidence:

| Layer | Question | Evidence it needs |
|---|---|---|
| **Rules** | Given the same outcome, does our game award the same and move through the same states? | Rules screens, regulatory documents, recordings where an outcome and its award are both visible. |
| **Events** | Does the same thing happen in the same order (reel stop order, award order, feature steps)? | Recordings transcribed into traces. |
| **Timing** | Do the steps take as long (reel spin and stop intervals, count-up, transitions)? | Frame-accurate recordings. |
| **Visual** | Does a frame look the same (layout, artwork, typography, animation frames)? | Recordings, imported assets. |
| **Audio** | Does the same sound start at the same moment? | Recordings with clean audio, imported audio. |
| **Mathematics** | Are outcomes distributed the same way? | Cabinet software or regulatory mathematics. **Not** obtainable from recordings or from mobile software. |

The mathematics layer is separate on purpose. A game can match rules, events, timing, picture and
sound and still not be known to match the distribution of outcomes.

## Traces

Both sides of a comparison are written in one vocabulary, the simulation's events
(`crates/dragonlink-core/src/event.rs`):

- a **rewrite trace** is what our simulation emitted, with the math model's provenance and the
  outcome source named in the header;
- an **observation trace** is a transcription of a recording of the original: the same events, with
  what could be seen. Fields that cannot be seen (a reel's stop index) are left out.

A comparison lines the two up event by event and reports the first divergence and the timing
differences. Comparing two rewrite traces is a determinism check and is labelled as such, never as
parity.

The format is `dragonlink-trace/1`: JSON Lines, a header then one record per line
(`crates/dragonlink-trace`). The header names the origin and, for a rewrite, the math model's
provenance and the outcome source; for an observation, the source class and the SHA-256 of the
recording. A header that would mislead is refused: a rewrite that hides its model, synthetic
mathematics labelled as a cabinet source, an observation with no recording behind it or one that
claims to know the original's model.

```sh
cargo run -p dragonlink-trace-cli -- record --seed 7 --spins 200 --out a.jsonl
cargo run -p dragonlink-trace-cli -- validate a.jsonl
cargo run -p dragonlink-trace-cli -- compare a.jsonl b.jsonl --timing-tolerance-ms 33
```

`compare` states which of four things it did:

| Kind | When | What it shows |
|---|---|---|
| Determinism | two rewrite traces | our simulation repeats itself. **Not parity.** |
| Structure only | an observation against a rewrite on synthetic or mobile-derived mathematics | the order and kinds of events; outcomes are not expected to match |
| Parity | an observation against a rewrite on recovered mathematics | agreement with the original |
| Between observations | two observations | agreement between recordings |

Only the first can be produced today, because there is no observation of the original yet.

### Replay

Lined up against a free-running rewrite, an observation differs at the first reel: the two games
were not dealt the same outcomes. `replay` removes that difference.

```sh
cargo run -p dragonlink-trace-cli -- replay observation.jsonl --model synthetic
```

It reads **only the outcomes** a trace shows (the window each reel stopped on, the value each
prize symbol carried, where prize symbols landed on each respin, the bet), derives the draws that
make the simulation produce them under the given model, runs them, and compares what the
simulation emitted with the trace. Awards, credit movements, feature entry and counters are never
used to steer it; they are what is being checked. What still differs is a difference in rules.

A replay reports, and never papers over:

- **hidden choices**: several stops of a strip showing the same window, or several table entries
  carrying the same value. The lowest is scripted and the choice is listed;
- **inconsistencies**: an observed window the model's strip does not contain, a value its table
  cannot give, a reel that never stopped. Nothing from that point on is replayed, because that
  would mean inventing an outcome;
- **where the simulation could not follow**, when it refuses a command the trace implies.

And it states which of these it was:

| Kind | When | What it shows |
|---|---|---|
| Machinery only | through the synthetic fixture | that the replay tooling works. Nothing about any original game. **Not parity.** |
| Rules under a model | through mathematics that is not evidence of the cabinet game's (a mobile-derived model) | our rules under that model's reels and tables. **Not parity** with the cabinet game |
| Parity | through recovered cabinet mathematics | agreement of rules and event order **on the outcomes observed**. Nothing about how outcomes are distributed |

It has been exercised only on our own traces relabelled as observations.

## From a recording to an observation trace

Three steps, each of which can be repeated by anyone holding the same recording:

```sh
# 1. Find the moment each game came to rest, and cut one still per game.
cargo run -p dragonlink-capture -- rest-frames session.mp4 --reels 200:450:1440:430 --out moments.json
cargo run -p dragonlink-capture -- stills session.mp4 --moments moments.json --crop 90:400:1720:660 --out-dir <folder outside Git>

# 2. Read each still into a window sheet (by eye, or by a reader that is then checked).

# 3. Turn one configuration of the sheet into an observation trace.
cargo run -p dragonlink-trace-cli -- transcribe sheet.jsonl --denom 1c --lines 50 \
    --capture-sha256 <hash of session.mp4> --scope "cabinet-au, Queensland club" --title golden_century --out trace.jsonl
```

1. **Rest frames.** A game's reels start together and stop one after another, so the change in the
   reel area falls away and the picture is then still until the next game starts. `rest-frames`
   measures that change on a small grey copy of the reel area and takes, for every sharp start out
   of rest, the stillest frame shortly before it: the window of the game that ended. Not the frame
   just before the start: reels creep before they spin, and a creep of half a symbol barely
   registers on a small grey picture although it ruins the window. (The first version took the
   frame just before, and people reading its stills found a third of them mid-creep.) It reads no
   symbols and knows nothing about any game. Its thresholds are options, it prints the spread of
   its measurements so they can be chosen deliberately, and it records how still each chosen frame
   was. `stills` refuses to write into a Git work tree.
2. **The window sheet** is JSON Lines, one game per line: the time, the kind of game, the
   denomination, the line count, the three meters as displayed, any win message, and the window as
   five reels of three symbol codes, with `?` for anything not read
   (`crates/dragonlink-trace/src/transcript.rs`).
   A sheet is a transcription of a recording and is kept with it.
3. **`transcribe`** takes one configuration at a time, because reels and tables are not assumed to
   be shared between denominations or line counts. It **fills nothing in**: a reel with an unread
   position is left out, an unread award is left out, and a prize value that is not a whole
   multiple of the bet is counted and left out. It reports all of that, and it checks the meters
   against each other: each game's credit meter should equal the one before, less the bet, plus
   the win. Pairs that do not fit point at a misread digit or a missed game, which makes this the
   cheapest test of a transcription.

The trace is then input to reel reconstruction (`dragonlink-analyze reconstruct-reels`) and to
replay. An observation made this way contains only what a still shows: windows, prize values and
the game's total award. It has no line-by-line wins, so a replay against it will differ at the
first event the observer could not record; comparing only the kinds of event an observation holds
is tracked as the next step.

## Recordings

Recordings of the original are source material: they live under `captures/` in the private source
root and are never committed. What may be published from them: derived measurements (durations,
orders, counts), behavioural descriptions in our own words, and transcribed traces of events, each
citing the recording by hash.

For each recording, record: what it shows (title, market, cabinet if known, denomination and bet if
visible), where it came from, resolution and frame rate, and the SHA-256 of the file.

## Tolerances

A tolerance is set from measured noise, not chosen to make a comparison pass: the frame interval of
the recording bounds timing precision; compression and capture scaling bound visual precision. Each
parity test states its tolerance and where it came from.

## Statistics

Used carefully. A sample of observed spins is recorded with its size and never promoted to a
probability, an RTP or a feature frequency labelled verified ([EVIDENCE.md](EVIDENCE.md), rule 8).

## Discrepancies

A known difference between our behaviour and the evidence is recorded in
`docs/reverse-engineering/` with the evidence on both sides and stays visible until it is resolved.
A difference is not hidden by loosening a tolerance.
