//! The CB/DB extent register value, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/cb_db_extent.h`.

/// `AmdGpu::CbDbExtent`: a width and height packed into the low and high halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct CbDbExtent {
    pub raw: u32,
}

impl CbDbExtent {
    pub const fn from_parts(width: u16, height: u16) -> Self {
        Self {
            raw: (width as u32) | ((height as u32) << 16),
        }
    }

    pub const fn width(self) -> u16 {
        self.raw as u16
    }

    pub const fn height(self) -> u16 {
        (self.raw >> 16) as u16
    }

    /// `Valid`: a zero register means no extent was programmed.
    pub const fn valid(self) -> bool {
        self.raw != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_halves_round_trip_and_zero_is_invalid() {
        let extent = CbDbExtent::from_parts(1920, 1080);
        assert_eq!((extent.width(), extent.height()), (1920, 1080));
        assert!(extent.valid());
        assert_eq!(extent.raw, 1920 | (1080 << 16));
        assert!(!CbDbExtent::default().valid());
        assert_eq!(CbDbExtent { raw: 0x0001_0000 }.width(), 0);
        assert_eq!(CbDbExtent { raw: 0x0001_0000 }.height(), 1);
    }
}
