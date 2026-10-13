//! Evidence labels and sourced values.
//!
//! The labels are the ones used throughout `docs/` (see `docs/EVIDENCE.md`).
//! They describe how well a statement about the **original games** is
//! supported. They never describe our own implementation: code compiling or
//! tests passing does not move a label.

use serde::{Deserialize, Serialize};

/// How well a statement about the original is supported, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// No usable evidence.
    Unknown,
    /// Something a source says that nobody has checked.
    UnverifiedLead,
    /// Our own conjecture, stated so that it can be tested.
    Hypothesis,
    /// Supported in part; known to be incomplete.
    Partial,
    /// Several independent indicators agree.
    StrongEvidence,
    /// Seen directly in a primary source, reproducibly.
    Verified,
}

impl Confidence {
    /// The label as written in the documentation.
    pub fn label(self) -> &'static str {
        match self {
            Confidence::Unknown => "UNKNOWN",
            Confidence::UnverifiedLead => "UNVERIFIED LEAD",
            Confidence::Hypothesis => "HYPOTHESIS",
            Confidence::Partial => "PARTIAL",
            Confidence::StrongEvidence => "STRONG EVIDENCE",
            Confidence::Verified => "VERIFIED",
        }
    }
}

/// Where a finding came from. Kept separate from [`Confidence`]: a finding
/// from official mobile software can be verified *for the mobile software*
/// and still say nothing about cabinet mathematics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    /// Software or data from a gaming machine.
    Cabinet,
    /// A regulator's document or register.
    Regulatory,
    /// An official social-casino implementation.
    OfficialMobile,
    /// The manufacturer's own published material (pages, brochures, videos, patents).
    OfficialMedia,
    /// A filing or order in a court's public record. It verifies what the
    /// document says; a party's allegation in it remains that party's statement.
    CourtRecord,
    /// A recording of the original being played.
    GameplayRecording,
    /// Somebody else's observation or write-up.
    ThirdParty,
    /// Our own conjecture.
    Hypothesis,
    /// Invented by this project for development. Not about the original at all.
    SyntheticFixture,
}

impl SourceClass {
    /// Whether a finding from this class can ever be labelled
    /// [`Confidence::Verified`].
    pub fn can_verify(self) -> bool {
        matches!(
            self,
            SourceClass::Cabinet
                | SourceClass::Regulatory
                | SourceClass::OfficialMobile
                | SourceClass::OfficialMedia
                | SourceClass::CourtRecord
                | SourceClass::GameplayRecording
        )
    }

    /// Whether mathematics (reel strips, weights, odds, RTP) taken from this
    /// class may be presented as the Australian cabinet's mathematics.
    /// Official mobile software is excluded on purpose: it is valid for
    /// presentation, not for cabinet mathematics.
    pub fn is_cabinet_math_evidence(self) -> bool {
        matches!(self, SourceClass::Cabinet | SourceClass::Regulatory)
    }
}

/// A value together with its support. `value` is `None` exactly when nothing
/// is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sourced<T> {
    pub value: Option<T>,
    pub confidence: Confidence,
    pub source: Option<SourceClass>,
    /// URL, document reference or local evidence id. Empty when unknown.
    #[serde(default)]
    pub citation: String,
}

