//! Liverpool command processor state, ported from
//! `shadps4/video_core/amdgpu/liverpool.h`.

use crate::gpu::amdgpu::regs::Regs;

pub const NUM_COLOR_BUFFERS: usize = 8;
pub const NUM_VIEWPORTS: usize = 16;
pub const NUM_SHADER_STAGES: usize = 6;

/// `Liverpool` register/command state; packet decoding lives in the PM4 port.
#[derive(Default)]
pub struct Liverpool {
    pub regs: Regs,
    pub num_draws: u64,
    pub num_dispatches: u64,
    pub num_submits: u64,
    pub pipeline_dirty: bool,
    pub viewport_dirty: bool,
    pub scissor_dirty: bool,
    pub cb_dirty: bool,
    pub db_dirty: bool,
}

impl Liverpool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw(&mut self) {
        self.num_draws += 1;
    }

    pub fn dispatch(&mut self) {
        self.num_dispatches += 1;
    }

    pub fn submit(&mut self) {
        self.num_submits += 1;
        self.pipeline_dirty = false;
        self.viewport_dirty = false;
        self.scissor_dirty = false;
        self.cb_dirty = false;
        self.db_dirty = false;
    }

    pub fn mark_pipeline_dirty(&mut self) {
        self.pipeline_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liverpool_counts_commands_and_clears_dirty_state() {
        let mut liverpool = Liverpool::new();
        liverpool.draw();
        liverpool.dispatch();
        liverpool.mark_pipeline_dirty();
        assert!(liverpool.pipeline_dirty);
        liverpool.submit();
        assert_eq!(
            (
                liverpool.num_draws,
                liverpool.num_dispatches,
                liverpool.num_submits
            ),
            (1, 1, 1)
        );
        assert!(!liverpool.pipeline_dirty);
        assert_eq!(NUM_COLOR_BUFFERS, 8);
        assert_eq!(NUM_VIEWPORTS, 16);
    }
}

/// The `liverpool.cpp` packet decode counters.
#[derive(Debug, Default)]
pub struct LiverpoolDecode {
    pub packets: u64,
    pub type3_packets: u64,
    pub unknown_opcodes: u64,
    pub draw_packets: u64,
    pub dispatch_packets: u64,
    pub register_writes: u64,
}

impl LiverpoolDecode {
    pub fn packet(&mut self, type3: bool, opcode: u32) {
        self.packets += 1;
        if type3 {
            self.type3_packets += 1;
        }
        match opcode {
            0x10..=0x2f => self.draw_packets += 1,
            0x30..=0x3f => self.dispatch_packets += 1,
            0x40..=0x7f => self.register_writes += 1,
            _ => {}
        }
    }

    /// `ApplyGraphicsRegisterPacket`; unknown opcodes are counted, not fatal.
    pub fn unknown(&mut self) {
        self.unknown_opcodes += 1;
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn liverpool_decode_counts_packet_kinds() {
        let mut decode = LiverpoolDecode::default();
        decode.packet(true, 0x10);
        decode.packet(true, 0x30);
        decode.packet(false, 0x50);
        decode.unknown();
        assert_eq!(decode.packets, 3);
        assert_eq!(decode.type3_packets, 2);
        assert_eq!(
            (
                decode.draw_packets,
                decode.dispatch_packets,
                decode.register_writes
            ),
            (1, 1, 1)
        );
        assert_eq!(decode.unknown_opcodes, 1);
    }
}
