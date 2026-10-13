//! Reading the bytes of one entry of a listed archive, within a budget.
//!
//! An entry is read from where the central directory says it is, stored
//! (method 0) or deflated (method 8), and handed to a sink in pieces. Three
//! things are checked, and an entry that fails one is an error, never a
//! best guess:
//!
//! - the local header in front of the data names the same entry and method as
//!   the directory does, so two readers cannot be shown two different files;
//! - the data gives exactly the declared number of bytes;
//! - those bytes have the declared CRC-32.
//!
//! **The budget.** A few kilobytes of deflate can expand to gigabytes. An
//! [`ExtractBudget`] is made per archive from [`ExtractLimits`] and refuses an
//! entry *before* reading it when its declared size is over the per-entry
//! ceiling, its expansion ratio is over the ratio ceiling, or it would take
//! the archive past its total. An entry that lies about its size gains
//! nothing: output stops at the first byte past the declared size, so the
//! sink never receives more than was declared and admitted.
//!
//! Entry names are not used here at all: bytes go to the sink the caller
//! provides. A caller that builds a path from a name must go through
//! [`ZipEntry::safe_relative_path`]. No link of any kind is ever created, so
//! there is no symlink to escape through.

use std::io::{Read, Seek, SeekFrom, Write};

use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};

use super::{ZipEntry, u16_at};
use crate::crc32::Crc32;

/// Compression method 0: the bytes as they are.
pub const METHOD_STORED: u16 = 0;
/// Compression method 8: deflate.
pub const METHOD_DEFLATE: u16 = 8;

/// Fixed part of a local file header.
const LOCAL_HEADER_BYTES: u64 = 30;
/// Largest piece read or produced at a time.
const CHUNK: u64 = 64 * 1024;

/// Ceilings on what reading from one archive may produce.
///
/// The defaults are chosen for mobile game packages, which are the archives
/// this project expects, and are generous on purpose: they exist to stop a
/// bomb, not to second-guess a large but honest package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractLimits {
    /// Most bytes one entry may expand to.
    ///
    /// Default 1 GiB. Without ZIP64 an entry cannot declare 4 GiB or more;
    /// the large members of a game package (data files, native libraries,
    /// bundles) are hundreds of megabytes.
    pub max_entry_bytes: u64,
    /// Most bytes all reads from one archive may produce together, prefixes
    /// and failed reads included.
    ///
    /// Default 8 GiB: room for every member of a package that is itself
    /// under the 4 GiB a non-ZIP64 archive can hold, while bounding what a
    /// directory of many overlapping or repeated entries can cost.
    pub max_archive_bytes: u64,
    /// Most bytes of output per byte of compressed input.
    ///
    /// Default 200. Media that is already compressed does not shrink; text
    /// and JSON shrink five to twenty times; deflate tops out near 1032.
    pub max_ratio: u64,
    /// Entries that expand to no more than this are not held to `max_ratio`.
    ///
    /// Default 16 MiB. A short run of zeros or a silent sound compresses a
    /// thousandfold and harms nobody; the ratio matters only where the result
    /// is large. Many small entries are still bounded by `max_archive_bytes`.
    pub ratio_exempt_bytes: u64,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        ExtractLimits {
            max_entry_bytes: 1 << 30,
            max_archive_bytes: 8 << 30,
            max_ratio: 200,
            ratio_exempt_bytes: 16 << 20,
        }
    }
}

/// The running account of one archive against its [`ExtractLimits`].
/// Make one per archive and pass it to every read from that archive.
#[derive(Debug, Clone)]
pub struct ExtractBudget {
    limits: ExtractLimits,
    produced: u64,
}

impl ExtractBudget {
    pub fn new(limits: ExtractLimits) -> Self {
        ExtractBudget {
            limits,
            produced: 0,
        }
    }

    pub fn limits(&self) -> &ExtractLimits {
        &self.limits
    }

