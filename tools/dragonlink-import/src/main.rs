//! dragonlink-import — source material → normalised local asset set.
//!
//! ```text
//! dragonlink-import <SOURCE> --source-class official_mobile [--data-dir DIR] [--dry-run]
//! ```
//!
//! **Stage 1.** This first stage imports only files that already are a
//! standard media format (PNG, JPEG, WebP, GIF, WAV, Ogg, MP3, FLAC, TrueType,
//! OpenType, JSON), found loose under SOURCE, by copying them unchanged. Each
//! is recorded in the manifest with where it came from, both hashes, and the
//! source class given on the command line. It does not unpack containers and
//! does not know what any file *is*: imported assets get the category of
//! their format, an id built from their hash, no title, and confidence
//! UNKNOWN. Identifying assets (this file is that symbol) is research, and is
//! recorded later as a mapping from hashes to logical ids.
//!
//! The source is read and never modified. The asset set holds proprietary
//! material: it is written outside any Git work tree (by default to the
//! source root's `imported/` folder), and a destination inside one is refused.
//! Running the importer twice on the same material changes nothing.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use dragonlink_assets::{
    AssetCategory, AssetEntry, AssetManifest, AssetOutput, AssetSource, DATA_DIR_VAR,
    MANIFEST_NAME, SOURCE_ROOT_VAR, data_dir_from, enclosing_git_tree, hash_file,
};
use dragonlink_formats::{FileKind, SNIFF_BYTES, sniff};
use dragonlink_model::{Confidence, SourceClass};

#[derive(Parser, Debug)]
#[command(about = "Import recognised media from your own source material into a local asset set")]
struct Args {
    /// Folder of source material to import from.
    source: PathBuf,
    /// What kind of source material this is: official_mobile, cabinet,
    /// gameplay_recording or official_media.
    #[arg(long, value_parser = parse_source_class)]
    source_class: SourceClass,
    /// The asset set to write. Default: DRAGONLINK_DATA_DIR, else `imported/`
    /// in DRAGONLINK_SOURCE_ROOT.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Report what would be imported without writing anything.
    #[arg(long)]
    dry_run: bool,
}

fn parse_source_class(text: &str) -> Result<SourceClass, String> {
    match text {
        "official_mobile" => Ok(SourceClass::OfficialMobile),
        "cabinet" => Ok(SourceClass::Cabinet),
        "gameplay_recording" => Ok(SourceClass::GameplayRecording),
        "official_media" => Ok(SourceClass::OfficialMedia),
        other => Err(format!(
            "{other:?} is not a kind of source material (official_mobile, cabinet, gameplay_recording, official_media)"
        )),
    }
}

/// How a recognised file is imported: `(category, folder, extension, media type)`.
fn importable(kind: FileKind) -> Option<(AssetCategory, &'static str, &'static str, &'static str)> {
    use AssetCategory::{Audio, Data, Font, Image};
    Some(match kind {
        FileKind::Png => (Image, "image", "png", "image/png"),
        FileKind::Jpeg => (Image, "image", "jpg", "image/jpeg"),
        FileKind::WebP => (Image, "image", "webp", "image/webp"),
        FileKind::Gif => (Image, "image", "gif", "image/gif"),
        FileKind::Wav => (Audio, "audio", "wav", "audio/wav"),
        FileKind::Ogg => (Audio, "audio", "ogg", "audio/ogg"),
        FileKind::Mp3 => (Audio, "audio", "mp3", "audio/mpeg"),
        FileKind::Flac => (Audio, "audio", "flac", "audio/flac"),
        FileKind::TrueType => (Font, "font", "ttf", "font/ttf"),
        FileKind::OpenType => (Font, "font", "otf", "font/otf"),
        FileKind::Json => (Data, "data", "json", "application/json"),
        _ => return None,
    })
}

/// What one run did.
#[derive(Debug, Default, PartialEq, Eq)]
struct Summary {
    imported: usize,
    already_present: usize,
    /// Files not imported, by what their content looks like.
    skipped: BTreeMap<String, usize>,
}

