# Golden Century: which sound plays when

Source classes: **gameplay recording** (cabinet footage, measured by audio
fingerprinting against the game's own clips, each finding re-measured by an
independent check on other instances) and **cabinet scene data** (the
unencrypted scene files name the cues: per-reel `ReelClink` and
`Anticipation_1..5`, per-respin-reel `ReelClink`, and the global cue list).
The scene data names cues; it does not say when game logic fires them.

| Event | Clip | Confidence |
|---|---|---|
| SPIN pressed, reels spinning | nothing | VERIFIED (121 spins) |
| A reel lands (plain, or flags, or coins that can no longer make six) | `reel_clink1..5` (identical waveforms), at the hard stop | VERIFIED (283 landings) |
| A reel lands coins while six are still reachable (coins on screen + 3 × reels still spinning ≥ 6); n-th such reel this spin | `SND_JPDoonk{n}` | STRONG EVIDENCE for n = 1, 2; PARTIAL for 3, 4 |
| Anticipation (two flags, five coins) | nothing audible | STRONG EVIDENCE |
| `Anticipation_1..5` scene cues | rule unknown | UNKNOWN |
| Spin that triggers a feature, after the last reel | `feature_bell` | STRONG EVIDENCE (Hold & Spin); PARTIAL (free games) |
| Free games won ("x FREE GAMES WON", flags wave) | `SND_Scatter` | PARTIAL |
| Free games: music | `SND_Fea_BGM`, looped, from the start press | VERIFIED |
| Free games: spin start, door reveal, an ordinary win | nothing (the music carries on; no win tune) | VERIFIED |
| Hold & Spin: coins ignite at the intro | `SND_FeatureTrigger` | VERIFIED |
| Hold & Spin: music | `SND_FreeSpin_BGM` looped from the end of the intro; `SND_FreeSpin_BGMFast` once 13 coins are held | PARTIAL |
| Hold & Spin: each respin pressed | `SND_Button` | VERIFIED |
| Hold & Spin: counter count-up, an empty spot stopping | no evidence of any clip | UNKNOWN |
| Hold & Spin: n-th coin landing in a respin | `SND_HandSDoonk{n}` | PARTIAL |
| Hold & Spin: "FREE SPINS COMPLETED" | `SND_Fea_End`; the music stops | PARTIAL |
| Hold & Spin: each value coin counted into the total | `SND_Medium_1..5` (identical) from 0.40 s into the clip | VERIFIED |
| Hold & Spin: each MINI coin counted | `SND_Credit1` | PARTIAL |
| Base win | `wintuneNN`, rising with the win; meter counts over the tune | STRONG EVIDENCE for the tune family; the exact choice rule is PARTIAL |
| Gamble: a win | the platform's gamble-win tune, one fixed sound of about 0.72 s: a soft odd-harmonic arpeggio D4, F#3, A3 (0.095 s each) into a held D3+F#3+A3 chord that decays about 8 dB and stops at 0.70 s; 0.13-0.28 s after the card's face, the same for colour and suit, every round. Heard on every gamble win with usable sound on about 15 Aristocrat titles, Golden Century included (iHDxUc1dlKs 12.33 s; _XHUzg4qDXI 76.94 s; Autumn Moon, 5 Dragons Deluxe, 5 Koi five times in a row, and more). Not in any bank we hold (242 clips) | VERIFIED |
| Gamble: a loss | a low square-wave buzz, D#2 (0.105 s), E2 (0.115 s), D#2 (0.12 s), about 0.34 s, about 8 dB under the win tune, 0.15-0.30 s after the losing card's face; on four titles including Golden Century (zS4a6gr5iL4 68.90 s; Lightning Link, Black Panther, Where's the Gold). Not in any bank we hold | VERIFIED |
| Gamble: opening, waiting, choosing, the card turning, take win, closing, the limit exit, the next round | nothing from the cabinet: only the deck key's own mechanical click at a press (touch picks are silent); win music playing at the opening stops. No C4 beep at any of 13 openings (the earlier one was venue sound) | STRONG (several machines each) |
| Gamble, in this app | our own synthesis of the win tune and the loss buzz from the measurements above (they correlate with the Golden Century recordings at 0.43-0.47 against 0.21-0.30 in control windows), each 0.2 s after the card's face; nothing else | synthetic recreation, labelled in `cabinet::gamble_cue` |
| Grand: the bell | SND_GrandBell about 3.5 s after the fifteenth coin lands, as the topper's GRAND sign bursts into flame; the mobile engine plays it twice with a pause | VERIFIED on a Happy & Prosperous cabinet (bUbSjAGwqs8 6.95 s); the clip is shared by the family, so PARTIAL for Golden Century; in this app once, 3.2 s after the filling respin ends |
| Hold & Spin: fourteen coins held | a drum roll from the start of a respin that begins with fourteen coins held (its first 1.25 s match SND_DrumRoll_extended), SND_DrumRoll_term at the last stop | trigger STRONG (mobile engine: a respin with fourteen held); the opening VERIFIED on a Happy & Prosperous cabinet (bUbSjAGwqs8 0.48 s); the end from the code only |
| A picture's win | SND_Pic1 to SND_Pic4 (the pictures highest first) on a line win of four or five of that picture with no wild; silent on two of a kind | STRONG for the mobile engine's win-sound rule; PARTIAL for Golden Century (its paytable names are encrypted); silent on 2 of a kind VERIFIED in the official app recording |
| Not placed | SND_JackpotFanfare (guarded by a win-size test in the engine, trigger unknown), SND_Major (its footage hit was the rumble), SND_LargePrize (the engine's jackpot animation sound, cabinet unheard), the anticipation and whirring clips, and others listed in the private notes | UNKNOWN |
