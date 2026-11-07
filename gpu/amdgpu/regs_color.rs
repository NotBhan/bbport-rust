//! Color-buffer registers, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs_color.h`.
//!
//! The C++ describes most registers as bit-field unions; the port keeps the raw word and
//! exposes each field as an accessor, as the other `regs_*` modules do.

#![allow(non_upper_case_globals)]

use crate::gpu::amdgpu::pixel_format::{
    map_number_conversion, num_bits_per_block, num_components, remap_data_format,
    remap_number_format, remap_swizzle, CompMapping, CompSwizzle, DataFormat, NumberConversion,
    NumberFormat,
};
use crate::gpu::amdgpu::tiling::TileMode;

/// `AmdGpu::NUM_COLOR_BUFFERS`.
pub const NUM_COLOR_BUFFERS: usize = 8;

/// `AmdGpu::BlendConstants`.
pub type BlendConstants = [f32; 4];

/// `AmdGpu::BlendControl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct BlendControl(u32);

/// `BlendControl::BlendFactor`.
pub mod blend_factor {
    pub const Zero: u32 = 0;
    pub const One: u32 = 1;
    pub const SrcColor: u32 = 2;
    pub const OneMinusSrcColor: u32 = 3;
    pub const SrcAlpha: u32 = 4;
    pub const OneMinusSrcAlpha: u32 = 5;
    pub const DstAlpha: u32 = 6;
    pub const OneMinusDstAlpha: u32 = 7;
    pub const DstColor: u32 = 8;
    pub const OneMinusDstColor: u32 = 9;
    pub const SrcAlphaSaturate: u32 = 10;
    pub const ConstantColor: u32 = 13;
    pub const OneMinusConstantColor: u32 = 14;
    pub const Src1Color: u32 = 15;
    pub const InvSrc1Color: u32 = 16;
    pub const Src1Alpha: u32 = 17;
    pub const InvSrc1Alpha: u32 = 18;
    pub const ConstantAlpha: u32 = 19;
    pub const OneMinusConstantAlpha: u32 = 20;
}

/// `BlendControl::BlendFunc`.
pub mod blend_func {
    pub const Add: u32 = 0;
    pub const Subtract: u32 = 1;
    pub const Min: u32 = 2;
    pub const Max: u32 = 3;
    pub const ReverseSubtract: u32 = 4;
}

impl BlendControl {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    fn field(self, offset: u32, bits: u32) -> u32 {
        (self.0 >> offset) & ((1 << bits) - 1)
    }

    pub fn color_src_factor(self) -> u32 {
        self.field(0, 5)
    }

    pub fn color_func(self) -> u32 {
        self.field(5, 3)
    }

    pub fn color_dst_factor(self) -> u32 {
        self.field(8, 5)
    }

    pub fn alpha_src_factor(self) -> u32 {
        self.field(16, 5)
    }

    pub fn alpha_func(self) -> u32 {
        self.field(21, 3)
    }

    pub fn alpha_dst_factor(self) -> u32 {
        self.field(24, 5)
    }

    pub fn separate_alpha_blend(self) -> bool {
        self.field(29, 1) != 0
    }

    pub fn enable(self) -> bool {
        self.field(30, 1) != 0
    }

    pub fn disable_rop3(self) -> bool {
        self.field(31, 1) != 0
    }
}

/// `AmdGpu::ColorControl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorControl(u32);

/// `ColorControl::OperationMode`.
pub mod operation_mode {
    pub const Disable: u32 = 0;
    pub const Normal: u32 = 1;
    pub const EliminateFastClear: u32 = 2;
    pub const Resolve: u32 = 3;
    pub const Err: u32 = 4;
    pub const FmaskDecompress: u32 = 5;
}

/// `ColorControl::LogicOp`.
pub mod logic_op {
    pub const Clear: u32 = 0x00;
    pub const Nor: u32 = 0x11;
    pub const AndInverted: u32 = 0x22;
    pub const CopyInverted: u32 = 0x33;
    pub const AndReverse: u32 = 0x44;
    pub const Invert: u32 = 0x55;
    pub const BrushXor: u32 = 0x5a;
    pub const Xor: u32 = 0x66;
    pub const Nand: u32 = 0x77;
    pub const And: u32 = 0x88;
    pub const Equiv: u32 = 0x99;
    pub const Noop: u32 = 0xAA;
    pub const OrInverted: u32 = 0xBB;
    pub const Copy: u32 = 0xCC;
    pub const OrReverse: u32 = 0xDD;
    pub const Or: u32 = 0xEE;
    pub const Set: u32 = 0xFF;
}