    /// Bytes produced from this archive so far.
    pub fn produced(&self) -> u64 {
        self.produced
    }

    /// Bytes the archive may still produce.
    pub fn remaining(&self) -> u64 {
        self.limits.max_archive_bytes.saturating_sub(self.produced)
    }

    /// Whether `entry` may be read whole, judged by what it declares.
    fn admit_whole(&self, entry: &ZipEntry) -> Result<(), ExtractError> {
        let size = entry.uncompressed_size;
        if size > self.limits.max_entry_bytes {
            return Err(ExtractError::EntryTooLarge {
                name: entry.name.clone(),
                size,
                limit: self.limits.max_entry_bytes,
            });
        }
        if size > self.limits.ratio_exempt_bytes
            && size > entry.compressed_size.saturating_mul(self.limits.max_ratio)
        {
            return Err(ExtractError::RatioTooHigh {
                name: entry.name.clone(),
                compressed: entry.compressed_size,
                uncompressed: size,
                limit: self.limits.max_ratio,
            });
        }
        Ok(())
    }
}

/// Why an entry could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The archive could not be read or the sink could not be written. This
    /// is a failure of the machine, not a fault in the archive.
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("entry {name:?} is encrypted; it is not decrypted")]
    Encrypted { name: String },
    #[error(
        "entry {name:?} uses compression method {method}; only stored (0) and deflate (8) are read"
    )]
    UnsupportedMethod { name: String, method: u16 },
    #[error("entry {name:?}: its header or its data lies outside the file")]
    OutOfBounds { name: String },
    #[error("entry {name:?}: there is no local file header where the directory says it is")]
    BadLocalHeader { name: String },
    #[error(
        "entry {name:?}: its local header and the central directory disagree about the {field}"
    )]
    HeaderMismatch { name: String, field: &'static str },
    #[error("entry {name:?}: the compressed data ends before the deflate stream does")]
    Truncated { name: String },
    #[error("entry {name:?}: the deflate stream is corrupt")]
    CorruptDeflate { name: String },
    #[error(
        "entry {name:?}: the deflate stream ends {unused} bytes before the declared compressed size"
    )]
    TrailingData { name: String, unused: u64 },
    /// When `actual` is larger than `declared`, reading stopped as soon as
    /// the excess was seen: the real size is `actual` or more.
    #[error("entry {name:?} declares {declared} bytes but its data gives {actual}")]
    SizeMismatch {
        name: String,
        declared: u64,
        actual: u64,
    },
    #[error("entry {name:?}: CRC-32 is {actual:08x}, the directory declares {declared:08x}")]
    CrcMismatch {
        name: String,
        declared: u32,
        actual: u32,
    },
    #[error(
        "entry {name:?} declares {size} bytes; refusing to expand one entry past {limit} bytes"
    )]
    EntryTooLarge { name: String, size: u64, limit: u64 },
    #[error(
        "entry {name:?} declares {uncompressed} bytes from {compressed} compressed; \
         refusing an expansion ratio over {limit} to 1"
    )]
    RatioTooHigh {
        name: String,
        compressed: u64,
        uncompressed: u64,
        limit: u64,
    },
    #[error(
        "entry {name:?} needs {size} bytes but only {remaining} are left of the {limit} byte \
         budget for the whole archive"
    )]
    ArchiveBudgetExceeded {
        name: String,
        size: u64,
        remaining: u64,
        limit: u64,
    },
}

/// Where an entry's bytes go, and the account of them.
struct Output<'a, W: Write> {
    sink: &'a mut W,
    budget: &'a mut ExtractBudget,
    crc: Crc32,
    /// Bytes given to the sink so far. Never more than `want`.
    produced: u64,
    /// How many bytes the caller is to receive.
    want: u64,
}

