//! A stop-token/jthread emulation, ported from `legacy/gpu/shadps4/common/polyfill_thread.h`.
//!
//! Rust has no `std::stop_token`, so the same protocol is built from an atomic flag, a
//! callback registry and a condition variable. `JThread` joins on drop after requesting stop.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

type Callback = Box<dyn Fn() + Send + Sync>;

struct StopState {
    requested: AtomicBool,
    callbacks: Mutex<Vec<(usize, Callback)>>,
    next_callback: AtomicUsize,
}

impl StopState {
    fn new() -> Self {
        Self {
            requested: AtomicBool::new(false),
            callbacks: Mutex::new(Vec::new()),
            next_callback: AtomicUsize::new(0),
        }
    }

    fn request_stop(&self) -> bool {
        if self.requested.swap(true, Ordering::AcqRel) {
            return false;
        }
        let callbacks: Vec<Callback> = {
            let mut registered = self
                .callbacks
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            registered.drain(..).map(|(_, callback)| callback).collect()
        };
        for callback in callbacks {
            callback();
        }
        true
    }
}

/// `std::stop_source`.
#[derive(Clone)]
pub struct StopSource {
    state: Option<Arc<StopState>>,
}

impl StopSource {
    /// `stop_source()`.
    pub fn new() -> Self {
        Self {
            state: Some(Arc::new(StopState::new())),
        }
    }

    /// `stop_source(nostopstate)`.
    pub fn nostopstate() -> Self {
        Self { state: None }
    }

    /// `get_token`.
    pub fn get_token(&self) -> StopToken {
        StopToken {
            state: self.state.clone(),
        }
    }

    /// `stop_possible`.
    pub fn stop_possible(&self) -> bool {
        self.state.is_some()
    }

    /// `stop_requested`.
    pub fn stop_requested(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.requested.load(Ordering::Acquire))
    }

    /// `request_stop`.
    pub fn request_stop(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.request_stop())
    }
}

impl Default for StopSource {
    fn default() -> Self {
        Self::new()
    }
}

/// `std::stop_token`.
#[derive(Clone, Default)]
pub struct StopToken {
    state: Option<Arc<StopState>>,
}

impl StopToken {
    /// `stop_possible`.
    pub fn stop_possible(&self) -> bool {
        self.state.is_some()
    }

    /// `stop_requested`.
    pub fn stop_requested(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.requested.load(Ordering::Acquire))
    }

    /// Registers a callback that runs on `request_stop`; the returned guard unregisters it.
    pub fn register_callback(&self, callback: impl Fn() + Send + Sync + 'static) -> StopCallback {
        match &self.state {
            None => StopCallback { state: None, id: 0 },
            Some(state) => {
                if state.requested.load(Ordering::Acquire) {
                    callback();
                    return StopCallback {
                        state: Some(state.clone()),
                        id: 0,
                    };
                }
                let id = state.next_callback.fetch_add(1, Ordering::AcqRel) + 1;
                state
                    .callbacks
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push((id, Box::new(callback)));
                StopCallback {
                    state: Some(state.clone()),
                    id,
                }
            }
        }
    }
}

/// `std::stop_callback`: unregisters on drop.
pub struct StopCallback {
    state: Option<Arc<StopState>>,
    id: usize,
}

impl Drop for StopCallback {
    fn drop(&mut self) {
        let (Some(state), id) = (&self.state, self.id) else {
            return;
        };
        if id == 0 {
            return;
        }
        state
            .callbacks
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .retain(|(callback_id, _)| *callback_id != id);
    }
}

/// `Common::CondvarWait`: waits until the predicate holds or stop is requested.
pub fn condvar_wait<'a, T>(
    cv: &Arc<Condvar>,
    mut guard: MutexGuard<'a, T>,
    token: &StopToken,
    predicate: impl Fn(&T) -> bool,
) -> MutexGuard<'a, T> {
    if token.stop_requested() {
        return guard;
    }
    let notify = cv.clone();
    let callback = token.register_callback(move || notify.notify_all());
    while !predicate(&guard) && !token.stop_requested() {
        guard = cv.wait(guard).unwrap_or_else(|error| error.into_inner());
    }
    drop(callback);
    guard
}

