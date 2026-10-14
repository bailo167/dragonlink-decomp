# Golden Century: presentation, timing and sound, as measured

What the real game shows and plays, measured frame by frame and sound by sound, and what this
project's app does with it. Measurements and descriptions are ours; no game material is reproduced.

**Sources.**

- **Official mobile app (gameplay recording).** The owner recorded the official mobile app with
  sound: a 3:41 session and six excerpts of it (base wins, a stacked wild, the free games' trigger
  and rounds, the Hold & Spin trigger, its last respins and its payout), 960 x 442 at 30 fps, clean
  digital sound. The recordings are held privately in the source root and never committed. This is
  the reference for **gameplay presentation** (owner, 2026-10-14). It is not evidence for cabinet
  mathematics (CLAUDE.md rule 4).
- **Cabinet footage (gameplay recordings).** Public recordings of Australian cabinets
  ([FOOTAGE.md](FOOTAGE.md), [golden-century-sound-cues.md](golden-century-sound-cues.md)).

**Method.** Frame times from per-region frame differences and vertical-shift tracking. Sounds
identified by waveform cross-correlation of the game's own clips (from the owner's mobile build)
against the recording, with control windows; true matches score 0.95-0.999 on the app's clean
sound. Each topic was measured by one agent and re-measured by an independent verifier who tried to
refute every claim. The recordings' sound lags the picture by about 0.08-0.10 s; times below have
that removed where it matters.

Labels: **VERIFIED** (measured directly, reproducible from the recording), **STRONG**, **PARTIAL**,
**UNKNOWN** ([../EVIDENCE.md](../EVIDENCE.md)).

## Reels

| What | Real app | Label |
|---|---|---|
| Start | All five reels move the frame after the press, at full speed at once; no kick | VERIFIED |
| Speed | Half a symbol a frame (15 symbols/s), constant to the stop | VERIFIED |
| Stops | Reel 1 is deepest past its stop 24 frames after the press; each further reel 14 frames (0.467 s) later, left to right, every spin | VERIFIED (17 spins) |
| Landing | Runs on at full speed up to a whole symbol past its stop, shows there a frame, snaps back to 0.36 of a symbol past it, slides home in a straight line in 9 frames | VERIFIED |
| Anticipation | None seen, even with 6 coins or 2 flags showing | VERIFIED |
| Stop sound | One clink sample (the five reel-clink files are the same sound), at the snap; panned left to right by reel | VERIFIED |
| Result | Nothing changes until 4 frames after the last reel is still; the result shows then; the next spin is taken a frame later | VERIFIED |
| Coins landing | A reel landing coins while six can still be reached plays SND_JPDoonkN (N counting such reels), else the clink | VERIFIED |

**The app:** same speed, cadence and landing shape, every reel within a frame of the real timing;
results wait for the reels to be still; a spin is refused until then.

## Wins

| What | Real app | Label |
|---|---|---|
| Win tune | A base win plays a win tune: 0.4x the bet wintune04, 0.6x wintune05, 1.6x wintune13, 2x wintune14 | VERIFIED (4 wins); rule between UNKNOWN |
| Count | Linear, for exactly the tune's audible length (its file has a long quiet tail) | VERIFIED |
| Free games | No tune; the count is silent under the music, same length rule | STRONG |
| Lines | Each winning line 43-45 frames in turn; its badges light twice; its winning symbols that do not animate blink out twice | VERIFIED |
| Stacked wild | Two or three stacked emperors become the tall emperor only on a winning line, when the win would show, to SND_Wild_2 or SND_Wild_3 by height; the win waits 2.5 s; the animation runs at 20 fps. A stack on no winning line stays as heads | VERIFIED (2 cases) |

## Free games

| Step | Real app | Label |
|---|---|---|
| Trigger | Flags land with the clink only; the feature bell 0.2 s after the reels are still, played out (2.55 s) | VERIFIED |
| Flags | As the bell ends the flags wave (their animation from its first frame, at 20 fps) to SND_Scatter (4.75 s) | VERIFIED |
| Scatter win | Counts when SND_Scatter ends, to its win tune | VERIFIED |
| Prompt | SELECT SPIN TO START FEATURE after the count; "Scatter wins N" only while it waits | VERIFIED |
| Start | The press is silent; the flags wave on about 1 s; a hard cut to the free games: SND_Fea_BGM from its start, the first game spinning at once, the banner's total counting up from 1 with a SND_Fea_Inc tick a step | VERIFIED |
| Doors | Land with the reels, open together 3 frames after the reels are still (1.4 s), to SND_EndMask; wins 47 frames after they begin to open | VERIFIED (6 games) |
| Pace | Next game 1.25 s after a win's count; about 2.5 s after the doors with no win | VERIFIED |
| End | FEATURE COMPLETED in the win slot, SND_Fea_End as the music stops | VERIFIED |

## Hold & Spin

