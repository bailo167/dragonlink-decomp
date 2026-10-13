//! The asset manifest of a normalised local asset set.
//!
//! The runtime never reads source material. An importer converts what it
//! recognises into an **asset set**: a folder of normalised files with a
//! manifest, `manifest.json`, that records for every asset where it came
//! from and what it became (`docs/SOURCES.md`). The asset set is produced on
//! the user's machine from their own source material and lives outside the
//! repository.
//!
//! This crate defines the manifest, checks it, verifies an asset set against
//! it, and finds the asset set at run time. Without one the application runs
//! in placeholder mode, and [`discover`] says why in plain words.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use dragonlink_model::{Confidence, SourceClass, TitleId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Identifier of the manifest format.
pub const SCHEMA: &str = "dragonlink-assets/1";
/// File name of the manifest inside an asset set.
pub const MANIFEST_NAME: &str = "manifest.json";
/// Environment variable naming the asset set.
pub const DATA_DIR_VAR: &str = "DRAGONLINK_DATA_DIR";
/// Environment variable naming the private source root.
pub const SOURCE_ROOT_VAR: &str = "DRAGONLINK_SOURCE_ROOT";
/// The asset set's default folder inside the source root.
pub const DEFAULT_DATA_FOLDER: &str = "imported";

/// What an asset is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Image,
    Audio,
    Font,
    Animation,
    Data,
    /// Imported but not yet identified as anything in particular.
    Unclassified,
}

/// Where an asset came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSource {
    /// The package or folder the file was found in, relative to the source root.
    pub package: String,
    /// Path of the file inside that package.
    pub path: String,
    pub sha256: String,
}

/// What an asset became in the asset set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetOutput {
    /// Path relative to the asset set, with forward slashes.
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// One asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetEntry {
    /// Logical id the runtime asks for (`golden-century/symbol/wild`). Unique.
    pub id: String,
    #[serde(default)]
    pub title: Option<TitleId>,
    pub category: AssetCategory,
    /// Media type of the output (`image/png`, `audio/ogg`, …).
    pub media_type: String,
    pub source: AssetSource,
    pub output: AssetOutput,
    /// Which kind of source material this is. Mobile and cabinet stay distinguishable.
    pub provenance: SourceClass,
    /// How sure we are that this asset is what its id says it is.
    pub confidence: Confidence,
    /// How it was produced: `copy`, `transcode:…`, `extract:…`.
    pub method: String,
    /// Width, height, sample rate and the like.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

/// The manifest of an asset set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetManifest {
    pub schema: String,
    /// The tool and version that wrote the manifest.
    pub generated_by: String,
    pub entries: Vec<AssetEntry>,
}

