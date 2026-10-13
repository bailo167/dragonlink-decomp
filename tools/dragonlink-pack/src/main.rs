//! dragonlink-pack — verify an asset set and assemble a local full package.
//!
//! ```text
//! dragonlink-pack verify   [--data-dir DIR]
//! dragonlink-pack assemble --exe FILE --out DIR [--data-dir DIR]
//! ```
//!
//! `verify` checks that the asset set's manifest is valid and that every file
//! it lists is present and unchanged.
//!
//! `assemble` makes a **local full package**: the executable, the verified
//! asset set, and a notice. Such a package contains proprietary material
//! imported from the user's own source material. It is for that user's own
//! machine, must not be distributed, and is never written inside a Git work
//! tree. Public releases are a different thing: they contain this project's
//! code only and are not made with this tool.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dragonlink_assets::{
    AssetManifest, DATA_DIR_VAR, MANIFEST_NAME, SOURCE_ROOT_VAR, data_dir_from, enclosing_git_tree,
    verify,
};

const NOTICE: &str = "LOCAL FULL PACKAGE - DO NOT DISTRIBUTE\n\
======================================\n\n\
This folder was assembled by dragonlink-pack on the machine of the person who\n\
supplied the source material. The files under assets/ were imported from that\n\
person's own legitimately obtained source material and are the property of\n\
their respective owners. They are not part of the dragonlink-decomp project\n\
and may not be redistributed.\n\n\
The software is an unofficial preservation project, unaffiliated with\n\
Aristocrat Leisure Limited or Product Madness. It uses simulation credits\n\
only and is not for real-money gambling.\n";

