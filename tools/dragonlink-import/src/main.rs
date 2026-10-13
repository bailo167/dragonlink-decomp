//! dragonlink-import — source material → normalised local asset set.
//!
//! ```text
//! dragonlink-import <SOURCE> --source-class official_mobile [--data-dir DIR] [--dry-run]
//! ```
//!
//! **Stages 1 and 2.** The importer takes only files that already are a
//! standard media format (PNG, JPEG, WebP, GIF, WAV, Ogg, MP3, FLAC, TrueType,
//! OpenType, JSON), unchanged:
//!
//! 1. found loose under SOURCE, by copying them;
//! 2. found as entries of a ZIP-family archive under SOURCE (ZIP, APK, OBB),
//!    by reading them out within a decompression budget and only when their
//!    size and CRC-32 check (`dragonlink_formats::zip`).
//!
//! Each is recorded in the manifest with where it came from, both hashes, and
//! the source class given on the command line. An archive inside an archive
//! is not opened, an encrypted entry is not decrypted, and no proprietary
//! container is understood. The importer does not know what any file *is*:
//! imported assets get the category of their format, an id built from their
//! hash, no title, and confidence UNKNOWN. Identifying assets (this file is
//! that symbol) is research, and is recorded later as a mapping from hashes
//! to logical ids.
//!
//! The source is read and never modified. The asset set holds proprietary
//! material: it is written outside any Git work tree (by default to the
//! source root's `imported/` folder), and a destination inside one is refused.
//! Running the importer twice on the same material changes nothing.
//!
//! An archive or an entry that cannot be read is reported and passed over;
//! it does not stop the run, and nothing unverified enters the asset set.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use dragonlink_assets::{
    AssetCategory, AssetEntry, AssetManifest, AssetOutput, AssetSource, DATA_DIR_VAR,
    MANIFEST_NAME, SOURCE_ROOT_VAR, data_dir_from, enclosing_git_tree, hash_file,
};
use dragonlink_formats::{
    ExtractBudget, ExtractLimits, FileKind, SNIFF_BYTES, extract_entry, list_zip,
    read_entry_prefix, sniff,
};
use dragonlink_model::{Confidence, SourceClass};
use sha2::{Digest, Sha256};

/// Folder inside the asset set where an entry is written before it is known
/// to be good. Removed at the end of a run.
const INCOMING: &str = ".incoming";

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
    /// Archives that were opened and listed.
    archives: usize,
    /// Archives and entries that could not be read, and why.
    problems: Vec<String>,
}

/// A sink that hashes and counts what passes through it.
struct Hashing<W: Write> {
    inner: W,
    hasher: Sha256,
}

impl<W: Write> Hashing<W> {
    fn new(inner: W) -> Self {
        Hashing {
            inner,
            hasher: Sha256::new(),
        }
    }

    /// The sink, and the SHA-256 of what it was given, in lowercase hexadecimal.
    fn finish(self) -> (W, String) {
        let hex = self
            .hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (self.inner, hex)
    }
}

