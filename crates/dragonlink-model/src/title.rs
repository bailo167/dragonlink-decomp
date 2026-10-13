//! The titles in scope.

use serde::{Deserialize, Serialize};

/// One of the four Dragon Link titles this project targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleId {
    GoldenCentury,
    HappyAndProsperous,
    PandaMagic,
    AutumnMoon,
}

impl TitleId {
    /// All titles, in the order they are worked on (Golden Century first).
    pub const ALL: [TitleId; 4] = [
        TitleId::GoldenCentury,
        TitleId::HappyAndProsperous,
        TitleId::PandaMagic,
        TitleId::AutumnMoon,
    ];

    /// Stable lowercase identifier used in file names, asset ids and the command line.
    pub fn slug(self) -> &'static str {
        match self {
            TitleId::GoldenCentury => "golden-century",
            TitleId::HappyAndProsperous => "happy-and-prosperous",
            TitleId::PandaMagic => "panda-magic",
            TitleId::AutumnMoon => "autumn-moon",
        }
    }

    /// The name as it is written in prose.
    pub fn display_name(self) -> &'static str {
        match self {
            TitleId::GoldenCentury => "Golden Century",
            TitleId::HappyAndProsperous => "Happy & Prosperous",
            TitleId::PandaMagic => "Panda Magic",
            TitleId::AutumnMoon => "Autumn Moon",
        }
    }

    /// The title with this slug.
    pub fn from_slug(slug: &str) -> Option<TitleId> {
        TitleId::ALL.into_iter().find(|t| t.slug() == slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip_and_are_distinct() {
        for t in TitleId::ALL {
            assert_eq!(TitleId::from_slug(t.slug()), Some(t));
        }
        assert_eq!(TitleId::from_slug("dragon-link"), None);
        assert_eq!(TitleId::ALL[0], TitleId::GoldenCentury);
    }
}
