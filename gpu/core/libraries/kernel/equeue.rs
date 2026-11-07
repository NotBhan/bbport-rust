//! Kernel event queues, ported from `shadps4/core/libraries/kernel/equeue.h`.

/// `OrbisKernelBintime`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisKernelBintime {
    pub sec: i64,
    pub frac: i64,
}

/// `OrbisKernelUseconds`.
pub type OrbisKernelUseconds = u32;
/// `OrbisKernelEqueue`.
pub type OrbisKernelEqueue = i64;

/// `OrbisKernelEvent::Filter`.
pub mod filter {
    pub const NONE: i16 = 0;
    pub const READ: i16 = -1;
    pub const WRITE: i16 = -2;
    pub const AIO: i16 = -3;
    pub const VNODE: i16 = -4;
    pub const PROC: i16 = -5;
    pub const SIGNAL: i16 = -6;
    pub const TIMER: i16 = -7;
    pub const FS: i16 = -9;
    pub const LIO: i16 = -10;
    pub const USER: i16 = -11;
    pub const POLLING: i16 = -12;
    pub const VIDEO_OUT: i16 = -13;
    pub const GRAPHICS_CORE: i16 = -14;
    pub const HR_TIMER: i16 = -15;
}

/// `OrbisKernelEvent::Flags`.
pub mod flags {
    pub const ADD: u16 = 1;
    pub const DELETE: u16 = 2;
    pub const ENABLE: u16 = 4;
    pub const DISABLE: u16 = 8;
    pub const ONE_SHOT: u16 = 0x10;
    pub const CLEAR: u16 = 0x20;
    pub const RECEIPT: u16 = 0x40;
    pub const DISPATCH: u16 = 0x80;
    pub const FLAG1: u16 = 0x2000;
    pub const SYSTEM: u16 = 0xf000;
}

/// `OrbisKernelEvent`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisKernelEvent {
    pub ident: u64,
    pub filter: i16,
    pub flags: u16,
    pub fflags: u32,
    pub data: u64,
    pub udata: *mut core::ffi::c_void,
}

impl Default for OrbisKernelEvent {
    fn default() -> Self {
        Self {
            ident: 0,
            filter: filter::NONE,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        }
    }
}

/// `OrbisVideoOutEventHint`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisVideoOutEventHint {
    pub raw: u64,
}

impl OrbisVideoOutEventHint {
    pub fn event_id(&self) -> u64 {
        self.raw & 0xff
    }

    pub fn video_id(&self) -> u64 {
        (self.raw >> 8) & 0xff
    }

    pub fn flip_arg(&self) -> u64 {
        self.raw >> 16
    }
}

/// `OrbisVideoOutEventData`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisVideoOutEventData {
    pub raw: u64,
}

impl OrbisVideoOutEventData {
    pub fn time(&self) -> u64 {
        self.raw & 0xfff
    }

    pub fn count(&self) -> u64 {
        (self.raw >> 12) & 0xf
    }

    pub fn flip_arg(&self) -> u64 {
        self.raw >> 16
    }
}

/// `EqueueEvent`; the timer handle is owned by the runtime's timer implementation.
#[derive(Debug, Clone, Default)]
pub struct EqueueEvent {
    pub event: OrbisKernelEvent,
    pub data: *mut core::ffi::c_void,
    pub time_added: Option<std::time::Instant>,
    pub timer_interval: std::time::Duration,
    triggered: bool,
}

impl EqueueEvent {
    /// `Clear`.
    pub fn clear(&mut self) {
        self.triggered = false;
        self.event.fflags = 0;
        self.event.data = 0;
    }

    /// `Trigger`.
    pub fn trigger(&mut self, data: *mut core::ffi::c_void) {
        self.triggered = true;
        self.event.data = data as usize as u64;
    }

    /// `TriggerUser`.
    pub fn trigger_user(&mut self, data: *mut core::ffi::c_void) {
        self.triggered = true;
        self.event.udata = data;
    }

