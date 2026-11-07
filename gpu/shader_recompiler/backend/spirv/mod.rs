//! `shadps4/shader_recompiler/backend/spirv` ports.
//!
//! The C++ backend inherits from the vendored Sirit builder. `builder` is the port's local
//! stand-in for it; `spv` holds the enumeration values from the vendored SPIR-V headers.

pub mod builder;
pub mod emit_context;
pub mod spv;

pub mod emit_spirv;
pub mod emit_spirv_atomic;
pub mod emit_spirv_barriers;
pub mod emit_spirv_bitwise_conversion;
pub mod emit_spirv_composite;
pub mod emit_spirv_context_get_set;
pub mod emit_spirv_convert;
pub mod emit_spirv_discard_frag;
pub mod emit_spirv_floating_point;
pub mod emit_spirv_image;
pub mod emit_spirv_instructions;
pub mod emit_spirv_integer;
pub mod emit_spirv_logical;
pub mod emit_spirv_quad_rect;
pub mod emit_spirv_select;
pub mod emit_spirv_shared_memory;
pub mod emit_spirv_special;
pub mod emit_spirv_undefined;
pub mod emit_spirv_warp;
