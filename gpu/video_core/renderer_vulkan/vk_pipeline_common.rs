//! Pipeline base, ported from
//! `shadps4/video_core/renderer_vulkan/vk_pipeline_common.h`.

pub const ALL_GRAPHICS_STAGE_BITS: u32 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10;
pub const MAX_STAGE_TYPES: usize = 6;

/// `Pipeline`.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub desc_layout: u64,
    pub pipeline_layout: u64,
    pub pipeline: u64,
    pub stages: Vec<u64>,
    pub uses_push_descriptors: bool,
    pub is_compute: bool,
    pub last_used_epoch: u64,
    pub last_used_tick: u64,
}

impl Pipeline {
    pub fn new(pipeline_cache: u64, is_compute: bool) -> Self {
        Self {
            desc_layout: 0,
            pipeline_layout: 0,
            pipeline: pipeline_cache,
            stages: Vec::new(),
            uses_push_descriptors: false,
            is_compute,
            last_used_epoch: 0,
            last_used_tick: 0,
        }
    }

    pub fn handle(&self) -> u64 {
        self.pipeline
    }

    pub fn is_valid(&self) -> bool {
        self.pipeline != 0
    }

    pub fn get_layout(&self) -> u64 {
        self.pipeline_layout
    }

    /// `GetStages`; compute pipelines expose only the last stage slot.
    pub fn get_stages(&self) -> &[u64] {
        if self.is_compute {
            &self.stages[self.stages.len().saturating_sub(1)..]
        } else {
            &self.stages[..self.stages.len().saturating_sub(1)]
        }
    }

    pub fn get_stage(&self, stage: usize) -> Option<&u64> {
        self.stages.get(stage)
    }

    pub fn is_compute(&self) -> bool {
        self.is_compute
    }

    pub fn bind_resources(
        &self,
        writes: &mut Vec<u64>,
        push_data: u64,
        binder: &mut dyn ResourceBinder,
    ) {
        binder.bind(writes, push_data, self.uses_push_descriptors);
    }
}

pub trait ResourceBinder {
    fn bind(&mut self, writes: &mut Vec<u64>, push_data: u64, push_descriptors: bool);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Binder(u32);

    impl ResourceBinder for Binder {
        fn bind(&mut self, writes: &mut Vec<u64>, push_data: u64, _: bool) {
            writes.push(push_data);
            self.0 += 1;
        }
    }

    #[test]
    fn pipelines_expose_the_right_stage_range() {
        let mut graphics = Pipeline::new(1, false);
        graphics.stages = vec![1, 2, 3, 4, 5];
        assert_eq!(graphics.get_stages(), &[1, 2, 3, 4]);
        assert!(graphics.is_valid());
        let mut compute = Pipeline::new(2, true);
        compute.stages = vec![1, 2, 3, 4, 5];
        assert_eq!(compute.get_stages(), &[5]);
        let mut writes = Vec::new();
        let mut binder = Binder(0);
        graphics.bind_resources(&mut writes, 9, &mut binder);
        assert_eq!(writes, vec![9]);
        assert_eq!(ALL_GRAPHICS_STAGE_BITS, 0x1f);
        assert_eq!(MAX_STAGE_TYPES, 6);
    }
}
