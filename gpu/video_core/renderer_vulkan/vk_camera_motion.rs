//! Camera motion vectors, ported from
//! `shadps4/video_core/renderer_vulkan/vk_camera_motion.h`.

pub type Matrix4 = [f32; 16];
pub type Affine3x4 = [f32; 12];

/// `Camera`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Camera {
    pub view: Affine3x4,
    pub inv_view: Affine3x4,
    pub proj: [f32; 4],
    pub valid: bool,
}

fn projection(p: &[f32; 4]) -> Matrix4 {
    [
        p[0], 0.0, 0.0, 0.0, 0.0, p[1], 0.0, 0.0, 0.0, 0.0, p[2], 1.0, 0.0, 0.0, p[3], 0.0,
    ]
}

fn inverse_projection(p: &[f32; 4]) -> Matrix4 {
    [
        1.0 / p[0],
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / p[1],
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / p[3],
        0.0,
        0.0,
        1.0,
        -p[2] / p[3],
    ]
}

fn row_vectors(a: &Affine3x4) -> Matrix4 {
    [
        a[0], a[4], a[8], 0.0, a[1], a[5], a[9], 0.0, a[2], a[6], a[10], 0.0, a[3], a[7], a[11],
        1.0,
    ]
}

fn multiply4(a: &Matrix4, b: &Matrix4) -> Matrix4 {
    let mut out = [0.0f32; 16];
    for r in 0..4 {
        for c in 0..4 {
            out[r * 4 + c] = (0..4).map(|k| a[r * 4 + k] * b[k * 4 + c]).sum();
        }
    }
    out
}

/// `Multiply` for a 3x4 affine matrix (column vectors).
pub fn multiply3x4(a: &Affine3x4, b: &Affine3x4) -> Affine3x4 {
    let mut out = [0.0f32; 12];
    for row in 0..3 {
        for column in 0..3 {
            out[row * 4 + column] = (0..3).map(|k| a[row * 4 + k] * b[k * 4 + column]).sum();
        }
        out[row * 4 + 3] =
            a[row * 4] * b[3] + a[row * 4 + 1] * b[7] + a[row * 4 + 2] * b[11] + a[row * 4 + 3];
    }
    #[cfg(test)]
    let _ = out[0];
    out
}

/// `CameraMotion`.
#[derive(Debug, Default)]
pub struct CameraMotion {
    pub debug_overlay: bool,
    pub for_upscaler: bool,
    pub current: Camera,
    pub previous: Camera,
    pub frame_has_camera: bool,
    pub jitter: [f32; 2],
    pub previous_jitter: [f32; 2],
    pub render_size: [u32; 2],
    pub depth_id: u64,
}

impl CameraMotion {
    pub fn new(debug_overlay: bool, for_upscaler: bool) -> Self {
        Self {
            debug_overlay,
            for_upscaler,
            ..Self::default()
        }
    }

    pub fn invalidate_history(&mut self) {
        self.current.valid = false;
        self.previous.valid = false;
    }

    pub fn set_jitter(&mut self, jitter: [f32; 2]) {
        self.previous_jitter = self.jitter;
        self.jitter = jitter;
    }

    pub fn enabled(&self) -> bool {
        self.debug_overlay || self.for_upscaler
    }

    pub fn ready(&self) -> bool {
        self.current.valid && self.previous.valid && self.depth_id != 0
    }

    pub fn depth(&self) -> u64 {
        self.depth_id
    }

    pub fn render_size(&self) -> [u32; 2] {
        self.render_size
    }

    pub fn vertical_fov(&self) -> f32 {
        2.0 * (1.0 / self.current.proj[1]).atan()
    }

    pub fn near(&self) -> f32 {
        -self.current.proj[3] / self.current.proj[2]
    }

    pub fn far_plane(&self, fallback: f32) -> f32 {
        let zs = self.current.proj[2];
        let zo = self.current.proj[3];
        let far_z = if zs < 1.0 { zo / (1.0 - zs) } else { 0.0 };
        if far_z > self.near() && far_z < 1.0e7 {
            far_z
        } else {
            fallback
        }
    }

    /// `ClipTransforms`.
    pub fn clip_transforms(&self) -> Option<(Matrix4, Matrix4, Matrix4)> {
        if !self.current.valid
            || !self.previous.valid
            || self.current.proj[0] == 0.0
            || self.current.proj[1] == 0.0
            || self.current.proj[3] == 0.0
            || self.previous.proj[0] == 0.0
            || self.previous.proj[1] == 0.0
            || self.previous.proj[3] == 0.0
        {
            return None;
        }
        let view_to_clip = projection(&self.current.proj);
        let to_prev = multiply4(
            &multiply4(
                &inverse_projection(&self.current.proj),
                &row_vectors(&multiply3x4(&self.previous.view, &self.current.inv_view)),
            ),
            &projection(&self.previous.proj),
        );
        let to_current = multiply4(
            &multiply4(
                &inverse_projection(&self.previous.proj),
                &row_vectors(&multiply3x4(&self.current.view, &self.previous.inv_view)),
            ),
            &projection(&self.current.proj),
        );
        Some((view_to_clip, to_prev, to_current))
    }

    /// `TaaDepthParameters`.
    pub fn taa_depth_parameters(&self) -> [[f32; 4]; 3] {
        let transform = multiply3x4(&self.previous.view, &self.current.inv_view);
        [
            self.current.proj,
            self.previous.proj,
            [transform[8], transform[9], transform[10], transform[11]],
        ]
    }

    /// `OnConstants`; `data` is the 864-byte scene constant buffer as floats.
    pub fn on_constants(&mut self, data: &[f32]) {
        if data.len() < 64
            || data[0] != 3000.0
            || data[4] < 64.0
            || data[5] < 64.0
            || (data[1] * data[0] - 1.0).abs() > 1.0e-3
        {
            return;
        }
        if self.frame_has_camera {
            return;
        }
        self.previous = self.current;
        self.current.view.copy_from_slice(&data[8..20]);
        self.current.inv_view.copy_from_slice(&data[180..192]);
        self.current.proj = [data[52], data[57], data[62], data[63]];
        self.current.valid = self.current.proj[0] != 0.0 && self.current.proj[1] != 0.0;
        let size = [data[4] as u32, data[5] as u32];
        if size != self.render_size {
            self.render_size = size;
        }
        self.frame_has_camera = true;
    }

    pub fn on_gbuffer_pass(&mut self, depth: u64) {
        self.depth_id = depth;
    }

    pub fn on_display_pass(&mut self, _frame: u64) {
        self.frame_has_camera = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_constants_and_transforms_work() {
        let mut motion = CameraMotion::new(false, true);
        let mut data = [0.0f32; 192];
        data[0] = 3000.0;
        data[1] = 1.0 / 3000.0;
        data[4] = 1920.0;
        data[5] = 1080.0;
        data[8] = 1.0;
        data[180] = 1.0;
        data[52] = 1.0;
        data[57] = 1.0;
        data[62] = 1.0;
        data[63] = 0.1;
        motion.on_constants(&data);
        assert!(motion.current.valid);
        assert_eq!(motion.render_size(), [1920, 1080]);
        assert!((motion.near() + 0.1).abs() < 1e-6);
        assert!(motion.vertical_fov() > 0.0);
        motion.current.valid = true;
        motion.previous = motion.current;
        assert!(motion.clip_transforms().is_some());
        motion.on_gbuffer_pass(5);
        assert!(motion.ready());
        assert_eq!(motion.taa_depth_parameters().len(), 3);
    }
}
