//! Pipeline serialization registration, ported from
//! `shadps4/video_core/renderer_vulkan/vk_pipeline_serialization.h`.

use crate::gpu::video_core::renderer_vulkan::vk_compute_pipeline::{
    ComputePipelineKey, SerializationSupport,
};

/// Opaque archive.
#[repr(C)]
pub struct Archive {
    pub _opaque: [u8; 0],
}

/// The registration surface used by the pipeline cache.
pub trait PipelineSerialization {
    fn register_compute_pipeline_data(
        &mut self,
        key: ComputePipelineKey,
        sdata: &mut SerializationSupport,
    );
    fn register_graphics_pipeline_data(&mut self, key: u64, hash: u64, sdata: &mut [u8]);
    fn register_shader_meta(&mut self, info_hash: u64, perm_hash: usize, perm_idx: usize);
    fn register_shader_binary(&mut self, spv: Vec<u32>, pgm_hash: u64, perm_idx: usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Recorder {
        binaries: usize,
        metas: usize,
    }

    impl PipelineSerialization for Recorder {
        fn register_compute_pipeline_data(
            &mut self,
            _key: ComputePipelineKey,
            _sdata: &mut SerializationSupport,
        ) {
        }
        fn register_graphics_pipeline_data(&mut self, _key: u64, _hash: u64, _sdata: &mut [u8]) {}
        fn register_shader_meta(&mut self, _info_hash: u64, _perm_hash: usize, _perm_idx: usize) {
            self.metas += 1;
        }
        fn register_shader_binary(&mut self, _spv: Vec<u32>, _pgm_hash: u64, _perm_idx: usize) {
            self.binaries += 1;
        }
    }

    #[test]
    fn registrations_reach_the_archive() {
        let mut recorder = Recorder {
            binaries: 0,
            metas: 0,
        };
        recorder.register_shader_meta(1, 2, 3);
        recorder.register_shader_binary(vec![1, 2], 3, 4);
        assert_eq!((recorder.metas, recorder.binaries), (1, 1));
    }
}

use crate::gpu::video_core::cache_storage::{BlobType, DataBase};
use crate::gpu::video_core::renderer_vulkan::vk_graphics_pipeline::{
    GraphicsPipelineKey, SerializationSupport as GraphicsSerializationSupport,
};

pub const SHADER_BINARY_VERSION: u32 = 7;
pub const SHADER_META_VERSION: u32 = 8;
pub const PIPELINE_KEY_VERSION: u32 = 5;

/// A little-endian archive used by the cpp's `Serialization::Archive`.
#[derive(Debug, Default, Clone)]
pub struct BinaryArchive {
    pub data: Vec<u8>,
    pub read: usize,
}

impl BinaryArchive {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, read: 0 }
    }

    pub fn write_u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_usize(value.len());
        self.data.extend_from_slice(value);
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.data.get(self.read..self.read + 4)?;
        self.read += 4;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        let bytes = self.data.get(self.read..self.read + 8)?;
        self.read += 8;
        Some(u64::from_le_bytes(bytes.try_into().ok()?))
    }

    pub fn read_usize(&mut self) -> Option<usize> {
        self.read_u64().map(|value| value as usize)
    }

    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        let length = self.read_usize()?;
        let bytes = self.data.get(self.read..self.read + length)?.to_vec();
        self.read += length;
        Some(bytes)
    }
}

/// The store the registration functions write through (`Storage::DataBase`).
pub trait PipelineBlobStore {
    fn is_open(&self) -> bool;
    fn save(&mut self, kind: BlobType, name: &str, data: &[u8]) -> bool;
    fn load(&self, kind: BlobType, name: &str) -> Option<Vec<u8>>;
}

impl PipelineBlobStore for DataBase {
    fn is_open(&self) -> bool {
        self.is_opened()
    }

    fn save(&mut self, kind: BlobType, name: &str, data: &[u8]) -> bool {
        self.save_bytes(kind, name, data).is_ok()
    }

    fn load(&self, kind: BlobType, name: &str) -> Option<Vec<u8>> {
        self.load_bytes(kind, name).ok()
    }
}

fn pipeline_name(prefix: char, hash: u64) -> String {
    format!("{prefix}_{hash:#018x}")
}

pub fn register_compute_pipeline_data(
    store: &mut dyn PipelineBlobStore,
    key: &ComputePipelineKey,
    _sdata: &SerializationSupport,
) -> bool {
    if !store.is_open() {
        return false;
    }
    let mut archive = BinaryArchive::default();
    archive.write_u32(PIPELINE_KEY_VERSION);
    archive.write_u32(1);
    archive.write_usize(key.value);
    store.save(
        BlobType::PipelineKey,
        &pipeline_name('c', key.value as u64),
        &archive.data,
    )
}

