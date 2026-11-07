//! Scene-resolution presets, ported from
//! `shadps4/video_core/renderer_vulkan/scene_resolution.h`.

/// `Size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

/// `ForPreset`.
pub fn for_preset(preset: i32, output: Size) -> Size {
    const SCALES: [f64; 5] = [1.0, 1.5, 1.7, 2.0, 3.0];
    let scale = SCALES[preset.clamp(0, 4) as usize];
    let width = ((output.width as f64 / scale / 2.0).round() as i64).max(1) as u32 * 2;
    let height = ((output.height as f64 / scale / 2.0).round() as i64).max(1) as u32 * 2;
    Size { width, height }
}

/// `Pack`.
pub fn pack(size: Size) -> u32 {
    size.width | (size.height << 16)
}

/// `Unpack`.
pub fn unpack(packed: u32) -> Size {
    if packed == 0 {
        Size::default()
    } else {
        Size {
            width: packed & 65535,
            height: packed >> 16,
        }
    }
}

/// `Coherence`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Coherence {
    pub valid: bool,
    pub dirty: bool,
}

impl Coherence {
    /// `ProxyWrite`.
    pub fn proxy_write(&mut self) {
        self.valid = true;
        self.dirty = true;
    }

    /// `CopiedToProxy`.
    pub fn copied_to_proxy(&mut self) {
        self.valid = true;
        self.dirty = false;
    }

    /// `Resolved`.
    pub fn resolved(&mut self) {
        self.dirty = false;
    }

    /// `NativeWrite`.
    pub fn native_write(&mut self) {
        self.valid = false;
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_scale_and_stay_even() {
        let output = Size {
            width: 3840,
            height: 2160,
        };
        assert_eq!(for_preset(0, output), output);
        assert_eq!(
            for_preset(1, output),
            Size {
                width: 2560,
                height: 1440
            }
        );
        let small = for_preset(
            4,
            Size {
                width: 1,
                height: 1,
            },
        );
        assert_eq!(small.width % 2, 0);
        assert_eq!(small.height % 2, 0);
    }

    #[test]
    fn packing_round_trips() {
        let size = Size {
            width: 1920,
            height: 1080,
        };
        assert_eq!(unpack(pack(size)), size);
        assert_eq!(unpack(0), Size::default());
    }

    #[test]
    fn coherence_tracks_proxy_and_native_writes() {
        let mut coherence = Coherence::default();
        coherence.proxy_write();
        assert!(coherence.valid && coherence.dirty);
        coherence.copied_to_proxy();
        assert!(coherence.valid && !coherence.dirty);
        coherence.resolved();
        assert!(!coherence.dirty);
        coherence.native_write();
        assert!(!coherence.valid && !coherence.dirty);
    }
}
