//! Texture-binding helper thread, ported from
//! `shadps4/video_core/renderer_vulkan/vk_bind_helper.h`.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub type Task = unsafe extern "C" fn(*mut core::ffi::c_void);

struct TaskSlot {
    task: Option<Task>,
    context: *mut core::ffi::c_void,
}

// The context pointer is only dereferenced by the installed task on the helper thread.
unsafe impl Send for TaskSlot {}

struct Shared {
    slot: Mutex<TaskSlot>,
    cv: Condvar,
    posted: Mutex<u64>,
    posted_cv: Condvar,
    done: Mutex<u64>,
    done_cv: Condvar,
    stop: Mutex<bool>,
    task_cycles: Mutex<u64>,
}

impl Shared {
    fn new() -> Self {
        Self {
            slot: Mutex::new(TaskSlot {
                task: None,
                context: std::ptr::null_mut(),
            }),
            cv: Condvar::new(),
            posted: Mutex::new(0),
            posted_cv: Condvar::new(),
            done: Mutex::new(0),
            done_cv: Condvar::new(),
            stop: Mutex::new(false),
            task_cycles: Mutex::new(0),
        }
    }
}

thread_local! {
    static ON_HELPER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `BindHelper`.
pub struct BindHelper {
    running: bool,
    active: bool,
    wait_cycles: u64,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl BindHelper {
    pub fn new(enabled: bool) -> Self {
        let shared = Arc::new(Shared::new());
        let thread = if enabled {
            let worker = shared.clone();
            Some(thread::spawn(move || run(worker)))
        } else {
            None
        };
        Self {
            running: enabled,
            active: false,
            wait_cycles: 0,
            shared,
            thread,
        }
    }

    /// `Available`.
    pub fn available(&self) -> bool {
        self.running
    }

    /// `OnHelper`.
    pub fn on_helper() -> bool {
        ON_HELPER.with(|flag| flag.get())
    }

    /// `Active`.
    pub fn active(&self) -> bool {
        self.active
    }

    /// `Fork`.
    ///
    /// # Safety
    /// `context` must stay valid until the task finishes (immediately when the helper is
    /// disabled, otherwise until [`BindHelper::join`] returns).
    pub unsafe fn fork(&mut self, task: Task, context: *mut core::ffi::c_void) {
        if !self.running {
            unsafe { task(context) };
            return;
        }
        {
            let mut slot = self
                .shared
                .slot
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            slot.task = Some(task);
            slot.context = context;
        }
        self.active = true;
        *self
            .shared
            .posted
            .lock()
            .unwrap_or_else(|error| error.into_inner()) += 1;
        self.shared.cv.notify_one();
        self.shared.posted_cv.notify_all();
    }

    /// `Join`.
    pub fn join(&mut self) {
        if !self.active {
            return;
        }
        let target = *self
            .shared
            .posted
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let start = Instant::now();
        {
            let mut done = self
                .shared
                .done
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            while *done != target {
                done = self
                    .shared
                    .done_cv
                    .wait_timeout(done, Duration::from_micros(50))
                    .unwrap_or_else(|error| error.into_inner())
                    .0;
            }
        }
        self.wait_cycles +=
            start.elapsed().as_secs() * 3_000_000_000 + u64::from(start.elapsed().subsec_nanos());
        self.active = false;
    }

    /// `wait_cycles`.
    pub fn wait_cycles(&self) -> u64 {
        self.wait_cycles
    }

    /// `task_cycles`.
    pub fn task_cycles(&self) -> u64 {
        *self
            .shared
            .task_cycles
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

fn run(shared: Arc<Shared>) {
    ON_HELPER.with(|flag| flag.set(true));
    let mut seen = 0u64;
    loop {
        let posted = {
            let mut posted = shared
                .posted
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            while *posted == seen {
                if *shared
                    .stop
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                {
                    return;
                }
                let (next, _) = shared
                    .cv
                    .wait_timeout(posted, Duration::from_millis(5))
                    .unwrap_or_else(|error| error.into_inner());
                posted = next;
            }
            *posted
        };
        let (task, context) = {
            let mut slot = shared
                .slot
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            (slot.task.take(), slot.context)
        };
        let start = Instant::now();
        if let Some(task) = task {
            // SAFETY: the GPU thread forked this task with a live context until join.
            unsafe { task(context) };
        }
        *shared
            .task_cycles
            .lock()
            .unwrap_or_else(|error| error.into_inner()) += start.elapsed().as_micros() as u64;
        seen = posted;
        *shared
            .done
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = seen;
        shared.done_cv.notify_all();
    }
}

impl Drop for BindHelper {
    fn drop(&mut self) {
        *self
            .shared
            .stop
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        self.shared.cv.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static RAN: AtomicU32 = AtomicU32::new(0);

    unsafe extern "C" fn task(context: *mut core::ffi::c_void) {
        unsafe {
            *(context as *mut u32) = 41;
        }
        RAN.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn the_helper_runs_a_forked_task_and_joins() {
        let mut helper = BindHelper::new(true);
        let mut value = 0u32;
        assert!(helper.available());
        unsafe {
            helper.fork(task, &mut value as *mut u32 as *mut core::ffi::c_void);
        }
        assert!(helper.active());
        helper.join();
        assert!(!helper.active());
        assert_eq!(value, 41);
        assert_eq!(RAN.load(Ordering::SeqCst), 1);
        assert!(!BindHelper::on_helper());
    }

    unsafe extern "C" fn inline_task(context: *mut core::ffi::c_void) {
        unsafe {
            *(context as *mut u32) = 42;
        }
    }

    #[test]
    fn a_disabled_helper_runs_the_task_inline() {
        let mut helper = BindHelper::new(false);
        let mut value = 0u32;
        assert!(!helper.available());
        unsafe {
            helper.fork(
                inline_task,
                &mut value as *mut u32 as *mut core::ffi::c_void,
            );
        }
        helper.join();
        assert_eq!(value, 42);
        assert!(!helper.active());
    }
}
