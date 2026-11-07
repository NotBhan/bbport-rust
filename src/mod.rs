//! Port of the C loader/runtime in `src/*.c` (milestone 3). The C sources are kept in
//! `legacy/src/` as the reference; each translation unit moves here one at a time behind the
//! C ABI.

pub mod runtime;
pub mod runtime_imports;
pub mod runtime_mutex;
pub mod runtime_rtc;
pub mod runtime_time;
