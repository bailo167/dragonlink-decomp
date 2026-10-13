# Building and testing

## Requirements

- **Rust 1.98.0**, pinned in `rust-toolchain.toml`. With `rustup` installed, the first `cargo`
  command installs the toolchain, `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target.
- **Windows**: Visual Studio Build Tools 2022 with the C++ workload (the MSVC linker).
- **Linux** (for the Bevy application, when it exists): `pkg-config libasound2-dev libudev-dev
  libwayland-dev libxkbcommon-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev`.
- **Git**.

No original game material is needed to build, test or run.

## Running the application

```sh
cargo run -p dragonlink                                   # placeholder mode
cargo run -p dragonlink -- --seed 7 --credits 5000
cargo run -p dragonlink -- --smoke-frames 600 --screenshot shot.png   # unattended, then a picture
```

Space or Enter spins and steps a feature; Up and Down change the bet; Escape quits.
Without imported assets the application runs in placeholder mode, with a banner saying so.
An unattended run advances on a fixed time step, so the same seed and frame count show the same
thing on any display.

## Everyday commands

```sh
cargo test --workspace                                    # all headless tests
cargo clippy --workspace --all-targets -- -D warnings     # lints, warnings are errors
cargo fmt --all --check                                   # formatting
cargo run -p repo-hygiene                                 # nothing proprietary in the tree
cargo run -p repo-hygiene -- --staged                     # only what is staged
cargo run -p progress-gen                                 # regenerate progress.svg and the README table
cargo run -p progress-gen -- --check                      # fail if the tracker is invalid or stale
```

## The Windows gate

The Windows host is the authoritative integration and test machine. Every meaningful change is run
through the gate there before it merges to `main`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\gate.ps1 -Release
```

| Step | What it runs | State |
|---|---|---|
| `fmt` | `cargo fmt --all --check` | active |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | active |
| `test` | `cargo test --workspace --locked` (simulation, fixtures, tools) | active |
| `hygiene` | `cargo run -p repo-hygiene` | active |
| `progress` | `cargo run -p progress-gen -- --check` | active |
| `app` | `cargo build -p dragonlink` | active |
| `smoke` | `dragonlink --smoke-frames 180`: open the window, spin unattended on a fixed time step, exit 0 | active |
| `wasm` | `cargo build -p dragonlink --target wasm32-unknown-unknown` | active |
| `release` | `cargo build --release -p dragonlink` (with `-Release`) | active |

Each step prints one line; full output is in `target\gate\<step>.log` and a summary in
`target\gate\summary.json` (commit, whether the tree was dirty, toolchain, per-step result and time).
All steps run even after a failure. The exit code is 0 only if every step that ran passed.

Linux, macOS and web CI (`.github/workflows/ci.yml`) are a second opinion on other machines. They do
not replace the gate.

## Windows host

`tools\dev\setup-windows.ps1` reports the tooling it finds, one line per tool, and with `-Install`
installs what is missing through `winget`. It never reboots the machine.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\setup-windows.ps1 -Tier all
```

### Recorded environment

The host the gate currently runs on, recorded so the environment can be reproduced:

| Component | Version | Notes |
|---|---|---|
| Windows | 11 Home, build 26200 | x86_64 |
| Rust | 1.98.0 stable, `x86_64-pc-windows-msvc` | via rustup; `wasm32-unknown-unknown` target added |
| Visual Studio Build Tools | 2022, 17.14 | C++ tools present |
| Git | 2.53.0 | |
| GitHub CLI | 2.81.0 | |
| Python | 3.13.7 | |
| Java | 21.0.12 LTS | required by Ghidra and JADX when they are installed |
| Node.js | 22.17.0 | not used by the native build; serves and checks the web bundle |
| wasm-bindgen CLI | 0.2.129 | `cargo install wasm-bindgen-cli --version 0.2.129 --locked`; must equal the `wasm-bindgen` version in `Cargo.lock` |
| Microsoft Edge | 155 | the headless browser the web check drives; any Chromium-family browser works |
| ffmpeg | 8.1 | for inspecting and measuring reference media |
| winget | 1.29 | used by the setup script |

| yt-dlp | 2026.08.19 | per-user Python package; fetches public video and its metadata for the footage passes |
| Android SDK command-line tools | build 15859902 | checksum verified against Google's download page and SDK index |
| Android Emulator | 37.2.12 | hardware accelerated through the Windows Hypervisor Platform, which was already enabled |
| Android SDK Platform-Tools (`adb`) | 37.0.1 | |
| Android system image | Android 16 (API 36), Google Play, x86_64, revision 7 | runs arm64 apps through translation |

The Android tooling and the virtual device live in a `dragonlink\tools` folder beside the private
source root, outside every repository. The emulator is how official mobile software is obtained:
the owner signs in to the store and installs; the project copies the package out with `adb`.

Not installed yet, to be installed when the work that needs them starts: 7-Zip, Ghidra, JADX,
Apktool, IL2CPP tooling. Reverse-engineering applications that have no installer are
unpacked under a tools folder next to the private source root, never inside the repository.

### The web bundle

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\web.ps1 -Check
powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\web.ps1 -NoBuild -Serve
```

`web.ps1` builds the application for `wasm32-unknown-unknown` with the profile in
`web/cargo-web.toml`, generates the JavaScript glue with `wasm-bindgen` (it refuses a version that
differs from `Cargo.lock`'s), and assembles `web/dist/`, which is build output and is never
committed. `-Check` loads the bundle in a headless Chromium-family browser, driven by
`tools/dev/web.mjs` with Node alone, and passes only if the page loads without an error, the
application starts, the canvas shows the placeholder banner, and a press of Space changes the
picture without scrolling the page. Results, with two pictures of placeholder mode, are in
`target\web\`. `-Serve` serves the bundle on localhost.

The check uses software rendering by default, so it gives the same answer without a GPU. It is not
part of the gate: the gate's `wasm` step proves the target compiles, and this check is run when the
application or the page changes. Checked on this host with Edge 155: the application starts in
about a second. The bundle is 41 MB of WebAssembly, not yet reduced with an optimiser.

### Source root on the host

The private source root was created with `dragonlink-inventory init` on the host's fixed data
drive, in a `dragonlink\source-root` folder beside the sibling project's work folder and outside
every repository, and `DRAGONLINK_SOURCE_ROOT` is set for the user. The boundary was exercised on
2026-10-13: a source root inside the repository is refused; an inventory cannot be written into
the repository; the source root's folder names are ignored by Git; and a synthetic package
renamed to a text extension and copied into the work tree is refused by `repo-hygiene` on its
content.

### Rules for the host

- Install only tooling the project needs, and record it in the table above.
- Never reboot it without the owner's permission. If an installer asks for a reboot, say so and
  carry on with everything that does not need it.
- Never stop processes you did not start; change no power, service or security setting.
- Source material stays under the private source root ([SOURCES.md](SOURCES.md)).

## Tests and source material

Public tests run on synthetic fixtures written by this project. A test that needs real material
reads `DRAGONLINK_SOURCE_ROOT` and skips when it is unset, so CI and a fresh clone always pass
without any original data.
