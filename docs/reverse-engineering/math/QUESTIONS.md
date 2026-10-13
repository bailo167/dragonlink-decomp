# Questions the mathematics lane must be able to answer

Each with what it needs and where it stands on 2026-10-13. "Tool ready" means the calculation
exists and is tested on the synthetic fixture; it does not mean the question is answered. **No
question about the original games is answered yet, because no original configuration is held.**

| Question | Needs | Tool | Answer for the originals |
|---|---|---|---|
| Which of the four games has the highest RTP under comparable configurations? | A configuration per title | `analyze-game`, `compare-games` | unknown |
| Does denomination change RTP or feature frequency? | Configurations of one title at two denominations | `compare-games` | unknown. Lead: Aristocrat's complaint quotes different pay-table figures for two denominations ([EVIDENCE_STATE.md](EVIDENCE_STATE.md)) |
| Does increasing the bet affect anything except monetary scale? | The bet-dependent parts of a configuration | `compare-games` (reports "prize scaling only" or not) | unknown |
| What is the expected frequency of Hold & Spin? | Reel strips | `analyze-game` (exact) | unknown |
| What is the expected value of entering Hold & Spin? | Feature reels or chances, value table | `analyze-game` (exact) | unknown |
| What proportion of RTP comes from base game, features and jackpots? | A full configuration | `analyze-game` (exact composition) | unknown |
| How rare is the Grand? | Feature mathematics | `analyze-game` (exact) | unknown |
| How do progressive meters change EV? | Jackpot frequencies, meter rules | `progressive-ev` | unknown |
| At what displayed values, if any, is a configuration unusually favourable? | The same | `progressive-ev` (break-even meters) | unknown |
| Are there observable states worth preferring or avoiding? | Evidence of state dependence | not yet: the model has no state between spins | unknown |
| Does anything carry from one spin to another? | Executable analysis | `behaviour.spin_independence` | unknown |
| Is reel stopping purely presentation? | Executable analysis | `behaviour.reel_stop_presentation` | unknown |
| Can button timing have any mathematical effect? | Executable analysis (where and when the generator is called) | `behaviour.input_timing` | unknown |
| Are two machines showing the same title necessarily running the same mathematics? | Configurations of two program versions | `compare-games` | unknown. Established: at least three Australian generations of each title exist ([../PLATFORMS.md](../PLATFORMS.md)) |
| What differs between specific approved program or version IDs? | The same | `compare-games` | unknown |
| Does the mobile implementation contain copies or variants of the math tables? | Official mobile software, analysed | `compare-games` (mobile vs cabinet) | unknown |
| What commonly held player beliefs are demonstrably false? | Answers to the rows above | a section of the report, to be written from findings | none can be judged yet |

## Sorting a belief

When there is evidence, each belief about the games is placed in exactly one class, with the
evidence that puts it there:

| Class | Meaning |
|---|---|
| Myth | Contradicted by the recovered implementation. |
| Mathematically irrelevant observation | True as observed, but changes no probability or value. |
| Configuration difference | Real, and explained by denomination, bet option or operator setting. |
| Genuine state-dependent behaviour | The implementation's behaviour depends on a state that persists. |
| Genuine expected-value difference | Some observable condition changes the expected return. |
| Implementation quirk | A departure from the obvious design that does not change value. |
| Defect | A departure from the intended mathematics ([README.md](README.md), "Defects"). |

Nothing is placed in a class on plausibility. Until evidence exists, a belief is listed as untested.