| Step | Real app | Label |
|---|---|---|
| Trigger | Feature bell as the reels settle; 2.8 s later every coin bursts into flame and the other positions go dark, to SND_FeatureTrigger | VERIFIED |
| Fire | Fire dragons rise and fill the screen (the fire animation at about 25 fps), clearing to the HOLD & SPIN medallion over the darkened board | VERIFIED |
| Medallion | 3.8 s, then "COLLECT 15 TO WIN GRAND JACKPOT" for 2 s, then a second fire to the board | VERIFIED |
| Owner's change | The app shows the medallion for about 1 s and leaves the COLLECT screen out | owner |
| Board | SELECT SPIN TO START FEATURE and SND_FreeSpin_BGM 0.57 s after the board clears; the press counts 1, 2, 3 (SND_Fea_Inc), a burst on the 3; the first respin starts by itself 1.9 s later | VERIFIED |
| Respins | Open positions of a reel stop together: reel 1 0.53 s after the press, each further reel 0.467 s later whether or not it has open positions; half a cell overshoot, settled in 11 frames; SND_Button at each start | VERIFIED |
| Respin sounds | Every reel with positions spinning clinks; a new coin plays SND_HandSDoonk1 and the respin's first coin also SND_GrandStrike | VERIFIED |
| Counter | Drops as a respin starts; back to 3 six frames after a coin lands, its burst a frame later; at 0 the banner is gone until a coin lands | VERIFIED |
| End | FREE SPINS COMPLETED with SND_Fea_End (3 s) | VERIFIED |

## Hold & Spin payout

| Step | Real app | Label |
|---|---|---|
| Rumble | SND_Rumble 0.37 s after FREE SPINS COMPLETED ends; fades out over the first coin | VERIFIED |
| Banner | An empty WIN banner over the reels 2.86 s after it | VERIFIED |
| Coins | Reel by reel, top to bottom, 1.35 s apart: the coin's fire comet (the game's bolt animations) flies to the banner, which adds the value at once; SND_Small_5 per coin, SND_Medium_1 for a 10x coin | VERIFIED (11 coins); the size boundary UNKNOWN |
| Count | 0.87 s after the last value: a coin shower and the payout music while the win counts up linearly over the music's length (62.9 s); a press completes it and stops the music | VERIFIED |
| Letters | The banner's WIN letters burst every 59 frames from 0.8 s after the last value | VERIFIED |
| Music | Not among the game's own clips: one of three tracks in the app's shared sound bank. The same tracks play under the cabinet's coin-shower counts; across one cabinet session they ran in turn | VERIFIED (app); STRONG (cabinet, rotation) |

**The app** follows the free games, Hold & Spin and payout sequences above at the measured times,
with the owner's two changes. It keeps the cabinet's own meters, button deck and gamble (owner).

## The other clips

Every clip of the game's sound bank that the app did not yet play was searched for across the app
recording and the cabinet footage (waveform and spectral-peak matching against chance, frames
checked at each hit), and looked up in the game's scene files. Only what footage or the scene files
show is used here.

| Clip | Where it plays | Label |
|---|---|---|
| SND_Wild_1..5 | One of them as a shown win through an emperor (stacked or single) begins to animate; the variant changes from win to win (one 3-high stack sounded Wild_3, Wild_4 and Wild_4 on different spins; a single sounded Wild_3), so it is not the stack's height. On the cabinet the next game cuts it | VERIFIED (12 plays) |
| SND_Medium_1..5, SND_Credit1/3 | Each family decodes to one identical sound. The cabinet ticks every value coin it collects with Medium (1x-10x the bet) and MINI/MINOR coins, a 20x coin and the GRAND plaque with Credit; the app ticks coins under 5x with SND_Small_5 | VERIFIED |
| SND_GrandStrike | Also about four times during the cabinet's Grand count; what times it is unknown | STRONG |
| betbeep1/3_reduced | The cabinet's x1 and x2 bet buttons (G3 and C4 beeps, as the bet changes on screen); x5 and x10 use beeps from the app's shared bank | VERIFIED |
| GenericButton | The help screen's page buttons (scene file) | VERIFIED (scene; unheard) |
| SND_Strike, BonusRollupTerm | Never heard in the game's flow | STRONG |
| Others | anticipation1..5, SND_Pic1..4, SND_LargePrize, SND_GrandBell, SND_JackpotFanfare, SND_DrumRoll_*, SND_WindUp, BonusRollup, SND_Major, SND_Credit2/4/5/Small, the ST_ and reel/whirring clips: no placement shown by footage | UNKNOWN |

Recorded but not followed here: on the cabinet a Grand is shown and counted before the coins; the
owner chose the coins first, then the Grand.

## Not known

- The rule choosing the win tune between and beyond the measured win sizes.
- The size above which a collected coin sounds SND_Medium_1.
- The flame plumes beside the medallion: not matched to any animation in the game's files.
- Whether the payout tracks rotate or are chosen by win size.
