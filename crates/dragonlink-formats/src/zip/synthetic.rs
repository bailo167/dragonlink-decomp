//! Synthetic archives for the tests, written byte by byte.
//!
//! An [`Entry`] starts out honest (its headers describe its payload) and has
//! public fields, so a test makes an archive lie by changing one of them.

use crate::crc32::crc32;

/// One entry as it will be written: what its headers claim, and the bytes
/// actually stored for it.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub name: String,
    pub flags: u16,
    pub method: u16,
    pub crc32: u32,
    pub uncompressed_size: u32,
    /// The bytes that follow the local header. The compressed size written to
    /// the headers is always the length of this.
    pub payload: Vec<u8>,
}

impl Entry {
    /// `data` as it is.
    pub fn stored(name: &str, data: &[u8]) -> Entry {
        Entry {
            name: name.to_string(),
            flags: 0,
            method: 0,
            crc32: crc32(data),
            uncompressed_size: data.len() as u32,
            payload: data.to_vec(),
        }
    }

    /// `data` compressed with deflate.
    pub fn deflated(name: &str, data: &[u8]) -> Entry {
        Entry {
            method: 8,
            payload: miniz_oxide::deflate::compress_to_vec(data, 6),
            ..Entry::stored(name, data)
        }
    }
}

/// A deflate stream of stored blocks only, written by hand: each block is a
/// header byte, the length, its complement and the bytes themselves.
pub(crate) fn deflate_stored_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut blocks = data.chunks(0xFFFF).peekable();
    if blocks.peek().is_none() {
        // No data: one empty final block.
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none())); // BFINAL, BTYPE 00
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        out.extend_from_slice(block);
    }
    out
}

/// The archive holding `entries`, with `comment` as the archive comment.
pub(crate) fn archive(entries: &[Entry], comment: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for entry in entries {
        let offset = out.len() as u32;
        let name = entry.name.as_bytes();
        let compressed = entry.payload.len() as u32;

        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&entry.flags.to_le_bytes());
        out.extend_from_slice(&entry.method.to_le_bytes());
        out.extend_from_slice(&[0; 4]); // time, date
        out.extend_from_slice(&entry.crc32.to_le_bytes());
        out.extend_from_slice(&compressed.to_le_bytes());
        out.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.payload);

        directory.extend_from_slice(b"PK\x01\x02");
        directory.extend_from_slice(&[20, 0, 20, 0]); // version made by, version needed
        directory.extend_from_slice(&entry.flags.to_le_bytes());
        directory.extend_from_slice(&entry.method.to_le_bytes());
        directory.extend_from_slice(&[0; 4]); // time, date
        directory.extend_from_slice(&entry.crc32.to_le_bytes());
        directory.extend_from_slice(&compressed.to_le_bytes());
        directory.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0u8; 12]); // extra, comment, disk, attributes
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
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
