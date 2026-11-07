//! Renderer-independent Atlas layout and cut-decoration geometry, ported from
//! `legacy/gpu/shim/bbport_atlas_style.h`.

use std::collections::HashMap;

pub const DESIGN_WIDTH: f32 = 1280.0;
pub const DESIGN_HEIGHT: f32 = 720.0;
/// LOD levels used by `CutCache`.
pub const LOD_COUNT: usize = 18;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Line {
    pub a: Point,
    pub b: Point,
    pub wear: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Triangle {
    pub p: [Point; 3],
}

/// The header's `Box` (renamed to avoid shadowing `std::boxed::Box`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Layout {
    pub scale: f32,
    pub x: f32,
    pub y: f32,
}

impl Layout {
    pub const EDGE_FADE: f32 = 34.0;
    pub const FRAME_LEFT: f32 = 10.0;
    pub const FRAME_TOP: f32 = 8.0;
    pub const FRAME_RIGHT: f32 = 1270.0;
    pub const FRAME_BOTTOM: f32 = 710.0;
    pub const TITLE_TOP: f32 = 16.0;
    pub const TITLE_SIZE: f32 = 51.0;
    pub const TITLE_MIN: f32 = 27.0;
    pub const TITLE_WIDTH: f32 = 1040.0;
    pub const TITLE_REGION_H: f32 = 47.0;
    pub const HEADER_RULE: f32 = 67.0;
    pub const HEADER_LEFT: f32 = 320.0;
    pub const HEADER_RIGHT: f32 = 960.0;
    pub const SUBTITLE_TOP: f32 = 77.0;
    pub const SUBTITLE_SIZE: f32 = 29.0;
    pub const SUBTITLE_MIN: f32 = 18.0;
    pub const SUBTITLE_WIDTH: f32 = 980.0;
    pub const SUBTITLE_REGION_H: f32 = 30.0;
    pub const FOOTER_RULE: f32 = 659.0;
    pub const FOOTER_BASELINE: f32 = 688.0;
    pub const FOOTER_SIZE: f32 = 18.0;
    pub const COMPASS_X: f32 = 77.0;
    pub const COMPASS_Y: f32 = 105.0;
    pub const COMPASS_SIZE: f32 = 73.0;
    pub const PLAYER_SIZE: f32 = 35.0;
    pub const LEGEND_X: f32 = 1163.0;
    pub const LEGEND_Y: f32 = 524.0;
    pub const LEGEND_W: f32 = 18.0;
    pub const LEGEND_H: f32 = 104.0;
    pub const KEY_X: f32 = 1126.0;
    pub const KEY_Y: f32 = 644.0;
    pub const KEY_W: f32 = 116.0;
    pub const KEY_H: f32 = 54.0;
    pub const FOOTER_X: f32 = 152.0;
    pub const FOOTER_W: f32 = 964.0;

    pub fn new(w: f32, h: f32, sx: f32, sy: f32) -> Self {
        let scale = (w / DESIGN_WIDTH).min(h / DESIGN_HEIGHT).max(0.01);
        Self {
            scale,
            x: sx + (w - DESIGN_WIDTH * scale) * 0.5,
            y: sy + (h - DESIGN_HEIGHT * scale) * 0.5,
        }
    }

    pub fn at(&self, px: f32, py: f32) -> Point {
        Point {
            x: self.x + px * self.scale,
            z: self.y + py * self.scale,
        }
    }

    pub fn rect(&self, px: f32, py: f32, w: f32, h: f32) -> Rect {
        Rect {
            x: self.x + px * self.scale,
            y: self.y + py * self.scale,
            w: w * self.scale,
            h: h * self.scale,
        }
    }

    /// Leave room for the subtitle's descenders before any map geometry.
    pub fn canvas(&self) -> Rect {
        self.rect(230.0, 114.0, 835.0, 538.0)
    }

    pub fn travel_dialog(&self) -> Rect {
        self.rect(
            (DESIGN_WIDTH - 350.0) * 0.5,
            (DESIGN_HEIGHT - 168.0) * 0.5,
            350.0,
            168.0,
        )
    }

    /// Every enabled control uses the same row; disabling travel leaves no holes.
    pub fn footer_slot(&self, index: i32, count: i32) -> Rect {
        let count = count.max(1);
        self.rect(
            Self::FOOTER_X + Self::FOOTER_W * index as f32 / count as f32,
            666.0,
            Self::FOOTER_W / count as f32,
            35.0,
        )
    }

