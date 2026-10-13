# Blockers

Genuine external blockers only: things that cannot be resolved by research, experiment or more
work, and need a person, a purchase, hardware or material nobody on the project holds. Ordinary
unfinished work is in `progress/progress.toml`, not here.

A blocker stops one lane, not the project. Each entry says what proceeds meanwhile.

## Open

### B-001 — No working GitHub login on the Windows host

- **Since:** 2026-10-13
- **Blocks:** pushing to `bailo167/dragonlink-decomp`; CI ever running; anything that needs the
  remote (pull requests, releases).
- **What was found:** the access token configured in the environment is rejected by GitHub
  (HTTP 401); no HTTPS credential is stored for the host; the SSH key present is not readable by the
  session. Reading works because the repository is public.
- **Needs:** the owner to log in once (`gh auth login`, then `gh auth setup-git`) after clearing
  the stale token variable.
- **Meanwhile:** all work is committed locally with full history and passes the Windows gate before
  each merge to the local `main`. It pushes as-is once a login exists.

### B-002 — No cabinet software or mathematics source

- **Since:** project start
- **Blocks:** every item about original mathematics (reel strips, weighting, RTP, jackpot odds,
  feature probabilities, the outcome generator), and cabinet-specific assets and timing.
- **Needs:** lawfully possessed cabinet software or machine images, or regulatory documentation
  that states the mathematics. Not obtainable by the project on its own within its rules.
- **Meanwhile:** the engine is developed on a labelled synthetic fixture; presentation research
  proceeds from official media and, when obtained, official mobile software; the cabinet workflow
  is designed so that a source slots in ([SOURCES.md](SOURCES.md)).

## Resolved

None yet.
