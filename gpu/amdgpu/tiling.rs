//! AMD GPU tile-mode math, ported from `legacy/gpu/shadps4/video_core/amdgpu/tiling.{h,cpp}`.
//!
//! The C++ original uses `UNREACHABLE_MSG` for unknown hardware values. This port returns
//! [`TilingError`] instead, so a corrupt guest command degrades to a skipped operation rather
//! than an abort.

use thiserror::Error;

pub const NUM_TILE_MODES: usize = 32;
pub const MICROTILE_SIZE: u32 = 8;
pub const DRAM_ROW_SIZE: u32 = 1024;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TilingError {
    #[error("unknown tile mode {0}")]
    UnknownTileMode(u32),
    #[error("unknown array mode {0}")]
    UnknownArrayMode(u32),
    #[error("unknown macro tile mode {0}")]
    UnknownMacroTileMode(u32),
    #[error("unknown pipe config {0}")]
    UnknownPipeConfig(u32),
    #[error("invalid sample count {0}")]
    InvalidSamples(u32),
    #[error("invalid bits per pixel {0}")]
    InvalidBpp(u32),
    #[error("tile mode {0:?} is not macro tiled")]
    NotMacroTiled(TileMode),
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

raw_enum!(PipeConfig {
    P2 = 0,
    P4_8x16 = 4,
    P4_16x16 = 5,
    P4_16x32 = 6,
    P4_32x32 = 7,
    P8_16x16_8x16 = 8,
    P8_16x32_8x16 = 9,
    P8_32x32_8x16 = 10,
    P8_16x32_16x16 = 11,
    P8_32x32_16x16 = 12,
    P8_32x32_16x32 = 13,
    P8_32x64_32x32 = 14,
    P16_32x32_8x16 = 16,
    P16_32x32_16x16 = 17,
    P16 = 18,
});

raw_enum!(MicroTileMode {
    Display = 0,
    Thin = 1,
    Depth = 2,
    Rotated = 3,
    Thick = 4,
});

raw_enum!(MacroTileMode {
    Mode_1x4_16 = 0,
    Mode_1x2_16 = 1,
    Mode_1x1_16 = 2,
    Mode_1x1_16_Dup = 3,
    Mode_1x1_8 = 4,
    Mode_1x1_4 = 5,
    Mode_1x1_2 = 6,
    Mode_1x1_2_Dup = 7,
    Mode_1x8_16 = 8,
    Mode_1x4_16_Dup = 9,
    Mode_1x2_16_Dup = 10,
    Mode_1x1_16_Dup2 = 11,
    Mode_1x1_8_Dup = 12,
    Mode_1x1_4_Dup = 13,
    Mode_1x1_2_Dup2 = 14,
    Mode_1x1_2_Dup3 = 15,
});

raw_enum!(ArrayMode {
    ArrayLinearGeneral = 0,
    ArrayLinearAligned = 1,
    Array1DTiledThin1 = 2,
    Array1DTiledThick = 3,
    Array2DTiledThin1 = 4,
    ArrayPrtTiledThin1 = 5,
    ArrayPrt2DTiledThin1 = 6,
    Array2DTiledThick = 7,
    Array2DTiledXThick = 8,
    ArrayPrtTiledThick = 9,
    ArrayPrt2DTiledThick = 10,
    ArrayPrt3DTiledThin1 = 11,
    Array3DTiledThin1 = 12,
    Array3DTiledThick = 13,
    Array3DTiledXThick = 14,
    ArrayPrt3DTiledThick = 15,
});

raw_enum!(TileMode {
    Depth2DThin64 = 0,
    Depth2DThin128 = 1,
    Depth2DThin256 = 2,
    Depth2DThin512 = 3,
    Depth2DThin1K = 4,
    Depth1DThin = 5,
    Depth2DThinPrt256 = 6,
    Depth2DThinPrt1K = 7,
    DisplayLinearAligned = 8,
    Display1DThin = 9,
    Display2DThin = 10,
    DisplayThinPrt = 11,
    Display2DThinPrt = 12,
    Thin1DThin = 13,
    Thin2DThin = 14,
    Thin3DThin = 15,
    ThinThinPrt = 16,
    Thin2DThinPrt = 17,
    Thin3DThinPrt = 18,
    Thick1DThick = 19,
    Thick2DThick = 20,
    Thick3DThick = 21,
    ThickThickPrt = 22,
    Thick2DThickPrt = 23,
    Thick3DThickPrt = 24,
    Thick2DXThick = 25,
    Thick3DXThick = 26,
    DisplayLinearGeneral = 31,
});

impl TileMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Depth2DThin64 => "Depth2DThin64",
            Self::Depth2DThin128 => "Depth2DThin128",
            Self::Depth2DThin256 => "Depth2DThin256",
            Self::Depth2DThin512 => "Depth2DThin512",
            Self::Depth2DThin1K => "Depth2DThin1K",
            Self::Depth1DThin => "Depth1DThin",
            Self::Depth2DThinPrt256 => "Depth2DThinPrt256",
            Self::Depth2DThinPrt1K => "Depth2DThinPrt1K",
            Self::DisplayLinearAligned => "DisplayLinearAligned",
            Self::Display1DThin => "Display1DThin",
            Self::Display2DThin => "Display2DThin",
            Self::DisplayThinPrt => "DisplayThinPrt",
            Self::Display2DThinPrt => "Display2DThinPrt",
            Self::Thin1DThin => "Thin1DThin",
            Self::Thin2DThin => "Thin2DThin",
            Self::Thin3DThin => "Thin3DThin",
            Self::ThinThinPrt => "ThinThinPrt",
            Self::Thin2DThinPrt => "Thin2DThinPrt",
            Self::Thin3DThinPrt => "Thin3DThinPrt",
            Self::Thick1DThick => "Thick1DThick",
            Self::Thick2DThick => "Thick2DThick",
            Self::Thick3DThick => "Thick3DThick",
            Self::ThickThickPrt => "ThickThickPrt",
            Self::Thick2DThickPrt => "Thick2DThickPrt",
            Self::Thick3DThickPrt => "Thick3DThickPrt",
            Self::Thick2DXThick => "Thick2DXThick",
            Self::Thick3DXThick => "Thick3DXThick",
            Self::DisplayLinearGeneral => "DisplayLinearGeneral",
        }
    }
}

