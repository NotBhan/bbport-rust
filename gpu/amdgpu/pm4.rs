//! PM4 packet headers and opcodes, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/pm4_opcodes.h` and `pm4_cmds.h`.
//!
//! Header parsing is strict and length-checked: an unknown packet type or a packet that runs
//! past the buffer ends the stream with an error instead of reading out of bounds. An unknown
//! type-3 opcode still yields a packet (with `opcode: None`), so the caller can skip exactly
//! `num_words` and continue, matching the hardened loader.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Pm4Error {
    #[error("unsupported PM4 packet type {kind} at dword {offset}")]
    UnsupportedType { kind: u8, offset: usize },
    #[error("PM4 packet at dword {offset} has zero length")]
    ZeroLength { offset: usize },
    #[error("PM4 packet at dword {offset} needs {needed} dwords, only {remaining} remain")]
    Truncated {
        offset: usize,
        needed: u32,
        remaining: usize,
    },
}

macro_rules! opcode_enum {
    ($($variant:ident = $value:expr),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        pub enum PM4ItOpcode {
            $($variant = $value),+
        }

        impl PM4ItOpcode {
            pub fn from_raw(value: u32) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub const fn raw(self) -> u32 {
                self as u32
            }
        }
    };
}

impl PM4ItOpcode {
    /// `CondIndirectBuffer` shares the `IndirectBuffer` opcode value (0x3F) in the hardware.
    pub const COND_INDIRECT_BUFFER: Self = Self::IndirectBuffer;
}

