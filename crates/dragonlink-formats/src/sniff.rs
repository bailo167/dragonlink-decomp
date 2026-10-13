//! File identification by leading bytes.

use serde::Serialize;

/// How many leading bytes [`sniff`] wants. More is ignored; fewer is fine.
pub const SNIFF_BYTES: usize = 2048;

/// What a file's content looks like. Public, documented formats only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    // Archives and packages
    Zip,
    Gzip,
    Xz,
    Zstd,
    Lz4Frame,
    Bzip2,
    SevenZip,
    Rar,
    Tar,
    // Executables and bytecode
    Elf,
    Pe,
    MachO,
    Dex,
    JavaClass,
    // File systems and disk images
    SquashFs,
    CramFs,
    Ext,
    Iso9660,
    Mbr,
    Gpt,
    // Engine containers
    UnityBundle,
    UnitySerialized,
    FmodBank,
    WwiseBank,
    // Media
    Png,
    Jpeg,
    Gif,
    WebP,
    Dds,
    Ktx,
    Astc,
    Wav,
    Ogg,
    Mp3,
    Flac,
    Mp4,
    // Fonts
    TrueType,
    OpenType,
    Woff,
    // Data
    Sqlite,
    AndroidBinaryXml,
    AndroidResourceTable,
    Json,
    Xml,
    Text,
    Empty,
    /// Matches nothing known. Not an error: recorded with its measurements.
    Unknown,
}

impl FileKind {
    /// Whether this is a container worth listing or unpacking.
    pub fn is_zip_family(self) -> bool {
        self == FileKind::Zip
    }
}

/// `(offset, magic, kind)`; the first match wins, so longer and more specific
/// signatures come first.
const SIGNATURES: &[(usize, &[u8], FileKind)] = &[
    (0, b"PK\x03\x04", FileKind::Zip),
    (0, b"PK\x05\x06", FileKind::Zip),
    (0, b"PK\x07\x08", FileKind::Zip),
    (0, &[0x1F, 0x8B, 0x08], FileKind::Gzip),
    (0, &[0xFD, b'7', b'z', b'X', b'Z', 0x00], FileKind::Xz),
    (0, &[0x28, 0xB5, 0x2F, 0xFD], FileKind::Zstd),
    (0, &[0x04, 0x22, 0x4D, 0x18], FileKind::Lz4Frame),
    (0, b"BZh", FileKind::Bzip2),
    (0, b"7z\xBC\xAF\x27\x1C", FileKind::SevenZip),
    (0, b"Rar!\x1A\x07", FileKind::Rar),
    (257, b"ustar", FileKind::Tar),
    (0, b"\x7FELF", FileKind::Elf),
    (0, b"dex\n", FileKind::Dex),
    (0, &[0xCF, 0xFA, 0xED, 0xFE], FileKind::MachO),
    (0, &[0xCE, 0xFA, 0xED, 0xFE], FileKind::MachO),
    (0, &[0xCA, 0xFE, 0xBA, 0xBE], FileKind::JavaClass),
    (0, b"hsqs", FileKind::SquashFs),
    (0, b"sqsh", FileKind::SquashFs),
    (0, &[0x45, 0x3D, 0xCD, 0x28], FileKind::CramFs),
    (1080, &[0x53, 0xEF], FileKind::Ext),
    (512, b"EFI PART", FileKind::Gpt),
    (0, b"UnityFS", FileKind::UnityBundle),
    (0, b"UnityWeb", FileKind::UnityBundle),
    (0, b"UnityRaw", FileKind::UnityBundle),
    (0, b"FSB5", FileKind::FmodBank),
    (0, b"BKHD", FileKind::WwiseBank),
    (0, b"AKPK", FileKind::WwiseBank),
    (0, b"\x89PNG\r\n\x1a\n", FileKind::Png),
    (0, &[0xFF, 0xD8, 0xFF], FileKind::Jpeg),
    (0, b"GIF87a", FileKind::Gif),
    (0, b"GIF89a", FileKind::Gif),
    (0, b"DDS ", FileKind::Dds),
    (0, b"\xABKTX ", FileKind::Ktx),
    (0, &[0x13, 0xAB, 0xA1, 0x5C], FileKind::Astc),
    (0, b"OggS", FileKind::Ogg),
    (0, b"fLaC", FileKind::Flac),
    (0, b"ID3", FileKind::Mp3),
    (4, b"ftyp", FileKind::Mp4),
    (0, b"OTTO", FileKind::OpenType),
    (0, &[0x00, 0x01, 0x00, 0x00, 0x00], FileKind::TrueType),
    (0, b"wOFF", FileKind::Woff),
    (0, b"wOF2", FileKind::Woff),
    (0, b"SQLite format 3\0", FileKind::Sqlite),
    (0, &[0x03, 0x00, 0x08, 0x00], FileKind::AndroidBinaryXml),
    (0, &[0x02, 0x00, 0x0C, 0x00], FileKind::AndroidResourceTable),
];

