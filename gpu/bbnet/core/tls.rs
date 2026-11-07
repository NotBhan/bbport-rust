//! Guest thread-local storage, ported from `legacy/gpu/bbnet/include/core/tls.h`.
//!
//! The C++ header has no declarations: the NP libraries get their guest TLS block from the C
//! runtime's own thread control. The Rust port keeps the same contract with a real
//! `thread_local!`, so a guest thread's block follows the host thread that runs it.

use std::cell::Cell;

thread_local! {
    /// The guest's TLS anchor for the running host thread (the `fs`/`gs` base in the C build).
    static GUEST_TLS: Cell<u8> = const { Cell::new(0) };
}

/// Runs `body` with the running thread's guest TLS anchor.
pub fn with_guest_tls<R>(body: impl FnOnce(&Cell<u8>) -> R) -> R {
    GUEST_TLS.with(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tls_anchor_is_per_thread() {
        with_guest_tls(|cell| cell.set(7));
        let other = std::thread::spawn(|| with_guest_tls(|cell| cell.get()));
        assert_eq!(other.join().unwrap(), 0, "a new host thread starts clean");
        assert_eq!(with_guest_tls(|cell| cell.get()), 7);
    }
}
