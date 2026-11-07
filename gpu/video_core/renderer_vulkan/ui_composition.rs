//! UI composition decisions, ported from
//! `shadps4/video_core/renderer_vulkan/ui_composition.h`.

use std::sync::OnceLock;

/// `Background`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    None,
    Copy,
    Temporal,
}

/// `Choose`.
pub const fn choose(
    scaled: bool,
    ui_draw: bool,
    scene_ready: bool,
    fsr_active: bool,
) -> Background {
    if !scaled || !ui_draw {
        return Background::None;
    }
    if scene_ready && fsr_active {
        Background::Temporal
    } else {
        Background::Copy
    }
}

/// `MovieViewport`; `BB_UI_RES=WxH` overrides the 1920x1080 stage.
pub fn movie_viewport() -> (u32, u32) {
    static SIZE: OnceLock<(u32, u32)> = OnceLock::new();
    *SIZE.get_or_init(|| {
        let mut width = 1920;
        let mut height = 1080;
        if let Ok(value) = std::env::var("BB_UI_RES") {
            if let Some((w, h)) = value.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                    if w != 0 && h != 0 {
                        width = w;
                        height = h;
                    }
                }
            }
        }
        (width, height)
    })
}

/// `NativeViewport`.
pub fn native_viewport(width: f32, height: f32) -> bool {
    let (w, h) = movie_viewport();
    (width.abs() - w as f32).abs() < 0.5 && (height.abs() - h as f32).abs() < 0.5
}

/// `MovieShader`.
pub const fn movie_shader(hash: u64) -> bool {
    matches!(
        hash,
        0x34e8a281
            | 0x81d336ce
            | 0x09957251
            | 0x24042a9b
            | 0xa400228b
            | 0xbb4d5f8c
            | 0xe94b5b06
            | 0x9951d199
            | 0x44072322
            | 0xa334a845
    )
}

/// `Scale`.
pub fn scale(
    guest_width: u32,
    guest_height: u32,
    output_width: u32,
    output_height: u32,
    native_coordinates: bool,
) -> [f32; 2] {
    let (ui_width, ui_height) = movie_viewport();
    [
        output_width as f32
            / if native_coordinates {
                ui_width
            } else {
                guest_width
            } as f32,
        output_height as f32
            / if native_coordinates {
                ui_height
            } else {
                guest_height
            } as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_background_choice_matches_the_header() {
        assert_eq!(choose(false, true, true, true), Background::None);
        assert_eq!(choose(true, false, true, true), Background::None);
        assert_eq!(choose(true, true, false, true), Background::Copy);
        assert_eq!(choose(true, true, true, false), Background::Copy);
        assert_eq!(choose(true, true, true, true), Background::Temporal);
    }

    #[test]
    fn movie_shaders_and_viewports_are_recognized() {
        assert!(movie_shader(0x34e8a281));
        assert!(movie_shader(0xa334a845));
        assert!(!movie_shader(0x12345678));
        let (w, h) = movie_viewport();
        assert!(native_viewport(w as f32, h as f32));
        assert!(!native_viewport(w as f32 + 10.0, h as f32));
        let scale = scale(1920, 1080, 3840, 2160, false);
        assert_eq!(scale, [2.0, 2.0]);
    }
}
