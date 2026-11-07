//! Two-stage draw pipeline ring, ported from
//! `shadps4/video_core/renderer_vulkan/vk_draw_pipe.h`.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

const CAPACITY: usize = 1 << 20;
const HEADER_SIZE: u64 = 8;

#[derive(Debug)]
struct RingState {
    data: Vec<u8>,
    head: u64,
    published: u64,
    consumed: u64,
    packets: u64,
    sleeping: bool,
    stop: bool,
}

type Handler = Box<dyn Fn(&[u8]) + Send + Sync>;

/// `DrawPipe`.
pub struct DrawPipe {
    state: Arc<(Mutex<RingState>, Condvar, Condvar)>,
    handler: Arc<Handler>,
    thread: Option<JoinHandle<()>>,
    pending_size: u64,
}

impl DrawPipe {
    pub fn new(handler: Handler) -> Self {
        let state = Arc::new((
            Mutex::new(RingState {
                data: vec![0; CAPACITY],
                head: 0,
                published: 0,
                consumed: 0,
                packets: 0,
                sleeping: false,
                stop: false,
            }),
            Condvar::new(),
            Condvar::new(),
        ));
        let worker_state = state.clone();
        let worker_handler = Arc::new(handler);
        let worker_handler_clone = worker_handler.clone();
        let thread = thread::spawn(move || run(worker_state, worker_handler_clone));
        Self {
            state,
            handler: worker_handler,
            thread: Some(thread),
            pending_size: 0,
        }
    }

    /// Stage A: reserves `size` payload bytes (8-byte aligned) and returns their offset.
    pub fn begin(&mut self, size: u32) -> usize {
        let total = align(u64::from(size) + HEADER_SIZE);
        assert!(total as usize <= CAPACITY / 4);
        let (lock, space_cv, _) = &*self.state;
        let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
        let mut at = ring.head;
        let offset = at % CAPACITY as u64;
        if offset + total > CAPACITY as u64 {
            let skip = CAPACITY as u64 - offset;
            while at + skip - ring.consumed > CAPACITY as u64 {
                ring = space_cv
                    .wait_timeout(ring, std::time::Duration::from_millis(1))
                    .unwrap_or_else(|error| error.into_inner())
                    .0;
            }
            write_header(&mut ring.data, offset as usize, 0, 0);
            at += skip;
            ring.head = at;
            ring.published = at;
            drop(ring);
            return self.begin(size);
        }
        while at + total - ring.consumed > CAPACITY as u64 {
            ring = space_cv
                .wait_timeout(ring, std::time::Duration::from_millis(1))
                .unwrap_or_else(|error| error.into_inner())
                .0;
        }
        write_header(&mut ring.data, offset as usize, total as u32, 0);
        self.pending_size = total;
        ring.head = at;
        ((offset + HEADER_SIZE) % CAPACITY as u64) as usize
    }

    /// Writes payload bytes into the slot returned by `begin`.
    pub fn write(&mut self, offset: usize, data: &[u8]) {
        let (lock, _, _) = &*self.state;
        let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
        let at = offset % CAPACITY;
        debug_assert!(at + data.len() <= CAPACITY);
        ring.data[at..at + data.len()].copy_from_slice(data);
    }

    /// Stage A: hands the packet to stage B.
    pub fn commit(&mut self, payload_size: u32) {
        let (lock, _, wake_cv) = &*self.state;
        let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
        let header_at = (ring.head % CAPACITY as u64) as usize;
        write_header(
            &mut ring.data,
            header_at,
            self.pending_size as u32,
            payload_size,
        );
        ring.head += self.pending_size;
        ring.published = ring.head;
        ring.packets += 1;
        if ring.sleeping {
            ring.sleeping = false;
        }
        drop(ring);
        wake_cv.notify_one();
    }

    pub fn packets(&self) -> u64 {
        self.state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .packets
    }

    /// Stage A waits until stage B consumed everything published.
    pub fn drain(&mut self) {
        let (lock, space_cv, _) = &*self.state;
        let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
        while ring.consumed != ring.published {
            ring = space_cv
                .wait_timeout(ring, std::time::Duration::from_millis(1))
                .unwrap_or_else(|error| error.into_inner())
                .0;
        }
    }
}

impl Drop for DrawPipe {
    fn drop(&mut self) {
        self.drain();
        {
            let (lock, _, wake_cv) = &*self.state;
            let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
            ring.stop = true;
            drop(ring);
            wake_cv.notify_one();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = &self.handler;
    }
}

fn run(state: Arc<(Mutex<RingState>, Condvar, Condvar)>, handler: Arc<Handler>) {
    let (lock, space_cv, wake_cv) = &*state;
    loop {
        let packet = {
            let mut ring = lock.lock().unwrap_or_else(|error| error.into_inner());
            while ring.consumed == ring.published && !ring.stop {
                ring.sleeping = true;
                let (next, timeout) = wake_cv
                    .wait_timeout(ring, std::time::Duration::from_millis(5))
                    .unwrap_or_else(|error| error.into_inner());
                ring = next;
                if timeout.timed_out() {
                    ring.sleeping = false;
                }
            }
            if ring.stop && ring.consumed == ring.published {
                return;
            }
            let offset = (ring.consumed % CAPACITY as u64) as usize;
            let (size, payload) = read_header(&ring.data, offset);
            if size == 0 {
                ring.consumed += CAPACITY as u64 - offset as u64;
                space_cv.notify_one();
                continue;
            }
            let payload_start = (offset + HEADER_SIZE as usize) % CAPACITY;
            let payload = payload as usize;
            let data = if payload_start + payload <= CAPACITY {
                ring.data[payload_start..payload_start + payload].to_vec()
            } else {
                let first = CAPACITY - payload_start;
                let mut data = ring.data[payload_start..].to_vec();
                data.extend_from_slice(&ring.data[..payload - first]);
                data
            };
            ring.consumed += size as u64;
            space_cv.notify_one();
            data
        };
        handler(&packet);
    }
}

fn align(size: u64) -> u64 {
    (size + 7) & !7
}

fn write_header(data: &mut [u8], at: usize, size: u32, payload: u32) {
    data[at..at + 4].copy_from_slice(&size.to_le_bytes());
    data[at + 4..at + 8].copy_from_slice(&payload.to_le_bytes());
}

fn read_header(data: &[u8], at: usize) -> (u32, u32) {
    let size = u32::from_le_bytes(data[at..at + 4].try_into().expect("header size"));
    let payload = u32::from_le_bytes(data[at + 4..at + 8].try_into().expect("header payload"));
    (size, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn packets_cross_the_ring_in_order() {
        let (tx, rx) = mpsc::channel();
        let mut pipe = DrawPipe::new(Box::new(move |packet| {
            tx.send(packet.to_vec()).expect("receiver");
        }));
        let offset = pipe.begin(4);
        pipe.write(offset, &[1, 2, 3, 4]);
        pipe.commit(4);
        let offset = pipe.begin(2);
        pipe.write(offset, &[9, 8]);
        pipe.commit(2);
        pipe.drain();
        assert_eq!(rx.recv().expect("first"), vec![1, 2, 3, 4]);
        assert_eq!(rx.recv().expect("second"), vec![9, 8]);
        assert_eq!(pipe.packets(), 2);
    }
}
