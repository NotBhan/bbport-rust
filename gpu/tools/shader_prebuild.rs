//! `bb-shader-prebuild` CLI, ported from `legacy/gpu/tools/shader_prebuild.cpp`.

use std::path::{Path, PathBuf};

/// Parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrebuildArgs {
    pub cache: PathBuf,
    pub code: PathBuf,
    pub write: bool,
    pub rejects: Option<PathBuf>,
}

impl PrebuildArgs {
    /// `main` argument parsing; `None` when the usage line should be printed.
    pub fn parse(args: &[String]) -> Option<Self> {
        if args.len() < 3 {
            return None;
        }
        let write = args.len() > 3 && args[3] == "--write";
        let rejects = if args.len() > 4 && args[3] == "--rejects" {
            Some(PathBuf::from(&args[4]))
        } else {
            None
        };
        Some(Self {
            cache: PathBuf::from(&args[1]),
            code: PathBuf::from(&args[2]),
            write,
            rejects,
        })
    }

    /// Sets `BB_SHADER_CODE` for the prebuild translation.
    pub fn set_code_env(&self) {
        std::env::set_var("BB_SHADER_CODE", &self.code);
    }
}

pub fn read_file(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_default()
}

pub fn spv_path(cache: &Path, pgm_hash: u64, perm_idx: usize) -> PathBuf {
    cache.join(format!("0x{pgm_hash:016x}_{perm_idx}.spv"))
}

/// Whether the cached SPIR-V equals the translation (`memcmp` in the tool).
pub fn spv_identical(cached: &[u8], translated: &[u32]) -> bool {
    cached.len() == translated.len() * 4
        && translated
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .eq(cached.iter().copied())
}

/// Writes the translated SPIR-V when `--write` is active.
pub fn write_spv(path: &Path, translated: &[u32]) -> std::io::Result<()> {
    let bytes: Vec<u8> = translated
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    std::fs::write(path, bytes)
}

/// The tool's summary line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub total: u32,
    pub identical: u32,
    pub different: u32,
    pub skipped: u32,
    pub failed: u32,
}

impl Summary {
    pub fn exit_code(&self) -> i32 {
        i32::from(self.failed != 0)
    }

    pub fn line(&self) -> String {
        format!(
            "shader pre-build: {} permutations: {} identical, {} different, {} skipped, {} could not be translated",
            self.total, self.identical, self.different, self.skipped, self.failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_and_comparison_match_the_tool() {
        let args = PrebuildArgs::parse(&[
            "bin".into(),
            "cache".into(),
            "code".into(),
            "--rejects".into(),
            "rejects.txt".into(),
        ])
        .unwrap();
        assert!(!args.write);
        assert_eq!(args.rejects, Some(PathBuf::from("rejects.txt")));
        assert_eq!(args.cache, PathBuf::from("cache"));
        assert_eq!(
            spv_path(Path::new("cache"), 0xab, 2),
            PathBuf::from("cache/0x00000000000000ab_2.spv")
        );
        assert!(spv_identical(&[7, 0, 0, 0], &[7]));
        assert!(!spv_identical(&[7, 0, 0, 0], &[8]));
        let summary = Summary {
            total: 3,
            identical: 2,
            different: 1,
            ..Summary::default()
        };
        assert_eq!(summary.exit_code(), 0);
        assert!(summary.line().contains("3 permutations"));
        assert!(PrebuildArgs::parse(&["bin".into()]).is_none());
    }
}