    pub fn lamp_on_map(same_area: bool, discovered: bool) -> bool {
        same_area && discovered
    }
}

pub fn cross(a: Point, b: Point) -> f32 {
    a.x * b.z - a.z * b.x
}

pub fn sub(a: Point, b: Point) -> Point {
    Point {
        x: a.x - b.x,
        z: a.z - b.z,
    }
}

pub fn mix(a: Point, b: Point, t: f32) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        z: a.z + (b.z - a.z) * t,
    }
}

/// Exact segment/convex-polygon clipping. Returns the `[lo, hi]` parameter interval of the
/// segment inside the polygon, or `None` when it misses.
pub fn clip<const N: usize>(
    polygon: &[Point; N],
    a: Point,
    b: Point,
    epsilon: f32,
) -> Option<(f32, f32)> {
    let mut area = 0.0;
    for i in 0..N {
        area += cross(polygon[i], polygon[(i + 1) % N]);
    }
    if area.abs() < 1e-8 {
        return None;
    }
    let sign = if area > 0.0 { 1.0 } else { -1.0 };
    let d = sub(b, a);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for i in 0..N {
        let e = sub(polygon[(i + 1) % N], polygon[i]);
        let edge_length = e.x.hypot(e.z);
        let start = sign * cross(e, sub(a, polygon[i])) + epsilon * edge_length;
        let slope = sign * cross(e, d);
        if slope.abs() < 1e-9 {
            if start < 0.0 {
                return None;
            }
            continue;
        }
        let t = -start / slope;
        if slope > 0.0 {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
        if lo > hi {
            return None;
        }
    }
    (hi >= 0.0 && lo <= 1.0).then_some((lo, hi))
}

pub fn cell_key(x: i32, z: i32) -> i64 {
    (((x as u32 as u64) << 32) | z as u32 as u64) as i64
}

#[derive(Default)]
pub struct FloorIndex {
    pub triangles: Vec<Triangle>,
    cells: HashMap<i64, Vec<u32>>,
    broad: Vec<u32>,
    marks: Vec<u32>,
    serial: u32,
}

impl FloorIndex {
    pub const CELL: f32 = 8.0;

    pub fn build(&mut self, geometry: Vec<Triangle>) {
        self.triangles = geometry;
        self.marks = vec![0; self.triangles.len()];
        self.cells.clear();
        self.broad.clear();
        for (index, triangle) in self.triangles.iter().enumerate() {
            let t = triangle.p;
            let x0 = (t[0].x.min(t[1].x).min(t[2].x) / Self::CELL).floor() as i32;
            let x1 = (t[0].x.max(t[1].x).max(t[2].x) / Self::CELL).floor() as i32;
            let z0 = (t[0].z.min(t[1].z).min(t[2].z) / Self::CELL).floor() as i32;
            let z1 = (t[0].z.max(t[1].z).max(t[2].z) / Self::CELL).floor() as i32;
            if (x1 - x0 + 1) as i64 * (z1 - z0 + 1) as i64 > 512 {
                self.broad.push(index as u32);
                continue;
            }
            for x in x0..=x1 {
                for z in z0..=z1 {
                    self.cells
                        .entry(cell_key(x, z))
                        .or_default()
                        .push(index as u32);
                }
            }
        }
    }

    pub fn query(&mut self, a: Point, b: Point, mut f: impl FnMut(&Triangle)) {
        self.serial = self.serial.wrapping_add(1);
        if self.serial == 0 {
            self.marks.fill(0);
            self.serial = 1;
        }
        let serial = self.serial;
        let FloorIndex {
            triangles,
            cells,
            broad,
            marks,
            ..
        } = self;
        let mut ids: Vec<u32> = Vec::new();
        let x0 = ((a.x.min(b.x) - 0.02) / Self::CELL).floor() as i32;
        let x1 = ((a.x.max(b.x) + 0.02) / Self::CELL).floor() as i32;
        let z0 = ((a.z.min(b.z) - 0.02) / Self::CELL).floor() as i32;
        let z1 = ((a.z.max(b.z) + 0.02) / Self::CELL).floor() as i32;
        for x in x0..=x1 {
            for z in z0..=z1 {
                if let Some(list) = cells.get(&cell_key(x, z)) {
                    for &id in list {
                        if marks[id as usize] != serial {
                            marks[id as usize] = serial;
                            ids.push(id);
                        }
                    }
                }
            }
        }
        for &id in broad.iter() {
            if marks[id as usize] != serial {
                marks[id as usize] = serial;
                ids.push(id);
            }
        }
        for id in ids {
            f(&triangles[id as usize]);
        }
    }

    pub fn contains(&mut self, p: Point) -> bool {
        let mut hit = false;
        self.query(p, p, |t| {
            if clip(&t.p, p, p, 0.002).is_some() {
                hit = true;
            }
        });
        hit
    }

    pub fn outside(&mut self, a: Point, b: Point, wear: u32) -> Vec<Line> {
        let mut occupied: Vec<(f32, f32)> = Vec::new();
        self.query(a, b, |t| {
            if let Some((lo, hi)) = clip(&t.p, a, b, 0.012) {
                occupied.push((lo.max(0.0), hi.min(1.0)));
            }
        });
        occupied.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut result = Vec::new();
        let mut begin = 0.0f32;
        let emit = |lo: f32, hi: f32, result: &mut Vec<Line>| {
            if hi - lo > 0.001 {
                result.push(Line {
                    a: mix(a, b, lo),
                    b: mix(a, b, hi),
                    wear,
                });
            }
        };
        for (lo, hi) in occupied {
            if lo > begin {
                emit(begin, lo, &mut result);
            }
            begin = begin.max(hi);
        }
        if begin < 1.0 {
            emit(begin, 1.0, &mut result);
        }
        result
    }
}

pub fn hash(mut n: u32) -> u32 {
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846ca68b);
    n ^ (n >> 16)
}

