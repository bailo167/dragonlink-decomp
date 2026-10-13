//! Identify files by content, measure entropy, and list ZIP-family archives.
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
//!
//! Nothing here knows anything about a proprietary format. Signatures are of
//! public, documented formats; a proprietary container is identified only
//! after it has been researched (`docs/reverse-engineering/`).

pub mod entropy;
pub mod sniff;
pub mod zip;

pub use entropy::{ByteHistogram, shannon_entropy};
pub use sniff::{FileKind, SNIFF_BYTES, sniff};
pub use zip::{ZipEntry, ZipError, ZipListing, list_zip};
