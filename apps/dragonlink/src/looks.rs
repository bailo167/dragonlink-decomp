//! Each title's own presentation values, in one place: what the title's
//! club cabinet shows differently from Golden Century's where the pack holds
//! no art for it (colours drawn by us, placements read from the title's own
//! mobile scene). Everything else a title shows comes from its pack and its
//! model; the shared presentation never names a title.
//!
//! Measured on public club-cabinet footage (2026-10-20 side-by-side audit;
//! private notes in the source root, `research/side-audit-2026-10-20/`).
//! Camera colours are approximate (PARTIAL); which colour family a title uses
//! is VERIFIED on the recordings cited.

use bevy::prelude::*;

/// One title's presentation values.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct TitleLook {
    /// The GAMES MENU's cent keys and its confirmation's YES / NO: their face
    /// colour, or `None` for Golden Century's own red-orange shades.
    pub cents_key: Option<[u8; 3]>,
    /// The chooser's header lettering, or `None` for Golden Century's cream.
    pub chooser_header: Option<[u8; 3]>,
    /// The START FEATURE prompt's lettering, or `None` for Golden Century's
    /// cream.
    pub prompt: Option<[u8; 3]>,
    /// A free game's x2 wild: where its badge sits over the wild's own win
    /// animation, as shares of the wild's box: the badge's centre from the
    /// box's centre (x, y; y down) and its width and height. From the title's
    /// mobile scene (`WildFG`: the `Wild_x2` layer over the wild's frames).
    pub x2_badge: Option<(f32, f32, f32, f32)>,
    /// FREE GAME n OF m: where the game number ends and the total starts,
    /// as shares of the banner's width: about 24 and 21 of the art's 751 px
    /// from OF's lettering (club cabinets: Golden Century and Panda Magic,
    /// the number right-aligned to OF and the total left-aligned after it,
    /// "10 OF 12" growing away from OF; VERIFIED on one recording each).
    /// From each pack's banner_free_game.png alpha: every pack's OF sits as
    /// Golden Century's except Panda Magic's, 23 px to the right.
    pub counter: (f32, f32),
    /// The chooser's $1 key face, or `None` for Golden Century's amber.
    pub dollar_key: Option<[u8; 3]>,
    /// The tall wild over a 3-high and a 2-high stack: its rect relative to
    /// the stack, width and height in reel and row pitches and its centre's
    /// offset in rows (down positive; a 2-high: over the top pair, over the
    /// bottom pair). Each title's own mobile scene (main.mercury: StackSymbols
    /// 3H_1 / 2H_1 / 2H_6 groups, pivots and dimensions, against its reel
    /// cells; computed in the side audit, peace-long-life-verify.md section
    /// 2). Golden Century's row is what its tall emperor drew before (within
    /// 2 px of its scene's).
    pub tall3: (f32, f32, f32),
    pub tall2: (f32, f32, f32, f32),
    /// A filling reel's frames: their pixels a reel and a row, and their
    /// centre's offset from the window's in rows (Golden Gong's scene: 274
    /// scene px at 0.996 against a 246.25 x 200 cell pitch for its 168 px
    /// frames, 0.033 row above the window's centre).
    pub expand_px: (f32, f32),
    pub expand_dy: f32,
    /// The free games' mystery symbol's opening rate: Golden Century's doors
    /// 42 frames a second (their 42 frames in 1.0 s on club cabinets),
    /// Autumn Moon's moon gate 30 (its 20 frames open in 0.60-0.67 s on three
    /// club recordings; the mobile video's own rate 30; STRONG).
    pub reveal_fps: f32,
    /// The gamble screen's backdrop brightness (1 full): Peace & Long Life's
    /// orchard dims to about 0.45 while the gamble shows (wRZ5CyogZ0Q
    /// 15:52-16:00, the meters as bright as before; PARTIAL as a factor);
    /// Golden Century's, Golden Gong's and Happy & Prosperous's do not dim.
    pub gamble_backdrop: f32,
    /// Symbols drawn off Golden Century's sizing rule: (key, width and
    /// height factors on the still, its centre's offset in rows, down
    /// positive); its animation and a free game's x2 badge follow it.
    pub symbol_fit: &'static [(&'static str, f32, f32, f32)],
    /// Animations drawn larger than their still: (animation key, width and
    /// height factors).
    pub anim_fit: &'static [(&'static str, f32, f32)],
}

