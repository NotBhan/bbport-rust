//! Process memory usage, ported from `legacy/gpu/shim/bbport_process_memory.h`.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Committed memory of the process (Linux: its virtual size).
    pub private_bytes: u64,
    /// Bytes in RAM now.
    pub working_set: u64,
}

/// Read this process's usage. `None` when the host cannot report it.
pub fn get() -> Option<Usage> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
        let mut fields = statm.split_whitespace();
        let virtual_pages: u64 = fields.next()?.parse().ok()?;
        let resident_pages: u64 = fields.next()?.parse().ok()?;
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return None;
        }
        let page_size = page_size as u64;
        Some(Usage {
            private_bytes: virtual_pages * page_size,
            working_set: resident_pages * page_size,
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn usage_is_positive_and_working_set_fits_virtual_size() {
        let usage = get().expect("Linux exposes /proc/self/statm");
        assert!(usage.private_bytes > 0);
        assert!(usage.working_set > 0);
        assert!(usage.working_set <= usage.private_bytes);
    }
}