    /// `TriggerDisplay`.
    pub fn trigger_display(&mut self, data: *mut core::ffi::c_void) {
        self.triggered = true;
        if data.is_null() {
            return;
        }
        let event_data = OrbisVideoOutEventData {
            raw: self.event.data,
        };
        let hint_raw = data as usize as u64;
        let hint = OrbisVideoOutEventHint { raw: hint_raw };
        if hint.event_id() == self.event.ident && self.event.ident != 0xfe {
            let time = crate::gpu::common::rdtsc::fenced_rdtsc();
            let mut counter = event_data.count();
            if counter != 0xf {
                counter += 1;
            }
            self.event.data =
                (time & 0xfff) | (counter << 0xc) | (hint_raw & 0xffff_ffff_ffff_0000);
        }
    }

    /// `TriggerTimer`.
    pub fn trigger_timer(&mut self) {
        self.triggered = true;
        self.event.data = self.event.data.wrapping_add(1);
    }

    /// `IsTriggered`.
    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    /// `operator==`: events are identified by id and filter.
    pub fn matches(&self, other: &EqueueEvent) -> bool {
        self.event.ident == other.event.ident && self.event.filter == other.event.filter
    }
}

/// The `EqueueInternal` surface; the runtime provides the implementation.
pub trait EqueueApi {
    fn name(&self) -> &str;
    fn add_event(&mut self, event: EqueueEvent) -> bool;
    fn remove_event(&mut self, id: u64, filter: i16) -> bool;
    fn wait_for_events(&mut self, events: &mut [OrbisKernelEvent], timeout: Option<u32>) -> i32;
    fn trigger_event(&mut self, ident: u64, filter: i16, data: *mut core::ffi::c_void) -> bool;
    fn get_triggered_events(&mut self, events: &mut [OrbisKernelEvent], max: i32) -> i32;
    fn event_exists(&self, id: u64, filter: i16) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_layout_matches_the_header() {
        assert_eq!(core::mem::size_of::<OrbisKernelEvent>(), 32);
        assert_eq!(core::mem::offset_of!(OrbisKernelEvent, filter), 8);
        assert_eq!(core::mem::offset_of!(OrbisKernelEvent, data), 16);
        assert_eq!(filter::HR_TIMER, -15);
        assert_eq!(flags::DISPATCH, 0x80);
        assert_eq!(core::mem::size_of::<OrbisKernelBintime>(), 16);
    }

