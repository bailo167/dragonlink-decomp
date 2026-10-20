# The seven later titles against their own club footage (2026-10-20)

Golden Century was audited against its own club-cabinet footage on 2026-10-18/19
([GOLDEN_CENTURY_PRESENTATION.md](GOLDEN_CENTURY_PRESENTATION.md)). The other seven titles
(Autumn Moon, Happy & Prosperous, Panda Magic, Genghis Khan, Golden Gong, Peace & Long Life,
Spring Festival) shared that presentation and had never been compared with their own footage;
their sounds had only been matched by file name. This pass compared each with its own Australian
club footage, screen by screen, and checked where each title's own clips play on recordings with
sound.

Labels and source classes follow [EVIDENCE.md](../EVIDENCE.md). Every finding was found by one
reader and then re-measured by a second, refute-first, on other moments and, wherever possible,
other recordings. Claims are in `crates/dragonlink-model/claims/` (the `2026-10-20` entries); the
detailed private notes, frame lists and measurements are in the source root
(`research/side-audit-2026-10-20/`). Source class: gameplay recording (club cabinets, Queensland
clubs unless said; one NSW hotel for Spring Festival) unless said. Nothing from the recordings is
in Git.

## Method

- Footage: about 40 public recordings fetched at 1080p with their original sound into the private
  source root; frames cut with their real video time, checked against single-frame cuts.
- Our build: muted unattended captures of every title (`DRAGONLINK_MUTE=1`,
  `DRAGONLINK_FEATURE_TEST`, `DRAGONLINK_CAPTURE`), timed from the log's save stamps (with a pack
  the build runs on the real clock, so a frame number is not a time).
- Sound: each title's own clips (and the other titles' same-named clips as controls) matched in
  the recordings by cross-correlation, with a null distribution, a segment-lag test, a reversed
  clip and, for absences, injection of the clip at the recording's own level; every hit tied to
  an on-screen event.

## Family-wide findings (now in the shared presentation)

| Finding | Label | Evidence |
|---|---|---|
| The trigger's wave lasts the title's own SND_Scatter + 0.12 s; then the scatter win counts with PRESS START FEATURE | STRONG | Genghis Khan ten triggers on five recordings (6.20-6.25 s against 6.11 s); Panda Magic, Golden Gong, Peace & Long Life, Golden Century |
| A retrigger replays the trigger without a press: "N FREE GAMES WON!", the bell, the wave with "Scatter wins N", the banner counting up from the old total, then the win | VERIFIED | five titles frame by frame |
| A wild win shows its lines at once; only WIN's count waits, the title's wild clip + 0.2 s | STRONG | Panda Magic, Autumn Moon, Golden Century |
| Only stacks on a winning line stand; a stack stands only where the pack has tall art (others animate cell by cell) | STRONG / VERIFIED | Genghis Khan; Panda Magic and Happy & Prosperous |
| Every wild in the game's wins animates through the whole line cycle | STRONG | Genghis Khan, Panda Magic |
| Video-texture animations play at 20 fps (their pack's `fps.txt`), image sequences at 30 | STRONG | official app and club cabinets (Golden Century's pictures, Happy & Prosperous's jade, the x2 badge) |
| The coin shower from 25x the bet; a free game's big-win plaque burns every 2.0 s under it | STRONG (threshold PARTIAL) | Autumn Moon, Golden Century |
| The Hold & Spin counter bursts on every return to 3 | VERIFIED | Panda Magic, Golden Century |
| Tall wilds in each title's own scene rect (Golden Century's unchanged) | VERIFIED | scene computation; Peace & Long Life, Golden Gong footage |
| A revealed run of wilds stands tall | STRONG | Autumn Moon, Golden Century |
| The first free game's reels stop later: Golden Century 1.79 s, Spring Festival 2.60 s | VERIFIED (others HYPOTHESIS: Golden Century's) | two recordings each |
| End stings: FREE SPINS COMPLETED the 3.0 s everywhere; FEATURE COMPLETED per title (below) | VERIFIED | 42 recordings with sound |
| No SND_EndMask at doors or moon gates | STRONG | Autumn Moon 19 openings, Golden Century 5 |
| In three titles' free games a picture win of three or more (wilds allowed) sounds the picture | STRONG (two-of-its-own rule HYPOTHESIS) | Autumn Moon, Panda Magic, Happy & Prosperous |

## Each title's own

Per-title values live in one table, `apps/dragonlink/src/looks.rs`, or come from the title's pack
and model; the shared presentation names no title.

- **Autumn Moon.** Purple chooser keys (VERIFIED); the moon gate opens at 30 fps (STRONG);
  FEATURE COMPLETED's 3.0 s sting; picture sounds in the free games.