impl<W: Write> Write for Hashing<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(bytes)?;
        self.hasher.update(&bytes[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// One run: where it writes, what it has recorded and what it has counted.
struct Run<'a> {
    package: &'a str,
    class: SourceClass,
    data_dir: &'a Path,
    dry_run: bool,
    manifest: AssetManifest,
    summary: Summary,
}

impl Run<'_> {
    fn skip(&mut self, what: String) {
        *self.summary.skipped.entry(what).or_insert(0) += 1;
    }

    /// Record an asset whose bytes are (or in a dry run would be) in the asset set.
    fn record(
        &mut self,
        id: String,
        output_path: String,
        (category, media_type): (AssetCategory, &str),
        source_path: String,
        (sha256, bytes): (String, u64),
        method: &str,
    ) {
        self.manifest.entries.push(AssetEntry {
            id,
            title: None,
            category,
            media_type: media_type.to_string(),
            source: AssetSource {
                package: self.package.to_string(),
                path: source_path,
                sha256: sha256.clone(),
            },
            output: AssetOutput {
                path: output_path,
                sha256,
                bytes,
            },
            provenance: self.class,
            confidence: Confidence::Unknown,
            method: method.to_string(),
            properties: BTreeMap::new(),
        });
        self.summary.imported += 1;
    }

    /// Import the standard media among the entries of the archive at `path`.
    /// `rel` is the archive's path inside the source.
    fn import_archive(&mut self, path: &Path, rel: &str) -> Result<()> {
        let mut file = fs::File::open(path)?;
        let listing = match list_zip(&mut file) {
            Ok(listing) => listing,
            Err(error) => {
                self.summary.problems.push(format!("{rel}: {error}"));
                self.skip("Zip, not listable".to_string());
                return Ok(());
            }
        };
        self.summary.archives += 1;
        let mut budget = ExtractBudget::new(ExtractLimits::default());
        for entry in listing.entries.iter().filter(|e| !e.is_directory()) {
            let inside = format!("{rel}!/{}", entry.name);
            let head = match read_entry_prefix(&mut file, entry, &mut budget, SNIFF_BYTES) {
                Ok(head) => head,
                Err(error) => {
                    self.summary.problems.push(format!("{rel}: {error}"));
                    self.skip("entry, not readable".to_string());
                    continue;
                }
            };
            let kind = sniff(&head);
            let Some((category, folder, extension, media_type)) = importable(kind) else {
                self.skip(format!("{kind:?}, in an archive"));
                continue;
            };
            // The id is built from the hash, so the entry is read out before
            // it is known whether the asset set already has it: to nowhere in
            // a dry run, otherwise to a file of its own that becomes the
            // asset only if the entry verifies.
            let incoming = self.data_dir.join(INCOMING).join("entry");
            let read = if self.dry_run {
                let mut sink = Hashing::new(std::io::sink());
                extract_entry(&mut file, entry, &mut budget, &mut sink)
                    .map(|bytes| (sink.finish().1, bytes))
            } else {
                fs::create_dir_all(self.data_dir.join(INCOMING))?;
                let mut sink = Hashing::new(std::io::BufWriter::new(fs::File::create(&incoming)?));
                let outcome = extract_entry(&mut file, entry, &mut budget, &mut sink);
                let (mut writer, sha256) = sink.finish();
                writer.flush()?;
                outcome.map(|bytes| (sha256, bytes))
            };
            let (sha256, bytes) = match read {
                Ok(read) => read,
                Err(error) => {
                    self.summary.problems.push(format!("{rel}: {error}"));
                    self.skip("entry, not readable".to_string());
                    continue;
                }
            };
            let id = format!("unclassified/{folder}/{}", &sha256[..16]);
            if self.manifest.get(&id).is_some() {
                self.summary.already_present += 1;
                continue;
            }
            let output_path = format!("{id}.{extension}");
            if !self.dry_run {
                let destination = self.data_dir.join(&output_path);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&incoming, &destination)
                    .with_context(|| format!("moving {inside} into the asset set"))?;
            }
            self.record(
                id,
                output_path,
                (category, media_type),
                inside,
                (sha256, bytes),
                "extract:zip",
            );
        }
        Ok(())
    }
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
    let manifest = if data_dir.join(MANIFEST_NAME).is_file() {
        AssetManifest::load(data_dir).context("reading the existing manifest")?
    } else {
        AssetManifest::new(concat!("dragonlink-import ", env!("CARGO_PKG_VERSION")))
    };
    let mut run = Run {
        package,
        class,
        data_dir,
        dry_run,
        manifest,
        summary: Summary::default(),
    };
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
            if kind.is_zip_family() {
                run.import_archive(entry.path(), &rel)
                    .with_context(|| format!("reading the archive {rel}"))?;
            } else {
                run.skip(format!("{kind:?}"));
            }
            continue;
        };
        let (sha256, bytes) = hash_file(entry.path()).with_context(|| format!("hashing {rel}"))?;
        let id = format!("unclassified/{folder}/{}", &sha256[..16]);
        if run.manifest.get(&id).is_some() {
            run.summary.already_present += 1;
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
        run.record(
            id,
            output_path,
            (category, media_type),
            rel,
            (sha256, bytes),
            "copy",
        );
    }
    let Run {
        mut manifest,
        summary,
        ..
    } = run;
    if !dry_run {
        let incoming = data_dir.join(INCOMING);
        if incoming.is_dir() {
            fs::remove_dir_all(&incoming)?;
        }
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
    if summary.archives > 0 {
        println!("  archives read            {}", summary.archives);
    }
    for (kind, n) in &summary.skipped {
        println!("  skipped {kind:<28} {n}");
    }
    for problem in &summary.problems {
        println!("  not read: {problem}");
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

    /// A ZIP archive written by hand: each entry is `(name, method, payload,
    /// the bytes the payload stands for)`.
    fn zip_of(entries: &[(&str, u16, Vec<u8>, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, method, payload, data) in entries {
            let offset = out.len() as u32;
            let mut fields = Vec::new();
            fields.extend_from_slice(&[20, 0, 0, 0]); // version needed, flags
            fields.extend_from_slice(&method.to_le_bytes());
            fields.extend_from_slice(&[0; 4]); // time, date
            fields.extend_from_slice(&dragonlink_formats::crc32(data).to_le_bytes());
            fields.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            fields.extend_from_slice(&(data.len() as u32).to_le_bytes());
            fields.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&fields);
            out.extend_from_slice(&[0, 0]); // extra
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(payload);
            directory.extend_from_slice(b"PK\x01\x02");
            directory.extend_from_slice(&[20, 0]); // version made by
            directory.extend_from_slice(&fields);
            directory.extend_from_slice(&[0; 12]); // extra, comment, disk, attributes
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }
        let directory_offset = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    fn stored<'a>(name: &'a str, data: &'a [u8]) -> (&'a str, u16, Vec<u8>, &'a [u8]) {
        (name, 0, data.to_vec(), data)
    }

    fn deflated<'a>(name: &'a str, data: &'a [u8]) -> (&'a str, u16, Vec<u8>, &'a [u8]) {
        (
            name,
            8,
            miniz_oxide::deflate::compress_to_vec(data, 6),
            data,
        )
    }

    const OGG: &[u8] = b"OggS\0\x02synthetic-sound-in-an-archive";

    #[test]
    fn media_inside_an_archive_is_read_out_verified_and_recorded() {
        let json = br#"{"synthetic": true, "note": "written by the test"}"#.repeat(40);
        let inner = zip_of(&[stored("deep.png", PNG)]);
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("pkg")).unwrap();
        fs::write(
            src.path().join("pkg/base.apk"),
            zip_of(&[
                stored("res/raw/win.ogg", OGG),
                deflated("assets/table.json", &json),
                stored("assets/", b""),
                stored("lib/arm64-v8a/libgame.so", b"\x7FELF\x02\x01\x01"),
                // The same bytes as a loose file: one asset, not two.
                deflated("res/drawable/a.png", PNG),
                stored("assets/nested.zip", &inner),
            ]),
        )
        .unwrap();
        fs::write(src.path().join("a.png"), PNG).unwrap();
        let before = fs::read(src.path().join("pkg/base.apk")).unwrap();

        let data = tempfile::tempdir().unwrap();
        let dry = import(
            src.path(),
            "p",
            SourceClass::OfficialMobile,
            data.path(),
            true,
        )
        .unwrap();
        assert_eq!(dry.imported, 3);
        assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);

        let summary = import(
            src.path(),
            "p",
            SourceClass::OfficialMobile,
            data.path(),
            false,
        )
        .unwrap();
        assert_eq!(summary.imported, 3);
        assert_eq!(summary.already_present, 1);
        assert_eq!(summary.archives, 1);
        assert_eq!(summary.skipped.get("Elf, in an archive"), Some(&1));
        // An archive inside an archive is not opened.
        assert_eq!(summary.skipped.get("Zip, in an archive"), Some(&1));
        assert!(summary.problems.is_empty(), "{:?}", summary.problems);

        let manifest = AssetManifest::load(data.path()).unwrap();
        assert_eq!(manifest.entries.len(), 3);
        assert!(verify(data.path(), &manifest).is_empty());
        let sound = manifest
            .entries
            .iter()
            .find(|e| e.category == AssetCategory::Audio)
            .unwrap();
        assert_eq!(sound.source.path, "pkg/base.apk!/res/raw/win.ogg");
        assert_eq!(sound.method, "extract:zip");
        assert_eq!(sound.provenance, SourceClass::OfficialMobile);
        assert_eq!(sound.confidence, Confidence::Unknown);
        assert_eq!(fs::read(data.path().join(&sound.output.path)).unwrap(), OGG);
        let table = manifest
            .entries
            .iter()
            .find(|e| e.category == AssetCategory::Data)
            .unwrap();
        assert_eq!(table.source.path, "pkg/base.apk!/assets/table.json");
        assert_eq!(table.output.bytes, json.len() as u64);
        assert_eq!(
            fs::read(data.path().join(&table.output.path)).unwrap(),
            json
        );
        // The loose copy was seen first and is the recorded source of the image.
        let image = manifest
            .entries
            .iter()
            .find(|e| e.category == AssetCategory::Image)
            .unwrap();
        assert_eq!(image.source.path, "a.png");
        assert_eq!(image.method, "copy");

        // Nothing is left behind, the archive is untouched, and a second run changes nothing.
        assert!(!data.path().join(INCOMING).exists());
        assert_eq!(fs::read(src.path().join("pkg/base.apk")).unwrap(), before);
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
        assert_eq!(again.already_present, 4);
        assert_eq!(
            fs::read_to_string(data.path().join(MANIFEST_NAME)).unwrap(),
            first
        );
    }

    #[test]
    fn a_damaged_entry_is_reported_and_never_enters_the_asset_set() {
        let good = br#"{"good": true}"#.repeat(20);
        let bad = br#"{"good": false}"#.repeat(20);
        let mut wrong_crc = deflated("bad-crc.json", &bad);
        wrong_crc.3 = &b"{ not what the payload holds }"[..];
        let src = tempfile::tempdir().unwrap();
        fs::write(
            src.path().join("pack.zip"),
            zip_of(&[wrong_crc, deflated("good.json", &good)]),
        )
        .unwrap();
        // Looks like an archive, and is not one.
        fs::write(
            src.path().join("broken.zip"),
            b"PK\x03\x04 and then nothing",
        )
        .unwrap();

        let data = tempfile::tempdir().unwrap();
        let summary = import(src.path(), "p", SourceClass::Cabinet, data.path(), false).unwrap();
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.archives, 1);
        assert_eq!(summary.skipped.get("entry, not readable"), Some(&1));
        assert_eq!(summary.skipped.get("Zip, not listable"), Some(&1));
        assert_eq!(summary.problems.len(), 2, "{:?}", summary.problems);
        assert!(summary.problems.iter().any(|p| p.starts_with("pack.zip: ")));
        assert!(
            summary
                .problems
                .iter()
                .any(|p| p.starts_with("broken.zip: "))
        );

        let manifest = AssetManifest::load(data.path()).unwrap();
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].source.path, "pack.zip!/good.json");
        assert!(verify(data.path(), &manifest).is_empty());
        // Only the manifest and the one asset's folder exist.
        assert!(!data.path().join(INCOMING).exists());
        let files = walkdir::WalkDir::new(data.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();
        assert_eq!(files, 2);
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
