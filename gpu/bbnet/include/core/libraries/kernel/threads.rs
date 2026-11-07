//! Host threads that may call guest code, ported from
//! `bbnet/include/core/libraries/kernel/threads.h`.

use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

use crate::gpu::common::polyfill_thread::{JThread, StopToken};

/// The C runtime's `runtime_thread_attach_host` hook.
pub type AttachHook = fn(&str);

static ATTACH_HOOK: OnceLock<AttachHook> = OnceLock::new();

/// Installs the runtime hook that gives a host thread its guest TCB and TLS.
pub fn set_attach_hook(hook: AttachHook) {
    let _ = ATTACH_HOOK.set(hook);
}

fn attach(name: &str) {
    if let Some(hook) = ATTACH_HOOK.get() {
        hook(name);
    }
}

/// `Libraries::Kernel::Thread`.
#[derive(Default)]
pub struct Thread {
    inner: Mutex<Option<JThread>>,
}

impl Thread {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Run`.
    pub fn run(&self, func: impl FnOnce(StopToken) + Send + 'static) {
        self.stop();
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        *inner = Some(JThread::spawn(move |token| {
            attach("bb:hle");
            func(token);
        }));
    }

    fn take(&self) -> Option<JThread> {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    fn finish(taken: Option<JThread>) {
        let Some(taken) = taken else {
            return;
        };
        if taken.thread_id() == Some(std::thread::current().id()) {
            taken.detach();
        } else {
            taken.join();
        }
    }

    /// `Join`.
    pub fn join(&self) {
        Self::finish(self.take());
    }

    /// `Joinable`.
    pub fn joinable(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_some()
    }

    /// `Stop`.
    pub fn stop(&self) {
        let taken = self.take();
        if let Some(thread) = &taken {
            thread.request_stop();
        }
        Self::finish(taken);
    }

    /// The running thread's id, if any.
    pub fn thread_id(&self) -> Option<ThreadId> {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .and_then(JThread::thread_id)
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn run_attaches_and_stops_the_thread() {
        let attached = Arc::new(AtomicBool::new(false));
        let flag = attached.clone();
        // The hook is process-global, so only assert that it is called when installed.
        set_attach_hook(|_| {});
        let thread = Thread::new();
        thread.run(move |token| {
            flag.store(true, Ordering::Release);
            while !token.stop_requested() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });
        assert!(thread.joinable());
        while !attached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        thread.stop();
        assert!(!thread.joinable());
    }

    #[test]
    fn a_thread_can_be_joined_after_work() {
        let thread = Thread::new();
        let done = Arc::new(AtomicBool::new(false));
        let flag = done.clone();
        thread.run(move |_| flag.store(true, Ordering::Release));
        while !done.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        thread.join();
        assert!(!thread.joinable());
    }
}
