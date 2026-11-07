//! The emitter interface, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_instructions.h`.
//!
//! In C++ this header declares every `Emit*` function so the dispatcher can call them by
//! template. In Rust the emitters are free functions already, so this module is the same
//! interface as a set of re-exports.

pub use super::emit_spirv_atomic::*;
pub use super::emit_spirv_barriers::*;
pub use super::emit_spirv_bitwise_conversion::*;
pub use super::emit_spirv_composite::*;
pub use super::emit_spirv_context_get_set::*;
pub use super::emit_spirv_convert::*;
pub use super::emit_spirv_floating_point::*;
pub use super::emit_spirv_image::*;
pub use super::emit_spirv_integer::*;
pub use super::emit_spirv_logical::*;
pub use super::emit_spirv_select::*;
pub use super::emit_spirv_shared_memory::*;
pub use super::emit_spirv_special::*;
pub use super::emit_spirv_undefined::*;
pub use super::emit_spirv_warp::*;