#[derive(Default, Clone)]
pub struct Lod {
    pub built: bool,
    pub strokes: Vec<Line>,
}

/// Cached cut decorations; the wall geometry is hashed into wear so the strokes stay stable.
#[derive(Default)]
pub struct CutCache {
    pub ready: bool,
    pub floors: FloorIndex,
    pub layers: Vec<Vec<Lod>>,
}

impl CutCache {
    pub fn build(&mut self, floor_geometry: Vec<Triangle>, layer_count: usize) {
        self.floors.build(floor_geometry);
        self.layers = vec![vec![Lod::default(); LOD_COUNT]; layer_count];
        self.ready = true;
    }

    /// The diagonal's perpendicular pitch is `spacing / sqrt(2)`. A 1.5x hierarchy keeps the
    /// nearest-LOD pitch in 4.49..6.74 design pixels.
    pub fn spacing(level: i32) -> f32 {
        0.125 * 1.5f32.powi(level.clamp(0, LOD_COUNT as i32 - 1))
    }

    pub fn level(zoom: f32, scale: f32) -> usize {
        let ratio = 5.5 * std::f32::consts::SQRT_2 * scale / (zoom * 0.125).max(0.0001);
        let level = (ratio.ln() / 1.5f32.ln()).round() as i32;
        level.clamp(0, LOD_COUNT as i32 - 1) as usize
    }

