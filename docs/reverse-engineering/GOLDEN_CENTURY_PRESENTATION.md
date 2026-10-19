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
| Lines | Each winning line 43-45 frames in turn; its badges light twice; its winning symbols that do not animate blink out twice | VERIFIED |
| Stacked wild | Two or three stacked emperors become the tall emperor only on a winning line, when the win would show, to SND_Wild_2 or SND_Wild_3 by height; the win waits 2.5 s; the animation runs at 20 fps. A stack on no winning line stays as heads | VERIFIED (2 cases) |

**The cabinet's line pieces differ** (club footage rectified onto our canvas: `CAM8vranI9Y`,
`y8c8lEdPW8Y`, `f5sMl9a5jds`). The badge is sized to the denomination's line configuration: about
37x22 at 50 lines, 97x43 at 25 and 100x44 at 5 (STRONG). A line has one badge at 50 lines, one at
25 lines except line 1, and both ends only at 5 lines (STRONG, 40 line wins); which end is fixed per
line and differs between the 50- and 25-line maps (VERIFIED for the lines filmed; no rule fits).
The badge sits outside the reel frame touching it, with a stub of its colour across the frame. The
app draws them so ([GOLDEN_CENTURY_CABINET_DECK.md](GOLDEN_CENTURY_CABINET_DECK.md), "Win line
pieces"); the mobile app's two large badges per line are gone.

## Free games

| Step | Real app | Label |
|---|---|---|
| Trigger | Flags land with the clink only; the feature bell 0.2 s after the reels are still, played out (2.55 s) | VERIFIED |
| Flags | As the bell ends the flags wave (their animation from its first frame, at 20 fps) to SND_Scatter (4.75 s) | VERIFIED |
| Scatter win | Counts when SND_Scatter ends, to its win tune | VERIFIED |
| Prompt | SELECT SPIN TO START FEATURE after the count (the official app's wording and timing; the cabinet's differ, below); "Scatter wins N" only while it waits | VERIFIED |
| Start | The press is silent; the flags wave on about 1 s; a hard cut to the free games: SND_Fea_BGM from its start, the first game spinning at once, the banner's total counting up from 1 with a SND_Fea_Inc tick a step (the cabinet is quicker and stiller, below) | VERIFIED |
| Doors | Land with the reels, open together 3 frames after the reels are still (1.4 s), to SND_EndMask; wins 47 frames after they begin to open | VERIFIED (6 games) |
| Pace | Next game 1.25 s after a win's count; about 2.5 s after the doors with no win | VERIFIED |
| End | FEATURE COMPLETED in the win slot, SND_Fea_End as the music stops | VERIFIED |

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
after the count, and the dark 0.43 s return. Its status shows "Free Game 1 of 6" as the reels
start, not 1.2 s later. A Hold & Spin's payment always shows its banner, however small (`WIN $0.29
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
owner chose the coins first, then the Grand.

## Not known

- The rule choosing the win tune between and beyond the measured win sizes.
- The size above which a collected coin sounds SND_Medium_1.
- The flame plumes beside the medallion: not matched to any animation in the game's files.
- Whether the payout tracks rotate or are chosen by win size.
