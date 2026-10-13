# Game configuration files

A configuration is the machine-readable description of **one** game, version and platform: a
complete math model, the provenance of every constant in it, and what is known about how the
implementation behaves. It is what `dragonlink-analyze` loads.

Schema `dragonlink-config/1`, JSON. Defined in `crates/dragonlink-math/src/config.rs` and
`crates/dragonlink-model/src/math.rs`; unknown fields are rejected.

```sh
dragonlink-analyze export-fixture --out fixture.json     # the synthetic fixture, as a template
```

## Structure

| Section | Content |
|---|---|
| `schema` | `"dragonlink-config/1"` |
| `identity` | `name`, and where known: `title`, `market` (`cabinet-au`, `cabinet-us`, `mobile`), `platform`, `program_version`, `specification` (the regulatory specification number), `denomination`, `notes`. |
| `model` | The math model: `id`, `provenance`, `grid`, `symbols`, `reel_strips`, `paylines`, `line_pays`, `scatter_pays`, `bet`, `hold_and_spin` (trigger count, initial respins, reset rule, land chance, value table, full-grid award, jackpot levels). |
| `constants` | One provenance record per constant group: `path`, `provenance`, `source`, `note` ([PROVENANCE.md](PROVENANCE.md)). |
| `behaviour` | Sourced findings: `rng_architecture`, `outcome_selection`, `reel_stop_presentation`, `input_timing`, `spin_independence`, `persistent_state`. All unknown by default. |

One configuration is one denomination and one program version. Denomination effects, bet effects
and version differences are found by comparing configurations, not by parameters inside one.

## Where configurations live

- **The synthetic fixture** is built into the tools (`synthetic`) and may be exported anywhere.
- **A recovered configuration is proprietary material.** It lives in the private source root under
  `math/<title>/<market>-<specification>-<denomination>.json`, never in the repository
  ([../../DECISIONS.md](../../DECISIONS.md), D-010). The repository's ignore rules and hygiene tool
  do not know one JSON file from another, so this rests on where the file is put; the analysis
  tool additionally refuses to write its output about a non-synthetic configuration into the
  repository.

## What the model cannot express yet

Tracked in `progress/progress.toml` under "Mathematics and analysis". A configuration for a real
game will need most of these, and the format will grow as evidence shows what the games do:

- free games and their reels;
- separate reels or weights for the Hold & Spin feature (the current model uses one chance per
  empty position);
- virtual-reel weighting distinct from strip position (today every stop is equally likely);
- progressive meters: start values, increments, contribution rates, resets;
- jackpot values that are not multiples of the bet;
- pay tables or reels that change with the bet rather than only with the denomination;
- ways rather than lines;
- any state carried between spins.

The format is extended when evidence requires it, not in anticipation.
