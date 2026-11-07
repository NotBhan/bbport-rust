//! Map-facing direction, ported from `legacy/gpu/shim/bbport_map_direction.h`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapDirection {
    pub x: f32,
    pub y: f32,
}

/// Character transform yaw points backwards; screen Y also reverses world Z.
pub fn facing_on_map(yaw: f32) -> MapDirection {
    MapDirection {
        x: -yaw.sin(),
        y: yaw.cos(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_zero_faces_up_and_quarter_turn_faces_left() {
        let up = facing_on_map(0.0);
        assert!(up.x.abs() < 1e-6);
        assert!((up.y - 1.0).abs() < 1e-6);

        let left = facing_on_map(std::f32::consts::FRAC_PI_2);
        assert!((left.x + 1.0).abs() < 1e-6);
        assert!(left.y.abs() < 1e-6);
    }
}
