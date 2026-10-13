# The official mobile package: first inventory

What the project's own tools measured on the official mobile reference, the first original material
held. **Sanitized metadata only**: names, sizes, hashes, signatures and measurements. The files
themselves are in the private source root and are never committed.

Everything here describes **the mobile implementation, Mighty Fu Casino 5.6.0**. None of it is
evidence about the Australian cabinet game's mathematics ([../EVIDENCE.md](../EVIDENCE.md), rule 4).

## Provenance

| | |
|---|---|
| App | Mighty Fu Casino, Product Madness, package `com.productmadness.fafafagold` |
| Version | 5.6.0 (version code 262821042) |
| Obtained | 2026-10-13, installed by the owner from the Google Play Store (installer `com.android.vending`) on an Android 16 emulator on the project's Windows host, then copied out with `adb pull` of the paths reported by `pm path` |
| Source class | official mobile |
| Held at | `official-mobile/com.productmadness.fafafagold/5.6.0/` in the private source root, read-only, with a provenance record and hash lists |

### Installed package files

| File | Bytes | SHA-256 |
|---|---:|---|
| `base.apk` | 96,098,199 | `df209e82489dbdfb9698cb22bef55bb565a330ca4621a13327b10b04290949c5` |
| `split_config.arm64_v8a.apk` | 90,294,005 | `fb9021d295cfa4066ef889e79d9ce0b8ca86f3758f462d6e811a86c2a900c500` |
| `split_config.xxhdpi.apk` | 278,091 | `1772d3bc1fe404ba04ef8f7023fc8f5a170bd248dd23894c366219043bcb4002` |
| `split_UnityDataAssetPack.apk` | 161,393,069 | `9e3bfe2dd1f59407b4f9d8f76d7d7d635e17d500c0086e5d811055bd9dfe7dc7` |

### Content the app downloaded at run time

After the owner opened Golden Century once, the app's external storage held 113 files (168 MB),
copied to `…/5.6.0/device-files-2026-10-13/` with a hash list. This storage is readable with `adb`
on a stock Play Store image; the app's private data directory is not, and was not accessed.

## Findings

Labels: VERIFIED here means measured on the held files by a committed tool
(`dragonlink-inventory`, `dragonlink-inspect`) and reproducible from them.

### The application

| Finding | Basis | Label |
|---|---|---|
| The app is built on **Unity with IL2CPP** | `assets/bin/Data/Managed/Metadata/global-metadata.dat` (25.3 MB) in `base.apk`; `lib/arm64-v8a/libil2cpp.so` (146.8 MB) and `libunity.so` (28.4 MB) in the ABI split; a split named `UnityDataAssetPack` | VERIFIED (file presence and names) |
| Native code is shipped for **arm64-v8a only** | the only ABI split installed; 18 `.so` files | VERIFIED |
| Audio middleware is **FMOD** | `libfmodex.so` | STRONG EVIDENCE (file name) |
| A second very large native library, `libAndroidHostExtension.so` (132.8 MB), is present | file listing | VERIFIED that it exists; what it is: **UNKNOWN**, under analysis |
| Asset bundles ship in the install: a 119 MB `main.assetbundle`, an `embedded` bundle and seasonal bundles | `assets/EmbeddedBundles/` in the asset-pack split | VERIFIED |
| The base package contains 34 `.proto` files | file listing | VERIFIED; what they describe: not yet examined |
| Individual slot games are **not** in the install; they are downloaded when first opened | no per-game content in the APKs; a per-game folder appeared on the device after opening the game | STRONG EVIDENCE |

### How a game is delivered

Golden Century arrived as a folder named `GDK/Assets/dlgoldencentury/` beside `GDK/Assets/common/`.

