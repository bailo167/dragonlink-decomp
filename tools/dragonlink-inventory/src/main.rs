//! dragonlink-inventory — the private source root, and inventories of what is in it.
//!
//! ```text
//! dragonlink-inventory init  <ROOT>            # create the source root layout
//! dragonlink-inventory check                   # is DRAGONLINK_SOURCE_ROOT usable?
//! dragonlink-inventory scan  <PATH> [--out F]  # hash and classify everything under PATH
//! ```
//!
//! The source root holds original material and lives outside any Git work
//! tree (`docs/SOURCES.md`). `init` and `check` refuse a location inside one.
//!
//! `scan` reads files and never modifies them. For each file it records the
//! path relative to the scanned folder, size, SHA-256, what the content looks
//! like, entropy, and for ZIP-family packages the list of entries. The output
//! contains no absolute path and no file content, so it is sanitized metadata
//! in the sense of `docs/EVIDENCE.md`. It is still written to the source
//! root's `manifests/` folder by default; publishing any part of it is a
//! deliberate step.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use dragonlink_formats::{ByteHistogram, FileKind, SNIFF_BYTES, ZipListing, list_zip, sniff};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Environment variable naming the private source root.
const ROOT_VAR: &str = "DRAGONLINK_SOURCE_ROOT";
/// Marker file written into a source root.
const MARKER: &str = "dragonlink-source-root.toml";
/// Identifier of the inventory format.
const SCHEMA: &str = "dragonlink-inventory/1";

/// The folders of a source root and what each is for (`docs/SOURCES.md`).
const LAYOUT: &[(&str, &str)] = &[
    (
        "official-mobile",
        "official app packages exactly as obtained, one folder per app and version",
    ),
    (
        "cabinet",
        "cabinet software and machine images exactly as obtained",
    ),
    (
        "captures",
        "recordings and screenshots of the original being played",
    ),
    (
        "extracted",
        "working copies and unpacked contents, one folder per source",
    ),
    ("decompiler", "Ghidra / JADX / Apktool projects and output"),
    (
        "audio",
        "loose reference audio that is not part of a package",
    ),
    (
        "images",
        "loose reference images that are not part of a package",
    ),
    (
        "video",
        "loose reference video that is not part of a package",
    ),
    (
        "manifests",
        "inventories and manifests produced by the tools",
    ),
    ("hashes", "hash lists of originals, for integrity checks"),
    (
        "math",
        "recovered game configurations and reports generated from them",
    ),
    (
        "research",
        "private notes, strings dumps, anything not fit to publish",
    ),
    (
        "imported",
        "the normalised runtime asset set (default DRAGONLINK_DATA_DIR)",
    ),
    ("packages", "local full packages (never distributed)"),
    ("temporary", "scratch; safe to delete"),
];

#[derive(Parser, Debug)]
#[command(about = "Create and check the private source root; inventory source material")]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Create the source root layout at ROOT (existing folders are left alone).
    Init { root: PathBuf },
    /// Check that DRAGONLINK_SOURCE_ROOT points at a usable source root.
    Check,
    /// Hash and classify every file under PATH.
    Scan {
        path: PathBuf,
        /// Where to write the inventory. Default: the source root's manifests folder.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Do not list the entries of ZIP-family packages.
        #[arg(long)]
        no_zip: bool,
    },
}

/// The nearest ancestor of `path` (or `path` itself) that is a Git work tree.
fn enclosing_git_tree(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

fn init(root: &Path) -> Result<Vec<String>> {
    // Refuse before creating anything: look at the nearest folder that exists.
    let existing = root
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
            "{} is inside the Git work tree {}; the source root must be outside any repository",
            root.display(),
            tree.display()
        );
    }
    fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    let root = root
        .canonicalize()
        .with_context(|| format!("resolving {}", root.display()))?;
    let mut created = Vec::new();
    let mut readme = String::from(
        "dragonlink-decomp private source root\n\
         =====================================\n\n\
         Original material and everything derived from it that is not publishable.\n\
         Never commit anything from here. Originals are immutable: work on copies.\n\
         See docs/SOURCES.md in the repository.\n\n",
    );
    for (name, purpose) in LAYOUT {
        let dir = root.join(name);
        if !dir.exists() {
            fs::create_dir(&dir).with_context(|| format!("creating {}", dir.display()))?;
            created.push((*name).to_string());
        }
        readme.push_str(&format!("{name:<16} {purpose}\n"));
    }
    fs::write(root.join("README.txt"), readme)?;
    let marker = root.join(MARKER);
    if !marker.exists() {
        let created_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        fs::write(
            &marker,
            format!(
                "# Marks this folder as a dragonlink-decomp private source root.\nschema = 1\ncreated_unix = {created_unix}\n"
            ),
        )?;
    }
    Ok(created)
}

