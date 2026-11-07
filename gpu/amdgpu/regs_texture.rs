//! Texture registers, ported from `legacy/gpu/shadps4/video_core/amdgpu/regs_texture.h`.

/// `AmdGpu::BorderColorBuffer`: a 40-bit base address in the low bits of a 64-bit word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct BorderColorBuffer {
    pub base_addr: u64,
}

/// The field's width in the register.
pub const BORDER_COLOR_BUFFER_ADDR_BITS: u32 = 40;
const ADDR_MASK: u64 = (1 << BORDER_COLOR_BUFFER_ADDR_BITS) - 1;

impl BorderColorBuffer {
    pub const fn from_raw(raw: u64) -> Self {
        Self {
            base_addr: raw & ADDR_MASK,
        }
    }

    /// `BorderColorBuffer::Address` (the C++ `bit_cast<T>(base_addr << 8)`).
    pub const fn address(self) -> u64 {
        (self.base_addr & ADDR_MASK) << 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_low_forty_bits_are_the_address() {
        let buffer = BorderColorBuffer::from_raw(0xFFFF_FF00_0012_3400);
        assert_eq!(buffer.base_addr, 0x00_0012_3400);
        assert_eq!(buffer.address(), 0x0012_3400 << 8);
        assert_eq!(BorderColorBuffer::default().address(), 0);
        // The high bits of `base_addr` are not part of the address either (the C++ shifts then
        // truncates to the field width).
        assert_eq!(
            BorderColorBuffer {
                base_addr: u64::MAX
            }
            .address(),
            ADDR_MASK << 8
        );
    }
}
