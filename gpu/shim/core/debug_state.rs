//! Frame/register dump tooling, ported from `legacy/gpu/shim/core/debug_state.h`.
//! The port keeps the surface as no-ops; shadPS4's dump UI is not part of it.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QueueType {
    Dcb = 0,
    Ccb = 1,
    Acb = 2,
}

#[derive(Debug, Clone)]
pub struct QueueDump {
    pub kind: QueueType,
    pub submit_num: u32,
    pub num2: u32,
    pub data: Vec<u32>,
    pub base_addr: usize,
}

#[derive(Default)]
pub struct DebugStateImpl {
    pub is_using_fsr: bool,
}

impl DebugStateImpl {
    pub fn inc_flip_frame_num(&mut self) {}
    pub fn inc_draw_call(&self) {}
    pub fn inc_dispatch(&self) {}
    pub fn inc_gnm_frame_num(&mut self) {}
    pub const fn get_frame_num(&self) -> u32 {
        0
    }
    pub const fn dumping_current_frame(&self) -> bool {
        false
    }
    pub const fn dumping_current_reg(&mut self) -> bool {
        false
    }
    pub const fn should_pause_in_submit(&self) -> bool {
        false
    }
    pub fn push_queue_dump(&mut self, _dump: QueueDump) {}
    pub fn push_regs_dump(&mut self, _address: usize, _size: usize, _regs: &[u32]) {}
    pub fn show_debug_message(&mut self, _message: String) {}
    pub const fn is_guest_threads_paused(&self) -> bool {
        false
    }
    pub fn pause_guest_threads(&mut self) {}
    pub fn resume_guest_threads(&mut self) {}
}

/// The single `DebugState` instance (the C++ build has a plain global).
pub fn debug_state() -> &'static std::sync::Mutex<DebugStateImpl> {
    static STATE: std::sync::OnceLock<std::sync::Mutex<DebugStateImpl>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(|| std::sync::Mutex::new(DebugStateImpl::default()))
}
