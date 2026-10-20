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

**The cabinet differs** (60 fps cabinet footage, wEmf3ep-euA 12 landings and 8 spins; BMavaGxuOD0):
the reels spin at 20 symbols a second, reel 1 comes to rest about 0.95 s after the reels start,
each further reel 0.45 s later; a reel runs through its stop to about 0.62-0.66 of a symbol for one
refresh, is at 0.40 the next frame and slides home in a straight line in 8 frames at 30 fps. It
never shows a frame a whole symbol past, and it draws moving symbols crisp (doubled images in some
recordings are the camera's exposure). STRONG (two cabinets).

**The app** follows the cabinet here (the owner plays cabinets; the app's whole-symbol dip read as a
jolt): 20 symbols a second, reels 0.45 s apart, deepest 0.65 then 0.40, an 8-frame slide, no smear;
measured on a muted 60 fps capture of the app. Results wait for the reels to be still; a spin is
refused until then.

## Wins

| What | Real app | Label |
|---|---|---|
| Win tune | A base win plays a win tune: 0.4x the bet wintune04, 0.6x wintune05, 1.6x wintune13, 2x wintune14 | VERIFIED (4 wins); rule between UNKNOWN |
| Count | Linear, for exactly the tune's audible length (its file has a long quiet tail) | VERIFIED |
| Free games | No tune; the count is silent under the music, same length rule | STRONG |
| Lines | Each winning line 43-45 frames in turn; its badges light twice; its winning symbols that do not animate blink out twice (uneven, with gaps; the cabinet's differ, below) | VERIFIED (the mobile app only) |
| Stacked wild | Two or three stacked emperors become the tall emperor only on a winning line, when the win would show, to SND_Wild_2 or SND_Wild_3 by height; the win waits 2.5 s; the animation runs at 20 fps. A stack on no winning line stays as heads | VERIFIED (2 cases) |

**The cabinet's line pieces differ** (club footage rectified onto our canvas: `CAM8vranI9Y`,
`y8c8lEdPW8Y`, `f5sMl9a5jds`). The badge is sized to the denomination's line configuration: about
37x22 at 50 lines, 97x43 at 25 and 100x44 at 5 (STRONG). A line has one badge at 50 lines, one at
25 lines except line 1, and both ends only at 5 lines (STRONG, 40 line wins); which end is fixed per
line and differs between the 50- and 25-line maps (VERIFIED for the lines filmed; no rule fits).
The badge sits outside the reel frame touching it, with a stub of its colour across the frame. The
app draws them so ([GOLDEN_CENTURY_CABINET_DECK.md](GOLDEN_CENTURY_CABINET_DECK.md), "Win line
pieces"); the mobile app's two large badges per line are gone.

**The cabinet's blink differs** (club footage, frame by frame: `y8c8lEdPW8Y` 21.93-24.23 s, one line,
badge lit 9-10 frames then off 9-10 frames, cycles of 19-20 frames; `CAM8vranI9Y`, its 0:04-0:44
section, a four-line win whose cells blank every 0.66 s on average and whose message moves to the
next line every 1.333 s). Each line shows for two even cycles of about 0.66 s, each starting lit;
its badge and dashes are lit exactly while its winning symbols show, and the symbols blink out
exactly while the badge is off, with no gap and no overlap; a line lasts 1.29-1.33 s. STRONG (two
Queensland club machines). **The app** follows the cabinet: 1.32 s a line, lit 0-0.34 s and
0.66-0.99 s, the symbols out in between.

## Free games

| Step | Real app | Label |
|---|---|---|
| Trigger | Flags land with the clink only; the feature bell 0.2 s after the reels are still, played out (2.55 s) | VERIFIED |
| Flags | As the bell ends the flags wave (their animation from its first frame, at 20 fps) to SND_Scatter (4.75 s) | VERIFIED |
| Scatter win | Counts when SND_Scatter ends, to its win tune | VERIFIED |
| Prompt | SELECT SPIN TO START FEATURE after the count (the official app's wording and timing; the cabinet's differ, below); "Scatter wins N" only while it waits | VERIFIED |
| Start | The press is silent; the flags wave on about 1 s; a hard cut to the free games: SND_Fea_BGM from its start, the first game spinning at once, the banner's total counting up from 1, 4 frames a step, with a SND_Fea_Inc tick a step (the cabinet is quicker and stiller, and counts from 0, below) | VERIFIED |
| Doors | Land with the reels, open together 3 frames after the reels are still (1.4 s), to SND_EndMask; wins 47 frames after they begin to open (the cabinet's are quicker, below) | VERIFIED (6 games; the mobile app only) |
| Pace | Next game 1.25 s after a win's count; about 2.5 s after the doors with no win (the cabinet's 2.0 s after the rest, below) | VERIFIED (the mobile app only for the no-win pace) |
| End | FEATURE COMPLETED in the win slot, SND_Fea_End as the music stops (on the cabinet as the last count ends, below) | VERIFIED |

**The cabinet's doors and pace differ** (motion per door cell at 30 fps, real-time recordings:
`1bAevGz0K2k` free game 3, 346.57 s rest; `y8c8lEdPW8Y` free games 4-6, 1023.18, 1033.67 and
1041.70 s rest; a coarser pass over `1bAevGz0K2k` free games 1, 2, 4, 5 and 6). The doors begin to
open 0.22-0.37 s after the last reel rests (three of four about 0.30 s), their motion visible for
0.60-0.73 s and over by the rest + 0.93-0.97 s: STRONG. The wins (the first badge and dashes) show
at the rest + 1.47-1.53 s (`y8c8lEdPW8Y` 1024.67, 1035.2, 1043.2): PARTIAL (three cases, one
machine). A free game with doors and no win is followed by the next game's reels at the rest +
1.97-2.0 s, 1.03-1.07 s after the doors stop (`1bAevGz0K2k` 333.67-335.63, 346.57-348.57,
359.60-361.60; `y8c8lEdPW8Y` 954.90-956.87, WIN unchanged in all four): STRONG; with no doors and
no win 0.5 s after the rest (`y8c8lEdPW8Y` 959.63-960.13): HYPOTHESIS, one case. **The app** follows
the cabinet: the doors 0.30 s after the rest, the pack's 42-frame sequence at 42 fps (its motion,
frames 1-29, over by the rest + 0.99 s), the wins at the rest + 1.5 s, and a no-win free game's
next reels 0.5 s after the reveal ends (rest + 2.0 s, measured on a muted capture of the doors run:
2.00 s, 2.14 s when a landed coin's value follows the reveal); with no doors about 0.67 s. Golden
Gong's monk keeps the app's 3-frame delay (its cabinet timing unmeasured). A win's pace (1.25 s
after its count) stays the app's.

