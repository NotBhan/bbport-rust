//! Parallel guest-memory copy helpers, ported from `legacy/gpu/shim/bbport_copy.h` and
//! `bbport_copy.cpp`: a small copy pool, the FIFO batch queue, and `ParallelFor`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, Weak};

use crate::gpu::shim::bbport_toggles;

/// A small copy for `QueueCopy()`: `run(item)` does the work on a copy thread.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub run: fn(&Item),
    pub context: *mut c_void,
    pub source: u64,
    pub destination: u64,
    pub size: u64,
    pub extra: u64,
}

struct SendItems(Vec<Item>);
// The copy pool owns the items and runs them on its thread, as in the C++ port.
unsafe impl Send for SendItems {}

/// The batch is started once it holds about 512 KiB or 256 items.
const BATCH_BYTES: u64 = 512 * 1024;
const BATCH_ITEMS: usize = 256;

thread_local! {
    static IN_COPY_THREAD: Cell<bool> = const { Cell::new(false) };
    static BATCH: RefCell<Vec<Item>> = const { RefCell::new(Vec::new()) };
}

struct Task {
    run: Box<dyn FnOnce() + Send>,
    epoch: u64,
}

#[derive(Default)]
struct Epoch {
    pending: usize,
    callbacks: Vec<Box<dyn FnOnce() + Send>>,
}

#[derive(Default)]
struct PoolState {
    stop: bool,
    tasks: VecDeque<Task>,
    epochs: VecDeque<Epoch>,
    epoch_base: u64,
    pending: usize,
    draining: bool,
}

struct Pool {
    state: Mutex<PoolState>,
    cv: Condvar,
}

impl Pool {
    fn lock(&self) -> MutexGuard<'_, PoolState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn worker_count() -> usize {
    let mut count = (crate::gpu::shim::bbport_threads::available() / 4).clamp(1, 4) as usize;
    if let Some(value) = std::env::var("BB_COPY_THREADS")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
    {
        count = value.clamp(0, 16) as usize;
    }
    count
}

fn pool() -> Option<&'static Arc<Pool>> {
    static POOL: OnceLock<Option<Arc<Pool>>> = OnceLock::new();
    POOL.get_or_init(|| {
        let count = worker_count();
        if count == 0 {
            return None;
        }
        let pool = Arc::new(Pool {
            state: Mutex::new(PoolState {
                epochs: VecDeque::from([Epoch::default()]),
                ..PoolState::default()
            }),
            cv: Condvar::new(),
        });
        for _ in 0..count {
            let weak = Arc::downgrade(&pool);
            std::thread::spawn(move || worker(weak));
        }
        Some(pool)
    })
    .as_ref()
}

fn worker(pool: Weak<Pool>) {
    loop {
        let Some(pool) = pool.upgrade() else {
            return;
        };
        let task = {
            let mut state = pool.lock();
            while !state.stop && state.tasks.is_empty() {
                state = pool
                    .cv
                    .wait(state)
                    .unwrap_or_else(|error| error.into_inner());
            }
            if state.stop {
                return;
            }
            state.tasks.pop_front()
        };
        if let Some(task) = task {
            run_task(&pool, task);
        }
    }
}

fn run_task(pool: &Pool, task: Task) {
    IN_COPY_THREAD.with(|flag| {
        let was = flag.get();
        flag.set(true);
        (task.run)();
        flag.set(was);
    });
    complete(pool, task.epoch);
}

/// Retires a task and runs the callbacks of epochs with nothing left, oldest first. One thread
/// drains at a time so the callbacks keep their order.
fn complete(pool: &Pool, epoch: u64) {
    let mut state = pool.lock();
    let index = epoch.saturating_sub(state.epoch_base) as usize;
    if let Some(epoch) = state.epochs.get_mut(index) {
        epoch.pending = epoch.pending.saturating_sub(1);
    }
    state.pending = state.pending.saturating_sub(1);
    if state.draining {
        return;
    }
    state.draining = true;
    loop {
        let mut ready: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
        while state.epochs.len() > 1
            && state
                .epochs
                .front()
                .map(|epoch| epoch.pending == 0)
                .unwrap_or(false)
        {
            if let Some(front) = state.epochs.pop_front() {
                ready.extend(front.callbacks);
            }
            state.epoch_base += 1;
        }
        if ready.is_empty() {
            break;
        }
        drop(state);
        for callback in ready {
            callback();
        }
        state = pool.lock();
    }
    state.draining = false;
}

/// True when the pool exists and parallel copies are not toggled off.
pub fn enabled() -> bool {
    pool().is_some() && !bbport_toggles::disabled(bbport_toggles::PARALLEL_COPIES)
}

/// Starts `task` on a copy thread now (inline when the pool is disabled).
pub fn async_task(task: Box<dyn FnOnce() + Send>) {
    let Some(pool) = pool() else {
        task();
        return;
    };
    if !enabled() {
        task();
        return;
    }
    let mut state = pool.lock();
    let epoch = state.epoch_base + state.epochs.len().saturating_sub(1) as u64;
    if let Some(back) = state.epochs.back_mut() {
        back.pending += 1;
    }
    state.pending += 1;
    state.tasks.push_back(Task { run: task, epoch });
    drop(state);
    pool.cv.notify_one();
}