/// Golden Century's look: the shared presentation's own values.
pub const GOLDEN_CENTURY: TitleLook = TitleLook {
    cents_key: None,
    chooser_header: None,
    prompt: None,
    x2_badge: None,
    counter: (0.6378, 0.7738),
    dollar_key: None,
    tall3: (1.6117, 3.6610, 0.0),
    tall2: (1.6117, 2.6442, -0.0085, -0.0085),
    expand_px: (151.4, 122.0),
    expand_dy: 0.0,
    reveal_fps: crate::cabinet::REVEAL_FPS,
    gamble_backdrop: 1.0,
    symbol_fit: &[],
    anim_fit: &[],
};

impl Default for TitleLook {
    fn default() -> Self {
        GOLDEN_CENTURY
    }
}

/// The green of the later titles' cent keys: the mean of three camera
/// medians (Genghis Khan (32,150,26), 3FEkN_FN2ik 0:01 and Rlxgwa3neEg 0:01;
/// Panda Magic (43,185,8), y3yNze50f3U and nYM0NGR3px0; Golden Gong
/// (25,195,60), DWBTkIVcHNM 0:01). Green VERIFIED; the shade PARTIAL.
const GREEN_KEYS: [u8; 3] = [33, 177, 31];
/// The gold the later titles letter their chooser header in (Panda Magic's
/// brightest decile (240,225,90); Golden Gong, Peace & Long Life and Spring
/// Festival gold serif headers; PARTIAL).
const GOLD_HEADER: [u8; 3] = [240, 220, 100];
/// Genghis Khan's and Peace & Long Life's gold START FEATURE lettering (about
/// (235,195,90) on ten Genghis Khan triggers; PARTIAL as a colour).
const GOLD_PROMPT: [u8; 3] = [235, 195, 90];

/// Each title's look, by its model's slug. Golden Century and Happy &
/// Prosperous share the shared presentation's (red cent keys, yellow $1:
/// DWBTkIVcHNM 681.5, WKjdoixnVjg 752).
const LOOKS: [(&str, TitleLook); 6] = [
    // Purple cent keys (nYM0NGR3px0 0:02, face median (112,40,207);
    // AWJciGGOUEk storyboard); cream header as Golden Century's.
    (
        "autumn-moon",
        TitleLook {
            cents_key: Some([112, 40, 207]),
            tall3: (1.5975, 3.6903, -0.0136),
            tall2: (1.5975, 2.6801, -0.0182, -0.0089),
            reveal_fps: 30.0,
            ..GOLDEN_CENTURY
        },
    ),
    // Its $1 key a deeper gold (227,204,8; y3yNze50f3U, 8V086h-IJ6I).
    (
        "panda-magic",
        TitleLook {
            cents_key: Some(GREEN_KEYS),
            chooser_header: Some(GOLD_HEADER),
            counter: (0.6648, 0.8020),
            dollar_key: Some([227, 204, 8]),
            ..GOLDEN_CENTURY
        },
    ),
    // WildFG: Wild_x2 (250 x 250 at 0.9) over the 380 x 336 wild.
    (
        "genghis-khan",
        TitleLook {
            cents_key: Some(GREEN_KEYS),
            prompt: Some(GOLD_PROMPT),
            x2_badge: Some((0.1473, 0.1533, 0.595, 0.670)),
            tall3: (1.4680, 3.7230, -0.0135),
            tall2: (1.4680, 2.6665, -0.0173, -0.0080),
            ..GOLDEN_CENTURY
        },
    ),
    (
        "golden-gong",
        TitleLook {
            cents_key: Some(GREEN_KEYS),
            chooser_header: Some(GOLD_HEADER),
            tall3: (1.1105, 3.1537, 0.0038),
            tall2: (1.1082, 2.2410, -0.0012, -0.0012),
            expand_dy: -0.033,
            // Cells against the cabinet's on three recordings (DWBTkIVcHNM,
            // 0ApUzEKj3X8, I2HtFMVzZK4): the bamboo small, the disciples big
            // and low (STRONG); the rooster tall, the monk and gong panels
            // small (PARTIAL); the royals and the scroll as the cabinet's.
            symbol_fit: &[
                ("bamboo", 1.45, 1.45, 0.0),
                ("disciples", 0.88, 0.88, -0.22),
                ("rooster", 1.0, 0.82, 0.0),
                ("monk", 1.10, 1.06, 0.0),
                ("gong", 1.10, 1.06, 0.0),
            ],
            // A winning monk grows past its still (its scene's box, 1.18;
            // STRONG on the cabinet: ours shrank).
            anim_fit: &[("monk", 1.18, 1.18)],
            ..GOLDEN_CENTURY
        },
    ),
    // Green keys on four recordings (3ggOSOerdhc 17:20, DWBTkIVcHNM 26:19,
    // jM1NDlBHZCs 0:10, tjQ6rIGJSCk 3:50).
    (
        "peace-long-life",
        TitleLook {
            cents_key: Some(GREEN_KEYS),
            chooser_header: Some(GOLD_HEADER),
            prompt: Some(GOLD_PROMPT),
            x2_badge: Some((0.156, 0.256, 0.620, 0.654)),
            tall3: (1.7230, 4.0652, 0.0),
            tall2: (1.4018, 2.6522, -0.1954, -0.1860),
            gamble_backdrop: 0.45,
            // The sage drawn as its scene's reel cells draw it (the Wild
            // group's rect: 1.12 x 1.10, 0.2015 row above the cell's centre),
            // filling the cell as on club cabinets (DWBTkIVcHNM 24:07.5,
            // 3ggOSOerdhc 27:48; VERIFIED look).
            symbol_fit: &[
                ("sage", 1.12, 1.10, -0.2015),
                ("sage_x2", 1.12, 1.10, -0.2015),
            ],
            ..GOLDEN_CENTURY
        },
    ),
    // Cyan-blue cent and YES / NO keys (WKjdoixnVjg, EO7Q6xvcJMA; camera
    // about (26,226,249)), gold serif header.
    (
        "spring-festival",
        TitleLook {
            cents_key: Some([26, 205, 240]),
            chooser_header: Some(GOLD_HEADER),
            tall3: (1.5975, 3.6883, -0.0136),
            tall2: (1.5975, 2.6791, -0.0182, -0.0089),
            ..GOLDEN_CENTURY
        },
    ),
];

