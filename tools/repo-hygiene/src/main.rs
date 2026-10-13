//! repo-hygiene — refuse proprietary source material, raw reverse-engineering
//! dumps, secrets and private paths in this public repository.
//!
//! Adapted from the tool of the same name in ASAMU-decomp (same owner, same
//! licence); the checks are widened for this project's source material:
//! Android packages, cabinet/disk images, engine asset containers, audio,
//! fonts and credentials.
//!
//! Run before every push (CI and the Windows gate run it too):
//!
//! ```text
//! cargo run -p repo-hygiene             # all tracked + untracked, non-ignored files
//! cargo run -p repo-hygiene -- --staged # only what is staged for commit
//! ```
//!
//! A line can opt out of the *path and decompiler* text checks with the
//! marker `hygiene:allow` (for documentation that must quote a pattern).
//! Extension, magic, size, directory and secret checks cannot be opted out of.
//!
//! Raster images are accepted in one place only, `docs/images/`: curated
//! screenshots and short clips of **our runtime in placeholder mode** (see
//! `docs/LEGAL.md`). Anywhere else an image file is refused, so an extracted
//! texture cannot slip in as a `.png`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Check the repository for proprietary source material, RE dumps and secrets")]
struct Args {
    /// Only check files staged for commit.
    #[arg(long)]
    staged: bool,
    /// Treat warnings (decompiler fingerprints) as failures.
    #[arg(long)]
    strict: bool,
    /// Repository root (defaults to the git toplevel of the current directory).
    #[arg(long)]
    root: Option<PathBuf>,
}

/// Extensions that are source material, extracted assets or RE databases.
/// Never committed, whatever the content.
const FORBIDDEN_EXT: &[&str] = &[
    // Android / mobile packages
    "apk",
    "apks",
    "xapk",
    "apkm",
    "aab",
    "obb",
    "dex",
    "odex",
    "vdex",
    "oat",
    "ipa",
    // Executables and native libraries
    "exe",
    "dll",
    "so",
    "dylib",
    "elf",
    "ko",
    "sys",
    "msi",
    "jar",
    "class",
    // Cabinet / disk / firmware images
    "img",
    "iso",
    "rom",
    "dd",
    "raw",
    "vhd",
    "vhdx",
    "vmdk",
    "qcow2",
    "e01",
    "dmg",
    "cfi",
    "cfast",
    "squashfs",
    "sqfs",
    "cramfs",
    "ext2",
    "ext3",
    "ext4",
    "fw",
    "hex",
    "upd",
    // Archives
    "zip",
    "7z",
    "rar",
    "tar",
    "tgz",
    "gz",
    "xz",
    "zst",
    "lz4",
    "bz2",
    "cab",
    // Engine asset containers
    "unity3d",
    "assets",
    "ress",
    "resource",
    "bundle",
    "assetbundle",
    "pak",
    "pck",
    "bnk",
    "wem",
    "fsb",
    "fev",
    "acb",
    "awb",
    "sbk",
    "atlas",
    "skel",
    "spine",
    "swf",
    "gfx",
    // Audio
    "wav",
    "ogg",
    "oga",
    "opus",
    "mp3",
    "m4a",
    "aac",
    "flac",
    "wma",
    // Fonts
    "ttf",
    "otf",
    "ttc",
    "woff",
    "woff2",
    "fnt",
    // RE databases and dumps
    "gpr",
    "rep",
    "gzf",
    "idb",
    "i64",
    "bndb",
    "til",
    "nam",
    "id0",
    "id1",
    "id2",
    "smali",
    // Key material
    "pem",
    "key",
    "p12",
    "pfx",
    "jks",
    "keystore",
    "token",
];

/// Directory names of the private source root (see `docs/SOURCES.md`). A file
/// under a directory with one of these names is refused wherever it is: the
/// source root does not belong inside the work tree at all.
const SOURCE_ROOT_DIRS: &[&str] = &[
    "official-mobile",
    "cabinet",
    "captures",
    "extracted",
    "decompiler",
    "temporary",
    "source-root",
    "local-package",
];

/// Largest file we accept. Our own sources, docs and sanitized metadata are small.
const MAX_BYTES: u64 = 1024 * 1024;

/// The only folder that may hold raster images: showcase screenshots and
/// short clips of our runtime.
const SHOWCASE_DIR: &str = "docs/images/";

/// Largest showcase file (a short README clip needs more than [`MAX_BYTES`]).
const SHOWCASE_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Showcase formats: (extension, the bytes such a file starts with).
const SHOWCASE_FORMATS: &[(&str, &[u8])] = &[
    ("png", b"\x89PNG\r\n\x1a\n"),
    ("jpg", &[0xFF, 0xD8, 0xFF]),
    ("jpeg", &[0xFF, 0xD8, 0xFF]),
    ("gif", b"GIF8"),
    ("webp", b"RIFF"),
];

