# Legal scope

dragonlink-decomp is an **unofficial**, non-commercial software preservation, study and
reimplementation project. It is not affiliated with, endorsed by, or connected to Aristocrat Leisure
Limited, Aristocrat Technologies, Product Madness or any of their affiliates. *Dragon Link*,
*Golden Century*, *Happy & Prosperous*, *Panda Magic*, *Autumn Moon*, *Hold & Spin* and all related
names, marks, artwork, audio and software belong to their respective owners.

This document describes how the project is run. It is not legal advice.

## What this project is for

Preserving and understanding how these games present and behave, by writing an independent
implementation from evidence. It is a simulator for study.

## What this project is not for

- **Not real-money gambling.** Credits in this software are simulation credits. They cannot be bought,
  cashed out or exchanged, and stand for no money. Public builds use simulation credits only.
- **Not for gaming machines.** Nothing here is designed to be installed on, connected to, or to
  interoperate with an operational gaming machine, a venue system, a monitoring system or a jackpot
  controller. No such functionality will be added.
- **Not a tool for cheating or tampering.** The project does not develop, document or accept methods
  for manipulating regulated gaming equipment, predicting or influencing outcomes on real machines,
  bypassing venue or machine security, or forging machine or jackpot communications.
- **Not a statement about any real machine's odds.** Until mathematics is recovered from cabinet or
  regulatory evidence, every number the simulation runs on is synthetic and labelled so. Nothing this
  software does predicts the behaviour of a real machine.

## What this repository contains

- Independently written Rust code, tooling and tests.
- Documentation of file formats, program structure and observed behaviour, written in our own words.
- Sanitized metadata: file hashes, sizes, counts, names and signatures where needed to explain structure.
- Measurements and behavioural descriptions with their sources.
- Synthetic test fixtures and placeholder assets created by this project.
- Citations of public sources.

## What this repository never contains

- Aristocrat or Product Madness software in any form: cabinet software, disk, CFast, firmware or ROM
  images, Android or iOS packages, native libraries, bytecode.
- Extracted or converted proprietary assets: textures, sprites, fonts, music, sound, voice, video,
  animation data, unpacked or decrypted bundles.
- Decompiled or disassembled code in bulk, decompiler project databases, or verbatim proprietary source.
- Large raw dumps (`strings`, symbol tables, resource listings with payloads).
- Cryptographic keys, account tokens, credentials, or anything that circumvents access control.
- Recordings or screenshots of the original games.
- A packaged, playable copy containing original assets.

## Game mathematics

The manufacturer treats these games' mathematics (reel strips, weights, probabilities, return models)
as trade secrets, and has said so in court. The project therefore distinguishes:

- **player-visible facts** (rules, pay tables, denominations, displayed return ranges, meter values),
  which are researched from public sources and recordings and published with their sources;
- **recovered mathematics**, which, if ever obtained from lawfully held software, is kept in the
  private source root like any other proprietary material and is not committed here by default.

The analysis tools in this repository operate on a configuration file supplied locally. The only
configuration distributed is a synthetic fixture invented by the project.

## How source material is obtained

Only lawfully and legitimately: official software from its official distribution channel, material
the user owns or is authorised to hold, regulatory filings, patents, manuals, official promotional
material and public research. The project does not use pirated cabinet software, stolen or leaked
credentials, or access to operational gaming systems. See [SOURCES.md](SOURCES.md).

Users supply their own legitimately obtained source material. Tools in this repository read it locally
and write converted data to a private location outside the repository. Converted data and local
full packages must not be redistributed.

## Screenshots

`docs/images/` may hold a few screenshots or short clips of **this project's runtime in placeholder
mode**, showing only synthetic or generated art, so that a visitor can see what the software does.
Nothing rendered from imported original assets is committed, and nothing from the original games'
own screens. `tools/repo-hygiene` refuses image files anywhere else in the repository and refuses
texture, audio, video and font formats everywhere. If a rights holder asks for an image to be removed,
it is removed.

## Enforcement

- The private source root is outside the Git work tree by design.
- `.gitignore` excludes source material, extractions, RE databases and secrets by name.
- `tools/repo-hygiene` (run locally, in the Windows gate and in CI) refuses forbidden extensions,
  the magic bytes of archives, executables, images of file systems, engine bundles, audio and fonts,
  oversized files, image files outside `docs/images/`, the directory names of the source root,
  credential shapes and private absolute paths, and flags decompiler fingerprints.
- Release packaging contains this project's code only. A local full package is a separate,
  explicit, local-only workflow whose output is ignored by Git and refused by the hygiene tool.

## Contact

A rights holder with a concern about anything in this repository can open an issue or contact the
repository owner; it will be dealt with promptly.
