//! In-game map controller state, ported from `shim/bbport_map.cpp`.

/// `Rect`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
}

/// `Point`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

/// `Triangle`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Triangle {
    pub vertices: [f32; 6],
    pub heights: [f32; 3],
}

/// `Segment`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
}

/// `Layer`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layer {
    pub height: f32,
    pub span: f32,
    pub anchor: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub has_vertex_heights: bool,
    pub rects: Vec<Rect>,
    pub triangles: Vec<Triangle>,
    pub outlines: Vec<Vec<Point>>,
    pub walls: Vec<Segment>,
}

/// `Lamp`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub name: String,
}

/// `Marker`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Marker {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub kind: String,
    pub name: String,
    pub entity: i32,
}

/// `Area`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Area {
    pub id: String,
    pub name: String,
    pub bounds: [f32; 4],
    pub layers: Vec<Layer>,
    pub lamps: Vec<Lamp>,
    pub markers: Vec<Marker>,
    pub loaded: bool,
    pub error: String,
}

/// `BbMapLocation`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapLocation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
    pub layer: i32,
    pub floor: i32,
}

/// The map controller state (`MapUiScale`, follow/zoom).
#[derive(Debug)]
pub struct MapController {
    pub location: MapLocation,
    pub selected: MapLocation,
    pub follow: bool,
    pub zoom: f32,
    pub area: Option<Area>,
}

impl Default for MapController {
    fn default() -> Self {
        Self {
            location: MapLocation {
                x: 0.5,
                y: 3.8,
                z: 0.5,
                angle: 0.0,
                layer: 0,
                floor: 1,
            },
            selected: MapLocation::default(),
            follow: true,
            zoom: 9.0,
            area: None,
        }
    }
}

impl MapController {
    /// `MapUiScale`.
    pub fn ui_scale(height: f32) -> f32 {
        (height / 1080.0).clamp(0.5, 2.0)
    }

    pub fn load_area(&mut self, area: Area) {
        self.area = Some(area);
    }

    pub fn selected_is_followed(&self) -> bool {
        self.follow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_controller_holds_location_and_area() {
        let mut map = MapController::default();
        assert!(map.follow);
        assert_eq!(MapController::ui_scale(2160.0), 2.0);
        map.load_area(Area {
            id: "cathedral".to_string(),
            loaded: true,
            ..Area::default()
        });
        assert!(map.area.as_ref().unwrap().loaded);
        assert_eq!(map.location.floor, 1);
    }
}
