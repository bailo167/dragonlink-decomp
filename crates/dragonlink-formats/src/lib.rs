//! Identify files by content, measure entropy, and list and read ZIP-family archives.
//!
//! Tools for looking at source material that is not understood yet. Three
//! rules, taken from experience on ASAMU-decomp and from reading `rea`
//! (`docs/PRIOR_ART.md`):
//!
//! - **Content before names.** A file is classified by its bytes; the
//!   extension is recorded but never trusted.
//! - **Unknown is an answer.** A file that matches nothing is reported as
//!   unknown with its measurements, not forced into a category.
//! - **Input is hostile.** Parsers use checked arithmetic and explicit
//!   limits, and return errors on malformed input instead of panicking.
//!   Decompression runs within a budget, so a small archive cannot become a
//!   large amount of memory or disk.
//!
//! Nothing here knows anything about a proprietary format. Signatures are of
//! public, documented formats; a proprietary container is identified only
//! after it has been researched (`docs/reverse-engineering/`).

pub mod crc32;
pub mod entropy;
pub mod sniff;
pub mod zip;

pub use crc32::{Crc32, crc32};
pub use entropy::{ByteHistogram, shannon_entropy};
pub use sniff::{FileKind, SNIFF_BYTES, sniff};
pub use zip::{
    ExtractBudget, ExtractError, ExtractLimits, ZipEntry, ZipError, ZipListing, extract_entry,
    list_zip, read_entry, read_entry_prefix,
};
