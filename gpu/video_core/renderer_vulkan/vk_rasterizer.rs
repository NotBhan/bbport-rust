//! Rasterizer command surface, ported from
//! `shadps4/video_core/renderer_vulkan/vk_rasterizer.h`.

/// The rasterizer state; draw recording is supplied by the renderer backend.
#[derive(Debug, Default)]
pub struct Rasterizer {
    pub num_draws: u64,
    pub num_dispatches: u64,
    pub num_copies: u64,
    pub pipeline_dirty: bool,
}

impl Rasterizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw(&mut self) {
        self.num_draws += 1;
    }

    pub fn dispatch(&mut self) {
        self.num_dispatches += 1;
    }

    pub fn copy(&mut self) {
        self.num_copies += 1;
    }

    pub fn mark_pipeline_dirty(&mut self) {
        self.pipeline_dirty = true;
    }

    pub fn reset_pipeline_dirty(&mut self) {
        self.pipeline_dirty = false;
    }

    pub fn totals(&self) -> (u64, u64, u64) {
        (self.num_draws, self.num_dispatches, self.num_copies)
    }
}

/// The command-recording backend the rasterizer forwards to.
pub trait RasterizerBackend {
    fn draw(&mut self, vertex_count: u32, instance_count: u32);
    fn dispatch(&mut self, x: u32, y: u32, z: u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Backend(u32);
    impl RasterizerBackend for Backend {
        fn draw(&mut self, _: u32, _: u32) {
            self.0 += 1;
        }
        fn dispatch(&mut self, _: u32, _: u32, _: u32) {
            self.0 += 1;
        }
    }

    #[test]
    fn rasterizer_counts_commands() {
        let mut rasterizer = Rasterizer::new();
        rasterizer.draw();
        rasterizer.dispatch();
        rasterizer.copy();
        assert_eq!(rasterizer.totals(), (1, 1, 1));
        rasterizer.mark_pipeline_dirty();
        assert!(rasterizer.pipeline_dirty);
        rasterizer.reset_pipeline_dirty();
        assert!(!rasterizer.pipeline_dirty);
        let mut backend = Backend(0);
        backend.draw(3, 1);
        backend.dispatch(1, 1, 1);
        assert_eq!(backend.0, 2);
    }
}

/// The `vk_rasterizer.cpp` draw-record state.
#[derive(Debug, Default)]
pub struct RasterizerDrawState {
    pub bound_pipeline: u64,
    pub descriptor_writes: u32,
    pub draw_calls: u64,
    pub vertices: u64,
    pub instances: u64,
    pub flushes: u64,
}

impl RasterizerDrawState {
    pub fn bind_pipeline(&mut self, pipeline: u64) {
        self.bound_pipeline = pipeline;
        self.descriptor_writes += 1;
    }

    pub fn draw(&mut self, vertex_count: u32, instance_count: u32) -> bool {
        if self.bound_pipeline == 0 {
            return false;
        }
        self.draw_calls += 1;
        self.vertices += u64::from(vertex_count);
        self.instances += u64::from(instance_count);
        true
    }

    pub fn flush(&mut self) {
        self.flushes += 1;
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn rasterizer_draw_state_requires_a_pipeline() {
        let mut state = RasterizerDrawState::default();
        assert!(!state.draw(3, 1));
        state.bind_pipeline(7);
        assert!(state.draw(3, 2));
        assert_eq!((state.vertices, state.instances), (3, 2));
        state.flush();
        assert_eq!(state.flushes, 1);
    }
}
