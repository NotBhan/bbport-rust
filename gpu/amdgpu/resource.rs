//! Buffer, image and sampler resource descriptors, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/resource.h`. Each descriptor is the words the hardware
//! holds it in, with the C++'s bit fields as accessors; the dword views are how the recompiler
//! pokes a single word the way the C++ does through `reinterpret_cast`.

use crate::gpu::amdgpu::pixel_format::{
    map_number_conversion, num_bits_per_element, remap_data_format, remap_number_format,
    remap_swizzle, CompMapping, CompSwizzle, DataFormat, NumberConversion, NumberFormat,
};
use crate::gpu::amdgpu::tiling::{
    calculate_macrotile_mode, get_alt_num_banks, get_array_mode, is_macro_tiled, TileMode,
};
use crate::gpu::common::alignment::align_up_pow2;

/// `AmdGpu::Buffer`: Table 8.5's buffer resource descriptor, 128 bits wide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buffer {
    /// `base_address` in the low 40 bits, then `_padding0`, `stride`, `cache_swizzle` and
    /// `swizzle_enable`.
    pub word0: u64,
    pub num_records: u32,
    pub word2: u32,
}

const STRIDE_POSITION: u32 = 48;
const CACHE_SWIZZLE_POSITION: u32 = 62;
const SWIZZLE_ENABLE_POSITION: u32 = 63;

impl Buffer {
    /// `Buffer::Null`: an unbound descriptor, which is one with a base address of one.
    pub const fn null() -> Self {
        Buffer {
            word0: 1,
            num_records: 0,
            word2: 0,
        }
    }

    /// `Buffer::Placeholder`.
    pub const fn placeholder(size: u32) -> Self {
        Buffer {
            word0: 1,
            num_records: size,
            word2: 0,
        }
    }

    /// `Buffer::Valid`.
    pub fn is_valid(&self) -> bool {
        self.type_() == 0
    }

    /// The C++'s `operator bool`: a buffer with no records is not bound.
    pub fn is_bound(&self) -> bool {
        self.num_records != 0
    }

    /// `reinterpret_cast<u32*>(&buffer)`: the four words the C++ can see.
    pub fn dwords(&self) -> [u32; 4] {
        [
            self.word0 as u32,
            (self.word0 >> 32) as u32,
            self.num_records,
            self.word2,
        ]
    }

    /// The same words back into a descriptor, which is where the C++'s casts leave them.
    pub fn from_dwords(dwords: &[u32; 4]) -> Self {
        Buffer {
            word0: dwords[0] as u64 | ((dwords[1] as u64) << 32),
            num_records: dwords[2],
            word2: dwords[3],
        }
    }

    pub fn base_address(&self) -> u64 {
        self.word0 & 0xFF_FFFF_FFFF
    }

    pub fn set_base_address(&mut self, value: u64) {
        self.word0 = (self.word0 & !0xFF_FFFF_FFFF) | (value & 0xFF_FFFF_FFFF);
    }

    pub fn stride(&self) -> u64 {
        (self.word0 >> STRIDE_POSITION) & 0x3FFF
    }

    pub fn cache_swizzle(&self) -> bool {
        (self.word0 >> CACHE_SWIZZLE_POSITION) & 1 != 0
    }

    pub fn swizzle_enable(&self) -> bool {
        (self.word0 >> SWIZZLE_ENABLE_POSITION) & 1 != 0
    }

    fn field(&self, position: u32, bits: u32) -> u32 {
        (self.word2 >> position) & ((1 << bits) - 1)
    }

    pub fn dst_sel_x(&self) -> u32 {
        self.field(0, 3)
    }

    pub fn dst_sel_y(&self) -> u32 {
        self.field(3, 3)
    }

    pub fn dst_sel_z(&self) -> u32 {
        self.field(6, 3)
    }

    pub fn dst_sel_w(&self) -> u32 {
        self.field(9, 3)
    }

    pub fn num_format(&self) -> u32 {
        self.field(12, 3)
    }

    pub fn data_format(&self) -> u32 {
        self.field(15, 4)
    }

    pub fn set_data_format(&mut self, value: u32) {
        self.word2 = (self.word2 & !(0xF << 15)) | ((value & 0xF) << 15);
    }

    pub fn set_num_format(&mut self, value: u32) {
        self.word2 = (self.word2 & !(7 << 12)) | ((value & 7) << 12);
    }

    pub fn element_size(&self) -> u32 {
        self.field(19, 2)
    }

    pub fn index_stride(&self) -> u32 {
        self.field(21, 2)
    }

    pub fn add_tid_enable(&self) -> bool {
        self.field(23, 1) != 0
    }

    pub fn mtype(&self) -> u32 {
        self.field(27, 3)
    }

    /// `type`: it overlaps with a texture descriptor's, so a buffer has zero.
    pub fn type_(&self) -> u32 {
        self.field(30, 2)
    }

