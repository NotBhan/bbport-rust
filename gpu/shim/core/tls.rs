//! Co-op TLS hook, ported from `legacy/gpu/shim/core/tls.h`.
//!
//! The header only documents that the NP libraries include it while guest threads get their
//! TLS from the C runtime; there is no code to port yet. Kept so the module tree mirrors the
//! legacy layout.