/// `Common::StoppableTimedWait`: `true` when the full duration elapsed, `false` when stopped.
pub fn stoppable_timed_wait(token: &StopToken, duration: Duration) -> bool {
    if token.stop_requested() {
        return false;
    }
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let notify = pair.clone();
    let callback = token.register_callback(move || {
        *notify.0.lock().unwrap_or_else(|error| error.into_inner()) = true;
        notify.1.notify_one();
    });
    let mut stopped = pair.0.lock().unwrap_or_else(|error| error.into_inner());
    let (guard, _) = pair
        .1
        .wait_timeout_while(stopped, duration, |stopped| !*stopped)
        .unwrap_or_else(|error| error.into_inner());
    stopped = guard;
    let result = !*stopped;
    drop(stopped);
    drop(callback);
    result
}

/// A `std::jthread` equivalent: requests stop and joins when dropped.
pub struct JThread {
    handle: Option<JoinHandle<()>>,
    thread_id: Option<std::thread::ThreadId>,
    source: StopSource,
}

impl JThread {
    /// `jthread(f)`: the closure receives the thread's stop token.
    pub fn spawn(f: impl FnOnce(StopToken) + Send + 'static) -> Self {
        let source = StopSource::new();
        let token = source.get_token();
        let handle = std::thread::spawn(move || f(token));
        let thread_id = Some(handle.thread().id());
        Self {
            handle: Some(handle),
            thread_id,
            source,
        }
    }

    /// `joinable`.
    pub fn joinable(&self) -> bool {
        self.handle.is_some()
    }

    /// `join`.
    pub fn join(mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// `detach`.
    pub fn detach(mut self) {
        self.handle.take();
    }

    /// The spawned thread's id, while it is still owned.
    pub fn thread_id(&self) -> Option<std::thread::ThreadId> {
        self.thread_id
    }

    /// `get_stop_source`.
    pub fn get_stop_source(&self) -> StopSource {
        self.source.clone()
    }

    /// `get_stop_token`.
    pub fn get_stop_token(&self) -> StopToken {
        self.source.get_token()
    }

    /// `request_stop`.
    pub fn request_stop(&self) -> bool {
        self.source.request_stop()
    }
}

impl Drop for JThread {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.source.request_stop();
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Instant;

    #[test]
    fn a_callback_runs_once_when_stop_is_requested() {
        let source = StopSource::new();
        let calls = Arc::new(AtomicU32::new(0));
        let counter = calls.clone();
        let _callback = source.get_token().register_callback(move || {
            counter.fetch_add(1, Ordering::AcqRel);
        });
        assert!(source.request_stop());
        assert!(!source.request_stop());
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert!(source.stop_requested());
    }

    #[test]
    fn a_late_callback_runs_immediately() {
        let source = StopSource::new();
        source.request_stop();
        let calls = Arc::new(AtomicU32::new(0));
        let counter = calls.clone();
        let _callback = source.get_token().register_callback(move || {
            counter.fetch_add(1, Ordering::AcqRel);
        });
        assert_eq!(calls.load(Ordering::Acquire), 1);
    }

    #[test]
    fn condvar_wait_wakes_when_stopped() {
        let ready = Arc::new(Mutex::new(false));
        let cv = Arc::new(Condvar::new());
        let source = StopSource::new();
        let waiter = {
            let ready = ready.clone();
            let cv = cv.clone();
            let token = source.get_token();
            std::thread::spawn(move || {
                let guard = ready.lock().unwrap();
                let _guard = condvar_wait(&cv, guard, &token, |ready| *ready);
            })
        };
        std::thread::sleep(Duration::from_millis(10));
        source.request_stop();
        waiter.join().unwrap();
    }

    #[test]
    fn the_stoppable_wait_distinguishes_timeout_from_stop() {
        let source = StopSource::new();
        assert!(stoppable_timed_wait(
            &source.get_token(),
            Duration::from_millis(1)
        ));

        let stopper = source.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(5));
            stopper.request_stop();
        });
        let start = Instant::now();
        assert!(!stoppable_timed_wait(
            &source.get_token(),
            Duration::from_secs(5)
        ));
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn dropping_a_jthread_requests_stop_and_joins() {
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        let thread = JThread::spawn(move |token| {
            while !token.stop_requested() {
                std::thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, Ordering::Release);
        });
        drop(thread);
        assert!(stopped.load(Ordering::Acquire));
    }
}
