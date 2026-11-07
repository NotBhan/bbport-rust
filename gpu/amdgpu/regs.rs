//! Flat GPU register file and dirty/delta tracking, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs.{h,cpp}`.
//!
//! The C++ bitfield unions in `regs_*.h` describe the fields of these words and are a separate
//! port; this module provides the word-addressable register file, the reset defaults and the
//! 32-word block dirty/delta mechanism used to hand state between submissions.

pub const NUM_REGS: u32 = 0xD000;
pub const UCONFIG_REG_WORD_OFFSET: u32 = 0xC000;
pub const CONTEXT_REG_WORD_OFFSET: u32 = 0xA000;
pub const CONFIG_REG_WORD_OFFSET: u32 = 0x2000;
pub const SH_REG_WORD_OFFSET: u32 = 0x2C00;

/// Shader program blocks (word offsets), from the static_asserts in `regs.cpp`.
pub const PS_PROGRAM_OFFSET: u32 = 0x2C08;
pub const VS_PROGRAM_OFFSET: u32 = 0x2C48;
pub const GS_PROGRAM_OFFSET: u32 = 0x2C88;
pub const ES_PROGRAM_OFFSET: u32 = 0x2CC8;
pub const HS_PROGRAM_OFFSET: u32 = 0x2D08;
pub const LS_PROGRAM_OFFSET: u32 = 0x2D48;
pub const CS_PROGRAM_OFFSET: u32 = 0x2E00;

/// The C++ `REG_ARRAY_DEFAULT` (fpPS4), copied to `CONTEXT_REG_WORD_OFFSET + 0x80`.
const REG_ARRAY_DEFAULT: [u32; 85] = [
    0x00000000, 0x80000000, 0x40004000, 0xdeadbeef, 0x00000000, 0x40004000, 0x00000000, 0x40004000,
    0x00000000, 0x40004000, 0x00000000, 0x40004000, 0xaa99aaaa, 0x00000000, 0xdeadbeef, 0xdeadbeef,
    0x80000000, 0x40004000, 0x00000000, 0x00000000, 0x80000000, 0x40004000, 0x80000000, 0x40004000,
    0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000,
    0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000,
    0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x80000000, 0x40004000,
    0x80000000, 0x40004000, 0x80000000, 0x40004000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000,
    0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000,
    0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000,
    0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x00000000, 0x3f800000,
    0x00000000, 0x3f800000, 0x00000000, 0x3f800000, 0x2a00161a,
];

/// Individual context-register defaults after the table copy.
const CONTEXT_DEFAULTS: [(u32, u32); 16] = [
    (0x000D, 0x40004000),
    (0x01B6, 0x00000002),
    (0x0204, 0x00090000),
    (0x0205, 0x00000004),
    (0x0295, 0x00000100),
    (0x0296, 0x00000080),
    (0x0297, 0x00000002),
    (0x02AA, 0x00001000),
    (0x02F7, 0x00001000),
    (0x02F9, 0x00000005),
    (0x02FA, 0x3f800000),
    (0x02FB, 0x3f800000),
    (0x02FC, 0x3f800000),
    (0x02FD, 0x3f800000),
    (0x0316, 0x0000000E),
    (0x0317, 0x00000010),
];

#[derive(Clone)]
pub struct Regs {
    words: Box<[u32]>,
}

impl Default for Regs {
    fn default() -> Self {
        Self::new()
    }
}

impl Regs {
    /// A register file at the hardware reset defaults.
    pub fn new() -> Self {
        let mut regs = Self {
            words: vec![0; NUM_REGS as usize].into_boxed_slice(),
        };
        regs.set_defaults();
        regs
    }

