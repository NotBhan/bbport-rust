# Rust launcher port

This is a source copy of `linux_launcher` used to reimplement the port in Rust. The Rust crate
is the tree root (`Cargo.toml`, `lib.rs`, `main.rs`); the Rust files keep the folder structure
of the sources they replace:

```
lib.rs, main.rs        crate root and the `bbport-rs` CLI
gui.rs                 `bbport-gui`: native GTK4 launcher window
launcher/              mirrors launcher/*.py
  bbport_activity.rs         bbport_activity.py
  bbport_assets.rs           bbport_assets.py
  bbport_coop.rs             bbport_coop.py
  bbport_i18n.rs             bbport_i18n.py (+ generated bbport_i18n_catalog.json)
  bbport_net.rs              bbport_net.py
  bbport_pkg_setup.rs        bbport_pkg_setup.py
  bbport_readiness.rs        bbport_readiness.py
  bbport_startup.rs          bbport_startup.py
  bbport_steam.rs            bbport_steam.py
  bbport_trophies.rs         bbport_trophies.py
  bbport_vulkan.rs           bbport_vulkan.py
  launcher_core/             parts of bbport_launcher.py
    paths.rs, settings.rs, ini.rs, env.rs, update.rs, process.rs
scripts/               mirrors scripts/*.py
  mods.rs, patches.rs, prepare.rs (SFO parser)
util/                  helpers with no Python counterpart (fsutil, zipread)
src/                   milestone 3: the Rust port of src/*.c
gpu/                   milestone 3: the Rust port of gpu/**
```

The original Python/C/C++ sources moved to `legacy/` (see `legacy/README.md`) and `legacy/` and
`/target/` are ignored in git. The legacy build/run pipelines (`build.sh`, `run.sh`, `run.py`)
are in `legacy/` too, so `bbport-rs launch` waits on the run-pipeline milestone.

## Status

### Milestone 1 — launcher core (done)

| Python | Rust | State |
|---|---|---|
| `bbport_launcher.py` paths, defaults, settings, `bbport.ini` | `launcher/launcher_core/{paths,settings,ini}.rs` | ported |
| `prepare.sfo`, `game_info`, `save_title_id` | `scripts/prepare.rs` | ported |
| `bbport_readiness.py` | `launcher/bbport_readiness.rs` | ported |
| `bbport_launcher.py` `game_environment` | `launcher/launcher_core/env.rs` | ported |
| update check + cache | `launcher/launcher_core/update.rs` | ported (`curl` fetcher) |
| `bbport_startup.py` | `launcher/bbport_startup.rs` | ported |
| `--play` / `run.sh` launch | `launcher/launcher_core/process.rs` | ported (`exec` on Unix) |

### Milestone 2 — the remaining launcher Python files (done)

| Python | Rust | State |
|---|---|---|
| `bbport_i18n.py` | `launcher/bbport_i18n.rs` + catalogue | ported |
| `bbport_activity.py` | `launcher/bbport_activity.rs` | ported |
| `bbport_trophies.py` | `launcher/bbport_trophies.rs` | ported |
| `bbport_assets.py` | `launcher/bbport_assets.rs` | ported |
| `bbport_net.py` | `launcher/bbport_net.rs` | ported |
| `bbport_steam.py` | `launcher/bbport_steam.rs` | ported |
| `bbport_vulkan.py` | `launcher/bbport_vulkan.rs` | ported |
| `scripts/mods.py` | `scripts/mods.rs` | ported |
| `scripts/patches.py` (listing/selection) | `scripts/patches.rs` | ported; ELF compiler deferred |
| `bbport_pkg_setup.py` | `launcher/bbport_pkg_setup.rs` + `util/zipread.rs` | ported |
| `bbport_coop.py` | `launcher/bbport_coop.rs` | ported; Windows firewall/host behind `cfg(windows)` |

Deferred from milestone 2:

- ~~`bbport_launcher.py`'s GTK4/libadwaita window, pages and widgets~~ — Rust GTK4
  replacement landed as `gui.rs` / `bbport-gui` (readiness, game folder picker, Launch Game).
- The ELF patch compiler in `scripts/patches.py` (`patch_fingerprint`, `compile_external`,
  SELF parsing) — needs the loader work.
- The run-pipeline scripts (`prepare.py`, `enhancements.py`, `hud_layout.py`, `atlas.py`, ...)
  now in `legacy/scripts/`.