impl<W: Write> Output<'_, W> {
    /// Take bytes the entry produced. Only as many as are still wanted reach
    /// the sink; the number of surplus bytes is returned.
    fn accept(&mut self, bytes: &[u8]) -> std::io::Result<u64> {
        let room = self.want.saturating_sub(self.produced);
        let take = (bytes.len() as u64).min(room);
        let wanted = &bytes[..take as usize];
        self.budget.produced = self.budget.produced.saturating_add(take);
        self.produced = self.produced.saturating_add(take);
        self.crc.update(wanted);
        self.sink.write_all(wanted)?;
        Ok(bytes.len() as u64 - take)
    }
}

/// Read `entry`, or with `prefix` only its first bytes, into `sink`.
/// Returns the number of bytes the sink received.
fn pump<R: Read + Seek, W: Write>(
    reader: &mut R,
    entry: &ZipEntry,
    budget: &mut ExtractBudget,
    sink: &mut W,
    prefix: Option<u64>,
) -> Result<u64, ExtractError> {
    let name = || entry.name.clone();
    if entry.encrypted {
        return Err(ExtractError::Encrypted { name: name() });
    }
    if entry.method != METHOD_STORED && entry.method != METHOD_DEFLATE {
        return Err(ExtractError::UnsupportedMethod {
            name: name(),
            method: entry.method,
        });
    }
    let declared = entry.uncompressed_size;
    let want = match prefix {
        Some(most) => declared.min(most),
        None => {
            budget.admit_whole(entry)?;
            declared
        }
    };
    if want > budget.remaining() {
        return Err(ExtractError::ArchiveBudgetExceeded {
            name: name(),
            size: want,
            remaining: budget.remaining(),
            limit: budget.limits.max_archive_bytes,
        });
    }
    if entry.method == METHOD_STORED && entry.compressed_size != declared {
        return Err(ExtractError::SizeMismatch {
            name: name(),
            declared,
            actual: entry.compressed_size,
        });
    }

    // The local header: is it there, and does it describe the same entry?
    let file_len = reader.seek(SeekFrom::End(0))?;
    let out_of_bounds = || ExtractError::OutOfBounds { name: name() };
    let bad_header = || ExtractError::BadLocalHeader { name: name() };
    let header_end = entry
        .local_header_offset
        .checked_add(LOCAL_HEADER_BYTES)
        .filter(|end| *end <= file_len)
        .ok_or_else(out_of_bounds)?;
    reader.seek(SeekFrom::Start(entry.local_header_offset))?;
    let mut header = [0u8; LOCAL_HEADER_BYTES as usize];
    reader.read_exact(&mut header)?;
    if header[..4] != *b"PK\x03\x04" {
        return Err(bad_header());
    }
    let flags = u16_at(&header, 6).ok_or_else(bad_header)?;
    let method = u16_at(&header, 8).ok_or_else(bad_header)?;
    let name_len = u16_at(&header, 26).ok_or_else(bad_header)?;
    let extra_len = u16_at(&header, 28).ok_or_else(bad_header)?;
    let mismatch = |field| ExtractError::HeaderMismatch {
        name: name(),
        field,
    };
    if method != entry.method {
        return Err(mismatch("compression method"));
    }
    if flags & 1 != 0 {
        return Err(mismatch("encryption flag"));
    }
    let data_start = header_end
        .checked_add(u64::from(name_len))
        .and_then(|at| at.checked_add(u64::from(extra_len)))
        .ok_or_else(out_of_bounds)?;
    if data_start
        .checked_add(entry.compressed_size)
        .is_none_or(|end| end > file_len)
    {
        return Err(out_of_bounds());
    }
    let mut local_name = vec![0u8; usize::from(name_len)];
    reader.read_exact(&mut local_name)?;
    if String::from_utf8_lossy(&local_name) != entry.name {
        return Err(mismatch("name"));
    }
    reader.seek(SeekFrom::Start(data_start))?;

    let mut out = Output {
        sink,
        budget,
        crc: Crc32::new(),
        produced: 0,
        want,
    };
    if entry.method == METHOD_STORED {
        let mut buffer = vec![0u8; want.min(CHUNK) as usize];
        while out.produced < want {
            let n = (want - out.produced).min(CHUNK) as usize;
            reader.read_exact(&mut buffer[..n])?;
            out.accept(&buffer[..n])?;
        }
    } else {
        let mut state = InflateState::new_boxed(DataFormat::Raw);
        let mut input = vec![0u8; entry.compressed_size.min(CHUNK) as usize];
        // One byte more than is wanted, so that an entry holding more than it
        // declares is seen to.
        let mut output = vec![0u8; want.saturating_add(1).min(CHUNK) as usize];
        // Compressed bytes not yet read from the file, and the part of
        // `input` the decompressor has not consumed.
        let mut unread = entry.compressed_size;
        let (mut start, mut end) = (0usize, 0usize);
        loop {
            if prefix.is_some() && out.produced == want {
                break;
            }
            if start == end && unread > 0 {
                let n = unread.min(CHUNK) as usize;
                reader.read_exact(&mut input[..n])?;
                unread -= n as u64;
                (start, end) = (0, n);
            }
            let step = inflate(&mut state, &input[start..end], &mut output, MZFlush::None);
            start = start.saturating_add(step.bytes_consumed).min(end);
            let produced = output.get(..step.bytes_written).unwrap_or(&output);
            let surplus = out.accept(produced)?;
            if surplus > 0 && prefix.is_none() {
                return Err(ExtractError::SizeMismatch {
                    name: name(),
                    declared,
                    actual: declared.saturating_add(surplus),
                });
            }
            match step.status {
                Ok(MZStatus::StreamEnd) => break,
                // Neither input taken nor output given: it will never finish.
                Ok(_) if step.bytes_consumed == 0 && step.bytes_written == 0 => {
                    return Err(ExtractError::CorruptDeflate { name: name() });
                }
                Ok(_) => {}
                Err(MZError::Buf) if start == end && unread == 0 => {
                    return Err(ExtractError::Truncated { name: name() });
                }
                Err(_) => return Err(ExtractError::CorruptDeflate { name: name() }),
            }
        }
        let unused = unread.saturating_add((end - start) as u64);
        if prefix.is_none() && unused > 0 {
            return Err(ExtractError::TrailingData {
                name: name(),
                unused,
            });
        }
    }

    if prefix.is_none() {
        if out.produced != declared {
            return Err(ExtractError::SizeMismatch {
                name: name(),
                declared,
                actual: out.produced,
            });
        }
        let actual = out.crc.finish();
        if actual != entry.crc32 {
            return Err(ExtractError::CrcMismatch {
                name: name(),
                declared: entry.crc32,
                actual,
            });
        }
    }
    Ok(out.produced)
}

