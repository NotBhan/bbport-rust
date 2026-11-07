//! The Hunter's Dream `sceNpSignaling` backend, ported from `bbnet/thd/thd_np.cpp`.

use std::collections::HashMap;

pub const SIG_EVENT_DEAD: u16 = 0x00;
pub const SIG_EVENT_ESTABLISHED: u16 = 0x01;
pub const SIG_EVENT_ACTIVE: u16 = 0x0c;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThdConnection {
    pub ctx: u32,
    pub online_id: String,
    pub member_id: u16,
    pub announced: bool,
    pub dead: bool,
    pub peer_addr: u32,
    pub peer_port: u16,
}

/// `thd_np.cpp` signaling state.
#[derive(Debug, Default)]
pub struct ThdNpState {
    pub next_ctx: u32,
    pub next_conn: u32,
    pub contexts: HashMap<u32, usize>,
    pub connections: HashMap<u32, ThdConnection>,
}

impl ThdNpState {
    pub fn create_context(&mut self) -> u32 {
        self.next_ctx += 1;
        self.contexts.insert(self.next_ctx, 0);
        self.next_ctx
    }

    pub fn delete_context(&mut self, ctx: u32) -> i32 {
        if self.contexts.remove(&ctx).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn activate(&mut self, ctx: u32, online_id: &str, addr: u32, port: u16) -> Option<u32> {
        if !self.contexts.contains_key(&ctx) {
            return None;
        }
        self.next_conn += 1;
        self.connections.insert(
            self.next_conn,
            ThdConnection {
                ctx,
                online_id: online_id.to_string(),
                peer_addr: addr,
                peer_port: port,
                ..ThdConnection::default()
            },
        );
        Some(self.next_conn)
    }

    pub fn announce(&mut self, conn: u32) -> i32 {
        match self.connections.get_mut(&conn) {
            Some(connection) if !connection.announced => {
                connection.announced = true;
                0
            }
            _ => -1,
        }
    }

    pub fn deactivate(&mut self, conn: u32) -> i32 {
        match self.connections.get_mut(&conn) {
            Some(connection) => {
                connection.dead = true;
                0
            }
            None => -1,
        }
    }

    pub fn active_connection_for_peer(&self, online_id: &str) -> Option<u32> {
        self.connections
            .iter()
            .find(|(_, connection)| connection.online_id == online_id && !connection.dead)
            .map(|(id, _)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thd_np_activates_and_announces_connections() {
        let mut np = ThdNpState::default();
        let ctx = np.create_context();
        let conn = np.activate(ctx, "peer", 1, 2).unwrap();
        assert_eq!(np.active_connection_for_peer("peer"), Some(conn));
        assert_eq!(np.announce(conn), 0);
        assert_eq!(np.announce(conn), -1);
        assert_eq!(np.deactivate(conn), 0);
        assert!(np.active_connection_for_peer("peer").is_none());
        assert_eq!(SIG_EVENT_ACTIVE, 0x0c);
    }
}
