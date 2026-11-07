//! GNM driver entry points, ported from
//! `shadps4/core/libraries/gnmdriver/gnmdriver.h`.

/// The driver surface; the HLE loader implements it.
pub trait GnmDriverApi {
    fn are_submits_allowed(&mut self) -> i32;
    fn begin_workload(&mut self, workload_stream: u32, workload: &mut u64) -> i32;
    fn create_workload_stream(&mut self, param: u64, workload_stream: &mut u32) -> i32;
    fn destroy_workload_stream(&mut self) -> i32;
    fn ding_dong(&mut self, gnm_vqid: u32, next_offs_dw: u32);
    fn ding_dong_for_workload(&mut self, gnm_vqid: u32, next_offs_dw: u32, workload_id: u64);
    fn dispatch_direct(&mut self, cmdbuf: &mut [u32], threads: (u32, u32, u32), flags: u32) -> i32;
    fn dispatch_indirect(&mut self, cmdbuf: &mut [u32], data_offset: u32, flags: u32) -> i32;
    fn draw_index(
        &mut self,
        cmdbuf: &mut [u32],
        index_count: u32,
        index_addr: u64,
        flags: u32,
        kind: u32,
    ) -> i32;
    fn draw_index_auto(&mut self, cmdbuf: &mut [u32], index_count: u32, flags: u32) -> i32;
    fn draw_indirect(
        &mut self,
        cmdbuf: &mut [u32],
        data_offset: u32,
        shader_stage: u32,
        vertex_sgpr_offset: u32,
        instance_sgpr_offset: u32,
        flags: u32,
    ) -> i32;
    fn flush_garlic(&mut self);
    fn submit(&mut self, cmdbuf: &[u32]) -> i32;
    fn add_eq_event(&mut self, eq: i32, id: u64, udata: *mut core::ffi::c_void) -> i32;
    fn delete_eq_event(&mut self, eq: i32, id: u64) -> i32;
}

/// `sceGnmDriverInternal...` interface ids.
pub const GNM_INTERFACE_DEBUGGER: u32 = 1;
pub const GNM_INTERFACE_GPU_EXCEPTION: u32 = 2;
pub const GNM_INTERFACE_HDR_SCOPES: u32 = 3;
pub const GNM_INTERFACE_REPLAY: u32 = 4;
pub const GNM_INTERFACE_RESOURCE_REGISTRATION: u32 = 5;
pub const GNM_INTERFACE_VALIDATION: u32 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_ids_are_distinct() {
        let ids = [
            GNM_INTERFACE_DEBUGGER,
            GNM_INTERFACE_GPU_EXCEPTION,
            GNM_INTERFACE_HDR_SCOPES,
            GNM_INTERFACE_REPLAY,
            GNM_INTERFACE_RESOURCE_REGISTRATION,
            GNM_INTERFACE_VALIDATION,
        ];
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
        assert_eq!(GNM_INTERFACE_DEBUGGER, 1);
    }
}

/// The GNM command submission state (`gnmdriver.cpp`).
#[derive(Debug, Default)]
pub struct GnmDriver {
    pub initialized: bool,
    pub submitted: u64,
    pub workload_streams: std::collections::HashSet<u32>,
    pub next_workload: u64,
    pub eq_events: Vec<(i32, u64, usize)>,
    pub equeues: std::collections::HashMap<i32, Vec<u64>>,
}

impl GnmDriver {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        0
    }

    pub fn submit(&mut self) -> i32 {
        if !self.initialized {
            return -1;
        }
        self.submitted += 1;
        0
    }

    pub fn create_workload_stream(&mut self, id: u32) -> bool {
        self.workload_streams.insert(id)
    }

    pub fn begin_workload(&mut self) -> u64 {
        self.next_workload += 1;
        self.next_workload
    }

    pub fn add_eq_event(&mut self, eq: i32, id: u64, udata: usize) -> i32 {
        self.events_entry(eq).push(id);
        self.eq_events.push((eq, id, udata));
        0
    }

    pub fn delete_eq_event(&mut self, eq: i32, id: u64) -> i32 {
        let before = self.eq_events.len();
        self.eq_events
            .retain(|(queue, event, _)| *queue != eq || *event != id);
        if self.eq_events.len() != before {
            0
        } else {
            -1
        }
    }

    fn events_entry(&mut self, eq: i32) -> &mut Vec<u64> {
        self.equeues.entry(eq).or_default()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn gnm_driver_submits_workloads_and_events() {
        let mut driver = GnmDriver::default();
        assert_eq!(driver.submit(), -1);
        assert_eq!(driver.initialize(), 0);
        assert_eq!(driver.submit(), 0);
        assert!(driver.create_workload_stream(1));
        assert_eq!(driver.begin_workload(), 1);
        assert_eq!(driver.add_eq_event(1, 2, 0), 0);
        assert_eq!(driver.delete_eq_event(1, 2), 0);
        assert_eq!(driver.delete_eq_event(1, 2), -1);
    }
}