/// Image and texture formats that are refused outside [`SHOWCASE_DIR`]
/// (the first five) or everywhere (texture containers, video and model
/// formats).
const IMAGE_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tga", "dds", "ktx", "ktx2", "astc", "pvr", "pkm",
    "tif", "tiff", "psd", "exr", "hdr", "avif", "mov", "mp4", "m4v", "webm", "mkv", "avi", "bik",
    "bk2", "usm", "gltf", "glb", "fbx", "obj", "dae", "blend",
];

/// What the image rules say about `rel` (`len` bytes, starting with `head`):
/// `None` = not an image, or an acceptable showcase file.
fn image_finding(rel: &str, len: u64, head: &[u8]) -> Option<String> {
    let ext = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)?;
    if !IMAGE_EXT.contains(&ext.as_str()) {
        return None;
    }
    let Some((_, magic)) = SHOWCASE_FORMATS.iter().find(|(e, _)| *e == ext) else {
        return Some(format!(
            "image/texture/video/model format .{ext} is never committed"
        ));
    };
    // Directly inside the folder: no sub-folders to hide a texture tree in.
    let in_showcase = rel
        .strip_prefix(SHOWCASE_DIR)
        .is_some_and(|name| !name.is_empty() && !name.contains('/'));
    if !in_showcase {
        return Some(format!(
            "image .{ext} outside {SHOWCASE_DIR} (only showcase screenshots of our runtime are \
             committed, and only there)"
        ));
    }
    if !head.starts_with(magic) {
        return Some(format!("content is not a .{ext} image"));
    }
    if len > SHOWCASE_MAX_BYTES {
        return Some(format!(
            "showcase file is {len} bytes (limit {SHOWCASE_MAX_BYTES})"
        ));
    }
    None
}

/// The size limit that applies to `rel`.
fn size_limit(rel: &str) -> u64 {
    if rel.starts_with(SHOWCASE_DIR) {
        SHOWCASE_MAX_BYTES
    } else {
        MAX_BYTES
    }
}

/// The source-root directory name `rel` lies under, if any.
fn source_root_dir(rel: &str) -> Option<&'static str> {
    let mut parts: Vec<&str> = rel.split('/').collect();
    parts.pop(); // the file name itself is not a directory
    SOURCE_ROOT_DIRS
        .iter()
        .copied()
        .find(|d| parts.iter().any(|p| p.eq_ignore_ascii_case(d)))
}

/// (offset, magic bytes, description). Content that starts like one of these
/// is refused whatever the file is called.
const FORBIDDEN_MAGIC: &[(usize, &[u8], &str)] = &[
    (
        0,
        b"PK\x03\x04",
        "ZIP archive (APK/OBB/JAR/XAPK are ZIP files)",
    ),
    (0, b"PK\x05\x06", "ZIP archive (empty)"),
    (0, b"\x7FELF", "ELF binary"),
    (0, b"MZ", "PE/DOS executable"),
    (0, b"dex\n", "Android DEX bytecode"),
    (
        0,
        &[0xCA, 0xFE, 0xBA, 0xBE],
        "Java class file or Mach-O universal binary",
    ),
    (0, &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O 64-bit binary"),
    (0, &[0xCE, 0xFA, 0xED, 0xFE], "Mach-O 32-bit binary"),
    (0, b"UnityFS", "Unity asset bundle"),
    (0, b"UnityWeb", "Unity asset bundle (web)"),
    (0, b"UnityRaw", "Unity asset bundle (raw)"),
    (0, &[0x1F, 0x8B, 0x08], "gzip stream"),
    (0, b"7z\xBC\xAF\x27\x1C", "7-Zip archive"),
    (0, b"Rar!", "RAR archive"),
    (0, &[0xFD, b'7', b'z', b'X', b'Z', 0x00], "xz stream"),
    (0, &[0x28, 0xB5, 0x2F, 0xFD], "Zstandard stream"),
    (0, b"hsqs", "SquashFS image"),
    (0, b"sqsh", "SquashFS image (big-endian)"),
    (0, b"OggS", "Ogg audio/video"),
    (0, b"fLaC", "FLAC audio"),
    (0, b"ID3", "MP3 audio (ID3 tag)"),
    (0, b"FSB5", "FMOD sound bank"),
    (0, b"BKHD", "Wwise sound bank"),
    (0, b"AKPK", "Wwise package"),
    (0, b"FWS", "Flash SWF"),
    (0, b"CWS", "Flash SWF (compressed)"),
    (0, b"DDS ", "DDS texture"),
    (0, b"\xABKTX ", "KTX texture"),
    (0, &[0x13, 0xAB, 0xA1, 0x5C], "ASTC texture"),
    (0, b"OTTO", "OpenType font"),
    (0, &[0x00, 0x01, 0x00, 0x00, 0x00], "TrueType font"),
    (0, b"wOFF", "WOFF font"),
    (0, b"wOF2", "WOFF2 font"),
    (0, b"SQLite format 3", "SQLite database"),
    (4, b"ftyp", "MP4/MOV container"),
    (8, b"WAVE", "RIFF WAVE audio"),
    (8, b"AVI ", "RIFF AVI video"),
];

/// How many leading bytes [`check_file`] reads for the magic checks.
const HEAD_BYTES: usize = 16;

/// The description of the forbidden content `head` starts like, if any.
fn magic_finding(head: &[u8]) -> Option<&'static str> {
    FORBIDDEN_MAGIC.iter().find_map(|(off, magic, what)| {
        head.get(*off..off + magic.len())
            .is_some_and(|s| s == *magic)
            .then_some(*what)
    })
}