**The cabinet's end differs** (club footage, every frame: `y8c8lEdPW8Y` 1044.0-1047.1;
`1bAevGz0K2k` 366.9-370.1 at 15 fps). FEATURE COMPLETED comes the frame after WIN reaches the
feature's total (the last game's count reaches $320.20 at 1044.233, the banner 1044.267; WIN and the
banner together at 367.567); the status reads "Feature Completed" for one frame, then "PLAY ON,
GAMBLE or TAKE WIN" with the meters switching every second; CREDIT rises only when the win is taken
(1045.933; 368.300), with a 2-frame "Feature Completed" then the bet line; the banner stays until the
next spin. STRONG (two Queensland club recordings); the one-frame status VERIFIED at 30 fps. **The
app** follows it: FEATURE COMPLETED waits for the last count, the status reads "Feature Completed"
while the feature's last event is still queued (one event interval, 0.14 s, longer than the
cabinet's frame) and then the offer, and CREDIT holds the total off until the win is taken (it
flickered to the total for 0.13 s before). The 2-frame flash after the take is not drawn.

**The cabinet's prompt differs** (club footage, both features): the banner reads PRESS START
FEATURE BUTTON on one line, PRESS and BUTTON small caps (0.81 of START FEATURE's cap height) on the
same baseline, the line across 0.96 of the blue field, START FEATURE's caps 0.42 of its height, in
the brush face of the app's lettering but pale cream, not gold. Only the lettering blinks, on a
0.667 s cycle (20 video frames in every cycle measured), about 0.35 s lit, the first lit phase
about 0.25 s as the banner pops in; the bar stays. VERIFIED (y8c8lEdPW8Y 969.10-970.53 and
2519.57-2521.77; 1bAevGz0K2k 113.77-115.37; free games y8c8lEdPW8Y 945.8-949.8). The status line
reads "Press START FEATURE button" on both. **The app** follows the cabinet for both features: our
own lettering over the banner's bar (the pack has no such art; Arial Black stands in for the brush
face, the cream judged by eye against white text, not measured as a colour), blinking from the
moment the prompt shows.