/// Starts the calling thread's batch.
pub fn flush_batch() {
    let items = BATCH.with(|batch| std::mem::take(&mut *batch.borrow_mut()));
    if items.is_empty() {
        return;
    }
    let items = SendItems(items);
    async_task(Box::new(move || {
        let items = items;
        for item in items.0.iter() {
            (item.run)(item);
        }
    }));
}

/// Starts the calling thread's batch, then waits until every task passed to `async_task` so far
/// has finished.
pub fn wait_async() {
    // The caller's own batch runs here: handing it over only to wait for it costs a wakeup.
    let items = BATCH.with(|batch| std::mem::take(&mut *batch.borrow_mut()));
    for item in &items {
        (item.run)(item);
    }
    let Some(pool) = pool() else {
        return;
    };
    if !enabled() {
        return;
    }
    loop {
        let task = {
            let mut state = pool.lock();
            if state.pending == 0 && state.epochs.len() <= 1 && !state.draining {
                return;
            }
            state.tasks.pop_front()
        };
        match task {
            Some(task) => run_task(pool, task),
            None => std::thread::yield_now(),
        }
    }
}

/// Runs `callback` once every task passed to `async_task` so far has finished, in order with
/// earlier callbacks. Runs at once when none is pending.
pub fn after_copies(callback: Box<dyn FnOnce() + Send>) {
    flush_batch();
    let Some(pool) = pool() else {
        callback();
        return;
    };
    if !enabled() {
        callback();
        return;
    }
    let mut state = pool.lock();
    let should_defer = state.epochs.len() > 1
        || state
            .epochs
            .back()
            .map(|epoch| epoch.pending != 0)
            .unwrap_or(false)
        || state.draining;
    if should_defer {
        if let Some(back) = state.epochs.back_mut() {
            back.callbacks.push(callback);
        }
        state.epochs.push_back(Epoch::default());
        return;
    }
    drop(state);
    callback();
}

/// Queues a small copy in the calling thread's batch; the batch is started with `async_task`
/// once it holds about 512 KiB or 256 items, or by `flush_batch()`/`wait_async()`.
pub fn queue_copy(item: Item) {
    if !enabled() {
        (item.run)(&item);
        return;
    }
    // Smaller batches (32 items / 64 KiB, done before the next fence) measured slower.
    let flush = BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();
        batch.push(item);
        let bytes: u64 = batch.iter().map(|item| item.size).sum();
        bytes >= BATCH_BYTES || batch.len() >= BATCH_ITEMS
    });
    if flush {
        flush_batch();
    }
}

fn in_copy_thread() -> bool {
    IN_COPY_THREAD.with(|flag| flag.get())
}

/// Runs `task(i)` for every `i` in `0..count`, inline for a single item, one worker, or a copy
/// thread (no nesting).
pub fn parallel_for(count: usize, task: &(dyn Fn(usize) + Sync)) {
    let workers = (crate::gpu::shim::bbport_threads::available() as usize)
        .min(count)
        .max(1);
    if count <= 1 || in_copy_thread() || !enabled() || workers == 1 {
        for index in 0..count {
            task(index);
        }
        return;
    }
    std::thread::scope(|scope| {
        for worker in 0..workers {
            let task = &task;
            scope.spawn(move || {
                for index in (worker..count).step_by(workers) {
                    task(index);
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    static COPIED: AtomicUsize = AtomicUsize::new(0);

    fn count_item(_item: &Item) {
        COPIED.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn async_tasks_finish_before_wait_async_returns() {
        let done = Arc::new(AtomicBool::new(false));
        {
            let done = Arc::clone(&done);
            async_task(Box::new(move || done.store(true, Ordering::Release)));
        }
        wait_async();
        assert!(done.load(Ordering::Acquire));
    }

    #[test]
    fn queued_copies_run_by_wait_async() {
        COPIED.store(0, Ordering::Relaxed);
        for _ in 0..4 {
            queue_copy(Item {
                run: count_item,
                context: std::ptr::null_mut(),
                source: 0,
                destination: 0,
                size: 4096,
                extra: 0,
            });
        }
        wait_async();
        assert_eq!(COPIED.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn after_copies_runs_inline_when_nothing_is_pending() {
        wait_async();
        let ran = Arc::new(AtomicBool::new(false));
        {
            let ran = Arc::clone(&ran);
            after_copies(Box::new(move || ran.store(true, Ordering::Release)));
        }
        wait_async();
        assert!(ran.load(Ordering::Acquire));
    }

    #[test]
    fn parallel_for_visits_every_index_once() {
        let count = 100;
        let visits = AtomicUsize::new(0);
        parallel_for(count, &|_| {
            visits.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(visits.load(Ordering::Relaxed), count);
    }
}