impl ColorControl {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    fn field(self, offset: u32, bits: u32) -> u32 {
        (self.0 >> offset) & ((1 << bits) - 1)
    }

    pub fn disable_dual_quad(self) -> bool {
        self.field(0, 1) != 0
    }

    pub fn degamma_enable(self) -> bool {
        self.field(3, 1) != 0
    }

    pub fn mode(self) -> u32 {
        self.field(4, 3)
    }

    pub fn rop3(self) -> u32 {
        self.field(16, 8)
    }
}

/// `AmdGpu::ColorBufferMask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorBufferMask {
    pub raw: u32,
}

/// `ColorBufferMask::ColorComponent`.
pub const COMPONENT_R: u32 = 1 << 0;
pub const COMPONENT_G: u32 = 1 << 1;
pub const COMPONENT_B: u32 = 1 << 2;
pub const COMPONENT_A: u32 = 1 << 3;

impl ColorBufferMask {
    pub fn get_mask(self, buffer_id: u32) -> u32 {
        (self.raw >> (buffer_id * 4)) & 0xf
    }

    pub fn set_mask(&mut self, buffer_id: u32, mask: u32) {
        self.raw &= !(0xf << (buffer_id * 4));
        self.raw |= mask << (buffer_id * 4);
    }
}

/// `ColorBuffer::EndianSwap`.
pub mod endian_swap {
    pub const None: u32 = 0;
    pub const Swap8In16: u32 = 1;
    pub const Swap8In32: u32 = 2;
    pub const Swap8In64: u32 = 3;
}

/// `ColorBuffer::SwapMode`.
pub mod swap_mode {
    pub const Standard: u32 = 0;
    pub const Alternate: u32 = 1;
    pub const StandardReverse: u32 = 2;
    pub const AlternateReverse: u32 = 3;
}

/// `ColorBuffer::RoundMode`.
pub mod round_mode {
    pub const ByHalf: u32 = 0;
    pub const Truncate: u32 = 1;
}

/// `ColorBuffer::Color0Info`, the packed `info` word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Color0Info {
    pub raw: u32,
}

impl Color0Info {
    fn field(self, offset: u32, bits: u32) -> u32 {
        (self.raw >> offset) & ((1 << bits) - 1)
    }

    pub fn endian(self) -> u32 {
        self.field(0, 2)
    }

    pub fn format(self) -> u32 {
        self.field(2, 5)
    }

    pub fn linear_general(self) -> bool {
        self.field(7, 1) != 0
    }

    pub fn number_type(self) -> u32 {
        self.field(8, 3)
    }

    pub fn comp_swap(self) -> u32 {
        self.field(11, 2)
    }

    pub fn fast_clear(self) -> bool {
        self.field(13, 1) != 0
    }

    pub fn compression(self) -> bool {
        self.field(14, 1) != 0
    }

    pub fn blend_clamp(self) -> bool {
        self.field(15, 1) != 0
    }

    pub fn blend_bypass(self) -> bool {
        self.field(16, 1) != 0
    }

    pub fn simple_float(self) -> bool {
        self.field(17, 1) != 0
    }

    pub fn round_mode(self) -> u32 {
        self.field(18, 1)
    }

    pub fn cmask_is_linear(self) -> bool {
        self.field(19, 1) != 0
    }

    pub fn blend_opt_dont_rd_dst(self) -> u32 {
        self.field(20, 3)
    }

    pub fn blend_opt_discard_pixel(self) -> u32 {
        self.field(23, 3)
    }

    pub fn fmask_compression_disable_ci(self) -> bool {
        self.field(26, 1) != 0
    }

    pub fn fmask_compress_1frag_only(self) -> bool {
        self.field(27, 1) != 0
    }

    pub fn dcc_enable(self) -> bool {
        self.field(28, 1) != 0
    }

    pub fn cmask_addr_type(self) -> u32 {
        self.field(29, 2)
    }

    pub fn alt_tile_mode(self) -> bool {
        self.field(31, 1) != 0
    }
}

/// `ColorBuffer::Color0Attrib`, the packed `attrib` word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Color0Attrib {
    pub raw: u32,
}