    /// `DstSelect`.
    pub fn dst_select(&self) -> CompMapping {
        let dst_sel = CompMapping {
            r: CompSwizzle::from_raw(self.dst_sel_x() as u8).unwrap_or(CompSwizzle::Zero),
            g: CompSwizzle::from_raw(self.dst_sel_y() as u8).unwrap_or(CompSwizzle::Zero),
            b: CompSwizzle::from_raw(self.dst_sel_z() as u8).unwrap_or(CompSwizzle::Zero),
            a: CompSwizzle::from_raw(self.dst_sel_w() as u8).unwrap_or(CompSwizzle::Zero),
        };
        remap_swizzle(
            DataFormat::from_raw(self.data_format()).unwrap_or(DataFormat::FormatInvalid),
            dst_sel,
        )
    }

    pub fn get_number_fmt(&self) -> NumberFormat {
        let data_format =
            DataFormat::from_raw(self.data_format()).unwrap_or(DataFormat::FormatInvalid);
        remap_number_format(
            NumberFormat::from_raw(self.num_format()).unwrap_or(NumberFormat::Unorm),
            data_format,
        )
    }

    pub fn get_data_fmt(&self) -> DataFormat {
        remap_data_format(
            DataFormat::from_raw(self.data_format()).unwrap_or(DataFormat::FormatInvalid),
        )
    }

    pub fn get_number_conversion(&self) -> NumberConversion {
        let data_format =
            DataFormat::from_raw(self.data_format()).unwrap_or(DataFormat::FormatInvalid);
        map_number_conversion(
            NumberFormat::from_raw(self.num_format()).unwrap_or(NumberFormat::Unorm),
            data_format,
        )
    }

    pub fn get_stride(&self) -> u32 {
        self.stride() as u32
    }

    /// `NumDwords`.
    pub fn num_dwords(&self) -> u32 {
        (align_up_pow2(self.get_size() as u64, 4) >> 2) as u32
    }

    /// `GetSize`: a stride of zero means the records are one dword each.
    pub fn get_size(&self) -> u32 {
        if self.stride() == 0 {
            self.num_records
        } else {
            self.stride() as u32 * self.num_records
        }
    }

    /// `GetIndexStride`: two bits meaning 8, 16, 32 or 64.
    pub fn get_index_stride(&self) -> u32 {
        8 << self.index_stride()
    }

    /// `GetElementSize`: two bits meaning 2, 4, 8 or 16.
    pub fn get_element_size(&self) -> u32 {
        2 << self.element_size()
    }
}

/// `AmdGpu::ImageType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum ImageType {
    Invalid = 0,
    Color1D = 8,
    Color2D = 9,
    Color3D = 10,
    Cube = 11,
    Color1DArray = 12,
    Color2DArray = 13,
    Color2DMsaa = 14,
    Color2DMsaaArray = 15,
}

impl ImageType {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    /// A descriptor's type field holds four bits, which the C++ casts straight to the type.
    pub fn from_raw(value: u64) -> Self {
        match value {
            8 => ImageType::Color1D,
            9 => ImageType::Color2D,
            10 => ImageType::Color3D,
            11 => ImageType::Cube,
            12 => ImageType::Color1DArray,
            13 => ImageType::Color2DArray,
            14 => ImageType::Color2DMsaa,
            15 => ImageType::Color2DMsaaArray,
            _ => ImageType::Invalid,
        }
    }
}

/// `AmdGpu::NameOf(ImageType)`.
pub fn name_of_image_type(type_: ImageType) -> &'static str {
    match type_ {
        ImageType::Invalid => "Invalid",
        ImageType::Color1D => "Color1D",
        ImageType::Color2D => "Color2D",
        ImageType::Color3D => "Color3D",
        ImageType::Cube => "Cube",
        ImageType::Color1DArray => "Color1DArray",
        ImageType::Color2DArray => "Color2DArray",
        ImageType::Color2DMsaa => "Color2DMsaa",
        ImageType::Color2DMsaaArray => "Color2DMsaaArray",
    }
}

/// The header's `fmt::formatter<AmdGpu::ImageType>`.
impl std::fmt::Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(name_of_image_type(*self))
    }
}

/// `AmdGpu::Image`: 256 bits wide, in four 64-bit words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image {
    pub word0: u64,
    pub word1: u64,
    pub word2: u64,
    pub word3: u64,
}

fn field64(word: u64, position: u32, bits: u32) -> u64 {
    (word >> position) & ((1u64 << bits) - 1)
}