- **Happy & Prosperous.** The Buddha keeps its lotus tile with falling coins under the figure;
  runs animate cell by cell (STRONG); picture runs on the free reels (look STRONG, strips
  synthetic, line return unchanged); FEATURE COMPLETED's 2.0 s sting (VERIFIED; the clip is Golden
  Gong's).
- **Panda Magic.** Stacked pandas animate in their own cells (VERIFIED); green chooser keys,
  gold $1 and header; the FREE GAME n OF m numbers placed on its own banner (VERIFIED); no
  FEATURE COMPLETED sting (STRONG).
- **Genghis Khan.** "Line N wins A x M = B" for x2 wins (VERIFIED); the winning x2 wild plays the
  Khan's animation under its pulsing badge (STRONG); green chooser keys; gold START FEATURE
  lettering (STRONG); its own 2.2 s FEATURE COMPLETED sting, Golden Century's 3.0 s at FREE SPINS
  COMPLETED.
- **Golden Gong.** The monk grows into its reel from the rest (win and no-win sequences at 20
  fps; full at + 0.7 s), the lines at + 1.5 s and the count after the expansion's sound
  (VERIFIED, twelve events); the standing monk scrolls out whole at the next spin (HYPOTHESIS;
  ours flashed three heads); the expansion's one sound, SND_Wild in a win (STRONG); live "N LINES"
  tabs on its tab-free frame (VERIFIED); its symbols' sizes (STRONG, some PARTIAL); green keys.
- **Peace & Long Life.** A retrigger adds six games (VERIFIED as displayed, one case); the sage
  fills its cell and the tall sage its own rect (VERIFIED); the x2 wild is silent (STRONG); green
  keys, gold lettering; the gamble's orchard dimmed (PARTIAL); the 2.0 s FEATURE COMPLETED sting.
- **Spring Festival.** The BIG reel spins as one reel of whole BIG pictures, 4.6 a second, with
  the small reels' overshoot and slide, the first free game scrolling the base window's middle
  columns out (VERIFIED, 12 spins, two mosaics); the BIG free games' slower pace and stops
  (VERIFIED); SND_WindUp as a BIG wild lands, reel 5 held about a second (VERIFIED / STRONG); the
  three-window panel; the BIG coin's size, burning rim, digits (3.3x) and dark backing, one ring,
  one burst and the free games' board (VERIFIED); passing BIG coins lettered (VERIFIED); the BIG pictures' own animations; gold frames tracing the shown
  line over a BIG symbol (VERIFIED; colours PARTIAL); cyan-blue keys; no FEATURE COMPLETED sting;
  the first base spin after the feature scrolls the last BIG picture out whole (VERIFIED, one
  feature end; ours scrolled out nine small copies).

## Golden Century, changed by this pass

Golden Century shares the presentation, so the family-wide findings moved it too, each measured
on Golden Century's own club footage or applied as the same framework's rule:

- the wave 4.87 s (its club audit's 4.87 s; the official app's 4.75 s clip + 0.12 s);
- its pictures, flags and wilds at 20 fps (official app and club cabinet; it played them at 30);
- the first free game's reels 1.79 s later (VERIFIED, two recordings);
- FEATURE COMPLETED's sting is Genghis Khan's 2.2 s on its club cabinet (STRONG, one recording),
  where the official app plays its own 3.0 s: a conflict recorded, the cabinet followed;
- no SND_EndMask at the doors (STRONG); its lines show while WIN waits for a stand (STRONG);
- the coin shower from 25x (its 20x and 22x wins had none on the cabinet); the counter's burst on
  every reset (VERIFIED).

## Not done, or not known

- Chooser splashes and logos: no title's pack holds the art (asset gap, not drawn).
- Spring Festival: coins in the first free game's leaving block lose their prizes as they scroll
  out; the line frames are about 6 canvas px against the cabinet's 8.5.
- After a retrigger (Peace & Long Life, Spring Festival) the next free game starts about a second
  later than the one filmed case of each; Golden Gong's "1 LINE" flash after a denomination change
  is not drawn (no unattended run reaches it).
- Peace & Long Life's thick key-pattern dividers (own art only); Genghis Khan's and Peace & Long
  Life's brush lettering (no face in the packs); Happy & Prosperous's still-wild draw order (low).
- Golden Gong's and the later titles' base strips cannot land a full-reel stack (synthetic strips
  inherit Golden Century's blocks of two; a maths change, not made).
- UNKNOWN: the faint sting at Autumn Moon's moon gates and Golden Century's doors; the first free
  game's longer spin on the five titles not measured; what an orb on an expanding reel does;
  whether a one-cell stack stands; Peace & Long Life's second retrigger (one case only).
