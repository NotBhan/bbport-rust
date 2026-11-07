//! The devtools GPU-memory viewer is not part of the port, ported from
//! `legacy/gpu/shim/core/devtools/widget/gpu_memory.h`.

#[derive(Debug, Clone, Default)]
pub struct GpuMemoryRow {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GpuMemoryGroup {
    pub name: String,
    pub rows: Vec<GpuMemoryRow>,
}

pub struct GpuMemoryViewer;

impl GpuMemoryViewer {
    pub const fn is_enabled() -> bool {
        false
    }

    pub fn publish(_group: GpuMemoryGroup) {}
}
