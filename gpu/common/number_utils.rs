//! Guest number formats, ported from `legacy/gpu/shadps4/common/number_utils.h` and its
//! `.cpp`: the packed float formats the vertex and texture paths unpack, and the normalised
//! integer formats.

const UF11_EXPONENT_SHIFT: u32 = 6;
const UF10_EXPONENT_SHIFT: u32 = 5;
const F32_INFINITY: u32 = 0x7F80_0000;

/// `NumberUtils::Uf11ToF32`: a 5-bit exponent with a 6-bit mantissa.
pub fn uf11_to_f32(val: u16) -> f32 {
    let val = val as u32;
    let exponent = ((val & 0x07C0) >> UF11_EXPONENT_SHIFT) as i32;
    let mantissa = val & 0x003F;
    if exponent == 0 {
        if mantissa != 0 {
            return (1.0 / (1 << 20) as f32) * mantissa as f32;
        }
        return 0.0;
    }
    if exponent == 31 {
        return f32::from_bits(F32_INFINITY | mantissa);
    }
    let exponent = exponent - 15;
    let scale = if exponent < 0 {
        1.0 / (1 << -exponent) as f32
    } else {
        (1 << exponent) as f32
    };
    let decimal = 1.0 + mantissa as f32 / 64.0;
    scale * decimal
}

/// `NumberUtils::Uf10ToF32`: a 5-bit exponent with a 5-bit mantissa.
pub fn uf10_to_f32(val: u16) -> f32 {
    let val = val as u32;
    let exponent = ((val & 0x03E0) >> UF10_EXPONENT_SHIFT) as i32;
    let mantissa = val & 0x001F;
    if exponent == 0 {
        if mantissa != 0 {
            return (1.0 / (1 << 19) as f32) * mantissa as f32;
        }
        return 0.0;
    }
    if exponent == 31 {
        return f32::from_bits(F32_INFINITY | mantissa);
    }
    let exponent = exponent - 15;
    let scale = if exponent < 0 {
        1.0 / (1 << -exponent) as f32
    } else {
        (1 << exponent) as f32
    };
    let decimal = 1.0 + mantissa as f32 / 32.0;
    scale * decimal
}

/// `NumberUtils::Uf16ToF32`: the C++ casts through `half_float::half`, so the port converts the
/// IEEE 754 half directly.
pub fn uf16_to_f32(val: u16) -> f32 {
    let sign = ((val & 0x8000) as u32) << 16;
    let exponent = (val >> 10) & 0x1F;
    let mantissa = (val & 0x03FF) as u32;
    let bits = match exponent {
        0 => {
            if mantissa == 0 {
                sign
            } else {
                // A subnormal half: normalise it into a full float.
                let mut mantissa = mantissa;
                let mut exponent = -14i32;
                while mantissa & 0x400 == 0 {
                    mantissa <<= 1;
                    exponent -= 1;
                }
                sign | (((exponent + 127) as u32) << 23) | ((mantissa & 0x3FF) << 13)
            }
        }
        31 => sign | F32_INFINITY | (mantissa << 13),
        _ => sign | ((exponent as u32 + 112) << 23) | (mantissa << 13),
    };
    f32::from_bits(bits)
}

/// `NumberUtils::U2ToUnorm`.
pub fn u2_to_unorm(val: u8) -> f32 {
    val as f32 / 3.0
}

/// `NumberUtils::S2ToSnorm`. The C++'s factor is one, so a two-bit value maps to itself.
pub fn s2_to_snorm(val: i8) -> f32 {
    val as f32
}

/// `NumberUtils::U4ToUnorm`.
pub fn u4_to_unorm(val: u8) -> f32 {
    val as f32 / 15.0
}

/// `NumberUtils::S4ToSnorm`.
pub fn s4_to_snorm(val: i8) -> f32 {
    val as f32 / 7.0
}

/// `NumberUtils::U5ToUnorm`.
pub fn u5_to_unorm(val: u8) -> f32 {
    val as f32 / 31.0
}

/// `NumberUtils::S5ToSnorm`.
pub fn s5_to_snorm(val: i8) -> f32 {
    val as f32 / 15.0
}

/// `NumberUtils::U6ToUnorm`.
pub fn u6_to_unorm(val: u8) -> f32 {
    val as f32 / 63.0
}

/// `NumberUtils::S6ToSnorm`.
pub fn s6_to_snorm(val: i8) -> f32 {
    val as f32 / 31.0
}

/// `NumberUtils::U8ToUnorm`.
pub fn u8_to_unorm(val: u8) -> f32 {
    val as f32 / 255.0
}

/// `NumberUtils::S8ToSnorm`.
pub fn s8_to_snorm(val: i8) -> f32 {
    val as f32 / 127.0
}

/// `NumberUtils::U10ToUnorm`.
pub fn u10_to_unorm(val: u16) -> f32 {
    val as f32 / 1023.0
}

/// `NumberUtils::S10ToSnorm`.
pub fn s10_to_snorm(val: i16) -> f32 {
    val as f32 / 511.0
}