pub fn get_array_mode(tile_mode: TileMode) -> ArrayMode {
    use ArrayMode::*;
    use TileMode::*;
    match tile_mode {
        Depth1DThin | Display1DThin | Thin1DThin => Array1DTiledThin1,
        Depth2DThin64 | Depth2DThin128 | Depth2DThin256 | Depth2DThin512 | Depth2DThin1K
        | Display2DThin | Thin2DThin => Array2DTiledThin1,
        DisplayThinPrt | ThinThinPrt => ArrayPrtTiledThin1,
        Depth2DThinPrt256 | Depth2DThinPrt1K | Display2DThinPrt | Thin2DThinPrt => {
            ArrayPrt2DTiledThin1
        }
        Thin3DThin | Thin3DThinPrt => Array3DTiledThin1,
        Thick1DThick => Array1DTiledThick,
        Thick2DThick => Array2DTiledThick,
        Thick3DThick => Array3DTiledThick,
        ThickThickPrt => ArrayPrtTiledThick,
        Thick2DThickPrt => ArrayPrt2DTiledThick,
        Thick3DThickPrt => ArrayPrt3DTiledThick,
        Thick2DXThick => Array2DTiledXThick,
        Thick3DXThick => Array3DTiledXThick,
        DisplayLinearAligned => ArrayLinearAligned,
        DisplayLinearGeneral => ArrayLinearGeneral,
    }
}

