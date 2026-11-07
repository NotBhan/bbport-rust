//! Bloodborne summon helpers from shadp2p, ported from
//! `legacy/gpu/shim/core/bloodborne_re.h`. The header value is kept between requests.

use std::sync::{Mutex, OnceLock};

fn storage() -> &'static Mutex<Option<String>> {
    static VALUE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    VALUE.get_or_init(|| Mutex::new(None))
}

/// The last seamless-host placement header, if any.
pub fn get_seamless_host_placement_header() -> Option<String> {
    storage().lock().ok().and_then(|value| value.clone())
}

/// Store the header value; `false` for an empty value.
pub fn set_seamless_host_placement_header(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    match storage().lock() {
        Ok(mut slot) => {
            *slot = Some(value.to_string());
            true
        }
        Err(_) => false,
    }
}

pub fn clear_seamless_host_placement_header() {
    if let Ok(mut slot) = storage().lock() {
        *slot = None;
    }
}

/// Diagnostic hook for `Matching2::LeaveRoom`; the port only logs it.
pub fn trace_matching2_leave_room(_return_address: usize, _room_id: u64) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_value_round_trips_and_clears() {
        clear_seamless_host_placement_header();
        assert_eq!(get_seamless_host_placement_header(), None);
        assert!(set_seamless_host_placement_header("host:1234"));
        assert_eq!(
            get_seamless_host_placement_header().as_deref(),
            Some("host:1234")
        );
        assert!(!set_seamless_host_placement_header(""));
        clear_seamless_host_placement_header();
        assert_eq!(get_seamless_host_placement_header(), None);
    }
}