#[derive(Parser, Debug)]
#[command(about = "Verify a local asset set and assemble a local full package")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Check the asset set against its manifest.
    Verify {
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Assemble the executable and the verified asset set into a local package.
    Assemble {
        /// The built application.
        #[arg(long)]
        exe: PathBuf,
        /// Folder to create the package in. Must be outside any Git work tree.
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

fn resolve_data_dir(arg: Option<PathBuf>) -> Result<PathBuf> {
    match arg {
        Some(dir) => Ok(dir),
        None => data_dir_from(
            std::env::var(DATA_DIR_VAR).ok().as_deref(),
            std::env::var(SOURCE_ROOT_VAR).ok().as_deref(),
        )
        .with_context(|| {
            format!("no asset set: pass --data-dir, or set {DATA_DIR_VAR} or {SOURCE_ROOT_VAR}")
        }),
    }
}

/// Load the manifest of `data_dir` and check every file. Returns the manifest
/// when the asset set is complete and unchanged.
fn verified(data_dir: &Path) -> Result<AssetManifest> {
    if !data_dir.join(MANIFEST_NAME).is_file() {
        bail!(
            "{} has no {MANIFEST_NAME}: nothing has been imported (run dragonlink-import)",
            data_dir.display()
        );
    }
    let manifest = AssetManifest::load(data_dir).context("reading the manifest")?;
    let problems = verify(data_dir, &manifest);
    if !problems.is_empty() {
        for (id, problem) in &problems {
            eprintln!("problem: {id}: {problem:?}");
        }
        bail!(
            "{} of {} assets do not match the manifest",
            problems.len(),
            manifest.entries.len()
        );
    }
    Ok(manifest)
}

fn assemble(exe: &Path, out: &Path, data_dir: &Path) -> Result<usize> {
    if !exe.is_file() {
        bail!("{} is not a file", exe.display());
    }
    let existing = out
        .ancestors()
        .find(|p| p.exists())
        .unwrap_or(Path::new("."));
    if let Some(tree) = existing
        .canonicalize()
        .ok()
        .as_deref()
        .and_then(enclosing_git_tree)
    {
        bail!(
            "refusing to assemble a local full package inside the Git work tree {}: it contains \
             proprietary material",
            tree.display()
        );
    }
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        bail!("{} exists and is not empty", out.display());
    }
    let manifest = verified(data_dir)?;
    let assets = out.join("assets");
    fs::create_dir_all(&assets)?;
    for entry in &manifest.entries {
        let destination = assets.join(&entry.output.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(data_dir.join(&entry.output.path), &destination)
            .with_context(|| format!("copying asset {}", entry.id))?;
    }
    fs::copy(data_dir.join(MANIFEST_NAME), assets.join(MANIFEST_NAME))?;
    let name = exe.file_name().context("the executable has no file name")?;
    fs::copy(exe, out.join(name)).context("copying the executable")?;
    fs::write(out.join("DO-NOT-DISTRIBUTE.txt"), NOTICE)?;
    // The copy is checked too: a package is only as good as what landed in it.
    let problems = verify(&assets, &manifest);
    if !problems.is_empty() {
        bail!("{} assets were not copied correctly", problems.len());
    }
    Ok(manifest.entries.len())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::Verify { data_dir } => {
            let data_dir = resolve_data_dir(data_dir)?;
            let manifest = verified(&data_dir)?;
            println!(
                "asset set verified: {} assets present and unchanged",
                manifest.entries.len()
            );
            Ok(())
        }
        Cmd::Assemble { exe, out, data_dir } => {
            let data_dir = resolve_data_dir(data_dir)?;
            let count = assemble(&exe, &out, &data_dir)?;
            println!(
                "local full package assembled in {} ({count} assets). It contains proprietary \
                 material: do not distribute it.",
                out.display()
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_assets::{AssetCategory, AssetEntry, AssetOutput, AssetSource, hash_file};
    use dragonlink_model::{Confidence, SourceClass};
    use std::collections::BTreeMap;

    /// An asset set of two small synthetic files.
    fn asset_set() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = AssetManifest::new("test");
        for name in ["a", "b"] {
            let path = format!("unclassified/image/{name}.png");
            fs::create_dir_all(dir.path().join("unclassified/image")).unwrap();
            fs::write(dir.path().join(&path), format!("synthetic {name}")).unwrap();
            let (sha256, bytes) = hash_file(&dir.path().join(&path)).unwrap();
            manifest.entries.push(AssetEntry {
                id: format!("unclassified/image/{name}"),
                title: None,
                category: AssetCategory::Image,
                media_type: "image/png".into(),
                source: AssetSource {
                    package: "p".into(),
                    path: format!("{name}.png"),
                    sha256: sha256.clone(),
                },
                output: AssetOutput {
                    path,
                    sha256,
                    bytes,
                },
                provenance: SourceClass::OfficialMobile,
                confidence: Confidence::Unknown,
                method: "copy".into(),
                properties: BTreeMap::new(),
            });
        }
        fs::write(dir.path().join(MANIFEST_NAME), manifest.to_json().unwrap()).unwrap();
        dir
    }

    #[test]
    fn a_complete_asset_set_verifies_and_a_damaged_one_does_not() {
        let set = asset_set();
        assert_eq!(verified(set.path()).unwrap().entries.len(), 2);
        fs::write(set.path().join("unclassified/image/a.png"), "tampered").unwrap();
        assert!(verified(set.path()).is_err());
        fs::remove_file(set.path().join("unclassified/image/a.png")).unwrap();
        assert!(verified(set.path()).is_err());
        let empty = tempfile::tempdir().unwrap();
        assert!(
            verified(empty.path())
                .unwrap_err()
                .to_string()
                .contains("nothing has been imported")
        );
    }

    #[test]
    fn a_package_holds_the_executable_the_assets_and_the_notice() {
        let set = asset_set();
        let work = tempfile::tempdir().unwrap();
        let exe = work.path().join("dragonlink.bin");
        fs::write(&exe, "not a real executable").unwrap();
        let out = work.path().join("package");
        assert_eq!(assemble(&exe, &out, set.path()).unwrap(), 2);
        assert!(out.join("dragonlink.bin").is_file());
        assert!(out.join("assets").join(MANIFEST_NAME).is_file());
        assert!(out.join("assets/unclassified/image/b.png").is_file());
        let notice = fs::read_to_string(out.join("DO-NOT-DISTRIBUTE.txt")).unwrap();
        assert!(notice.contains("DO NOT DISTRIBUTE"));
        // A second run does not overwrite a package.
        assert!(assemble(&exe, &out, set.path()).is_err());
    }

    #[test]
    fn a_package_is_not_assembled_inside_a_git_tree_or_from_a_damaged_set() {
        let set = asset_set();
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir(repo.path().join(".git")).unwrap();
        let exe = repo.path().join("app.bin");
        fs::write(&exe, "x").unwrap();
        let inside = repo.path().join("dist");
        let err = assemble(&exe, &inside, set.path()).unwrap_err().to_string();
        assert!(err.contains("Git work tree"), "{err}");
        assert!(!inside.exists());

        let work = tempfile::tempdir().unwrap();
        let exe = work.path().join("app.bin");
        fs::write(&exe, "x").unwrap();
        fs::write(set.path().join("unclassified/image/a.png"), "tampered").unwrap();
        assert!(assemble(&exe, &work.path().join("package"), set.path()).is_err());
    }
}
