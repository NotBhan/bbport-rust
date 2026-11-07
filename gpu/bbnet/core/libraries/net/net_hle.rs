//! `sceNet*` kernel wrappers, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net.cpp`.

use std::collections::HashMap;

/// `Net` HLE socket table state.
#[derive(Debug, Default)]
pub struct NetHleState {
    pub initialized: bool,
    pub next_id: i32,
    pub sockets: HashMap<i32, (i32, i32, i32)>,
    pub epolls: HashMap<i32, Vec<i32>>,
    pub dns_cache_size: usize,
}

impl NetHleState {
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        self.next_id = 1;
        0
    }

    pub fn term(&mut self) -> i32 {
        self.initialized = false;
        self.sockets.clear();
        self.epolls.clear();
        0
    }

    pub fn socket(&mut self, family: i32, kind: i32, protocol: i32) -> i32 {
        if !self.initialized {
            return -1;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.sockets.insert(id, (family, kind, protocol));
        id
    }

    pub fn close(&mut self, id: i32) -> i32 {
        if self.sockets.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn epoll_create(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        self.epolls.insert(id, Vec::new());
        id
    }

    pub fn epoll_control(&mut self, eid: i32, socket: i32, add: bool) -> i32 {
        let Some(list) = self.epolls.get_mut(&eid) else {
            return -1;
        };
        if add {
            if !list.contains(&socket) {
                list.push(socket);
            }
        } else {
            list.retain(|entry| *entry != socket);
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_hle_tracks_sockets_and_epolls() {
        let mut net = NetHleState::default();
        assert_eq!(net.socket(2, 1, 0), -1);
        assert_eq!(net.init(), 0);
        let socket = net.socket(2, 1, 0);
        let epoll = net.epoll_create();
        assert_eq!(net.epoll_control(epoll, socket, true), 0);
        assert!(net.epolls[&epoll].contains(&socket));
        assert_eq!(net.close(socket), 0);
        assert_eq!(net.term(), 0);
    }
}
