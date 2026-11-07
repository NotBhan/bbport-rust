//! Host threads that may call guest code, ported from
//! `legacy/gpu/shim/core/libraries/kernel/threads.h`.

pub mod pthread;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Thread {
    handle: Option<std::thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}

impl Thread {
    pub fn new() -> Self {
        Self {
            handle: None,
            stop: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run<F>(&mut self, function: F)
    where
        F: FnOnce(&AtomicBool) + Send + 'static,
    {
        let stop = Arc::clone(&self.stop);
        let finished = Arc::clone(&self.finished);
        finished.store(false, Ordering::Release);
        self.handle = Some(std::thread::spawn(move || {
            function(&stop);
            finished.store(true, Ordering::Release);
        }));
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn joinable(&self) -> bool {
        self.handle.is_some()
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.join();
    }

    pub fn finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_runs_and_joins() {
        let mut thread = Thread::new();
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        thread.run(move |_| flag.store(true, Ordering::Release));
        thread.join();
        assert!(ran.load(Ordering::Acquire));
        assert!(thread.finished());
        assert!(!thread.joinable());
    }
}