impl Image {
    /// `Image::Null`.
    pub fn null(is_depth: bool) -> Self {
        let mut image = Image {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: 0,
        };
        image.set_data_format(if is_depth {
            DataFormat::Format32.raw()
        } else {
            DataFormat::Format8_8_8_8.raw()
        } as u64);
        image.set_num_format(if is_depth {
            NumberFormat::Float.raw()
        } else {
            NumberFormat::Unorm.raw()
        } as u64);
        image.set_dst_sel_x(CompSwizzle::Red as u64);
        image.set_dst_sel_y(CompSwizzle::Green as u64);
        image.set_dst_sel_z(CompSwizzle::Blue as u64);
        image.set_dst_sel_w(CompSwizzle::Alpha as u64);
        image.set_tiling_index(TileMode::Thin1DThin.raw() as u64);
        image.set_type(ImageType::Color2D.raw());
        image
    }

    /// `Valid`: the type's top bit tells an image from a buffer descriptor.
    pub fn is_valid(&self) -> bool {
        self.type_() & 0x8 != 0
    }

    pub fn address(&self) -> u64 {
        self.base_address() << 8
    }

    /// The C++'s `operator bool`.
    pub fn is_bound(&self) -> bool {
        self.base_address() != 0
    }

    pub fn dwords(&self) -> [u32; 8] {
        [
            self.word0 as u32,
            (self.word0 >> 32) as u32,
            self.word1 as u32,
            (self.word1 >> 32) as u32,
            self.word2 as u32,
            (self.word2 >> 32) as u32,
            self.word3 as u32,
            (self.word3 >> 32) as u32,
        ]
    }

    pub fn from_dwords(dwords: &[u32; 8]) -> Self {
        Image {
            word0: dwords[0] as u64 | ((dwords[1] as u64) << 32),
            word1: dwords[2] as u64 | ((dwords[3] as u64) << 32),
            word2: dwords[4] as u64 | ((dwords[5] as u64) << 32),
            word3: dwords[6] as u64 | ((dwords[7] as u64) << 32),
        }
    }

    pub fn base_address(&self) -> u64 {
        self.word0 & 0x3F_FFFF_FFFF
    }

    pub fn mtype_l2(&self) -> u64 {
        field64(self.word0, 38, 2)
    }

    pub fn min_lod(&self) -> u64 {
        field64(self.word0, 40, 12)
    }

    pub fn data_format(&self) -> u64 {
        field64(self.word0, 52, 6)
    }

    pub fn set_data_format(&mut self, value: u64) {
        self.word0 = (self.word0 & !(0x3F << 52)) | ((value & 0x3F) << 52);
    }

    pub fn num_format(&self) -> u64 {
        field64(self.word0, 58, 4)
    }

    pub fn set_num_format(&mut self, value: u64) {
        self.word0 = (self.word0 & !(0xF << 58)) | ((value & 0xF) << 58);
    }

    pub fn mtype(&self) -> u64 {
        field64(self.word0, 62, 2)
    }

    pub fn width(&self) -> u64 {
        field64(self.word1, 0, 14)
    }

    pub fn height(&self) -> u64 {
        field64(self.word1, 14, 14)
    }

    pub fn perf_modulation(&self) -> u64 {
        field64(self.word1, 28, 3)
    }

    pub fn interlaced(&self) -> bool {
        field64(self.word1, 31, 1) != 0
    }

    pub fn dst_sel_x(&self) -> u64 {
        field64(self.word1, 32, 3)
    }

    pub fn set_dst_sel_x(&mut self, value: u64) {
        self.word1 = (self.word1 & !(7 << 32)) | ((value & 7) << 32);
    }

    pub fn dst_sel_y(&self) -> u64 {
        field64(self.word1, 35, 3)
    }

    pub fn set_dst_sel_y(&mut self, value: u64) {
        self.word1 = (self.word1 & !(7 << 35)) | ((value & 7) << 35);
    }

    pub fn dst_sel_z(&self) -> u64 {
        field64(self.word1, 38, 3)
    }

    pub fn set_dst_sel_z(&mut self, value: u64) {
        self.word1 = (self.word1 & !(7 << 38)) | ((value & 7) << 38);
    }

    pub fn dst_sel_w(&self) -> u64 {
        field64(self.word1, 41, 3)
    }

    pub fn set_dst_sel_w(&mut self, value: u64) {
        self.word1 = (self.word1 & !(7 << 41)) | ((value & 7) << 41);
    }

    pub fn base_level(&self) -> u64 {
        field64(self.word1, 44, 4)
    }

    pub fn last_level(&self) -> u64 {
        field64(self.word1, 48, 4)
    }

    pub fn tiling_index(&self) -> u64 {
        field64(self.word1, 52, 5)
    }

    pub fn set_tiling_index(&mut self, value: u64) {
        self.word1 = (self.word1 & !(0x1F << 52)) | ((value & 0x1F) << 52);
    }

    pub fn pow2pad(&self) -> bool {
        field64(self.word1, 57, 1) != 0
    }

    pub fn mtype2(&self) -> bool {
        field64(self.word1, 58, 1) != 0
    }