    pub fn get(&mut self, layer: usize, level: i32, walls: &[Line]) -> &[Line] {
        let index = level.clamp(0, LOD_COUNT as i32 - 1) as usize;
        let CutCache { floors, layers, .. } = self;
        let lod = &mut layers[layer][index];
        if lod.built {
            return &lod.strokes;
        }
        lod.built = true;
        let spacing = Self::spacing(index as i32);
        for (wi, wall) in walls.iter().enumerate() {
            let d = sub(wall.b, wall.a);
            let length = d.x.hypot(d.z);
            if !(0.03..=10000.0).contains(&length) {
                continue;
            }
            let normal = Point {
                x: -d.z / length,
                z: d.x / length,
            };
            let pieces = ((length / 1.25).ceil() as i32).max(1);
            for piece in 0..pieces {
                let a = mix(wall.a, wall.b, piece as f32 / pieces as f32);
                let b = mix(wall.a, wall.b, (piece + 1) as f32 / pieces as f32);
                let mid = mix(a, b, 0.5);
                // A narrow .65 m band near a proven floor boundary is all we draw.
                let mut supported = false;
                for side in [-0.35f32, 0.35] {
                    if floors.contains(Point {
                        x: mid.x + normal.x * side,
                        z: mid.z + normal.z * side,
                    }) {
                        supported = true;
                    }
                }
                if !supported {
                    continue;
                }
                let band = 0.65;
                let polygon = [
                    Point {
                        x: a.x + normal.x * band,
                        z: a.z + normal.z * band,
                    },
                    Point {
                        x: b.x + normal.x * band,
                        z: b.z + normal.z * band,
                    },
                    Point {
                        x: b.x - normal.x * band,
                        z: b.z - normal.z * band,
                    },
                    Point {
                        x: a.x - normal.x * band,
                        z: a.z - normal.z * band,
                    },
                ];
                let mut low = polygon[0].x - polygon[0].z;
                let mut high = low;
                for p in polygon {
                    low = low.min(p.x - p.z);
                    high = high.max(p.x - p.z);
                }
                // Constant x-z lines are world anchored, including LOD phase.
                let first = (low / spacing).ceil() as i32;
                let last = (high / spacing).floor() as i32;
                for n in first..=last {
                    let phase = n as f32 * spacing;
                    let origin = Point {
                        x: mid.x,
                        z: mid.x - phase,
                    };
                    let start = Point {
                        x: origin.x - 4.0,
                        z: origin.z - 4.0,
                    };
                    let end = Point {
                        x: origin.x + 4.0,
                        z: origin.z + 4.0,
                    };
                    let Some((lo, hi)) = clip(&polygon, start, end, 0.0) else {
                        continue;
                    };
                    let wear = hash((n as u32) ^ hash(wi as u32 * 71 + piece as u32));
                    let cuts = floors.outside(mix(start, end, lo), mix(start, end, hi), wear);
                    lod.strokes.extend(cuts);
                }
            }
        }
        &lod.strokes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_scales_and_centres_the_design() {
        let layout = Layout::new(2560.0, 1440.0, 0.0, 0.0);
        assert_eq!(layout.scale, 2.0);
        assert_eq!(layout.x, 0.0);
        assert_eq!(layout.y, 0.0);
        let point = layout.at(10.0, 20.0);
        assert_eq!((point.x, point.z), (20.0, 40.0));
        assert!(Layout::lamp_on_map(true, true));
        assert!(!Layout::lamp_on_map(true, false));
    }

    #[test]
    fn clip_reports_the_inside_interval_of_a_square() {
        let square = [
            Point { x: 0.0, z: 0.0 },
            Point { x: 10.0, z: 0.0 },
            Point { x: 10.0, z: 10.0 },
            Point { x: 0.0, z: 10.0 },
        ];
        let (lo, hi) = clip(
            &square,
            Point { x: -5.0, z: 5.0 },
            Point { x: 15.0, z: 5.0 },
            0.0,
        )
        .unwrap();
        assert!((lo - 0.25).abs() < 1e-5);
        assert!((hi - 0.75).abs() < 1e-5);
        assert!(clip(
            &square,
            Point { x: -5.0, z: -5.0 },
            Point { x: -1.0, z: -1.0 },
            0.0
        )
        .is_none());
    }

    #[test]
    fn a_floor_index_finds_triangles_and_cuts_outside_them() {
        let mut floors = FloorIndex::default();
        floors.build(vec![Triangle {
            p: [
                Point { x: 0.0, z: 0.0 },
                Point { x: 10.0, z: 0.0 },
                Point { x: 0.0, z: 10.0 },
            ],
        }]);
        assert!(floors.contains(Point { x: 1.0, z: 1.0 }));
        assert!(!floors.contains(Point { x: 9.0, z: 9.0 }));
        let cuts = floors.outside(Point { x: 0.0, z: 5.0 }, Point { x: 10.0, z: 5.0 }, 7);
        assert!(!cuts.is_empty());
        assert!(cuts.iter().all(|line| line.wear == 7));
    }

    #[test]
    fn lod_spacing_and_level_are_clamped() {
        assert_eq!(CutCache::spacing(0), 0.125);
        assert!((CutCache::spacing(1) - 0.1875).abs() < 1e-6);
        assert_eq!(CutCache::spacing(-5), CutCache::spacing(0));
        assert_eq!(CutCache::level(1.0, 1.0), CutCache::level(1.0, 1.0));
        assert!(CutCache::level(1000.0, 1.0) < LOD_COUNT);
        assert_ne!(hash(1), hash(2));
    }
}
