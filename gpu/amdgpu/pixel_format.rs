//! AMD data/number formats, swizzles and remapping, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/pixel_format.{h,cpp}`.
//!
//! The C++ tables assert on out-of-range formats. This port returns `None` for undefined
//! values, so a damaged guest descriptor degrades to "unsupported" rather than an abort.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FormatError {
    #[error("unknown data format {0}")]
    UnknownDataFormat(u32),
    #[error("unknown number format {0}")]
    UnknownNumberFormat(u32),
    #[error("unknown number class {0}")]
    UnknownNumberClass(u8),
    #[error("unknown component swizzle {0}")]
    UnknownSwizzle(u8),
    #[error("unknown number conversion {0}")]
    UnknownConversion(u32),
}

macro_rules! raw_enum {
    ($name:ident { $($variant:ident = $value:expr),+ $(,)? }) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        pub enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub fn from_raw(value: u32) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub const fn raw(self) -> u32 {
                self as u32
            }
        }
    };
}

pub(crate) use raw_enum;

raw_enum!(DataFormat {
    FormatInvalid = 0,
    Format8 = 1,
    Format16 = 2,
    Format8_8 = 3,
    Format32 = 4,
    Format16_16 = 5,
    Format10_11_11 = 6,
    Format11_11_10 = 7,
    Format10_10_10_2 = 8,
    Format2_10_10_10 = 9,
    Format8_8_8_8 = 10,
    Format32_32 = 11,
    Format16_16_16_16 = 12,
    Format32_32_32 = 13,
    Format32_32_32_32 = 14,
    Format5_6_5 = 16,
    Format1_5_5_5 = 17,
    Format5_5_5_1 = 18,
    Format4_4_4_4 = 19,
    Format8_24 = 20,
    Format24_8 = 21,
    FormatX24_8_32 = 22,
    FormatGB_GR = 32,
    FormatBG_RG = 33,
    Format5_9_9_9 = 34,
    FormatBc1 = 35,
    FormatBc2 = 36,
    FormatBc3 = 37,
    FormatBc4 = 38,
    FormatBc5 = 39,
    FormatBc6 = 40,
    FormatBc7 = 41,
    FormatFmask8_S2_F1 = 44,
    FormatFmask8_S4_F1 = 45,
    FormatFmask8_S8_F1 = 46,
    FormatFmask8_S2_F2 = 47,
    FormatFmask8_S4_F2 = 48,
    FormatFmask8_S4_F4 = 49,
    FormatFmask16_S16_F1 = 50,
    FormatFmask16_S8_F2 = 51,
    FormatFmask32_S16_F2 = 52,
    FormatFmask32_S8_F4 = 53,
    FormatFmask32_S8_F8 = 54,
    FormatFmask64_S16_F4 = 55,
    FormatFmask64_S16_F8 = 56,
    Format4_4 = 57,
    Format6_5_5 = 58,
    Format1 = 59,
    Format1_Reversed = 60,
    Format32_As_8 = 61,
    Format32_As_8_8 = 62,
    Format32_As_32_32_32_32 = 63,
});

raw_enum!(NumberFormat {
    Unorm = 0,
    Snorm = 1,
    Uscaled = 2,
    Sscaled = 3,
    Uint = 4,
    Sint = 5,
    SnormNz = 6,
    Float = 7,
    Srgb = 9,
    Ubnorm = 10,
    UbnormNz = 11,
    Ubint = 12,
    Ubscaled = 13,
});