    pub fn atc(&self) -> bool {
        field64(self.word1, 59, 1) != 0
    }

    pub fn type_(&self) -> u64 {
        field64(self.word1, 60, 4)
    }

    pub fn set_type(&mut self, value: u64) {
        self.word1 = (self.word1 & !(0xF << 60)) | ((value & 0xF) << 60);
    }

    pub fn depth(&self) -> u64 {
        field64(self.word2, 0, 13)
    }

    pub fn set_depth(&mut self, value: u64) {
        self.word2 = (self.word2 & !0x1FFF) | (value & 0x1FFF);
    }

    pub fn pitch(&self) -> u64 {
        field64(self.word2, 13, 14)
    }

    pub fn set_pitch(&mut self, value: u64) {
        self.word2 = (self.word2 & !(0x3FFF << 13)) | ((value & 0x3FFF) << 13);
    }

    pub fn base_array(&self) -> u64 {
        field64(self.word2, 32, 13)
    }

    pub fn last_array(&self) -> u64 {
        field64(self.word2, 45, 13)
    }

    pub fn min_lod_warn(&self) -> u64 {
        field64(self.word3, 0, 12)
    }

    pub fn counter_bank_id(&self) -> u64 {
        field64(self.word3, 12, 8)
    }

    pub fn lod_hw_cnt_en(&self) -> bool {
        field64(self.word3, 20, 1) != 0
    }

    /// Neo-mode only.
    pub fn compression_en(&self) -> bool {
        field64(self.word3, 21, 1) != 0
    }

    /// Neo-mode only.
    pub fn alpha_is_on_msb(&self) -> bool {
        field64(self.word3, 22, 1) != 0
    }

    /// Neo-mode only.
    pub fn color_transform(&self) -> bool {
        field64(self.word3, 23, 1) != 0
    }

    /// Neo-mode only.
    pub fn alt_tile_mode(&self) -> bool {
        field64(self.word3, 24, 1) != 0
    }

    /// `DstSelect`.
    pub fn dst_select(&self) -> CompMapping {
        let dst_sel = CompMapping {
            r: CompSwizzle::from_raw(self.dst_sel_x() as u8).unwrap_or(CompSwizzle::Zero),
            g: CompSwizzle::from_raw(self.dst_sel_y() as u8).unwrap_or(CompSwizzle::Zero),
            b: CompSwizzle::from_raw(self.dst_sel_z() as u8).unwrap_or(CompSwizzle::Zero),
            a: CompSwizzle::from_raw(self.dst_sel_w() as u8).unwrap_or(CompSwizzle::Zero),
        };
        remap_swizzle(
            DataFormat::from_raw(self.data_format() as u32).unwrap_or(DataFormat::FormatInvalid),
            dst_sel,
        )
    }

    /// `Pitch`: the field is one less than the pitch.
    pub fn get_pitch(&self) -> u32 {
        self.pitch() as u32 + 1
    }

    /// `NumLayers`: depth is the number of layers, except on 3D images where it is the depth.
    pub fn num_layers(&self) -> u32 {
        let mut slices = self.depth() as u32 + 1;
        if self.get_type() == ImageType::Color3D {
            slices = 1;
        } else if self.is_cube() {
            slices *= 6;
        }
        if self.pow2pad() {
            slices = slices.next_power_of_two();
        }
        slices
    }

    /// `NumLevels`.
    pub fn num_levels(&self) -> u32 {
        if matches!(
            self.get_type(),
            ImageType::Color2DMsaa | ImageType::Color2DMsaaArray
        ) {
            return 1;
        }
        self.last_level() as u32 + 1
    }

    /// `NumSamples`.
    pub fn num_samples(&self) -> u32 {
        if matches!(
            self.get_type(),
            ImageType::Color2DMsaa | ImageType::Color2DMsaaArray
        ) {
            return 1 << self.last_level();
        }
        1
    }

    pub fn is_cube(&self) -> bool {
        ImageType::from_raw(self.type_()) == ImageType::Cube
    }

    /// `GetType`: a cube is read as an array of 2D images.
    pub fn get_type(&self) -> ImageType {
        if self.is_cube() {
            ImageType::Color2DArray
        } else {
            ImageType::from_raw(self.type_())
        }
    }

    pub fn get_data_fmt(&self) -> DataFormat {
        remap_data_format(
            DataFormat::from_raw(self.data_format() as u32).unwrap_or(DataFormat::FormatInvalid),
        )
    }

    pub fn get_number_fmt(&self) -> NumberFormat {
        let data_format =
            DataFormat::from_raw(self.data_format() as u32).unwrap_or(DataFormat::FormatInvalid);
        remap_number_format(
            NumberFormat::from_raw(self.num_format() as u32).unwrap_or(NumberFormat::Unorm),
            data_format,
        )
    }