/// Why a manifest or an asset set is not acceptable.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("i/o on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("the manifest is not valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema {0:?} is not {SCHEMA:?}")]
    Schema(String),
    #[error("asset id {0:?} is used twice")]
    DuplicateId(String),
    #[error("asset id {0:?} is empty or has characters outside a-z, 0-9, '-', '_', '.' and '/'")]
    BadId(String),
    #[error("asset {id:?}: output path {path:?} is absolute or leaves the asset set")]
    UnsafePath { id: String, path: String },
    #[error("asset {id:?}: {what} is not a SHA-256 in lowercase hexadecimal")]
    BadHash { id: String, what: &'static str },
    #[error(
        "asset {id:?} is a synthetic fixture or a conjecture; an asset set holds imported material only"
    )]
    NotImported { id: String },
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn path_is_safe(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn id_is_valid(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_./".contains(&b))
        && path_is_safe(id)
}

impl AssetManifest {
    pub fn new(generated_by: impl Into<String>) -> Self {
        AssetManifest {
            schema: SCHEMA.to_string(),
            generated_by: generated_by.into(),
            entries: Vec::new(),
        }
    }

    /// Check the format and every entry. Says nothing about whether the files exist.
    pub fn validate(&self) -> Result<(), AssetError> {
        if self.schema != SCHEMA {
            return Err(AssetError::Schema(self.schema.clone()));
        }
        let mut ids = std::collections::BTreeSet::new();
        for e in &self.entries {
            if !id_is_valid(&e.id) {
                return Err(AssetError::BadId(e.id.clone()));
            }
            if !ids.insert(e.id.as_str()) {
                return Err(AssetError::DuplicateId(e.id.clone()));
            }
            if !path_is_safe(&e.output.path) {
                return Err(AssetError::UnsafePath {
                    id: e.id.clone(),
                    path: e.output.path.clone(),
                });
            }
            for (hash, what) in [
                (&e.source.sha256, "the source hash"),
                (&e.output.sha256, "the output hash"),
            ] {
                if !is_sha256(hash) {
                    return Err(AssetError::BadHash {
                        id: e.id.clone(),
                        what,
                    });
                }
            }
            if matches!(
                e.provenance,
                SourceClass::SyntheticFixture | SourceClass::Hypothesis
            ) {
                return Err(AssetError::NotImported { id: e.id.clone() });
            }
        }
        Ok(())
    }

    pub fn from_json(text: &str) -> Result<AssetManifest, AssetError> {
        let manifest: AssetManifest = serde_json::from_str(text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, AssetError> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// Read and check the manifest of the asset set in `dir`.
    pub fn load(dir: &Path) -> Result<AssetManifest, AssetError> {
        let path = dir.join(MANIFEST_NAME);
        let text = fs::read_to_string(&path).map_err(|source| AssetError::Io {
            path: MANIFEST_NAME.to_string(),
            source,
        })?;
        AssetManifest::from_json(&text)
    }

    /// The entry with this logical id.
    pub fn get(&self, id: &str) -> Option<&AssetEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

/// SHA-256 of a file, in lowercase hexadecimal, with its size.
pub fn hash_file(path: &Path) -> std::io::Result<(String, u64)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        total += n as u64;
    }
    let hex = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((hex, total))
}

/// What is wrong with one asset of an asset set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Problem {
    Missing,
    /// The file is there but is not the file the manifest describes.
    Changed {
        expected: String,
        found: String,
    },
    Unreadable {
        reason: String,
    },
}

/// Compare every file of the asset set in `dir` with the manifest.
/// Returns `(asset id, problem)` for each asset that does not match.
pub fn verify(dir: &Path, manifest: &AssetManifest) -> Vec<(String, Problem)> {
    let mut problems = Vec::new();
    for entry in &manifest.entries {
        let path = dir.join(&entry.output.path);
        if !path.is_file() {
            problems.push((entry.id.clone(), Problem::Missing));
            continue;
        }
        match hash_file(&path) {
            Ok((found, _)) if found == entry.output.sha256 => {}
            Ok((found, _)) => problems.push((
                entry.id.clone(),
                Problem::Changed {
                    expected: entry.output.sha256.clone(),
                    found,
                },
            )),
            Err(e) => problems.push((
                entry.id.clone(),
                Problem::Unreadable {
                    reason: e.to_string(),
                },
            )),
        }
    }
    problems
}

/// The nearest ancestor of `path` (or `path` itself) that is a Git work tree.
/// Asset sets and local packages hold proprietary material and must not be
/// written inside one.
pub fn enclosing_git_tree(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// Where the assets come from at run time.
#[derive(Debug)]
pub enum AssetMode {
    /// An asset set was found and its manifest is valid.
    Imported {
        dir: PathBuf,
        manifest: AssetManifest,
    },
    /// No usable asset set. `reason` is written for the person at the keyboard.
    Placeholder { reason: String },
}

/// The asset set named by the given variables, if any. `data_dir` wins;
/// otherwise the default folder inside `source_root`.
pub fn data_dir_from(data_dir: Option<&str>, source_root: Option<&str>) -> Option<PathBuf> {
    match (
        data_dir.filter(|s| !s.is_empty()),
        source_root.filter(|s| !s.is_empty()),
    ) {
        (Some(d), _) => Some(PathBuf::from(d)),
        (None, Some(root)) => Some(Path::new(root).join(DEFAULT_DATA_FOLDER)),
        (None, None) => None,
    }
}

/// Find the asset set from the environment. Never fails: the absence of
/// assets is a mode, with the reason stated, not an error.
pub fn discover() -> AssetMode {
    let data = std::env::var(DATA_DIR_VAR).ok();
    let root = std::env::var(SOURCE_ROOT_VAR).ok();
    discover_in(data_dir_from(data.as_deref(), root.as_deref()))
}

/// As [`discover`], for a given folder.
pub fn discover_in(dir: Option<PathBuf>) -> AssetMode {
    let Some(dir) = dir else {
        return AssetMode::Placeholder {
            reason: format!(
                "No asset set is configured. Set {DATA_DIR_VAR}, or {SOURCE_ROOT_VAR}, and import \
                 your own source material (docs/SOURCES.md)."
            ),
        };
    };
    if !dir.join(MANIFEST_NAME).is_file() {
        return AssetMode::Placeholder {
            reason: format!(
                "The asset set folder has no {MANIFEST_NAME}: nothing has been imported yet. Run \
                 dragonlink-import on your own source material (docs/SOURCES.md)."
            ),
        };
    }
    match AssetManifest::load(&dir) {
        Ok(manifest) if manifest.entries.is_empty() => AssetMode::Placeholder {
            reason: "The asset set's manifest lists no assets.".to_string(),
        },
        Ok(manifest) => AssetMode::Imported { dir, manifest },
        Err(e) => AssetMode::Placeholder {
            reason: format!("The asset set's manifest cannot be used: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn entry(id: &str, path: &str) -> AssetEntry {
        AssetEntry {
            id: id.to_string(),
            title: Some(TitleId::GoldenCentury),
            category: AssetCategory::Image,
            media_type: "image/png".into(),
            source: AssetSource {
                package: "official-mobile/example/1.0".into(),
                path: "assets/a.png".into(),
                sha256: HASH_ABC.into(),
            },
            output: AssetOutput {
                path: path.to_string(),
                sha256: HASH_ABC.into(),
                bytes: 3,
            },
            provenance: SourceClass::OfficialMobile,
            confidence: Confidence::Unknown,
            method: "copy".into(),
            properties: BTreeMap::new(),
        }
    }

    fn manifest(entries: Vec<AssetEntry>) -> AssetManifest {
        AssetManifest {
            entries,
            ..AssetManifest::new("test")
        }
    }

    #[test]
    fn a_manifest_round_trips_and_is_looked_up_by_id() {
        let m = manifest(vec![entry("golden-century/symbol/a", "images/a.png")]);
        m.validate().unwrap();
        let back = AssetManifest::from_json(&m.to_json().unwrap()).unwrap();
        assert_eq!(back, m);
        assert!(back.get("golden-century/symbol/a").is_some());
        assert!(back.get("missing").is_none());
    }

    #[test]
    fn ids_paths_and_hashes_are_checked() {
        let dup = manifest(vec![entry("a", "a.png"), entry("a", "b.png")]);
        assert!(matches!(dup.validate(), Err(AssetError::DuplicateId(_))));
        for bad_id in ["", "Upper", "a b", "../a", "a//b"] {
            let m = manifest(vec![entry(bad_id, "a.png")]);
            assert!(
                matches!(m.validate(), Err(AssetError::BadId(_))),
                "{bad_id:?}"
            );
        }
        for bad_path in [
            "../outside.png",
            "/abs.png",
            "c:/x.png",
            "a\\b.png",
            "a/./b.png",
            "",
        ] {
            let m = manifest(vec![entry("a", bad_path)]);
            assert!(
                matches!(m.validate(), Err(AssetError::UnsafePath { .. })),
                "{bad_path:?}"
            );
        }
        let mut e = entry("a", "a.png");
        e.output.sha256 = "ABC".into();
        assert!(matches!(
            manifest(vec![e]).validate(),
            Err(AssetError::BadHash { .. })
        ));
        let mut m = manifest(vec![]);
        m.schema = "dragonlink-assets/0".into();
        assert!(matches!(m.validate(), Err(AssetError::Schema(_))));
    }

    #[test]
    fn an_asset_set_holds_imported_material_only() {
        let mut e = entry("a", "a.png");
        e.provenance = SourceClass::SyntheticFixture;
        assert!(matches!(
            manifest(vec![e]).validate(),
            Err(AssetError::NotImported { .. })
        ));
    }

    #[test]
    fn verification_finds_missing_and_changed_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("images")).unwrap();
        fs::write(dir.path().join("images/ok.png"), b"abc").unwrap();
        fs::write(dir.path().join("images/changed.png"), b"abd").unwrap();
        let m = manifest(vec![
            entry("ok", "images/ok.png"),
            entry("changed", "images/changed.png"),
            entry("missing", "images/missing.png"),
        ]);
        assert_eq!(
            hash_file(&dir.path().join("images/ok.png")).unwrap(),
            (HASH_ABC.to_string(), 3)
        );
        let problems = verify(dir.path(), &m);
        assert_eq!(problems.len(), 2);
        assert!(matches!(&problems[0], (id, Problem::Changed { .. }) if id == "changed"));
        assert_eq!(problems[1], ("missing".to_string(), Problem::Missing));
    }

    #[test]
    fn the_data_folder_comes_from_the_environment_in_a_fixed_order() {
        assert_eq!(data_dir_from(None, None), None);
        assert_eq!(data_dir_from(Some(""), Some("")), None);
        assert_eq!(
            data_dir_from(Some("d"), Some("r")),
            Some(PathBuf::from("d"))
        );
        assert_eq!(
            data_dir_from(None, Some("r")),
            Some(Path::new("r").join("imported"))
        );
    }

    #[test]
    fn missing_assets_are_a_mode_with_a_reason_not_an_error() {
        let AssetMode::Placeholder { reason } = discover_in(None) else {
            panic!("expected placeholder mode");
        };
        assert!(reason.contains(DATA_DIR_VAR));

        let dir = tempfile::tempdir().unwrap();
        let AssetMode::Placeholder { reason } = discover_in(Some(dir.path().to_path_buf())) else {
            panic!("expected placeholder mode");
        };
        assert!(reason.contains("nothing has been imported"));

        fs::write(dir.path().join(MANIFEST_NAME), "{broken").unwrap();
        let AssetMode::Placeholder { reason } = discover_in(Some(dir.path().to_path_buf())) else {
            panic!("expected placeholder mode");
        };
        assert!(reason.contains("cannot be used"));

        let m = manifest(vec![entry("a", "a.png")]);
        fs::write(dir.path().join(MANIFEST_NAME), m.to_json().unwrap()).unwrap();
        assert!(matches!(
            discover_in(Some(dir.path().to_path_buf())),
            AssetMode::Imported { .. }
        ));
    }
}
