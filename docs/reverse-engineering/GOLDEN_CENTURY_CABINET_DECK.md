# Golden Century: the cabinet's button deck, GAMES MENU and gamble

What a real Dragon Link cabinet's button deck, denomination chooser and gamble look like and do, as
read from recordings, photographs, the mobile build's scene files and the national standard. Every
item was read by one agent and re-checked by an independent verifier who tried to refute it; only
what survived is here, with the verifier's wording and label. Descriptions are ours. No image,
sound or proprietary text beyond short button legends and short on-screen prompts is reproduced.
Read 2026-10-17.

**Sources.**

- **Owner's recording (private).** A phone video of a Dragon Link **Happy & Prosperous** machine
  (not Golden Century) in an Australian club, filmed by the owner: the end of a Hold & Spin payout,
  the PLAY ON offer and a gamble, 17.3 s at 30 fps, **no sound**. Times below are minutes and
  seconds into it. Held privately in the source root.
- **Owner's photo (private).** One photo of a different machine set up in the casino style
  ("Bet Multiplier", "MIN BET"). Its game is probably Golden Century; its product line (Dragon
  Link or Dragon Cash) is not visible.
- **Public recordings** (YouTube identifiers, `youtube.com/watch?v=<id>`, timestamps in video
  time). Golden Century on machines whose own top-screen strip or sign reads Dragon Link:
  `_XHUzg4qDXI` ($1, club presentation), `iHDxUc1dlKs` ($2, NSW, video button deck),
  `wEmf3ep-euA` (a 28-minute session, casino presentation), `vNWXRftZOE4` ($1, NSW club),
  `nOgrwFAA0Rw` (10c), `1bAevGz0K2k` and `y8c8lEdPW8Y` (Queensland club), `VYUB_FdOLWE` and
  `HJ4cpRnnbEY` (The Star, Sydney), `kV-c6EmEHqU`. US or US-style builds: `yGonwq_1nzE`,
  `OL6Pu1AI-cQ`, `BMavaGxuOD0`, `R0to2zbJIrM`, `mIzgmWxlvag`, `gbtyzT97Zzs`. Used for the shared
  platform only, never for Golden Century Dragon Link's own values: Golden Century **Dragon Cash**
  (`WRGgkKeAksA`, `XHch1uw5L3M`), other titles (`bUbSjAGwqs8` Happy & Prosperous, `1fqUapBbcc0`
  Autumn Moon, `99qwIeznsRM` Panda Magic, `0ApUzEKj3X8` Golden Gong, `4FZlJQsbVUA` Genghis Khan),
  and machines whose product could not be read (`LwC3hVqFRMg`, the only Golden Century gamble with
  a legible screen in the public set; `N0ciWlR0M8U`, `UZT5LutBZpU`, `etaH-vbYBKY`).
- **Official mobile build.** Scene files `buttondeck.mercury`, `upi.mercury`, `help.mercury`,
  `main.mercury`; the common and game language tables (`en-us.xlf`); the shared sound bank
  `MercuryGlobalSounds.mercury`; symbol names in the native library. Evidence of the mobile app and
  of the shared framework's vocabulary, **not** of the cabinet (CLAUDE.md rule 4).
- **Regulatory and official media.** The Gaming Machine National Standard (2016, Rev 10.3,
  Rev 12.1), Queensland guideline G08, ACT instruments NI2018-199, NI2019-726 and NI2024-704,
  Aristocrat's own stills and product sheets.

**Method and limits.** Frames read at full rate where the recording allowed, otherwise from
storyboards (whose tiles show the frame half an interval after their label: add 2.5 s or 5 s).
`_XHUzg4qDXI` plays about 1.5 times fast from 0:04.80 to 0:08.57; timings from that stretch are
video time. Sounds matched by waveform correlation against the game's and the shared bank's clips,
with control windows. Most public captures have no sound.

Labels: **VERIFIED**, **STRONG**, **PARTIAL**, **HYPOTHESIS**, **UNKNOWN**
([../EVIDENCE.md](../EVIDENCE.md)). A third-party page never makes a claim VERIFIED.

## Which machine is which

Footage titles mislabel products; only the machine's own strip or sign is used. The strip's
"© 2016 Aristocrat Technologies Australia" appears unchanged on a US casino machine, so it says
nothing about the market (`R0to2zbJIrM` 0:02.0, VERIFIED).

| Recording | Product line | Market | Presentation | Label |
|---|---|---|---|---|
| Owner's recording | Dragon Link, Happy & Prosperous (strip at 00:05.00, title at 00:04.93, logo at 00:09.60) | Australian club ("Welcome to Club ..." on the band, A$) | club: 2c, 30 LINES, BET 30 credits | VERIFIED |
| Owner's photo | probably Golden Century (vase and flag symbols); Dragon Link or Dragon Cash not visible; its jackpots sit inside the main screen at exact multiples of the bet | not shown | casino style: "Bet Multiplier 1", 1c MIN BET 50, 50 LINES | STRONG (readings); PARTIAL (title) |
| `_XHUzg4qDXI` | Golden Century $1 - Dragon Link (1:13.00) | Australian club style; state not shown | club: "1 credit per line", 5 LINES | VERIFIED |
| `iHDxUc1dlKs` | Golden Century $2 - Dragon Link (0:01.00) | NSW (GambleAware sticker, 0:07.00) | club; video button deck | VERIFIED; market STRONG |
| `wEmf3ep-euA` | Golden Century - Dragon Link (02:10, 06:55, 14:20, 22:16) | **probably a US casino**: a sign reflected in the glass names a US casino restaurant brand (16:00.30); the venue is never named on screen | casino: "Bet Multiplier N", MIN BET, fixed lines | product VERIFIED; market STRONG |
| `VYUB_FdOLWE`, `HJ4cpRnnbEY` | Dragon Link sign over a Golden Century machine | The Star, Sydney (NSW casino) | casino: "Bet Multiplier 1", 50 fixed lines | STRONG |
| `yGonwq_1nzE`, `OL6Pu1AI-cQ` | Golden Century - Dragon Link | US-style (CASH OUT and SERVICE keys, INSERT BILL/TICKET); no market text | US casino | product VERIFIED; market HYPOTHESIS |
| `BMavaGxuOD0`, `R0to2zbJIrM` | Golden Century - Dragon Link | US casino (rewards-club player display; one machine, same asset tag) | US casino | VERIFIED |

## Deck variants

Four decks exist on Dragon Link cabinets. **The owner's recorded cabinet has variant A.**