    pub fn get_number_conversion(&self) -> NumberConversion {
        let data_format =
            DataFormat::from_raw(self.data_format() as u32).unwrap_or(DataFormat::FormatInvalid);
        map_number_conversion(
            NumberFormat::from_raw(self.num_format() as u32).unwrap_or(NumberFormat::Unorm),
            data_format,
        )
    }

    pub fn get_tile_mode(&self) -> TileMode {
        // An index outside the enumeration has no tile mode; the port reads it as linear.
        TileMode::from_raw(self.tiling_index() as u32).unwrap_or(TileMode::DisplayLinearAligned)
    }

    /// `IsTiled`.
    pub fn is_tiled(&self) -> bool {
        !matches!(
            self.get_tile_mode(),
            TileMode::DisplayLinearAligned | TileMode::DisplayLinearGeneral
        )
    }

    /// `GetBankSwizzle`.
    pub fn get_bank_swizzle(&self) -> u8 {
        let tile_mode = self.get_tile_mode();
        let array_mode = get_array_mode(tile_mode);
        let dfmt = self.get_data_fmt();
        if !self.alt_tile_mode() || dfmt == DataFormat::FormatInvalid || !is_macro_tiled(array_mode)
        {
            return 0;
        }
        let Some(bpp) = num_bits_per_element(dfmt) else {
            return 0;
        };
        let Ok(macro_tile_mode) = calculate_macrotile_mode(tile_mode, bpp, self.num_samples())
        else {
            return 0;
        };
        let banks = get_alt_num_banks(macro_tile_mode);
        ((((banks - 1) << 4) & self.base_address() as u32) >> 4) as u8
    }

    /// `GetBaseType`: the array and multisample forms reduce to their 2D base.
    pub fn get_base_type(&self) -> ImageType {
        match self.get_type() {
            ImageType::Color1DArray => ImageType::Color1D,
            ImageType::Color2DArray | ImageType::Color2DMsaa | ImageType::Color2DMsaaArray => {
                ImageType::Color2D
            }
            base_type => base_type,
        }
    }

    /// `GetViewType`.
    pub fn get_view_type(&self, is_array: bool) -> ImageType {
        let base_type = self.get_type();
        if self.is_cube() {
            // Cube needs to remain array type regardless of instruction array specifier.
            return base_type;
        }
        if base_type == ImageType::Color1DArray && !is_array {
            return ImageType::Color1D;
        }
        if base_type == ImageType::Color2DArray && !is_array {
            return ImageType::Color2D;
        }
        if base_type == ImageType::Color2DMsaaArray && !is_array {
            return ImageType::Color2DMsaa;
        }
        base_type
    }

    /// `NumViewLevels`.
    pub fn num_view_levels(&self, is_array: bool) -> u32 {
        match self.get_view_type(is_array) {
            ImageType::Color2DMsaa | ImageType::Color2DMsaaArray => 1,
            _ => {
                // Constrain to actual number of available levels.
                let max_level = (self.last_level() as u32 + 1).min(self.num_levels());
                if max_level > self.base_level() as u32 {
                    max_level - self.base_level() as u32
                } else {
                    1
                }
            }
        }
    }

    /// `NumViewLayers`.
    pub fn num_view_layers(&self, is_array: bool) -> u32 {
        match self.get_view_type(is_array) {
            ImageType::Color1D
            | ImageType::Color2D
            | ImageType::Color2DMsaa
            | ImageType::Color3D => 1,
            _ => {
                // Constrain to actual number of available layers.
                let max_array = (self.last_array() as u32 + 1).min(self.num_layers());
                if max_array > self.base_array() as u32 {
                    max_array - self.base_array() as u32
                } else {
                    1
                }
            }
        }
    }
}

/// A `u64`-backed enumeration, as the sampler's fields are.
macro_rules! u64_enum {
    ($name:ident { $($variant:ident = $value:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u64)]
        pub enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub fn from_raw(value: u64) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub const fn raw(self) -> u64 {
                self as u64
            }
        }
    };
}

u64_enum!(ClampMode {
    Wrap = 0,
    Mirror = 1,
    ClampLastTexel = 2,
    MirrorOnceLastTexel = 3,
    ClampHalfBorder = 4,
    MirrorOnceHalfBorder = 5,
    ClampBorder = 6,
    MirrorOnceBorder = 7,
});

u64_enum!(AnisoRatio {
    One = 0,
    Two = 1,
    Four = 2,
    Eight = 3,
    Sixteen = 4,
});

u64_enum!(DepthCompare {
    Never = 0,
    Less = 1,
    Equal = 2,
    LessEqual = 3,
    Greater = 4,
    NotEqual = 5,
    GreaterEqual = 6,
    Always = 7,
});

u64_enum!(FilterMode {
    Blend = 0,
    Min = 1,
    Max = 2,
});

u64_enum!(Filter {
    Point = 0,
    Bilinear = 1,
    AnisoPoint = 2,
    AnisoLinear = 3,
});

