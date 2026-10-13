# Source material

Where original material comes from, where it is kept, and how it is handled. Read with
[EVIDENCE.md](EVIDENCE.md) (how findings are labelled) and [LEGAL.md](LEGAL.md) (scope).

## What is held today

**No original material.** As of 2026-10-13 the project holds no Dragon Link cabinet software, no
Helix or MarsX images, no proprietary dumps, no extracted assets and no official mobile packages.
Everything is built on the assumption that evidence accumulates over time, and so that each new
source slots in without restructuring.

## What may be acquired

Lawfully and legitimately available material only:

- official software from its official distribution channel (for example an official social-casino
  app from the platform's own store, on a device or emulator the user controls);
- regulatory filings, approved-game registers and technical standards;
- patents, manuals, brochures and official promotional material;
- videos and screenshots, including the user's own recordings;
- public research and public repositories, within their licences;
- cabinet software or machine images the owner lawfully possesses and provides.

Never: pirated cabinet software, stolen or leaked credentials, access to operational gaming systems,
tampering with real machines. See [LEGAL.md](LEGAL.md).

## The private source root

All original material, and everything derived from it that is not publishable, lives in one folder
**outside any Git work tree**, named by an environment variable:

```
DRAGONLINK_SOURCE_ROOT=<a folder on a local data drive, not inside a repository>
```

Source code never contains a machine-specific path; tools read the variable, and say clearly what to
do when it is unset. Tests that need real material skip when it is unset.

### Layout

```
<DRAGONLINK_SOURCE_ROOT>/
  dragonlink-source-root.toml   marker written by the tooling: schema, creation date, notes
  official-mobile/              official app packages exactly as obtained, one folder per app and version
  cabinet/                      cabinet software and machine images exactly as obtained (none yet)
  captures/                     recordings and screenshots of the original being played
  extracted/                    working copies and unpacked contents, one folder per source
  decompiler/                   Ghidra / JADX / Apktool projects and output
  audio/  images/  video/       loose reference media that is not part of a package
  manifests/                    inventories and manifests produced by the tools
  hashes/                       hash lists of originals, for integrity checks
  math/                         recovered game configurations and reports generated from them
  research/                     private notes, strings dumps, anything not fit to publish
  imported/                     the normalised runtime asset set (default DRAGONLINK_DATA_DIR)
  packages/                     local full packages (never distributed)
  temporary/                    scratch; safe to delete
```

### Handling rules

1. **Originals are immutable.** Files under `official-mobile/` and `cabinet/` are never modified,
   patched, renamed or re-compressed. They are hashed on arrival and the hash is recorded.
2. **Work on copies.** Unpacking, mounting, decompression and analysis happen under `extracted/`,
   `decompiler/` or `temporary/`.
3. **Record provenance on arrival.** Where it came from, when, which version, the SHA-256, and for
   downloaded material the exact official channel.
4. **Nothing flows back into Git except sanitized facts**: hashes, sizes, names, counts, structures,
   measurements, and descriptions written in our own words.
5. **Archives are untrusted input.** Extraction uses a decompression budget and a safe output tree
   (no path traversal, no symlink escape).

### Intended workflow for a new source

1. Place the source under `official-mobile/` or `cabinet/`.
2. Inventory and hash it.
3. Identify containers, partitions, file systems and packages.
4. Extract to a working copy without touching the original.
5. Inspect executables and data.
6. Identify game resources and configuration.
7. Record provenance and findings (claims, research docs).
8. Import useful assets and data into the normalised asset set.
9. Compare behaviour against existing findings.
10. Update parity status and the tracker.

Step 2 exists (`dragonlink-inventory scan`); steps 3, 4 and 8 are tracked in
`progress/progress.toml` and are not written yet.

### Creating and checking the source root

```sh
cargo run -p dragonlink-inventory -- init <folder outside any repository>
# set DRAGONLINK_SOURCE_ROOT to that folder, then:
cargo run -p dragonlink-inventory -- check
cargo run -p dragonlink-inventory -- scan <a folder of source material>
```

`init` and `check` refuse a location inside a Git work tree. `scan` reads and never modifies; it
records relative paths, sizes, SHA-256, what the content looks like, entropy, and the entry list
of ZIP-family packages, and writes the inventory to `manifests/` in the source root. It refuses
to write an inventory into a repository.

## The normalised asset set

The runtime does not read the source root. An importer converts recognised material into a
normalised local asset set (`DRAGONLINK_DATA_DIR`, by default `imported/` under the source root)
with a machine-readable manifest that records, per asset: logical id, source package and path,
source SHA-256, imported path, output SHA-256, media type and properties, title, category,
provenance class, confidence, and import method.

Without an asset set the application runs in placeholder mode: generated shapes and labels, with a
banner saying so. Placeholder mode is not fidelity evidence.

## Mobile versus cabinet

Official mobile software is a legitimate reference for presentation. It is not assumed to share
mathematics with the Australian cabinet game (see [EVIDENCE.md](EVIDENCE.md), rule 4). Material and
findings from the two are kept in separate folders and carry separate scopes, so that one cannot be
mistaken for the other.