opcode_enum! {
    Nop = 0x10,
    SetBase = 0x11,
    ClearState = 0x12,
    IndexBufferSize = 0x13,
    DispatchDirect = 0x15,
    DispatchIndirect = 0x16,
    AtomicGds = 0x1D,
    Atomic = 0x1E,
    OcclusionQuery = 0x1F,
    SetPredication = 0x20,
    RegRmw = 0x21,
    CondExec = 0x22,
    PredExec = 0x23,
    DrawIndirect = 0x24,
    DrawIndexIndirect = 0x25,
    IndexBase = 0x26,
    DrawIndex2 = 0x27,
    ContextControl = 0x28,
    IndexType = 0x2A,
    DrawIndirectMulti = 0x2C,
    DrawIndexAuto = 0x2D,
    NumInstances = 0x2F,
    DrawIndexMultiAuto = 0x30,
    IndirectBufferConst = 0x33,
    StrmoutBufferUpdate = 0x34,
    DrawIndexOffset2 = 0x35,
    WriteData = 0x37,
    DrawIndexIndirectMulti = 0x38,
    MemSemaphore = 0x39,
    WaitRegMem = 0x3C,
    IndirectBuffer = 0x3F,
    CopyData = 0x40,
    CommandProcessorDma = 0x41,
    PfpSyncMe = 0x42,
    SurfaceSync = 0x43,
    CondWrite = 0x45,
    EventWrite = 0x46,
    EventWriteEop = 0x47,
    EventWriteEos = 0x48,
    ReleaseMem = 0x49,
    PreambleCntl = 0x4A,
    DmaData = 0x50,
    ContextRegRmw = 0x51,
    AcquireMem = 0x58,
    Rewind = 0x59,
    LoadShReg = 0x5F,
    LoadConfigReg = 0x60,
    LoadContextReg = 0x61,
    SetConfigReg = 0x68,
    SetContextReg = 0x69,
    SetContextRegIndirect = 0x73,
    SetShReg = 0x76,
    SetShRegOffset = 0x77,
    SetQueueReg = 0x78,
    SetUconfigReg = 0x79,
    LoadConstRam = 0x80,
    WriteConstRam = 0x81,
    DumpConstRam = 0x83,
    IncrementCeCounter = 0x84,
    IncrementDeCounter = 0x85,
    WaitOnCeCounter = 0x86,
    WaitOnDeCounterDiff = 0x88,
    GetLodStats = 0x8E,
    DrawIndexIndirectCountMulti = 0x9D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PM4ShaderType {
    ShaderGraphics = 0,
    ShaderCompute = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PM4Predicate {
    PredDisable = 0,
    PredEnable = 1,
}

pub const PACKET_TYPE0: u8 = 0;
pub const PACKET_TYPE3: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type0Header {
    raw: u32,
}

impl Type0Header {
    /// Validate the packet type; the length is checked by [`PacketStream`].
    pub fn parse(raw: u32) -> Result<Self, Pm4Error> {
        if packet_type(raw) != PACKET_TYPE0 {
            return Err(Pm4Error::UnsupportedType {
                kind: packet_type(raw),
                offset: 0,
            });
        }
        Ok(Self { raw })
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    /// DWORD memory-mapped base address.
    pub const fn base(self) -> u16 {
        (self.raw & 0xffff) as u16
    }

    /// Count of DWORDs in the information body (14 bits).
    pub const fn count(self) -> u16 {
        ((self.raw >> 16) & 0x3fff) as u16
    }

    /// Header plus body, in DWORDs.
    pub const fn num_words(self) -> u32 {
        self.count() as u32 + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type3Header {
    raw: u32,
}

impl Type3Header {
    pub fn parse(raw: u32) -> Result<Self, Pm4Error> {
        if packet_type(raw) != PACKET_TYPE3 {
            return Err(Pm4Error::UnsupportedType {
                kind: packet_type(raw),
                offset: 0,
            });
        }
        Ok(Self { raw })
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn predicate(self) -> PM4Predicate {
        if self.raw & 1 != 0 {
            PM4Predicate::PredEnable
        } else {
            PM4Predicate::PredDisable
        }
    }

    pub const fn shader_type(self) -> PM4ShaderType {
        if self.raw & 2 != 0 {
            PM4ShaderType::ShaderCompute
        } else {
            PM4ShaderType::ShaderGraphics
        }
    }

    /// IT opcode (bits 8..16); `None` for an opcode this port does not know yet.
    pub fn opcode(self) -> Option<PM4ItOpcode> {
        PM4ItOpcode::from_raw((self.raw >> 8) & 0xff)
    }

    /// Number of DWORDs minus one in the information body (14 bits).
    pub const fn count(self) -> u16 {
        ((self.raw >> 16) & 0x3fff) as u16
    }

    /// Header plus body, in DWORDs (wraps to 0 for the malformed count `0x3fff`).
    pub const fn num_words(self) -> u32 {
        (self.count() as u32 + 1) & 0x3fff
    }
}

pub const fn packet_type(raw: u32) -> u8 {
    ((raw >> 30) & 0x3) as u8
}

/// Packets the GPU command thread handles while the draw recording thread may still work on
/// earlier draws. Every other packet first waits for that thread to drain.
pub fn pipelined_opcode(opcode: PM4ItOpcode) -> bool {
    use PM4ItOpcode::*;
    matches!(
        opcode,
        Nop | ContextControl
            | ClearState
            | SetConfigReg
            | SetContextReg
            | SetShReg
            | SetUconfigReg
            | IndexType
            | NumInstances
            | IndexBase
            | IndexBufferSize
            | SetBase
            | DrawIndex2
            | DrawIndexOffset2
            | DrawIndexAuto
            | IndirectBuffer
            | AcquireMem
            | PfpSyncMe
            | IncrementDeCounter
            | WaitOnCeCounter
            | EventWrite
            | EventWriteEop
            | EventWriteEos
            | WaitRegMem
            | DispatchDirect
            | DrawIndirect
            | DrawIndirectMulti
            | DrawIndexIndirect
            | DrawIndexIndirectMulti
            | DrawIndexIndirectCountMulti
            | DispatchIndirect
            | WriteData
            | DmaData
    )
}

/// One decoded packet, borrowing its body from the command buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Packet<'a> {
    Type0 {
        header: Type0Header,
        body: &'a [u32],
    },
    Type3 {
        header: Type3Header,
        opcode: Option<PM4ItOpcode>,
        body: &'a [u32],
    },
}

impl Packet<'_> {
    pub fn num_words(&self) -> u32 {
        match self {
            Self::Type0 { header, .. } => header.num_words(),
            Self::Type3 { header, .. } => header.num_words(),
        }
    }
}

/// Iterates PM4 packets, validating each header and length.
pub struct PacketStream<'a> {
    data: &'a [u32],
    offset: usize,
}

impl<'a> PacketStream<'a> {
    pub fn new(data: &'a [u32]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<'a> Iterator for PacketStream<'a> {
    type Item = Result<Packet<'a>, Pm4Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        let start = self.offset;
        let raw = self.data[start];
        let kind = packet_type(raw);
        let (words, packet) = match kind {
            PACKET_TYPE0 => {
                let header = Type0Header { raw };
                let words = header.num_words();
                (words, PacketKind::Type0(header))
            }
            PACKET_TYPE3 => {
                let header = Type3Header { raw };
                let words = header.num_words();
                (words, PacketKind::Type3(header))
            }
            other => {
                self.offset = self.data.len();
                return Some(Err(Pm4Error::UnsupportedType {
                    kind: other,
                    offset: start,
                }));
            }
        };
        if words == 0 {
            self.offset = self.data.len();
            return Some(Err(Pm4Error::ZeroLength { offset: start }));
        }
        let end = start.saturating_add(words as usize);
        if end > self.data.len() {
            self.offset = self.data.len();
            return Some(Err(Pm4Error::Truncated {
                offset: start,
                needed: words,
                remaining: self.data.len() - start,
            }));
        }
        let body = &self.data[start + 1..end];
        self.offset = end;
        Some(Ok(match packet {
            PacketKind::Type0(header) => Packet::Type0 { header, body },
            PacketKind::Type3(header) => Packet::Type3 {
                header,
                opcode: header.opcode(),
                body,
            },
        }))
    }
}

enum PacketKind {
    Type0(Type0Header),
    Type3(Type3Header),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type3(opcode: u32, count: u32) -> u32 {
        (3 << 30) | (opcode << 8) | (count << 16)
    }

    fn type0(base: u32, count: u32) -> u32 {
        base | (count << 16)
    }

    #[test]
    fn type3_header_fields_decode() {
        let header = Type3Header::parse(type3(PM4ItOpcode::DrawIndex2.raw(), 4)).unwrap();
        assert_eq!(header.opcode(), Some(PM4ItOpcode::DrawIndex2));
        assert_eq!(header.count(), 4);
        assert_eq!(header.num_words(), 5);
        assert_eq!(header.predicate(), PM4Predicate::PredDisable);
        assert_eq!(header.shader_type(), PM4ShaderType::ShaderGraphics);

        let compute = Type3Header::parse((3 << 30) | (0x15 << 8) | 2 | 3).unwrap();
        assert_eq!(compute.predicate(), PM4Predicate::PredEnable);
        assert_eq!(compute.shader_type(), PM4ShaderType::ShaderCompute);
        assert_eq!(compute.opcode(), Some(PM4ItOpcode::DispatchDirect));
    }

    #[test]
    fn type0_header_fields_decode() {
        let header = Type0Header::parse(type0(0x1a2b, 3)).unwrap();
        assert_eq!(header.base(), 0x1a2b);
        assert_eq!(header.count(), 3);
        assert_eq!(header.num_words(), 4);
    }

    #[test]
    fn header_type_validation() {
        assert!(matches!(
            Type3Header::parse(type0(0, 0)),
            Err(Pm4Error::UnsupportedType { kind: 0, .. })
        ));
        assert!(matches!(
            Type0Header::parse(type3(0x10, 0)),
            Err(Pm4Error::UnsupportedType { kind: 3, .. })
        ));
    }

    #[test]
    fn stream_decodes_mixed_packets_and_bodies() {
        let data = [
            type0(0x100, 1),
            0xdead_beef,
            type3(PM4ItOpcode::Nop.raw(), 0),
            type3(PM4ItOpcode::SetContextReg.raw(), 2),
            7,
            9,
        ];
        let packets: Vec<_> = PacketStream::new(&data).collect::<Result<_, _>>().unwrap();
        assert_eq!(packets.len(), 3);
        match packets[0] {
            Packet::Type0 { header, body } => {
                assert_eq!(header.base(), 0x100);
                assert_eq!(body, &[0xdead_beef]);
            }
            _ => panic!("expected a type-0 packet"),
        }
        match packets[1] {
            Packet::Type3 { opcode, body, .. } => {
                assert_eq!(opcode, Some(PM4ItOpcode::Nop));
                assert!(body.is_empty());
            }
            _ => panic!("expected a type-3 packet"),
        }
        match packets[2] {
            Packet::Type3 { body, .. } => assert_eq!(body, &[7, 9]),
            _ => panic!("expected a type-3 packet"),
        }
    }

    #[test]
    fn unknown_opcodes_keep_their_length() {
        let data = [type3(0xff, 1), 5];
        let packet = PacketStream::new(&data).next().unwrap().unwrap();
        match packet {
            Packet::Type3 { opcode, body, .. } => {
                assert_eq!(opcode, None);
                assert_eq!(body, &[5]);
            }
            _ => panic!("expected a type-3 packet"),
        }
    }

    #[test]
    fn truncated_and_unsupported_packets_end_the_stream() {
        let data = [type3(PM4ItOpcode::Nop.raw(), 3), 1];
        assert!(matches!(
            PacketStream::new(&data).next().unwrap(),
            Err(Pm4Error::Truncated {
                needed: 4,
                remaining: 2,
                ..
            })
        ));

        let data = [(1 << 30), 0];
        assert!(matches!(
            PacketStream::new(&data).next().unwrap(),
            Err(Pm4Error::UnsupportedType { kind: 1, .. })
        ));

        // count = 0x3fff wraps the 14-bit length to zero.
        let data = [type3(PM4ItOpcode::Nop.raw(), 0x3fff)];
        assert!(matches!(
            PacketStream::new(&data).next().unwrap(),
            Err(Pm4Error::ZeroLength { .. })
        ));
    }

    #[test]
    fn pipelining_matches_the_command_thread_rules() {
        assert!(pipelined_opcode(PM4ItOpcode::Nop));
        assert!(pipelined_opcode(PM4ItOpcode::DrawIndex2));
        assert!(pipelined_opcode(PM4ItOpcode::WriteData));
        assert!(pipelined_opcode(PM4ItOpcode::DmaData));
        assert!(!pipelined_opcode(PM4ItOpcode::SetPredication));
        assert!(!pipelined_opcode(PM4ItOpcode::SurfaceSync));
    }
}