/// `AmdGpu::IsAnisoFilter`.
pub fn is_aniso_filter(filter: Filter) -> bool {
    filter == Filter::AnisoPoint || filter == Filter::AnisoLinear
}

u64_enum!(MipFilter {
    None = 0,
    Point = 1,
    Linear = 2,
});

u64_enum!(BorderColor {
    TransparentBlack = 0,
    OpaqueBlack = 1,
    White = 2,
    Custom = 3,
});

/// `AmdGpu::Sampler`: Table 8.12's sampler resource descriptor, two words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sampler {
    pub raw0: u64,
    pub raw1: u64,
}

impl Sampler {
    /// The C++'s `operator bool`.
    pub fn is_bound(&self) -> bool {
        self.raw0 != 0 || self.raw1 != 0
    }

    /// `Sampler::Valid`, which the header always answers yes to.
    pub fn is_valid(&self) -> bool {
        true
    }

    pub fn dwords(&self) -> [u32; 4] {
        [
            self.raw0 as u32,
            (self.raw0 >> 32) as u32,
            self.raw1 as u32,
            (self.raw1 >> 32) as u32,
        ]
    }

    pub fn from_dwords(dwords: &[u32; 4]) -> Self {
        Sampler {
            raw0: dwords[0] as u64 | ((dwords[1] as u64) << 32),
            raw1: dwords[2] as u64 | ((dwords[3] as u64) << 32),
        }
    }

    fn set_field(value: u64, position: u32, bits: u32, new_value: u64) -> u64 {
        (value & !(((1u64 << bits) - 1) << position))
            | ((new_value & ((1u64 << bits) - 1)) << position)
    }

    pub fn clamp_x(&self) -> ClampMode {
        ClampMode::from_raw(field64(self.raw0, 0, 3)).unwrap_or(ClampMode::Wrap)
    }

    /// The decoder's `clamp_x.Assign(...)`.
    pub fn set_clamp_x(&mut self, value: ClampMode) {
        self.raw0 = Self::set_field(self.raw0, 0, 3, value.raw());
    }

    pub fn clamp_y(&self) -> ClampMode {
        ClampMode::from_raw(field64(self.raw0, 3, 3)).unwrap_or(ClampMode::Wrap)
    }

    pub fn set_clamp_y(&mut self, value: ClampMode) {
        self.raw0 = Self::set_field(self.raw0, 3, 3, value.raw());
    }

    pub fn clamp_z(&self) -> ClampMode {
        ClampMode::from_raw(field64(self.raw0, 6, 3)).unwrap_or(ClampMode::Wrap)
    }

    pub fn set_clamp_z(&mut self, value: ClampMode) {
        self.raw0 = Self::set_field(self.raw0, 6, 3, value.raw());
    }

    pub fn max_aniso(&self) -> AnisoRatio {
        AnisoRatio::from_raw(field64(self.raw0, 9, 3)).unwrap_or(AnisoRatio::One)
    }

    pub fn set_max_aniso(&mut self, value: AnisoRatio) {
        self.raw0 = Self::set_field(self.raw0, 9, 3, value.raw());
    }

    pub fn depth_compare_func(&self) -> DepthCompare {
        DepthCompare::from_raw(field64(self.raw0, 12, 3)).unwrap_or(DepthCompare::Never)
    }

    pub fn force_unnormalized(&self) -> u64 {
        field64(self.raw0, 15, 1)
    }

    pub fn aniso_threshold(&self) -> u64 {
        field64(self.raw0, 16, 3)
    }

    pub fn set_aniso_threshold(&mut self, value: u64) {
        self.raw0 = Self::set_field(self.raw0, 16, 3, value);
    }

    pub fn mc_coord_trunc(&self) -> u64 {
        field64(self.raw0, 19, 1)
    }

    pub fn force_degamma(&self) -> u64 {
        field64(self.raw0, 20, 1)
    }

    pub fn aniso_bias(&self) -> u64 {
        field64(self.raw0, 21, 6)
    }

    pub fn trunc_coord(&self) -> u64 {
        field64(self.raw0, 27, 1)
    }

    pub fn disable_cube_wrap(&self) -> u64 {
        field64(self.raw0, 28, 1)
    }

    pub fn filter_mode(&self) -> FilterMode {
        FilterMode::from_raw(field64(self.raw0, 29, 2)).unwrap_or(FilterMode::Blend)
    }

    pub fn min_lod(&self) -> u64 {
        field64(self.raw0, 32, 12)
    }

    pub fn max_lod(&self) -> u64 {
        field64(self.raw0, 44, 12)
    }

    pub fn perf_mip(&self) -> u64 {
        field64(self.raw0, 56, 4)
    }

    pub fn perf_z(&self) -> u64 {
        field64(self.raw0, 60, 4)
    }

