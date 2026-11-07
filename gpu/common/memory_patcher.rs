//! Memory patch parsing and application, ported from
//! `shadps4/common/memory_patcher.h`.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PatchMask {
    #[default]
    None = 0,
    Mask = 1,
    MaskJump32 = 2,
}

/// `patchInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatchInfo {
    pub game_serial: String,
    pub mod_name: String,
    pub offset: String,
    pub value: String,
    pub target: String,
    pub size: String,
    pub is_offset: bool,
    pub little_endian: bool,
    pub patch_mask: PatchMask,
    pub mask_offset: i32,
}

/// The patcher's global address state.
#[derive(Debug, Default)]
pub struct PatcherState {
    pub eboot_address: u64,
    pub eboot_image_size: u64,
    pub game_serial: String,
    pub patch_file: String,
    pub queued: Vec<PatchInfo>,
}

/// `convertValueToHex`.
pub fn convert_value_to_hex(kind: &str, value: &str) -> String {
    match kind {
        "byte" | "bytes" => value.to_string(),
        "float" => value
            .parse::<f32>()
            .map(|value| format!("{:08X}", value.to_bits()))
            .unwrap_or_default(),
        "double" => value
            .parse::<f64>()
            .map(|value| format!("{:016X}", value.to_bits()))
            .unwrap_or_default(),
        _ => value.trim_start_matches("0x").to_uppercase(),
    }
}

/// `PatternToByte`; `??` wildcards become -1.
pub fn pattern_to_byte(pattern: &str) -> Vec<i32> {
    pattern
        .split_whitespace()
        .map(|token| {
            if token.eq_ignore_ascii_case("??") || token.eq_ignore_ascii_case("?") {
                -1
            } else {
                i32::from_str_radix(token, 16).unwrap_or(-1)
            }
        })
        .collect()
}

/// The guest memory surface the patcher writes through.
pub trait PatchMemory {
    fn write(&mut self, address: u64, bytes: &[u8]);
    fn read(&self, address: u64, length: usize) -> Vec<u8>;
}

fn parse_bytes(value: &str) -> Vec<u8> {
    let clean = value.trim_start_matches("0x").replace(' ', "");
    clean
        .as_bytes()
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// `PatchMemory`; applies one patch at its resolved address.
pub fn patch_memory(
    patch: &PatchInfo,
    state: &mut PatcherState,
    memory: &mut dyn PatchMemory,
    resolve: impl FnOnce(&str) -> Option<u64>,
) -> bool {
    let address = if patch.is_offset {
        state.eboot_address.wrapping_add(
            u64::from_str_radix(patch.offset.trim_start_matches("0x"), 16).unwrap_or(0),
        )
    } else {
        match resolve(&patch.target) {
            Some(address) => address,
            None => return false,
        }
    };
    let bytes = parse_bytes(&patch.value);
    if bytes.is_empty() {
        return false;
    }
    match patch.patch_mask {
        PatchMask::None => memory.write(address, &bytes),
        PatchMask::Mask => {
            for (index, byte) in bytes.iter().enumerate() {
                if *byte != 0 {
                    memory.write(address + index as u64, &[*byte]);
                }
            }
        }
        PatchMask::MaskJump32 => {
            let mut jump = vec![0u8; 5];
            jump[0] = 0xE9;
            jump[1..].copy_from_slice(&(address as i32 as u32).to_le_bytes()[..4]);
            memory.write(address, &jump);
        }
    }
    true
}

/// `AddPatchToQueue`.
pub fn add_patch_to_queue(state: &mut PatcherState, patch: PatchInfo) {
    state.queued.push(patch);
}

/// `OnGameLoaded`.
pub fn on_game_loaded(state: &mut PatcherState) -> Vec<PatchInfo> {
    std::mem::take(&mut state.queued)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Memory {
        bytes: std::collections::HashMap<u64, u8>,
    }

    impl PatchMemory for Memory {
        fn write(&mut self, address: u64, bytes: &[u8]) {
            for (index, byte) in bytes.iter().enumerate() {
                self.bytes.insert(address + index as u64, *byte);
            }
        }

        fn read(&self, address: u64, length: usize) -> Vec<u8> {
            (0..length)
                .map(|index| *self.bytes.get(&(address + index as u64)).unwrap_or(&0))
                .collect()
        }
    }

    #[test]
    fn patches_write_at_offsets_and_patterns() {
        assert_eq!(convert_value_to_hex("byte", "90"), "90");
        assert_eq!(convert_value_to_hex("float", "1.0"), "3F800000");
        assert_eq!(pattern_to_byte("90 ?? 11"), vec![0x90, -1, 0x11]);
        let mut state = PatcherState {
            eboot_address: 0x1000,
            ..PatcherState::default()
        };
        let patch = PatchInfo {
            offset: "10".to_string(),
            value: "AABB".to_string(),
            is_offset: true,
            ..PatchInfo::default()
        };
        let mut memory = Memory::default();
        assert!(patch_memory(&patch, &mut state, &mut memory, |_| None));
        assert_eq!(memory.read(0x1010, 2), vec![0xAA, 0xBB]);
        add_patch_to_queue(&mut state, patch.clone());
        assert_eq!(on_game_loaded(&mut state).len(), 1);
        assert!(state.queued.is_empty());
    }
}

/// `PatternScan`; finds the first match of `pattern` (-1 = wildcard) in `memory`.
pub fn pattern_scan(
    memory: &dyn PatchMemory,
    base: u64,
    length: u64,
    pattern: &[i32],
) -> Option<u64> {
    if pattern.is_empty() || length < pattern.len() as u64 {
        return None;
    }
    let bytes = memory.read(base, length as usize);
    let window = pattern.len();
    let mut index = 0usize;
    while index + window <= bytes.len() {
        if pattern
            .iter()
            .zip(&bytes[index..index + window])
            .all(|(expected, actual)| *expected < 0 || *expected == i32::from(*actual))
        {
            return Some(base + index as u64);
        }
        index += 1;
    }
    None
}

/// `OnGameLoaded` followed by each queued patch.
pub fn apply_queued_patches(
    state: &mut PatcherState,
    memory: &mut dyn PatchMemory,
    mut resolve: impl FnMut(&str) -> Option<u64>,
) -> usize {
    let patches = on_game_loaded(state);
    let mut applied = 0;
    for patch in &patches {
        if patch_memory(patch, state, memory, &mut resolve) {
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[derive(Default)]
    struct Memory {
        bytes: Vec<u8>,
    }

    impl PatchMemory for Memory {
        fn write(&mut self, address: u64, bytes: &[u8]) {
            let start = address as usize;
            if start + bytes.len() <= self.bytes.len() {
                self.bytes[start..start + bytes.len()].copy_from_slice(bytes);
            }
        }

        fn read(&self, address: u64, length: usize) -> Vec<u8> {
            let start = address as usize;
            self.bytes
                .get(start..start + length)
                .unwrap_or(&[])
                .to_vec()
        }
    }

    #[test]
    fn pattern_scan_finds_wildcarded_signatures() {
        let memory = Memory {
            bytes: vec![0x11, 0x22, 0x33, 0x44, 0x55],
        };
        assert_eq!(pattern_scan(&memory, 0, 5, &[0x22, -1, 0x44]), Some(1));
        assert_eq!(pattern_scan(&memory, 0, 5, &[0x99]), None);
        assert_eq!(pattern_scan(&memory, 0, 5, &[]), None);
    }
}
