# Methods

How each quantity is computed, what it assumes, and how it is checked. Code:
`crates/dragonlink-math/src/`. All quantities are properties of the configuration analysed.

## Preference order

1. **Exact, in closed form or by finite recursion**, from the configuration's distributions.
2. **Exact by enumeration** of every reel-stop combination, when there are few enough.
3. **Seeded simulation with a stated interval**, for what the first two cannot give.

A simulated figure is never reported without its interval, and never replaces an exact one.

## Assumptions of the current model

These are the generic rules of `dragonlink-core`. Each is a place where a real game may differ, and
each difference would change the method, not just a number:

- every stop of a reel is equally likely, and reels are independent;
- lines pay left to right; a wild substitutes for regular symbols only; one win per line;
- an orb's value is drawn independently of everything else;
- in the feature, each empty position lands an orb independently with one fixed chance;
- nothing is carried from one spin to the next.

## Line pays — exact

The symbol showing at one row of a reel is distributed as the strip's symbol frequencies, and reels
are independent. So for one line

```
E[pay] = Σ over symbol combinations (s₁ … s_R) of  Π_i count_i(s_i) / Π_i length_i  ×  pay(s₁ … s_R)
```

a sum of at most `symbols^reels` terms with integer weights: an exact rational, reported as
numerator and denominator. Every line has the same expectation, and a line award is
`pay × credits per line`, so the line share of return equals `E[pay]`.

The pay of a combination is evaluated by the simulation's own rule function, so analysis and
simulation cannot disagree about what a wild does.

## Scatter pays and orb counts — exact

For one reel, counting the symbol in the window at each stop gives a distribution over 0…rows. The
window total is the convolution over reels. Scatter return is `Σ P(count) × multiple(count)`.
The same convolution for orbs gives the distribution of orbs after a spin, and so the feature's
trigger probability `P(orbs ≥ trigger)` and the distribution of its starting count.

## Hold & Spin — exact recursion

State: `e` empty positions, `r` respins left. One respin lands `j` of the `e` with probability
`C(e,j) pʲ (1−p)^(e−j)`. With no landing the state becomes `(e, r−1)`; with `j ≥ 1` it becomes
`(e−j, R)` if the counter resets, else `(e−j, r−1)`. The feature ends at `r = 0` or `e = 0`.
Every transition lowers `e` or `r`, so the distribution of the final number of empty positions and
the expected number of respins follow from a finite recursion over `(e, r)`. It is evaluated in
double precision; with at most a few hundred states the rounding error is far below reporting
precision.

From it: the distribution of orbs held at the end, the probability of a full grid, the expected
feature award (expected final orbs × mean orb value, plus the full-grid award), and the expected
number of awards of each jackpot level per spin.

## Hit frequency and base-game variance — enumeration

Whether a spin pays anything depends on all lines jointly, so it is found by building the window
for every stop combination and evaluating it with the simulation's evaluator. Done when the number
of combinations is at most the limit (default 50 million). The same pass gives the mean, variance
and maximum of the base-game award.

## Whole-spin variance — simulation

The spread of a whole spin's award, feature included, is estimated by simulation today and reported
with the run's seed and size. (An exact value is obtainable from the feature recursion and is
tracked as future work.)

## Simulation

`dragonlink-math::sim` plays the real state machine from a seed. It reports the mean award with a
normal-approximation 95% interval, proportions with Wilson intervals, and the component shares.
Rare large awards make the interval on the mean optimistic for short runs; the report says so.

The generator is the project's test generator. It is not, and is not claimed to resemble, any
original machine's.

## Progressive expected value

```
return(meters) = return from fixed pays + Σ_levels  awards per spin × meter value / total bet
break-even(level) = (1 − return from everything else) × total bet / awards per spin
```

Assumes award probabilities do not depend on the meters and that a meter pays its displayed value.
It is the long-run value of one spin; it says nothing about variance. If evidence shows that
probabilities, eligibility or the bet required depend on state, that dependence must be modelled
first.

## Comparison

A structural diff of two configurations by constant group, the relation between their pay tables
(identical, scaled by one factor, or different), whether anything affecting probabilities differs,
and the main quantities side by side.

## Checks on the method itself

Run by `cargo test -p dragonlink-math`:

| Check | What it shows |
|---|---|
| A one-reel game against hand arithmetic | the line sum and enumeration are right in the simplest case |
| Line + scatter return from frequencies equals the enumerated base-game mean, to 1e-12 | two independent routes agree |
| The feature recursion against hand-computed cases, with and without reset | the recursion is right |
| Exact return, trigger frequency and component shares against a 400,000-spin simulation | the analysis and the state machine agree |
| At the computed break-even meter the return is exactly 1 | the EV algebra is consistent |

These check our mathematics against our implementation. Agreement with an original game is a
separate matter and needs a recovered configuration and captures.
