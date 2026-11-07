//! Reduced scene targets, ported from
//! `shadps4/video_core/renderer_vulkan/vk_scene_resolution.h`.

use std::collections::{HashMap, HashSet};

use crate::gpu::video_core::renderer_vulkan::scene_resolution::{Coherence, Size};
use crate::gpu::video_core::texture_cache::image_view::ImageViewInfo;

/// `SceneTargets::Target`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Target {
    pub image: u64,
    pub view: u64,
    pub layout: i32,
    pub usage: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Entry {
    pub source: u64,
    pub uid: u64,
    pub level: u32,
    pub image: u64,
    pub layout: i32,
    pub state: Coherence,
}

/// `SceneTargets`.
#[derive(Debug, Default)]
pub struct SceneTargets {
    pub size: Size,
    pub entries: HashMap<u64, Entry>,
    pub tracked: HashSet<u64>,
    pub copying: bool,
    pub debug: bool,
    pub generation: u64,
}

impl SceneTargets {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            generation: 1,
            ..Self::default()
        }
    }

    pub fn key(uid: u64, level: u32) -> u64 {
        (uid << 4) | u64::from(level)
    }

    /// `SetSize`; resizing destroys the proxies and bumps the generation.
    pub fn set_size(&mut self, size: Size) -> bool {
        if size == self.size {
            return false;
        }
        self.size = size;
        self.entries.clear();
        self.tracked.clear();
        self.generation += 1;
        true
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn reduced(&self) -> bool {
        self.size != Size::default()
    }

    /// `Eligible`; the proxy exists for 1920x1080 or half-resolution scenes.
    pub fn eligible(&self, image: &ImageStandIn) -> bool {
        (image.width == 1920 && image.height == 1080) || (image.width == 960 && image.height == 540)
    }

    /// `EligibleScene`.
    pub fn eligible_scene(&self, image: &ImageStandIn) -> bool {
        image.width == 1920 && image.height == 1080
    }

    /// `ProxySize`.
    pub fn proxy_size(&self, image: &ImageStandIn, _level: u32) -> Size {
        let scale = if image.width == 0 {
            1.0
        } else {
            f64::from(self.size.width) / f64::from(image.width)
        };
        Size {
            width: ((f64::from(image.width) * scale).round() as u32).max(1),
            height: ((f64::from(image.height) * scale).round() as u32).max(1),
        }
    }

    /// `Attachment`.
    pub fn attachment(&mut self, id: u64, _info: &ImageViewInfo) -> Target {
        let target = {
            let entry = self.get(id, 0);
            entry.state.proxy_write();
            Target {
                image: entry.image,
                view: entry.image,
                layout: entry.layout,
                usage: 0,
            }
        };
        self.tracked.insert(id);
        target
    }

    /// `Read`.
    pub fn read(&mut self, id: u64, _info: &ImageViewInfo, _stage: u64, _access: u32) -> Target {
        let entry = self.get(id, 0);
        Target {
            image: entry.image,
            view: entry.image,
            layout: entry.layout,
            usage: 0,
        }
    }

    /// `NativeAccess`; a native write invalidates the proxy.
    pub fn native_access(&mut self, image: ImageStandIn, _access: u32) {
        if let Some(entry) = self.entries.get_mut(&Self::key(image.uid, 0)) {
            entry.state.native_write();
        }
    }

    /// `SampleProxy`.
    pub fn sample_proxy(&self, image: &ImageStandIn, _info: &ImageViewInfo) -> Option<Target> {
        let entry = self.entries.get(&Self::key(image.uid, 0))?;
        entry.state.valid.then_some(Target {
            image: entry.image,
            view: entry.image,
            layout: entry.layout,
            usage: 0,
        })
    }

    /// `ProxyCurrent`.
    pub fn proxy_current(&self, image: &ImageStandIn, _level: u32, _levels: u32) -> bool {
        self.entries
            .get(&Self::key(image.uid, 0))
            .is_some_and(|entry| entry.state.valid)
    }

    /// `PrepareSample`.
    pub fn prepare_sample(&self, image: &ImageStandIn, _level: u32) -> i32 {
        self.entries
            .get(&Self::key(image.uid, 0))
            .map_or(0, |entry| entry.layout)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn tracks(&self, uid: u64) -> bool {
        !self.copying && self.tracked.contains(&uid)
    }

    pub fn resolve_all(&mut self) {
        for entry in self.entries.values_mut() {
            entry.state.resolved();
        }
    }

    fn get(&mut self, id: u64, level: u32) -> &mut Entry {
        let key = Self::key(id, level);
        self.entries.entry(key).or_insert(Entry {
            source: id,
            uid: id,
            level,
            image: 1,
            layout: 0,
            state: Coherence::default(),
        })
    }
}

/// The image fields this header reads; the texture cache owns the real image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageStandIn {
    pub uid: u64,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_targets_track_proxies_and_resize_generations() {
        let mut targets = SceneTargets::new(Size::default());
        assert!(!targets.reduced());
        assert_eq!(targets.generation(), 1);
        let image = ImageStandIn {
            uid: 7,
            width: 1920,
            height: 1080,
        };
        assert!(targets.eligible(&image));
        assert!(targets.eligible_scene(&image));
        assert_eq!(targets.proxy_size(&image, 0), Size::default());
        let target = targets.attachment(7, &ImageViewInfo::default());
        assert_eq!(target.image, 1);
        assert!(targets.tracks(7));
        assert!(targets.proxy_current(&image, 0, 1));
        assert!(targets
            .sample_proxy(&image, &ImageViewInfo::default())
            .is_some());
        assert!(targets.set_size(Size {
            width: 1280,
            height: 720
        }));
        assert_eq!(targets.generation(), 2);
        assert!(!targets.tracks(7));
        assert!(!targets.proxy_current(&image, 0, 1));
    }
}
