//! Enclosed gaps in the verified floor union, ported from
//! `legacy/gpu/shim/bbport_atlas_interiors.h`. Ring winding is immaterial; open, crossing or
//! self-touching topology fails closed (no decoration).

use crate::gpu::shim::bbport_atlas_style::{
    self, hash, mix, CutCache, FloorIndex, Line, Point, Triangle, LOD_COUNT,
};

pub type Ring = Vec<Point>;

#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
}

pub const MAX_FRAGMENTS: usize = 4096;
pub const MAX_LAYER_TRIANGLES: usize = 60000;

pub fn bbox(ring: &[Point]) -> Bounds {
    let Some(first) = ring.first() else {
        return Bounds::default();
    };
    let mut bounds = Bounds {
        x0: first.x,
        z0: first.z,
        x1: first.x,
        z1: first.z,
    };
    for p in ring {
        bounds.x0 = bounds.x0.min(p.x);
        bounds.z0 = bounds.z0.min(p.z);
        bounds.x1 = bounds.x1.max(p.x);
        bounds.z1 = bounds.z1.max(p.z);
    }
    bounds
}

pub fn triangle_bounds(t: &Triangle) -> Bounds {
    bbox(&t.p)
}

pub fn overlap(a: Bounds, b: Bounds) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.z0 <= b.z1 && b.z0 <= a.z1
}

pub fn turn(a: Point, b: Point, c: Point) -> f64 {
    (b.x as f64 - a.x as f64) * (c.z as f64 - a.z as f64)
        - (b.z as f64 - a.z as f64) * (c.x as f64 - a.x as f64)
}

pub fn same(a: Point, b: Point) -> bool {
    a.x == b.x && a.z == b.z
}

pub fn area(r: &[Point]) -> f64 {
    let mut value = 0.0;
    for i in 1..r.len().saturating_sub(1) {
        value += turn(r[0], r[i], r[i + 1]);
    }
    value * 0.5
}

pub fn on(a: Point, b: Point, p: Point) -> bool {
    turn(a, b, p).abs() < 1e-8
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.z >= a.z.min(b.z)
        && p.z <= a.z.max(b.z)
}

pub fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    if a.x.max(b.x) < c.x.min(d.x)
        || c.x.max(d.x) < a.x.min(b.x)
        || a.z.max(b.z) < c.z.min(d.z)
        || c.z.max(d.z) < a.z.min(b.z)
    {
        return false;
    }
    let u = turn(a, b, c);
    let v = turn(a, b, d);
    let s = turn(c, d, a);
    let t = turn(c, d, b);
    (((u > 0.0 && v < 0.0) || (u < 0.0 && v > 0.0))
        && ((s > 0.0 && t < 0.0) || (s < 0.0 && t > 0.0)))
        || on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
}

pub fn inside(r: &[Point], p: Point) -> bool {
    let mut result = false;
    let mut j = r.len() - 1;
    for i in 0..r.len() {
        let a = r[i];
        let b = r[j];
        if (a.z > p.z) != (b.z > p.z)
            && (p.x as f64)
                < (b.x as f64 - a.x as f64) * (p.z as f64 - a.z as f64) / (b.z as f64 - a.z as f64)
                    + a.x as f64
        {
            result = !result;
        }
        j = i;
    }
    result
}