/// Why a [`Sourced`] value breaks the evidence rules.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvidenceError {
    #[error(
        "a value is given but the confidence is UNKNOWN and the source is not a synthetic fixture"
    )]
    ValueWithoutSupport,
    #[error("confidence {0} needs a value")]
    SupportWithoutValue(&'static str),
    #[error("confidence {0} needs a source class")]
    MissingSource(&'static str),
    #[error("confidence {0} needs a citation")]
    MissingCitation(&'static str),
    #[error("a synthetic fixture says nothing about the original and cannot carry confidence {0}")]
    SyntheticWithConfidence(&'static str),
    #[error("source class {0:?} cannot support VERIFIED")]
    CannotVerify(SourceClass),
    #[error("a conjecture of ours cannot be labelled above HYPOTHESIS")]
    HypothesisAboveHypothesis,
}

impl<T> Sourced<T> {
    /// Nothing is known.
    pub fn unknown() -> Self {
        Sourced {
            value: None,
            confidence: Confidence::Unknown,
            source: None,
            citation: String::new(),
        }
    }

    /// A value with its support.
    pub fn known(
        value: T,
        confidence: Confidence,
        source: SourceClass,
        citation: impl Into<String>,
    ) -> Self {
        Sourced {
            value: Some(value),
            confidence,
            source: Some(source),
            citation: citation.into(),
        }
    }

    /// A value invented for development. It has no evidentiary status.
    pub fn synthetic(value: T) -> Self {
        Sourced {
            value: Some(value),
            confidence: Confidence::Unknown,
            source: Some(SourceClass::SyntheticFixture),
            citation: String::new(),
        }
    }

    /// Check the evidence rules (`docs/EVIDENCE.md`, "Rules").
    pub fn validate(&self) -> Result<(), EvidenceError> {
        let label = self.confidence.label();
        let synthetic = self.source == Some(SourceClass::SyntheticFixture);
        if synthetic {
            return if self.confidence == Confidence::Unknown {
                Ok(())
            } else {
                Err(EvidenceError::SyntheticWithConfidence(label))
            };
        }
        if self.confidence == Confidence::Unknown {
            return if self.value.is_some() {
                Err(EvidenceError::ValueWithoutSupport)
            } else {
                Ok(())
            };
        }
        if self.value.is_none() {
            return Err(EvidenceError::SupportWithoutValue(label));
        }
        let Some(source) = self.source else {
            return Err(EvidenceError::MissingSource(label));
        };
        if source == SourceClass::Hypothesis {
            // Our own conjecture needs no citation but cannot be promoted.
            return if self.confidence > Confidence::Hypothesis {
                Err(EvidenceError::HypothesisAboveHypothesis)
            } else {
                Ok(())
            };
        }
        if self.citation.trim().is_empty() {
            return Err(EvidenceError::MissingCitation(label));
        }
        if self.confidence == Confidence::Verified && !source.can_verify() {
            return Err(EvidenceError::CannotVerify(source));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_has_no_value() {
        let s: Sourced<u8> = Sourced::unknown();
        assert_eq!(s.validate(), Ok(()));
        let bad = Sourced {
            value: Some(5u8),
            ..Sourced::unknown()
        };
        assert_eq!(bad.validate(), Err(EvidenceError::ValueWithoutSupport));
    }

    #[test]
    fn synthetic_values_never_carry_confidence() {
        assert_eq!(Sourced::synthetic(3u8).validate(), Ok(()));
        let mut s = Sourced::synthetic(3u8);
        s.confidence = Confidence::Partial;
        assert!(matches!(
            s.validate(),
            Err(EvidenceError::SyntheticWithConfidence(_))
        ));
    }

    #[test]
    fn support_needs_a_value_a_source_and_a_citation() {
        let ok = Sourced::known(
            5u8,
            Confidence::StrongEvidence,
            SourceClass::OfficialMedia,
            "https://example.invalid/brochure",
        );
        assert_eq!(ok.validate(), Ok(()));
        let mut no_cite = ok.clone();
        no_cite.citation = "  ".into();
        assert!(matches!(
            no_cite.validate(),
            Err(EvidenceError::MissingCitation(_))
        ));
        let mut no_source = ok.clone();
        no_source.source = None;
        assert!(matches!(
            no_source.validate(),
            Err(EvidenceError::MissingSource(_))
        ));
        let mut no_value = ok;
        no_value.value = None;
        assert!(matches!(
            no_value.validate(),
            Err(EvidenceError::SupportWithoutValue(_))
        ));
    }

    #[test]
    fn third_party_and_conjecture_cannot_be_verified() {
        let third = Sourced::known(
            1u8,
            Confidence::Verified,
            SourceClass::ThirdParty,
            "https://example.invalid/blog",
        );
        assert_eq!(
            third.validate(),
            Err(EvidenceError::CannotVerify(SourceClass::ThirdParty))
        );
        let guess = Sourced::known(1u8, Confidence::Partial, SourceClass::Hypothesis, "");
        assert_eq!(
            guess.validate(),
            Err(EvidenceError::HypothesisAboveHypothesis)
        );
        let honest_guess = Sourced::known(1u8, Confidence::Hypothesis, SourceClass::Hypothesis, "");
        assert_eq!(honest_guess.validate(), Ok(()));
    }

    #[test]
    fn mobile_software_is_not_cabinet_math_evidence() {
        assert!(SourceClass::OfficialMobile.can_verify());
        assert!(!SourceClass::OfficialMobile.is_cabinet_math_evidence());
        assert!(SourceClass::Regulatory.is_cabinet_math_evidence());
        assert!(SourceClass::Cabinet.is_cabinet_math_evidence());
        assert!(!SourceClass::SyntheticFixture.is_cabinet_math_evidence());
        assert!(SourceClass::CourtRecord.can_verify());
        assert!(!SourceClass::CourtRecord.is_cabinet_math_evidence());
    }

    #[test]
    fn confidence_orders_weakest_first() {
        assert!(Confidence::Unknown < Confidence::UnverifiedLead);
        assert!(Confidence::Hypothesis < Confidence::Partial);
        assert!(Confidence::StrongEvidence < Confidence::Verified);
    }
}
