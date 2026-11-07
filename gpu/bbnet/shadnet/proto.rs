//! shadNet wire protocol, ported from `bbnet/shadnet/shadnet.proto` and the client framing.

use crate::gpu::bbnet::shadnet::client::{PacketType, SHAD_HEADER_SIZE};

/// The 15-byte packet header.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PacketHeader {
    pub packet_type: u8,
    pub command: u16,
    pub packet_id: u64,
    pub total_size: u32,
}

impl PacketHeader {
    /// `BuildPacket`; encodes the header little-endian.
    pub fn encode(&self) -> [u8; SHAD_HEADER_SIZE as usize] {
        let mut out = [0u8; SHAD_HEADER_SIZE as usize];
        out[0] = self.packet_type;
        out[1..3].copy_from_slice(&self.command.to_le_bytes());
        out[3..11].copy_from_slice(&self.packet_id.to_le_bytes());
        out[11..15].copy_from_slice(&self.total_size.to_le_bytes());
        out
    }

    /// `GetLE16/32/64` decode.
    pub fn decode(bytes: &[u8; SHAD_HEADER_SIZE as usize]) -> Self {
        Self {
            packet_type: bytes[0],
            command: u16::from_le_bytes([bytes[1], bytes[2]]),
            packet_id: u64::from_le_bytes([
                bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10],
            ]),
            total_size: u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
        }
    }

    pub fn packet_type(&self) -> Option<PacketType> {
        match self.packet_type {
            0 => Some(PacketType::Request),
            1 => Some(PacketType::Reply),
            2 => Some(PacketType::Notification),
            3 => Some(PacketType::ServerInfo),
            _ => None,
        }
    }
}

/// `ExtractBlob`; a u32-LE length followed by the bytes.
pub fn extract_blob(data: &[u8], position: &mut usize) -> Option<Vec<u8>> {
    if *position + 4 > data.len() {
        return None;
    }
    let length = u32::from_le_bytes(data[*position..*position + 4].try_into().ok()?) as usize;
    *position += 4;
    if *position + length > data.len() {
        return None;
    }
    let blob = data[*position..*position + length].to_vec();
    *position += length;
    Some(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_headers_round_trip_and_blobs_extract() {
        let header = PacketHeader {
            packet_type: PacketType::Request as u8,
            command: 201,
            packet_id: 0x1234,
            total_size: 15,
        };
        let bytes = header.encode();
        assert_eq!(bytes.len(), 15);
        assert_eq!(PacketHeader::decode(&bytes), header);
        assert_eq!(header.packet_type(), Some(PacketType::Request));
        let data = [3, 0, 0, 0, 0xaa, 0xbb, 0xcc];
        let mut position = 0;
        assert_eq!(
            extract_blob(&data, &mut position),
            Some(vec![0xaa, 0xbb, 0xcc])
        );
        assert_eq!(position, 7);
        assert_eq!(extract_blob(&[1], &mut 0), None);
    }
}
