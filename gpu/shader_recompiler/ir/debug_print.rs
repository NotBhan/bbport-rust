//! Debug print flags, ported from `legacy/gpu/shadps4/shader_recompiler/ir/debug_print.h`.

use super::opcodes::{num_args_of, Opcode};
use crate::gpu::common::bit_field::BitField;

/// `DEBUGPRINT_NUM_FORMAT_ARGS`: the variadic arguments of `DebugPrint`, minus the format
/// string itself.
pub const DEBUGPRINT_NUM_FORMAT_ARGS: usize = num_args_of(Opcode::DebugPrint) - 1;

/// The `BitField<0, 32, u32>` the header declares for the flags word.
type NumArgsField = BitField<u32, 0, 32>;

/// `DebugPrintFlags`: the word that follows the format string. The C++'s union exposes the raw
/// word and a 32-bit field over it, which here is a bit field over the whole word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DebugPrintFlags(u32);

impl DebugPrintFlags {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The union's `raw` member.
    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn new(num_args: u32) -> Self {
        let mut flags = Self::default();
        flags.set_num_args(num_args);
        flags
    }

    /// The union's `num_args` member, read.
    pub fn num_args(self) -> u32 {
        NumArgsField::from_raw(self.0).value()
    }

    /// The union's `num_args` member, written.
    pub fn set_num_args(&mut self, num_args: u32) {
        let mut field = NumArgsField::from_raw(self.0);
        field.assign(num_args);
        self.0 = field.raw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::opcodes::{arg_type_of, Type};

    #[test]
    fn the_format_argument_count_follows_the_opcode_table() {
        assert_eq!(DEBUGPRINT_NUM_FORMAT_ARGS, 4);
        assert_eq!(num_args_of(Opcode::DebugPrint), 5);
        assert_eq!(arg_type_of(Opcode::DebugPrint, 0), Type::StringLiteral);
    }

    #[test]
    fn the_flags_word_round_trips() {
        let mut flags = DebugPrintFlags::default();
        assert_eq!(flags.raw(), 0);
        assert_eq!(flags.num_args(), 0);
        flags.set_num_args(3);
        assert_eq!(flags.num_args(), 3);
        assert_eq!(flags.raw(), 3);
        assert_eq!(
            DebugPrintFlags::new(DEBUGPRINT_NUM_FORMAT_ARGS as u32).num_args(),
            4
        );
        // The field spans the whole word, so any value survives it.
        flags.set_num_args(u32::MAX);
        assert_eq!(flags.raw(), u32::MAX);
        assert_eq!(flags.num_args(), u32::MAX);
        assert_eq!(DebugPrintFlags::from_raw(7).num_args(), 7);
    }
}
