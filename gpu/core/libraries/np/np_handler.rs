//! NpHandler state surface, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler.h`.

use std::collections::HashMap;

/// A per-user client record.
#[derive(Debug, Clone, Default)]
pub struct HandlerClient {
    pub npid: String,
    pub authenticated: bool,
    pub trophies_enabled: bool,
}

/// `NpHandler` host state used by the NP libraries.
#[derive(Debug, Default)]
pub struct NpHandler {
    pub clients: HashMap<i32, HandlerClient>,
    pub np_ids: HashMap<i32, [u8; 16]>,
    pub pending_invites: HashMap<i32, Vec<String>>,
}

impl NpHandler {
    pub fn add_client(&mut self, user_id: i32, npid: &str) {
        self.clients.insert(
            user_id,
            HandlerClient {
                npid: npid.to_string(),
                authenticated: true,
                trophies_enabled: false,
            },
        );
        let mut online_id = [0u8; 16];
        let count = npid.len().min(16);
        online_id[..count].copy_from_slice(&npid.as_bytes()[..count]);
        self.np_ids.insert(user_id, online_id);
    }

    pub fn get_client(&self, user_id: i32) -> Option<&HandlerClient> {
        self.clients.get(&user_id)
    }

    pub fn authenticated(&self, user_id: i32) -> bool {
        self.clients
            .get(&user_id)
            .is_some_and(|client| client.authenticated)
    }

    pub fn trophies_enabled(&self, user_id: i32) -> bool {
        self.clients
            .get(&user_id)
            .is_some_and(|client| client.trophies_enabled)
    }

    pub fn get_comm_id(&self, service_label: i32) -> String {
        format!("NPWR{service_label:05}_00")
    }

    pub fn is_valid_comm_id(comm_id: &str) -> bool {
        comm_id.starts_with("NPWR") && comm_id.ends_with("_00")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_tracks_clients_and_comm_ids() {
        let mut handler = NpHandler::default();
        assert!(!handler.authenticated(1));
        handler.add_client(1, "hunter");
        assert!(handler.authenticated(1));
        assert!(!handler.trophies_enabled(1));
        assert_eq!(handler.get_client(1).unwrap().npid, "hunter");
        let comm_id = handler.get_comm_id(12345);
        assert!(NpHandler::is_valid_comm_id(&comm_id));
        assert!(!NpHandler::is_valid_comm_id("bad"));
    }
}

/// The process-wide handler instance used by the cpp.
#[derive(Debug, Default)]
pub struct NpHandlerGlobal {
    pub handler: NpHandler,
    pub next_request: u64,
    pub callbacks: Vec<usize>,
}

impl NpHandlerGlobal {
    pub fn initialize(&mut self) -> i32 {
        self.next_request = 1;
        0
    }

    pub fn alloc_request(&mut self) -> u64 {
        let id = self.next_request;
        self.next_request += 1;
        id
    }

    pub fn register_callback(&mut self, callback: usize) -> usize {
        self.callbacks.push(callback);
        self.callbacks.len() - 1
    }

    pub fn get_account_id(&self, user_id: i32) -> u64 {
        let _ = user_id;
        0
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn handler_global_allocates_requests_and_callbacks() {
        let mut global = NpHandlerGlobal::default();
        assert_eq!(global.initialize(), 0);
        assert_eq!(global.alloc_request(), 1);
        assert_eq!(global.register_callback(7), 0);
        assert_eq!(global.callbacks, vec![7]);
    }
}