| Variant | What it is | Where seen | Label |
|---|---|---|---|
| **A. Australian club LCD deck** | Two rows of six square chrome-rimmed keys whose legends are small displays, plus a large clear teardrop (plectrum) key at the right spanning both rows. GAMBLE top left over TAKE WIN bottom left; credits-per-line keys along the top, lines keys along the bottom | `vNWXRftZOE4` 0:55.0 (Dragon Link $1); `_XHUzg4qDXI` 0:05.80 (Dragon Link $1); `nOgrwFAA0Rw` 3:15 (Dragon Link 10c); `WRGgkKeAksA` 0:04 (Dragon Cash $2, the most legible); `bUbSjAGwqs8` 0:08.95; `1fqUapBbcc0` 2:45; owner's recording 00:09.67, 00:13.67, 00:15.00 | VERIFIED (layout); legends by denomination below |
| **B. US casino LCD deck** | The same 2 x 6 + teardrop hardware with other legends: CASH OUT then five identical PLAY / 50 / LINES keys on the top row; SERVICE then BET x 1, x 2, x 3, x 5, x 10 on the bottom row; teardrop PLAY. No GAMBLE or TAKE WIN key, and no gamble is offered | `yGonwq_1nzE` 0:00.00-0:00.50; top row also `OL6Pu1AI-cQ` 01:49 | STRONG (bottom-row values for x5 and x10 cut by the frame) |
| **C. Video (touch) button deck** | A flat touch display instead of the keys, mirroring the screen (fiery WINNER, numbered buttons, the gamble's RED/BLACK and suits), with one physical PLAY button | `iHDxUc1dlKs` 0:00.4-0:15.6 (NSW, $2); US portrait `mIzgmWxlvag`; the later MarsX cabinets (official sheets, ACT NI2024-704) | STRONG |
| **D. Australian casino deck** | Never legible: a row of chrome keys lit purple and white, glimpsed under a player display | `VYUB_FdOLWE` 5:47.0 | UNKNOWN |

The long casino session `wEmf3ep-euA` never shows its deck (00:00-27:59, STRONG). The owner's photo
shows only the tops of four silver-framed panels at its bottom edge (PARTIAL).

Owner's recording, deck A in detail: 2 x 6 rounded clear-capped backlit keys and a teardrop with a
blue LED ring (VERIFIED). Legible legends: a "1" on the bottom row's second key and a "10" on its
fourth, a single digit on the third and a two-digit number on the sixth, the fifth key plain pale
(00:09.60-00:09.90, STRONG). With 30 LINES on screen that matches the club lines ladder
1 / 5 / 10 / 30 / 50 with 30 selected (PARTIAL: our reading). Top left violet (the GAMBLE position),
bottom left cream (the TAKE WIN position) (VERIFIED as colours).

Cabinet platform: Golden Century - Dragon Link (1.HDG11) is approved on Gen8 (Helix family) base
software (ACT NI2018-199, NI2019-726, STRONG); a HELIX badge is legible only on a Dragon Cash deck
(`WRGgkKeAksA` 0:04.2). A MarsX Slant re-release (1.HDG96) followed in 2024 (ACT NI2024-704,
STRONG). No ACT instrument names Dragon Link's deck hardware (PARTIAL).

## Deck A: legends

| What | Real cabinet | Label |
|---|---|---|
| Left column | GAMBLE (purple) over TAKE WIN (gold, two lines) | VERIFIED (`WRGgkKeAksA` 0:04, `_XHUzg4qDXI` 0:05.80, `4FZlJQsbVUA` 5:52) |
| Top row, keys 2-6 | Credits per line: orange or red keys, a large numeral with a small word above (about three letters, unread) | VERIFIED as numerals |
| Bottom row, keys 2-6 | Lines: blue keys reading PLAY / n / LINE(S) / n CREDIT(S) | STRONG (`WRGgkKeAksA` 0:04.8) |
| $1 (Dragon Link) | Top: GAMBLE, 1 (pale), 2 (red), three orange keys with no legend. Bottom: TAKE WIN, 1 2 3 4 5 | STRONG (`vNWXRftZOE4` 0:55.0-0:55.5; `_XHUzg4qDXI` 0:05.80, 1:05.00) |
| $2 (Dragon Cash) | Credits: 1 only, four keys blank. Lines: 1-5 | STRONG (`WRGgkKeAksA` 0:04) |
| 10c (Dragon Link) | Lines 1 / 5 / 10 / 20 / 25 (the first two inferred); credits row not legible | STRONG (`nOgrwFAA0Rw` 3:15-3:18) |
| 1c (product unconfirmed) | Credits 1 / 2 / 3 / 5 / 10; lines 1 / 5 / 10 / 30 / 50 | VERIFIED as read; PARTIAL for Golden Century Dragon Link (`N0ciWlR0M8U` 1:15.0, `UZT5LutBZpU` 8:50) |
| 2c (Autumn Moon) | Credits 1 / 2 / 3 / 4 / [5]; lines 1 / 5 / 10 / 30 / 50. A Golden Century 2c game reaches 50 lines x 5 credits (`1bAevGz0K2k` 0:17.40) | PARTIAL (`1fqUapBbcc0` 2:45) |
| Selection | In each row the key drawn pale white is the current choice, matching the screen's bet | STRONG (five machines) |
| Unavailable keys | Credit keys not offered show no legend and stay dark orange: three at $1, four at $2. Whether a $10 cap explains it is open | observation STRONG; cause HYPOTHESIS |
| No cash or service key | Every position is used by GAMBLE, TAKE WIN, credits, lines and the teardrop; cash-out is the on-screen COLLECT | STRONG |
| Teardrop at idle | Its display alternates PLAY and REPEAT BET, about 1 s each | VERIFIED (`nOgrwFAA0Rw` 3:15.0-3:20.5; `1fqUapBbcc0` 2:45) |
| Teardrop, feature waiting | START FEATURE, rim lit blue, while the screen says to press START FEATURE | VERIFIED (`WRGgkKeAksA` 0:04.0-0:05.6) |
| Teardrop, Hold & Spin | SPIN | VERIFIED (`_XHUzg4qDXI` 1:05.00) |
| Teardrop, gamble | A card-back lattice, no text; PLAY again when the gamble ends | VERIFIED (`_XHUzg4qDXI` 0:06.80-0:08.57, 1:12.00-1:14.50) |
| Reels spinning | G O O D on top keys 2-5 and L U C K on bottom keys 2-5, wiping in from the left one letter about every 0.1 s; GAMBLE, bottom key 1 and bottom key 6 go dark; the idle legends return when the reels stop. Also on Hold & Spin respins | VERIFIED (`_XHUzg4qDXI` 0:09.13-0:10.47, 0:11.47-0:12.67, 0:40.00) |
| Idle, once | GOOD LUCK shown for about 2 s with the game idle and a hand on the teardrop; what triggered it is unknown | STRONG (`kV-c6EmEHqU` 15:49.4-15:51.4) |
| On the chooser | Six keys of one row spell C H O O S E (which row is out of frame) | STRONG (`kV-c6EmEHqU` 0:09.5-0:12.5, Happy & Prosperous) |
| Gamble | See [The gamble](#the-gamble) | |

## What the keys do

| What | Real cabinet | Label |
|---|---|---|
| A lines key (club) | Sets the line count **and starts a game** at once at the current credits per line: side boxes, BET and CREDIT change within three frames and the reels move 0.1-0.2 s later | STRONG (`y8c8lEdPW8Y` 0:15.23-0:15.47, 11:17.4; `1bAevGz0K2k` 0:17.37-0:17.53; `vNWXRftZOE4` 0:55.9-0:57.2 with a hand on the bottom row) |
| A credits-per-line key (club) | Only selects: "1 credit per line" becomes "4 credits per line" with BET and CREDIT unchanged | STRONG (`y8c8lEdPW8Y` 0:12.97; `1bAevGz0K2k` 0:16.4-0:16.7; `y8c8lEdPW8Y` 11:16.2) |
| A lines key while PLAY ON shows | Banks the win and starts a game at the new line count | STRONG (`0ApUzEKj3X8` 8:53-8:56, same platform) |
| Teardrop | Repeats the last bet | STRONG (`kV-c6EmEHqU` 15:51.4-15:53.0) |
| After a denomination change (club) | 1 LINE and "1 credit per line" | VERIFIED (`1bAevGz0K2k` 0:15.2; `y8c8lEdPW8Y` 0:11.8, 11:15.0) |
| BET meter (club) | Does not follow selections; right after a change it shows a stale value until the next game starts | STRONG |
| Credit short of the bet | Status: "Insert More Credits or Change Selection" | STRONG (`UZT5LutBZpU` 8:42) |
| A bet-multiplier press (casino) | Sets the multiplier **and starts the game**: "Bet Multiplier N" appears, 0.07-0.13 s later CREDIT drops by the bet with a beep, the reels move 0.1-0.3 s after that. No frame ever shows a newly chosen multiplier on an idle screen. The deck is out of frame | STRONG (`wEmf3ep-euA`, 13 events: 00:20.55, 02:49.13, 09:05.62, 12:21.92, 16:01.82, 17:04.98, 18:09.97, 20:50.84, 24:24.25, 25:09.23 and others) |
| Multipliers seen (casino) | 1, 2, 5, 10; every game at $5 (1c x 50 lines x 10, 2c x 50 x 5, 10c x 25 x 2, $1 x 5 x 1) | VERIFIED (`wEmf3ep-euA`) |
| Lines (casino) | Fixed by denomination and shown in side boxes: 50 at 1c and 2c, 25 at 10c, 5 at $1; MIN BET equals the line count. That lines cannot be chosen at all is inferred | VERIFIED (counts); STRONG (no choice) |
| Repeat games (casino) | Carry no bet beep; whether a PLAY key started them is not visible | STRONG (`wEmf3ep-euA`, about 138 starts) |
| The regulation | A game may be started "by pressing a play or bet button" (Rev 10.3); each play needs a separate press | STRONG (regulatory) |

## Bet beeps

| What | Real cabinet | Label |
|---|---|---|
| First game at a denomination | A 0.1 s square tone: x1 G3, x2 C4, x5 G4, x10 C5 | STRONG (`wEmf3ep-euA` 07:22.37, 12:22.09, 02:49.30, 10:49.17, 09:05.80; 14:58.97, 18:10.05, 17:05.12, 16:01.98) |
| By button or by denomination | In that video each multiplier occurs at one denomination only (x1 $1, x2 10c, x5 2c, x10 1c), so the ladder may follow either | PARTIAL |
| The notes | The shared bank's betbeep1-6 are G3 C4 E4 G4 C5 E5. The game's betbeep1_reduced is bank betbeep1 and its betbeep3_reduced is bank betbeep2 (C4), quieter; betbeep2_reduced (G#3) is a different clip. x3 on E4 would fill the ladder | VERIFIED (files); x3 HYPOTHESIS |
| Timing | The beep starts 0.07-0.13 s after the new multiplier shows and within 0.07 s of the debit | VERIFIED |
| Club machines | Not measured: the club captures with a legible deck have no usable sound | UNKNOWN |

## Deck lighting by state

| State | Real cabinet | Label |
|---|---|---|
| Idle | All twelve keys and the teardrop lit, GAMBLE purple and TAKE WIN gold or pale | STRONG (`nOgrwFAA0Rw` 3:14-3:21) |
| GAMBLE after a denomination change | Lit blue rather than purple while ARE YOU SURE? shows; whether its colour signals gamble availability is untested | HYPOTHESIS (`vNWXRftZOE4` 0:55.0) |
| Win shown, gamble offered (owner's recording) | Steadily vivid at first (00:07.97-00:08.90), then alternating plain and vivid every 15 frames, a 1.00 s cycle. Vivid: top left violet, top 2 lavender, top 3-6 orange with a blue icon, bottom left cream, bottom 2-4 deep blue with digits, bottom 5 light blue, bottom 6 deep blue. Plain: every key light blue except top 3-6, which stay orange | VERIFIED (Happy & Prosperous); PARTIAL for Golden Century |
| Gamble | Choice legends flash at 1 Hz, half pale and half full colour; top left stays violet. Flashing stops when a choice is made, and the chosen suit's key turns steady light blue with a small bar under the icon | VERIFIED (owner's recording 00:11.43-00:15.27; `_XHUzg4qDXI` 1:11.733-1:15.200, 0:07.87) |
| One clock | The deck flash and the meter band's dollars/credits switch run on one 1.00 s clock: the vivid or coloured phase starts as the meters switch | VERIFIED (owner's recording) |
| A press | Appears to reset the flash: the vivid look returns inside a plain phase just after the gamble-entry press, and the plain phase due after the BLACK press never comes | HYPOTHESIS |
| Feature waiting (US deck B) | The teardrop blinks, about 0.5 s on and 0.5 s off; the banner on the reels blinks too | STRONG (`yGonwq_1nzE` 0.000-1.2) |
| Count-up after Hold & Spin | Irregular shapes on the key faces; reflections or animation cannot be told apart | HYPOTHESIS |
| Cabinet light | Deck surface red after Hold & Spin, neutral in base game and gamble; the card slot's light pink, then blue in the gamble, both switching on the gamble's first frame | STRONG (owner's recording 00:11.40-00:11.47; `_XHUzg4qDXI`) |
| Side light strips (casino) | Yellow in base play and free games; fade to red as the chooser opens and through every Hold & Spin | VERIFIED (`wEmf3ep-euA`) |

## Meter band and on-screen buttons

| What | Real cabinet | Label |
|---|---|---|
| Club meters | CREDIT / BET / WIN switch between dollars and credits every 1.00 s, one blended frame per switch, in the PLAY ON state, through the gamble, and from a denomination choice until the first game (a game cuts a credits phase short and shows dollars at once; the chooser's own CREDIT stays in dollars); dollars at base idle | VERIFIED (owner's recording 00:09.43-00:14.43; `_XHUzg4qDXI` 1:11.433-1:14.433); after a choice VERIFIED (`y8c8lEdPW8Y` 11.77-15.30 and 171.77-173.73; `CAM8vranI9Y` 15.43-17.67 video time; `1bAevGz0K2k` 15.20-17.4; `etaH-vbYBKY` 4.45-9.8: five cases, four recordings, three venues); idle STRONG |
| Meters after a choice: phase | The 1 s clock is not tied to the YES: the first switch comes 0.26-1.04 s after the chooser starts to close, the first unit shown is dollars three times and credits twice, and once the alternation was already running before the GAMES MENU opened and carried on in the same phase after the choice. A credits-key press does not restart it (`y8c8lEdPW8Y` 12.97, `1bAevGz0K2k` 16.05) | PARTIAL (phase); VERIFIED (no restart) |
| Meters after a choice: BET | In a credits phase BET shows the last game's bet in credits of the new denomination ("BET 50" against "$1.00" at 2c, one line, one credit per line) | VERIFIED (`CAM8vranI9Y` sec 12.1-13.05; `1bAevGz0K2k` 15.8-16.8) |
| Short credit windows at idle | Brief alternation windows with no denomination change, each opening on credits and ending at a game or the GAMES MENU (`y8c8lEdPW8Y` 61.2-61.6, 168.967, 1311.70-1314.70; `etaH-vbYBKY` 17.5-18.3; `CAM8vranI9Y` sec 17.67-18.47); the deck is out of view, so the trigger (perhaps a bet key that does not start a game) is unseen; long idle stretches stay in dollars | HYPOTHESIS (trigger) |
| Casino meters | Dollars throughout, at every denomination; no switch to credits | VERIFIED (`wEmf3ep-euA`; US builds) |
| The standard | A meter that alternates must not alternate during a play or the win count-up | STRONG (GMNS 2016 6.37) |
| Club status line | "N credits per line"; "Game Over - Play Now"; "PLAY ON, GAMBLE or TAKE WIN"; "Line N wins X"; "3 [flag] Start FREE GAMES" or "6 [coin] Start HOLD & SPIN FEATURE" (the last three on the message row below it, at its right). "Credit reached $100.00, cash-in disabled" and the venue's greeting, listed here before, are the message row's left slot (next row; corrected 2026-10-19 from the rectified frames) | VERIFIED (`y8c8lEdPW8Y`, `_XHUzg4qDXI`) |
| Club system message | The message row's left slot, always filled on Queensland club machines: left-aligned at canvas x 225-227, capitals centred near y 923 (club 913-932), in the right slot's face and colour. Texts: "Gamble not offered if gamble win will exceed $10000.00", "Gamble win limit is $10000.00", "Credit reached $100.00, cash-in disabled", the venue's greeting ("Welcome To" and the club's name), and one-off lines ("Cashless Out $0.78"); "$10000.00" never has a thousands separator | texts and place VERIFIED (`CAM8vranI9Y` sec 13.0-19, 17.0; `T49m2COe0Ho` f_0005; `y8c8lEdPW8Y` 3076.2; homography on the reel frame); an NSW club (`_XHUzg4qDXI`) leaves it empty but for one-off lines, and a US casino shows "Cashout Voucher Printed": the always-filled slot is the Queensland club configuration |
| Club system message: rules | The message changes 2.0-2.25 s after each game start (13 of 13). At $100 or more the start itself brings the greeting and the gamble notice follows 2.0 s later; below $100 the start leaves it. A start that ends a win offer or cuts off the cash-in notice brings the gamble notice. "Gamble win limit is $10000.00" comes with PLAY ON after a base game's line win and stays until the next start: not during the count, not with the offer after a feature. "Credit reached" is a one-off of 1.0-1.5 s (2.4 s once) at the first 2 s switch after the credit rises and ends at $100 or more (cash-in, a win paid), even if the credit has dropped below by then. At idle one switch after about 2.2 s was seen; the slot stays on the chooser, and the owner's recording shows the greeting there in the gamble | STRONG (`y8c8lEdPW8Y` at 4 fps over 14-65, 236-244, 2860-2885 and 3044-3085 s, more than 30 transitions; `CAM8vranI9Y`; `T49m2COe0Ho`); the cash-in duration PARTIAL; long idle HYPOTHESIS; in the gamble PARTIAL (owner's recording 00:13.53, 00:14.50) |
| Club idle alternation | The bet line alternating with "Game Over - Play Now", 2.0 s each at idle, bet line first from the end of each game and from a bet change (a credits-per-line key restarts the cycle: 12.967 to 14.967 s is exactly 2.000 s); "Game Over - Play Now" for 0.07-0.13 s at each debit | VERIFIED (`y8c8lEdPW8Y` 12.967-15.367, 1310.6-1314.70; flashes at 1318.87, 1322.57; `1bAevGz0K2k` 17.40; `f5sMl9a5jds` 11.87). Conflict: one reading found the bet line steady for 25 s (`y8c8lEdPW8Y` 3060-3085) and 4.5 s (`CAM8vranI9Y` 13.8-18.3); whether the machine was at rest then is UNKNOWN |
| Club line-win message | "Line N wins X" always in credits, plain digits with no grouping comma ("Line 13 wins 20" at 5c, "Line 4 wins 100" at $1, "Scatter wins 1000"), while WIN counts in dollars | VERIFIED (`y8c8lEdPW8Y`: 306 messages at 1c, 2c, 5c, 10c and $1, none in dollars; 22.0 s, 491-497.5 s, 787 s) |
| Club start prompt | The message row's start prompt changes to the other at the first game start after a win message has used the slot, never on a timer; no change at 7 or more no-win starts | STRONG (`y8c8lEdPW8Y` 15-84 s, 7 of 7; `CAM8vranI9Y` 18-44 s, 2 of 2); behaviour after features and gambles unobserved |
| Casino status line | "Bet Multiplier N" alternating with "Game Over - Play Now", 2.0 s each, at idle; a press breaks the cycle. "Game Over - Play Now" flashes for 0.07-0.17 s at each debit | VERIFIED (`wEmf3ep-euA` 04:35-04:50) |
| Club right side | A COLLECT pill over the denomination box, which shows the denomination large and its conversion small ("$1 = 20 Credits" at 5c, "$1 = 1 Credit" at $1) | VERIFIED |
| Club left side | GAMES MENU pill, clock, GAME RULES pill (`_XHUzg4qDXI` 0:05.50); RESERVE beside GAMES MENU on 2018 machines; a round "i" instead of GAME RULES on 2022-26 machines; an official demo still shows RESERVE, GAMES MENU, clock, GAME RULES | VERIFIED (`_XHUzg4qDXI`); STRONG (versions) |
| Casino right side | GAMES MENU pill directly above a dark box with the denomination and "MIN BET N" | VERIFIED (`wEmf3ep-euA`, owner's photo, US builds) |
| Casino left side | A tall yellow pill with a stacked-bars icon (function unseen), the ARISTOCRAT logo, a GAME RULES pill | VERIFIED |
| Lines boxes | "N LINES" boxes either side of the reels, on both presentations | VERIFIED |
| Lines boxes (club look) | A large panel each side, centred on the middle row: on our canvas, with its thin dark outline, y 405-613 and x 12-13 to under the reel frame (left), under the frame to x 1901-1903 (right); the inner edge runs under the frame's gold with no corner showing. The reels' blue lattice fill, dark at the edges with a bright centre band 0.40 of the fill's width; a gold rim of about 10 px, gold outside and orange inside. "50" (glyphs 47 px tall, about 78 wide) over "LINES" (34 px tall, about 95 wide) in a gold brush serif, centred at x 84 and 1831, y 478 and 550 | look VERIFIED (four Queensland club machines: `CAM8vranI9Y`, `T49m2COe0Ho`, `y8c8lEdPW8Y`, `1bAevGz0K2k`); geometry and glyph heights STRONG (three machines rectified onto our reel frame agree within 3 px); colours from camera footage are shifted (the rim reads as the cabinet's own frame bar) |
| Lines boxes (casino) | One casino machine shows no side boxes; its "50" sits beside CREDIT | one machine (`VYUB_FdOLWE` 6:00-6:12), not measured |
| Player-information strip | A pink "Tap here for player information and promotion" line under the band. It stays even when the game hides its own logo, so it is the venue's system, not the game | STRONG |
| Hidden touches | None seen: a hand on the reel glass did not stop the reels | PARTIAL (`wEmf3ep-euA` 26:32) |

## GAMES MENU and the denomination chooser

| What | Real cabinet | Label |
|---|---|---|
| Opening | A touch on the GAMES MENU pill opens the chooser | VERIFIED (`wEmf3ep-euA` 02:47.93, 07:18.78, 12:20.10; `etaH-vbYBKY` 3:27.1) |
| Press or release | The finger rests on the pill 0.3-1.0 s; the splash appears 0.05-0.1 s after it lifts. Acting on release is suggested, not shown | HYPOTHESIS |
| Club path | Game, a brief attract frame, then the chooser, within about 0.2-0.6 s | STRONG |
| One title only | The chooser shows Golden Century's splash and one row of denominations; no other Dragon Link title is offered on any machine. Titles sit on separate cabinets | VERIFIED |
| Header | Alternates "CHOOSE YOUR DENOMINATION" and "TOUCH SCREEN TO MAKE SELECTION" every 2.00 s | VERIFIED (`wEmf3ep-euA` 00:00.40-00:16.40; `1bAevGz0K2k` 0:13.4) |
| Queensland club | Five buttons: 1c 2c 5c 10c $1 | VERIFIED (`1bAevGz0K2k`, `y8c8lEdPW8Y`, five more) |
| Other clubs | Six: 1c 2c 5c 10c $1 $2 (`_XHUzg4qDXI` 0:04.27-0:04.77; Dragon Cash and unconfirmed machines) | VERIFIED (`_XHUzg4qDXI`) |
| The Star (casino) | Six: 5c 10c 25c 50c $1 $2 | VERIFIED (`VYUB_FdOLWE` 6:07.40-6:08.92) |
| Long casino session | Six: 1c 2c 5c 10c $1 $2 | VERIFIED (`wEmf3ep-euA` 00:10.0) |
| Look | A gold-framed panel with rounded buttons, cent buttons orange and dollar buttons lighter yellow, pale-blue figures. The current denomination is not highlighted | VERIFIED |
| Club confirmation | Touching a denomination opens a small panel "&lt;denomination&gt; GAME / ARE YOU SURE?" with YES at the left and NO at the right, centred above the row | VERIFIED (`1bAevGz0K2k` 0:14.2-0:15.0; `y8c8lEdPW8Y` 0:10.9-0:11.7; `vNWXRftZOE4` 0:55.0); a "$1 GAME" panel with two buttons on `_XHUzg4qDXI` 0:04.17-0:04.43 STRONG |
| Club timing | Touch to panel and YES to game each about 0.1-0.5 s, with a short fade | PARTIAL |
| Casino, no confirmation | One touch closes the panel 1-3 frames after the finger leaves; the splash alone stays about 0.25-0.3 s; the game returns at the new denomination with no prompt and no conversion text | VERIFIED (`wEmf3ep-euA` 00:17.85-00:18.20, 10:48.25-10:48.70); STRONG at The Star (one event, `VYUB_FdOLWE` 6:08.96-6:09.32) |
| On the chooser | CREDIT, the system message and the venue's overlays stay; BET, WIN, GAME RULES, GAMES MENU and the denomination box go | VERIFIED (`wEmf3ep-euA` 00:10; 16:00.10-16:01.20) |
| Club chooser | Keeps CREDIT and the COLLECT pill; no BET or WIN | VERIFIED |
| Top screen meanwhile (club) | A framed "PLAY &lt;denomination&gt;" box steps through the denominations with that denomination's Minor and Mini, about 3.5 s each; it empties once a choice is made | STRONG (`y8c8lEdPW8Y` 0:05.5-0:11.5) |
| Return | Each denomination comes back with the reels it last showed (a running win animation is not restored); WIN reads $0.00 | VERIFIED (`wEmf3ep-euA` 15:59.9 against 19:58.9, and three more pairs) |
| Multiplier on return (casino) | "Bet Multiplier 1" on all nine returns, while BET still shows the last game's $5.00; whether the bet itself resets is not shown. In one half BET read $0.50 at 1c on return, so the meter may keep each denomination's own last bet | VERIFIED (text); HYPOTHESIS (per-denomination memory) |
| Opening during a win | Allowed while a credited win still shows on WIN | VERIFIED |
| Menu sound | Not isolated above venue noise | UNKNOWN |

## The gamble

Two independent recordings of the gamble agree in layout, prompts and odds: Golden Century Dragon
Link (`_XHUzg4qDXI`, `iHDxUc1dlKs`) and the owner's Happy & Prosperous machine. US builds offer no
gamble (`yGonwq_1nzE` 0:47.4-0:49.6, `BMavaGxuOD0` 1:42.1, STRONG).

### Offer and entry

| What | Real cabinet | Label |
|---|---|---|
| Offer | "PLAY ON, GAMBLE or TAKE WIN" after a win | VERIFIED |
| After Hold & Spin | The offer alternates with "Free Spins Completed", about 2 s each; the small line beside it reads "Gamble not offered if gamble win will exceed $10000.00" with the offer and the venue greeting with "Free Spins Completed" | VERIFIED (`iHDxUc1dlKs` 0:05.0-0:08.6); STRONG (owner's recording 00:08.47-00:11.37) |
| Taking it during the count | GAMBLE was pressed while the line win was still presenting; WIN posted its value at the moment of entry | STRONG (`_XHUzg4qDXI` 0:06.133-0:06.767) |
| Key | GAMBLE, the top-left key (hand over the top-left keys, contact hidden) | STRONG (`_XHUzg4qDXI` 0:06.47-0:06.83); owner's recording narrows it to top keys 1-2 (PARTIAL) |
| Transition | Within 1-5 frames: a one-frame cut or crossfade, captions first, "Full Gamble Selected", then the amounts, about 0.17 s in all | STRONG (`_XHUzg4qDXI` 1:11.033-1:11.233; owner's recording 00:11.40-00:11.43; `LwC3hVqFRMg` 18:30.33-18:30.43) |
| Not offered | After a progressive (Grand or Major) win; when the gamble win would pass $10,000 | STRONG (regulatory and the on-screen notice) |

### Screen

| What | Real cabinet | Label |
|---|---|---|
| Background | The reels give way to the game's background art (the Great Wall at sunset for Golden Century) | VERIFIED (`LwC3hVqFRMg` 18:30.4) |
| Layout | "Full Gamble Selected" pill top left; HISTORY pill with five suit marks top right; a left box "&lt;kind&gt; Gamble / Gamble Amount / credits / dollars"; a right box "&lt;kind&gt; Gamble / to win / credits / dollars"; one face-down card in the centre, white border and pink lattice back; a RED pill over heart and diamond buttons at the left, a BLACK pill over club and spade buttons at the right | VERIFIED (`_XHUzg4qDXI` 0:07.60, 1:16.73; `iHDxUc1dlKs` 0:12.40; owner's recording 00:13.53) |
| Prompt | "Select a SUIT, RED/BLACK or TAKE WIN" | VERIFIED |
| Kinds alternate | Both boxes relabel together between Suit and Red/Black every 2.0 s, starting with Suit as the gamble opens | VERIFIED (`_XHUzg4qDXI` 1:11.23-1:15.263; `iHDxUc1dlKs` 0:08.7-0:10.73; owner's recording 00:11.43-00:13.47) |
| Meters | CREDIT does not change while gambling; WIN carries the amount at stake. The meters keep switching units every 1.00 s | VERIFIED |
| History | The last five cards as suit marks, newest at the left; it updates when the next round starts, not at the reveal, and carries over from earlier gambles | VERIFIED (`iHDxUc1dlKs` 0:09.0-0:13.6; `_XHUzg4qDXI` 0:07.60, 1:13.00) |

### Rules

| What | Real cabinet | Label |
|---|---|---|
| Odds | A colour pays twice the stake, a suit four times | VERIFIED (both recordings and the owner's: 1020 to 2040 on BLACK) |
| Stake | The whole win ("Full Gamble Selected" in every gamble frame) | VERIFIED |
| Half gamble | The banner's wording implies another mode; it was never shown. The standard allows a half stake with the odd credit kept; older Aristocrat games toggled it on GAMBLE (third party) | HYPOTHESIS |
| Limit on a suit | At a stake whose suit win would pass $10,000 (a $4,680 stake at $2: $18,720 against $9,360 for a colour) the suit buttons on screen and deck go blank, only Red/Black shows with no alternation, and the prompt shortens to "Select RED/BLACK or TAKE WIN" | STRONG (`iHDxUc1dlKs` 0:13.433-0:15.77, one event) |
| Limit notices | "Gamble not offered if gamble win will exceed $10000.00" at idle and on the chooser; "Gamble win limit is $10000.00" while a base game's win is offered (see Club system message). Earlier this row also said "in the gamble": neither cited time is a gamble moment, so that part was dropped (2026-10-19); no Queensland gamble footage shows the slot | VERIFIED (`1bAevGz0K2k` 0:13.4; `y8c8lEdPW8Y` 0:47.00) |
| Rounds in a row | Up to two seen in each recording; no count limit shown. The standard of the time allows at most five | PARTIAL; five STRONG (regulatory) |
| In Queensland | No gamble on progressive prizes and no auto-gamble; the gamble win cap is set by the operator and the largest non-progressive win is $10,000 | STRONG (G08, GMNS tables) |

### Choosing

| What | Real cabinet | Label |
|---|---|---|
| By touch | The on-screen RED, BLACK and suit buttons take the choice | VERIFIED (`_XHUzg4qDXI` 0:07.63-0:07.73, 1:16.40-1:16.70) |
| Video deck | Mirrors the screen's RED / BLACK pills and suit ovals; touching the diamond chose | STRONG (`iHDxUc1dlKs` 0:12.13-0:12.20) |
| Deck A, top row | Key 1 purple with no legend; 2 HEART; 3 DIAMOND; 4 a lattice (inactive); 5 CLUB; 6 SPADE | VERIFIED (`_XHUzg4qDXI` 0:07.60-0:08.43, 1:12.00-1:14.50) |
| Deck A, bottom row | Key 6 BLACK; keys 3-5 lattice; key 1 yellow or teal and key 2 red-edged, neither legible (TAKE WIN and RED are guesses) | BLACK STRONG; keys 1-2 HYPOTHESIS |
| Owner's recording | The same colour pattern (top 2-3 orange with a blue icon, bottom 1 cream, bottom 2 orange, top 5-6 and bottom 6 a blue icon on light blue). BLACK was taken with the hand over the right end, contact on top key 6 or hidden bottom key 6 | PARTIAL |
| Picked | The chosen screen button fills yellow; the deck stops flashing and the chosen suit key turns steady light blue | VERIFIED |

### Result and exit

| What | Real cabinet | Label |
|---|---|---|
| Reveal | The card turns face up from one frame to the next, with no flip | VERIFIED (three reveals) |
| Win | A large gold WIN over the card's lower half and "Gamble Win / credits / dollars" above it; WIN takes the new amount in the same frame; the side boxes keep the stake and to-win of the gamble just played | VERIFIED |
| After a win | WIN shows for 1.0 s, then the next round opens by itself on the new amount; the "Gamble Win" header stays | STRONG (`iHDxUc1dlKs` 0:12.433-0:13.433); VERIFIED (header kept) |
| Loss | WIN goes to 0; the card shows about 1 s; the game returns to the base screen by itself with "Gamble Completed - Gamble Feature Lost" alternating with the bet line; the deck returns to its idle legends and the teardrop to PLAY | STRONG (`_XHUzg4qDXI` 0:07.90-0:08.567; `LwC3hVqFRMg` 18:32.73-18:34.0) |
| Take win | A deck key in the left half, not the touch screen or the teardrop (bottom left is the likely key). The screen goes straight back to the game screen it left; CREDIT takes the win in one step a frame later with "Gamble Completed", then the previous status ("Free Spins Completed") | STRONG (`_XHUzg4qDXI` 1:19.00-1:19.40) |
| Sound | A win plays the platform's gamble-win tune (a short arpeggio into a chord) 0.13-0.28 s after the face; a loss a low square-wave buzz 0.15-0.30 s after it; everything else is silent apart from the deck key's own click. Read from about 20 machines' recordings with sound (see [golden-century-sound-cues.md](golden-century-sound-cues.md)) | win and loss VERIFIED (Golden Century among them); the silences STRONG |

## The mobile build

| What | Mobile build | Label |
|---|---|---|
| Deck | Four controls per device: minus, plus, MAX BET and a spin selector (SPIN, STOP, hold for autospin, the free-games and respin counters). No bet-multiplier, line, gamble, collect or denomination keys. The same deck and band files in all eight Dragon Link titles | VERIFIED (mobile only) |
| Status line | "Bet Multiplier 40,000" from the press through the win; "Game Over - Play Now" only between games | VERIFIED (mobile) |
| Two wordings, one quantity | The game's table holds "Bet Multiplier %ls" and, under the bet-multiplier id, "%lld credit%ls per line" | STRONG |
| Framework vocabulary | Names for "bash" bet buttons, LCD key colours (bet, line, red, black, gamble, cash-out, reserve, game rules, service), dynamic legends (Play, Repeat Bet, Take Win, Start Feature, Insert Credit), small deck animations (good luck, well done, choose), separate suit and colour availability per gamble round, gamble key maps held as market configuration (encrypted) | STRONG (names); cabinet footage now shows GOOD LUCK, CHOOSE, REPEAT BET, START FEATURE and TAKE WIN |
| Gamble prompts | The common table holds the offer, both choice prompts and three endings, including "Gamble Completed - Gamble Limit Reached" (never seen on footage) | VERIFIED (table) |
| GAMES MENU | Inactive, art-less GAMES MENU and denomination nodes under the mobile SPIN button; no denomination list or title list anywhere | VERIFIED (vestigial) |
| Button sounds | No beep at any base-game SPIN press; SND_Button once per Hold & Spin respin | VERIFIED (mobile) |

## In this project's app

The app holds the presentation as data (`Profile` in `apps/dragonlink/src/cabinet.rs`). The
**club** profile is the default, because it is the owner's recorded cabinet type and the only
Golden Century deck legible on footage; `DRAGONLINK_PRESENTATION=casino` selects the casino style.

| What | The app | Basis |
|---|---|---|
| Club deck | Deck A: GAMBLE over TAKE WIN; credits keys along the top (1c 1/2/3/5/10, 2c 1-5, $1 1 and 2 with three blank) that only select; lines keys along the bottom (1c and 2c 1/5/10/30/50, 10c 1/5/10/20/25, $1 1-5) that set the lines and play; BET stale until the next game | as above; 5c and 10c credits and 5c lines are not seen and take a neighbour's (HYPOTHESIS) |
| Teardrop | PLAY / REPEAT BET at rest (club), START FEATURE blinking while a feature waits, SPIN in Hold & Spin, a lattice and no action in the gamble | as above |
| Spelled legends | GOOD over LUCK wiping in from the left as the reels start; C H O O S E along the top row on the chooser (which row is not seen) | as above |
| Flash and meters | One 1 s clock: on the club profile the keys that can be pressed flash while a win is offered and in the gamble (top keys 1-2 and the bottom row; the gamble's choices), and the meters switch to credits every other second then and from a denomination choice on the GAMES MENU until the first game (the clock then counts from the YES: dollars as the game returns, credits from 1.0 s after the YES, a game showing dollars at once; a credits key does not restart it; any choice counts, the current denomination's included), BET showing the last game's bet in credits of the new denomination in a credits phase; the casino profile stays in dollars | as above, after a choice VERIFIED (`y8c8lEdPW8Y` 11.77-15.30 and 171.77-173.73; `CAM8vranI9Y` 15.43-17.67 video time; `1bAevGz0K2k` 15.20-17.4; `etaH-vbYBKY` 4.45-9.8); the phase from the YES PARTIAL (the cabinet's is not tied to it: 0.26-1.04 s to the first switch; 1.0 s fits two of five); a choice of the current denomination untested on the cabinet; the short windows at idle not drawn (trigger unseen, HYPOTHESIS); which keys flash in PLAY ON is the Happy & Prosperous machine's |
| Deck over the screen | The bezel, the deck panel and the keys form one layer drawn above every screen effect (bezel, then panel, then keys): coin comets, the coin shower and flares are cut at the bezel line, as at the cabinet's glass edge, and presses reach the keys during a collection or a count | the deck is hardware below the glass; nothing on screen can draw over it (every cabinet recording) |
| Lines boxes | Club: the panels as filmed, spawned under the reel panel so the frame covers their inner edges, filled with the game's own lines-box art (the pack's `payline_tab`, its fill cropped without the rim, the band centred); rim colours our frame art's gold and orange; the number in the game's own banner numerals (the pack's `bdigit` art, the brush serif; glyphs 47 px tall), "LINES" in Impact at 42 (glyphs 34 px) with a dark shadow (Impact at 58 for the number if a pack lacks the numerals). A title whose reel art carries its own tabs (Golden Gong, with its own brush-serif "50 LINES") has no `payline_tab`: its reel panel is drawn wider, its frame on the reels and its tabs outside, and no panels or text of ours are added. Casino: the small dark box as before | geometry and glyph heights STRONG, look VERIFIED (as above); the fill the game's own art (STRONG: its band is 0.41 of the fill, the cabinet's 0.40); colours matched to our art, not the camera; the numerals the game's own brush serif (STRONG: the same face as Golden Gong's own "50 LINES" tab); Impact a substitute for "LINES"; the casino profile's box not measured. Golden Gong's tabs land at canvas y 408-616 once its frame is fitted to the reels, the cabinet's box's place (an independent agreement, STRONG for that title's art) |
| Win line pieces | Sized to the denomination's line configuration (50 lines at 1c and 2c, 25 at 5c and 10c, 5 at $1): at 50 lines a small badge (node 39x27, visible about 37x22, number at 20) and dashes 36x14; at 25 and 5 a wide badge (102x52, visible about 97x43, number at 38) and dashes 52x26. One badge per line at 50 lines; one at 25, but both for line 1; both at 5. The side per line from footage (50 lines: left 1, 3, 7, 17, 21, 23, right 5, 10, 19, 20, 24, 25, 28; 25 lines: left 4, 9, 10, 11, 16, 17, 18, 19, 21, right 2, 3, 7, 12, 13, 20, 22, 25); other lines by parity, odd left. The badge sits outside the frame touching its outer gold (visible edge x 142 / 1771) at the end row's centre, over the lines box, with a stub of the line's colour across the frame into the reel. The wide badge is the blank art with its number set on it, its rounded ends kept by slicing | sizes STRONG (three machines at 50 lines, two at 25, one at 5); count STRONG (40 line wins); the side tables VERIFIED for the lines filmed (gameplay recordings `y8c8lEdPW8Y`, `f5sMl9a5jds`, `CAM8vranI9Y`); the parity rule for unfilmed lines HYPOTHESIS ("first half left" is falsified); that the size follows the denomination rather than the lines chosen HYPOTHESIS (one case: 10c with 20 lines shows the 25-line size); the per-line height offsets within the row (-109 to +17 px) not drawn; colours past line 5 still the art's five (the cabinet has more) |
| System message (club) | The message row's left slot, left-aligned at x 225 (top at the row's top + 5, Verdana Bold 23 in the right slot's colour), over the chooser's splash and the gamble screen. Texts: "Gamble not offered if gamble win will exceed $10000.00", "Gamble win limit is $10000.00", "Credit reached $100.00, cash-in disabled" (a $100.00 cash-in limit) and "Welcome", a generic stand-in for the venue's greeting. Rules: the win limit while a base game's win is offered; the cash-in notice due when CREDIT rises to $100 or more (the session's loaded credit counts); a start ends the win limit or the cash-in notice with the gamble notice (a cut-off cash-in notice stays due), or at $100 or more before its bet brings the greeting, or changes nothing; 2.0 s after a start the cash-in notice if due (1.2 s, then the greeting), else the gamble notice at $100 or more, else the other of the gamble notice and the greeting; with no game and no offer the same switch every 2.0 s, the cash-in notice included; a Hold & Spin respin counts as a start. The casino profile leaves the slot empty | texts and place VERIFIED; the rules STRONG (Club system message: rules); the 1.2 s PARTIAL; the idle switch and the cash-in notice at an idle switch HYPOTHESIS (one idle toggle; one idle cash-in, `y8c8lEdPW8Y` 14.0 s); "Welcome" a SUBSTITUTE (the venue sets the line; no venue's name is used); one-off lines ("Cashless Out") not drawn |
| Status line and message row | At idle the bet line and "Game Over - Play Now" alternate 2.0 s each, counted from the end of each game and from a bet change, bet line first (both profiles); "Game Over - Play Now" for 0.10 s at each debit; line wins in credits, plain digits ("Line 17 wins 10"); the start prompt opens on FREE GAMES and changes to the other at the first game start after a base game's win, not after free games or Hold & Spin | idle and debit VERIFIED as above; line wins VERIFIED; the start prompt STRONG, starting on FREE GAMES our choice (one recording opens on each), no change after a feature HYPOTHESIS |
| Jackpot values | Coloured by level (Grand crimson, Major green, Minor blue, Mini orange) in a heavy wide face (Arial Black, digits about 46 px tall, centred at canvas y about 95) with a dark edge | the colour family VERIFIED (`p6OesyvM5vA` 5:59, `y8c8lEdPW8Y` 0:05-0:18 and 9.0 s, 732.8 s; the official app at 20 s; Happy & Prosperous `bUbSjAGwqs8` agrees); the exact tints from camera colour, STRONG for hue only (PARTIAL as values); the face a substitute; the cabinet's pale rim inside a dark outline not drawn |
| Gamble keys | Suits on top keys 2, 3, 5, 6; BLACK on bottom key 6; lattice on top key 4 and bottom keys 3-5; RED on bottom key 2 and TAKE WIN on bottom key 1; the screen's buttons by touch | as above; RED and TAKE WIN HYPOTHESIS |
| Gamble rules | Colour x2, suit x4; up to five in a row; not after a Major or Grand; not when the win would pass $10,000; suits off past it, with the shorter prompt; cards shown 1.0 s | as above; five is the standard's maximum |
| Half gamble | Top left switches "Full Gamble Selected" / "Half Gamble Selected"; half the win at stake, the rest kept | HYPOTHESIS (never shown) |
| Gamble screen | Laid out from the footage by homography onto the canvas: the base game's backdrop undimmed, columns at x 519 / 1401, banners 510x55, RED/BLACK 502x80, suit capsules 198x100, card 303x361, WIN 415x125, upright captions over heavier amounts, the prompt in the status line; the card turns 0.15 s after a choice; the chosen button fills yellow; the art is our own drawing | STRONG (positions); the art matched by eye |
| Gamble sounds | Our own synthesis of the win tune and the loss buzz from their measured pitches and timing (no bank holds them), 0.2 s after the face; nothing else | synthetic recreation of VERIFIED sounds |
| Chooser | Club: 1c 2c 5c 10c $1, ARE YOU SURE? with YES / NO, then 1 line and 1 credit per line. Casino: 1c-$2, no confirmation, the fixed lines and multiplier 1. Both: the money kept across the change (the core recounts the credits and holds the cents left over), each denomination's last reels | as above |
| Splash | The game's own Great Wall backdrop with a drawn title: the cabinet's emperor portrait and logo are not in the mobile build | substitute |
| Casino deck | BET x 1, 2, 3, 5, 10 along the top, setting the bet and playing, with the measured beeps on a new bet; PLAY keys along the bottom | no Australian casino deck is legible: a composite, HYPOTHESIS |
| Meter band and rows | Mapped from six cabinet sources onto the canvas by homography on the reel frame: status line caps at y 897 (club) / 889 (casino) in Verdana Bold 23 with a dark edge (the club's bet line yellow, spaced); the message row with the feature prompts and line wins at y 923 / 917, right-aligned to x 1685, wholly above the band, and the club's system message left-aligned at x 225 on it; the band from y 940 (club, a rounded panel x 200-1713 over a darker strip) or 933 (casino, as the owner's photo); meters centred at y 990 / 984; capsule tabs; COLLECT or GAMES MENU over the denomination box | STRONG |
| Attract and idle | At zero credit, after 30 s at rest with no press, the GAMES MENU opens and stays until a denomination is chosen; while it shows, a PLAY plaque between the Minor and Mini steps 1c, 2c, 5c, 10c, $1 and wraps (the first step 1.1 s after it opens, then every 4.0 s; the label at +0.10 s, the values at +0.30 s, a 0.25 s white flare), the Minor and Mini showing that denomination's prizes, fire round their boxes, the Grand and Major still climbing; after a choice they go blank a moment; no demo spins, no sound | an idle club machine sits on the chooser at $0.00 VERIFIED (1bAevGz0K2k 0:05, y8c8lEdPW8Y 0:05); the plaque's steps and values VERIFIED (14 openings, one 4.0 s period; $2's values HYPOTHESIS); fire STRONG, its route not followed (HYPOTHESIS); the 30 s delay HYPOTHESIS (the shared framework's default; the mobile build switches attract off); research notes held privately |
| Jackpots | Mini and Minor fixed by denomination: 1,000 and 5,000 credits from 1c to 10c, 250 and 1,250 at $1 ($10 / $50 at 1c to $250 / $1,250 at $1). Grand and Major progressive meters in dollars, opening at random between their restart ($5,000 / $500) and 80% of a ceiling ($15,000 / $1,500), climbing a few cents at irregular moments with an occasional jump, through features, stopping at the ceiling; a win pays the meter (whole credits, the cents held over) and restarts it, the meter holding the amount won until the count reaches its coin; each jackpot's chance the stake over its prize (a progressive's restart value), so its share of the return is the same at every bet; no gamble after a Major or Grand | Mini and Minor STRONG (claims/family.toml `jackpots.minor-mini-au`); that the Grand and Major climb STRONG (`jackpots.meters-au`); chance proportional to the bet: the national standard's requirement (NS 4.17), not seen in this software; no gamble after a progressive: the standard; the openings, climb, restarts, ceilings, chances and the moment the meter restarts synthetic, the openings and ceilings the owner's ranges (meters seen on nine Golden Century machines: Grand $10,343 to $15,545.85, Major $512.94 to $1,190.03) |
| Doors | Open onto one symbol: K to 9, the pictures, the emperor (wild) or the coin, the emperor about one reveal in five and the coin one in eight; revealed coins each carry a prize (a bet multiple or Mini, Minor, Major) and open with it, counting with coins landed on the free reels; fewer than six pay nothing | the eleven outcomes VERIFIED (the game's help screen); a prize per coin STRONG (the mobile build, claims/family.toml `mobile.reveal`; filmed reveals agree); coins on the free reels STRONG (public recording y8c8lEdPW8Y, claims/golden-century.toml `free-games.hold-and-spin-inside`); the weights synthetic, near a tally of 60 filmed reveals (emperor 12, coin 7: a small sample leaning to big wins, HYPOTHESIS as a rate); fewer than six paying nothing VERIFIED (help: coins pay only in Hold & Spin) |
| Hold & Spin inside the free games | Six or more coins in a free game start Hold & Spin with its usual intro, START FEATURE and music; the status line keeps "Free Game N of 6"; it ends with FREE SPINS COMPLETED and its count, WIN counting on from the free games' total and CREDIT held; the next free game follows with no START FEATURE, or FEATURE COMPLETED after the last; the free games' music starts again | STRONG for the Queensland club recording y8c8lEdPW8Y (claims/golden-century.toml `free-games.hold-and-spin-inside`); its music and the ending after the last free game from a probably US casino recording, HYPOTHESIS for the club machine; the music on resuming HYPOTHESIS |
| Free games trigger and start | The status line "6 FREE GAMES WON!" from the result, steady; the message row's start prompt until the flags wave, then "Scatter wins N"; PRESS START FEATURE BUTTON and its status at the scatter win's first count step, the count going on under them; at the press the flags go still, the count completes and its tune stops, both rows go blank and nothing moves; the first free game's reels, its banner and its counting total 0.52 s after the press | VERIFIED (club recordings `y8c8lEdPW8Y` 938.57, 941.20, 946.07, 949.80-950.40; `1bAevGz0K2k` 319.3, 321.9, 326.80, 328.60-329.17); the cabinet's status shows "Free Game 1 of 6" 1.2-1.3 s after the reels start, the app's at once |
| Free game win banner | A free game's win shows "WIN $x WIN" over the reels only from 10x the total bet, from its first count step until 1.0 s after the count, then "FREE GAME n OF 6" again; a smaller win counts on WIN under "FREE GAME n OF 6" and its counter; a Hold & Spin's payment always shows its banner | the rule STRONG (26 paid free games in five club features, `1bAevGz0K2k`, `y8c8lEdPW8Y`, `f5sMl9a5jds`, `kVjwYtVwhkk`: shown from 11.0x, never up to 6.0x); the 10x limit HYPOTHESIS (anywhere above 6.0x and at most 11.0x fits; a dollar limit is not ruled out); the 1.0 s hold STRONG (three cases) |
| Free games after a Hold & Spin inside them | The WIN plaque goes; the board stays 0.43 s, its open positions dark under the veil, no banner or counter, the status keeping "Free Game N of 6"; then "FREE GAME N+1 OF 6", the status and the reels together, the dimmed symbols scrolling away still dimmed | VERIFIED for one club machine (`y8c8lEdPW8Y` 1019.93-1020.43, 2562.93-2563.40, 3046.57-3047.00); the moving strips draw the dimmed symbols at the resting positions' dim tint, lighter than the cabinet's veil (ours) |
| Feature start prompt | Both features: PRESS START FEATURE BUTTON over the banner's blue bar, our own lettering (Arial Black, START FEATURE 37 px, PRESS and BUTTON 30 px small caps on one baseline, widened 1.09 to span 0.96 of the field, pale cream with a dark shadow); only the lettering blinks, a 0.667 s cycle, 0.35 s lit, phased from the prompt's start and restarted, lit, as a scatter win's count under it completes; the status line "Press START FEATURE button", for the free games alternating with "6 FREE GAMES WON!" in 2.0 s slots from the prompt's start, Press first; a feature's status texts in the bet line's yellow and spacing | wording, layout and blink VERIFIED (`y8c8lEdPW8Y` 969.10-970.53, 2519.57-2521.77, 945.8-949.8; `1bAevGz0K2k` 113.77-115.37); the restart and the 2.0 s slots VERIFIED (`y8c8lEdPW8Y` 791.67-793.80, 946.10-948.50; `1bAevGz0K2k` 326.77-327.77), Press first on two of three starts; the yellow STRONG (one recording measured, `1bAevGz0K2k`); the face a substitute for the brush face; the cream judged by eye |
| Hold & Spin intro | One fire from 0.87 s after the coins burst, at 22.3 fps, the board coming up under its white; the prompt 3.50 s after the burst; no medallion over the reels, no second fire | STRONG (two club recordings, [GOLDEN_CENTURY_PRESENTATION.md](GOLDEN_CENTURY_PRESENTATION.md)) |
| Hold & Spin board | The count holds the 3 for 0.59 s, then the first respin; on the 3 the pack's burst recoloured blue-white, frames 0-17 at 40 fps on a 230 px square at the digit; the pack frame's own row lines cleared, ours drawn 3 px pale cream-gold on the row boundaries under its dividers; the status line blank through the intro and running respins, "Press START FEATURE button" through the count-up until the first respin, "Press SPIN button" between respins, "Free Spins Completed" through the tally and the count (inside the free games "Free Game N of 6" in place of the blanks and through the count-up) | the respin's timing, the flash, the line weight and the base game's status line VERIFIED (club recordings, [GOLDEN_CENTURY_PRESENTATION.md](GOLDEN_CENTURY_PRESENTATION.md)); the free games' status line STRONG (one recording); the flash's size and the lines' hue PARTIAL; the recoloured burst a substitute (the pack has no white flash) |
| Hold & Spin collection | For every Hold & Spin: a navy plate over the whole jackpot bar with WINNER over the pack's flame-bordered box and a line of the live Grand and Major under it, from the frame FREE SPINS COMPLETED goes; the first coin 0.38 s later, 1.35 s apart, its comet 0.67 s to the box (the bolt's frames up to its arrival stretched over the flight), a splash on each value; no banner over the reels until the count, whose banner counts from nothing with WIN, its amount white; the plate gone 0.55 s after the count | behaviour and timing VERIFIED (four club recordings at 30 fps, [GOLDEN_CENTURY_PRESENTATION.md](GOLDEN_CENTURY_PRESENTATION.md)); the plate's layout ours (no top screen; the owner's call); the line's colour PARTIAL |
| Coin plates on respins | A coin landing on a respinning position carries its MINI / MINOR / MAJOR plate as it arrives, like its prize digits | HYPOTHESIS: our choice, matching the base reels' moving strips; the cabinet's respinning coins are not measured |
| Coins | Every coin on a black tile (landed and moving); MINI / MINOR / MAJOR plates 280x219 (the word 0.62 of the reel's width), the same moving and landed; digits 0.295 of a row tall and 4% wider; "$" 0.69 of a digit, a little low | STRONG (blur-invariant measures on cabinet footage and the app) |
| Not done | Clock, RESERVE, "i", COLLECT and GAME RULES pages (none shown); the top screen's denomination cycle on the chooser; cabinet lighting; the gamble limits as market data; the casino configuration's Mini and Minor at $1 ($1,000 and $5,000 on one Australian casino machine; the app uses the club values in both profiles) | |

## Conflicts

1. **The long casino session's market.** An early reading took `wEmf3ep-euA` for an NSW venue
   (because the chooser reaches $2). The glass reflects a US casino restaurant's sign, and the US
   builds share its dollars-only band and "Bet Multiplier"; it is probably a US casino (STRONG).
   Its chooser (1c to $2, no confirmation), fixed lines and beep ladder are therefore not
   Australian evidence. The only Australian casino evidence is The Star: "Bet Multiplier 1", 50
   fixed lines, 5c to $2, and one denomination change with no confirmation.
2. **Which key took BLACK.** The owner's recording places the press on top key 6 (PARTIAL; bottom
   key 6 was hidden). Golden Century's own deck shows SPADE on top key 6 and BLACK on bottom key 6
   (VERIFIED). The owner's card was the ace of clubs and paid twice: a spade choice would have
   lost, so the choice was BLACK, which on the Golden Century layout is bottom key 6.
3. **The teardrop in the gamble.** Golden Century's deck shows a lattice there, with no text
   (VERIFIED); the NSW machine's physical button still reads PLAY (PARTIAL). Neither shows it
   taking the win; the observed take-win used a left-half key.
4. **Two presentations on the owner's own evidence.** The owner's recording is a club machine
   (credits per line, line keys, the gamble); the owner's photo is casino style ("Bet Multiplier",
   MIN BET, jackpots inside the screen at fixed multiples of the bet). The photo shows neither its
   market nor its product line, which may be Dragon Cash.
5. **Deck B against deck A.** The US casino deck carries CASH OUT and SERVICE where the Australian
   deck carries GAMBLE and TAKE WIN, and its bet keys sit on the bottom row where the Australian
   deck's credits keys sit on the top. No machine shows BET x keys together with GAMBLE and TAKE
   WIN.
6. **Meters.** Club machines switch units every second after a win and in the gamble; the casino
   session and the US builds never switch.
7. **BET meter after a change (casino).** The second half of `wEmf3ep-euA` always shows the last
   game's $5.00; its first half once shows $0.50 at 1c. A per-denomination memory fits both
   (HYPOTHESIS).
8. **Beep ladder.** The project's presentation notes list the x1 and x2 beeps as VERIFIED for the
   bet buttons; here the button-or-denomination question stays PARTIAL (see Bet beeps).
9. **An early reading said the long session's sounds could not be measured.** It was refuted: the
   bet beeps stand 25-32 dB clear of controls at five of seven bet changes.
10. **Cycle length.** The owner's deck flash was first read as a 33-35 frame cycle; frame-exact
    boundaries give exactly 30 frames.
11. **Lines at 2c.** The owner's club machine plays 30 lines at 2c, where [FOOTAGE.md](FOOTAGE.md)
    lists 50: lines are chosen by the player, and 30 is on the 1c-2c ladder.

## Not known

- The legends of an Australian casino ("Bet Multiplier") deck, and whether that presentation offers
  a gamble.
- Which deck keys take RED, TAKE WIN and (by deck) the other choices beyond those above; the small
  word over the credits numerals; the full credits row at 5c and 10c.
- Whether a lines key press with too little credit changes the line count.
- Whether repeat games are started by the teardrop or the bet key, on either presentation.
- Whether the bet beep follows the key or the denomination; the beep of x3; any club deck sound.
- The half gamble: whether it exists on Golden Century and how it is chosen.
- The number of gambles allowed in a row on Dragon Link, and the screen at that limit.
- What the tall pill with the bars icon does; what RESERVE, GAME RULES and "i" open (no Australian
  rules page of Golden Century appears in any recording found).
- The deck's lighting while the win counts and inside the free games.
- Whether the GAMES MENU acts on press or release; any menu sound.
- What triggers GOOD LUCK at idle.
- The cabinet model of each recording beyond the Helix family and MarsX generations.