/// Text fragments that indicate private paths. Literal fragments, not regexes.
const PRIVATE_PATH_FRAGMENTS: &[&str] = &["/Users/", ":\\Users\\", "/home/"];

/// Text fragments that indicate pasted decompiler output (warning by default).
const DECOMPILER_FRAGMENTS: &[&str] = &[
    "FUN_0",
    "FUN_1",
    "DAT_0",
    "DAT_1",
    "LAB_0",
    "undefined8 ",
    "undefined4 ",
    "Decompiled with",
    "WARNING: Subroutine does not return",
    "/* loaded from: ",
    "/* renamed from: ",
    "JADX WARN",
    "invoke-virtual {",
    ".end method",
];

const ALLOW_MARKER: &str = "hygiene:allow";

/// A token shape: a literal prefix followed by at least `min_tail` characters
/// from `tail` (see [`secret_finding`]).
struct SecretShape {
    prefix: &'static str,
    min_tail: usize,
    tail: fn(char) -> bool,
    what: &'static str,
}

fn alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}
fn alnum_underscore(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
fn alnum_dash_underscore(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}
fn upper_digit(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

/// Credential shapes. Matching needs the prefix *and* a long enough tail, so
/// prose that only names a prefix does not match.
const SECRET_SHAPES: &[SecretShape] = &[
    SecretShape {
        prefix: "ghp_",
        min_tail: 30,
        tail: alnum,
        what: "GitHub personal access token",
    },
    SecretShape {
        prefix: "gho_",
        min_tail: 30,
        tail: alnum,
        what: "GitHub OAuth token",
    },
    SecretShape {
        prefix: "ghu_",
        min_tail: 30,
        tail: alnum,
        what: "GitHub user token",
    },
    SecretShape {
        prefix: "ghs_",
        min_tail: 30,
        tail: alnum,
        what: "GitHub server token",
    },
    SecretShape {
        prefix: "ghr_",
        min_tail: 30,
        tail: alnum,
        what: "GitHub refresh token",
    },
    SecretShape {
        prefix: "github_pat_",
        min_tail: 40,
        tail: alnum_underscore,
        what: "GitHub fine-grained token",
    },
    SecretShape {
        prefix: "AKIA",
        min_tail: 16,
        tail: upper_digit,
        what: "AWS access key id",
    },
    SecretShape {
        prefix: "AIza",
        min_tail: 35,
        tail: alnum_dash_underscore,
        what: "Google API key",
    },
    SecretShape {
        prefix: "xoxb-",
        min_tail: 20,
        tail: alnum_dash_underscore,
        what: "Slack bot token",
    },
    SecretShape {
        prefix: "xoxp-",
        min_tail: 20,
        tail: alnum_dash_underscore,
        what: "Slack user token",
    },
    SecretShape {
        prefix: "sk-ant-",
        min_tail: 20,
        tail: alnum_dash_underscore,
        what: "Anthropic API key",
    },
];

/// What kind of credential `line` appears to contain, if any.
fn secret_finding(line: &str) -> Option<&'static str> {
    if line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----") {
        return Some("private key block");
    }
    SECRET_SHAPES.iter().find_map(|shape| {
        line.match_indices(shape.prefix)
            .any(|(at, _)| {
                line[at + shape.prefix.len()..]
                    .chars()
                    .take_while(|c| (shape.tail)(*c))
                    .count()
                    >= shape.min_tail
            })
            .then_some(shape.what)
    })
}

