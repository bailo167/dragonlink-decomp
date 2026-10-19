//! Game configurations: a runnable math model with the provenance of every
//! constant in it.
//!
//! A configuration describes **one** game, version and platform. It is the
//! unit the analysis tools load, compare and report on. Format and rules are
//! documented in `docs/reverse-engineering/math/CONFIGURATION.md` and
//! `PROVENANCE.md`.
//!
//! Recovered configurations are proprietary material and live in the private
//! source root, not in the repository (`docs/DECISIONS.md`, D-010). The one
//! configuration built into this crate is the synthetic fixture.

use std::collections::BTreeMap;

use dragonlink_model::evidence::EvidenceError;
use dragonlink_model::fixtures::synthetic_dev_fixture;
use dragonlink_model::{MathModel, MathProvenance, ModelError, Sourced, TitleId};
use serde::{Deserialize, Serialize};

/// Identifier of this file format.
pub const SCHEMA: &str = "dragonlink-config/1";

/// How one numeric constant (or group of constants) was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstantProvenance {
    /// Not known. Never valid for a constant that has a value.
    Unknown,
    /// Invented by this project. Only in a synthetic configuration.
    Synthetic,
    /// Our conjecture.
    Hypothesised,
    /// Estimated from a sample of observed play, with its sample size.
    InferredStatistically,
    /// Read off the game as a player sees it (a rules screen, a meter).
    Observed,
    /// Taken from an official mobile implementation.
    OfficialMobile,
    /// Computed from other constants by a stated derivation.
    Derived,
    /// Stated in regulatory material.
    Regulatory,
    /// Read directly out of the game's own software or data.
    Extracted,
}

impl ConstantProvenance {
    pub const ALL: [ConstantProvenance; 9] = [
        ConstantProvenance::Extracted,
        ConstantProvenance::Regulatory,
        ConstantProvenance::Derived,
        ConstantProvenance::OfficialMobile,
        ConstantProvenance::Observed,
        ConstantProvenance::InferredStatistically,
        ConstantProvenance::Hypothesised,
        ConstantProvenance::Synthetic,
        ConstantProvenance::Unknown,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConstantProvenance::Unknown => "unknown",
            ConstantProvenance::Synthetic => "synthetic (invented by this project)",
            ConstantProvenance::Hypothesised => "hypothesised",
            ConstantProvenance::InferredStatistically => "inferred statistically",
            ConstantProvenance::Observed => "observed",
            ConstantProvenance::OfficialMobile => "official mobile software",
            ConstantProvenance::Derived => "derived mathematically",
            ConstantProvenance::Regulatory => "regulatory material",
            ConstantProvenance::Extracted => "extracted directly",
        }
    }

    /// Whether a record of this class must name where the value came from.
    fn needs_source(self) -> bool {
        !matches!(
            self,
            ConstantProvenance::Synthetic
                | ConstantProvenance::Hypothesised
                | ConstantProvenance::Unknown
        )
    }
}

/// Provenance of one constant group of the model, addressed by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstantRecord {
    /// One of [`required_paths`].
    pub path: String,
    pub provenance: ConstantProvenance,
    /// Citation, evidence hash, derivation or sample description.
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub note: String,
}

/// Which game, version and platform a configuration describes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigIdentity {
    pub name: String,
    #[serde(default)]
    pub title: Option<TitleId>,
    /// `cabinet-au`, `cabinet-us`, `mobile`, …
    #[serde(default)]
    pub market: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub program_version: Option<String>,
    /// Regulatory specification number, when known.
    #[serde(default)]
    pub specification: Option<String>,
    #[serde(default)]
    pub denomination: Option<String>,
    #[serde(default)]
    pub notes: String,
}

/// Findings about how the implementation behaves that are not numbers in the
/// model. Each starts unknown and stays so until there is evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    /// Which generator, how it is seeded, where it is called.
    pub rng_architecture: Sourced<String>,
    /// Whether the outcome is chosen before the reels are presented, and how.
    pub outcome_selection: Sourced<String>,
    /// How the reel-stop animation relates to the selected result.
    pub reel_stop_presentation: Sourced<String>,
    /// Whether the timing of player input can affect an outcome.
    pub input_timing: Sourced<String>,
    /// Whether previous spins influence later ones.
    pub spin_independence: Sourced<String>,
    /// Any persistent state that changes expected value.
    pub persistent_state: Sourced<String>,
}

impl Default for Behaviour {
    fn default() -> Self {
        Behaviour {
            rng_architecture: Sourced::unknown(),
            outcome_selection: Sourced::unknown(),
            reel_stop_presentation: Sourced::unknown(),
            input_timing: Sourced::unknown(),
            spin_independence: Sourced::unknown(),
            persistent_state: Sourced::unknown(),
        }
    }
}