/// The look of the title whose model has this id.
pub fn title_look(model_id: &str) -> TitleLook {
    LOOKS
        .iter()
        .find(|(slug, _)| model_id.starts_with(slug))
        .map_or(GOLDEN_CENTURY, |(_, look)| *look)
}

/// A colour from 0-255 components.
pub fn rgb(c: [u8; 3]) -> Color {
    Color::srgb_u8(c[0], c[1], c[2])
}

/// A bevelled key's shades from its face colour: the face, a lighter top, a
/// slightly lighter bottom and a dark rim (in Golden Century's proportions).
pub fn key_shades(face: [u8; 3]) -> (Color, Color, Color, Color) {
    let mix = |k: f32| {
        let f = |c: u8| (c as f32 + (255.0 - c as f32) * k).round() as u8;
        [f(face[0]), f(face[1]), f(face[2])]
    };
    let rim = face.map(|c| (c as f32 * 0.49).round() as u8);
    (rgb(face), rgb(mix(0.3)), rgb(mix(0.05)), rgb(rim))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_have_their_own_chooser_keys() {
        assert_eq!(title_look("golden-century-synthetic").cents_key, None);
        assert_eq!(title_look("happy-prosperous-synthetic").cents_key, None);
        assert_eq!(
            title_look("genghis-khan-synthetic").cents_key,
            Some(GREEN_KEYS)
        );
        assert!(title_look("autumn-moon-synthetic").cents_key.is_some());
        assert!(title_look("spring-festival-synthetic").cents_key.is_some());
        // Only the titles with an x2 wild place its badge.
        assert!(title_look("genghis-khan-synthetic").x2_badge.is_some());
        assert!(title_look("peace-long-life-synthetic").x2_badge.is_some());
        assert!(title_look("golden-gong-synthetic").x2_badge.is_none());
    }

    #[test]
    fn key_shades_darken_the_rim() {
        let (face, light, _, rim) = key_shades([100, 50, 200]);
        assert_eq!(face, rgb([100, 50, 200]));
        assert_eq!(light, rgb([147, 112, 217]));
        assert_eq!(rim, rgb([49, 25, 98]));
    }
}
