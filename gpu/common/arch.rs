//! Target architecture flags, ported from `legacy/gpu/shadps4/common/arch.h`.

/// `ARCH_X86_64`.
pub const ARCH_X86_64: bool = cfg!(target_arch = "x86_64");
/// `ARCH_ARM64`.
pub const ARCH_ARM64: bool = cfg!(target_arch = "aarch64");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_target_is_one_of_the_supported_architectures() {
        assert_eq!(
            u8::from(ARCH_X86_64) + u8::from(ARCH_ARM64),
            1,
            "the renderer is built for x86-64 or AArch64"
        );
    }
}
