//! Instruction decoding, ported from `legacy/gpu/shadps4/common/decoder.h`. The C++ wraps
//! Zydis; the Rust port keeps the same surface and installs the disassembler from the host.

use std::sync::{Mutex, OnceLock};

/// The longest an x86-64 instruction can be, the C++'s default `size` argument.
pub const MAX_INSTRUCTION_LENGTH: usize = 15;

/// A decoded instruction: its length and its formatted text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub length: usize,
    pub text: String,
}

/// The disassembler the host installs in place of Zydis.
pub trait InstructionDecoder: Send + Sync {
    /// `DecoderImpl::decodeInstruction`.
    fn decode(&self, data: &[u8], address: u64) -> Option<DecodedInstruction>;
}

fn backend() -> &'static Mutex<Option<Box<dyn InstructionDecoder>>> {
    static BACKEND: OnceLock<Mutex<Option<Box<dyn InstructionDecoder>>>> = OnceLock::new();
    BACKEND.get_or_init(|| Mutex::new(None))
}

pub fn set_instruction_decoder(decoder: Box<dyn InstructionDecoder>) {
    *backend().lock().unwrap_or_else(|error| error.into_inner()) = Some(decoder);
}

/// `Common::DecoderImpl` (`Common::Decoder` singleton).
#[derive(Debug, Default)]
pub struct DecoderImpl;

impl DecoderImpl {
    /// `DecoderImpl::decodeInstruction`, with the C++'s default 15-byte window.
    pub fn decode_instruction(&self, data: &[u8], address: u64) -> Option<DecodedInstruction> {
        self.decode_instruction_with_size(data, address, MAX_INSTRUCTION_LENGTH)
    }

    pub fn decode_instruction_with_size(
        &self,
        data: &[u8],
        address: u64,
        size: usize,
    ) -> Option<DecodedInstruction> {
        if data.is_empty() || size == 0 {
            return None;
        }
        let window = &data[..data.len().min(size)];
        let backend = backend().lock().unwrap_or_else(|error| error.into_inner());
        backend.as_ref()?.decode(window, address)
    }

    /// `DecoderImpl::disassembleInst`: the formatted text of an instruction.
    pub fn disassemble_inst(&self, instruction: &DecodedInstruction) -> String {
        instruction.text.clone()
    }
}

/// `Common::Decoder`: the process-wide instance.
pub fn decoder() -> &'static DecoderImpl {
    crate::gpu::common::singleton::instance::<DecoderImpl>()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TwoByteNop;

    impl InstructionDecoder for TwoByteNop {
        fn decode(&self, data: &[u8], address: u64) -> Option<DecodedInstruction> {
            if data.first().copied()? != 0x90 {
                return None;
            }
            Some(DecodedInstruction {
                length: 1,
                text: format!("nop @ {address:#x}"),
            })
        }
    }

    #[test]
    fn decoding_goes_through_the_installed_disassembler() {
        set_instruction_decoder(Box::new(TwoByteNop));
        let instance = decoder();
        let instruction = instance.decode_instruction(&[0x90, 0x90], 0x1000).unwrap();
        assert_eq!(instruction.length, 1);
        assert_eq!(instance.disassemble_inst(&instruction), "nop @ 0x1000");
        assert_eq!(instance.decode_instruction(&[0xcc], 0x1000), None);
        assert_eq!(instance.decode_instruction(&[], 0x1000), None);
        assert_eq!(
            instance.decode_instruction_with_size(&[0x90], 0x10, 0),
            None
        );
        // The window never grows past the requested size.
        let windowed = instance.decode_instruction_with_size(&[0x90; 32], 0x10, 4);
        assert!(windowed.is_some());
        assert!(
            std::ptr::eq(instance, decoder()),
            "the singleton is one instance"
        );
    }
}
