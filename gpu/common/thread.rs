//! Thread naming, priority and precise sleeping, ported from
//! `legacy/gpu/shadps4/common/thread.h` and `thread.cpp`.

use std::time::{Duration, Instant};

/// `Common::ThreadPriority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ThreadPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    VeryHigh = 3,
    Critical = 4,
}

/// `Common::SetCurrentThreadRealtime`; not implemented outside macOS in the original.
pub fn set_current_thread_realtime(_period: Duration) {}

/// `Common::SetCurrentThreadPriority`. Mirrors the `SCHED_OTHER` computation and ignores
/// failures, as the original does.
pub fn set_current_thread_priority(priority: ThreadPriority) {
    // SAFETY: the pthread calls only read scheduling parameters for the current thread.
    unsafe {
        let thread = libc::pthread_self();
        let scheduling_type = libc::SCHED_OTHER;
        let max_prio = libc::sched_get_priority_max(scheduling_type);
        let min_prio = libc::sched_get_priority_min(scheduling_type);
        let level = (priority as u32 + 1).max(4) as i32;

        let mut params: libc::sched_param = std::mem::zeroed();
        params.sched_priority = if max_prio > min_prio {
            min_prio + ((max_prio - min_prio) * level) / 4
        } else {
            min_prio - ((min_prio - max_prio) * level) / 4
        };
        libc::pthread_setschedparam(thread, scheduling_type, &params);
    }
}

/// `Common::SetCurrentThreadName`. Linux rejects names longer than 15 characters with
/// `ERANGE`, so the name is truncated first, matching the original.
pub fn set_current_thread_name(name: &str) {
    let truncated = &name[..name.len().min(15)];
    let mut buffer = [0i8; 16];
    for (dst, src) in buffer.iter_mut().zip(truncated.bytes()) {
        *dst = src as i8;
    }
    // SAFETY: the buffer is NUL-terminated and valid for the call.
    let error = unsafe { libc::pthread_setname_np(libc::pthread_self(), buffer.as_ptr()) };
    if error != 0 {
        eprintln!(
            "Failed to set thread name to '{}': {}",
            truncated,
            std::io::Error::from_raw_os_error(error)
        );
    }
}

/// `Common::SetThreadName`; the POSIX build leaves this unimplemented.
pub fn set_thread_name(_thread: usize, _name: &str) {}

fn duration_to_timespec(duration: Duration) -> libc::timespec {
    libc::timespec {
        tv_sec: duration.as_secs() as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    }
}

fn timespec_to_duration(timespec: &libc::timespec) -> Duration {
    Duration::new(
        timespec.tv_sec.max(0) as u64,
        timespec.tv_nsec.max(0) as u32,
    )
}

/// `Common::AccurateSleep`. Returns whether the sleep completed and the time left over when it
/// was interrupted with `interruptible` set.
pub fn accurate_sleep(duration: Duration, interruptible: bool) -> (bool, Duration) {
    let mut request = duration_to_timespec(duration);
    loop {
        let mut remain: libc::timespec = unsafe { std::mem::zeroed() };
        // SAFETY: both pointers refer to live `timespec` values for the duration of the call.
        let result = unsafe { libc::nanosleep(&request, &mut remain) };
        if result == 0 {
            return (true, Duration::ZERO);
        }
        let error = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        if error == libc::EINTR {
            if interruptible {
                return (false, timespec_to_duration(&remain));
            }
            request = remain;
        } else {
            return (false, timespec_to_duration(&remain));
        }
    }
}

/// `Common::PreciseSleepUntil`: sleeps to within 500 µs of the deadline, then spins.
pub fn precise_sleep_until(deadline: Instant) {
    const SPIN_MARGIN: Duration = Duration::from_micros(500);
    let now = Instant::now();
    let remaining = deadline.saturating_duration_since(now);
    if remaining > SPIN_MARGIN {
        std::thread::sleep(remaining - SPIN_MARGIN);
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
    }
}

/// `Common::AccurateTimer`.
#[derive(Debug, Clone)]
pub struct AccurateTimer {
    target_interval: Duration,
    total_wait: Duration,
    start_time: Instant,
}

impl AccurateTimer {
    /// `AccurateTimer(target_interval)`.
    pub fn new(target_interval: Duration) -> Self {
        Self {
            target_interval,
            total_wait: Duration::ZERO,
            start_time: Instant::now(),
        }
    }

    /// `Start`.
    pub fn start(&mut self) {
        let begin_sleep = Instant::now();
        if !self.total_wait.is_zero() {
            let (_, _) = accurate_sleep(self.total_wait, false);
        }
        self.start_time = Instant::now();
        self.total_wait = self
            .total_wait
            .saturating_sub(self.start_time.saturating_duration_since(begin_sleep));
    }

    /// `End`.
    pub fn end(&mut self) {
        let now = Instant::now();
        self.total_wait = self
            .total_wait
            .saturating_add(self.target_interval)
            .saturating_sub(now.saturating_duration_since(self.start_time));
    }

    /// `GetTotalWait`.
    pub fn total_wait(&self) -> Duration {
        self.total_wait
    }
}

/// `Common::GetCurrentThreadName`.
pub fn get_current_thread_name() -> String {
    let mut buffer = [0i8; 256];
    // SAFETY: the buffer is valid for the given length and the thread name is at most 15
    // characters on Linux.
    let result = unsafe {
        libc::pthread_getname_np(libc::pthread_self(), buffer.as_mut_ptr(), buffer.len())
    };
    if result != 0 {
        return "<unknown name>".to_string();
    }
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|character| **character != 0)
        .map(|character| *character as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_priority_values_match_the_header() {
        assert_eq!(ThreadPriority::Low as u32, 0);
        assert_eq!(ThreadPriority::Critical as u32, 4);
    }

    #[test]
    fn a_short_sleep_completes() {
        let (completed, remaining) = accurate_sleep(Duration::from_millis(1), false);
        assert!(completed);
        assert_eq!(remaining, Duration::ZERO);
    }

    #[test]
    fn the_precise_sleep_returns_at_or_after_the_deadline() {
        let deadline = Instant::now() + Duration::from_millis(2);
        precise_sleep_until(deadline);
        assert!(Instant::now() >= deadline);
    }

    #[test]
    fn the_accurate_timer_accumulates_the_remainder() {
        let mut timer = AccurateTimer::new(Duration::from_millis(20));
        timer.start();
        std::thread::sleep(Duration::from_millis(5));
        timer.end();
        let wait = timer.total_wait();
        assert!(wait >= Duration::from_millis(10) && wait <= Duration::from_millis(25));
    }

    #[test]
    fn thread_names_round_trip_and_truncate() {
        set_current_thread_name("bb:thread-test");
        assert_eq!(get_current_thread_name(), "bb:thread-test");
        set_current_thread_name("a-name-that-is-too-long");
        assert_eq!(get_current_thread_name(), "a-name-that-is-");
    }
}