/// Validates and cleans a closed ring; returns false when the topology is uncertain.
pub fn prepare(r: &mut Ring) -> bool {
    if r.len() < 4 || r.len() > 4096 || !same(r[0], *r.last().unwrap()) {
        return false;
    }
    r.pop();
    if r.iter().any(|p| !p.x.is_finite() || !p.z.is_finite()) {
        return false;
    }
    let mut i = 0;
    while r.len() >= 3 && i < r.len() {
        let prev = (i + r.len() - 1) % r.len();
        let next = (i + 1) % r.len();
        if same(r[i], r[next]) || on(r[prev], r[next], r[i]) {
            r.remove(i);
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    if r.len() < 3 || area(r).abs() < 1e-5 {
        return false;
    }
    for i in 0..r.len() {
        for j in i + 1..r.len() {
            if j == i + 1 || (i == 0 && j + 1 == r.len()) {
                continue;
            }
            if crosses(r[i], r[(i + 1) % r.len()], r[j], r[(j + 1) % r.len()]) {
                return false;
            }
        }
    }
    true
}

/// Ear clipping; returns `None` when the ring is not simple.
pub fn triangulate(r: &mut Ring) -> Option<Vec<Triangle>> {
    if area(r) < 0.0 {
        r.reverse();
    }
    let mut result = Vec::new();
    while r.len() > 3 {
        let mut found = false;
        for i in 0..r.len() {
            let prev = (i + r.len() - 1) % r.len();
            let next = (i + 1) % r.len();
            let (a, b, c) = (r[prev], r[i], r[next]);
            if turn(a, b, c) <= 1e-9 {
                continue;
            }
            let mut occupied = false;
            for (k, point) in r.iter().enumerate() {
                if k != prev
                    && k != i
                    && k != next
                    && turn(a, b, *point) >= -1e-9
                    && turn(b, c, *point) >= -1e-9
                    && turn(c, a, *point) >= -1e-9
                {
                    occupied = true;
                    break;
                }
            }
            if occupied {
                continue;
            }
            result.push(Triangle { p: [a, b, c] });
            r.remove(i);
            found = true;
            break;
        }
        if !found {
            return None;
        }
    }
    if r.len() == 3 {
        result.push(Triangle {
            p: [r[0], r[1], r[2]],
        });
    }
    Some(result)
}

/// Half-plane clipping in double arithmetic. A 2 cm clearance makes the float output
/// conservative even at the large world coordinates in the shipped maps.
pub fn half(polygon: &[Point], a: Point, b: Point, inside_half: bool, clearance: f64) -> Ring {
    let mut out = Ring::new();
    if polygon.is_empty() {
        return out;
    }
    let length = (b.x as f64 - a.x as f64).hypot(b.z as f64 - a.z as f64);
    let side = |p: Point| {
        let value = turn(a, b, p) + clearance * length;
        if inside_half {
            value
        } else {
            -value
        }
    };
    let mut prev = *polygon.last().unwrap();
    let mut prev_value = side(prev);
    for cur in polygon {
        let current_value = side(*cur);
        if (prev_value >= 0.0) != (current_value >= 0.0) {
            let t = prev_value / (prev_value - current_value);
            out.push(Point {
                x: (prev.x as f64 + (cur.x as f64 - prev.x as f64) * t) as f32,
                z: (prev.z as f64 + (cur.z as f64 - prev.z as f64) * t) as f32,
            });
        }
        if current_value >= 0.0 {
            out.push(*cur);
        }
        prev = *cur;
        prev_value = current_value;
    }
    out
}

pub fn subtract(polygon: &[Point], floor: &Triangle) -> Vec<Ring> {
    let mut floor = *floor;
    if turn(floor.p[0], floor.p[1], floor.p[2]) < 0.0 {
        floor.p.swap(1, 2);
    }
    if turn(floor.p[0], floor.p[1], floor.p[2]).abs() < 1e-10 {
        return vec![polygon.to_vec()];
    }
    let mut remainder = polygon.to_vec();
    let mut out = Vec::new();
    for i in 0..3 {
        if remainder.is_empty() {
            break;
        }
        let a = floor.p[i];
        let b = floor.p[(i + 1) % 3];
        let fragment = half(&remainder, a, b, false, 0.02);
        if fragment.len() >= 3 && area(&fragment).abs() > 1e-7 {
            out.push(fragment);
        }
        remainder = half(&remainder, a, b, true, 0.02);
    }
    out
}

#[derive(Default, Clone)]
pub struct Hatch {
    pub ready: bool,
    pub lines: Vec<Line>,
}

/// The interior cache: validated rings, their triangles, and the per-LOD hatching.
#[derive(Default)]
pub struct Cache {
    pub ready: bool,
    pub holes: usize,
    pub rejected: usize,
    pub budget_skips: usize,
    pub layers: Vec<Vec<Triangle>>,
    pub hatches: Vec<Vec<Hatch>>,
}

impl Cache {
    pub fn build(&mut self, outlines: &[Vec<Ring>], layer_floors: &mut [FloorIndex]) {
        self.layers = vec![Vec::new(); outlines.len()];
        self.hatches = vec![vec![Hatch::default(); LOD_COUNT]; outlines.len()];
        self.holes = 0;
        self.rejected = 0;
        self.budget_skips = 0;
        for li in 0..outlines.len() {
            let mut rings = outlines[li].clone();
            let floors = &mut layer_floors[li];
            let mut valid = vec![false; rings.len()];
            let mut bounds = Vec::with_capacity(rings.len());
            for (i, ring) in rings.iter_mut().enumerate() {
                valid[i] = prepare(ring);
                bounds.push(if ring.is_empty() {
                    Bounds::default()
                } else {
                    bbox(ring)
                });
                if !valid[i] {
                    self.rejected += 1;
                }
            }
            // Rounding to millimetres can create touching edges. Do not guess the topology in
            // those cases; invalidate both affected rings.
            for i in 0..rings.len() {
                for j in i + 1..rings.len() {
                    if !overlap(bounds[i], bounds[j]) || !valid[i] || !valid[j] {
                        continue;
                    }
                    let mut crossing = false;
                    for u in 0..rings[i].len() {
                        for v in 0..rings[j].len() {
                            if crosses(
                                rings[i][u],
                                rings[i][(u + 1) % rings[i].len()],
                                rings[j][v],
                                rings[j][(v + 1) % rings[j].len()],
                            ) {
                                crossing = true;
                                break;
                            }
                        }
                        if crossing {
                            break;
                        }
                    }
                    if crossing {
                        valid[i] = false;
                        valid[j] = false;
                        self.rejected += 2;
                    }
                }
            }
            for i in 0..rings.len() {
                if !valid[i] {
                    continue;
                }
                let mut depth = 0usize;
                let mut uncertain = false;
                for j in 0..rings.len() {
                    if i == j || !overlap(bounds[i], bounds[j]) {
                        continue;
                    }
                    if !valid[j] {
                        uncertain = true;
                        break;
                    }
                    if inside(&rings[j], rings[i][0]) {
                        depth += 1;
                    }
                }
                if uncertain || depth.is_multiple_of(2) {
                    continue;
                }
                let Some(seeds) = triangulate(&mut rings[i]) else {
                    self.rejected += 1;
                    continue;
                };
                self.holes += 1;
                let mut filled: Vec<Triangle> = Vec::new();
                let mut exhausted = false;
                for seed in &seeds {
                    let mut fragments: Vec<Ring> = vec![seed.p.to_vec()];
                    let b = bbox(&fragments[0]);
                    let query_bounds = Bounds {
                        x0: b.x0 - 0.03,
                        z0: b.z0 - 0.03,
                        x1: b.x1 + 0.03,
                        z1: b.z1 + 0.03,
                    };
                    let mut operations = 0usize;
                    floors.query(
                        Point {
                            x: query_bounds.x0,
                            z: query_bounds.z0,
                        },
                        Point {
                            x: query_bounds.x1,
                            z: query_bounds.z1,
                        },
                        |floor| {
                            if exhausted || fragments.is_empty() {
                                return;
                            }
                            if !overlap(query_bounds, triangle_bounds(floor)) {
                                return;
                            }
                            let mut next: Vec<Ring> = Vec::new();
                            for polygon in &fragments {
                                operations += 1;
                                if operations > 100_000 {
                                    exhausted = true;
                                    return;
                                }
                                next.extend(subtract(polygon, floor));
                                if next.len() > MAX_FRAGMENTS {
                                    exhausted = true;
                                    return;
                                }
                            }
                            fragments = next;
                        },
                    );
                    if exhausted {
                        break;
                    }
                    for polygon in &fragments {
                        for k in 1..polygon.len().saturating_sub(1) {
                            if turn(polygon[0], polygon[k], polygon[k + 1]).abs() > 1e-6 {
                                filled.push(Triangle {
                                    p: [polygon[0], polygon[k], polygon[k + 1]],
                                });
                            }
                        }
                    }
                    if filled.len() + self.layers[li].len() > MAX_LAYER_TRIANGLES {
                        exhausted = true;
                        break;
                    }
                }
                if exhausted {
                    self.budget_skips += 1;
                    continue;
                }
                self.layers[li].extend(filled);
            }
        }
        self.ready = true;
    }

    pub fn get(&mut self, layer: usize, level: i32) -> &[Line] {
        let index = level.clamp(0, LOD_COUNT as i32 - 1) as usize;
        let Cache {
            layers, hatches, ..
        } = self;
        let hatch = &mut hatches[layer][index];
        if hatch.ready {
            return &hatch.lines;
        }
        hatch.ready = true;
        let spacing = CutCache::spacing(index as i32) * 1.5;
        for triangle in &layers[layer] {
            let mut low = triangle.p[0].x - triangle.p[0].z;
            let mut high = low;
            for p in triangle.p {
                low = low.min(p.x - p.z);
                high = high.max(p.x - p.z);
            }
            let b = triangle_bounds(triangle);
            let first = (low / spacing).ceil() as i32;
            let last = (high / spacing).floor() as i32;
            for n in first..=last {
                let phase = n as f32 * spacing;
                let a = Point {
                    x: b.x0 - 1.0,
                    z: b.x0 - 1.0 - phase,
                };
                let end = Point {
                    x: b.x1 + 1.0,
                    z: b.x1 + 1.0 - phase,
                };
                if let Some((lo, hi)) = bbport_atlas_style::clip(&triangle.p, a, end, 0.0) {
                    hatch.lines.push(Line {
                        a: mix(a, end, lo),
                        b: mix(a, end, hi),
                        wear: hash(n as u32),
                    });
                }
                if hatch.lines.len() > 150_000 {
                    hatch.lines.clear();
                    return &hatch.lines;
                }
            }
        }
        &hatch.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f32) -> Ring {
        vec![
            Point { x: 0.0, z: 0.0 },
            Point { x: size, z: 0.0 },
            Point { x: size, z: size },
            Point { x: 0.0, z: size },
            Point { x: 0.0, z: 0.0 },
        ]
    }

    #[test]
    fn prepare_rejects_open_rings_and_keeps_closed_ones() {
        let mut open = vec![
            Point { x: 0.0, z: 0.0 },
            Point { x: 1.0, z: 0.0 },
            Point { x: 1.0, z: 1.0 },
            Point { x: 0.0, z: 1.0 },
        ];
        assert!(!prepare(&mut open));
        let mut closed = square(10.0);
        assert!(prepare(&mut closed));
        assert_eq!(closed.len(), 4);
        assert!(!inside(&closed, Point { x: 20.0, z: 20.0 }));
        assert!(inside(&closed, Point { x: 5.0, z: 5.0 }));
    }

    #[test]
    fn a_square_triangulates_into_two_triangles() {
        let mut ring = square(10.0);
        ring.pop();
        let triangles = triangulate(&mut ring).unwrap();
        assert_eq!(triangles.len(), 2);
        let total: f64 = triangles
            .iter()
            .map(|t| turn(t.p[0], t.p[1], t.p[2]).abs() / 2.0)
            .sum();
        assert!((total - 100.0).abs() < 1e-3);
    }

    #[test]
    fn subtracting_a_floor_triangle_leaves_the_remaining_area() {
        let polygon = square(10.0);
        let floor = Triangle {
            p: [
                Point { x: 0.0, z: 0.0 },
                Point { x: 10.0, z: 0.0 },
                Point { x: 0.0, z: 10.0 },
            ],
        };
        let fragments = subtract(&polygon, &floor);
        let remaining: f64 = fragments.iter().map(|f| area(f).abs()).sum();
        assert!((remaining - 50.0).abs() < 1.0);
    }
}