pub fn get_micro_tile_mode(tile_mode: TileMode) -> MicroTileMode {
    use MicroTileMode::*;
    use TileMode::*;
    match tile_mode {
        Depth2DThin64 | Depth2DThin128 | Depth2DThin256 | Depth2DThin512 | Depth2DThin1K
        | Depth1DThin | Depth2DThinPrt256 | Depth2DThinPrt1K => Depth,
        DisplayLinearAligned | Display1DThin | Display2DThin | DisplayThinPrt
        | Display2DThinPrt | DisplayLinearGeneral => Display,
        Thin1DThin | Thin2DThin | Thin3DThin | ThinThinPrt | Thin2DThinPrt | Thin3DThinPrt => Thin,
        Thick1DThick | Thick2DThick | Thick3DThick | ThickThickPrt | Thick2DThickPrt
        | Thick3DThickPrt | Thick2DXThick | Thick3DXThick => Thick,
    }
}

pub fn get_pipe_config(tile_mode: TileMode) -> PipeConfig {
    use PipeConfig::*;
    use TileMode::*;
    match tile_mode {
        Depth2DThin64 | Depth2DThin128 | Depth2DThin256 | Depth2DThin512 | Depth2DThin1K
        | Depth1DThin | Depth2DThinPrt256 | Depth2DThinPrt1K | DisplayLinearAligned
        | Display1DThin | Display2DThin | Display2DThinPrt | Thin1DThin | Thin2DThin
        | Thin2DThinPrt | Thin3DThinPrt | Thick1DThick | Thick2DThick | Thick2DThickPrt
        | Thick2DXThick => P8_32x32_16x16,
        DisplayThinPrt | Thin3DThin | ThinThinPrt | Thick3DThick | ThickThickPrt
        | Thick3DThickPrt | Thick3DXThick => P8_32x32_8x16,
        DisplayLinearGeneral => P2,
    }
}

pub fn get_alt_pipe_config(tile_mode: TileMode) -> PipeConfig {
    use PipeConfig::*;
    match tile_mode {
        TileMode::DisplayLinearGeneral => P2,
        _ => P16_32x32_8x16,
    }
}

pub fn get_sample_split(tile_mode: TileMode) -> u32 {
    use TileMode::*;
    match tile_mode {
        Depth2DThin64 | Depth2DThin128 | Depth2DThin256 | Depth2DThin512 | Depth2DThin1K
        | Depth1DThin | Depth2DThinPrt256 | Depth2DThinPrt1K | DisplayLinearAligned
        | Display1DThin | Thin1DThin | Thick1DThick | Thick2DThick | Thick3DThick
        | ThickThickPrt | Thick2DThickPrt | Thick3DThickPrt | Thick2DXThick | Thick3DXThick
        | DisplayLinearGeneral => 1,
        Display2DThin | DisplayThinPrt | Display2DThinPrt | Thin2DThin | Thin3DThin
        | ThinThinPrt | Thin2DThinPrt | Thin3DThinPrt => 2,
    }
}

pub fn get_tile_split_hw(tile_mode: TileMode) -> u32 {
    use TileMode::*;
    match tile_mode {
        Depth2DThin64 | Depth1DThin | DisplayLinearAligned | Display1DThin | Display2DThin
        | DisplayThinPrt | Display2DThinPrt | Thin1DThin | Thin2DThin | Thin3DThin
        | ThinThinPrt | Thin2DThinPrt | Thin3DThinPrt | Thick1DThick | Thick2DThick
        | Thick3DThick | ThickThickPrt | Thick2DThickPrt | Thick3DThickPrt | Thick2DXThick
        | Thick3DXThick | DisplayLinearGeneral => 64,
        Depth2DThin128 => 128,
        Depth2DThin256 | Depth2DThinPrt256 => 256,
        Depth2DThin512 => 512,
        Depth2DThin1K | Depth2DThinPrt1K => 1024,
    }
}

pub fn get_bank_width(mode: MacroTileMode) -> u32 {
    let _ = mode;
    1
}

pub fn get_bank_height(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x1_8 | Mode_1x1_4 | Mode_1x1_2 | Mode_1x1_2_Dup
        | Mode_1x1_16_Dup2 | Mode_1x1_8_Dup | Mode_1x1_4_Dup | Mode_1x1_2_Dup2
        | Mode_1x1_2_Dup3 => 1,
        Mode_1x2_16 | Mode_1x2_16_Dup => 2,
        Mode_1x4_16 | Mode_1x4_16_Dup => 4,
        Mode_1x8_16 => 8,
    }
}

