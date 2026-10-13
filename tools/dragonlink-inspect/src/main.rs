//! dragonlink-inspect — look at one file that is not understood yet.
//!
//! ```text
//! dragonlink-inspect <FILE> [--block-kib N] [--json]
//! ```
//!
//! Records what the project's research rules ask for before any conclusion is
//! drawn about an unfamiliar file (`CLAUDE.md`, "Research before assertion"):
//! size, SHA-256, leading bytes, what the content looks like, entropy overall
//! and block by block, how much of it is printable text, how many strings it
//! holds, and for a ZIP-family package a summary of its entries.
//!
//! It reads the file and never modifies it. It prints **measurements, not
//! content**: no strings and no bytes beyond the first sixteen, so its output
//! is sanitized metadata and can be quoted in research notes. A file it does
//! not recognise is reported as unknown; it does not guess.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use dragonlink_formats::{ByteHistogram, FileKind, SNIFF_BYTES, list_zip, shannon_entropy, sniff};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Shortest run of printable characters counted as a string.
const MIN_STRING: usize = 6;

#[derive(Parser, Debug)]
#[command(about = "Measure one unknown file: type, hash, entropy profile, strings statistics")]
struct Args {
    file: PathBuf,
    /// Size of the blocks of the entropy profile, in KiB.
    #[arg(long, default_value_t = 64)]
    block_kib: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct ZipSummary {
    entries: usize,
    total_uncompressed: u64,
    /// Entry count by lowercase extension, most common first.
    extensions: Vec<(String, usize)>,
    unsafe_names: usize,
    encrypted: usize,
}

#[derive(Debug, Serialize)]
struct Inspection {
    /// File name only; no folder.
    name: String,
    bytes: u64,
    sha256: String,
    /// First bytes in hexadecimal (at most sixteen).
    magic: String,
    kind: FileKind,
    /// Bits per byte over the whole file.
    entropy: f64,
    printable: f64,
    /// Entropy of each block, in order.
    block_bytes: usize,
    block_entropy: Vec<f64>,
    /// Runs of at least six printable ASCII characters.
    strings: usize,
    longest_string: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip: Option<ZipSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_error: Option<String>,
    /// What these measurements suggest, stated as observations.
    observations: Vec<String>,
}

fn inspect(path: &Path, block_bytes: usize) -> Result<Inspection> {
    let block_bytes = block_bytes.max(1);
    let mut file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut histogram = ByteHistogram::new();
    let mut head = Vec::with_capacity(SNIFF_BYTES);
    let mut block = Vec::with_capacity(block_bytes);
    let mut block_entropy = Vec::new();
    let (mut strings, mut longest, mut run) = (0usize, 0usize, 0usize);
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let chunk = &buffer[..n];
        hasher.update(chunk);
        histogram.update(chunk);
        if head.len() < SNIFF_BYTES {
            head.extend_from_slice(&chunk[..(SNIFF_BYTES - head.len()).min(n)]);
        }
        for byte in chunk {
            if (0x20..0x7F).contains(byte) {
                run += 1;
            } else {
                if run >= MIN_STRING {
                    strings += 1;
                    longest = longest.max(run);
                }
                run = 0;
            }
            block.push(*byte);
            if block.len() == block_bytes {
                block_entropy.push(shannon_entropy(&block));
                block.clear();
            }
        }
    }
    if run >= MIN_STRING {
        strings += 1;
        longest = longest.max(run);
    }
    if !block.is_empty() {
        block_entropy.push(shannon_entropy(&block));
    }

    let kind = sniff(&head);
    let (zip, zip_error) = if kind.is_zip_family() {
        match list_zip(&mut file) {
            Ok(listing) => {
                let mut by_ext = std::collections::BTreeMap::new();
                for e in listing.entries.iter().filter(|e| !e.is_directory()) {
                    let ext = Path::new(&e.name)
                        .extension()
                        .and_then(|x| x.to_str())
                        .map_or_else(|| "(none)".to_string(), str::to_ascii_lowercase);
                    *by_ext.entry(ext).or_insert(0usize) += 1;
                }
                let mut extensions: Vec<_> = by_ext.into_iter().collect();
                extensions.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                (
                    Some(ZipSummary {
                        entries: listing.entries.len(),
                        total_uncompressed: listing.total_uncompressed(),
                        extensions,
                        unsafe_names: listing
                            .entries
                            .iter()
                            .filter(|e| e.name_is_unsafe())
                            .count(),
                        encrypted: listing.entries.iter().filter(|e| e.encrypted).count(),
                    }),
                    None,
                )
            }
            Err(e) => (None, Some(e.to_string())),
        }
    } else {
        (None, None)
    };