    pub fn lod_bias(&self) -> u64 {
        field64(self.raw1, 0, 14)
    }

    pub fn lod_bias_sec(&self) -> u64 {
        field64(self.raw1, 14, 6)
    }

    pub fn xy_mag_filter(&self) -> Filter {
        Filter::from_raw(field64(self.raw1, 20, 2)).unwrap_or(Filter::Point)
    }

    pub fn xy_min_filter(&self) -> Filter {
        Filter::from_raw(field64(self.raw1, 22, 2)).unwrap_or(Filter::Point)
    }

    pub fn z_filter(&self) -> u64 {
        field64(self.raw1, 24, 2)
    }

    pub fn mip_filter(&self) -> MipFilter {
        MipFilter::from_raw(field64(self.raw1, 26, 2)).unwrap_or(MipFilter::None)
    }

    pub fn mip_point_preclamp(&self) -> u64 {
        field64(self.raw1, 28, 1)
    }

    pub fn disable_lsb_ceil(&self) -> u64 {
        field64(self.raw1, 29, 1)
    }

    pub fn unused0(&self) -> u64 {
        field64(self.raw1, 30, 2)
    }

    pub fn border_color_ptr(&self) -> u64 {
        field64(self.raw1, 32, 12)
    }

    pub fn unused1(&self) -> u64 {
        field64(self.raw1, 42, 18)
    }

    pub fn border_color_type(&self) -> BorderColor {
        BorderColor::from_raw(field64(self.raw1, 62, 2)).unwrap_or(BorderColor::TransparentBlack)
    }

    /// `LodBias`: the field is a signed ninth-format value, and the C++ builds it with a xor.
    pub fn get_lod_bias(&self) -> f32 {
        let value = ((self.lod_bias() as u16 ^ 0x2000).wrapping_sub(0x2000)) as i16;
        value as f32 / 256.0
    }

    /// `MinLod`.
    pub fn get_min_lod(&self) -> f32 {
        self.min_lod() as f32 / 256.0
    }

    /// `MaxLod`.
    pub fn get_max_lod(&self) -> f32 {
        self.max_lod() as f32 / 256.0
    }

