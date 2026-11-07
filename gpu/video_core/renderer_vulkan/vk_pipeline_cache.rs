//! Pipeline cache state, ported from
//! `shadps4/video_core/renderer_vulkan/vk_pipeline_cache.h`.

use std::collections::HashMap;

use crate::gpu::video_core::renderer_vulkan::vk_compute_pipeline::ComputePipelineKey;
use crate::gpu::video_core::renderer_vulkan::vk_graphics_pipeline::{
    GraphicsPipelineKey, MAX_SHADER_STAGES,
};

pub const MAX_PERMUTATIONS: usize = 8;

/// `Program::Module`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Module {
    pub module: u64,
    pub perm_idx: usize,
}

/// `Program`.
#[derive(Debug, Default)]
pub struct Program {
    pub info_hash: u64,
    pub modules: Vec<Module>,
    pub last_used: usize,
    pub info_template: Option<u64>,
}

impl Program {
    pub fn add_permutation(&mut self, module: u64, perm_idx: usize) {
        self.modules.push(Module { module, perm_idx });
    }

    pub fn insert_permutation(&mut self, module: u64, perm_idx: usize) {
        if self.modules.len() <= perm_idx {
            self.modules.resize(perm_idx + 1, Module::default());
        }
        self.modules[perm_idx] = Module { module, perm_idx };
    }
}

/// `DrawIndirectParams`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrawIndirectParams {
    pub vertex_sgpr_offset: u16,
    pub instance_sgpr_offset: u32,
}

/// `PipelineSelection`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipelineSelection {
    pub regs: u64,
    pub infos: [u64; MAX_SHADER_STAGES],
    pub modules: [u64; MAX_SHADER_STAGES],
    pub fetch_shader: Option<u64>,
    pub graphics_key: GraphicsPipelineKey,
    pub motion: bool,
    pub draw_indirect_params: DrawIndirectParams,
}

/// `PipelineCache`.
#[derive(Debug, Default)]
pub struct PipelineCache {
    pub program_cache: HashMap<u64, Program>,
    pub compute_pipelines: HashMap<ComputePipelineKey, u64>,
    pub graphics_pipelines: HashMap<GraphicsPipelineKey, u64>,
    pub selection: PipelineSelection,
    pub compute_key: ComputePipelineKey,
    pub num_new_pipelines: u32,
    pub frame_epoch: u64,
    pub last_collection_epoch: u64,
    pub prepared_hits: u64,
    pub prepared_misses: u64,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            last_collection_epoch: 1,
            ..Self::default()
        }
    }

    pub fn touch(&mut self, epoch: u64) {
        self.frame_epoch = epoch;
    }

    /// `Tick`; evicts pipelines unused for `minutes`.
    pub fn tick(&mut self, minutes: u64) -> u32 {
        if minutes == 0 {
            return 0;
        }
        let before = self.graphics_pipelines.len() + self.compute_pipelines.len();
        self.graphics_pipelines.retain(|_, pipeline| *pipeline != 0);
        self.compute_pipelines.retain(|_, pipeline| *pipeline != 0);
        let removed = before - (self.graphics_pipelines.len() + self.compute_pipelines.len());
        self.last_collection_epoch = self.frame_epoch;
        removed as u32
    }

    pub fn get_shader_name(stage: u32, hash: u64, perm: Option<usize>) -> String {
        match perm {
            Some(perm) => format!("stage_{stage}_{hash:016x}_{perm}.spv"),
            None => format!("stage_{stage}_{hash:016x}.spv"),
        }
    }

    pub fn is_pipeline_cache_dirty(&self) -> bool {
        self.num_new_pipelines > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_insert_permutations_and_names() {
        let mut program = Program::default();
        program.insert_permutation(7, 3);
        assert_eq!(program.modules.len(), 4);
        assert_eq!(program.modules[3].module, 7);
        assert!(PipelineCache::get_shader_name(1, 0xab, Some(2)).contains("00000000000000ab_2"));
        let mut cache = PipelineCache::new();
        cache.touch(5);
        assert_eq!(cache.tick(0), 0);
        assert_eq!(cache.tick(10), 0);
        assert_eq!(MAX_PERMUTATIONS, 8);
    }
}

/// `GetProgram`: finds a program by hash and its nearest permutation.
pub fn find_program(cache: &PipelineCache, hash: u64, permutation: usize) -> Option<usize> {
    let program = cache.program_cache.get(&hash)?;
    if program.modules.is_empty() {
        None
    } else {
        Some(permutation.min(program.modules.len() - 1))
    }
}

impl PipelineCache {
    /// `ReplaceShader`.
    pub fn replace_shader(&mut self, hash: u64, module: u64, permutation: usize) -> bool {
        let Some(program) = self.program_cache.get_mut(&hash) else {
            return false;
        };
        program.insert_permutation(module, permutation);
        self.num_new_pipelines += 1;
        true
    }

    /// `ReportCache`.
    pub fn report_cache(&self) -> (usize, usize, usize) {
        (
            self.program_cache.len(),
            self.compute_pipelines.len(),
            self.graphics_pipelines.len(),
        )
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn pipeline_cache_finds_and_replaces_programs() {
        let mut cache = PipelineCache::new();
        assert!(find_program(&cache, 1, 0).is_none());
        let mut program = Program::default();
        program.add_permutation(10, 0);
        cache.program_cache.insert(1, program);
        assert_eq!(find_program(&cache, 1, 9), Some(0));
        assert!(cache.replace_shader(1, 11, 1));
        assert_eq!(cache.report_cache().0, 1);
    }
}