**The cabinet's trigger and start differ from the app** (club footage, frame by frame:
`y8c8lEdPW8Y` 938.43-951.6 and `1bAevGz0K2k` 319.2-330.3; the two agree; VERIFIED for these club
machines):

- **Status and message rows.** From the result (0.14 s after the reels rest) the status line reads
  "6 FREE GAMES WON!", steady; the message row keeps its start prompt until the flags wave (2.6 s
  later), then reads "Scatter wins N" through the count and the wait. The official app shows
  "Scatter wins N" only while it waits.
- **The prompt comes with the count.** PRESS START FEATURE BUTTON and "Press START FEATURE
  button" appear within one frame of the scatter win's first count step (`y8c8lEdPW8Y` 946.07,
  `1bAevGz0K2k` 326.77-326.80), 7.6 s after the reels rest; the count goes on under the prompt.
  The app's official recording asks after the count. While it waits the status alternates in
  2.0 s slots from the prompt's start, "Press START FEATURE button" first (791.67-793.70 and
  946.10-948.08 on `y8c8lEdPW8Y`; one case, 946.03, showed the other first after a single frame).
  The lettering's blink restarts, lit, the moment the count completes (948.50 and 793.80,
  `1bAevGz0K2k` 327.77, each a press that completed the count).
- **The start.** The symbols go still at the press (within 0.05-0.07 s), nothing moves, and the
  first free game's reels start 0.50-0.53 s after the prompt goes (`y8c8lEdPW8Y` 949.80 to
  950.33, `1bAevGz0K2k` 328.60 to 329.10), with "FREE GAME 1 OF" and its counting total 2 frames
  after the first motion. The status and message rows are blank from the press; the status shows
  "Free Game 1 of 6" about 1.2-1.3 s after the reels start.
- **The counter.** The banner reads "FREE GAME 1 OF 0" for 5 frames, then its total steps 1 to 6,
  one step every 5 video frames (0.167 s); the status line stays blank until 5 frames after the 6,
  then reads "Free Game 1 of 6" (`y8c8lEdPW8Y` 950.400-951.567, `1bAevGz0K2k` 329.167-330.333,
  the cadence identical; "1 OF 0" also on `BMavaGxuOD0`, a US casino). VERIFIED. The official app
  counts from 1, 4 frames a step.
- **A free game's WIN banner only for big wins.** "WIN $x WIN" replaces "FREE GAME n OF 6" only
  for a win of 11x the bet or more (seen at 11.0x, 12.8x, 16x, 20x, 22x twice, 40x and 44x); 18
  wins from 0.1x to 6.0x counted on the WIN meter under "FREE GAME n OF 6" and its counter
  (`1bAevGz0K2k`, `y8c8lEdPW8Y`, `f5sMl9a5jds`, `kVjwYtVwhkk`; 26 paid free games in five
  features). The rule STRONG; the limit lies above 6.0x and at most 11.0x (a fixed dollar limit
  of $25.50 to $32 also fits). A big win's banner stays 1.0 s after its count, then "FREE GAME n
  OF 6" returns until the next game (three cases ended by a press).
- **The return after a Hold & Spin inside the free games.** The WIN plaque goes; the board stays
  0.37-0.43 s with its open positions dark and no banner, the status still "Free Game N of 6";
  then "FREE GAME N+1 OF 6", the status N+1 and the reels come together, the dimmed symbols
  scrolling away still dimmed (`y8c8lEdPW8Y` 1019.93-1020.43, 2562.93-2563.40, 3046.57-3047.00;
  VERIFIED, one machine).