#[derive(Debug)]
struct Finding {
    path: String,
    fatal: bool,
    msg: String,
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn check_file(root: &Path, rel: &str, findings: &mut Vec<Finding>) -> Result<()> {
    let path = root.join(rel);
    let Ok(meta) = fs::metadata(&path) else {
        return Ok(()); // deleted in working tree
    };
    if !meta.is_file() {
        return Ok(());
    }
    let mut push = |fatal: bool, msg: String| {
        findings.push(Finding {
            path: rel.to_string(),
            fatal,
            msg,
        })
    };

    let ext = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    if let Some(ext) = &ext
        && FORBIDDEN_EXT.contains(&ext.as_str())
    {
        push(true, format!("forbidden extension .{ext}"));
    }
    if let Some(dir) = source_root_dir(rel) {
        push(
            true,
            format!("lies under a private source-root directory name ({dir}/)"),
        );
    }
    let limit = size_limit(rel);
    if meta.len() > limit {
        push(
            true,
            format!("file is {} bytes (limit {limit})", meta.len()),
        );
    }

    let mut head = [0u8; HEAD_BYTES];
    let n = fs::File::open(&path)?.read(&mut head)?;
    if let Some(msg) = image_finding(rel, meta.len(), &head[..n]) {
        push(true, msg);
    }
    if let Some(what) = magic_finding(&head[..n]) {
        push(true, format!("content looks like a {what}"));
    }

    // Text checks only for files that decode as UTF-8 and are small enough.
    if meta.len() <= MAX_BYTES
        && let Ok(text) = fs::read_to_string(&path)
    {
        for (lineno, line) in text.lines().enumerate() {
            if let Some(what) = secret_finding(line) {
                push(
                    true,
                    format!("line {}: looks like a credential ({what})", lineno + 1),
                );
            }
            if line.contains(ALLOW_MARKER) {
                continue;
            }
            for frag in PRIVATE_PATH_FRAGMENTS {
                if line.contains(frag) {
                    push(
                        true,
                        format!(
                            "line {}: private absolute path fragment {frag:?}",
                            lineno + 1
                        ),
                    );
                }
            }
            for frag in DECOMPILER_FRAGMENTS {
                if line.contains(frag) {
                    push(
                        false,
                        format!("line {}: decompiler fingerprint {frag:?}", lineno + 1),
                    );
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = match args.root {
        Some(r) => r,
        None => PathBuf::from(git(Path::new("."), &["rev-parse", "--show-toplevel"])?.trim()),
    };
    let list = if args.staged {
        git(
            &root,
            &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        )?
    } else {
        git(&root, &["ls-files", "-co", "--exclude-standard"])?
    };

    let mut findings = Vec::new();
    let mut count = 0usize;
    for rel in list.lines().filter(|l| !l.is_empty()) {
        // This tool necessarily contains the patterns it searches for.
        if rel == "tools/repo-hygiene/src/main.rs" {
            continue;
        }
        count += 1;
        check_file(&root, rel, &mut findings)?;
    }

    let fatal = findings.iter().filter(|f| f.fatal).count();
    let warn = findings.len() - fatal;
    for f in &findings {
        let tag = if f.fatal { "FAIL" } else { "WARN" };
        eprintln!("{tag} {}: {}", f.path, f.msg);
    }
    println!("repo-hygiene: checked {count} files — {fatal} failure(s), {warn} warning(s)");
    if fatal > 0 || (args.strict && warn > 0) {
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];
    const GIF: &[u8] = b"GIF89a";

    #[test]
    fn showcase_images_are_accepted_only_in_their_folder() {
        assert_eq!(image_finding("docs/images/shell.png", 500_000, PNG), None);
        assert_eq!(image_finding("docs/images/shell.JPG", 500_000, JPG), None);
        assert_eq!(image_finding("docs/images/spin.gif", 6_000_000, GIF), None);
        // Anywhere else an image is refused, however small.
        for rel in [
            "shell.png",
            "docs/shell.png",
            "docs/images/symbols/dragon.png",
            "crates/dragonlink-assets/tests/fixtures/t.png",
            "docs/imagesx/a.png",
        ] {
            assert!(image_finding(rel, 10, PNG).is_some(), "{rel}");
        }
    }

    #[test]
    fn showcase_files_must_be_what_their_name_says() {
        assert!(image_finding("docs/images/a.png", 10, JPG).is_some());
        assert!(image_finding("docs/images/a.gif", 10, b"MZ\x90\x00").is_some());
        assert!(image_finding("docs/images/a.png", 10, b"").is_some());
        assert!(image_finding("docs/images/a.gif", SHOWCASE_MAX_BYTES + 1, GIF).is_some());
        assert_eq!(
            image_finding("docs/images/a.gif", SHOWCASE_MAX_BYTES, GIF),
            None
        );
    }

    #[test]
    fn texture_and_video_formats_are_refused_everywhere() {
        for rel in [
            "docs/images/symbol.dds",
            "docs/images/symbol.tga",
            "docs/images/clip.mov",
            "docs/images/clip.mp4",
            "a/b.ktx2",
            "a/b.astc",
            "a/b.bmp",
        ] {
            assert!(image_finding(rel, 10, PNG).is_some(), "{rel}");
        }
    }

    #[test]
    fn other_files_are_not_images() {
        assert_eq!(image_finding("progress/progress.svg", 10, b"<svg"), None);
        assert_eq!(image_finding("README.md", 10, b"# dragonlink"), None);
        assert_eq!(image_finding("docs/images/README.md", 10, b"# x"), None);
    }

    #[test]
    fn only_the_showcase_folder_gets_the_larger_limit() {
        assert_eq!(size_limit("docs/images/spin.gif"), SHOWCASE_MAX_BYTES);
        assert_eq!(size_limit("docs/PARITY.md"), MAX_BYTES);
        assert_eq!(size_limit("src/docs/images/a.gif"), MAX_BYTES);
    }

    #[test]
    fn package_and_image_extensions_are_forbidden() {
        for ext in [
            "apk", "obb", "xapk", "so", "dex", "img", "iso", "zip", "ogg", "ttf", "unity3d", "pem",
        ] {
            assert!(FORBIDDEN_EXT.contains(&ext), "{ext}");
        }
    }

    #[test]
    fn content_is_recognised_whatever_the_name() {
        // An APK renamed to .txt is still a ZIP archive.
        assert!(magic_finding(b"PK\x03\x04\x14\x00").is_some());
        assert!(magic_finding(b"\x7FELF\x02\x01").is_some());
        assert!(magic_finding(b"dex\n035\0").is_some());
        assert!(magic_finding(b"UnityFS\0\0\0\0\x06").is_some());
        assert!(magic_finding(b"hsqs\x00\x00").is_some());
        assert!(magic_finding(b"OggS\x00\x02").is_some());
        assert!(magic_finding(b"RIFF\x24\x00\x00\x00WAVEfmt ").is_some());
        assert!(magic_finding(b"\x00\x00\x00\x18ftypmp42").is_some());
        // Our own text files are not.
        assert_eq!(magic_finding(b"# README"), None);
        assert_eq!(magic_finding(b"[workspace]"), None);
        assert_eq!(magic_finding(b"<svg xmlns"), None);
        assert_eq!(magic_finding(b""), None);
        // A RIFF file that is not audio/video (WebP) is left to the image rules.
        assert_eq!(magic_finding(b"RIFF\x24\x00\x00\x00WEBPVP8 "), None);
    }

    #[test]
    fn source_root_directories_are_refused_anywhere() {
        assert_eq!(
            source_root_dir("official-mobile/app/base.txt"),
            Some("official-mobile")
        );
        assert_eq!(source_root_dir("x/Cabinet/notes.md"), Some("cabinet"));
        assert_eq!(
            source_root_dir("research/extracted/list.json"),
            Some("extracted")
        );
        // A file merely *named* like one is fine; so are our own folders.
        assert_eq!(source_root_dir("docs/cabinet"), None);
        assert_eq!(
            source_root_dir("docs/reverse-engineering/PLATFORMS.md"),
            None
        );
        assert_eq!(source_root_dir("crates/dragonlink-core/src/lib.rs"), None);
    }

    #[test]
    fn credentials_are_recognised_by_shape() {
        let token = format!("export GH_TOKEN=\"ghp_{}\"", "a1B2".repeat(9));
        assert_eq!(secret_finding(&token), Some("GitHub personal access token"));
        let aws = format!("key = AKIA{}", "ABCD1234".repeat(2));
        assert_eq!(secret_finding(&aws), Some("AWS access key id"));
        assert_eq!(
            secret_finding("-----BEGIN RSA PRIVATE KEY-----"),
            Some("private key block")
        );
        // Prose that only names a prefix is not a credential.
        assert_eq!(
            secret_finding("tokens start with ghp_ or github_pat_"),
            None
        );
        assert_eq!(secret_finding("AKIA is the AWS key prefix"), None);
        assert_eq!(secret_finding("-----BEGIN CERTIFICATE-----"), None);
    }
}
