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

Only the first can be produced today. The next step towards the others is replaying an
observation's outcomes through the simulation with a scripted outcome source, so that rules are
compared on the same outcomes; that is tracked under "Parity".

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
