//! Shader pre-build, ported from
//! `shadps4/video_core/renderer_vulkan/vk_shader_prebuild.{h,cpp}`.

use std::path::{Path, PathBuf};

/// `BB_SHADER_CODE`; empty when unset.
pub fn code_dir() -> PathBuf {
    std::env::var_os("BB_SHADER_CODE")
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// `Available`; the directory exists.
pub fn available() -> bool {
    let dir = code_dir();
    !dir.as_os_str().is_empty() && dir.is_dir()
}

/// `CodeFile`; `<hash>.bin` or `<hash>.copy.bin`.
pub fn code_file(hash: u64, copy: bool) -> PathBuf {
    code_dir().join(format!(
        "{hash:016x}{}.bin",
        if copy { ".copy" } else { "" }
    ))
}

/// `ReadCode`; empty unless the file holds whole SPIR-V words.
pub fn read_code(path: &Path) -> Vec<u32> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return Vec::new();
    }
    bytes
        .chunks(4)
        .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
        .collect()
}

/// The translator the shader recompiler supplies.
pub trait ShaderPrebuild {
    fn translate(&mut self, code: &[u32], stored_hash: u64, perm_idx: usize) -> Vec<u32>;
}

/// `Translate`; empty when the code is missing or translation fails.
pub fn translate(
    translator: &mut dyn ShaderPrebuild,
    stored_info_hash: u64,
    perm_idx: usize,
    code_dir: &Path,
) -> Vec<u32> {
    if !code_dir.is_dir() {
        return Vec::new();
    }
    let path = code_dir.join(format!("{stored_info_hash:016x}.bin"));
    let code = read_code(&path);
    if code.is_empty() {
        return Vec::new();
    }
    translator.translate(&code, stored_info_hash, perm_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Vec<u32>);

    impl ShaderPrebuild for Fake {
        fn translate(&mut self, code: &[u32], _: u64, _: usize) -> Vec<u32> {
            if code == [7, 8] {
                self.0.clone()
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn prebuild_reads_whole_words_and_refuses_missing_dirs() {
        let dir = std::env::temp_dir().join("bbport-no-such-shader-code");
        assert!(translate(&mut Fake(vec![1]), 1, 0, &dir).is_empty());
        let file = std::env::temp_dir().join("bbport-prebuild-test.bin");
        std::fs::write(&file, [7u8, 0, 0, 0, 8, 0, 0, 0]).unwrap();
        assert_eq!(read_code(&file), vec![7, 8]);
        std::fs::write(&file, [1u8, 2, 3]).unwrap();
        assert!(read_code(&file).is_empty());
        assert!(code_file(0xab, true).ends_with("00000000000000ab.copy.bin"));
        let _ = std::fs::remove_file(file);
    }
}
