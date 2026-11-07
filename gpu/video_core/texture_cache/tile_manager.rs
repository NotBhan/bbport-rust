//! Tile/detile dispatch cache, ported from
//! `shadps4/video_core/texture_cache/tile_manager.h`.

use std::collections::HashMap;

use crate::gpu::amdgpu::tiling::TileMode;

/// `TileManager::TilingKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilingKey {
    pub tile_mode: TileMode,
    pub num_bits: u32,
    pub num_samples: u32,
    pub is_tiler: bool,
}

/// `TileManager`.
#[derive(Debug, Default)]
pub struct TileManager {
    pub tiling_pipelines: HashMap<TilingKey, u64>,
}

impl TileManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tile_image(
        &mut self,
        key: TilingKey,
        dim_x: u32,
        recorder: &mut dyn TileRecorder,
    ) -> u64 {
        let pipeline = self.get_tiling_pipeline(key);
        recorder.record(pipeline, dim_x);
        pipeline
    }

    pub fn detile_image(
        &mut self,
        key: TilingKey,
        in_offset: u64,
        recorder: &mut dyn TileRecorder,
    ) -> (u64, u64) {
        let pipeline = self.get_tiling_pipeline(key);
        recorder.record(pipeline, 1);
        (pipeline, in_offset)
    }

    pub fn get_tiling_pipeline(&mut self, key: TilingKey) -> u64 {
        let next = self.tiling_pipelines.len() as u64 + 1;
        *self.tiling_pipelines.entry(key).or_insert(next)
    }
}

pub trait TileRecorder {
    fn record(&mut self, pipeline: u64, dim_x: u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(u32);

    impl TileRecorder for Recorder {
        fn record(&mut self, _: u64, _: u32) {
            self.0 += 1;
        }
    }

    #[test]
    fn tiling_pipelines_are_cached_by_key() {
        let mut manager = TileManager::new();
        let key = TilingKey {
            tile_mode: TileMode::DisplayLinearAligned,
            num_bits: 32,
            num_samples: 1,
            is_tiler: true,
        };
        let a = manager.get_tiling_pipeline(key);
        let b = manager.get_tiling_pipeline(key);
        assert_eq!(a, b);
        let other = TilingKey {
            num_samples: 4,
            ..key
        };
        assert_ne!(a, manager.get_tiling_pipeline(other));
        let mut recorder = Recorder::default();
        manager.tile_image(key, 16, &mut recorder);
        assert_eq!(recorder.0, 1);
    }
}