    #[test]
    fn events_are_identified_by_id_and_filter() {
        let first = EqueueEvent {
            event: OrbisKernelEvent {
                ident: 7,
                filter: filter::TIMER,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut second = first.clone();
        assert!(first.matches(&second));
        second.event.filter = filter::USER;
        assert!(!first.matches(&second));

        let mut event = EqueueEvent::default();
        event.trigger(std::ptr::dangling_mut());
        assert!(event.is_triggered());
        event.clear();
        assert!(!event.is_triggered());
        event.trigger_timer();
        assert_eq!(event.event.data, 1);
    }

    #[test]
    fn video_out_bitfields_are_packed() {
        let hint = OrbisVideoOutEventHint {
            raw: 0x1234_5678_9ABC_DEF0,
        };
        assert_eq!(hint.event_id(), 0xF0);
        assert_eq!(hint.video_id(), 0xDE);
        assert_eq!(hint.flip_arg(), 0x1234_5678_9ABC);
        let data = OrbisVideoOutEventData { raw: 0xABC5 };
        assert_eq!(data.time(), 0xBC5);
        assert_eq!(data.count(), 0xA);
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::gpu::core::libraries::kernel::orbis_error::*;
use crate::gpu::core::libraries::kernel::posix_error::*;

struct SmallTimer {
    event: OrbisKernelEvent,
    added: Instant,
    interval: Duration,
}

#[derive(Default)]
struct EqueueState {
    events: Vec<EqueueEvent>,
    small_timers: HashMap<u64, SmallTimer>,
}

/// `EqueueInternal`: events, their triggered payloads and the small timers.
pub struct EqueueInternal {
    handle: OrbisKernelEqueue,
    name: String,
    state: Mutex<EqueueState>,
    cond: Condvar,
}

impl EqueueInternal {
    pub fn new(handle: OrbisKernelEqueue, name: impl Into<String>) -> Self {
        Self {
            handle,
            name: name.into(),
            state: Mutex::new(EqueueState::default()),
            cond: Condvar::new(),
        }
    }

    /// `GetName`.
    pub fn name(&self) -> &str {
        &self.name
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, EqueueState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    unsafe fn interval_for(event: &OrbisKernelEvent) -> Duration {
        if event.filter == filter::TIMER {
            Duration::from_millis(event.data)
        } else if event.filter == filter::HR_TIMER {
            // SAFETY: the caller stores the bintime pointer in `data` for HR timers.
            let time = unsafe { &*(event.data as usize as *const OrbisKernelBintime) };
            let nanoseconds = ((1_000_000_000i64 * (time.frac >> 32)) >> 32) as u64;
            Duration::new(time.sec.max(0) as u64, nanoseconds.min(999_999_999) as u32)
        } else {
            Duration::ZERO
        }
    }

    /// `AddEvent`.
    ///
    /// # Safety
    /// For an `HR_TIMER` event, `event.event.data` must point to a readable
    /// `OrbisKernelBintime`.
    pub unsafe fn add_event(&self, mut event: EqueueEvent) -> bool {
        let id = event.event.ident;
        let event_filter = event.event.filter;
        let mut state = self.lock();
        event.time_added = Some(Instant::now());
        event.timer_interval = unsafe { Self::interval_for(&event.event) };
        if let Some(existing) = state
            .events
            .iter_mut()
            .find(|existing| existing.event.ident == id && existing.event.filter == event_filter)
        {
            existing.timer_interval = event.timer_interval;
            existing.event.udata = event.event.udata;
            return true;
        }
        event.event.data = 0;
        event.event.flags &= !flags::ADD;
        if event.event.filter != filter::USER {
            event.event.flags |= flags::CLEAR;
        }
        state.events.push(event);
        true
    }

    /// `ScheduleEvent`; timers are evaluated by [`Self::wait_for_events`].
    pub fn schedule_event(&self, id: u64, event_filter: i16) -> bool {
        self.lock()
            .events
            .iter()
            .any(|event| event.event.ident == id && event.event.filter == event_filter)
    }

    /// `RemoveEvent`.
    pub fn remove_event(&self, id: u64, event_filter: i16) -> bool {
        let mut state = self.lock();
        let before = state.events.len();
        state
            .events
            .retain(|event| !(event.event.ident == id && event.event.filter == event_filter));
        state.events.len() != before
    }

    fn trigger_due_timers(&self, state: &mut EqueueState) {
        let now = Instant::now();
        for event in &mut state.events {
            if event.event.filter != filter::TIMER && event.event.filter != filter::HR_TIMER {
                continue;
            }
            if event.timer_interval.is_zero() || event.is_triggered() {
                continue;
            }
            let Some(added) = event.time_added else {
                continue;
            };
            if now.duration_since(added) >= event.timer_interval {
                event.trigger_timer();
                event.time_added = Some(now);
            }
        }
    }

    /// `WaitForEvents`.
    pub fn wait_for_events(&self, events: &mut [OrbisKernelEvent], timeout: Option<u32>) -> i32 {
        if timeout == Some(0) {
            return self.get_triggered_events(events);
        }
        if self.has_small_timer() {
            return self.wait_for_small_timer(events, timeout.unwrap_or(0));
        }
        let deadline =
            timeout.map(|micros| Instant::now() + Duration::from_micros(u64::from(micros)));
        let mut state = self.lock();
        loop {
            self.trigger_due_timers(&mut state);
            let count = Self::collect_triggered(&mut state, events);
            if count > 0 {
                return count;
            }
            state = match deadline {
                None => self
                    .cond
                    .wait(state)
                    .unwrap_or_else(|error| error.into_inner()),
                Some(deadline) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return 0;
                    }
                    self.cond
                        .wait_timeout(state, deadline - now)
                        .map(|(state, _)| state)
                        .unwrap_or_else(|error| error.into_inner().0)
                }
            };
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                self.trigger_due_timers(&mut state);
                return Self::collect_triggered(&mut state, events);
            }
        }
    }

    fn collect_triggered(state: &mut EqueueState, events: &mut [OrbisKernelEvent]) -> i32 {
        let mut count = 0;
        let mut index = 0;
        while index < state.events.len() && count < events.len() {
            if !state.events[index].is_triggered() {
                index += 1;
                continue;
            }
            let event = state.events[index].event;
            events[count] = event;
            count += 1;
            if event.flags & flags::CLEAR != 0 {
                state.events[index].clear();
            }
            if event.flags & flags::ONE_SHOT != 0 {
                state.events.remove(index);
            } else {
                index += 1;
            }
        }
        count as i32
    }

    /// `TriggerEvent`.
    pub fn trigger_event(
        &self,
        ident: u64,
        event_filter: i16,
        data: *mut core::ffi::c_void,
    ) -> bool {
        let mut state = self.lock();
        let mut found = false;
        for event in &mut state.events {
            if event.event.ident != ident || event.event.filter != event_filter {
                continue;
            }
            if event_filter == filter::VIDEO_OUT {
                event.trigger_display(data);
            } else if event_filter == filter::USER {
                event.trigger_user(data);
            } else if event_filter == filter::TIMER || event_filter == filter::HR_TIMER {
                event.trigger_timer();
            } else {
                event.trigger(data);
            }
            found = true;
        }
        drop(state);
        self.cond.notify_all();
        found
    }

    /// `GetTriggeredEvents`.
    pub fn get_triggered_events(&self, events: &mut [OrbisKernelEvent]) -> i32 {
        let mut state = self.lock();
        Self::collect_triggered(&mut state, events)
    }

    /// `AddSmallTimer`.
    ///
    /// # Safety
    /// `event.data` must point to a readable `OrbisKernelBintime`.
    pub unsafe fn add_small_timer(&self, event: &OrbisKernelEvent) -> bool {
        // SAFETY: the caller stores the bintime pointer in `data`.
        let time = unsafe { &*(event.data as usize as *const OrbisKernelBintime) };
        let nanoseconds = ((1_000_000_000i64 * (time.frac >> 32)) >> 32) as u64;
        let interval = Duration::new(time.sec.max(0) as u64, nanoseconds.min(999_999_999) as u32);
        let mut state = self.lock();
        state.small_timers.insert(
            event.ident,
            SmallTimer {
                event: *event,
                added: Instant::now(),
                interval,
            },
        );
        true
    }

    /// `HasSmallTimer`.
    pub fn has_small_timer(&self) -> bool {
        !self.lock().small_timers.is_empty()
    }

    /// `RemoveSmallTimer`.
    pub fn remove_small_timer(&self, id: u64) -> bool {
        self.lock().small_timers.remove(&id).is_some()
    }

    /// `WaitForSmallTimer`.
    pub fn wait_for_small_timer(&self, events: &mut [OrbisKernelEvent], micros: u32) -> i32 {
        if events.is_empty() {
            return 0;
        }
        let deadline = if micros == 0 {
            None
        } else {
            Some(Instant::now() + Duration::from_micros(u64::from(micros)))
        };
        loop {
            let now = Instant::now();
            let mut state = self.lock();
            let mut count = 0;
            let mut due = Vec::new();
            for (id, timer) in state.small_timers.iter() {
                if count >= events.len() {
                    break;
                }
                if now.duration_since(timer.added) >= timer.interval {
                    events[count] = timer.event;
                    count += 1;
                    due.push(*id);
                }
            }
            for id in due {
                state.small_timers.remove(&id);
            }
            if count > 0 {
                return count as i32;
            }
            drop(state);
            if deadline.is_some_and(|deadline| now >= deadline) {
                return 0;
            }
            std::thread::yield_now();
        }
    }

    /// `EventExists`.
    pub fn event_exists(&self, id: u64, event_filter: i16) -> bool {
        self.lock()
            .events
            .iter()
            .any(|event| event.event.ident == id && event.event.filter == event_filter)
    }

    /// The queue handle.
    pub fn handle(&self) -> OrbisKernelEqueue {
        self.handle
    }
}

// The guest data/udata pointers are opaque values guarded by the queue mutex.
unsafe impl Send for EqueueInternal {}
unsafe impl Sync for EqueueInternal {}

fn kqueues() -> &'static Mutex<HashMap<OrbisKernelEqueue, Arc<EqueueInternal>>> {
    static KQUEUES: std::sync::OnceLock<Mutex<HashMap<OrbisKernelEqueue, Arc<EqueueInternal>>>> =
        std::sync::OnceLock::new();
    KQUEUES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_handle() -> OrbisKernelEqueue {
    static NEXT: AtomicI32 = AtomicI32::new(0x40000);
    NEXT.fetch_add(1, Ordering::Relaxed) as OrbisKernelEqueue
}

/// `GetEqueue`.
pub fn get_equeue(eq: OrbisKernelEqueue) -> Option<Arc<EqueueInternal>> {
    kqueues()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&eq)
        .cloned()
}

/// `SupportedEqueueFilter`.
pub fn supported_equeue_filter(event_filter: i16) -> bool {
    matches!(
        event_filter,
        filter::GRAPHICS_CORE | filter::HR_TIMER | filter::TIMER | filter::USER | filter::VIDEO_OUT
    )
}

/// `posix_kqueue`.
pub fn posix_kqueue() -> OrbisKernelEqueue {
    let handle = next_handle();
    let name = format!("kqueue{handle}");
    kqueues()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(handle, Arc::new(EqueueInternal::new(handle, name)));
    handle
}

/// `sceKernelCreateEqueue`.
pub fn create_equeue(name: &str) -> Result<OrbisKernelEqueue, i32> {
    if name.len() > 32 {
        return Err(ORBIS_KERNEL_ERROR_ENAMETOOLONG);
    }
    let handle = next_handle();
    kqueues()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(handle, Arc::new(EqueueInternal::new(handle, name)));
    Ok(handle)
}

/// `sceKernelDeleteEqueue`.
pub fn delete_equeue(eq: OrbisKernelEqueue) -> i32 {
    if kqueues()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&eq)
        .is_some()
    {
        0
    } else {
        ORBIS_KERNEL_ERROR_EBADF
    }
}