    /// Restore the power-on defaults (`Regs::SetDefaults`).
    pub fn set_defaults(&mut self) {
        self.words.fill(0);
        let base = CONTEXT_REG_WORD_OFFSET as usize + 0x80;
        self.words[base..base + REG_ARRAY_DEFAULT.len()].copy_from_slice(&REG_ARRAY_DEFAULT);
        for (offset, value) in CONTEXT_DEFAULTS {
            self.words[CONTEXT_REG_WORD_OFFSET as usize + offset as usize] = value;
        }
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    pub fn word(&self, offset: u32) -> Option<u32> {
        self.words.get(offset as usize).copied()
    }

    /// Write one word; `false` when `offset` is outside the register file.
    pub fn set_word(&mut self, offset: u32, value: u32) -> bool {
        match self.words.get_mut(offset as usize) {
            Some(word) => {
                *word = value;
                true
            }
            None => false,
        }
    }

    /// Shader program block offset for stages 0..=5 (PS, VS, GS, ES, HS, LS), like
    /// `ProgramForStage`.
    pub fn program_offset(stage: u32) -> Option<u32> {
        match stage {
            0 => Some(PS_PROGRAM_OFFSET),
            1 => Some(VS_PROGRAM_OFFSET),
            2 => Some(GS_PROGRAM_OFFSET),
            3 => Some(ES_PROGRAM_OFFSET),
            4 => Some(HS_PROGRAM_OFFSET),
            5 => Some(LS_PROGRAM_OFFSET),
            _ => None,
        }
    }
}

/// Register blocks written by a stretch of packets, and their values at its end.
#[derive(Debug, Clone)]
pub struct RegDirty {
    blocks: [u64; RegDirty::NUM_BLOCKS / 64],
    pub reset: bool,
}

impl Default for RegDirty {
    fn default() -> Self {
        Self::new()
    }
}

impl RegDirty {
    pub const BLOCK_WORDS: u32 = 32;
    pub const NUM_BLOCKS: usize = NUM_REGS as usize / Self::BLOCK_WORDS as usize;

    pub fn new() -> Self {
        Self {
            blocks: [0; Self::NUM_BLOCKS / 64],
            reset: false,
        }
    }

    /// Mark every 32-word block touched by `[word, word + count)`.
    pub fn mark(&mut self, word: u32, count: u32) {
        if count == 0 || word >= NUM_REGS {
            return;
        }
        let last = (word.saturating_add(count)).min(NUM_REGS) - 1;
        let first_block = (word / Self::BLOCK_WORDS) as usize;
        let last_block = (last / Self::BLOCK_WORDS) as usize;
        for block in first_block..=last_block {
            self.blocks[block / 64] |= 1 << (block % 64);
        }
    }

    /// Mark the words covered by a field at `offset_bytes` with `size_bytes`.
    pub fn mark_bytes(&mut self, offset_bytes: u64, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }
        let word = (offset_bytes / 4) as u32;
        let count = size_bytes.div_ceil(4) as u32;
        self.mark(word, count);
    }

    pub fn mark_reset(&mut self) {
        self.reset = true;
    }

    pub fn clear(&mut self) {
        self.blocks.fill(0);
        self.reset = false;
    }

    pub fn is_set(&self, block: usize) -> bool {
        block < Self::NUM_BLOCKS && self.blocks[block / 64] & (1 << (block % 64)) != 0
    }
}

/// A delta of the marked 32-word blocks (`RegDelta`), used to hand register state between
/// submissions without replaying the packets in between.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegDelta {
    pub reset: bool,
    pub blocks: Vec<u16>,
    pub words: Vec<u32>,
}

impl RegDelta {
    pub fn capture(&mut self, regs: &Regs, dirty: &RegDirty) {
        self.reset = dirty.reset;
        self.blocks.clear();
        self.words.clear();
        for block in 0..RegDirty::NUM_BLOCKS {
            if !dirty.is_set(block) {
                continue;
            }
            self.blocks.push(block as u16);
            let start = block * RegDirty::BLOCK_WORDS as usize;
            self.words
                .extend_from_slice(&regs.words[start..start + RegDirty::BLOCK_WORDS as usize]);
        }
    }