impl Behaviour {
    /// The findings with the question each answers.
    pub fn entries(&self) -> [(&'static str, &Sourced<String>); 6] {
        [
            ("RNG architecture", &self.rng_architecture),
            ("Outcome-selection timing", &self.outcome_selection),
            ("Reel-stop presentation", &self.reel_stop_presentation),
            ("Effect of input timing", &self.input_timing),
            ("Independence of spins", &self.spin_independence),
            ("Persistent state", &self.persistent_state),
        ]
    }
}

/// A complete description of one game for analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConfiguration {
    pub schema: String,
    pub identity: ConfigIdentity,
    pub model: MathModel,
    pub constants: Vec<ConstantRecord>,
    #[serde(default)]
    pub behaviour: Behaviour,
}

/// Why a configuration is not acceptable.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("not valid JSON for a configuration: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema {0:?} is not {SCHEMA:?}")]
    Schema(String),
    #[error("the math model is not runnable: {0}")]
    Model(#[from] ModelError),
    #[error("constant {0:?} has no provenance record")]
    MissingRecord(&'static str),
    #[error("constant {0:?} has more than one provenance record")]
    DuplicateRecord(String),
    #[error("{0:?} is not a constant of this model")]
    UnknownPath(String),
    #[error("constant {0:?} has a value, so its provenance cannot be unknown")]
    UnknownProvenance(String),
    #[error("constant {path:?} is {class}, which needs a source")]
    MissingSource { path: String, class: &'static str },
    #[error("constant {0:?} is synthetic but the model is not labelled a synthetic fixture")]
    SyntheticInRealModel(String),
    #[error("the model is a synthetic fixture but constant {0:?} claims another provenance")]
    RealInSyntheticModel(String),
    #[error("behaviour finding {name:?}: {source}")]
    Behaviour {
        name: &'static str,
        #[source]
        source: EvidenceError,
    },
}

/// The constant groups of `model`, each of which needs a provenance record.
pub fn required_paths(model: &MathModel) -> Vec<&'static str> {
    let mut paths = vec![
        "grid",
        "symbols",
        "reel_strips",
        "paylines",
        "line_pays",
        "scatter_pays",
        "bet",
    ];
    if model.hold_and_spin.is_some() {
        paths.extend([
            "hold_and_spin.trigger_count",
            "hold_and_spin.initial_respins",
            "hold_and_spin.reset_respins_on_new_orb",
            "hold_and_spin.orb_land_chance",
            "hold_and_spin.orb_values",
            "hold_and_spin.full_grid_award",
            "hold_and_spin.jackpots",
        ]);
    }
    paths
}

impl GameConfiguration {
    /// The synthetic development fixture as a configuration. Every constant
    /// is recorded as synthetic and every behaviour finding as unknown.
    pub fn synthetic_fixture() -> GameConfiguration {
        let model = synthetic_dev_fixture();
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: "Synthetic development fixture".to_string(),
                notes: "Invented by dragonlink-decomp for engine development. Not any Dragon \
                        Link game; its statistics are its own."
                    .to_string(),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// A **synthetic** Golden Century configuration: the game's evidenced
    /// structure with invented numbers (`dragonlink_model::fixtures`).
    pub fn golden_century_synthetic() -> GameConfiguration {
        let model = dragonlink_model::fixtures::golden_century_synthetic();
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: "Golden Century (synthetic)".to_string(),
                notes: "Golden Century's evidenced structure with invented strips, weights, pays                         and jackpot amounts. Not recovered from any machine; its statistics are                         its own (docs/reverse-engineering/GOLDEN_CENTURY_SESSION.md)."
                    .to_string(),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// A **synthetic** Autumn Moon configuration (`dragonlink_model::fixtures`).
    pub fn autumn_moon_synthetic() -> GameConfiguration {
        let model = dragonlink_model::fixtures::autumn_moon_synthetic();
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: "Autumn Moon (synthetic)".to_string(),
                notes: "Autumn Moon's evidenced structure with invented strips, weights and                         jackpot amounts. Not recovered from any machine; its statistics are its                         own (docs/reverse-engineering/FOOTAGE.md)."
                    .to_string(),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// A **synthetic** Happy & Prosperous configuration (`dragonlink_model::fixtures`).
    pub fn happy_prosperous_synthetic() -> GameConfiguration {
        let model = dragonlink_model::fixtures::happy_prosperous_synthetic();
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: "Happy & Prosperous (synthetic)".to_string(),
                notes: "Happy & Prosperous's evidenced structure with invented strips, weights                         and jackpot amounts. Not recovered from any machine; its statistics are                         its own (docs/reverse-engineering/FOOTAGE.md)."
                    .to_string(),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// A **synthetic** Panda Magic configuration (`dragonlink_model::fixtures`).
    pub fn panda_magic_synthetic() -> GameConfiguration {
        let model = dragonlink_model::fixtures::panda_magic_synthetic();
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: "Panda Magic (synthetic)".to_string(),
                notes: "Panda Magic's evidenced structure with invented strips, weights and                         jackpot amounts. Not recovered from any machine; its statistics are its                         own (docs/reverse-engineering/FOOTAGE.md)."
                    .to_string(),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// A **synthetic** Genghis Khan configuration (`dragonlink_model::fixtures`).
    pub fn genghis_khan_synthetic() -> GameConfiguration {
        Self::synthetic_title(
            dragonlink_model::fixtures::genghis_khan_synthetic(),
            "Genghis Khan",
        )
    }

    /// A **synthetic** Golden Gong configuration (`dragonlink_model::fixtures`).
    pub fn golden_gong_synthetic() -> GameConfiguration {
        Self::synthetic_title(
            dragonlink_model::fixtures::golden_gong_synthetic(),
            "Golden Gong",
        )
    }

    /// A **synthetic** Peace & Long Life configuration (`dragonlink_model::fixtures`).
    pub fn peace_long_life_synthetic() -> GameConfiguration {
        Self::synthetic_title(
            dragonlink_model::fixtures::peace_long_life_synthetic(),
            "Peace & Long Life",
        )
    }

    /// A **synthetic** Spring Festival configuration (`dragonlink_model::fixtures`).
    pub fn spring_festival_synthetic() -> GameConfiguration {
        Self::synthetic_title(
            dragonlink_model::fixtures::spring_festival_synthetic(),
            "Spring Festival",
        )
    }

    /// A synthetic title's configuration: every constant synthetic.
    fn synthetic_title(model: MathModel, title: &str) -> GameConfiguration {
        let constants = required_paths(&model)
            .into_iter()
            .map(|path| ConstantRecord {
                path: path.to_string(),
                provenance: ConstantProvenance::Synthetic,
                source: String::new(),
                note: String::new(),
            })
            .collect();
        GameConfiguration {
            schema: SCHEMA.to_string(),
            identity: ConfigIdentity {
                name: format!("{title} (synthetic)"),
                notes: format!(
                    "{title}'s evidenced structure with invented strips, weights and jackpot \
                     amounts. Not recovered from any machine; its statistics are its own."
                ),
                ..ConfigIdentity::default()
            },
            model,
            constants,
            behaviour: Behaviour::default(),
        }
    }

    /// Parse and check a configuration.
    pub fn from_json(text: &str) -> Result<GameConfiguration, ConfigError> {
        let config: GameConfiguration = serde_json::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    /// The configuration as pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Whether this is the project's own invention rather than a description
    /// of an original game.
    pub fn is_synthetic(&self) -> bool {
        self.model.provenance == MathProvenance::SyntheticDevFixture
    }

    /// How many constant groups are of each provenance class.
    pub fn provenance_tally(&self) -> BTreeMap<ConstantProvenance, usize> {
        let mut tally = BTreeMap::new();
        for record in &self.constants {
            *tally.entry(record.provenance).or_insert(0) += 1;
        }
        tally
    }

    /// The weakest provenance among the constants: a result computed from the
    /// configuration is no better supported than this.
    pub fn weakest_provenance(&self) -> ConstantProvenance {
        self.constants
            .iter()
            .map(|r| r.provenance)
            .min()
            .unwrap_or(ConstantProvenance::Unknown)
    }

    /// Check the format, the model, and the provenance rules.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema != SCHEMA {
            return Err(ConfigError::Schema(self.schema.clone()));
        }
        self.model.validate()?;
        let required = required_paths(&self.model);
        let mut seen: Vec<&str> = Vec::new();
        for record in &self.constants {
            let path = record.path.as_str();
            if !required.contains(&path) {
                return Err(ConfigError::UnknownPath(record.path.clone()));
            }
            if seen.contains(&path) {
                return Err(ConfigError::DuplicateRecord(record.path.clone()));
            }
            seen.push(path);
            if record.provenance == ConstantProvenance::Unknown {
                return Err(ConfigError::UnknownProvenance(record.path.clone()));
            }
            let synthetic = record.provenance == ConstantProvenance::Synthetic;
            if synthetic && !self.is_synthetic() {
                return Err(ConfigError::SyntheticInRealModel(record.path.clone()));
            }
            if !synthetic && self.is_synthetic() {
                return Err(ConfigError::RealInSyntheticModel(record.path.clone()));
            }
            if record.provenance.needs_source() && record.source.trim().is_empty() {
                return Err(ConfigError::MissingSource {
                    path: record.path.clone(),
                    class: record.provenance.label(),
                });
            }
        }
        if let Some(missing) = required.iter().find(|p| !seen.contains(*p)) {
            return Err(ConfigError::MissingRecord(missing));
        }
        for (name, finding) in self.behaviour.entries() {
            finding
                .validate()
                .map_err(|source| ConfigError::Behaviour { name, source })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_model::{Confidence, SourceClass};

    fn record<'a>(config: &'a mut GameConfiguration, path: &str) -> &'a mut ConstantRecord {
        config
            .constants
            .iter_mut()
            .find(|r| r.path == path)
            .unwrap()
    }

    /// The fixture relabelled as if it had been recovered, for testing the rules.
    fn pretend_recovered() -> GameConfiguration {
        let mut c = GameConfiguration::synthetic_fixture();
        c.model.provenance = MathProvenance::Cabinet;
        for r in &mut c.constants {
            r.provenance = ConstantProvenance::Extracted;
            r.source = "sha256:test".into();
        }
        c
    }

    #[test]
    fn the_fixture_is_a_valid_synthetic_configuration() {
        let c = GameConfiguration::synthetic_fixture();
        c.validate().unwrap();
        assert!(c.is_synthetic());
        assert_eq!(c.weakest_provenance(), ConstantProvenance::Synthetic);
        assert_eq!(c.provenance_tally().len(), 1);
        for (_, finding) in c.behaviour.entries() {
            assert_eq!(finding.confidence, Confidence::Unknown);
        }
    }

    #[test]
    fn a_configuration_round_trips_through_json() {
        let c = GameConfiguration::synthetic_fixture();
        let back = GameConfiguration::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn every_constant_needs_exactly_one_record() {
        let mut c = GameConfiguration::synthetic_fixture();
        c.constants.retain(|r| r.path != "reel_strips");
        assert!(matches!(
            c.validate(),
            Err(ConfigError::MissingRecord("reel_strips"))
        ));

        let mut c = GameConfiguration::synthetic_fixture();
        let dup = c.constants[0].clone();
        c.constants.push(dup);
        assert!(matches!(c.validate(), Err(ConfigError::DuplicateRecord(_))));

        let mut c = GameConfiguration::synthetic_fixture();
        record(&mut c, "grid").path = "rtp".into();
        assert!(matches!(c.validate(), Err(ConfigError::UnknownPath(_))));
    }

    #[test]
    fn synthetic_and_recovered_constants_cannot_be_mixed() {
        // A recovered-looking constant inside the fixture.
        let mut c = GameConfiguration::synthetic_fixture();
        let r = record(&mut c, "line_pays");
        r.provenance = ConstantProvenance::Extracted;
        r.source = "sha256:test".into();
        assert!(matches!(
            c.validate(),
            Err(ConfigError::RealInSyntheticModel(_))
        ));

        // An invented constant inside a model presented as recovered.
        let mut c = pretend_recovered();
        c.validate().unwrap();
        record(&mut c, "reel_strips").provenance = ConstantProvenance::Synthetic;
        assert!(matches!(
            c.validate(),
            Err(ConfigError::SyntheticInRealModel(_))
        ));
    }

    #[test]
    fn a_valued_constant_cannot_be_of_unknown_provenance_and_sources_are_required() {
        let mut c = pretend_recovered();
        record(&mut c, "bet").provenance = ConstantProvenance::Unknown;
        assert!(matches!(
            c.validate(),
            Err(ConfigError::UnknownProvenance(_))
        ));

        let mut c = pretend_recovered();
        record(&mut c, "bet").source = " ".into();
        assert!(matches!(
            c.validate(),
            Err(ConfigError::MissingSource { .. })
        ));

        // A hypothesis needs no source and is the weakest class present.
        let mut c = pretend_recovered();
        let r = record(&mut c, "hold_and_spin.orb_land_chance");
        r.provenance = ConstantProvenance::Hypothesised;
        r.source.clear();
        c.validate().unwrap();
        assert_eq!(c.weakest_provenance(), ConstantProvenance::Hypothesised);
    }

    #[test]
    fn behaviour_findings_follow_the_evidence_rules() {
        let mut c = GameConfiguration::synthetic_fixture();
        c.behaviour.input_timing = Sourced::known(
            "no effect".to_string(),
            Confidence::Verified,
            SourceClass::ThirdParty,
            "https://example.invalid/forum",
        );
        assert!(matches!(c.validate(), Err(ConfigError::Behaviour { .. })));
    }

    #[test]
    fn a_wrong_schema_is_refused() {
        let mut c = GameConfiguration::synthetic_fixture();
        c.schema = "dragonlink-config/0".into();
        assert!(matches!(c.validate(), Err(ConfigError::Schema(_))));
    }
}
