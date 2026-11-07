//! VideoOut port state, ported from `shadps4/core/libraries/videoout/driver.h`.

use crate::gpu::core::libraries::videoout::buffer::*;
use crate::gpu::core::libraries::videoout::video_out::*;

/// `VideoOutPort`.
#[derive(Debug)]
pub struct VideoOutPort {
    pub resolution: SceVideoOutResolutionStatus,
    pub buffer_slots: [VideoOutBuffer; MAX_DISPLAY_BUFFERS],
    pub buffer_labels: [u64; MAX_DISPLAY_BUFFERS],
    pub groups: [BufferAttributeGroup; MAX_DISPLAY_BUFFER_GROUPS],
    pub flip_status: FlipStatus,
    pub vblank_status: SceVideoOutVblankStatus,
    pub flip_events: Vec<i64>,
    pub vblank_events: Vec<i64>,
    pub flip_rate: i32,
    pub prev_index: i32,
    pub is_open: bool,
    pub is_hdr: bool,
}

impl Default for VideoOutPort {
    fn default() -> Self {
        Self {
            resolution: SceVideoOutResolutionStatus::default(),
            buffer_slots: std::array::from_fn(|_| VideoOutBuffer {
                group_index: -1,
                address_left: 0,
                address_right: 0,
            }),
            buffer_labels: [0; MAX_DISPLAY_BUFFERS],
            groups: std::array::from_fn(|_| BufferAttributeGroup {
                is_occupied: false,
                attrib: BufferAttribute {
                    pixel_format: PixelFormat::Unknown,
                    tiling_mode: TilingMode::Tile,
                    aspect_ratio: 0,
                    width: 0,
                    height: 0,
                    pitch_in_pixel: 0,
                    option: 0,
                    reserved0: 0,
                    reserved1: 0,
                },
            }),
            flip_status: FlipStatus::default(),
            vblank_status: SceVideoOutVblankStatus::default(),
            flip_events: Vec::new(),
            vblank_events: Vec::new(),
            flip_rate: 0,
            prev_index: -1,
            is_open: false,
            is_hdr: false,
        }
    }
}

impl VideoOutPort {
    pub fn new() -> Self {
        Self::default()
    }

    /// `FindFreeGroup`.
    pub fn find_free_group(&self) -> i32 {
        let mut index = 0;
        while index < self.groups.len() && self.groups[index].is_occupied {
            index += 1;
        }
        index as i32
    }

    /// `IsVoLabel`.
    pub fn is_vo_label(&self, address: *const u64) -> bool {
        let start = self.buffer_labels.as_ptr() as usize;
        let end = start + MAX_DISPLAY_BUFFERS * std::mem::size_of::<u64>() - 1;
        let address = address as usize;
        address >= start && address <= end
    }

    /// `NumRegisteredBuffers`.
    pub fn num_registered_buffers(&self) -> i32 {
        self.buffer_slots
            .iter()
            .filter(|buffer| buffer.group_index != -1)
            .count() as i32
    }
}

/// `ServiceThreadParams`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServiceThreadParams {
    pub unknown: u32,
    pub set_priority: bool,
    pub priority: u32,
    pub set_affinity: bool,
    pub affinity: u64,
}

/// The `VideoOutDriver` methods; the rasterizer side implements them.
pub trait VideoOutDriverApi {
    fn open(&mut self, params: Option<&ServiceThreadParams>) -> i32;
    fn close(&mut self, handle: i32);
    fn register_buffers(
        &mut self,
        port: &mut VideoOutPort,
        start_index: i32,
        addresses: &[*mut core::ffi::c_void],
        attribute: &BufferAttribute,
    ) -> i32;
    fn unregister_buffers(&mut self, port: &mut VideoOutPort, attribute_index: i32) -> i32;
    fn change_buffer_attribute(
        &mut self,
        port: &mut VideoOutPort,
        buffer_index: i32,
        attribute: &BufferAttribute,
    ) -> i32;
    fn submit_flip(
        &mut self,
        port: &mut VideoOutPort,
        index: i32,
        flip_arg: i64,
        eop: bool,
    ) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_port_has_no_buffers_or_groups() {
        let mut port = VideoOutPort::new();
        assert_eq!(port.num_registered_buffers(), 0);
        assert_eq!(port.find_free_group(), 0);
        assert_eq!(port.buffer_slots[0].group_index, -1);
        port.groups[0].is_occupied = true;
        assert_eq!(port.find_free_group(), 1);
        port.buffer_slots[0].group_index = 0;
        assert_eq!(port.num_registered_buffers(), 1);

        let start = port.buffer_labels.as_ptr() as usize as *const u64;
        assert!(port.is_vo_label(start));
        assert!(port.is_vo_label(start.wrapping_add(8)));
        assert!(!port.is_vo_label(start.wrapping_add(16 * 8)));
    }

    #[test]
    fn the_service_thread_params_match_the_header() {
        assert_eq!(core::mem::size_of::<ServiceThreadParams>(), 24);
        assert_eq!(core::mem::offset_of!(ServiceThreadParams, affinity), 16);
    }
}

/// The VideoOut driver state (`driver.cpp`).
#[derive(Debug, Default)]
pub struct VideoOutDriver {
    pub open: bool,
    pub next_port: i32,
    pub ports: std::collections::HashMap<i32, (u32, u32, u32)>,
    pub flips: u64,
    pub vblank_events: u64,
}

impl VideoOutDriver {
    pub fn open_port(&mut self, width: u32, height: u32, format: u32) -> i32 {
        self.next_port += 1;
        self.ports.insert(self.next_port, (width, height, format));
        self.open = true;
        self.next_port
    }

    pub fn close_port(&mut self, port: i32) -> i32 {
        if self.ports.remove(&port).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn flip(&mut self, port: i32) -> i32 {
        if !self.ports.contains_key(&port) {
            return -1;
        }
        self.flips += 1;
        0
    }

    pub fn vblank(&mut self) {
        self.vblank_events += 1;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.ports.clear();
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn videoout_driver_opens_ports_and_flips() {
        let mut driver = VideoOutDriver::default();
        assert_eq!(driver.flip(1), -1);
        let port = driver.open_port(1920, 1080, 1);
        assert_eq!(driver.flip(port), 0);
        driver.vblank();
        assert_eq!(driver.flips, 1);
        assert_eq!(driver.close_port(port), 0);
        driver.close();
    }
}
