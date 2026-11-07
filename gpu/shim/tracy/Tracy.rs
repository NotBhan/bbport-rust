//! Profiler-disabled Tracy primitives, ported from `legacy/gpu/shim/tracy/Tracy.hpp`.

#[derive(Debug, Clone, Copy)]
pub struct SourceLocationData {
    pub name: *const std::ffi::c_char,
    pub function: *const std::ffi::c_char,
    pub file: *const std::ffi::c_char,
    pub line: u32,
    pub color: u32,
}

pub mod color {
    pub const RED: u32 = 0xff0000;
    pub const HOT_PINK: u32 = 0xff69b4;
    pub const DARK_ORANGE: u32 = 0xff8c00;
}

pub struct LockableCtx;

impl LockableCtx {
    pub fn new(_location: *const SourceLocationData) -> Self {
        Self
    }
    pub fn before_lock(&self) {}
    pub fn after_lock(&self) {}
    pub fn after_unlock(&self) {}
    pub fn after_try_lock(&self, _locked: bool) {}
    pub fn mark(&self, _location: *const SourceLocationData) {}
    pub fn custom_name(&self, _name: &str) {}
}

pub struct Profiler;

impl Profiler {
    pub const fn is_connected(&self) -> bool {
        false
    }
}

pub fn get_profiler() -> &'static Profiler {
    static PROFILER: Profiler = Profiler;
    &PROFILER
}
