//! AMD GPU command processing, mirroring `legacy/gpu/shadps4/video_core/amdgpu/`.
//!
//! Ported so far: tile-mode math (`tiling.rs`) and the PM4 packet/opcode layer (`pm4.rs`).

pub mod cb_db_extent;
pub mod liverpool;
pub mod pixel_format;
pub mod pm4;
pub mod regs;
pub mod regs_color;
pub mod regs_depth;
pub mod regs_primitive;
pub mod regs_shader;
pub mod regs_texture;
pub mod regs_vertex;
pub mod resource;
pub mod tiling;