pub fn get_num_banks(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_2 | Mode_1x1_2_Dup | Mode_1x1_2_Dup2 | Mode_1x1_2_Dup3 => 2,
        Mode_1x1_4 | Mode_1x1_4_Dup => 4,
        Mode_1x1_8 | Mode_1x1_8_Dup => 8,
        Mode_1x4_16 | Mode_1x2_16 | Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x8_16
        | Mode_1x4_16_Dup | Mode_1x2_16_Dup | Mode_1x1_16_Dup2 => 16,
    }
}

pub fn get_macrotile_aspect(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_8 | Mode_1x1_4 | Mode_1x1_2 | Mode_1x1_2_Dup | Mode_1x1_8_Dup | Mode_1x1_4_Dup
        | Mode_1x1_2_Dup2 | Mode_1x1_2_Dup3 => 1,
        Mode_1x2_16 | Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x2_16_Dup | Mode_1x1_16_Dup2 => 2,
        Mode_1x4_16 | Mode_1x8_16 | Mode_1x4_16_Dup => 4,
    }
}

pub fn get_alt_bank_height(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_8 | Mode_1x1_4 | Mode_1x1_2 | Mode_1x1_2_Dup | Mode_1x1_16_Dup2
        | Mode_1x1_8_Dup | Mode_1x1_4_Dup | Mode_1x1_2_Dup2 | Mode_1x1_2_Dup3 => 1,
        Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x2_16_Dup => 2,
        Mode_1x4_16 | Mode_1x2_16 | Mode_1x8_16 | Mode_1x4_16_Dup => 4,
    }
}

pub fn get_alt_num_banks(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_2_Dup | Mode_1x1_2_Dup2 | Mode_1x1_2_Dup3 => 2,
        Mode_1x1_2 | Mode_1x1_8_Dup | Mode_1x1_4_Dup => 4,
        Mode_1x4_16 | Mode_1x2_16 | Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x1_8 | Mode_1x1_4
        | Mode_1x4_16_Dup | Mode_1x2_16_Dup | Mode_1x1_16_Dup2 => 8,
        Mode_1x8_16 => 16,
    }
}

pub fn get_alt_macrotile_aspect(mode: MacroTileMode) -> u32 {
    use MacroTileMode::*;
    match mode {
        Mode_1x1_16 | Mode_1x1_16_Dup | Mode_1x1_8 | Mode_1x1_4 | Mode_1x1_2 | Mode_1x1_2_Dup
        | Mode_1x2_16_Dup | Mode_1x1_16_Dup2 | Mode_1x1_8_Dup | Mode_1x1_4_Dup
        | Mode_1x1_2_Dup2 | Mode_1x1_2_Dup3 => 1,
        Mode_1x4_16 | Mode_1x2_16 | Mode_1x8_16 | Mode_1x4_16_Dup => 2,
    }
}

pub fn is_macro_tiled(array_mode: ArrayMode) -> bool {
    use ArrayMode::*;
    !matches!(
        array_mode,
        ArrayLinearGeneral | ArrayLinearAligned | Array1DTiledThin1 | Array1DTiledThick
    )
}

pub fn is_prt(array_mode: ArrayMode) -> bool {
    use ArrayMode::*;
    matches!(
        array_mode,
        ArrayPrtTiledThin1
            | ArrayPrtTiledThick
            | ArrayPrt2DTiledThin1
            | ArrayPrt2DTiledThick
            | ArrayPrt3DTiledThin1
            | ArrayPrt3DTiledThick
    )
}

pub fn get_micro_tile_thickness(array_mode: ArrayMode) -> u32 {
    use ArrayMode::*;
    match array_mode {
        ArrayLinearGeneral | ArrayLinearAligned | Array1DTiledThin1 | Array2DTiledThin1
        | ArrayPrtTiledThin1 | ArrayPrt2DTiledThin1 | ArrayPrt3DTiledThin1 | Array3DTiledThin1 => 1,
        Array1DTiledThick | Array2DTiledThick | Array3DTiledThick | ArrayPrtTiledThick
        | ArrayPrt2DTiledThick | ArrayPrt3DTiledThick => 4,
        Array2DTiledXThick | Array3DTiledXThick => 8,
    }
}