/// `sceKernelWaitEqueue`.
pub fn wait_equeue(
    eq: OrbisKernelEqueue,
    events: &mut [OrbisKernelEvent],
    timeout: Option<u32>,
) -> Result<(i32, i32), i32> {
    let Some(queue) = get_equeue(eq) else {
        return Err(ORBIS_KERNEL_ERROR_EBADF);
    };
    let count = queue.wait_for_events(events, timeout);
    Ok((0, count))
}

/// `sceKernelTriggerEqueue`.
pub fn trigger_equeue(
    eq: OrbisKernelEqueue,
    ident: u64,
    event_filter: i16,
    data: *mut core::ffi::c_void,
    timeout: Option<u32>,
) -> Result<i32, i32> {
    let Some(queue) = get_equeue(eq) else {
        return Err(ORBIS_KERNEL_ERROR_EBADF);
    };
    queue.trigger_event(ident, event_filter, data);
    let mut events = Vec::new();
    let count = queue.wait_for_events(&mut events, timeout);
    Ok(count)
}

/// `sceKernelGetEventData`.
pub fn get_event_data(event: &OrbisKernelEvent) -> u64 {
    event.data
}

/// `posix_kevent`, without the errno side channel.
/// # Safety
/// HR-timer entries in `changelist` must carry a readable `OrbisKernelBintime` pointer, and
/// `eventlist` must be writable.
pub unsafe fn posix_kevent(
    handle: OrbisKernelEqueue,
    changelist: &[OrbisKernelEvent],
    eventlist: &mut [OrbisKernelEvent],
    timeout: Option<u32>,
) -> i32 {
    let Some(queue) = get_equeue(handle) else {
        return -1;
    };
    for event in changelist {
        if !supported_equeue_filter(event.filter) {
            continue;
        }
        if event.flags & flags::ADD != 0 {
            let internal = EqueueEvent {
                event: *event,
                ..EqueueEvent::default()
            };
            if !unsafe { queue.add_event(internal) } {
                return -1;
            }
        }
        if event.flags & flags::DELETE != 0 && !queue.remove_event(event.ident, event.filter) {
            return -1;
        }
        if event.filter == filter::USER
            && event.fflags == 0x1000000
            && !queue.trigger_event(event.ident, filter::USER, event.udata)
        {
            return -1;
        }
    }
    let _ = POSIX_EBADF;
    queue.wait_for_events(eventlist, timeout)
}