impl Color0Attrib {
    fn field(self, offset: u32, bits: u32) -> u32 {
        (self.raw >> offset) & ((1 << bits) - 1)
    }

    pub fn tile_mode_index(self) -> u32 {
        self.field(0, 5)
    }

    pub fn fmask_tile_mode_index(self) -> u32 {
        self.field(5, 5)
    }

    pub fn fmask_bank_height(self) -> u32 {
        self.field(10, 2)
    }

    pub fn num_samples_log2(self) -> u32 {
        self.field(12, 3)
    }

    pub fn num_fragments_log2(self) -> u32 {
        self.field(15, 2)
    }

    pub fn force_dst_alpha_1(self) -> bool {
        self.field(17, 1) != 0
    }
}

/// `ColorBuffer::pitch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorPitch(pub u32);

impl ColorPitch {
    pub fn tile_max(self) -> u32 {
        self.0 & 0x7ff
    }

    pub fn fmask_tile_max(self) -> u32 {
        (self.0 >> 20) & 0x7ff
    }
}

/// `ColorBuffer::slice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorSlice(pub u32);

impl ColorSlice {
    pub fn tile_max(self) -> u32 {
        self.0 & 0x3f_ffff
    }
}

/// `ColorBuffer::view`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorView(pub u32);

impl ColorView {
    pub fn slice_start(self) -> u32 {
        self.0 & 0x7ff
    }

    pub fn slice_max(self) -> u32 {
        (self.0 >> 13) & 0x7ff
    }
}

/// `ColorBuffer::cmask_slice` and `fmask_slice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ColorMaskSlice(pub u32);

impl ColorMaskSlice {
    pub fn tile_max(self) -> u32 {
        self.0 & 0x3fff
    }
}

/// `AmdGpu::ColorBuffer`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColorBuffer {
    pub base_address: u32,
    pub pitch: ColorPitch,
    pub slice: ColorSlice,
    pub view: ColorView,
    pub info: Color0Info,
    pub attrib: Color0Attrib,
    pub pad0: u32,
    pub cmask_base_address: u32,
    pub cmask_slice: ColorMaskSlice,
    pub fmask_base_address: u32,
    pub fmask_slice: ColorMaskSlice,
    pub clear_word0: u32,
    pub clear_word1: u32,
    pub pad1: [u32; 2],
}

impl ColorBuffer {
    /// The C++ `operator bool`.
    pub fn is_valid(&self) -> bool {
        self.base_address != 0 && self.info.format() != 0
    }

    pub fn pitch(&self) -> u32 {
        (self.pitch.tile_max() + 1) << 3
    }

    pub fn height(&self) -> u32 {
        (self.slice.tile_max() + 1) * 64 / self.pitch()
    }

    pub fn address(&self) -> u64 {
        (u64::from(self.base_address) << 8)
            | if self.info.linear_general() {
                u64::from(self.view.slice_start() & 0xff)
            } else {
                0
            }
    }

    pub fn cmask_address(&self) -> u64 {
        u64::from(self.cmask_base_address) << 8
    }

    pub fn fmask_address(&self) -> u64 {
        u64::from(self.fmask_base_address) << 8
    }

    pub fn num_samples(&self) -> u32 {
        1 << self.attrib.num_fragments_log2()
    }

    pub fn base_slice(&self) -> u32 {
        if self.info.linear_general() {
            0
        } else {
            self.view.slice_start()
        }
    }

    pub fn num_slices(&self) -> u32 {
        self.view.slice_max() + 1
    }

    pub fn color_slice_size(&self) -> u32 {
        let format = DataFormat::from_raw(self.info.format()).unwrap_or(DataFormat::FormatInvalid);
        let bytes_per_element = num_bits_per_block(format).unwrap_or(0) / 8;
        bytes_per_element * (self.slice.tile_max() + 1) * 64 * self.num_samples()
    }

    pub fn tile_mode(&self) -> TileMode {
        if self.info.linear_general() {
            TileMode::DisplayLinearGeneral
        } else {
            TileMode::from_raw(self.attrib.tile_mode_index())
                .unwrap_or(TileMode::DisplayLinearAligned)
        }
    }

    pub fn is_tiled(&self) -> bool {
        let mode = self.tile_mode();
        mode != TileMode::DisplayLinearAligned && mode != TileMode::DisplayLinearGeneral
    }