    /// `MaxAniso`.
    pub fn get_max_aniso(&self) -> f32 {
        match self.max_aniso() {
            AnisoRatio::One => 1.0,
            AnisoRatio::Two => 2.0,
            AnisoRatio::Four => 4.0,
            AnisoRatio::Eight => 8.0,
            AnisoRatio::Sixteen => 16.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffer_descriptor_keeps_its_fields() {
        // A stride of 16, 4 records, a data format of 10 and a number format of 0.
        let mut buffer = Buffer::default();
        assert_eq!(buffer.dwords(), [0; 4]);
        assert!(!buffer.is_bound());
        assert!(buffer.is_valid(), "type 0 is a buffer");
        buffer.word0 |= 0x12_3456_789A;
        buffer.word0 |= 16 << STRIDE_POSITION;
        buffer.num_records = 4;
        buffer.word2 = (10 << 15) | (3 << 12) | 2;
        assert_eq!(buffer.base_address(), 0x12_3456_789A);
        assert_eq!(buffer.stride(), 16);
        assert_eq!(buffer.get_stride(), 16);
        assert_eq!(buffer.get_size(), 64, "sixteen bytes over four records");
        assert!(buffer.is_bound());
        assert_eq!(buffer.data_format(), 10);
        assert_eq!(buffer.num_format(), 3);
        assert_eq!(buffer.dst_sel_x(), 2);
        assert_eq!(buffer.get_index_stride(), 8);
        assert_eq!(buffer.get_element_size(), 2);
        assert_eq!(buffer.num_dwords(), 16, "sixty-four bytes");

        // A stride of zero means one dword per record.
        let buffer = Buffer {
            num_records: 7,
            ..Buffer::default()
        };
        assert_eq!(buffer.get_size(), 7);
        assert_eq!(buffer.get_data_fmt(), DataFormat::FormatInvalid);
        let null = Buffer::null();
        assert_eq!(null.base_address(), 1);
        assert!(!null.is_bound());
        let placeholder = Buffer::placeholder(0x100);
        assert_eq!(placeholder.num_records, 0x100);
        assert_eq!(Buffer::from_dwords(&null.dwords()), null);
    }

    #[test]
    fn the_image_descriptor_keeps_its_fields() {
        let image = Image::null(true);
        assert_eq!(image.get_type(), ImageType::Color2D);
        assert!(image.is_valid());
        assert!(!image.is_bound());
        assert_eq!(image.get_data_fmt(), DataFormat::Format32);
        assert_eq!(image.get_number_fmt(), NumberFormat::Float);
        assert_eq!(image.get_tile_mode(), TileMode::Thin1DThin);
        assert!(image.is_tiled());
        assert_eq!(image.get_pitch(), 1);
        assert_eq!(image.num_layers(), 1);
        assert_eq!(image.num_levels(), 1);
        assert_eq!(image.num_samples(), 1);
        // A one-channel format remaps the swizzle, so only the red channel survives.
        assert_eq!(image.dst_select().r, CompSwizzle::Red);
        assert_eq!(image.dst_select().g, CompSwizzle::Green);

        // A cube is read as an array, and its depth multiplies by six.
        let mut cube = Image::null(false);
        cube.set_type(ImageType::Cube.raw());
        cube.set_depth(1);
        assert!(cube.is_cube());
        assert_eq!(cube.get_type(), ImageType::Color2DArray);
        assert_eq!(cube.get_base_type(), ImageType::Color2D);
        assert_eq!(cube.num_layers(), 12, "two cubes");
        assert_eq!(cube.get_view_type(false), ImageType::Color2DArray);

        // A 3D image keeps its depth as a layer count of one.
        let mut volume = Image::null(false);
        volume.set_type(ImageType::Color3D.raw());
        volume.set_depth(8);
        assert_eq!(volume.num_layers(), 1);
        assert_eq!(volume.get_base_type(), ImageType::Color3D);
        assert_eq!(volume.num_view_layers(false), 1);

        let mut array = Image::null(false);
        array.set_type(ImageType::Color2DArray.raw());
        array.set_depth(3);
        assert_eq!(array.num_layers(), 4);
        assert_eq!(array.get_view_type(false), ImageType::Color2D);
        assert_eq!(array.get_view_type(true), ImageType::Color2DArray);

        // A multisampled image takes its samples from the last level field.
        let mut msaa = Image::null(false);
        msaa.set_type(ImageType::Color2DMsaa.raw());
        msaa.word1 = (msaa.word1 & !(0xF << 48)) | (3 << 48);
        assert_eq!(msaa.num_samples(), 8);
        assert_eq!(msaa.num_levels(), 1);
        assert_eq!(msaa.num_view_levels(true), 1);
        assert_eq!(Image::from_dwords(&image.dwords()), image);
    }

    #[test]
    fn the_sampler_descriptor_keeps_its_fields() {
        let mut sampler = Sampler::default();
        assert!(!sampler.is_bound());
        assert!(sampler.is_valid());
        sampler.set_clamp_x(ClampMode::ClampLastTexel);
        sampler.set_clamp_y(ClampMode::Wrap);
        sampler.set_clamp_z(ClampMode::Mirror);
        sampler.set_max_aniso(AnisoRatio::Sixteen);
        sampler.set_aniso_threshold(5);
        assert_eq!(sampler.clamp_x(), ClampMode::ClampLastTexel);
        assert_eq!(sampler.clamp_y(), ClampMode::Wrap);
        assert_eq!(sampler.clamp_z(), ClampMode::Mirror);
        assert_eq!(sampler.max_aniso(), AnisoRatio::Sixteen);
        assert_eq!(sampler.aniso_threshold(), 5);
        assert!(sampler.is_bound());
        assert_eq!(sampler.get_max_aniso(), 16.0);

        // The LOD fields are fixed point with eight fractional bits.
        sampler.raw0 = (0x100 << 32) | (0x200 << 44);
        assert_eq!(sampler.get_min_lod(), 1.0);
        assert_eq!(sampler.get_max_lod(), 2.0);
        // The LOD bias is a signed field, whose sign the C++'s xor moves into place.
        sampler.raw1 = 0;
        assert_eq!(sampler.get_lod_bias(), 0.0);
        sampler.raw1 = 0x100;
        assert_eq!(sampler.get_lod_bias(), 1.0);
        sampler.raw1 = 0x3F00;
        assert_eq!(sampler.get_lod_bias(), -1.0);

        sampler.raw1 = (1 << 20) | (2 << 22) | (2 << 26) | (3 << 62);
        assert_eq!(sampler.xy_mag_filter(), Filter::Bilinear);
        assert_eq!(sampler.xy_min_filter(), Filter::AnisoPoint);
        assert!(is_aniso_filter(sampler.xy_min_filter()));
        assert_eq!(sampler.mip_filter(), MipFilter::Linear);
        assert_eq!(sampler.border_color_type(), BorderColor::Custom);
        assert_eq!(sampler.depth_compare_func(), DepthCompare::Never);
        assert_eq!(Sampler::from_dwords(&sampler.dwords()), sampler);
        assert_eq!(
            name_of_image_type(ImageType::Color2DMsaaArray),
            "Color2DMsaaArray"
        );
        assert_eq!(ImageType::Cube.to_string(), "Cube");
        assert_eq!(ClampMode::MirrorOnceBorder.raw(), 7);
        assert_eq!(AnisoRatio::from_raw(5), None);
        assert_eq!(FilterMode::Max.raw(), 2);
    }
}
