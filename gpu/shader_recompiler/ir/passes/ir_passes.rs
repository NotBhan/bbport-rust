//! The IR pass index, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/ir_passes.h`. The C++ header includes every
//! pass declaration in one place; the port keeps each pass in its own module and re-exports the
//! entry points here so a pipeline can pull them from one path.

pub use super::constant_propogation::constant_propagation_pass;
pub use super::dead_code_elimination::dead_code_elimination_pass;
pub use super::flatten_extended_userdata::flatten_extended_userdata_pass;
pub use super::hull_shader_transform::{
    domain_shader_transform, hull_shader_transform, tessellation_preprocess,
};
pub use super::inject_clip_distance_attributes::inject_clip_distance_attributes;
pub use super::interpolated_integer::interpolated_integer_pass;
pub use super::inverse_ballot_elimination::inverse_ballot_elimination_pass;
pub use super::lower_buffer_format_to_raw::lower_buffer_format_to_raw;
pub use super::lower_fp64_to_fp32::lower_fp64_to_fp32;
pub use super::lower_hardware_intrinsics::lower_hardware_intrinsics;
pub use super::lower_phis_to_regs::lower_phis_to_regs_pass;
pub use super::lower_user_clip_planes::lower_user_clip_planes;
pub use super::lower_wave64::lower_wave64_ballot_pass;
pub use super::phi_simplification::phi_simplification_pass;
pub use super::readlane_elimination::readlane_elimination_pass;
pub use super::resource_discover::resource_discover_pass;
pub use super::resource_pass::ResourceDiscoveryList;
pub use super::resource_patching::resource_patching_pass;
pub use super::ring_access_elimination::ring_access_elimination;
pub use super::shader_info_collection::collect_shader_info_pass;
pub use super::shared_memory_barrier::shared_memory_barrier_pass;
pub use super::shared_memory_simplify::shared_memory_simplify_pass;
pub use super::shared_memory_to_storage::shared_memory_to_storage_pass;
pub use super::ssa_repair::ssa_repair_pass;
pub use super::ssa_rewrite::ssa_rewrite_pass;
