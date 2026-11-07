//! Window point on the menus' 1920x1080 Scaleform stage, ported from
//! `legacy/gpu/shim/stage_point.h`.

/// `(mx, my)` in window units, the window `(ww, wh)`, the frame `(fw, fh)` in pixels.
/// Returns stage coordinates when the point is on the stage.
pub fn point(mx: f32, my: f32, ww: f32, wh: f32, fw: f32, fh: f32) -> Option<(f32, f32)> {
    if ww <= 0.0 || wh <= 0.0 || fw <= 0.0 || fh <= 0.0 {
        return None;
    }
    let fit = (ww / fw).min(wh / fh);
    let fx = (mx - (ww - fw * fit) * 0.5) / fit;
    let fy = (my - (wh - fh * fit) * 0.5) / fit;
    let stage = (fw / 1920.0).min(fh / 1080.0);
    let x = (fx - (fw - 1920.0 * stage) * 0.5) / stage;
    let y = (fy - (fh - 1080.0 * stage) * 0.5) / stage;
    (x >= 0.0 && y >= 0.0 && x <= 1920.0 && y <= 1080.0).then_some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_maps_to_the_stage_centre() {
        let (x, y) = point(960.0, 540.0, 1920.0, 1080.0, 1920.0, 1080.0).unwrap();
        assert!((x - 960.0).abs() < 1e-4);
        assert!((y - 540.0).abs() < 1e-4);
    }

    #[test]
    fn outside_points_and_empty_sizes_are_rejected() {
        assert!(point(-1.0, 540.0, 1920.0, 1080.0, 1920.0, 1080.0).is_none());
        assert!(point(0.0, 0.0, 0.0, 1080.0, 1920.0, 1080.0).is_none());
    }

    #[test]
    fn letterboxed_16_9_frame_in_a_wider_window() {
        // A 21:9 window with a 16:9 frame: the stage is centred.
        let (x, y) = point(1280.0, 540.0, 2560.0, 1080.0, 2560.0, 1080.0).unwrap();
        assert!((0.0..=1920.0).contains(&x));
        assert!((y - 540.0).abs() < 1e-4);
    }
}