#[cfg(test)]
mod equeue_impl_tests {
    use super::*;

    fn user_event(ident: u64) -> OrbisKernelEvent {
        OrbisKernelEvent {
            ident,
            filter: filter::USER,
            flags: flags::ADD,
            ..OrbisKernelEvent::default()
        }
    }

    #[test]
    fn the_queue_create_delete_and_wait_paths_work() {
        let handle = create_equeue("test").expect("a queue");
        assert!(get_equeue(handle).is_some());
        {
            let queue = get_equeue(handle).unwrap();
            let internal = EqueueInternal::new(handle, "test");
            assert!(unsafe {
                internal.add_event(EqueueEvent {
                    event: user_event(7),
                    ..EqueueEvent::default()
                })
            });
            assert!(internal.event_exists(7, filter::USER));
            assert!(internal.trigger_event(7, filter::USER, 0x1234 as *mut _));
            let mut events = [OrbisKernelEvent::default(); 4];
            assert_eq!(internal.get_triggered_events(&mut events), 1);
            assert_eq!(events[0].ident, 7);
            assert_eq!(events[0].udata as usize, 0x1234);
            assert!(internal.remove_event(7, filter::USER));

            // The registry queue accepts events and waits with a timeout.
            unsafe {
                queue.add_event(EqueueEvent {
                    event: user_event(9),
                    ..EqueueEvent::default()
                });
            }
            queue.trigger_event(9, filter::USER, std::ptr::null_mut());
            let mut events = [OrbisKernelEvent::default(); 4];
            assert_eq!(queue.wait_for_events(&mut events, Some(10)), 1);
            assert_eq!(events[0].ident, 9);
        }
        assert_eq!(delete_equeue(handle), 0);
        assert_eq!(delete_equeue(handle), ORBIS_KERNEL_ERROR_EBADF);
        assert!(create_equeue("0123456789012345678901234567890123").is_err());
    }