pub fn register_graphics_pipeline_data(
    store: &mut dyn PipelineBlobStore,
    key: &GraphicsPipelineKey,
    hash: u64,
    _sdata: &GraphicsSerializationSupport,
) -> bool {
    if !store.is_open() {
        return false;
    }
    let mut archive = BinaryArchive::default();
    archive.write_u32(PIPELINE_KEY_VERSION);
    archive.write_u32(0);
    for value in key.stage_hashes {
        archive.write_usize(value);
    }
    archive.write_u32(key.num_color_attachments);
    archive.write_u32(key.motion_vectors);
    store.save(
        BlobType::PipelineKey,
        &pipeline_name('g', hash),
        &archive.data,
    )
}

pub fn register_shader_meta(
    store: &mut dyn PipelineBlobStore,
    perm_hash: usize,
    perm_idx: usize,
    spec_bytes: &[u8],
    info_bytes: &[u8],
) -> bool {
    if !store.is_open() {
        return false;
    }
    let mut archive = BinaryArchive::default();
    archive.write_u32(SHADER_META_VERSION);
    archive.write_u32(SHADER_BINARY_VERSION);
    archive.write_usize(perm_hash);
    archive.write_usize(perm_idx);
    archive.write_bytes(spec_bytes);
    archive.write_bytes(info_bytes);
    store.save(
        BlobType::ShaderMeta,
        &format!("{perm_hash:#018x}"),
        &archive.data,
    )
}

pub fn register_shader_binary(
    store: &mut dyn PipelineBlobStore,
    spv: &[u32],
    pgm_hash: u64,
    perm_idx: usize,
) -> bool {
    if !store.is_open() {
        return false;
    }
    let bytes: Vec<u8> = spv.iter().flat_map(|word| word.to_le_bytes()).collect();
    store.save(
        BlobType::ShaderBinary,
        &format!("{pgm_hash:#018x}_{perm_idx}"),
        &bytes,
    )
}

/// `LoadShaderMeta`; returns the perm index and whether the stored SPIR-V is current.
pub fn load_shader_meta(archive: &mut BinaryArchive, motion_vertex: bool) -> Option<(usize, bool)> {
    let meta_version = archive.read_u32()?;
    if meta_version != SHADER_META_VERSION {
        return None;
    }
    let binary_version = archive.read_u32()?;
    let mut spirv_current = binary_version == SHADER_BINARY_VERSION;
    let _perm_hash = archive.read_usize()?;
    let perm_idx = archive.read_usize()?;
    let _spec = archive.read_bytes()?;
    let _info = archive.read_bytes()?;
    if motion_vertex {
        if !spirv_current {
            return None;
        }
        spirv_current = false;
    }
    Some((perm_idx, spirv_current))
}

/// `PipelineCache::LoadGraphicsPipeline`; motion pipelines are never preloaded.
pub fn load_graphics_pipeline(store: &dyn PipelineBlobStore, key: &GraphicsPipelineKey) -> bool {
    if key.motion_vectors != 0 {
        return false;
    }
    for hash in key.stage_hashes {
        if hash == 0 {
            continue;
        }
        let Some(blob) = store.load(BlobType::ShaderMeta, &format!("{hash:#018x}")) else {
            return false;
        };
        let mut archive = BinaryArchive::new(blob);
        if load_shader_meta(&mut archive, false).is_none() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod cpp_tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Store {
        open: bool,
        blobs: HashMap<(BlobType, String), Vec<u8>>,
    }

    impl PipelineBlobStore for Store {
        fn is_open(&self) -> bool {
            self.open
        }
        fn save(&mut self, kind: BlobType, name: &str, data: &[u8]) -> bool {
            self.blobs.insert((kind, name.to_string()), data.to_vec());
            true
        }
        fn load(&self, kind: BlobType, name: &str) -> Option<Vec<u8>> {
            self.blobs.get(&(kind, name.to_string())).cloned()
        }
    }

    #[test]
    fn pipeline_keys_and_shader_meta_round_trip() {
        let mut store = Store::default();
        let key = ComputePipelineKey { value: 7 };
        assert!(!register_compute_pipeline_data(
            &mut store,
            &key,
            &SerializationSupport::default()
        ));
        store.open = true;
        assert!(register_compute_pipeline_data(
            &mut store,
            &key,
            &SerializationSupport::default()
        ));
        assert!(register_shader_binary(&mut store, &[1, 2], 0xab, 3));
        assert!(register_shader_meta(&mut store, 9, 2, &[1], &[2]));
        let blob = store
            .load(BlobType::PipelineKey, "c_0x0000000000000007")
            .unwrap();
        assert_eq!(&blob[..4], &PIPELINE_KEY_VERSION.to_le_bytes());
        let meta = store
            .load(BlobType::ShaderMeta, "0x0000000000000009")
            .unwrap();
        let mut archive = BinaryArchive::new(meta);
        assert_eq!(load_shader_meta(&mut archive, false), Some((2, true)));
        let graphics = GraphicsPipelineKey {
            motion_vectors: 1,
            ..GraphicsPipelineKey::default()
        };
        assert!(!load_graphics_pipeline(&store, &graphics));
        assert_eq!(SHADER_META_VERSION, 8);
    }
}