/// Problems with `root` as a source root; empty when it is usable.
fn check(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    if !root.is_dir() {
        problems.push("the folder does not exist".to_string());
        return problems;
    }
    if let Some(tree) = root
        .canonicalize()
        .ok()
        .as_deref()
        .and_then(enclosing_git_tree)
    {
        problems.push(format!(
            "it is inside the Git work tree {}; move it outside any repository",
            tree.display()
        ));
    }
    if !root.join(MARKER).is_file() {
        problems.push(format!(
            "no {MARKER}; run `dragonlink-inventory init` on it"
        ));
    }
    for (name, _) in LAYOUT {
        if !root.join(name).is_dir() {
            problems.push(format!("folder {name}/ is missing"));
        }
    }
    problems
}

/// One file of an inventory.
#[derive(Debug, Serialize)]
struct FileRecord {
    /// Relative to the scanned folder, with forward slashes.
    path: String,
    bytes: u64,
    sha256: String,
    kind: FileKind,
    extension: Option<String>,
    /// Bits per byte over the whole file.
    entropy: f64,
    /// Fraction of bytes that are printable ASCII or common whitespace.
    printable: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip: Option<ZipListing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Inventory {
    schema: &'static str,
    /// Last component of the scanned folder. No absolute path is recorded.
    scanned: String,
    file_count: usize,
    total_bytes: u64,
    kinds: BTreeMap<String, usize>,
    files: Vec<FileRecord>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn examine(path: &Path, rel: String, list_zips: bool) -> Result<FileRecord> {
    let mut file = fs::File::open(path).with_context(|| format!("opening {rel}"))?;
    let mut hasher = Sha256::new();
    let mut histogram = ByteHistogram::new();
    let mut head = Vec::with_capacity(SNIFF_BYTES);
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = file
            .read(&mut buffer)
            .with_context(|| format!("reading {rel}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        histogram.update(&buffer[..n]);
        if head.len() < SNIFF_BYTES {
            let take = (SNIFF_BYTES - head.len()).min(n);
            head.extend_from_slice(&buffer[..take]);
        }
    }
    let kind = sniff(&head);
    let (zip, zip_error) = if list_zips && kind.is_zip_family() {
        match list_zip(&mut file) {
            Ok(listing) => (Some(listing), None),
            Err(e) => (None, Some(e.to_string())),
        }
    } else {
        (None, None)
    };
    Ok(FileRecord {
        bytes: histogram.total(),
        sha256: hex(hasher.finalize().as_slice()),
        kind,
        extension: Path::new(&rel)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase),
        entropy: histogram.entropy(),
        printable: histogram.printable_fraction(),
        path: rel,
        zip,
        zip_error,
    })
}

fn scan(root: &Path, list_zips: bool) -> Result<Inventory> {
    if !root.is_dir() {
        bail!("{} is not a folder", root.display());
    }
    let mut files = Vec::new();
    // Sorted so that an inventory of unchanged material is byte-for-byte the same.
    for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
        let entry = entry.context("walking the folder")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        files.push(examine(entry.path(), rel, list_zips)?);
    }
    let mut kinds = BTreeMap::new();
    for f in &files {
        let name = serde_json::to_value(f.kind)?
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        *kinds.entry(name).or_insert(0) += 1;
    }
    Ok(Inventory {
        schema: SCHEMA,
        scanned: root
            .file_name()
            .map_or_else(|| "root".to_string(), |n| n.to_string_lossy().into_owned()),
        file_count: files.len(),
        total_bytes: files.iter().map(|f| f.bytes).sum(),
        kinds,
        files,
    })
}

fn source_root() -> Result<PathBuf> {
    match std::env::var_os(ROOT_VAR) {
        Some(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => bail!(
            "{ROOT_VAR} is not set. Create a source root outside any repository with \
             `dragonlink-inventory init <folder>` and set {ROOT_VAR} to it (docs/SOURCES.md)."
        ),
    }
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::Init { root } => {
            let created = init(&root)?;
            println!(
                "source root ready: {} folder(s) created, {} already present",
                created.len(),
                LAYOUT.len() - created.len()
            );
            println!("set {ROOT_VAR} to this folder so the tools can find it");
            Ok(())
        }
        Cmd::Check => {
            let root = source_root()?;
            let problems = check(&root);
            if problems.is_empty() {
                println!(
                    "{ROOT_VAR} is a usable source root ({} folders)",
                    LAYOUT.len()
                );
                Ok(())
            } else {
                for p in &problems {
                    eprintln!("problem: {p}");
                }
                bail!(
                    "{ROOT_VAR} is not a usable source root ({} problem(s))",
                    problems.len()
                )
            }
        }
        Cmd::Scan { path, out, no_zip } => {
            let inventory = scan(&path, !no_zip)?;
            let out = match out {
                Some(p) => p,
                None => source_root()?
                    .join("manifests")
                    .join(format!("inventory-{}.json", inventory.scanned)),
            };
            if let Some(tree) = out
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .as_deref()
                .and_then(enclosing_git_tree)
            {
                bail!(
                    "refusing to write an inventory into the Git work tree {}; write it to the \
                     source root's manifests folder",
                    tree.display()
                );
            }
            fs::write(&out, serde_json::to_string_pretty(&inventory)? + "\n")
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "inventory: {} file(s), {} bytes, written to {}",
                inventory.file_count,
                inventory.total_bytes,
                out.display()
            );
            for (kind, n) in &inventory.kinds {
                println!("  {kind:<24} {n}");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_the_layout_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("source");
        let created = init(&root).unwrap();
        assert_eq!(created.len(), LAYOUT.len());
        assert!(root.join(MARKER).is_file());
        assert!(root.join("official-mobile").is_dir());
        assert!(root.join("math").is_dir());
        assert!(check(&root).is_empty());

        // A second run changes nothing and keeps what is there.
        fs::write(root.join("captures/keep.txt"), "x").unwrap();
        let marker = fs::read_to_string(root.join(MARKER)).unwrap();
        assert!(init(&root).unwrap().is_empty());
        assert!(root.join("captures/keep.txt").is_file());
        assert_eq!(fs::read_to_string(root.join(MARKER)).unwrap(), marker);
    }

    #[test]
    fn a_source_root_inside_a_git_tree_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        let root = dir.path().join("evidence");
        let err = init(&root).unwrap_err().to_string();
        assert!(err.contains("inside the Git work tree"), "{err}");
        // Nothing was created by the refused attempt.
        assert!(!root.exists());
    }

    #[test]
    fn check_names_what_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            check(&dir.path().join("absent")),
            ["the folder does not exist"]
        );
        let problems = check(dir.path());
        assert!(problems.iter().any(|p| p.contains(MARKER)));
        assert!(problems.iter().any(|p| p.contains("cabinet/")));
    }

    #[test]
    fn scan_hashes_classifies_and_records_only_relative_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("pkg");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("abc.txt"), b"abc").unwrap();
        fs::write(root.join("sub/data.unknown"), [0x9A, 0x00, 0xF3, 0x11]).unwrap();
        fs::write(root.join("sub/empty"), b"").unwrap();
        // A file whose content is ZIP-like but is not a readable archive.
        fs::write(root.join("fake.apk"), b"PK\x03\x04 truncated").unwrap();

        let inv = scan(&root, true).unwrap();
        assert_eq!(inv.scanned, "pkg");
        assert_eq!(inv.file_count, 4);
        assert_eq!(inv.total_bytes, 3 + 4 + 14);
        let paths: Vec<&str> = inv.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            ["abc.txt", "fake.apk", "sub/data.unknown", "sub/empty"]
        );

        let abc = &inv.files[0];
        // Known vector: SHA-256("abc").
        assert_eq!(
            abc.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(abc.kind, FileKind::Text);
        assert_eq!(abc.extension.as_deref(), Some("txt"));

        let fake = &inv.files[1];
        assert_eq!(fake.kind, FileKind::Zip);
        assert_eq!(fake.extension.as_deref(), Some("apk"));
        assert!(fake.zip.is_none() && fake.zip_error.is_some());

        assert_eq!(inv.files[2].kind, FileKind::Unknown);
        assert_eq!(inv.files[3].kind, FileKind::Empty);
        assert_eq!(inv.kinds["unknown"], 1);

        // Nothing in the output names the folder the scan ran in.
        let json = serde_json::to_string(&inv).unwrap();
        let parent = dir.path().to_string_lossy().replace('\\', "\\\\");
        assert!(!json.contains(parent.as_str()));
    }

    #[test]
    fn two_scans_of_the_same_material_are_identical() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["b", "a", "c"] {
            fs::write(dir.path().join(name), name).unwrap();
        }
        let first = serde_json::to_string(&scan(dir.path(), true).unwrap()).unwrap();
        let second = serde_json::to_string(&scan(dir.path(), true).unwrap()).unwrap();
        assert_eq!(first, second);
    }
}