fn at(head: &[u8], offset: usize, magic: &[u8]) -> bool {
    head.get(offset..offset.saturating_add(magic.len())) == Some(magic)
}

/// Classify a file from its leading bytes (up to [`SNIFF_BYTES`]).
pub fn sniff(head: &[u8]) -> FileKind {
    if head.is_empty() {
        return FileKind::Empty;
    }
    if at(head, 0, b"RIFF") {
        if at(head, 8, b"WAVE") {
            return FileKind::Wav;
        }
        if at(head, 8, b"WEBP") {
            return FileKind::WebP;
        }
    }
    if at(head, 0, b"MZ") {
        return FileKind::Pe;
    }
    for (offset, magic, kind) in SIGNATURES {
        if at(head, *offset, magic) {
            return *kind;
        }
    }
    // ISO 9660 and MBR signatures sit beyond or at the end of the first sector.
    if at(head, 510, &[0x55, 0xAA]) {
        return FileKind::Mbr;
    }
    // A Unity serialized file has no magic; it is recognised only weakly, by a
    // version string such as "2021.3.4f1" in its header, and reported as such.
    if looks_like_unity_serialized(head) {
        return FileKind::UnitySerialized;
    }
    let sample = &head[..head.len().min(SNIFF_BYTES)];
    let body = sample.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(sample);
    if std::str::from_utf8(body).is_ok() || is_mostly_text(body) {
        let trimmed = body.iter().copied().find(|b| !b.is_ascii_whitespace());
        return match trimmed {
            Some(b'{') | Some(b'[') => FileKind::Json,
            Some(b'<') => FileKind::Xml,
            Some(_) => FileKind::Text,
            None => FileKind::Text,
        };
    }
    FileKind::Unknown
}

fn is_mostly_text(bytes: &[u8]) -> bool {
    // A full sniff window can end inside a multi-byte character; allow for that.
    // Shorter inputs are whole files and must decode completely.
    bytes.len() >= SNIFF_BYTES
        && std::str::from_utf8(&bytes[..bytes.len() - 3]).is_ok()
        && !bytes.contains(&0)
}