| Finding | Basis | Label |
|---|---|---|
| Game content is in files with the extension `.mercury`, beginning with the bytes `4D 45 52 43` ("MERC"), entropy 3.4 to 4.4 bits per byte | `dragonlink-inspect` on `help.mercury`, `5x3_5Paylines_LL.mercury`, `AssetLibrary.mercury` | VERIFIED: a structured container, **not encrypted**. The format itself is unknown and under analysis |
| The game's content modules are named for their function: `AssetLibrary` (69 MB), `main`, `HoldAndSpinFeature`, `help`, `jackpotMeters`, `buttondeck`, `FeatureBanner`, `CoinShower_DL`, `Overlay`, `GlobalSounds`, `banners_LL`, `upi`, and `5x3_5Paylines_LL` | file names | VERIFIED as names |
| Game tables are in 49 files with the extension `.enc`. All begin with the same 16 bytes, every size is a multiple of 16, entropy is 7.85 to 7.93 bits per byte | `dragonlink-inspect`; sizes modulo 16 | VERIFIED as measured. That they are block-cipher encrypted is a HYPOTHESIS consistent with all three measurements |
| The tables are named `main_reelstrips`, `feature_reelstrips`, `respin_reelstrips` (each also `_no_bonus` for the first two), `configuration`, `configuration_highroller`, `GameConfig` | file names | VERIFIED as names; contents not read |
| There are four **variations**, folders `88`, `90`, `92`, `94`, each holding `Scat2_WeightTables`, `weighting`, `reveal`, `BracketManager`, `configuration`, `main_slot`, `feature_slot`, `respin_slot`, `respin_main_slot`, `respin_feature_slot` | file names | VERIFIED as names |
| Between the four variations, `main_slot`, `weighting` and `reveal` are byte-identical; `Scat2_WeightTables` differs in all four; `configuration` has two distinct contents | SHA-256 comparison | VERIFIED |
| The variation names are return percentages | nothing but the numbers | HYPOTHESIS |
| Language strings are plain XLIFF text | read directly | VERIFIED |

### What the plain-text strings show

Read from `GDK/Assets/dlgoldencentury/Language/Game/en-us.xlf` and
`GDK/Assets/common/Language/en-us.xlf` (41 and 42 strings). Described, not reproduced.

| Finding | Label |
|---|---|
| The game's promotional lines state that **three of the symbol it calls "Scatter1" start Free Games** and **six of the symbol it calls "Scatter2" start the Hold & Spin feature** | VERIFIED for this mobile build. It agrees with the rules Aristocrat states for the US cabinet family ([FEATURES.md](FEATURES.md)), and the name "Scatter2" matches the court order's "SCAT2" for the prize symbol |
| The bet is expressed as credits per line ("x BET PER LINE", "credit per line") | VERIFIED for this build |
| The strings are written for a **gaming machine**: pays by printed ticket and by an attendant above a stated amount, a "machine reserved" timer, a gamble feature with red/black and suit selection, inserting coins or credits, changing denomination, and "new set of reels selected" | VERIFIED that the strings are present. They are evidence about where the package comes from, not about what the mobile app lets a player do |
| String identifiers are prefixed `GDK_`, and the content folder is named `GDK` | VERIFIED |

**Reading of this, at STRONG EVIDENCE:** the mobile app hosts each slot game as a package built on
Aristocrat's own game framework, with content and strings that originate on the machine side,
rather than as a game re-created for mobile. That makes the mobile build a close relative of a
cabinet build. **It does not show that its tables equal those of any Australian cabinet build**:
which market or version the package derives from, and what was changed for mobile, is UNKNOWN.

## All eight Dragon Link packages

After the owner opened each Dragon Link title once, a second snapshot of the app's storage (249
files, 432 MB) was copied and hashed. Findings, from **listing names and comparing hashes only**:

| Finding | Label |
|---|---|
| Each title's download is kept as one ZIP archive under `DefaultWebCache/SlotAssets/<title>/`, about 20 MB (Spring Festival 43 MB), unpacking to the `GDK/Assets/<title>/` layout described above | VERIFIED |
| The app keeps only the most recently loaded game's unpacked folder; the archives remain | VERIFIED (the Golden Century folder was replaced; its archive is byte-identical in both snapshots) |
| Eight titles are held: Golden Century, Happy & Prosperous, Panda Magic, Autumn Moon (in scope), and Genghis Khan, Golden Gong, Peace & Long Life, Spring Festival | VERIFIED |

What the names show for each title. These are file names, so everything in this table is a
**lead** about content until the files are read.

| Title | Payline module | Variations | Main reel strips | `reveal` table | High-roller configuration |
|---|---|---|---|---|---|
| Golden Century | `5x3_5Paylines_LL` | 88 90 92 94 | one set, shared | yes | yes |
| Autumn Moon | `5x3_5Paylines_LL` | 88 90 92 94 | one set, shared | yes | yes |
| Happy & Prosperous | `5x3_5Paylines_LL` | 01 88 90 99 | per variation | no | yes |
| Panda Magic | `5x3_5Paylines_LL` | 01 88 90 99 | per variation | no | yes |
| Genghis Khan | `5x3_50Paylines` | 88 90 92 94 | one set, shared | no | no |
| Golden Gong | `5x3_50Paylines_LL` | 88 90 92 | per variation | no | no |
| Peace & Long Life | `5x3_25Paylines_LL` | 88 90 92 94 | one set, shared | no | no |
| Spring Festival | `5x3_25Paylines_LL` | 88 90 92 | one set, shared | no | no |