pub fn get_pipe_count(pipe_config: PipeConfig) -> Result<u32, TilingError> {
    use PipeConfig::*;
    match pipe_config {
        P2 => Ok(2),
        P8_32x32_8x16 | P8_32x32_16x16 => Ok(8),
        P16_32x32_8x16 => Ok(16),
        other => Err(TilingError::UnknownPipeConfig(other.raw())),
    }
}

pub fn calculate_tile_split(
    tile_mode: TileMode,
    array_mode: ArrayMode,
    micro_tile_mode: MicroTileMode,
    bpp: u32,
) -> u32 {
    let sample_split = get_sample_split(tile_mode);
    let tile_split_hw = get_tile_split_hw(tile_mode);
    let tile_thickness = get_micro_tile_thickness(array_mode);
    let tile_bytes_1x = (bpp * MICROTILE_SIZE * MICROTILE_SIZE * tile_thickness).div_ceil(8);
    let color_tile_split = 256u32.max(sample_split * tile_bytes_1x);
    let tile_split = if micro_tile_mode == MicroTileMode::Depth {
        tile_split_hw
    } else {
        color_tile_split
    };
    DRAM_ROW_SIZE.min(tile_split)
}

pub fn calculate_macrotile_mode(
    tile_mode: TileMode,
    bpp: u32,
    num_samples: u32,
) -> Result<MacroTileMode, TilingError> {
    if !num_samples.is_power_of_two() || num_samples > 16 {
        return Err(TilingError::InvalidSamples(num_samples));
    }
    if !(1..=128).contains(&bpp) {
        return Err(TilingError::InvalidBpp(bpp));
    }
    let array_mode = get_array_mode(tile_mode);
    if !is_macro_tiled(array_mode) {
        return Err(TilingError::NotMacroTiled(tile_mode));
    }
    let micro_tile_mode = get_micro_tile_mode(tile_mode);
    let tile_thickness = get_micro_tile_thickness(array_mode);
    let tile_bytes_1x = bpp * MICROTILE_SIZE * MICROTILE_SIZE * tile_thickness / 8;
    let tile_split = calculate_tile_split(tile_mode, array_mode, micro_tile_mode, bpp);
    let tile_bytes = tile_split.min(num_samples * tile_bytes_1x);
    let mtm_index = (tile_bytes / 64).ilog2();
    let raw = if is_prt(array_mode) {
        mtm_index + 8
    } else {
        mtm_index
    };
    MacroTileMode::from_raw(raw).ok_or(TilingError::UnknownMacroTileMode(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_modes_map_to_their_array_and_micro_modes() {
        assert_eq!(
            get_array_mode(TileMode::Thin2DThin),
            ArrayMode::Array2DTiledThin1
        );
        assert_eq!(
            get_array_mode(TileMode::Depth2DThin64),
            ArrayMode::Array2DTiledThin1
        );
        assert_eq!(
            get_array_mode(TileMode::Thin3DThinPrt),
            ArrayMode::Array3DTiledThin1
        );
        assert_eq!(
            get_array_mode(TileMode::DisplayLinearGeneral),
            ArrayMode::ArrayLinearGeneral
        );
        assert_eq!(
            get_micro_tile_mode(TileMode::Thin2DThin),
            MicroTileMode::Thin
        );
        assert_eq!(
            get_micro_tile_mode(TileMode::Depth2DThin64),
            MicroTileMode::Depth
        );
        assert_eq!(
            get_micro_tile_mode(TileMode::Thick2DXThick),
            MicroTileMode::Thick
        );
        assert_eq!(
            get_micro_tile_mode(TileMode::Display1DThin),
            MicroTileMode::Display
        );
        assert_eq!(TileMode::Thin2DThin.name(), "Thin2DThin");
    }

    #[test]
    fn pipe_and_split_tables_match_the_hardware() {
        assert_eq!(
            get_pipe_config(TileMode::Thin2DThin),
            PipeConfig::P8_32x32_16x16
        );
        assert_eq!(
            get_pipe_config(TileMode::Thin3DThin),
            PipeConfig::P8_32x32_8x16
        );
        assert_eq!(
            get_pipe_config(TileMode::DisplayLinearGeneral),
            PipeConfig::P2
        );
        assert_eq!(
            get_alt_pipe_config(TileMode::Thin2DThin),
            PipeConfig::P16_32x32_8x16
        );
        assert_eq!(get_sample_split(TileMode::Thin2DThin), 2);
        assert_eq!(get_sample_split(TileMode::Depth2DThin64), 1);
        assert_eq!(get_tile_split_hw(TileMode::Depth2DThin1K), 1024);
        assert_eq!(get_tile_split_hw(TileMode::Thin2DThin), 64);
        assert_eq!(get_pipe_count(PipeConfig::P2), Ok(2));
        assert_eq!(get_pipe_count(PipeConfig::P8_32x32_16x16), Ok(8));
        assert!(get_pipe_count(PipeConfig::P4_32x32).is_err());
    }

    #[test]
    fn array_mode_properties() {
        assert!(!is_macro_tiled(ArrayMode::Array1DTiledThin1));
        assert!(is_macro_tiled(ArrayMode::Array2DTiledThin1));
        assert!(is_prt(ArrayMode::ArrayPrt2DTiledThin1));
        assert!(!is_prt(ArrayMode::Array2DTiledThin1));
        assert_eq!(get_micro_tile_thickness(ArrayMode::Array3DTiledThin1), 1);
        assert_eq!(get_micro_tile_thickness(ArrayMode::Array2DTiledThick), 4);
        assert_eq!(get_micro_tile_thickness(ArrayMode::Array2DTiledXThick), 8);
    }

    #[test]
    fn tile_split_follows_depth_and_color_rules() {
        // Color: max(256, 2 * 32bpp * 8 * 8 * 1 / 8) = max(256, 512) = 512.
        assert_eq!(
            calculate_tile_split(
                TileMode::Thin2DThin,
                ArrayMode::Array2DTiledThin1,
                MicroTileMode::Thin,
                32
            ),
            512
        );
        // Depth uses the hardware split (64), not the colour formula.
        assert_eq!(
            calculate_tile_split(
                TileMode::Depth2DThin64,
                ArrayMode::Array2DTiledThin1,
                MicroTileMode::Depth,
                32
            ),
            64
        );
        // The result is capped at a DRAM row.
        assert_eq!(
            calculate_tile_split(
                TileMode::Depth2DThin1K,
                ArrayMode::Array2DTiledThin1,
                MicroTileMode::Depth,
                32
            ),
            1024
        );
    }

    #[test]
    fn macrotile_mode_matches_the_known_encodings() {
        // 32 bpp thin 1x: tile bytes = min(512, 256) = 256; 256/64 = 4 -> index 2.
        assert_eq!(
            calculate_macrotile_mode(TileMode::Thin2DThin, 32, 1),
            Ok(MacroTileMode::Mode_1x1_16)
        );
        // Depth 64: tile bytes = min(64, 256) = 64; 64/64 = 1 -> index 0.
        assert_eq!(
            calculate_macrotile_mode(TileMode::Depth2DThin64, 32, 1),
            Ok(MacroTileMode::Mode_1x4_16)
        );
        // PRT shifts the index into the duplicate half.
        assert_eq!(
            calculate_macrotile_mode(TileMode::Thin2DThinPrt, 32, 1),
            Ok(MacroTileMode::Mode_1x2_16_Dup)
        );
        assert_eq!(
            calculate_macrotile_mode(TileMode::Thin2DThin, 32, 3),
            Err(TilingError::InvalidSamples(3))
        );
        assert_eq!(
            calculate_macrotile_mode(TileMode::Thin2DThin, 0, 1),
            Err(TilingError::InvalidBpp(0))
        );
        assert_eq!(
            calculate_macrotile_mode(TileMode::DisplayLinearAligned, 32, 1),
            Err(TilingError::NotMacroTiled(TileMode::DisplayLinearAligned))
        );
    }
}