    pub fn data_fmt(&self) -> DataFormat {
        let format = DataFormat::from_raw(self.info.format()).unwrap_or(DataFormat::FormatInvalid);
        remap_data_format(format)
    }

    pub fn number_fmt(&self) -> NumberFormat {
        let format = DataFormat::from_raw(self.info.format()).unwrap_or(DataFormat::FormatInvalid);
        remap_number_format(self.fixed_number_format(), format)
    }

    pub fn number_conversion(&self) -> NumberConversion {
        let format = DataFormat::from_raw(self.info.format()).unwrap_or(DataFormat::FormatInvalid);
        map_number_conversion(self.fixed_number_format(), format)
    }

    pub fn fixed_number_format(&self) -> NumberFormat {
        let number = NumberFormat::from_raw(self.info.number_type()).unwrap_or(NumberFormat::Unorm);
        if number == NumberFormat::SnormNz {
            NumberFormat::Srgb
        } else {
            number
        }
    }

    pub fn swizzle(&self) -> CompMapping {
        const RED: CompSwizzle = CompSwizzle::Red;
        const GREEN: CompSwizzle = CompSwizzle::Green;
        const BLUE: CompSwizzle = CompSwizzle::Blue;
        const ALPHA: CompSwizzle = CompSwizzle::Alpha;
        const ZERO: CompSwizzle = CompSwizzle::Zero;
        const fn mapping(
            r: CompSwizzle,
            g: CompSwizzle,
            b: CompSwizzle,
            a: CompSwizzle,
        ) -> CompMapping {
            CompMapping { r, g, b, a }
        }
        let mrt_swizzles = [
            [
                mapping(RED, ZERO, ZERO, ZERO),
                mapping(RED, GREEN, ZERO, ZERO),
                mapping(RED, GREEN, BLUE, ZERO),
                mapping(RED, GREEN, BLUE, ALPHA),
            ],
            [
                mapping(GREEN, ZERO, ZERO, ZERO),
                mapping(RED, ALPHA, ZERO, ZERO),
                mapping(RED, GREEN, ALPHA, ZERO),
                mapping(BLUE, GREEN, RED, ALPHA),
            ],
            [
                mapping(BLUE, ZERO, ZERO, ZERO),
                mapping(GREEN, RED, ZERO, ZERO),
                mapping(BLUE, GREEN, RED, ZERO),
                mapping(ALPHA, BLUE, GREEN, RED),
            ],
            [
                mapping(ALPHA, ZERO, ZERO, ZERO),
                mapping(ALPHA, RED, ZERO, ZERO),
                mapping(ALPHA, GREEN, RED, ZERO),
                mapping(ALPHA, RED, GREEN, BLUE),
            ],
        ];
        let swap_index = self.info.comp_swap() as usize;
        let format = DataFormat::from_raw(self.info.format()).unwrap_or(DataFormat::FormatInvalid);
        let components_index = num_components(format).unwrap_or(0).wrapping_sub(1) as usize;
        let mrt_swizzle = mrt_swizzles[swap_index][components_index];
        remap_swizzle(format, mrt_swizzle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_color_buffer_decodes_its_address_and_pitch() {
        let mut buffer = ColorBuffer::default();
        assert!(!buffer.is_valid());
        buffer.base_address = 0x1234;
        buffer.info.raw = 2 << 2; // format 2
        assert!(buffer.is_valid());
        assert_eq!(buffer.address(), 0x1234 << 8);
        buffer.pitch = ColorPitch(3);
        buffer.slice = ColorSlice(7);
        assert_eq!(buffer.pitch(), 4 << 3);
        assert_eq!(buffer.height(), 8 * 64 / (4 << 3));
    }

    #[test]
    fn the_color_mask_reads_and_writes_each_nibble() {
        let mut mask = ColorBufferMask::default();
        mask.set_mask(2, 0b1010);
        assert_eq!(mask.raw, 0b1010 << 8);
        assert_eq!(mask.get_mask(2), 0b1010);
        assert_eq!(mask.get_mask(0), 0);
        mask.set_mask(2, 0b0101);
        assert_eq!(mask.get_mask(2), 0b0101);
    }

    #[test]
    fn the_fixed_number_format_maps_snorm_nz_to_srgb() {
        let mut buffer = ColorBuffer::default();
        buffer.info.raw = NumberFormat::SnormNz.raw() << 8;
        assert_eq!(buffer.fixed_number_format(), NumberFormat::Srgb);
    }
}