    pub fn apply(&self, regs: &mut Regs) {
        if self.reset {
            regs.set_defaults();
        }
        for (index, block) in self.blocks.iter().enumerate() {
            let start = *block as usize * RegDirty::BLOCK_WORDS as usize;
            let source = &self.words[index * RegDirty::BLOCK_WORDS as usize
                ..(index + 1) * RegDirty::BLOCK_WORDS as usize];
            regs.words[start..start + RegDirty::BLOCK_WORDS as usize].copy_from_slice(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_defaults_match_the_cpp_values() {
        let regs = Regs::new();
        let base = CONTEXT_REG_WORD_OFFSET + 0x80;
        assert_eq!(regs.word(base), Some(0x00000000));
        assert_eq!(regs.word(base + 1), Some(0x80000000));
        assert_eq!(
            regs.word(base + REG_ARRAY_DEFAULT.len() as u32 - 1),
            Some(0x2a00161a)
        );
        assert_eq!(
            regs.word(CONTEXT_REG_WORD_OFFSET + 0x000D),
            Some(0x40004000)
        );
        assert_eq!(
            regs.word(CONTEXT_REG_WORD_OFFSET + 0x01B6),
            Some(0x00000002)
        );
        assert_eq!(
            regs.word(CONTEXT_REG_WORD_OFFSET + 0x02FA),
            Some(0x3f800000)
        );
        assert_eq!(
            regs.word(CONTEXT_REG_WORD_OFFSET + 0x0317),
            Some(0x00000010)
        );
        assert_eq!(regs.word(0), Some(0));
        assert_eq!(regs.words().len(), NUM_REGS as usize);
    }

    #[test]
    fn word_access_is_bounds_checked() {
        let mut regs = Regs::new();
        assert!(regs.set_word(0xA00D, 0x1234));
        assert_eq!(regs.word(0xA00D), Some(0x1234));
        assert!(!regs.set_word(NUM_REGS, 1));
        assert_eq!(regs.word(NUM_REGS), None);
        assert_eq!(Regs::program_offset(0), Some(PS_PROGRAM_OFFSET));
        assert_eq!(Regs::program_offset(5), Some(LS_PROGRAM_OFFSET));
        assert_eq!(Regs::program_offset(6), None);
    }

    #[test]
    fn dirty_marks_spanning_blocks_and_clamps() {
        let mut dirty = RegDirty::new();
        dirty.mark(31, 2);
        assert!(dirty.is_set(0));
        assert!(dirty.is_set(1));
        assert!(!dirty.is_set(2));

        let mut dirty = RegDirty::new();
        dirty.mark(NUM_REGS - 1, 100);
        assert!(dirty.is_set(RegDirty::NUM_BLOCKS - 1));
        assert!(!dirty.is_set(0));

        let mut dirty = RegDirty::new();
        dirty.mark(NUM_REGS, 1);
        dirty.mark(0, 0);
        assert!(!dirty.is_set(0));

        let mut dirty = RegDirty::new();
        dirty.mark_bytes(63 * 4, 8);
        assert!(dirty.is_set(1));
        assert!(dirty.is_set(2));
        dirty.mark_reset();
        assert!(dirty.reset);
        dirty.clear();
        assert!(!dirty.reset);
        assert!(!dirty.is_set(1));
    }

    #[test]
    fn delta_captures_and_restores_blocks() {
        let mut regs = Regs::new();
        regs.set_word(CONTEXT_REG_WORD_OFFSET, 0x1111);
        regs.set_word(CONTEXT_REG_WORD_OFFSET + 40, 0x2222);
        let mut dirty = RegDirty::new();
        dirty.mark(CONTEXT_REG_WORD_OFFSET, 1);
        dirty.mark(CONTEXT_REG_WORD_OFFSET + 40, 1);

        let mut delta = RegDelta::default();
        delta.capture(&regs, &dirty);
        assert_eq!(delta.blocks.len(), 2);
        assert_eq!(delta.words.len(), 64);

        regs.set_word(CONTEXT_REG_WORD_OFFSET, 0);
        regs.set_word(CONTEXT_REG_WORD_OFFSET + 40, 0);
        delta.apply(&mut regs);
        assert_eq!(regs.word(CONTEXT_REG_WORD_OFFSET), Some(0x1111));
        assert_eq!(regs.word(CONTEXT_REG_WORD_OFFSET + 40), Some(0x2222));
    }

    #[test]
    fn delta_reset_restores_defaults_before_applying_blocks() {
        let mut regs = Regs::new();
        let mut dirty = RegDirty::new();
        dirty.mark_reset();
        let mut delta = RegDelta::default();
        delta.capture(&regs, &dirty);
        assert!(delta.reset);
        assert!(delta.blocks.is_empty());

        regs.set_word(CONTEXT_REG_WORD_OFFSET + 0x000D, 0);
        delta.apply(&mut regs);
        assert_eq!(
            regs.word(CONTEXT_REG_WORD_OFFSET + 0x000D),
            Some(0x40004000)
        );
    }
}
