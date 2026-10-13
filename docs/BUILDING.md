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
| Node.js | 22.17.0 | not used by the build |
| ffmpeg | 8.1 | for inspecting and measuring reference media |
| winget | 1.29 | used by the setup script |

Not installed yet, to be installed when the work that needs them starts: 7-Zip, Android platform
tools (ADB), Ghidra, JADX, Apktool. Reverse-engineering applications that have no installer are
unpacked under a tools folder next to the private source root, never inside the repository.

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