Three things stand out, each to be tested against the contents:

- The `reveal` table exists for exactly the two titles whose official pages describe free games in
  which the mystery symbols all reveal the same symbol ([FEATURES.md](FEATURES.md)).
- Variation names are not uniform (`01` and `99` appear), which weakens the hypothesis that they
  are return percentages.
- Titles differ in whether the main reel strips are shared by all variations or supplied per
  variation, so "what a variation changes" is not one answer for the family.

## Structural analysis (2026-10-13)

A static analysis pass over the held files, on the private side. "Checked" says how each finding
stands: *re-checked* means repeated independently; *one pass* means reported by a single analysis
and not yet reproduced by a committed tool, so it is STRONG EVIDENCE at most.

| Finding | Checked |
|---|---|
| The large native library is the manufacturer's own native game runtime: its symbol names are dominated by the manufacturer's and the container format's namespaces, with slot-game, recovery, trace and market-configuration components. It is not specific to a title; one runtime loads any game folder | one pass |
| The container format (`MERC`) is understood well enough that a private parser reads every such file of all eight titles with internal cross-checks: a tree of typed nodes and properties followed by a block of bulk data. Node types include timelines, sprites, text, sounds, textures, video textures and fonts | one pass |
| Golden Century's asset library holds 114 audio clips in Ogg Vorbis, 95 textures (uncompressed RGBA, ETC1, and video textures in WebM), and 18 bitmap fonts | one pass |
| The help and pay-table screens are **images**, not text. Pay values cannot be read from text in the clear files | one pass |
| Golden Century's payline module defines **five** paylines on a five-by-three window | one pass |
| The symbol set as the game names it: four picture symbols, five card symbols (Nine to King), a wild, "Scatter1" (Free Games) and "Scatter2" (the Hold & Spin prize symbol). This matches the classes Aristocrat states for the US cabinet family ([FEATURES.md](FEATURES.md)) | one pass |
| Four jackpot levels are named: Grand, Major, Minor, Mini | one pass |
| There are 350 `.enc` files across the eight titles | re-checked |
| **Golden Century and Autumn Moon**: 48 `.enc` files each, **45 byte-identical**; the three that differ are the `configuration` files of variations 88, 90 and 92 | re-checked |
| **Happy & Prosperous and Panda Magic**: 47 `.enc` files each, **46 byte-identical**; the one that differs is the top-level `configuration` | re-checked |
| `respin_reelstrips` is the same file in all four titles in scope | re-checked |
| Golden Century against Happy & Prosperous: of 25 files with the same name, one is identical | re-checked |

**What follows from the identical files.** The table files are encrypted with one scheme, and
identical encrypted files hold identical contents. So in this mobile build the four titles in scope
are **two pairs that share their tables**: Golden Century with Autumn Moon, and Happy & Prosperous
with Panda Magic, each pair differing in artwork, sound and a few configuration files. That is
consistent with the official pages, which describe the same free-games mechanic within each pair.
Label: STRONG EVIDENCE, for the mobile build only.

### What has not been read, and will not be by this project's tooling

The `.enc` tables are protected, and the key material is in the app. Recovering it and decrypting
the tables was **stopped by the safety controls of the AI tooling this project is built with**, as
was exporting the help and pay-table artwork from the containers. Those controls are treated as a
boundary, not worked around. Consequently:

- no reel strip, weight table, pay value or variation parameter of the mobile build has been read;
- what the variation names mean is still a hypothesis;
- the rules and pay tables are pursued from footage, the litigation record and archives instead
  ([../BLOCKERS.md](../BLOCKERS.md)).

Whether that content is read by other means is the owner's decision and outside this repository.

## What this changes

- **The mobile implementation's tables are on the device**, not only on a server, but protected;
  they have not been read (see above), so rung 2 of the mathematics ladder
  ([math/README.md](math/README.md)) is not available through this project's tooling.
- **The four titles are two mathematical pairs in this build**, which halves the number of distinct
  models to understand.
- **Leads, each to be tested and none yet established:** that the module name `5x3_5Paylines_LL`
  means this build plays five lines on a five-by-three window with a Lightning Link framework
  module; that the Hold & Spin feature has reels of its own (`respin_reelstrips`), as the court
  order's grouping suggested; that the variations differ in the prize-symbol weight tables and not
  in the reels.

## Not done

Nothing has been decrypted. No asset has been exported from a container or imported. The
IL2CPP side of the app has not been examined.