raw_enum!(NumberConversion {
    None = 0,
    UintToUscaled = 1,
    SintToSscaled = 2,
    UnormToUbnorm = 3,
    Sint8ToSnormNz = 4,
    Sint16ToSnormNz = 5,
    Uint32ToUnorm = 6,
    SrgbToNorm = 7,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberClass {
    Float,
    Sint,
    Uint,
}

impl NumberClass {
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Float),
            1 => Some(Self::Sint),
            2 => Some(Self::Uint),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        match self {
            Self::Float => 0,
            Self::Sint => 1,
            Self::Uint => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompSwizzle {
    Zero = 0,
    One = 1,
    Red = 4,
    Green = 5,
    Blue = 6,
    Alpha = 7,
}

impl CompSwizzle {
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            4 => Some(Self::Red),
            5 => Some(Self::Green),
            6 => Some(Self::Blue),
            7 => Some(Self::Alpha),
            _ => None,
        }
    }

    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompMapping {
    pub r: CompSwizzle,
    pub g: CompSwizzle,
    pub b: CompSwizzle,
    pub a: CompSwizzle,
}

impl CompMapping {
    pub const IDENTITY: Self = Self {
        r: CompSwizzle::Red,
        g: CompSwizzle::Green,
        b: CompSwizzle::Blue,
        a: CompSwizzle::Alpha,
    };

    pub fn identity() -> Self {
        Self::IDENTITY
    }

    pub fn components(self) -> [CompSwizzle; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Apply the swizzle to RGBA data (`Zero`/`One` produce `T::from(0)`/`T::from(1)`).
    pub fn apply<T: Copy + From<u8>>(self, data: [T; 4]) -> [T; 4] {
        [
            apply_single(&data, self.r),
            apply_single(&data, self.g),
            apply_single(&data, self.b),
            apply_single(&data, self.a),
        ]
    }

    /// Map a four-bit channel mask through the swizzle.
    pub fn apply_mask(self, mask: u32) -> u32 {
        let mut swizzled = 0;
        for index in 0..4u32 {
            swizzled |= ((mask >> index) & 1) << self.map(index);
        }
        swizzled
    }

    /// Swizzle index of a component: `Red..Alpha` map to 0..3, `Zero`/`One` keep their slot.
    pub fn map(self, component: u32) -> u32 {
        let selected = self.components()[(component as usize).min(3)].raw() as u32;
        let min_component = CompSwizzle::Red.raw() as u32;
        if selected >= min_component {
            selected - min_component
        } else {
            component
        }
    }

    /// The mapping that undoes this one (components not produced by it become `Zero`).
    pub fn inverse(self) -> Self {
        Self {
            r: inverse_single(self, CompSwizzle::Red),
            g: inverse_single(self, CompSwizzle::Green),
            b: inverse_single(self, CompSwizzle::Blue),
            a: inverse_single(self, CompSwizzle::Alpha),
        }
    }
}

fn apply_single<T: Copy + From<u8>>(data: &[T; 4], swizzle: CompSwizzle) -> T {
    match swizzle {
        CompSwizzle::Zero => T::from(0),
        CompSwizzle::One => T::from(1),
        CompSwizzle::Red => data[0],
        CompSwizzle::Green => data[1],
        CompSwizzle::Blue => data[2],
        CompSwizzle::Alpha => data[3],
    }
}

fn inverse_single(mapping: CompMapping, target: CompSwizzle) -> CompSwizzle {
    let components = mapping.components();
    for (index, component) in components.iter().enumerate() {
        if *component == target {
            return [
                CompSwizzle::Red,
                CompSwizzle::Green,
                CompSwizzle::Blue,
                CompSwizzle::Alpha,
            ][index];
        }
    }
    CompSwizzle::Zero
}

// Component counts for data formats 0..=41 (the C++ `NUM_COMPONENTS` table).
const NUM_COMPONENTS: [u32; 42] = [
    0, 1, 1, 2, 1, 2, 3, 3, 4, 4, 4, 2, 4, 3, 4, 0, 3, 4, 4, 4, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    3, 3, 4, 4, 4, 4, 1, 2, 3, 4,
];

// Bits per block for data formats 0..=41; -1 means "undefined".
const BITS_PER_BLOCK: [i32; 42] = [
    0, 8, 16, 16, 32, 32, 32, 32, 32, 32, 32, 64, 64, 96, 128, -1, 16, 16, 16, 16, 32, 32, 64, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, 16, 16, 32, 64, 128, 128, 64, 128, 128, 128,
];

// Bits per element for data formats 0..=41; -1 means "undefined".
const BITS_PER_ELEMENT: [i32; 42] = [
    0, 8, 16, 16, 32, 32, 32, 32, 32, 32, 32, 64, 64, 96, 128, -1, 16, 16, 16, 16, 32, 32, 64, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, 16, 16, 32, 4, 8, 8, 4, 8, 8, 8,
];

fn table_value(table: &[i32], format: DataFormat) -> Option<u32> {
    let value = *table.get(format.raw() as usize)?;
    (value >= 0).then_some(value as u32)
}

pub fn num_components(format: DataFormat) -> Option<u32> {
    NUM_COMPONENTS.get(format.raw() as usize).copied()
}

pub fn num_bits_per_block(format: DataFormat) -> Option<u32> {
    table_value(&BITS_PER_BLOCK, format)
}

pub fn num_bits_per_element(format: DataFormat) -> Option<u32> {
    table_value(&BITS_PER_ELEMENT, format)
}

pub fn name_of(format: DataFormat) -> &'static str {
    match format {
        DataFormat::FormatInvalid => "FormatInvalid",
        DataFormat::Format8 => "Format8",
        DataFormat::Format16 => "Format16",
        DataFormat::Format8_8 => "Format8_8",
        DataFormat::Format32 => "Format32",
        DataFormat::Format16_16 => "Format16_16",
        DataFormat::Format10_11_11 => "Format10_11_11",
        DataFormat::Format11_11_10 => "Format11_11_10",
        DataFormat::Format10_10_10_2 => "Format10_10_10_2",
        DataFormat::Format2_10_10_10 => "Format2_10_10_10",
        DataFormat::Format8_8_8_8 => "Format8_8_8_8",
        DataFormat::Format32_32 => "Format32_32",
        DataFormat::Format16_16_16_16 => "Format16_16_16_16",
        DataFormat::Format32_32_32 => "Format32_32_32",
        DataFormat::Format32_32_32_32 => "Format32_32_32_32",
        DataFormat::Format5_6_5 => "Format5_6_5",
        DataFormat::Format1_5_5_5 => "Format1_5_5_5",
        DataFormat::Format5_5_5_1 => "Format5_5_5_1",
        DataFormat::Format4_4_4_4 => "Format4_4_4_4",
        DataFormat::Format8_24 => "Format8_24",
        DataFormat::Format24_8 => "Format24_8",
        DataFormat::FormatX24_8_32 => "FormatX24_8_32",
        DataFormat::FormatGB_GR => "FormatGB_GR",
        DataFormat::FormatBG_RG => "FormatBG_RG",
        DataFormat::Format5_9_9_9 => "Format5_9_9_9",
        DataFormat::FormatBc1 => "FormatBc1",
        DataFormat::FormatBc2 => "FormatBc2",
        DataFormat::FormatBc3 => "FormatBc3",
        DataFormat::FormatBc4 => "FormatBc4",
        DataFormat::FormatBc5 => "FormatBc5",
        DataFormat::FormatBc6 => "FormatBc6",
        DataFormat::FormatBc7 => "FormatBc7",
        DataFormat::FormatFmask8_S2_F1 => "FormatFmask8_S2_F1",
        DataFormat::FormatFmask8_S4_F1 => "FormatFmask8_S4_F1",
        DataFormat::FormatFmask8_S8_F1 => "FormatFmask8_S8_F1",
        DataFormat::FormatFmask8_S2_F2 => "FormatFmask8_S2_F2",
        DataFormat::FormatFmask8_S4_F2 => "FormatFmask8_S4_F2",
        DataFormat::FormatFmask8_S4_F4 => "FormatFmask8_S4_F4",
        DataFormat::FormatFmask16_S16_F1 => "FormatFmask16_S16_F1",
        DataFormat::FormatFmask16_S8_F2 => "FormatFmask16_S8_F2",
        DataFormat::FormatFmask32_S16_F2 => "FormatFmask32_S16_F2",
        DataFormat::FormatFmask32_S8_F4 => "FormatFmask32_S8_F4",
        DataFormat::FormatFmask32_S8_F8 => "FormatFmask32_S8_F8",
        DataFormat::FormatFmask64_S16_F4 => "FormatFmask64_S16_F4",
        DataFormat::FormatFmask64_S16_F8 => "FormatFmask64_S16_F8",
        DataFormat::Format4_4 => "Format4_4",
        DataFormat::Format6_5_5 => "Format6_5_5",
        DataFormat::Format1 => "Format1",
        DataFormat::Format1_Reversed => "Format1_Reversed",
        DataFormat::Format32_As_8 => "Format32_As_8",
        DataFormat::Format32_As_8_8 => "Format32_As_8_8",
        DataFormat::Format32_As_32_32_32_32 => "Format32_As_32_32_32_32",
    }
}

pub fn name_of_number(format: NumberFormat) -> &'static str {
    match format {
        NumberFormat::Unorm => "Unorm",
        NumberFormat::Snorm => "Snorm",
        NumberFormat::Uscaled => "Uscaled",
        NumberFormat::Sscaled => "Sscaled",
        NumberFormat::Uint => "Uint",
        NumberFormat::Sint => "Sint",
        NumberFormat::SnormNz => "SnormNz",
        NumberFormat::Float => "Float",
        NumberFormat::Srgb => "Srgb",
        NumberFormat::Ubnorm => "Ubnorm",
        NumberFormat::UbnormNz => "UbnormNz",
        NumberFormat::Ubint => "Ubint",
        NumberFormat::Ubscaled => "Unscaled",
    }
}

pub fn remap_data_format(format: DataFormat) -> DataFormat {
    match format {
        DataFormat::Format11_11_10 => DataFormat::Format10_11_11,
        DataFormat::Format10_10_10_2 => DataFormat::Format2_10_10_10,
        DataFormat::Format5_5_5_1 => DataFormat::Format1_5_5_5,
        other => other,
    }
}

pub fn remap_number_format(format: NumberFormat, data_format: DataFormat) -> NumberFormat {
    match format {
        NumberFormat::Unorm => match data_format {
            DataFormat::Format32
            | DataFormat::Format32_32
            | DataFormat::Format32_32_32
            | DataFormat::Format32_32_32_32 => NumberFormat::Uint,
            _ => format,
        },
        NumberFormat::Srgb => match data_format {
            DataFormat::FormatBc4 | DataFormat::FormatBc5 | DataFormat::FormatBc6 => {
                NumberFormat::Unorm
            }
            _ => format,
        },
        NumberFormat::Uscaled => NumberFormat::Uint,
        NumberFormat::Sscaled | NumberFormat::SnormNz => NumberFormat::Sint,
        NumberFormat::Ubnorm => NumberFormat::Unorm,
        NumberFormat::Float if data_format == DataFormat::Format8 => NumberFormat::Uint,
        other => other,
    }
}

pub fn remap_swizzle(format: DataFormat, swizzle: CompMapping) -> CompMapping {
    match format {
        DataFormat::Format1_5_5_5 => CompMapping {
            r: swizzle.b,
            g: swizzle.g,
            b: swizzle.r,
            a: swizzle.a,
        },
        DataFormat::Format11_11_10 => CompMapping {
            r: swizzle.b,
            g: swizzle.g,
            b: swizzle.r,
            a: swizzle.a,
        },
        DataFormat::Format10_10_10_2 => CompMapping {
            r: swizzle.a,
            g: swizzle.b,
            b: swizzle.g,
            a: swizzle.r,
        },
        DataFormat::Format4_4_4_4 => CompMapping {
            r: swizzle.g,
            g: swizzle.b,
            b: swizzle.a,
            a: swizzle.r,
        },
        DataFormat::Format5_6_5 => CompMapping {
            r: swizzle.b,
            g: swizzle.g,
            b: swizzle.r,
            a: swizzle.a,
        },
        DataFormat::Format8 | DataFormat::Format16 | DataFormat::Format32 => {
            let remap = |component: CompSwizzle| {
                if component == CompSwizzle::Alpha {
                    CompSwizzle::Red
                } else {
                    component
                }
            };
            CompMapping {
                r: remap(swizzle.r),
                g: remap(swizzle.g),
                b: remap(swizzle.b),
                a: remap(swizzle.a),
            }
        }
        _ => swizzle,
    }
}

pub fn map_number_conversion(format: NumberFormat, data_format: DataFormat) -> NumberConversion {
    match format {
        NumberFormat::Unorm => match data_format {
            DataFormat::Format32
            | DataFormat::Format32_32
            | DataFormat::Format32_32_32
            | DataFormat::Format32_32_32_32 => NumberConversion::Uint32ToUnorm,
            _ => NumberConversion::None,
        },
        NumberFormat::Srgb => match data_format {
            DataFormat::FormatBc4 | DataFormat::FormatBc5 => NumberConversion::None,
            DataFormat::FormatBc6 => NumberConversion::SrgbToNorm,
            _ => NumberConversion::None,
        },
        NumberFormat::Uscaled => NumberConversion::UintToUscaled,
        NumberFormat::Sscaled => NumberConversion::SintToSscaled,
        NumberFormat::Ubnorm => NumberConversion::UnormToUbnorm,
        NumberFormat::SnormNz => match data_format {
            DataFormat::Format8 | DataFormat::Format8_8 | DataFormat::Format8_8_8_8 => {
                NumberConversion::Sint8ToSnormNz
            }
            DataFormat::Format16 | DataFormat::Format16_16 | DataFormat::Format16_16_16_16 => {
                NumberConversion::Sint16ToSnormNz
            }
            // A stale/unset descriptor: read it unconverted instead of stopping the game.
            _ => NumberConversion::None,
        },
        _ => NumberConversion::None,
    }
}

pub fn get_number_class(format: NumberFormat) -> NumberClass {
    match format {
        NumberFormat::Sint => NumberClass::Sint,
        NumberFormat::Uint => NumberClass::Uint,
        _ => NumberClass::Float,
    }
}

pub fn is_rgb(swizzle: CompSwizzle) -> bool {
    matches!(
        swizzle,
        CompSwizzle::Red | CompSwizzle::Green | CompSwizzle::Blue
    )
}

pub fn is_integer(format: NumberFormat) -> bool {
    matches!(format, NumberFormat::Sint | NumberFormat::Uint)
}

pub fn is_block_coded(format: DataFormat) -> bool {
    (DataFormat::FormatBc1.raw()..=DataFormat::FormatBc7.raw()).contains(&format.raw())
}

pub fn is_fmask(format: DataFormat) -> bool {
    (DataFormat::FormatFmask8_S2_F1.raw()..=DataFormat::FormatFmask64_S16_F8.raw())
        .contains(&format.raw())
}

impl DataFormat {
    /// The C++ `NameOf`, also exposed through `Display`.
    pub fn name(self) -> &'static str {
        name_of(self)
    }
}

