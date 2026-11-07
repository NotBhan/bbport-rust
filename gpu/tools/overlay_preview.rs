//! Overlay preview options, ported from `legacy/gpu/tools/overlay_preview.cpp`.

use std::path::PathBuf;

/// `ImGuiKey` entries the preview can press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKey {
    Up,
    Down,
    Left,
    Right,
    A,
}

impl PreviewKey {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "a" => Some(Self::A),
            _ => None,
        }
    }
}

/// Parsed `overlay-preview` arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewOptions {
    pub output: PathBuf,
    pub tab: i32,
    pub width: i32,
    pub height: i32,
    pub background: Option<PathBuf>,
    pub keys: Vec<PreviewKey>,
}

impl PreviewOptions {
    pub fn parse(args: &[String], keys: Option<&str>) -> Option<Self> {
        if args.len() < 2 {
            return None;
        }
        let tab = args
            .get(2)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let width = if args.len() > 4 {
            args[3].parse().unwrap_or(1920)
        } else {
            1920
        };
        let height = if args.len() > 4 {
            args[4].parse().unwrap_or(1080)
        } else {
            1080
        };
        let background = args.get(5).map(PathBuf::from);
        let keys = keys
            .map(|list| {
                list.split(',')
                    .filter_map(PreviewKey::parse)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(Self {
            output: PathBuf::from(&args[1]),
            tab,
            width,
            height,
            background,
            keys,
        })
    }

    /// `frames = 3 + keys * 4`.
    pub fn frame_count(&self) -> i32 {
        3 + self.keys.len() as i32 * 4
    }

    /// Whether `frame` holds `key` pressed (the first two frames of each four).
    pub fn key_pressed(&self, frame: i32, key_index: usize) -> bool {
        frame >= 2 && (frame - 2) / 4 == key_index as i32 && (frame - 2) % 4 < 2
    }

    /// Background cover scale (`max(width/bw, height/bh)`).
    pub fn fit_scale(&self, background_width: f32, background_height: f32) -> f32 {
        (self.width as f32 / background_width).max(self.height as f32 / background_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_options_parse_keys_and_frames() {
        let options = PreviewOptions::parse(
            &[
                "bin".into(),
                "out.bmp".into(),
                "2".into(),
                "800".into(),
                "600".into(),
                "bg.png".into(),
            ],
            Some("down,right,a,unknown"),
        )
        .unwrap();
        assert_eq!(options.tab, 2);
        assert_eq!((options.width, options.height), (800, 600));
        assert_eq!(options.background, Some(PathBuf::from("bg.png")));
        assert_eq!(options.keys.len(), 3);
        assert_eq!(options.frame_count(), 15);
        assert!(options.key_pressed(2, 0));
        assert!(!options.key_pressed(4, 0));
        assert!(options.key_pressed(6, 1));
        assert_eq!(options.fit_scale(400.0, 300.0), 2.0);
        assert!(PreviewOptions::parse(&["bin".into()], None).is_none());
    }
}