/// Read `entry` of the archive in `reader` into `sink`, verifying its size
/// and CRC-32. Returns the number of bytes written.
///
/// `entry` comes from [`list_zip`](super::list_zip) on the same `reader`;
/// `budget` is the one for that archive.
///
/// On an error the sink may already hold part of the entry (never more than
/// the entry declares): what it received is unverified and must be discarded.
pub fn extract_entry<R: Read + Seek, W: Write>(
    reader: &mut R,
    entry: &ZipEntry,
    budget: &mut ExtractBudget,
    sink: &mut W,
) -> Result<u64, ExtractError> {
    pump(reader, entry, budget, sink, None)
}

/// The verified bytes of `entry`, in memory. For entries known to be small;
/// anything else goes through [`extract_entry`] to a sink.
pub fn read_entry<R: Read + Seek>(
    reader: &mut R,
    entry: &ZipEntry,
    budget: &mut ExtractBudget,
) -> Result<Vec<u8>, ExtractError> {
    let mut bytes = Vec::new();
    pump(reader, entry, budget, &mut bytes, None)?;
    Ok(bytes)
}

/// The first `max_bytes` bytes of `entry`, or all of it when it is shorter:
/// enough to see what kind of file it is without expanding it.
///
/// **Unverified.** A CRC covers a whole entry, so a prefix cannot be checked
/// against it, and the entry's size is not confirmed. What is produced is
/// charged to the budget; the per-entry and ratio ceilings do not apply,
/// because no more than `max_bytes` can come out.
pub fn read_entry_prefix<R: Read + Seek>(
    reader: &mut R,
    entry: &ZipEntry,
    budget: &mut ExtractBudget,
    max_bytes: usize,
) -> Result<Vec<u8>, ExtractError> {
    let mut bytes = Vec::new();
    pump(reader, entry, budget, &mut bytes, Some(max_bytes as u64))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::list_zip;
    use crate::zip::synthetic::{Entry, archive, deflate_stored_blocks};
    use std::io::Cursor;

    /// Bytes that do not compress: a small xorshift generator.
    fn noise(len: usize) -> Vec<u8> {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 32) as u8
            })
            .collect()
    }

    /// Bytes that compress well.
    fn text(len: usize) -> Vec<u8> {
        b"reel stop symbol credit "
            .iter()
            .copied()
            .cycle()
            .take(len)
            .collect()
    }

    fn budget() -> ExtractBudget {
        ExtractBudget::new(ExtractLimits::default())
    }

    /// Read the entry at `index` of `zip` whole, with default limits.
    fn read(zip: &[u8], index: usize) -> Result<Vec<u8>, ExtractError> {
        let mut cursor = Cursor::new(zip);
        let listing = list_zip(&mut cursor).unwrap();
        read_entry(&mut cursor, &listing.entries[index], &mut budget())
    }

    #[test]
    fn stored_and_deflated_entries_come_back_exactly() {
        // Sizes on both sides of the chunk boundary, and none at all.
        for len in [0usize, 1, 100, 65_535, 65_536, 65_537, 300_000] {
            for data in [text(len), noise(len)] {
                let zip = archive(
                    &[
                        Entry::stored("a/stored.bin", &data),
                        Entry::deflated("a/deflated.bin", &data),
                    ],
                    b"",
                );
                assert_eq!(read(&zip, 0).unwrap(), data, "stored, {len} bytes");
                assert_eq!(read(&zip, 1).unwrap(), data, "deflated, {len} bytes");
            }
        }
    }

    #[test]
    fn a_deflate_stream_written_by_hand_is_read() {
        // Not produced by the library that reads it: stored blocks only.
        let data = noise(150_000);
        let entry = Entry {
            method: METHOD_DEFLATE,
            payload: deflate_stored_blocks(&data),
            ..Entry::stored("hand.bin", &data)
        };
        assert_eq!(read(&archive(&[entry], b""), 0).unwrap(), data);
        let empty = Entry {
            method: METHOD_DEFLATE,
            payload: deflate_stored_blocks(b""),
            ..Entry::stored("empty.bin", b"")
        };
        assert_eq!(read(&archive(&[empty], b""), 0).unwrap(), b"");
    }

    #[test]
    fn extraction_to_a_sink_reports_and_accounts_for_what_it_wrote() {
        let data = text(200_000);
        let zip = archive(&[Entry::deflated("t.txt", &data)], b"");
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = budget();
        let mut sink = Vec::new();
        let written =
            extract_entry(&mut cursor, &listing.entries[0], &mut budget, &mut sink).unwrap();
        assert_eq!(written, 200_000);
        assert_eq!(sink, data);
        assert_eq!(budget.produced(), 200_000);
        assert_eq!(
            budget.remaining(),
            budget.limits().max_archive_bytes - 200_000
        );
    }

    #[test]
    fn a_wrong_checksum_is_an_error() {
        let data = text(5000);
        for mut entry in [Entry::stored("s", &data), Entry::deflated("d", &data)] {
            entry.crc32 ^= 1;
            let declared = entry.crc32;
            match read(&archive(&[entry], b""), 0) {
                Err(ExtractError::CrcMismatch {
                    declared: d,
                    actual,
                    ..
                }) => {
                    assert_eq!(d, declared);
                    assert_eq!(actual, declared ^ 1);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn an_entry_that_lies_about_its_size_gains_nothing() {
        let data = text(1 << 20);

        // Declares 100 bytes; the stream holds a megabyte. The sink is given
        // no more than was declared.
        let mut small = Entry::deflated("small", &data);
        small.uncompressed_size = 100;
        let zip = archive(&[small], b"");
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = budget();
        let mut sink = Vec::new();
        let result = extract_entry(&mut cursor, &listing.entries[0], &mut budget, &mut sink);
        assert!(
            matches!(
                result,
                Err(ExtractError::SizeMismatch { declared: 100, actual, .. }) if actual > 100
            ),
            "{result:?}"
        );
        assert!(sink.len() <= 100);
        assert!(budget.produced() <= 100);

        // Declares more than the stream holds.
        let mut large = Entry::deflated("large", &data[..1000]);
        large.uncompressed_size = 1001;
        assert!(matches!(
            read(&archive(&[large], b""), 0),
            Err(ExtractError::SizeMismatch {
                declared: 1001,
                actual: 1000,
                ..
            })
        ));

        // A stored entry whose two sizes differ.
        let mut stored = Entry::stored("stored", &data[..1000]);
        stored.uncompressed_size = 5000;
        assert!(matches!(
            read(&archive(&[stored], b""), 0),
            Err(ExtractError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn a_bomb_is_refused_by_what_it_declares_before_anything_is_read() {
        // Two gigabytes declared: over the per-entry ceiling.
        let mut huge = Entry::deflated("huge", b"x");
        huge.uncompressed_size = 2 << 30;
        // Half a gigabyte from a few bytes: over the ratio ceiling.
        let mut dense = Entry::deflated("dense", b"x");
        dense.uncompressed_size = 500 << 20;
        // A megabyte of zeros is a thousandfold, and small enough to be let through.
        let zeros = vec![0u8; 1 << 20];
        let zip = archive(&[huge, dense, Entry::deflated("zeros", &zeros)], b"");
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = budget();
        let mut sink = Vec::new();
        assert!(matches!(
            extract_entry(&mut cursor, &listing.entries[0], &mut budget, &mut sink),
            Err(ExtractError::EntryTooLarge { .. })
        ));
        assert!(matches!(
            extract_entry(&mut cursor, &listing.entries[1], &mut budget, &mut sink),
            Err(ExtractError::RatioTooHigh { limit: 200, .. })
        ));
        assert!(sink.is_empty());
        assert_eq!(budget.produced(), 0);
        assert_eq!(
            read_entry(&mut cursor, &listing.entries[2], &mut budget).unwrap(),
            zeros
        );
    }

    #[test]
    fn the_ratio_ceiling_applies_above_the_exempt_size() {
        let limits = ExtractLimits {
            max_ratio: 10,
            ratio_exempt_bytes: 1000,
            ..ExtractLimits::default()
        };
        let zip = archive(
            &[
                Entry::deflated("small", &vec![0u8; 1000]),
                Entry::deflated("large", &vec![0u8; 100_000]),
                Entry::deflated("honest", &noise(100_000)),
            ],
            b"",
        );
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = ExtractBudget::new(limits);
        assert!(read_entry(&mut cursor, &listing.entries[0], &mut budget).is_ok());
        assert!(matches!(
            read_entry(&mut cursor, &listing.entries[1], &mut budget),
            Err(ExtractError::RatioTooHigh { limit: 10, .. })
        ));
        assert!(read_entry(&mut cursor, &listing.entries[2], &mut budget).is_ok());
    }

    #[test]
    fn the_archive_budget_is_shared_by_every_read() {
        let limits = ExtractLimits {
            max_archive_bytes: 250,
            ..ExtractLimits::default()
        };
        let data = noise(100);
        let zip = archive(
            &[
                Entry::stored("a", &data),
                Entry::deflated("b", &data),
                Entry::stored("c", &data),
            ],
            b"",
        );
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = ExtractBudget::new(limits);
        read_entry(&mut cursor, &listing.entries[0], &mut budget).unwrap();
        read_entry(&mut cursor, &listing.entries[1], &mut budget).unwrap();
        assert_eq!(budget.remaining(), 50);
        assert!(matches!(
            read_entry(&mut cursor, &listing.entries[2], &mut budget),
            Err(ExtractError::ArchiveBudgetExceeded {
                size: 100,
                remaining: 50,
                limit: 250,
                ..
            })
        ));
        // Reading the same entry again is charged again, and a prefix is charged too.
        assert_eq!(
            read_entry_prefix(&mut cursor, &listing.entries[0], &mut budget, 30)
                .unwrap()
                .len(),
            30
        );
        assert_eq!(budget.remaining(), 20);
        assert!(read_entry_prefix(&mut cursor, &listing.entries[0], &mut budget, 30).is_err());
    }

    #[test]
    fn a_prefix_is_the_start_of_the_entry_and_nothing_more() {
        let data = text(500_000);
        let mut wrong_crc = Entry::deflated("w", &data);
        wrong_crc.crc32 ^= 0xFFFF;
        // Declared far over the per-entry ceiling: a prefix can still be looked at.
        let mut huge = Entry::deflated("h", &data);
        huge.uncompressed_size = 3 << 30;
        let zip = archive(
            &[
                Entry::deflated("d", &data),
                Entry::stored("s", &data),
                Entry::stored("short", b"abc"),
                wrong_crc,
                huge,
            ],
            b"",
        );
        let mut cursor = Cursor::new(&zip);
        let listing = list_zip(&mut cursor).unwrap();
        let mut budget = budget();
        for index in [0, 1, 3, 4] {
            let head =
                read_entry_prefix(&mut cursor, &listing.entries[index], &mut budget, 2048).unwrap();
            assert_eq!(head, data[..2048], "entry {index}");
        }
        assert_eq!(
            read_entry_prefix(&mut cursor, &listing.entries[2], &mut budget, 2048).unwrap(),
            b"abc"
        );
        assert!(
            read_entry_prefix(&mut cursor, &listing.entries[0], &mut budget, 0)
                .unwrap()
                .is_empty()
        );
        assert_eq!(budget.produced(), 4 * 2048 + 3);
    }

    #[test]
    fn encrypted_entries_and_other_methods_are_refused() {
        let mut encrypted = Entry::stored("secret", b"data");
        encrypted.flags = 1;
        let mut bzip2 = Entry::stored("b", b"data");
        bzip2.method = 12;
        let zip = archive(&[encrypted, bzip2], b"");
        assert!(matches!(read(&zip, 0), Err(ExtractError::Encrypted { .. })));
        assert!(matches!(
            read(&zip, 1),
            Err(ExtractError::UnsupportedMethod { method: 12, .. })
        ));
    }

    #[test]
    fn a_local_header_that_disagrees_with_the_directory_is_an_error() {
        // One entry, so its local header is at offset 0.
        let zip = archive(&[Entry::stored("name.bin", b"payload")], b"");
        assert_eq!(read(&zip, 0).unwrap(), b"payload");

        let mut bad = zip.clone();
        bad[0] = b'X';
        assert!(matches!(
            read(&bad, 0),
            Err(ExtractError::BadLocalHeader { .. })
        ));

        let mut bad = zip.clone();
        bad[8] = 8; // the method
        assert!(matches!(
            read(&bad, 0),
            Err(ExtractError::HeaderMismatch {
                field: "compression method",
                ..
            })
        ));

        let mut bad = zip.clone();
        bad[6] |= 1; // the encryption flag
        assert!(matches!(
            read(&bad, 0),
            Err(ExtractError::HeaderMismatch {
                field: "encryption flag",
                ..
            })
        ));

        let mut bad = zip.clone();
        bad[30] ^= 0x20; // first byte of the name
        assert!(matches!(
            read(&bad, 0),
            Err(ExtractError::HeaderMismatch { field: "name", .. })
        ));
    }

    #[test]
    fn an_entry_outside_the_file_is_an_error() {
        let zip = archive(&[Entry::stored("a", b"payload")], b"");
        let mut cursor = Cursor::new(&zip);
        let mut entry = list_zip(&mut cursor).unwrap().entries.remove(0);
        entry.local_header_offset = zip.len() as u64;
        assert!(matches!(
            read_entry(&mut cursor, &entry, &mut budget()),
            Err(ExtractError::OutOfBounds { .. })
        ));
        entry.local_header_offset = u64::MAX;
        assert!(matches!(
            read_entry(&mut cursor, &entry, &mut budget()),
            Err(ExtractError::OutOfBounds { .. })
        ));
        // Data that would run past the end of the file.
        let mut entry = list_zip(&mut cursor).unwrap().entries.remove(0);
        entry.compressed_size = zip.len() as u64;
        entry.uncompressed_size = zip.len() as u64;
        assert!(matches!(
            read_entry(&mut cursor, &entry, &mut budget()),
            Err(ExtractError::OutOfBounds { .. })
        ));
    }

    #[test]
    fn a_damaged_deflate_stream_is_an_error() {
        let data = text(10_000);
        let honest = Entry::deflated("d", &data);

        let mut cut = honest.clone();
        cut.payload.truncate(cut.payload.len() / 2);
        assert!(matches!(
            read(&archive(&[cut], b""), 0),
            Err(ExtractError::Truncated { .. })
        ));

        let mut padded = honest.clone();
        padded.payload.extend_from_slice(b"extra");
        assert!(matches!(
            read(&archive(&[padded], b""), 0),
            Err(ExtractError::TrailingData { unused: 5, .. })
        ));

        let mut empty = honest.clone();
        empty.payload.clear();
        assert!(matches!(
            read(&archive(&[empty], b""), 0),
            Err(ExtractError::Truncated { .. })
        ));

        // Block type 3 does not exist.
        let mut garbage = honest;
        garbage.payload = vec![0x07; 64];
        assert!(matches!(
            read(&archive(&[garbage], b""), 0),
            Err(ExtractError::CorruptDeflate { .. })
        ));
    }

    #[test]
    fn no_single_damaged_byte_panics_or_yields_wrong_bytes() {
        let originals: [(&str, Vec<u8>); 3] = [
            ("one.txt", text(700)),
            ("two.bin", noise(300)),
            ("three.txt", text(900)),
        ];
        let zip = archive(
            &[
                Entry::deflated(originals[0].0, &originals[0].1),
                Entry::stored(originals[1].0, &originals[1].1),
                Entry::deflated(originals[2].0, &originals[2].1),
            ],
            b"",
        );
        let mut verified = 0usize;
        for at in 0..zip.len() {
            for flip in [0x01u8, 0x80, 0xFF] {
                let mut damaged = zip.clone();
                damaged[at] ^= flip;
                let mut cursor = Cursor::new(&damaged);
                let Ok(listing) = list_zip(&mut cursor) else {
                    continue;
                };
                let mut budget = budget();
                for entry in &listing.entries {
                    let Ok(bytes) = read_entry(&mut cursor, entry, &mut budget) else {
                        continue;
                    };
                    // Whatever is returned as verified is one of the originals,
                    // under its own name.
                    let original = originals.iter().find(|(name, _)| *name == entry.name);
                    assert_eq!(
                        original.map(|(_, data)| data),
                        Some(&bytes),
                        "byte {at} xor {flip:#04x}, entry {:?}",
                        entry.name
                    );
                    verified += 1;
                }
            }
        }
        // Damage outside an entry leaves that entry readable.
        assert!(verified > 0);
    }
}