### Milestone 4 — Rust GUI for booting the game (done)

`gui.rs` builds the `bbport-gui` binary with GTK4:

- **Play**: title/version, resolved port path, readiness checks, game-folder picker,
  **Launch Game** (starts the port's `run.sh` with the CLI environment);
- **Display / Graphics / Upscaling / Performance / Compatibility**: settings.json and
  bbport.ini controls (resolution, HUD, HDR, present mode, FPS mode/limit, frames ahead,
  presets, upscaler, sharpening, effects, motion vectors, live resolution, model LOD,
  shader warm-up, profiling, MangoHud, Discord, draw pipeline, readbacks, Vulkan validation);
- **Controller**: control scheme, keyboard-prompt installation and every `key_<action>`
  binding from bbport.ini;
- **Game & Data**: game/save/mod/patch/DLC folders, language, player name, mods, external
  patches, enhancements, cheats and tweaks;
- **Launcher / Trophies / Diagnostics**: UI language, update/motion preferences, extra
  environment, local trophy report, and paths/version diagnostics.

Path discovery prefers `BB_PORT_DIR`, then walks ancestors for a `run.sh` and checks sibling
`linux_launcher`/`windows_laucnher` trees, so the launcher works from `target/release`.

Run it with `./run-rust-gui.sh` (builds the release binary if needed).

### Milestone 3 — port the remaining C/C++/header files (in progress)

The C sources are in `legacy/src/`, the C++ renderer in `legacy/gpu/`; the Rust port goes into
`src/` and `gpu/` with the same sub-paths.

Ported so far:

| C source | Rust | State |
|---|---|---|
| `runtime.h` shared contracts, `runtime.c` `runtime_symbol`/`runtime_lookup` | `src/runtime.rs` | ported |
| `import_names.inc` (750 NIDs) | `src/runtime_imports.rs` (generated) | ported |
| `runtime_time.c` | `src/runtime_time.rs` | ported |
| `runtime_rtc.c` | `src/runtime_rtc.rs` | ported |
| `runtime_mutex.c` (mutexes, condition variables, POSIX wrappers) | `src/runtime_mutex.rs` | ported |

Rendering pipeline (started):

| C++ source | Rust | State |
|---|---|---|
| `video_core/amdgpu/pm4_opcodes.h`, `pm4_cmds.h` headers | `gpu/amdgpu/pm4.rs` | ported (opcodes, type-0/type-3 headers, validated packet stream, pipelining rules) |
| `video_core/amdgpu/tiling.{h,cpp}` | `gpu/amdgpu/tiling.rs` | ported (tile/array/macro modes, splits, macrotile selection) |
| `video_core/amdgpu/pixel_format.{h,cpp}` | `gpu/amdgpu/pixel_format.rs` | ported (formats, swizzles, remaps, conversions, size tables) |
| `gpu/bbgpu.h` | `gpu/bbgpu.rs` | ported (config, trophy summary, achievement layouts, full C ABI) |
| `dlss_bridge/bbport_dlss_bridge.h` | `gpu/dlss_bridge/mod.rs` | ported (ABI 2 types, Vulkan handles, function table) |
| `shim/bb_win_compat.h`, `bbport_map_direction.h`, `bbport_process_memory.h`, `core/tls.h` | `gpu/shim/…` | ported (thread id, map facing, process usage, TLS note) |
| `video_core/amdgpu/regs.{h,cpp}` (flat file, defaults, dirty/delta) | `gpu/amdgpu/regs.rs` | ported; `regs_*.h` bitfield unions pending |

Every file under `legacy/gpu` is tracked in `gpu/PORTING.tsv` with a status and Rust target;
`gpu/PORTING.md` summarises it, and `HANDOFF.md` records the working method, the standing rules
and the next files in order. Current inventory: 2144 files — 590 ported, 0 port-owned
source files pending, 1522 vendored third-party files (`third_party/`, kept as dependencies),
32 build/assets and upstream patch files. Completion means every pending row has a Rust target.

The PM4 port keeps the hardened loader rules: unknown packet types and truncation end the
stream with an error; unknown type-3 opcodes still report their exact length so the caller can
skip them, and tiling lookups return `TilingError` instead of aborting.

Next in this milestone: `runtime_sema.c`, `runtime_rwlock.c`, `runtime_file.c`,
`runtime_memory.c`, `runtime_thread.c`, `runtime_services.c`, `runtime_kernel.c`, then
`runtime.c`/`probe.c` (loader core). GPU remainder: `regs_*.h` bitfield unions, `pm4_cmds.h`
packet bodies, `resource.h`, `liverpool.cpp` decode/register state, `texture_cache` and
`buffer_cache`, `renderer_vulkan` (barriers, pipelines, present, FSR 3.1/4, DLSS bridge), and
the `shim`/`bbnet` layers.

## Audit 2026-11-07

A safety/robustness pass over the port found and fixed these issues:

- `bbport_coop::safe_extract` accepted `a/../../outside` entries (Zip Slip). It now rejects
  empty, absolute, and parent components before extracting; covered by
  `zip_paths_cannot_escape_the_destination`.
- `zipread::ZipArchive::open` trusted the central-directory size and entry count from the
  archive. It now caps the directory at 64 MiB and entries at 1,000,000, and rejects ranges
  beyond the file before allocating.
- `bbport_pkg_setup::extract` read stdout but not stderr; a verbose extractor could fill the
  stderr pipe and deadlock. A reader thread now drains stderr and its tail is included in the
  failure message.
- `runtime_mutex::ensure_cond` treated the destroyed handle value `2` as valid and dereferenced
  it. It now returns `EINVAL`, matching the mutex path; covered by a regression test.
- `pthread.rs` destroy/attribute/lock entry points took handle values rather than the C ABI's
  pointer-to-handle, leaving freed guest handles dangling. They now take and clear the guest
  slot, and the tests cover double-destroy.
- `avplayer_handles::add_source_ex` read a guest `(pointer, length)` pair unchecked. It is now
  an unsafe boundary with null/zero/length validation.
- `slot_vector_commit_pages` ignored `mmap(MAP_FIXED)` failure; it now asserts the mapping
  landed at the requested address, like the C++.
- `vk_bind_helper::fork` is now an unsafe context-lifetime boundary, and a disabled helper runs
  the task inline instead of leaving `join()` waiting forever.
- `equeue` HR-timer registration now marks the guest-bintime dereference unsafe at the API
  boundary; an empty small-timer output buffer returns 0 instead of panicking.
- `bbgpu` rasterizer hooks now clone an `Arc` before calling into the rasterizer, avoiding a
  self-deadlock when map/invalidate callbacks re-enter; sparse-copy raw pointers are now an
  explicit unsafe contract.
- `bbport_net` only read the legacy `frame_cap` key; it now falls back to `fps_limit`, so the
  Windows-style environment also receives `BB_FPS_LIMIT`.
- `zipread::extract_to` now uses checked local-header offset arithmetic, and `fsutil::atomic_write`
  has a Windows replace fallback (Windows `rename` does not overwrite an existing file).
- `get_bda_pointer` in the SPIR-V shader emitter now masks the guest address to the 40-bit
  `BufferCache::ADDRESS_SPACE_BITS` range before indexing the BDA and fault buffers. High or
  sign-extended bits previously produced an out-of-range GPU read, which RADV reported as a
  GPUVM fault at `0x80012b739000` and a lost device. Fixed in both the active C++ renderer and
  the Rust emitter; regression test: `bda_addresses_are_masked_to_40_bits`.


Remaining latent, not wired into the game binary: `bbport_write_log`'s fault-handler dump still
allocates and locks (opt-in `BB_WRITE_LOG`), and the Rust `EqueueApi` trait remains a safe
surface for future implementations. Both should be revisited when those layers are connected.

## Build and test

```bash
cargo test
cargo build --release
```

Latest run: 1136 tests, `cargo fmt --check` and `cargo clippy --all-targets` clean.

## CLI

```bash
cargo run --bin bbport-gui            # native GTK4 launcher (Launch Game boots run.sh)
cargo run -- status                  # readiness checks (game, trophies, saves)
cargo run -- route                   # setup / direct / launcher decision
cargo run -- settings list|get K|set K V
cargo run -- ini list|get K|set K V
cargo run -- mods                    # installed / selected / unrecognized mods
cargo run -- patches                 # external patch XML files and their selection
cargo run -- vulkan                  # host NVIDIA ICD setup status
cargo run -- check-update
cargo run -- launch                  # execs `bash run.sh` (legacy pipeline, port pending)
```

Global flags: `--port-dir DIR` (defaults to the discovered port root) and `--json` for
`status`/`check-update`.

