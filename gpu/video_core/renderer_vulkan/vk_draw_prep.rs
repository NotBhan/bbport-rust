//! Speculative draw-preparation data, ported from
//! `shadps4/video_core/renderer_vulkan/vk_draw_prep.h`.

use std::sync::atomic::AtomicU32;

pub const MAX_SHADER_STAGES: usize = 6;

/// `PreparedStage`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PreparedStage {
    pub program: *const core::ffi::c_void,
    pub hash: u64,
    pub hw_stage: u32,
    pub pgm_base: u64,
    pub flat: *const u32,
    pub flat_size: u32,
    pub image_sharps: *const u8,
    pub image_hashes: *const u64,
    pub sampler_sharps: *const u8,
    pub buffer_sharps: *const u8,
    pub num_images: u32,
    pub num_samplers: u32,
    pub num_buffers: u32,
}

impl Default for PreparedStage {
    fn default() -> Self {
        Self {
            program: std::ptr::null(),
            hash: 0,
            hw_stage: 0,
            pgm_base: 0,
            flat: std::ptr::null(),
            flat_size: 0,
            image_sharps: std::ptr::null(),
            image_hashes: std::ptr::null(),
            sampler_sharps: std::ptr::null(),
            buffer_sharps: std::ptr::null(),
            num_images: 0,
            num_samplers: 0,
            num_buffers: 0,
        }
    }
}

/// `ImageDescHash`; the caller supplies the sharp's first four words.
pub fn image_desc_hash(
    sharp_words: [u64; 4],
    is_written: bool,
    is_depth: bool,
    is_array: bool,
) -> u64 {
    let flags = u32::from(is_written) | u32::from(is_depth) << 1 | u32::from(is_array) << 2;
    let mut hash = u64::from(flags).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for word in sharp_words {
        hash = (hash ^ word).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        hash ^= hash >> 32;
    }
    hash
}

/// `PreparedVertexInputs`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PreparedVertexInputs {
    pub attributes: *const core::ffi::c_void,
    pub bindings: *const core::ffi::c_void,
    pub buffers: *const u8,
    pub range_index: *const u8,
    pub ranges: *const Range,
    pub count: u32,
    pub num_ranges: u32,
    pub buffers_hash: u64,
    pub valid: bool,
}

/// `PreparedVertexInputs::Range`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Range {
    pub base: u64,
    pub end: u64,
}

/// A `GraphicsPipelineKey` analogue: the prepared pipeline's identifying words.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GraphicsPipelineKey {
    pub words: [u64; 4],
}

/// `PreparedDraw`.
#[repr(C)]
pub struct PreparedDraw {
    pub state: AtomicU32,
    pub reg_checksum: u64,
    pub key: GraphicsPipelineKey,
    pub num_stages: u32,
    pub stages: [PreparedStage; MAX_SHADER_STAGES],
    pub vertex: PreparedVertexInputs,
}

impl PreparedDraw {
    pub const PENDING: u32 = 0;
    pub const READY: u32 = 1;
    pub const UNAVAILABLE: u32 = 2;

    pub fn new() -> Self {
        Self {
            state: AtomicU32::new(Self::PENDING),
            reg_checksum: 0,
            key: GraphicsPipelineKey::default(),
            num_stages: 0,
            stages: [PreparedStage::default(); MAX_SHADER_STAGES],
            vertex: PreparedVertexInputs::default(),
        }
    }

    pub fn set_state(&self, state: u32) {
        self.state
            .store(state, std::sync::atomic::Ordering::Release);
    }

    pub fn get_state(&self) -> u32 {
        self.state.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Default for PreparedDraw {
    fn default() -> Self {
        Self::new()
    }
}

/// The preparation workers' control surface.
pub trait DrawPreparation {
    fn start(&mut self);
    fn stop(&mut self);
    fn prepare(&mut self, buffer_index: usize) -> bool;
    fn drain(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_description_hash_is_stable() {
        let words = [1, 2, 3, 4];
        let hash = image_desc_hash(words, true, false, true);
        assert_eq!(hash, image_desc_hash(words, true, false, true));
        assert_ne!(hash, image_desc_hash(words, false, false, false));
        assert_eq!(image_desc_hash([0; 4], false, false, false), 0);
    }

    #[test]
    fn prepared_draws_start_pending() {
        let draw = PreparedDraw::new();
        assert_eq!(draw.get_state(), PreparedDraw::PENDING);
        draw.set_state(PreparedDraw::READY);
        assert_eq!(draw.get_state(), PreparedDraw::READY);
        assert_eq!(draw.num_stages, 0);
        assert!(draw.vertex.attributes.is_null());
        assert_eq!(core::mem::size_of::<Range>(), 16);
    }
}

/// The speculative draw-preparation worker state.
#[derive(Debug, Default)]
pub struct DrawPreparationState {
    pub buffers: usize,
    pub prepared: usize,
    pub available: usize,
    pub next_buffer: usize,
    pub active: bool,
}

impl DrawPreparationState {
    pub fn start(&mut self, buffers: usize) {
        self.buffers = buffers;
        self.active = true;
        self.next_buffer = 0;
    }

    pub fn claim(&mut self) -> Option<usize> {
        if !self.active || self.next_buffer >= self.buffers {
            return None;
        }
        let index = self.next_buffer;
        self.next_buffer += 1;
        Some(index)
    }

    pub fn mark_prepared(&mut self, ready: bool) {
        if ready {
            self.prepared += 1;
        } else {
            self.available += 1;
        }
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.buffers = 0;
        self.next_buffer = 0;
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn draw_prep_workers_claim_buffers_and_track_results() {
        let mut state = DrawPreparationState::default();
        assert!(state.claim().is_none());
        state.start(2);
        assert_eq!(state.claim(), Some(0));
        assert_eq!(state.claim(), Some(1));
        assert!(state.claim().is_none());
        state.mark_prepared(true);
        state.mark_prepared(false);
        assert_eq!((state.prepared, state.available), (1, 1));
        state.stop();
        assert!(!state.active);
    }
}
