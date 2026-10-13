//! Listing ZIP-family archives (ZIP, APK, XAPK, OBB, JAR) from their central directory.
//!
//! Listing only: names, sizes, methods and checksums are read from the
//! central directory; no entry is decompressed, so there is nothing to bomb.
//! The reader does not see bytes outside the directory (an APK signing block,
//! data appended after the archive) and says so in [`ZipListing::limitations`].
//!
//! Input is untrusted: every offset and length is checked against the file
//! size and against explicit limits before it is used.

use std::io::{Read, Seek, SeekFrom};

use serde::Serialize;

/// Most entries a listing will return.
pub const MAX_ENTRIES: usize = 200_000;
/// Largest central directory that will be read.
const MAX_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;
/// The end-of-central-directory record is within this many bytes of the end.
const EOCD_SEARCH: u64 = 22 + 65_535;

/// Why an archive could not be listed.
#[derive(Debug, thiserror::Error)]
pub enum ZipError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("no end-of-central-directory record: not a ZIP archive, or truncated")]
    NoEndRecord,
    #[error("ZIP64 archive: not supported by this reader yet")]
    Zip64,
    #[error("multi-disk archive: not supported")]
    MultiDisk,
    #[error(
        "the central directory ({size} bytes at {offset}) does not fit in the {file} byte file"
    )]
    DirectoryOutOfBounds { offset: u64, size: u64, file: u64 },
    #[error("the central directory is {0} bytes; refusing to read more than {MAX_DIRECTORY_BYTES}")]
    DirectoryTooLarge(u64),
    #[error("{0} entries; refusing to list more than {MAX_ENTRIES}")]
    TooManyEntries(usize),
    #[error("central directory entry {index} is malformed")]
    BadEntry { index: usize },
}

/// One entry of an archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZipEntry {
    /// Name as stored. Not sanitised: do not use as a path without checking it.
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    /// 0 stored, 8 deflate; other values as in the ZIP specification.
    pub method: u16,
    pub crc32: u32,
    pub local_header_offset: u64,
    pub encrypted: bool,
}

impl ZipEntry {
    pub fn is_directory(&self) -> bool {
        self.name.ends_with('/')
    }

    /// Whether the name could escape an extraction folder.
    pub fn name_is_unsafe(&self) -> bool {
        let n = &self.name;
        n.starts_with('/')
            || n.starts_with('\\')
            || n.contains('\0')
            || n.chars().nth(1) == Some(':')
            || n.split(['/', '\\']).any(|part| part == "..")
    }
}

/// The contents of an archive as its central directory describes them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZipListing {
    pub entries: Vec<ZipEntry>,
    /// What this listing could not see or did not check.
    pub limitations: Vec<String>,
}

