//! Core library of the Rust launcher port.
//!
//! The Rust files keep the folder structure of the sources they replace: `launcher/` mirrors
//! `launcher/*.py` (with `launcher_core/` for `bbport_launcher.py`'s parts), `scripts/`
//! mirrors `scripts/*.py`, `src/` will hold the loader/runtime port of `src/*.c`, `gpu/` the
//! renderer port of `gpu/**`, and `util/` the helpers with no direct source counterpart. The
//! original sources live in `legacy/` (ignored by git).

#![allow(function_casts_as_integer)]

#[cfg(feature = "port-modules")]
pub mod gpu;
pub mod launcher;
pub mod scripts;
#[cfg(feature = "port-modules")]
pub mod src;
pub mod util;

// Names of the Python modules the ports replace, kept as aliases so callers keep working while
// the tree mirrors the originals.
pub use launcher::bbport_activity as activity;
pub use launcher::bbport_assets as assets;
pub use launcher::bbport_coop as coop;
pub use launcher::bbport_i18n as i18n;
pub use launcher::bbport_net as net;
pub use launcher::bbport_pkg_setup as pkg_setup;
pub use launcher::bbport_readiness as readiness;
pub use launcher::bbport_startup as startup;
pub use launcher::bbport_steam as steam;
pub use launcher::bbport_trophies as trophies;
pub use launcher::bbport_vulkan as vulkan;
pub use launcher::launcher_core::{env, ini, paths, process, settings, update};
pub(crate) use util::fsutil;
pub(crate) use util::zipread;

pub use scripts::mods;
pub use scripts::patches;
pub use scripts::prepare as sfo;

pub use launcher::bbport_readiness::{Check, Level, SyncOutcome};
pub use launcher::bbport_startup::{Route, RouteMode};
pub use launcher::launcher_core::ini::Ini;
pub use launcher::launcher_core::paths::Paths;
pub use launcher::launcher_core::settings::Settings;
pub use scripts::prepare::GameInfo;
pub use update::VERSION;