    #[test]
    fn oneshot_events_are_removed_after_delivery() {
        let queue = EqueueInternal::new(1, "oneshot");
        let mut event = user_event(4);
        event.flags |= flags::ONE_SHOT;
        assert!(unsafe {
            queue.add_event(EqueueEvent {
                event,
                ..EqueueEvent::default()
            })
        });
        assert!(queue.trigger_event(4, filter::USER, std::ptr::null_mut()));
        let mut events = [OrbisKernelEvent::default(); 2];
        assert_eq!(queue.get_triggered_events(&mut events), 1);
        assert!(!queue.event_exists(4, filter::USER));
    }

    #[test]
    fn timer_events_fire_after_their_interval() {
        let queue = EqueueInternal::new(2, "timer");
        let mut event = user_event(5);
        event.filter = filter::TIMER;
        event.data = 1; // one millisecond
        assert!(unsafe {
            queue.add_event(EqueueEvent {
                event,
                ..EqueueEvent::default()
            })
        });
        let mut events = [OrbisKernelEvent::default(); 2];
        assert_eq!(
            queue.wait_for_events(&mut events, Some(20_000)),
            1,
            "the timer expires"
        );
        assert_eq!(events[0].ident, 5);
    }

    #[test]
    fn an_empty_small_timer_output_buffer_returns_zero() {
        let queue = EqueueInternal::new(4, "empty");
        let time = OrbisKernelBintime { sec: 0, frac: 0 };
        let mut event = user_event(6);
        event.data = &time as *const OrbisKernelBintime as usize as u64;
        assert!(unsafe { queue.add_small_timer(&event) });
        assert_eq!(queue.wait_for_small_timer(&mut [], 0), 0);
    }

    #[test]
    fn small_timers_are_waitable_and_removable() {
        let queue = EqueueInternal::new(3, "small");
        let time = OrbisKernelBintime { sec: 0, frac: 0 };
        let mut event = user_event(6);
        event.data = &time as *const OrbisKernelBintime as usize as u64;
        assert!(unsafe { queue.add_small_timer(&event) });
        assert!(queue.has_small_timer());
        let mut events = [OrbisKernelEvent::default(); 2];
        assert_eq!(queue.wait_for_small_timer(&mut events, 10_000), 1);
        assert_eq!(events[0].ident, 6);
        assert!(!queue.has_small_timer());
        assert!(!queue.remove_small_timer(6));
    }
}
