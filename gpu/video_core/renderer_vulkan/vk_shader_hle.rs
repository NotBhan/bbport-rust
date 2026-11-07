//! Shader HLE entry point, ported from
//! `shadps4/video_core/renderer_vulkan/vk_shader_hle.h`.

/// Opaque `Shader::Info`; the shader recompiler owns the concrete type.
#[repr(C)]
pub struct ShaderInfo {
    pub _opaque: [u8; 0],
}

/// Opaque `AmdGpu::Regs`.
#[repr(C)]
pub struct AmdGpuRegs {
    pub _opaque: [u8; 0],
}

/// Opaque `AmdGpu::ComputeProgram`.
#[repr(C)]
pub struct ComputeProgram {
    pub _opaque: [u8; 0],
}

/// `ExecuteShaderHLE`; the rasterizer implements the dispatch.
pub trait ShaderHle {
    fn execute_shader_hle(
        &mut self,
        info: &ShaderInfo,
        regs: &AmdGpuRegs,
        cs_program: &ComputeProgram,
    ) -> bool;
}

/// The header's free function, dispatched through the rasterizer.
pub fn execute_shader_hle(
    rasterizer: &mut dyn ShaderHle,
    info: &ShaderInfo,
    regs: &AmdGpuRegs,
    cs_program: &ComputeProgram,
) -> bool {
    rasterizer.execute_shader_hle(info, regs, cs_program)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl ShaderHle for Fake {
        fn execute_shader_hle(
            &mut self,
            _: &ShaderInfo,
            _: &AmdGpuRegs,
            _: &ComputeProgram,
        ) -> bool {
            true
        }
    }

    #[test]
    fn the_hle_entry_point_dispatches_to_the_rasterizer() {
        let info = ShaderInfo { _opaque: [] };
        let regs = AmdGpuRegs { _opaque: [] };
        let cs = ComputeProgram { _opaque: [] };
        assert!(execute_shader_hle(&mut Fake, &info, &regs, &cs));
    }
}

/// `COPY_SHADER_HASH`.
pub const COPY_SHADER_HASH: u64 = 0xfefebf9f;

/// The multi-copy fast path requires at least eight dword-aligned copies.
pub fn should_use_multi_copy(copies: &[(u64, u64, u64)], enabled: bool) -> bool {
    if !enabled || copies.len() < 8 {
        return false;
    }
    copies
        .iter()
        .all(|&(src, dst, size)| (src | dst | size) & 3 == 0)
}

/// The bounds a multi-copy dispatch would cover.
pub fn multi_copy_bounds(copies: &[(u64, u64, u64)]) -> Option<(u64, u64, u64, u64)> {
    let mut src_min = u64::MAX;
    let mut src_max = 0;
    let mut dst_min = u64::MAX;
    let mut dst_max = 0;
    for &(src, dst, size) in copies {
        if (src | dst | size) & 3 != 0 {
            return None;
        }
        src_min = src_min.min(src);
        src_max = src_max.max(src + size);
        dst_min = dst_min.min(dst);
        dst_max = dst_max.max(dst + size);
    }
    Some((src_min, src_max, dst_min, dst_max))
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn multi_copy_requires_aligned_batches() {
        let copies = vec![(0, 0, 4); 8];
        assert!(should_use_multi_copy(&copies, true));
        assert!(!should_use_multi_copy(&copies, false));
        assert!(!should_use_multi_copy(&copies[..7], true));
        assert!(!should_use_multi_copy(&[(0, 0, 3); 8], true));
        assert_eq!(multi_copy_bounds(&copies), Some((0, 4, 0, 4)));
        assert_eq!(multi_copy_bounds(&[(0, 0, 3); 8]), None);
        assert_eq!(COPY_SHADER_HASH, 0xfefebf9f);
    }
}
