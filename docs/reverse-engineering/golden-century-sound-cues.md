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
| Gamble: the screen opens | a C4 tone matching `betbeep3_reduced`, 0.12 s after the screen opens (iHDxUc1dlKs 0:08.79-0:08.91); equal tones occur in that clip with no press, and the $1 machine's on-camera opening (_XHUzg4qDXI 1:11.067) has none | HYPOTHESIS (weak) |
| Gamble: choosing, the card's turn, a win, a loss, waiting | no clip identified: the cabinet audio is too faint under voice and venue noise; a 0.16 s tone falling 315 to 293 Hz at the NSW reveal (12.29-12.45) matches no clip in any bank; the mobile game has no gamble, so its banks hold no gamble or card cue | UNKNOWN |
| Gamble: TAKE WIN and the return | nothing above the noise on the on-camera take win (_XHUzg4qDXI 1:19.00-1:19.30); CREDIT takes the win in one step | PARTIAL (weak) |
| Gamble, in this app | the opening beep as above; SUBSTITUTES from the game's own banks for the full/half switch (`betbeep3_reduced`), the turn (`bank_betbeep3` at 0.32) and a win (`wintune01`, as the card turns); nothing on a loss, a take win or the return; no loop | substitute, labelled in `cabinet::gamble_cue` |
