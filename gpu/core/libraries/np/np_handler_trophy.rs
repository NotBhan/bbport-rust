//! Trophy mirroring, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_trophy.cpp`.

use std::collections::HashMap;

use crate::gpu::bbnet::shadnet::client::{CommandType, ErrorType};

/// The client calls trophy mirroring needs.
pub trait TrophyClient {
    fn is_authenticated(&self) -> bool;
    fn trophies_enabled(&self) -> bool;
    fn unlock_trophy(&mut self, com_id: &str, trophy_id: i32, timestamp: u64) -> u64;
    fn sync_trophies(&mut self, com_id: &str, local: &[(i32, u64)]) -> u64;
}

pub type MergedCallback = Box<dyn FnOnce(&[(i32, u64)])>;

/// `NpHandler` trophy state.
#[derive(Default)]
pub struct TrophyHandler {
    pub comm_ids: HashMap<i32, String>,
    pub pending: HashMap<u64, MergedCallback>,
}

impl TrophyHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_comm_id(&mut self, service_label: i32, com_id: &str) {
        self.comm_ids.insert(service_label, com_id.to_string());
    }

    pub fn comm_id(&self, service_label: i32) -> Option<&str> {
        self.comm_ids
            .get(&service_label)
            .map(String::as_str)
            .filter(|com_id| !com_id.is_empty())
    }

    /// `ReportTrophyUnlock`.
    pub fn report_trophy_unlock(
        &mut self,
        client: Option<&mut dyn TrophyClient>,
        user_id: i32,
        service_label: i32,
        trophy_id: i32,
        timestamp: u64,
    ) -> Option<u64> {
        let _ = user_id;
        let client = client?;
        if !client.is_authenticated() || !client.trophies_enabled() {
            return None;
        }
        let com_id = self.comm_id(service_label)?.to_string();
        Some(client.unlock_trophy(&com_id, trophy_id, timestamp))
    }

    /// `SyncTrophies`; the callback runs when the reply arrives.
    pub fn sync_trophies(
        &mut self,
        client: Option<&mut dyn TrophyClient>,
        service_label: i32,
        local_trophies: &[(i32, u64)],
        on_merged: Option<MergedCallback>,
    ) -> Option<u64> {
        let client = client?;
        if !client.is_authenticated() || !client.trophies_enabled() {
            return None;
        }
        let com_id = self.comm_id(service_label)?.to_string();
        let packet_id = client.sync_trophies(&com_id, local_trophies);
        if let Some(on_merged) = on_merged {
            self.pending.insert(packet_id, on_merged);
        }
        Some(packet_id)
    }

    /// `OnTrophyReply`.
    pub fn on_trophy_reply(
        &mut self,
        command: CommandType,
        packet_id: u64,
        error: ErrorType,
        body: &[u8],
    ) {
        if command == CommandType::UnlockTrophy {
            return;
        }
        if command != CommandType::SyncTrophies {
            return;
        }
        let Some(on_merged) = self.pending.remove(&packet_id) else {
            return;
        };
        if error != ErrorType::NoError {
            return;
        }
        if let Some(merged) = parse_sync_reply(body) {
            on_merged(&merged);
        }
    }
}

fn read_varint(data: &[u8], position: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0;
    while *position < data.len() {
        let byte = data[*position];
        *position += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

fn parse_trophy(data: &[u8]) -> Option<(i32, u64)> {
    let mut position = 0;
    let mut trophy_id = None;
    let mut timestamp = 0;
    while position < data.len() {
        let key = read_varint(data, &mut position)?;
        match key >> 3 {
            1 => trophy_id = Some(read_varint(data, &mut position)? as i32),
            2 => timestamp = read_varint(data, &mut position)?,
            _ => return None,
        }
    }
    Some((trophy_id?, timestamp))
}

/// Parses the u32-LE blob size and `SyncTrophiesReply` repeated trophies.
pub fn parse_sync_reply(body: &[u8]) -> Option<Vec<(i32, u64)>> {
    if body.len() < 4 {
        return None;
    }
    let blob_size = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize;
    if body.len() < 4 + blob_size {
        return None;
    }
    let blob = &body[4..4 + blob_size];
    let mut position = 0;
    let mut trophies = Vec::new();
    while position < blob.len() {
        let key = read_varint(blob, &mut position)?;
        if key >> 3 != 1 || key & 7 != 2 {
            return None;
        }
        let length = read_varint(blob, &mut position)? as usize;
        if position + length > blob.len() {
            return None;
        }
        trophies.push(parse_trophy(&blob[position..position + length])?);
        position += length;
    }
    Some(trophies)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Client {
        authenticated: bool,
        enabled: bool,
    }

    impl TrophyClient for Client {
        fn is_authenticated(&self) -> bool {
            self.authenticated
        }
        fn trophies_enabled(&self) -> bool {
            self.enabled
        }
        fn unlock_trophy(&mut self, _: &str, _: i32, _: u64) -> u64 {
            7
        }
        fn sync_trophies(&mut self, _: &str, _: &[(i32, u64)]) -> u64 {
            8
        }
    }

    fn reply(trophies: &[(i32, u64)]) -> Vec<u8> {
        let mut blob = Vec::new();
        for (id, timestamp) in trophies {
            let message = vec![0x08, *id as u8, 0x10, *timestamp as u8];
            blob.push(0x0a);
            blob.push(message.len() as u8);
            blob.extend_from_slice(&message);
        }
        let mut body = (blob.len() as u32).to_le_bytes().to_vec();
        body.extend_from_slice(&blob);
        body
    }

    #[test]
    fn trophies_are_mirrored_only_when_authenticated_and_supported() {
        let mut handler = TrophyHandler::new();
        handler.set_comm_id(1, "NPWR12345_00");
        let mut client = Client {
            authenticated: false,
            enabled: true,
        };
        assert!(handler
            .report_trophy_unlock(Some(&mut client), 1, 1, 3, 4)
            .is_none());
        client.authenticated = true;
        client.enabled = false;
        assert!(handler
            .report_trophy_unlock(Some(&mut client), 1, 1, 3, 4)
            .is_none());
        client.enabled = true;
        assert_eq!(
            handler.report_trophy_unlock(Some(&mut client), 1, 1, 3, 4),
            Some(7)
        );
        assert!(handler
            .report_trophy_unlock(Some(&mut client), 1, 2, 3, 4)
            .is_none());
        let merged = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = merged.clone();
        let id = handler.sync_trophies(
            Some(&mut client),
            1,
            &[(3, 4)],
            Some(Box::new(move |trophies| {
                *captured.lock().unwrap() = trophies.to_vec()
            })),
        );
        assert_eq!(id, Some(8));
        handler.on_trophy_reply(
            CommandType::SyncTrophies,
            8,
            ErrorType::NoError,
            &reply(&[(3, 9)]),
        );
        assert_eq!(*merged.lock().unwrap(), vec![(3, 9)]);
    }
}