impl ZipListing {
    pub fn total_uncompressed(&self) -> u64 {
        self.entries
            .iter()
            .fold(0u64, |acc, e| acc.saturating_add(e.uncompressed_size))
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    bytes
        .get(at..at.checked_add(2)?)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// List the archive in `reader`.
pub fn list_zip<R: Read + Seek>(reader: &mut R) -> Result<ZipListing, ZipError> {
    let file_size = reader.seek(SeekFrom::End(0))?;
    let tail_len = file_size.min(EOCD_SEARCH);
    reader.seek(SeekFrom::Start(file_size - tail_len))?;
    let mut tail = vec![0u8; tail_len as usize];
    reader.read_exact(&mut tail)?;

    // The last end record whose comment length reaches exactly the end of the file.
    let eocd = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|at| {
            tail.get(*at..at + 4) == Some(b"PK\x05\x06")
                && u16_at(&tail, at + 20).is_some_and(|c| at + 22 + c as usize == tail.len())
        })
        .ok_or(ZipError::NoEndRecord)?;
    let field16 = |at: usize| u16_at(&tail, eocd + at).ok_or(ZipError::NoEndRecord);
    let field32 = |at: usize| u32_at(&tail, eocd + at).ok_or(ZipError::NoEndRecord);
    if field16(4)? != 0 || field16(6)? != 0 {
        return Err(ZipError::MultiDisk);
    }
    let count = field16(10)?;
    let size = field32(12)?;
    let offset = field32(16)?;
    if count == u16::MAX || size == u32::MAX || offset == u32::MAX {
        return Err(ZipError::Zip64);
    }
    let (count, size, offset) = (count as usize, u64::from(size), u64::from(offset));
    if count > MAX_ENTRIES {
        return Err(ZipError::TooManyEntries(count));
    }
    if size > MAX_DIRECTORY_BYTES {
        return Err(ZipError::DirectoryTooLarge(size));
    }
    if offset.checked_add(size).is_none_or(|end| end > file_size) {
        return Err(ZipError::DirectoryOutOfBounds {
            offset,
            size,
            file: file_size,
        });
    }

    reader.seek(SeekFrom::Start(offset))?;
    let mut directory = vec![0u8; size as usize];
    reader.read_exact(&mut directory)?;

    let mut entries = Vec::with_capacity(count);
    let mut at = 0usize;
    for index in 0..count {
        let bad = || ZipError::BadEntry { index };
        if directory.get(at..at + 4) != Some(b"PK\x01\x02") {
            return Err(bad());
        }
        let flags = u16_at(&directory, at + 8).ok_or_else(bad)?;
        let method = u16_at(&directory, at + 10).ok_or_else(bad)?;
        let crc32 = u32_at(&directory, at + 16).ok_or_else(bad)?;
        let compressed = u32_at(&directory, at + 20).ok_or_else(bad)?;
        let uncompressed = u32_at(&directory, at + 24).ok_or_else(bad)?;
        let name_len = u16_at(&directory, at + 28).ok_or_else(bad)? as usize;
        let extra_len = u16_at(&directory, at + 30).ok_or_else(bad)? as usize;
        let comment_len = u16_at(&directory, at + 32).ok_or_else(bad)? as usize;
        let local = u32_at(&directory, at + 42).ok_or_else(bad)?;
        if compressed == u32::MAX || uncompressed == u32::MAX || local == u32::MAX {
            return Err(ZipError::Zip64);
        }
        let name_start = at + 46;
        let name = directory
            .get(name_start..name_start + name_len)
            .ok_or_else(bad)?;
        entries.push(ZipEntry {
            name: String::from_utf8_lossy(name).into_owned(),
            compressed_size: u64::from(compressed),
            uncompressed_size: u64::from(uncompressed),
            method,
            crc32,
            local_header_offset: u64::from(local),
            encrypted: flags & 1 != 0,
        });
        at = name_start + name_len + extra_len + comment_len;
        if at > directory.len() {
            return Err(bad());
        }
    }

    let mut limitations = vec![
        "Listed from the central directory only; no entry was decompressed or checked against its CRC.".to_string(),
        "Bytes outside the entries and the directory (an APK signing block, appended data) are not examined.".to_string(),
    ];
    if entries.iter().any(ZipEntry::name_is_unsafe) {
        limitations.push(
            "Some entry names are absolute or contain '..' and must not be used as paths."
                .to_string(),
        );
    }
    Ok(ZipListing {
        entries,
        limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A ZIP archive of stored entries, built by hand. Checksums are left zero:
    /// the reader lists and does not verify.
    fn build(entries: &[(&str, &[u8])], comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, data) in entries {
            let offset = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // version, flags, method, time, date
            out.extend_from_slice(&0u32.to_le_bytes()); // crc
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            directory.extend_from_slice(b"PK\x01\x02");
            directory.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // versions, flags, method, time, date
            directory.extend_from_slice(&0u32.to_le_bytes()); // crc
            directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&[0u8; 12]); // extra, comment, disk, attributes
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }
        let dir_offset = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&dir_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    #[test]
    fn entries_are_listed_with_sizes_and_offsets() {
        let zip = build(
            &[
                ("AndroidManifest.xml", b"\x03\x00\x08\x00"),
                ("lib/arm64-v8a/", b""),
                ("assets/data.bin", &[1, 2, 3, 4, 5, 6, 7]),
            ],
            b"",
        );
        let listing = list_zip(&mut Cursor::new(&zip)).unwrap();
        assert_eq!(listing.entries.len(), 3);
        assert_eq!(listing.entries[0].name, "AndroidManifest.xml");
        assert_eq!(listing.entries[0].uncompressed_size, 4);
        assert_eq!(listing.entries[0].local_header_offset, 0);
        assert!(listing.entries[1].is_directory());
        assert_eq!(listing.entries[2].uncompressed_size, 7);
        assert_eq!(listing.entries[2].method, 0);
        assert!(!listing.entries[2].encrypted);
        assert_eq!(listing.total_uncompressed(), 11);
        assert_eq!(listing.limitations.len(), 2);
    }

    #[test]
    fn an_archive_comment_and_an_empty_archive_are_handled() {
        let with_comment = build(&[("a.txt", b"hi")], b"built by test");
        assert_eq!(
            list_zip(&mut Cursor::new(&with_comment))
                .unwrap()
                .entries
                .len(),
            1
        );
        let empty = build(&[], b"");
        assert!(
            list_zip(&mut Cursor::new(&empty))
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn unsafe_names_are_flagged_not_trusted() {
        let zip = build(
            &[
                ("../../etc/passwd", b"x"),
                ("C:/Windows/x", b"y"),
                ("ok/file", b"z"),
            ],
            b"",
        );
        let listing = list_zip(&mut Cursor::new(&zip)).unwrap();
        assert!(listing.entries[0].name_is_unsafe());
        assert!(listing.entries[1].name_is_unsafe());
        assert!(!listing.entries[2].name_is_unsafe());
        assert_eq!(listing.limitations.len(), 3);
    }

    #[test]
    fn non_archives_and_truncations_are_errors_not_panics() {
        assert!(matches!(
            list_zip(&mut Cursor::new(b"not a zip at all".to_vec())),
            Err(ZipError::NoEndRecord)
        ));
        assert!(matches!(
            list_zip(&mut Cursor::new(Vec::new())),
            Err(ZipError::NoEndRecord)
        ));
        let zip = build(&[("a.txt", b"hello"), ("b.txt", b"world")], b"");
        // Every truncation fails cleanly.
        for len in 0..zip.len() {
            assert!(
                list_zip(&mut Cursor::new(&zip[..len])).is_err(),
                "length {len}"
            );
        }
    }

    #[test]
    fn corrupted_directories_are_rejected() {
        let zip = build(&[("a.txt", b"hello"), ("b.txt", b"world")], b"");
        let eocd = zip.len() - 22;

        // Directory offset pointing past the end of the file.
        let mut bad = zip.clone();
        bad[eocd + 16..eocd + 20].copy_from_slice(&0x00FF_FFFFu32.to_le_bytes());
        assert!(matches!(
            list_zip(&mut Cursor::new(&bad)),
            Err(ZipError::DirectoryOutOfBounds { .. })
        ));

        // More entries claimed than the directory holds.
        let mut bad = zip.clone();
        bad[eocd + 10..eocd + 12].copy_from_slice(&9u16.to_le_bytes());
        assert!(matches!(
            list_zip(&mut Cursor::new(&bad)),
            Err(ZipError::BadEntry { index: 2 })
        ));

        // ZIP64 markers are reported, not misread.
        let mut bad = zip.clone();
        bad[eocd + 16..eocd + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            list_zip(&mut Cursor::new(&bad)),
            Err(ZipError::Zip64)
        ));

        // Flipping any single byte never panics.
        for i in 0..zip.len() {
            let mut fuzzed = zip.clone();
            fuzzed[i] ^= 0xFF;
            let _ = list_zip(&mut Cursor::new(&fuzzed));
        }
    }
}