    let entropy = histogram.entropy();
    let mut observations = Vec::new();
    if kind == FileKind::Unknown {
        observations.push(
            "The content matches no known signature. It is unknown, not misidentified.".into(),
        );
    }
    if histogram.total() >= 4096 && entropy > 7.9 {
        observations.push(
            "Entropy is close to 8 bits per byte throughout: compressed or encrypted data. The two cannot be told apart by entropy."
                .into(),
        );
    }
    if let (Some(min), Some(max)) = (
        block_entropy.iter().copied().reduce(f64::min),
        block_entropy.iter().copied().reduce(f64::max),
    ) && block_entropy.len() > 1
        && max - min > 2.0
    {
        observations.push(format!(
            "Block entropy ranges from {min:.2} to {max:.2}: the file has regions of different character (for example a header or table before packed data)."
        ));
    }
    Ok(Inspection {
        name: path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        bytes: histogram.total(),
        sha256: hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        magic: head
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" "),
        kind,
        entropy,
        printable: histogram.printable_fraction(),
        block_bytes,
        block_entropy,
        strings,
        longest_string: longest,
        zip,
        zip_error,
        observations,
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    let report = inspect(&args.file, args.block_kib.saturating_mul(1024))?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("file      {}", report.name);
    println!("bytes     {}", report.bytes);
    println!("sha256    {}", report.sha256);
    println!("magic     {}", report.magic);
    println!("kind      {:?}", report.kind);
    println!(
        "entropy   {:.4} bits/byte   printable {:.1}%",
        report.entropy,
        report.printable * 100.0
    );
    println!(
        "strings   {} of {MIN_STRING}+ characters, longest {}",
        report.strings, report.longest_string
    );
    if report.block_entropy.len() > 1 {
        // One character per block: '.' low, '-' text-like, '+' dense, '#' near-random.
        let profile: String = report
            .block_entropy
            .iter()
            .take(120)
            .map(|e| match *e {
                e if e < 2.0 => '.',
                e if e < 5.5 => '-',
                e if e < 7.5 => '+',
                _ => '#',
            })
            .collect();
        println!(
            "profile   {profile}   ({} blocks of {} bytes; . <2  - <5.5  + <7.5  # higher)",
            report.block_entropy.len(),
            report.block_bytes
        );
    }
    if let Some(zip) = &report.zip {
        println!(
            "archive   {} entries, {} bytes uncompressed, {} unsafe names, {} encrypted",
            zip.entries, zip.total_uncompressed, zip.unsafe_names, zip.encrypted
        );
        for (ext, n) in zip.extensions.iter().take(12) {
            println!("            .{ext:<14} {n}");
        }
    }
    if let Some(e) = &report.zip_error {
        println!("archive   could not be listed: {e}");
    }
    for o in &report.observations {
        println!("note      {o}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.bin");
        fs::write(&path, bytes).unwrap();
        (dir, path)
    }

    #[test]
    fn a_text_file_is_measured() {
        let (_dir, path) = write(b"hello world\nshort\nanother line of text\n");
        let r = inspect(&path, 16).unwrap();
        assert_eq!(r.name, "sample.bin");
        assert_eq!(r.bytes, 39);
        assert_eq!(r.kind, FileKind::Text);
        assert_eq!(r.printable, 1.0);
        // "hello world" and "another line of text" are strings; "short" is too short.
        assert_eq!(r.strings, 2);
        assert_eq!(r.longest_string, 20);
        assert_eq!(r.block_entropy.len(), 3);
        assert_eq!(r.magic.split(' ').count(), 16);
        assert!(r.zip.is_none() && r.observations.is_empty());
    }

    #[test]
    fn unknown_dense_data_is_reported_as_unknown_and_dense() {
        // A deterministic byte stream that uses every value evenly.
        let data: Vec<u8> = (0..65_536u32)
            .map(|i| (i.wrapping_mul(197) >> 3) as u8 ^ (i as u8))
            .collect();
        let mut data = data;
        data[0] = 0x9A;
        data[1] = 0x00;
        let (_dir, path) = write(&data);
        let r = inspect(&path, 4096).unwrap();
        assert_eq!(r.kind, FileKind::Unknown);
        assert!(
            r.observations
                .iter()
                .any(|o| o.contains("unknown, not misidentified"))
        );
        assert_eq!(r.block_entropy.len(), 16);
        assert_eq!(r.sha256.len(), 64);
    }

    #[test]
    fn regions_of_different_character_are_noticed() {
        let mut data = vec![0u8; 4096];
        data.extend((0..4096u32).map(|i| (i * 7 + i / 3) as u8));
        let (_dir, path) = write(&data);
        let r = inspect(&path, 4096).unwrap();
        assert_eq!(r.block_entropy.len(), 2);
        assert_eq!(r.block_entropy[0], 0.0);
        assert!(r.block_entropy[1] > 6.0);
        assert!(
            r.observations
                .iter()
                .any(|o| o.contains("regions of different character"))
        );
    }

    #[test]
    fn a_broken_archive_is_reported_not_guessed_at() {
        let (_dir, path) = write(b"PK\x03\x04 this is not a complete archive");
        let r = inspect(&path, 1024).unwrap();
        assert_eq!(r.kind, FileKind::Zip);
        assert!(r.zip.is_none());
        assert!(r.zip_error.is_some());
        assert!(inspect(Path::new("no-such-file"), 1024).is_err());
    }
}
