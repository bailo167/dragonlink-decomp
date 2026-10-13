//! The claims database: statements about the original games, one at a time,
//! each with a confidence label, a source class and a citation.
//!
//! The files under `crates/dragonlink-model/claims/` are the machine-readable
//! counterpart of `docs/reverse-engineering/`. They are compiled into this
//! crate so that the rules in [`crate::evidence`] are enforced by `cargo test`
//! on every claim: a claim cannot be labelled VERIFIED from a third-party
//! page, cannot carry support without a citation, and an open question
//! cannot quietly hold a value.
//!
//! Claims are not consumed by the simulation. Turning claims into a runnable
//! [`crate::math::MathModel`] is a deliberate, reviewed step that only
//! happens when the claims for a title are complete.

use serde::{Deserialize, Serialize};

use crate::evidence::{Confidence, EvidenceError, SourceClass, Sourced};
use crate::title::TitleId;

/// One statement about the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Stable identifier, unique within its file (`grid`, `hold-and-spin.trigger`, …).
    pub id: String,
    /// The statement, or for an unknown, the question.
    pub statement: String,
    /// The claimed value, when there is one. Absent for an unknown.
    #[serde(default)]
    pub value: Option<String>,
    pub confidence: Confidence,
    #[serde(default)]
    pub source: Option<SourceClass>,
    /// URL or document reference.
    #[serde(default)]
    pub citation: String,
    /// Date the source was read (`YYYY-MM-DD`).
    #[serde(default)]
    pub accessed: Option<String>,
    /// What the claim is about when that is narrower than the title
    /// (`mobile`, `cabinet-au`, `cabinet-us`, …).
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// The claims of one title, or of the whole family when `title` is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimFile {
    #[serde(default)]
    pub title: Option<TitleId>,
    #[serde(default, rename = "claim")]
    pub claims: Vec<Claim>,
}

/// Why a claim file is not acceptable.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("{file}: {source}")]
    Parse {
        file: &'static str,
        #[source]
        source: toml::de::Error,
    },
    #[error("{file}: claim id {id:?} is used twice")]
    DuplicateId { file: &'static str, id: String },
    #[error("{file}: claim {id:?} has an empty statement")]
    EmptyStatement { file: &'static str, id: String },
    #[error("{file}: claim {id:?}: {source}")]
    Evidence {
        file: &'static str,
        id: String,
        #[source]
        source: EvidenceError,
    },
    #[error("{file}: claim {id:?} cites a source but gives no access date")]
    MissingAccessDate { file: &'static str, id: String },
}

impl Claim {
    fn as_sourced(&self) -> Sourced<&str> {
        Sourced {
            value: self.value.as_deref(),
            confidence: self.confidence,
            source: self.source,
            citation: self.citation.clone(),
        }
    }
}

const FILES: &[(&str, &str)] = &[
    ("claims/family.toml", include_str!("../claims/family.toml")),
    (
        "claims/golden-century.toml",
        include_str!("../claims/golden-century.toml"),
    ),
    (
        "claims/happy-and-prosperous.toml",
        include_str!("../claims/happy-and-prosperous.toml"),
    ),
    (
        "claims/panda-magic.toml",
        include_str!("../claims/panda-magic.toml"),
    ),
    (
        "claims/autumn-moon.toml",
        include_str!("../claims/autumn-moon.toml"),
    ),
];

/// Parse and check one claim file.
pub fn parse(file: &'static str, text: &str) -> Result<ClaimFile, ClaimError> {
    let parsed: ClaimFile =
        toml::from_str(text).map_err(|source| ClaimError::Parse { file, source })?;
    let mut ids = std::collections::BTreeSet::new();
    for c in &parsed.claims {
        if !ids.insert(c.id.as_str()) {
            return Err(ClaimError::DuplicateId {
                file,
                id: c.id.clone(),
            });
        }
        if c.statement.trim().is_empty() {
            return Err(ClaimError::EmptyStatement {
                file,
                id: c.id.clone(),
            });
        }
        c.as_sourced()
            .validate()
            .map_err(|source| ClaimError::Evidence {
                file,
                id: c.id.clone(),
                source,
            })?;
        if !c.citation.trim().is_empty() && c.accessed.is_none() {
            return Err(ClaimError::MissingAccessDate {
                file,
                id: c.id.clone(),
            });
        }
    }
    Ok(parsed)
}

/// Every claim file compiled into the crate, checked.
pub fn all() -> Result<Vec<ClaimFile>, ClaimError> {
    FILES.iter().map(|(file, text)| parse(file, text)).collect()
}

/// Number of claims at each confidence level, weakest first.
pub fn tally(files: &[ClaimFile]) -> Vec<(Confidence, usize)> {
    [
        Confidence::Unknown,
        Confidence::UnverifiedLead,
        Confidence::Hypothesis,
        Confidence::Partial,
        Confidence::StrongEvidence,
        Confidence::Verified,
    ]
    .into_iter()
    .map(|level| {
        let n = files
            .iter()
            .flat_map(|f| &f.claims)
            .filter(|c| c.confidence == level)
            .count();
        (level, n)
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_claim_follows_the_evidence_rules() {
        let files = all().unwrap();
        assert_eq!(files.len(), FILES.len());
        // One family file and one file per title.
        assert_eq!(files.iter().filter(|f| f.title.is_none()).count(), 1);
        for t in TitleId::ALL {
            assert_eq!(files.iter().filter(|f| f.title == Some(t)).count(), 1);
        }
    }

    #[test]
    fn a_third_party_page_cannot_verify_a_claim() {
        let text = r#"
[[claim]]
id = "x"
statement = "S"
value = "5"
confidence = "verified"
source = "third_party"
citation = "https://example.invalid/"
accessed = "2026-10-13"
"#;
        assert!(matches!(parse("t", text), Err(ClaimError::Evidence { .. })));
    }

    #[test]
    fn an_open_question_cannot_hold_a_value() {
        let text = r#"
[[claim]]
id = "x"
statement = "What is the reel strip of reel 1?"
value = "A,B,C"
confidence = "unknown"
"#;
        assert!(matches!(parse("t", text), Err(ClaimError::Evidence { .. })));
    }

    #[test]
    fn citations_need_access_dates_and_ids_are_unique() {
        let cited = r#"
[[claim]]
id = "x"
statement = "S"
value = "5"
confidence = "partial"
source = "official_media"
citation = "https://example.invalid/"
"#;
        assert!(matches!(
            parse("t", cited),
            Err(ClaimError::MissingAccessDate { .. })
        ));
        let dup = r#"
[[claim]]
id = "x"
statement = "Q1?"
confidence = "unknown"
[[claim]]
id = "x"
statement = "Q2?"
confidence = "unknown"
"#;
        assert!(matches!(
            parse("t", dup),
            Err(ClaimError::DuplicateId { .. })
        ));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = r#"
[[claim]]
id = "x"
statement = "Q?"
confidence = "unknown"
certainty = "high"
"#;
        assert!(matches!(parse("t", text), Err(ClaimError::Parse { .. })));
    }
}