/// `NumberUtils::U16ToUnorm`.
pub fn u16_to_unorm(val: u16) -> f32 {
    val as f32 / 65535.0
}

/// `NumberUtils::S16ToSnorm`.
pub fn s16_to_snorm(val: i16) -> f32 {
    val as f32 / 32767.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UF11 value with the given exponent and mantissa.
    fn uf11(exponent: u16, mantissa: u16) -> u16 {
        (exponent << UF11_EXPONENT_SHIFT as u16) | mantissa
    }

    fn uf10(exponent: u16, mantissa: u16) -> u16 {
        (exponent << UF10_EXPONENT_SHIFT as u16) | mantissa
    }

    #[test]
    fn the_uf11_format_covers_zero_the_bias_and_infinity() {
        // Exponent 15 is the bias: the value is one plus the mantissa over 64.
        assert_eq!(uf11_to_f32(uf11(15, 0)), 1.0);
        assert_eq!(uf11_to_f32(uf11(15, 32)), 1.5);
        assert_eq!(uf11_to_f32(uf11(16, 0)), 2.0);
        assert_eq!(uf11_to_f32(uf11(14, 0)), 0.5);
        assert_eq!(uf11_to_f32(uf11(0, 0)), 0.0);
        assert_eq!(uf11_to_f32(uf11(0, 1)), 1.0 / (1 << 20) as f32);
        assert!(uf11_to_f32(uf11(31, 0)).is_infinite());
        assert!(uf11_to_f32(uf11(31, 1)).is_nan());
        // The exponent field starts at bit 6, so the bias sits at 15 << 6.
        assert_eq!(uf11_to_f32(15 << UF11_EXPONENT_SHIFT as u16), 1.0);
        assert!(
            uf11_to_f32(0x07C0).is_infinite(),
            "the top exponent is reserved"
        );
    }

    #[test]
    fn the_uf10_format_uses_the_smaller_mantissa() {
        assert_eq!(uf10_to_f32(uf10(15, 0)), 1.0);
        assert_eq!(uf10_to_f32(uf10(15, 16)), 1.5);
        assert_eq!(uf10_to_f32(uf10(16, 0)), 2.0);
        assert_eq!(uf10_to_f32(uf10(0, 1)), 1.0 / (1 << 19) as f32);
        assert!(uf10_to_f32(uf10(31, 0)).is_infinite());
        // The exponent field starts at bit 5.
        assert_eq!(uf10_to_f32(15 << UF10_EXPONENT_SHIFT as u16), 1.0);
        assert!(
            uf10_to_f32(0x03E0).is_infinite(),
            "the top exponent is reserved"
        );
    }

    #[test]
    fn the_half_conversion_handles_the_special_cases() {
        assert_eq!(uf16_to_f32(0x0000), 0.0);
        assert_eq!(uf16_to_f32(0x8000), -0.0);
        assert_eq!(uf16_to_f32(0x3C00), 1.0);
        assert_eq!(uf16_to_f32(0x4000), 2.0);
        assert_eq!(uf16_to_f32(0xC000), -2.0);
        assert_eq!(uf16_to_f32(0x3800), 0.5);
        assert_eq!(uf16_to_f32(0x7BFF), 65504.0, "the largest finite half");
        assert!(uf16_to_f32(0x7C00).is_infinite());
        assert!(uf16_to_f32(0xFC00).is_infinite());
        assert!(uf16_to_f32(0x7C01).is_nan());
        assert_eq!(
            uf16_to_f32(0x0001),
            2f32.powi(-24),
            "the smallest subnormal"
        );
        assert_eq!(
            uf16_to_f32(0x03FF),
            1023.0 * 2f32.powi(-24),
            "a large subnormal"
        );
    }

    #[test]
    fn the_normalised_formats_reach_their_ends() {
        assert_eq!(u2_to_unorm(0), 0.0);
        assert_eq!(u2_to_unorm(3), 1.0);
        assert_eq!(
            s2_to_snorm(-2),
            -2.0,
            "the two-bit snorm keeps its factor of one"
        );
        assert_eq!(s2_to_snorm(1), 1.0);
        assert_eq!(u4_to_unorm(15), 1.0);
        assert_eq!(s4_to_snorm(-8), -8.0 / 7.0);
        assert_eq!(u5_to_unorm(31), 1.0);
        assert_eq!(s5_to_snorm(-16), -16.0 / 15.0);
        assert_eq!(u6_to_unorm(63), 1.0);
        assert_eq!(s6_to_snorm(-32), -32.0 / 31.0);
        assert_eq!(u8_to_unorm(255), 1.0);
        assert_eq!(s8_to_snorm(-127), -1.0);
        assert_eq!(u10_to_unorm(1023), 1.0);
        assert_eq!(s10_to_snorm(-511), -1.0);
        assert_eq!(u16_to_unorm(65535), 1.0);
        assert_eq!(s16_to_snorm(-32767), -1.0);
        assert_eq!(u8_to_unorm(128), 128.0 / 255.0);
    }
}