impl std::fmt::Display for DataFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(name_of(*self))
    }
}

impl std::fmt::Display for NumberFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(name_of_number(*self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tables_match_the_cpp_tables() {
        assert_eq!(num_components(DataFormat::Format8_8_8_8), Some(4));
        assert_eq!(num_components(DataFormat::Format32), Some(1));
        assert_eq!(num_components(DataFormat::FormatBc1), Some(4));
        assert_eq!(num_bits_per_block(DataFormat::Format8_8_8_8), Some(32));
        assert_eq!(num_bits_per_block(DataFormat::FormatBc2), Some(128));
        assert_eq!(num_bits_per_element(DataFormat::FormatBc1), Some(4));
        assert_eq!(num_bits_per_element(DataFormat::Format32_32_32), Some(96));
        // Undefined entries and formats beyond the table are not fatal.
        assert_eq!(num_bits_per_block(DataFormat::FormatFmask8_S2_F1), None);
        assert_eq!(num_bits_per_element(DataFormat::FormatInvalid), Some(0));
        assert_eq!(name_of(DataFormat::FormatBc7), "FormatBc7");
        assert_eq!(name_of_number(NumberFormat::SnormNz), "SnormNz");
    }

    #[test]
    fn format_remapping_matches_the_hardware_fixups() {
        assert_eq!(
            remap_data_format(DataFormat::Format11_11_10),
            DataFormat::Format10_11_11
        );
        assert_eq!(
            remap_data_format(DataFormat::Format5_5_5_1),
            DataFormat::Format1_5_5_5
        );
        assert_eq!(
            remap_data_format(DataFormat::Format10_10_10_2),
            DataFormat::Format2_10_10_10
        );
        assert_eq!(
            remap_data_format(DataFormat::Format8_8),
            DataFormat::Format8_8
        );

        assert_eq!(
            remap_number_format(NumberFormat::Unorm, DataFormat::Format32_32),
            NumberFormat::Uint
        );
        assert_eq!(
            remap_number_format(NumberFormat::Srgb, DataFormat::FormatBc6),
            NumberFormat::Unorm
        );
        assert_eq!(
            remap_number_format(NumberFormat::Float, DataFormat::Format8),
            NumberFormat::Uint
        );
        assert_eq!(
            remap_number_format(NumberFormat::SnormNz, DataFormat::Format8_8),
            NumberFormat::Sint
        );
        assert_eq!(
            remap_number_format(NumberFormat::Float, DataFormat::Format32),
            NumberFormat::Float
        );
    }

    #[test]
    fn number_conversions_follow_the_cpp_rules() {
        assert_eq!(
            map_number_conversion(NumberFormat::Unorm, DataFormat::Format32_32_32),
            NumberConversion::Uint32ToUnorm
        );
        assert_eq!(
            map_number_conversion(NumberFormat::Srgb, DataFormat::FormatBc6),
            NumberConversion::SrgbToNorm
        );
        assert_eq!(
            map_number_conversion(NumberFormat::SnormNz, DataFormat::Format16_16),
            NumberConversion::Sint16ToSnormNz
        );
        assert_eq!(
            map_number_conversion(NumberFormat::SnormNz, DataFormat::Format10_11_11),
            NumberConversion::None
        );
        assert_eq!(
            map_number_conversion(NumberFormat::Ubnorm, DataFormat::Format8_8_8_8),
            NumberConversion::UnormToUbnorm
        );
    }

    #[test]
    fn swizzles_apply_mask_map_and_invert() {
        let mapping = CompMapping {
            r: CompSwizzle::Blue,
            g: CompSwizzle::Green,
            b: CompSwizzle::Red,
            a: CompSwizzle::Alpha,
        };
        assert_eq!(mapping.apply([1u8, 2, 3, 4]), [3, 2, 1, 4]);
        assert_eq!(mapping.apply_mask(0b0001), 0b0100);
        assert_eq!(mapping.map(0), 2);
        assert_eq!(mapping.inverse().apply([1u8, 2, 3, 4]), [3, 2, 1, 4]);

        let alpha_to_red = CompMapping {
            r: CompSwizzle::Alpha,
            g: CompSwizzle::Green,
            b: CompSwizzle::Blue,
            a: CompSwizzle::Alpha,
        };
        assert_eq!(alpha_to_red.apply([1u8, 2, 3, 4]), [4, 2, 3, 4]);
        assert_eq!(CompMapping::identity().apply([1u8, 2, 3, 4]), [1, 2, 3, 4]);
        assert_eq!(CompMapping::identity().apply_mask(0b1010), 0b1010);
    }

    #[test]
    fn data_format_specific_swizzle_remaps() {
        let identity = CompMapping::identity();
        assert_eq!(
            remap_swizzle(DataFormat::Format5_6_5, identity),
            CompMapping {
                r: CompSwizzle::Blue,
                g: CompSwizzle::Green,
                b: CompSwizzle::Red,
                a: CompSwizzle::Alpha,
            }
        );
        let alpha = CompMapping {
            r: CompSwizzle::Alpha,
            g: CompSwizzle::One,
            b: CompSwizzle::Alpha,
            a: CompSwizzle::Alpha,
        };
        assert_eq!(
            remap_swizzle(DataFormat::Format8, alpha),
            CompMapping {
                r: CompSwizzle::Red,
                g: CompSwizzle::One,
                b: CompSwizzle::Red,
                a: CompSwizzle::Red,
            }
        );
        assert_eq!(remap_swizzle(DataFormat::Format8_8, identity), identity);
    }

    #[test]
    fn format_classes_and_predicates() {
        assert_eq!(get_number_class(NumberFormat::Sint), NumberClass::Sint);
        assert_eq!(get_number_class(NumberFormat::Unorm), NumberClass::Float);
        assert!(is_rgb(CompSwizzle::Red));
        assert!(!is_rgb(CompSwizzle::Alpha));
        assert!(is_integer(NumberFormat::Uint));
        assert!(!is_integer(NumberFormat::Snorm));
        assert!(is_block_coded(DataFormat::FormatBc7));
        assert!(!is_block_coded(DataFormat::Format8_8_8_8));
        assert!(is_fmask(DataFormat::FormatFmask32_S8_F4));
        assert!(!is_fmask(DataFormat::Format32));
    }
}
