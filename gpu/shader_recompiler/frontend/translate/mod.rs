//! The GCN translator, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/`.

pub mod data_share;
pub mod export;
pub mod scalar_alu;
pub mod scalar_flow;
pub mod scalar_memory;
pub mod translator;
pub mod vector_alu;
pub mod vector_interpolation;
pub mod vector_memory;