fn sniff_file(path: &Path) -> Result<FileKind> {
    let mut head = vec![0u8; SNIFF_BYTES];
    let mut file = fs::File::open(path)?;
    let mut filled = 0;
    while filled < head.len() {
        let n = file.read(&mut head[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(sniff(&head[..filled]))
}

/// Import from `source` into the asset set at `data_dir`. `package` is how
/// the source is named in the manifest.
fn import(
    source: &Path,
    package: &str,
    class: SourceClass,
    data_dir: &Path,
    dry_run: bool,
) -> Result<Summary> {
    if !source.is_dir() {
        bail!("{} is not a folder", source.display());
    }
    let mut manifest = if data_dir.join(MANIFEST_NAME).is_file() {
        AssetManifest::load(data_dir).context("reading the existing manifest")?
    } else {
        AssetManifest::new(concat!("dragonlink-import ", env!("CARGO_PKG_VERSION")))
    };
    let mut summary = Summary::default();
    for entry in walkdir::WalkDir::new(source).sort_by_file_name() {
        let entry = entry.context("walking the source folder")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(source)
            .unwrap_or(entry.path())
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let kind = sniff_file(entry.path()).with_context(|| format!("reading {rel}"))?;
        let Some((category, folder, extension, media_type)) = importable(kind) else {
            *summary.skipped.entry(format!("{kind:?}")).or_insert(0) += 1;
            continue;
        };
        let (sha256, bytes) = hash_file(entry.path()).with_context(|| format!("hashing {rel}"))?;
        let id = format!("unclassified/{folder}/{}", &sha256[..16]);
        if manifest.get(&id).is_some() {
            summary.already_present += 1;
            continue;
        }
        let output_path = format!("{id}.{extension}");
        if !dry_run {
            let destination = data_dir.join(&output_path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &destination)
                .with_context(|| format!("copying {rel} into the asset set"))?;
        }
        manifest.entries.push(AssetEntry {
            id,
            title: None,
            category,
            media_type: media_type.to_string(),
            source: AssetSource {
                package: package.to_string(),
                path: rel,
                sha256: sha256.clone(),
            },
            output: AssetOutput {
                path: output_path,
                sha256,
                bytes,
            },
            provenance: class,
            confidence: Confidence::Unknown,
            method: "copy".to_string(),
            properties: BTreeMap::new(),
        });
        summary.imported += 1;
    }
    if !dry_run {
        manifest.entries.sort_by(|a, b| a.id.cmp(&b.id));
        manifest
            .validate()
            .context("the manifest that would be written")?;
        fs::create_dir_all(data_dir)?;
        fs::write(data_dir.join(MANIFEST_NAME), manifest.to_json()?)?;
    }
    Ok(summary)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let source_root = std::env::var(SOURCE_ROOT_VAR).ok();
    let data_dir = match args.data_dir {
        Some(dir) => dir,
        None => data_dir_from(
            std::env::var(DATA_DIR_VAR).ok().as_deref(),
            source_root.as_deref(),
        )
        .with_context(|| {
            format!(
                "no asset set to write to: pass --data-dir, or set {DATA_DIR_VAR} or \
                     {SOURCE_ROOT_VAR} (docs/SOURCES.md)"
            )
        })?,
    };
    let existing = data_dir
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
            "refusing to write an asset set into the Git work tree {}: it holds proprietary material",
            tree.display()
        );
    }
    // Name the source relative to the source root when it lies inside it.
    let package = source_root
        .as_deref()
        .and_then(|root| Path::new(root).canonicalize().ok())
        .zip(args.source.canonicalize().ok())
        .and_then(|(root, source)| source.strip_prefix(&root).ok().map(Path::to_path_buf))
        .map(|rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .filter(|rel| !rel.is_empty())
        .or_else(|| {
            args.source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "source".to_string());

    let summary = import(
        &args.source,
        &package,
        args.source_class,
        &data_dir,
        args.dry_run,
    )?;
    println!(
        "{}: {} imported, {} already present, {} not importable yet",
        if args.dry_run { "dry run" } else { "import" },
        summary.imported,
        summary.already_present,
        summary.skipped.values().sum::<usize>()
    );
    for (kind, n) in &summary.skipped {
        println!("  skipped {kind:<22} {n}");
    }
    if !args.dry_run {
        println!("asset set: {}", data_dir.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonlink_assets::verify;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-synthetic-test-bytes";

    fn source() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("art")).unwrap();
        fs::write(dir.path().join("art/a.png"), PNG).unwrap();
        // The same bytes under another name and a misleading extension.
        fs::write(dir.path().join("art/copy.dat"), PNG).unwrap();
        fs::write(dir.path().join("sound.bin"), b"OggS\0\x02synthetic").unwrap();
        fs::write(dir.path().join("lib.so"), b"\x7FELF\x02\x01\x01").unwrap();
        fs::write(dir.path().join("mystery"), [0x9A, 0x00, 0xF3]).unwrap();
        dir
    }

    #[test]
    fn recognised_media_is_copied_and_recorded_with_its_provenance() {
        let src = source();
        let data = tempfile::tempdir().unwrap();
        let summary = import(
            src.path(),
            "official-mobile/example/1.0",
            SourceClass::OfficialMobile,
            data.path(),
            false,
        )
        .unwrap();
        // Two distinct files are importable; the duplicate bytes are one asset.
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.already_present, 1);
        assert_eq!(summary.skipped.get("Elf"), Some(&1));
        assert_eq!(summary.skipped.get("Unknown"), Some(&1));

        let manifest = AssetManifest::load(data.path()).unwrap();
        assert_eq!(manifest.entries.len(), 2);
        assert!(verify(data.path(), &manifest).is_empty());
        let image = manifest
            .entries
            .iter()
            .find(|e| e.category == AssetCategory::Image)
            .unwrap();
        assert!(image.id.starts_with("unclassified/image/"));
        assert_eq!(image.media_type, "image/png");
        assert_eq!(image.source.package, "official-mobile/example/1.0");
        assert_eq!(image.source.path, "art/a.png");
        assert_eq!(image.provenance, SourceClass::OfficialMobile);
        assert_eq!(image.confidence, Confidence::Unknown);
        assert_eq!(image.title, None);
        assert_eq!(image.source.sha256, image.output.sha256);
        // The content decided the type: the Ogg data came from a .bin file.
        let audio = manifest
            .entries
            .iter()
            .find(|e| e.category == AssetCategory::Audio)
            .unwrap();
        assert!(audio.output.path.ends_with(".ogg"));
        assert_eq!(fs::read(data.path().join(&image.output.path)).unwrap(), PNG);
    }

    #[test]
    fn importing_twice_changes_nothing() {
        let src = source();
        let data = tempfile::tempdir().unwrap();
        import(
            src.path(),
            "p",
            SourceClass::OfficialMobile,
            data.path(),
            false,
        )
        .unwrap();
        let first = fs::read_to_string(data.path().join(MANIFEST_NAME)).unwrap();
        let again = import(
            src.path(),
            "p",
            SourceClass::OfficialMobile,
            data.path(),
            false,
        )
        .unwrap();
        assert_eq!(again.imported, 0);
        assert_eq!(again.already_present, 3);
        assert_eq!(
            fs::read_to_string(data.path().join(MANIFEST_NAME)).unwrap(),
            first
        );
    }

    #[test]
    fn a_dry_run_writes_nothing_and_the_source_is_never_modified() {
        let src = source();
        let before: Vec<_> = walkdir::WalkDir::new(src.path())
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| (e.path().to_path_buf(), fs::read(e.path()).unwrap()))
            .collect();
        let data = tempfile::tempdir().unwrap();
        let summary = import(src.path(), "p", SourceClass::Cabinet, data.path(), true).unwrap();
        assert_eq!(summary.imported, 2);
        assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);
        import(src.path(), "p", SourceClass::Cabinet, data.path(), false).unwrap();
        for (path, bytes) in before {
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn only_real_kinds_of_source_material_are_accepted() {
        assert_eq!(parse_source_class("cabinet"), Ok(SourceClass::Cabinet));
        assert!(parse_source_class("synthetic_fixture").is_err());
        assert!(parse_source_class("third_party").is_err());
        assert!(importable(FileKind::Zip).is_none());
        assert!(importable(FileKind::Unknown).is_none());
    }
}
