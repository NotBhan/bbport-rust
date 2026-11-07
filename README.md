# bbport-rust

The Rust launcher for the Bloodborne PC port: a standalone native GTK4 control surface and the
Rust reimplementation of the launcher/runtime modules.

**Repository:** <https://github.com/NotBhan/bbport-rust>

> No game files are included. You must provide your own decrypted Bloodborne dump
> (CUSA03173, version 1.09). This project is not affiliated with Sony Interactive
> Entertainment or FromSoftware.

## What this repository contains

- `bbport-gui` — the GTK4 launcher. It has a sidebar with Play, Display, Graphics, Upscaling,
  Performance, Compatibility, Controller, Game & Data, Launcher, Trophies and Diagnostics.
- `bbport-rs` — the headless CLI: readiness, settings, INI, mods, patches, update checks and
  launch.
- `launcher/`, `scripts/`, `src/` and `gpu/` — the progressively ported launcher, package,
  runtime and renderer modules, with tests for each unit.
- `run-rust-gui.sh` / `run-rust-launcher.sh` — build-if-needed wrappers.

The GUI resolves the port tree in this order: `BB_PORT_DIR`, the discovered port root, then
sibling `linux_launcher` / `windows_laucnher` trees containing `run.sh`. **Launch Game** starts
the port's existing `run.sh` with the settings/environment from this launcher.

## Releases

The current beta is [v0.1.0-beta.2](https://github.com/NotBhan/bbport-rust/releases/tag/v0.1.0-beta.2):

- **Linux x86_64** — `bbport-rust-*-linux-x86_64.AppImage`: self-contained, including the
  GTK4 runtime and data. Make it executable and run it. Use
  `--appimage-extract-and-run` if FUSE is unavailable. A plain tar.gz is also provided.
- **Windows x86_64** — `bbport-rust-*-windows-x86_64-setup.exe`: per-user installer with the
  GTK4 runtime bundled; no separate GTK installation is needed. A portable zip with
  `run-bbport-gui.cmd` is also provided.

Both still require a Bloodborne port tree containing `run.sh`; no game files are distributed.

## Requirements

- Linux x86_64 or Windows 10/11 x86_64 to run the launcher.
- Rust stable (2021 edition) and GTK4 development libraries only when building from source.

On Arch:

```bash
sudo pacman -S rust gtk4
```

## Build and run

```bash
cargo build --release
./run-rust-gui.sh        # build if needed, then open the GTK4 launcher
./run-rust-launcher.sh status
cargo run --bin bbport-rs -- settings list
```

Windows uses the launcher-only feature set, which excludes the Unix port modules:

```bash
cargo build --release --no-default-features
```

Useful CLI commands:

```bash
cargo run --bin bbport-rs -- status
cargo run --bin bbport-rs -- route
cargo run --bin bbport-rs -- settings list
cargo run --bin bbport-rs -- ini list
cargo run --bin bbport-rs -- mods
cargo run --bin bbport-rs -- patches
cargo run --bin bbport-rs -- check-update
cargo run --bin bbport-rs -- launch --port-dir /path/to/port
```

## Configuration

The launcher reads and writes the same files as the existing port:

- `settings.json` under `$XDG_CONFIG_HOME/bbport-launcher/` (or `%APPDATA%` on Windows).
- `bbport.ini` in the port data directory, preserving comments and unknown keys.

Settings include display/presentation, frame pacing, upscaling preset, sharpening and effects,
controller scheme and `key_<action>` bindings, game/save/mod/patch/DLC folders, language, mods,
enhancements, cheats, UI preferences and diagnostics. Values are validated and written
atomically.

## Tests

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets
cargo build --release
```

The suite covers the launcher core, settings/INI round-trips, readiness, packages and ZIP
parsing, mods/patches, networking, trophies, GUI path discovery and the ported runtime/FFI
modules.

## Relationship to the parent port

This crate is a source copy of the Linux launcher tree focused on the Rust reimplementation. The
legacy Python/C/C++ sources are kept outside this repository (in the parent tree under
`legacy/`) for reference. The port tree's `run.sh`, Vulkan renderer and game runtime are not
built by this crate; the launcher invokes them from the configured/sibling port directory.

## Attribution and license

This work derives from the Bloodborne PC port by Jgamrot, Supermedo, deadinside28, the shadPS4
project and the other contributors listed in the parent project. Third-party licenses are kept
in [`licenses/`](licenses/) and the project is licensed under GPL-2.0-or-later; see
[`LICENSE`](LICENSE).
