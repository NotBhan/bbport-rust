//! `shim/` — the GPU library's adaptation layer, mirroring `legacy/gpu/shim/`.
//!
//! Ported so far: the map direction helper, the process-memory query and the Windows/Linux
//! thread-id compatibility helper.

pub mod bb_win_compat;
pub mod bbgpu;
pub mod bbport_atlas_interiors;
pub mod bbport_atlas_menu;
pub mod bbport_atlas_menu_state;
pub mod bbport_atlas_paths;
pub mod bbport_atlas_style;
pub mod bbport_copy;
pub mod bbport_map;
pub mod bbport_map_art;
pub mod bbport_map_controller;
pub mod bbport_map_direction;
pub mod bbport_map_features;
pub mod bbport_overlay;
pub mod bbport_process_memory;
pub mod bbport_settings;
pub mod bbport_threads;
pub mod bbport_toggles;
pub mod bbport_trophies;
pub mod bbport_write_log;
pub mod common;
pub mod core;
pub mod imgui;
pub mod sdl_window;
pub mod stage_point;
pub mod tracy;
pub mod window;
