//! Incrementing ids, ported from `legacy/gpu/shadps4/common/incremental_id.h`.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// `Common::IncrementalIdProvider<u64>`: the first id a provider hands out is 1.
#[derive(Debug, Default)]
pub struct IncrementalIdProvider64 {
    counter: AtomicU64,
}

impl IncrementalIdProvider64 {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// `Common::IncrementalIdProvider<u32>`.
#[derive(Debug, Default)]
pub struct IncrementalIdProvider32 {
    counter: AtomicU32,
}

impl IncrementalIdProvider32 {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }

    pub fn next(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::Relaxed) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn ids_start_at_one_and_never_repeat_across_threads() {
        assert_eq!(IncrementalIdProvider32::new().next(), 1);
        assert_eq!(IncrementalIdProvider64::new().next(), 1);
        let provider = Arc::new(IncrementalIdProvider32::new());
        let mut threads = Vec::new();
        for _ in 0..4 {
            let provider = Arc::clone(&provider);
            threads.push(std::thread::spawn(move || {
                (0..250).map(|_| provider.next()).collect::<Vec<_>>()
            }));
        }
        let mut ids: Vec<u32> = threads
            .into_iter()
            .flat_map(|thread| thread.join().unwrap())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 1000, "every id is handed out once");
        assert_eq!(ids[0], 1);
    }
}
