//! `shadps4/shader_recompiler/frontend` ports: the GCN decoder and its control flow graph.

pub mod control_flow_graph;
pub mod copy_shader;
pub mod decode;
pub mod fetch_shader;
pub mod format_gen;
pub mod instruction;
pub mod opcodes;
pub mod opcodes_gen;
pub mod structured_control_flow;
pub mod tessellation;
pub mod translate;
