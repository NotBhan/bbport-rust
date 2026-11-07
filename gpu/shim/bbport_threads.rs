//! Thread sizing helpers, ported from `legacy/gpu/shim/bbport_threads.h`.

/// Hardware threads available to the process (its affinity mask).
pub fn available() -> u32 {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_ZERO(&mut set);
            if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) == 0 {
                return (libc::CPU_COUNT(&set) as u32).max(1);
            }
        }
    }
    std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1)
        .max(1)
}

/// The calling thread only runs on otherwise idle cores (lowest priority fallback).
pub fn make_background() {
    #[cfg(target_os = "linux")]
    unsafe {
        let param: libc::sched_param = std::mem::zeroed();
        if libc::sched_setscheduler(0, libc::SCHED_IDLE, &param) != 0 {
            libc::setpriority(libc::PRIO_PROCESS, 0, 19);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_one_thread_is_available() {
        assert!(available() >= 1);
    }
}