fn looks_like_unity_serialized(head: &[u8]) -> bool {
    // Header: metadata size, file size, version (big-endian u32 between 5 and 30
    // in known files), data offset; a Unity version string follows within the
    // first 64 bytes: digits, dots and a letter-digit suffix, NUL-terminated.
    let Some(version) = head.get(8..12) else {
        return false;
    };
    let version = u32::from_be_bytes([version[0], version[1], version[2], version[3]]);
    if !(5..=40).contains(&version) {
        return false;
    }
    head.get(12..64).is_some_and(|window| {
        window.windows(6).any(|w| {
            w[0].is_ascii_digit()
                && w[1].is_ascii_digit()
                && w[2].is_ascii_digit()
                && w[3].is_ascii_digit()
                && w[4] == b'.'
                && w[5].is_ascii_digit()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded(prefix: &[u8], len: usize) -> Vec<u8> {
        let mut v = prefix.to_vec();
        v.resize(len.max(prefix.len()), 0);
        v
    }

    #[test]
    fn common_formats_are_recognised_by_content() {
        let cases: &[(&[u8], FileKind)] = &[
            (b"PK\x03\x04\x14\x00\x08\x08", FileKind::Zip),
            (b"\x7FELF\x02\x01\x01\x00", FileKind::Elf),
            (b"MZ\x90\x00\x03\x00", FileKind::Pe),
            (b"dex\n035\0", FileKind::Dex),
            (b"UnityFS\0\0\0\0\x08", FileKind::UnityBundle),
            (b"hsqs\x10\x00\x00\x00", FileKind::SquashFs),
            (b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", FileKind::Png),
            (b"\xFF\xD8\xFF\xE0\x00\x10JFIF", FileKind::Jpeg),
            (b"RIFF\x24\x00\x00\x00WAVEfmt ", FileKind::Wav),
            (b"RIFF\x24\x00\x00\x00WEBPVP8 ", FileKind::WebP),
            (b"OggS\x00\x02\x00\x00", FileKind::Ogg),
            (b"\x00\x00\x00\x18ftypmp42", FileKind::Mp4),
            (b"FSB5\x01\x00\x00\x00", FileKind::FmodBank),
            (b"BKHD\x20\x00\x00\x00", FileKind::WwiseBank),
            (b"OTTO\x00\x0B\x00\x80", FileKind::OpenType),
            (b"\x1F\x8B\x08\x00", FileKind::Gzip),
            (b"\x28\xB5\x2F\xFD\x04", FileKind::Zstd),
            (b"SQLite format 3\0\x10\x00", FileKind::Sqlite),
            (
                b"\x03\x00\x08\x00\x5C\x0A\x00\x00",
                FileKind::AndroidBinaryXml,
            ),
        ];
        for (bytes, kind) in cases {
            assert_eq!(sniff(bytes), *kind, "{bytes:?}");
        }
    }

    #[test]
    fn signatures_at_an_offset_are_found() {
        let mut ext = vec![0u8; 2048];
        ext[1080] = 0x53;
        ext[1081] = 0xEF;
        assert_eq!(sniff(&ext), FileKind::Ext);
        let mut gpt = vec![0u8; 2048];
        gpt[510] = 0x55;
        gpt[511] = 0xAA;
        gpt[512..520].copy_from_slice(b"EFI PART");
        assert_eq!(sniff(&gpt), FileKind::Gpt);
        let mut mbr = vec![0u8; 512];
        mbr[510] = 0x55;
        mbr[511] = 0xAA;
        assert_eq!(sniff(&mbr), FileKind::Mbr);
        let mut tar = vec![0u8; 512];
        tar[257..262].copy_from_slice(b"ustar");
        assert_eq!(sniff(&tar), FileKind::Tar);
    }

    #[test]
    fn text_is_told_from_json_and_xml() {
        assert_eq!(sniff(b"  {\"schema\": 1}"), FileKind::Json);
        assert_eq!(sniff(b"[1, 2, 3]"), FileKind::Json);
        assert_eq!(sniff(b"\xEF\xBB\xBF<?xml version=\"1.0\"?>"), FileKind::Xml);
        assert_eq!(sniff(b"hello world\n"), FileKind::Text);
        assert_eq!(sniff("caf\u{e9} \u{9f99}".as_bytes()), FileKind::Text);
        assert_eq!(sniff(b""), FileKind::Empty);
    }

    #[test]
    fn unrecognised_binary_stays_unknown() {
        assert_eq!(
            sniff(&[0x9A, 0x00, 0xF3, 0x11, 0x00, 0x00, 0x7C, 0x02]),
            FileKind::Unknown
        );
        assert_eq!(
            sniff(&padded(&[0xDE, 0xAD, 0xBE, 0xEF], 64)),
            FileKind::Unknown
        );
        // Truncated magics do not match and do not panic.
        assert_eq!(sniff(b"PK"), FileKind::Text);
        assert_eq!(sniff(&[0x1F, 0x8B]), FileKind::Unknown);
        assert_eq!(sniff(&[0x89, b'P', b'N']), FileKind::Unknown);
    }

    #[test]
    fn a_text_window_cut_inside_a_character_is_still_text() {
        // SNIFF_BYTES bytes of text ending in the first byte of a two-byte character.
        let mut window = vec![b'a'; SNIFF_BYTES - 1];
        window.push(0xC3);
        assert_eq!(sniff(&window), FileKind::Text);
        // The same cut in a short file is not excused.
        assert_eq!(sniff(&[b'a', b'b', 0xC3]), FileKind::Unknown);
    }

    #[test]
    fn a_unity_serialized_header_is_recognised_weakly() {
        let mut head = vec![0u8; 64];
        head[8..12].copy_from_slice(&22u32.to_be_bytes());
        head[20..30].copy_from_slice(b"2021.3.4f1");
        assert_eq!(sniff(&head), FileKind::UnitySerialized);
        // The same bytes without a plausible format version are not.
        head[8..12].copy_from_slice(&900u32.to_be_bytes());
        assert_eq!(sniff(&head), FileKind::Unknown);
    }
}