**The app** follows the cabinet in all of these: the trigger's status and message rows, the
prompt at the count's first step with the count going on under it, the 2.0 s alternation from the
prompt's start (Press first) and the blink restarting as the count completes, the flags still at
the press and the first free game 0.52 s after it with its banner and total, the WIN banner for a
free game's win from **10x the bet (our choice inside the evidenced range: HYPOTHESIS)** held 1.0 s
after the count, and the dark 0.43 s return. The banner's total counts from "1 OF 0", a step every
0.167 s, and the status stays blank until one step after the total (1.17 s after the banner; it
showed "Free Game 1 of 6" as the reels started before, and counted from 1 at 0.133 s a step). As
each free game starts the last count is settled, so the WIN banner never shows the last game's
amount (our recording showed "WIN $1.00 WIN" for 4 frames before $0.01; the cabinet never does,
`y8c8lEdPW8Y` 1027.4). A Hold & Spin's payment always shows its banner, however small (`WIN $0.29
WIN`, `1bAevGz0K2k` 164.3).

## Hold & Spin

| Step | Real app | Label |
|---|---|---|
| Trigger | Feature bell as the reels settle; 2.8 s later every coin bursts into flame and the other positions go dark, to SND_FeatureTrigger | VERIFIED |
| Fire | Fire dragons rise and fill the screen (the fire animation at about 25 fps), clearing to the HOLD & SPIN medallion over the darkened board (official app only) | VERIFIED |
| Medallion | 3.8 s, then "COLLECT 15 TO WIN GRAND JACKPOT" for 2 s, then a second fire to the board (official app only) | VERIFIED |
| Owner's change | The app showed the medallion for about 1 s and left the COLLECT screen out; replaced by the cabinet's sequence (below) | owner, 2026-10-14; superseded |
| Board | SELECT SPIN TO START FEATURE and SND_FreeSpin_BGM 0.57 s after the board clears (the wording and the 0.57 s the official app's); the press counts 1, 2, 3 (SND_Fea_Inc), an orange burst on the 3; the first respin starts by itself 1.9 s later (official app only: the cabinet's 0.59 s, below) | VERIFIED |
| Respins | Open positions of a reel stop together: reel 1 0.53 s after the press, each further reel 0.467 s later whether or not it has open positions; half a cell overshoot, settled in 11 frames; SND_Button at each start | VERIFIED |
| Respin sounds | Every reel with positions spinning clinks; a new coin plays SND_HandSDoonk1 and the respin's first coin also SND_GrandStrike | VERIFIED |
| Counter | Drops as a respin starts; back to 3 six frames after a coin lands, its burst a frame later; at 0 the banner is gone until a coin lands | VERIFIED |
| End | FREE SPINS COMPLETED with SND_Fea_End (3 s) | VERIFIED |

**The cabinet's intro differs** (two club recordings, matched frame by frame against the fire's own
54 frames: y8c8lEdPW8Y 965.60-969.10 in the base game and 1bAevGz0K2k 110.17-113.67 inside free
games; they agree within 0.03-0.07 s): the coins burst 2.67-2.70 s after the reels are still; one
fire, at about 22 fps (a fit of its key frames on both, 21.5-23), its first dragon head 0.90-0.93 s
after the burst, the screen white about 1.8-2.3 s after it with the Hold & Spin board already under
it, gone but for its top band by 2.70 s; PRESS START FEATURE BUTTON 3.50 s after the burst. No
medallion over the reels (the cabinet's is on its top screen), no COLLECT screen and no second fire.
STRONG. The music's onset is not measurable: the copies have no sound. **The app** follows the
cabinet (the owner's 2026-10-18 rule that cabinet footage outranks the app), which replaces the
owner's 2026-10-14 change: the fire from 0.87 s after the burst at 22.3 fps, the board coming up
under its white, the prompt and the music at 3.50 s.

**The cabinet's board differs too** (club recordings, 30 fps, frame by frame):

- **The first respin** starts by itself 0.55-0.63 s after the counter's 3, mean 0.59 s
  (`y8c8lEdPW8Y` 970.93 to 971.53 and 2522.15 to 2522.70, `1bAevGz0K2k` 115.78 to 116.37,
  `f5sMl9a5jds` 212.67 to 213.30: four features, three recordings, two venues; VERIFIED). The
  counter drops to 2 a frame or two before the reels move. The official app waits about 1.9 s.
- **The counter's burst** on the 3 is a small white to blue-white star flash with white sparks on
  the digit for about 0.23 s, then a thin ring round it (white-blue on one camera, fire-edged on
  another), gone by 0.37-0.43 s (`y8c8lEdPW8Y` 970.967-971.30 and 2522.167-2522.567,
  `1bAevGz0K2k` 115.867-116.233, `f5sMl9a5jds` 212.700-213.133; VERIFIED), about 1.9 banner
  heights across with its sparks (PARTIAL, camera-measured). The official app's is the orange
  burst, far larger, spilling over the jackpot values.
- **The row lines** of the board are thin, about 2.5-3 px of our canvas, and far paler and less
  saturated than the 15-16 px gold dividers, which are the base frame's own, the same in the base
  game and Hold & Spin (`f5sMl9a5jds` 205.0 and 212.8, `y8c8lEdPW8Y` 973.0; VERIFIED). Their hue
  depends on the camera: a pale cream-gold once one camera's blue cast is removed (PARTIAL); the
  lavender that camera shows is its cast (HYPOTHESIS). The official app's frame draws them in
  saturated gold, about twice as thick on our canvas.
- **The status line** (VERIFIED for the base game on `1bAevGz0K2k` and `f5sMl9a5jds`): the bet line
  while the reels spin; blank from the trigger's result through the bell, burst, fire and board
  (`1bAevGz0K2k` 107.6-113.0, `f5sMl9a5jds` 206.5-211.5); "Press START FEATURE button" from the
  prompt through the count-up, until the first respin starts (gone at 116.37 and 213.27, as the
  reels move); blank while a respin runs; "Press SPIN button" between respins; "Free Spins
  Completed" from FREE SPINS COMPLETED through the tally and the count (also `rwatCLs3QO0`); then
  the bet line. Inside the free games (`y8c8lEdPW8Y` only, two of its features: STRONG) "Free Game
  3 of 6" shows through the intro, from the press through the count-up and while respins run,
  with the prompt and "Press SPIN button" as in the base game, "Free Spins Completed" through the
  tally and the count, and "Free Game 3 of 6" again 0.23 s after the count ends.

**The app** follows these: the count holds the 3 for 0.59 s and the first respin follows at once
(the press has already played it; the earlier hold of 1.4 s was meant to precede the 0.5 s step
pause, which never applied there, so the app waited about 1.4 s); the burst is the pack's own
burst recoloured blue-white at load (its alpha kept; the pack untouched), its visible frames 0-17
at 40 fps, on a 230 px square at the digit (its ring about 1.9 banner heights); the pack frame's
own row lines are cleared at load, anti-aliasing included, and 3 px lines of srgb(0.94, 0.82,
0.58) are drawn on the row boundaries under its dividers and border (the hue PARTIAL); the status
line as above, in the bet line's yellow and spacing, the trigger's result to our
HoldAndSpinStarted (about 0.2 s) still on the bet line.

## Hold & Spin payout

| Step | Real app | Label |
|---|---|---|
| Rumble | SND_Rumble 0.37 s after FREE SPINS COMPLETED ends; fades out over the first coin | VERIFIED |
| Banner | An empty WIN banner over the reels 2.86 s after it | VERIFIED |
| Coins | Reel by reel, top to bottom, 1.35 s apart: the coin's fire comet (the game's bolt animations) flies to the banner, which adds the value at once; SND_Small_5 per coin, SND_Medium_1 for a 10x coin | VERIFIED (11 coins); the size boundary UNKNOWN |
| Count | 0.87 s after the last value: a coin shower and the payout music while the win counts up linearly over the music's length (62.9 s); a press completes it and stops the music | VERIFIED |
| Letters | The banner's WIN letters burst every 59 frames from 0.8 s after the last value | VERIFIED |
| Music | Not among the game's own clips: one of three tracks in the app's shared sound bank. The same tracks play under the cabinet's coin-shower counts; across one cabinet session they ran in turn | VERIFIED (app); STRONG (cabinet, rotation) |

**The cabinet's collection differs** (VERIFIED: club recordings 1bAevGz0K2k 2:25-2:45 and
145.2-165.2 s, y8c8lEdPW8Y 1001.9-1019.9 s, f5sMl9a5jds 4:18-4:38, rwatCLs3QO0 about 1170-1174 s,
all Golden Century; a casino build, OL6Pu1AI-cQ 86 s, and the owner's Happy & Prosperous cabinet
agree). The table above is the official app's. On the cabinet, in the frame FREE SPINS COMPLETED
goes (3.07-3.10 s after it showed), the top screen's jackpot meters give way to WINNER over an
empty flame-bordered box, with a small line "GRAND JACKPOT: $x   MAJOR JACKPOT: $y" under it whose
values keep climbing; nothing shows over the reels and WIN does not move. This happens for every
Hold & Spin, not only a Grand. The first coin flares 0.33-0.43 s after the box appears; each coin's
comet flies up to the box, its value landing 0.57-0.73 s after the flare (mean 0.67; the first
coin's 0.70-0.73), the coins 1.33-1.37 s apart, with white splash arcs over the box each time
(measured at 30 fps on four recordings: box, first flare and first value at 1bAevGz0K2k 145.200,
145.533, 146.267; y8c8lEdPW8Y 1001.933, 1002.267, 1003.000; f5sMl9a5jds 258.800, 259.233, 259.933;
rwatCLs3QO0 clip 5.033, 5.467, 6.167; this replaces the first audit's 0.1 s, 0.3-0.5 s and
0.6-0.8 s, read at a coarser step). The WIN banner over the reels appears only when the count
begins, counting up from nothing in step with WIN (banner $1.08 with WIN $11.08 from $10.00;
banner $0.29 with WIN $0.29), its amount white between the gold WIN words (as bright and neutral
as white text in the same frames, while the WINNER total is yellow: f5sMl9a5jds 278.6, 281.1,
282.6, y8c8lEdPW8Y 1018.7, 1019.5, 1bAevGz0K2k 164.3, 165.5; the same on the free games' win
banner, y8c8lEdPW8Y 1029.8, f5sMl9a5jds 795.8; VERIFIED; the official app's amount is yellow), its
WIN letters bursting from that moment; the box keeps the total through the count, then gives way
to the meters 0.53-0.60 s after the count is complete, through one dark frame; the WIN banner
stays until the next game. The line's colour differs between cameras (yellow on one,
green-tinted on another): PARTIAL. The pause from the last value to the count differs between
cabinets (1.6 s and 3.8 s), so the app's 0.87 s is kept. Whether the cabinet plays SND_Rumble at
the collection's start is UNKNOWN (the copies have no sound).

**The app** follows the free games, Hold & Spin and payout sequences above at the measured times,
and the cabinet where it differs (the prompt, the intro and the collection above), with the owner's
order of the Grand (below). Its collection: a navy plate over the whole jackpot bar (our single
canvas has no top screen; the plate, the box's size and the line's place are our layout, the
owner's call), "WINNER" over the pack's flame-bordered box (440 x 80, the total in it), the live
Grand and Major under it; the box in the frame FREE SPINS COMPLETED goes, the first coin 0.38 s
later, 1.35 s apart, each comet 0.67 s to the box with the pack's splash (the bolt animation's
frames up to the one where it reaches the box, 12-17 of them by animation, stretched over the
flight, so the comet lands with the value; the rest at 30 fps); the count's banner from nothing
with WIN, its amount white with a faint cool tint (srgb(0.93, 0.94, 1.0); the free games' win
banner too), its letters bursting as it appears; the plate gone 0.55 s after the count. The
rumble (its place on the cabinet HYPOTHESIS) and the 0.87 s pause are the official app's (the
cabinet copies have no sound). It keeps the
cabinet's own meters, button deck and gamble (owner), rebuilt from cabinet footage in
[GOLDEN_CENTURY_CABINET_DECK.md](GOLDEN_CENTURY_CABINET_DECK.md).

**The jackpots through Hold & Spin** (second audit pass, 2026-10-19; [FOOTAGE.md](FOOTAGE.md)):

- **The disc.** Through every Hold & Spin the cabinet's top screen shows a sun disc, "N COLLECTED /
  15 WIN / GRAND JACKPOT", the count including the held coins and stepping as each coin lands, the
  four jackpots in small boxes at its corners (Grand and Minor left, Major and Mini right) and fire
  dragons either side (VERIFIED: 1bAevGz0K2k 141.5 s, OL6Pu1AI-cQ 0.5 and 130 s, Happy & Prosperous
  bUbSjAGwqs8). The app shows it on a plate over the jackpot bar, as it does the WINNER display:
  the game's coin with the count, "COLLECTED" and "15 WIN" in a substitute face, the GRAND JACKPOT
  lettering cut from the game's COLLECT 15 TO WIN art, each box cropped from the game's jackpot bar
  with its live value, and the game's fire dragon (our layout, the owner's call: HYPOTHESIS). The
  official app's "N COLLECTED" badge (VERIFIED for the app) is not used.
- **A Major first.** On an Autumn Moon club machine (5Z1-hFzpyh0 525-590 s, a storyboard; its Hold &
  Spin scenes are Golden Century's) the Major is awarded before the coins: WINNER over the box with
  the MAJOR JACKPOT title, empty, then its amount in green with a coin shower for about 20 s; the box
  empty again as the first comet leaves; the other coins tallied without it; WIN shows the coins only
  and CREDIT takes the Major apart (STRONG; PARTIAL for Golden Century). The app follows: the Major's
  step first, no comet, the pack's MAJOR JACKPOT plaque (729 x 108, a pack without it keeps the flame
  box), the amount green as it lands (its meter restarting then), its own count with the payout music
  and the shower when it is 25 times the bet or more (SPIN completes it), then the empty box and the
  coins. SND_Credit1 ticks its plaque as the cabinet's Grand plaque is ticked (HYPOTHESIS by analogy).
  The money is the owner's call: WIN counts the Major up during its own count and keeps it, and the
  count's banner shows the coins only (the cabinet credits the Major to CREDIT directly).
- **The Grand's fire.** Before WINNER / GRAND JACKPOT shows, a fire wipes both screens from the
  bottom up and clears, bottom first (OL6Pu1AI-cQ 3.4-4.4 s at 0.1 s steps: VERIFIED on one machine).
  The pack's transition fire, the intro's, has the same shape (frames 15-20 fill, 21-35 cover, 36-41
  clear; 26 frames, 0.87 s at 30 fps against the cabinet's 0.9-1.0 s: STRONG that it is the same
  fire). The app plays it at 30 fps, started 0.9 s before the Grand's step, so the plaque switches on
  under full cover (its frame 27) and shows as the fire clears (in the owner's coins-first order:
  HYPOTHESIS). The intro keeps its own 22.3 fps.
- **Status lines.** "Free Spins Completed" through the tally and the count (VERIFIED); then, at rest
  with the win offered, PLAY ON and "Free Spins Completed" in 2.0 s turns from the offer, PLAY ON
  first (VERIFIED on the owner's Happy & Prosperous club recording, 60 frames each; on two Golden
  Century recordings PLAY ON shows first and the player takes the win within 0.2 s, the bet line
  following: 1bAevGz0K2k 165.3-165.7, f5sMl9a5jds 282.5-283.9, so PARTIAL for Golden Century). A win
  never offered (a progressive, or past the gamble limit) keeps "Free Spins Completed" until the next
  game (HYPOTHESIS); a win taken leaves the bet line. After a Grand, from its award until the next
  game, the club line reads "GRAND JACKPOT Win $24122.63" in the bet line's yellow, the dollars
  ungrouped, and the system line "Call Attendant : JACKPOT HIT $24122.63" (Wc7QU1C2jWo 3.6 s, one
  legible frame: STRONG; the casino build's "Call Attendant - You have won a ..." and "GRAND JACKPOT
  $13566.04", OL6Pu1AI-cQ 161-163.5 s, recorded as a conflict). The line implies a hand pay; the app
  still credits the Grand and shows the lines as filmed.
- **The meters.** They climb in one- and two-cent steps at about the club machines' rate (STRONG on
  five machines; the app's intervals synthetic) and stay live through the feature, as filmed; the app
  holds every meter only while a respin the simulation has resolved plays out, so a meter it has
  paid does not stop alone before its coin lands (HYPOTHESIS, an engineering choice). A changed figure
  rolls up inside its box over 0.37 s, the old one out at the top, the new one in from below
  (VERIFIED on y8c8lEdPW8Y at 30 fps; a carry not seen, so every changed figure rolls). The Grand
  restarts at $10,000 (OL6Pu1AI-cQ 86-163.5 s: VERIFIED on one machine, STRONG for Australian Golden
  Century).
- **The banner.** The feature banner over the reels now sits 10 px lower, its gold rim (canvas y
  124.5) clear of the Major's and Minor's values; it hid their bottom 7-8 px (measured on a capture;
  the cabinet has separate screens, and the official app's banner sits below its bar).

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
| betbeep1/3_reduced | A first game at each denomination of a casino session sounded x1 G3 and x2 C4, x5 and x10 the shared bank's betbeep4 (G4) and betbeep5 (C5); each multiplier occurred at one denomination only, so whether the note follows the key or the denomination is open. The app's casino profile plays them on a new bet; x3 gets the bank's E4 (betbeep3), not heard; club decks are silent (unmeasured) | notes VERIFIED; key PARTIAL; x3 HYPOTHESIS ([GOLDEN_CENTURY_CABINET_DECK.md](GOLDEN_CENTURY_CABINET_DECK.md)) |
| GenericButton | The help screen's page buttons (scene file) | VERIFIED (scene; unheard) |
| SND_Strike, BonusRollupTerm | Never heard in the game's flow | STRONG |
| Others | anticipation1..5, SND_Pic1..4, SND_LargePrize, SND_GrandBell, SND_JackpotFanfare, SND_DrumRoll_*, SND_WindUp, BonusRollup, SND_Major, SND_Credit2/4/5/Small, the ST_ and reel/whirring clips: no placement shown by footage | UNKNOWN |

Recorded but not followed here: on the cabinet a Grand is shown and counted before the coins; the
owner chose the coins first, then the Grand. A Major follows the cabinet (first; above).

## Not known

- The rule choosing the win tune between and beyond the measured win sizes.
- The size above which a collected coin sounds SND_Medium_1.
- The flame plumes beside the medallion: not matched to any animation in the game's files.
- Whether the payout tracks rotate or are chosen by win size.

## Changed by the later titles' audit (2026-10-20)

The audit of the seven later titles against their own club footage
([LATER_TITLES_AUDIT.md](LATER_TITLES_AUDIT.md)) found family-wide rules that Golden Century's own
club footage confirms, so this presentation follows them now. Where a row above says otherwise,
this section wins:

- The flags' wave lasts the title's SND_Scatter + 0.12 s: 4.87 s here (club cabinets, STRONG;
  the official app's count follows its 4.75 s clip directly).
- The pictures, flags and emperor animate at 20 fps (video textures; official app and club
  cabinet), not 30.
- The first free game's reels rest 1.79 s later than a later free game's (1bAevGz0K2k 329.10,
  y8c8lEdPW8Y 950.33; VERIFIED).
- A stand's lines show at once; only WIN's count waits for SND_Wild + 0.2 s (y8c8lEdPW8Y
  732-733; STRONG). "The win waits 2.5 s" above means the count.
- FEATURE COMPLETED plays Genghis Khan's 2.2 s sting on the club cabinet (tg4nMylHxhI, 2 of 2;
  STRONG, one recording), FREE SPINS COMPLETED the 3.0 s SND_Fea_End; the doors open without
  SND_EndMask (5 of 5; STRONG). Conflicts with the official app, recorded; the cabinet followed.
- The coin shower from 25x the bet (none at 20x and 22x on the club cabinet); the free games'
  big-win plaque burns every 2.0 s under it; the counter bursts on every return to 3 (VERIFIED).
