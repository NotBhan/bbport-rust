//! The tessellation constants the hull and domain shaders read, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/tessellation.h`.

/// `Shader::TessellationDataConstantBuffer`: the buffer the tessellation stages read their
/// strides and patch sizes from, one dword per field.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TessellationDataConstantBuffer {
    pub ls_stride: u32,
    /// `HullStateConstants::m_cpStride != 0 ? HullStateConstants::m_cpStride : ls_stride`.
    pub hs_cp_stride: u32,
    /// The number of patches submitted in a threadgroup.
    pub num_patches: u32,
    /// `HullStateConstants::m_numInputCP::m_cpStride != 0 ?
    /// HullStateConstants::m_numInputCP * ls_stride * num_patches : 0`, which is zero when
    /// the hull shader is a passthrough.
    pub hs_output_base: u32,
    /// `16 * num_patch_attrs`.
    pub patch_const_size: u32,
    /// `hs_output_base + patch_output_size`.
    pub patch_const_base: u32,
    /// `output_cp_stride * num_output_cp_per_patch`.
    pub patch_output_size: u32,
    pub off_chip_tessellation_factor_threshold: f32,
    pub first_edge_tess_factor_index: u32,
}

/// `Shader::TessConstantAttribute`: the dword each field of the buffer sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TessConstantAttribute {
    LsStride,
    HsCpStride,
    HsNumPatch,
    HsOutputBase,
    PatchConstSize,
    PatchConstBase,
    PatchOutputSize,
    OffChipTessellationFactorThreshold,
    FirstEdgeTessFactorIndex,
}

impl TessConstantAttribute {
    /// The field's dword index in the buffer, in the order the header lists them.
    pub const fn index(self) -> u32 {
        self as u32
    }

    /// The field at a dword index, as the C++'s `static_cast<TessConstantAttribute>`.
    pub const fn from_index(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::LsStride),
            1 => Some(Self::HsCpStride),
            2 => Some(Self::HsNumPatch),
            3 => Some(Self::HsOutputBase),
            4 => Some(Self::PatchConstSize),
            5 => Some(Self::PatchConstBase),
            6 => Some(Self::PatchOutputSize),
            7 => Some(Self::OffChipTessellationFactorThreshold),
            8 => Some(Self::FirstEdgeTessFactorIndex),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attribute_names_number_the_buffer_dwords() {
        assert_eq!(TessConstantAttribute::LsStride.index(), 0);
        assert_eq!(TessConstantAttribute::HsCpStride.index(), 1);
        assert_eq!(TessConstantAttribute::HsNumPatch.index(), 2);
        assert_eq!(TessConstantAttribute::HsOutputBase.index(), 3);
        assert_eq!(TessConstantAttribute::PatchConstSize.index(), 4);
        assert_eq!(TessConstantAttribute::PatchConstBase.index(), 5);
        assert_eq!(TessConstantAttribute::PatchOutputSize.index(), 6);
        for attribute in [
            TessConstantAttribute::LsStride,
            TessConstantAttribute::HsCpStride,
            TessConstantAttribute::HsNumPatch,
            TessConstantAttribute::HsOutputBase,
            TessConstantAttribute::PatchConstSize,
            TessConstantAttribute::PatchConstBase,
            TessConstantAttribute::PatchOutputSize,
            TessConstantAttribute::OffChipTessellationFactorThreshold,
            TessConstantAttribute::FirstEdgeTessFactorIndex,
        ] {
            assert_eq!(
                TessConstantAttribute::from_index(attribute.index()),
                Some(attribute)
            );
        }
        assert_eq!(TessConstantAttribute::from_index(9), None);
        assert_eq!(
            TessConstantAttribute::OffChipTessellationFactorThreshold.index(),
            7
        );
        assert_eq!(TessConstantAttribute::FirstEdgeTessFactorIndex.index(), 8);
        let buffer = TessellationDataConstantBuffer {
            ls_stride: 16,
            hs_cp_stride: 16,
            ..TessellationDataConstantBuffer::default()
        };
        assert_eq!(buffer.num_patches, 0);
        assert_eq!(buffer.off_chip_tessellation_factor_threshold, 0.0);
        assert_eq!(buffer.patch_const_base, buffer.hs_output_base);
    }
}
