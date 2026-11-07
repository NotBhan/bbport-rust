//! Hunter's Atlas map artwork, ported from `legacy/gpu/shim/bbport_map_art.h` and
//! `bbport_map_art.cpp`. The ImGui draw calls are not linked in the Rust port yet, so the
//! geometry and the BBRA texture loading are implemented and the draw entry points keep the
//! same surface.

use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::gpu::shim::bbport_atlas_paths::atlas_path;

pub type ImDrawList = c_void;
pub type ImU32 = u32;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImVec2 {
    pub x: f32,
    pub y: f32,
}

pub const IM_COL32_WHITE: ImU32 = 0xffff_ffff;

const MAX_DIMENSION: u32 = 4096;
const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;
const ICON_CELL: i32 = 384;
const ICON_ATLAS_SIZE: u32 = 1536;
const FIRST_TEXTURE_ID: u32 = 1_000_000;

const ICON_NAMES: [&str; 12] = [
    "lamp", "ladder", "talk", "gate", "player", "compass", "X", "Y", "B", "LS", "LB", "RB",
];

#[derive(Debug, Clone, Default)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub id: u32,
}

#[derive(Default)]
struct ArtState {
    background: Option<Texture>,
    icons: Option<Texture>,
    parchment: Option<Texture>,
    menu_icon: Option<Texture>,
    ready: bool,
    next_texture_id: u32,
}

fn state() -> &'static Mutex<ArtState> {
    static STATE: OnceLock<Mutex<ArtState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(ArtState {
            next_texture_id: FIRST_TEXTURE_ID,
            ..ArtState::default()
        })
    })
}

fn lock() -> MutexGuard<'static, ArtState> {
    state().lock().unwrap_or_else(|error| error.into_inner())
}

fn read_le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Parses a `.bbra` asset: `BBRA`, width, height, then RGBA32 pixels.
pub fn decode_bbra(bytes: &[u8]) -> Option<Texture> {
    if bytes.len() < 12 || &bytes[0..4] != b"BBRA" {
        return None;
    }
    let width = read_le32(&bytes[4..8]);
    let height = read_le32(&bytes[8..12]);
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return None;
    }
    let pixel_bytes = u64::from(width) * u64::from(height) * 4;
    if pixel_bytes > MAX_ASSET_BYTES || bytes.len() as u64 != pixel_bytes + 12 {
        return None;
    }
    Some(Texture {
        width,
        height,
        pixels: bytes[12..].to_vec(),
        id: 0,
    })
}

pub fn load_texture(path: &Path) -> Option<Texture> {
    let metadata = std::fs::metadata(path).ok()?;
    let file_size = metadata.len();
    if !(12..=MAX_ASSET_BYTES + 12).contains(&file_size) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let mut texture = decode_bbra(&bytes)?;
    let mut state = lock();
    texture.id = state.next_texture_id;
    state.next_texture_id += 1;
    Some(texture)
}

/// `ArtDirectory`: `BB_MAP_ART_DIR`, or `<BB_MAP_DIR parent>/art`.
pub fn art_directory() -> PathBuf {
    let maps = atlas_path("BB_MAP_DIR", "navigation-map/maps");
    let fallback = maps.parent().unwrap_or_else(|| Path::new(".")).join("art");
    atlas_path("BB_MAP_ART_DIR", fallback)
}

/// `Init`: loads background/icons/parchment and requires a 1536x1536 icon atlas.
pub fn init() -> bool {
    init_in(&art_directory())
}

pub fn init_in(directory: &Path) -> bool {
    {
        let state = lock();
        if state.ready {
            return true;
        }
    }
    let background = load_texture(&directory.join("background.bbra"));
    let icons = load_texture(&directory.join("icons.bbra"));
    let parchment = load_texture(&directory.join("parchment.bbra"));
    let (Some(background), Some(icons), Some(parchment)) = (background, icons, parchment) else {
        shutdown();
        return false;
    };
    if icons.width != ICON_ATLAS_SIZE || icons.height != ICON_ATLAS_SIZE {
        shutdown();
        return false;
    }
    {
        let mut state = lock();
        state.background = Some(background);
        state.icons = Some(icons);
        state.parchment = Some(parchment);
        state.ready = true;
    }
    let menu_icon = load_texture(&directory.join("menu.bbra"));
    if let Some(menu_icon) = menu_icon {
        lock().menu_icon = Some(menu_icon);
    }
    true
}

pub fn shutdown() {
    let mut state = lock();
    state.background = None;
    state.icons = None;
    state.parchment = None;
    state.menu_icon = None;
    state.ready = false;
}

pub fn is_ready() -> bool {
    lock().ready
}
pub fn icon_index(name: &str) -> i32 {
    ICON_NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .map(|index| index as i32)
        .unwrap_or(-1)
}

/// The icon cell's UV rectangle, inset so linear sampling stays inside the cell.
pub fn icon_uv(index: i32, atlas_width: u32, atlas_height: u32) -> Option<(ImVec2, ImVec2)> {
    if !(0..ICON_NAMES.len() as i32).contains(&index) {
        return None;
    }
    let col = index % 4;
    let row = index / 4;
    let inset = 3.0f32;
    let width = atlas_width as f32;
    let height = atlas_height as f32;
    Some((
        ImVec2 {
            x: ((col * ICON_CELL) as f32 + inset) / width,
            y: ((row * ICON_CELL) as f32 + inset) / height,
        },
        ImVec2 {
            x: (((col + 1) * ICON_CELL) as f32 - inset) / width,
            y: (((row + 1) * ICON_CELL) as f32 - inset) / height,
        },
    ))
}

fn rotate(point: ImVec2, center: ImVec2, cos: f32, sin: f32) -> ImVec2 {
    let x = point.x - center.x;
    let y = point.y - center.y;
    ImVec2 {
        x: center.x + x * cos - y * sin,
        y: center.y + x * sin + y * cos,
    }
}

/// The four rotated corners of an icon quad.
pub fn icon_corners(center: ImVec2, size: f32, rotation_radians: f32) -> [ImVec2; 4] {
    let half = size * 0.5;
    let cos = rotation_radians.cos();
    let sin = rotation_radians.sin();
    [
        rotate(
            ImVec2 {
                x: center.x - half,
                y: center.y - half,
            },
            center,
            cos,
            sin,
        ),
        rotate(
            ImVec2 {
                x: center.x + half,
                y: center.y - half,
            },
            center,
            cos,
            sin,
        ),
        rotate(
            ImVec2 {
                x: center.x + half,
                y: center.y + half,
            },
            center,
            cos,
            sin,
        ),
        rotate(
            ImVec2 {
                x: center.x - half,
                y: center.y + half,
            },
            center,
            cos,
            sin,
        ),
    ]
}

/// One trapezoid of the background fade: its four screen positions, UVs and corner alphas.
#[derive(Debug, Clone, Copy)]
pub struct FadeQuad {
    pub positions: [ImVec2; 4],
    pub uv: [ImVec2; 4],
    pub alpha: [u8; 4],
}

/// `DrawBackgroundFade`: four nonoverlapping trapezoids around the map's rectangle.
pub fn fade_quads(
    background_min: ImVec2,
    background_max: ImVec2,
    map_min: ImVec2,
    map_max: ImVec2,
    fade_width: f32,
) -> Vec<FadeQuad> {
    if background_max.x <= background_min.x
        || background_max.y <= background_min.y
        || map_max.x <= map_min.x
        || map_max.y <= map_min.y
        || !fade_width.is_finite()
        || fade_width <= 0.0
    {
        return Vec::new();
    }
    let width = fade_width
        .min((map_max.x - map_min.x) * 0.5)
        .min((map_max.y - map_min.y) * 0.5);
    let outer = [
        map_min,
        ImVec2 {
            x: map_max.x,
            y: map_min.y,
        },
        map_max,
        ImVec2 {
            x: map_min.x,
            y: map_max.y,
        },
    ];
    let inner = [
        ImVec2 {
            x: map_min.x + width,
            y: map_min.y + width,
        },
        ImVec2 {
            x: map_max.x - width,
            y: map_min.y + width,
        },
        ImVec2 {
            x: map_max.x - width,
            y: map_max.y - width,
        },
        ImVec2 {
            x: map_min.x + width,
            y: map_max.y - width,
        },
    ];
    let mut quads = Vec::with_capacity(4);
    for side in 0..4 {
        let next = (side + 1) % 4;
        let positions = [outer[side], outer[next], inner[next], inner[side]];
        let uv = positions.map(|p| ImVec2 {
            x: (p.x - background_min.x) / (background_max.x - background_min.x),
            y: (p.y - background_min.y) / (background_max.y - background_min.y),
        });
        quads.push(FadeQuad {
            positions,
            uv,
            alpha: [255, 255, 0, 0],
        });
    }
    quads
}

/// The parchment UVs of a floor triangle, world-anchored and clamped.
pub fn floor_uvs(
    world_xz: [ImVec2; 3],
    world_tile_units: f32,
    world_origin: ImVec2,
) -> Option<[ImVec2; 3]> {
    if !world_tile_units.is_finite() || world_tile_units <= 0.0 {
        return None;
    }
    let reciprocal = 1.0 / world_tile_units;
    let mut uv = [ImVec2::default(); 3];
    for (index, point) in world_xz.iter().enumerate() {
        uv[index] = ImVec2 {
            x: (0.5 + (point.x - world_origin.x) * reciprocal).clamp(0.0, 1.0),
            y: (0.5 + (point.y - world_origin.y) * reciprocal).clamp(0.0, 1.0),
        };
    }
    Some(uv)
}

pub fn draw_background(_draw: *mut ImDrawList, _min: ImVec2, _max: ImVec2, _tint: ImU32) {}
pub fn draw_background_fade(
    _draw: *mut ImDrawList,
    _background_min: ImVec2,
    _background_max: ImVec2,
    _map_min: ImVec2,
    _map_max: ImVec2,
    _fade_width: f32,
) {
}
pub fn draw_icon(
    _draw: *mut ImDrawList,
    _name: *const c_char,
    _center: ImVec2,
    _size: f32,
    _rotation_radians: f32,
    _tint: ImU32,
) {
}
pub fn draw_menu_icon(_draw: *mut ImDrawList, _center: ImVec2, _size: f32, _tint: ImU32) {}
pub fn draw_floor_triangle(
    _draw: *mut ImDrawList,
    _screen: [ImVec2; 3],
    _world_xz: [ImVec2; 3],
    _tint: [ImU32; 3],
    _world_tile_units: f32,
    _world_origin: ImVec2,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bbra_header_and_pixels_are_decoded() {
        let mut bytes = Vec::from(*b"BBRA");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        let texture = decode_bbra(&bytes).unwrap();
        assert_eq!((texture.width, texture.height), (2, 2));
        assert_eq!(texture.pixels.len(), 16);
        assert!(decode_bbra(b"NOPE").is_none());
        let mut truncated = bytes.clone();
        truncated.truncate(20);
        assert!(decode_bbra(&truncated).is_none());
    }

    #[test]
    fn icons_are_indexed_and_uvs_inset() {
        assert_eq!(icon_index("lamp"), 0);
        assert_eq!(icon_index("RB"), 11);
        assert_eq!(icon_index("nope"), -1);
        let (uv0, uv1) = icon_uv(11, 1536, 1536).unwrap();
        assert!(uv0.x > (2.0 * 384.0) / 1536.0);
        assert!(uv1.y < 1.0);
        assert!(icon_uv(-1, 1536, 1536).is_none());
    }

    #[test]
    fn fade_quads_cover_the_map_edge_and_reject_degenerate_input() {
        let quads = fade_quads(
            ImVec2 { x: 0.0, y: 0.0 },
            ImVec2 { x: 100.0, y: 100.0 },
            ImVec2 { x: 10.0, y: 10.0 },
            ImVec2 { x: 90.0, y: 90.0 },
            8.0,
        );
        assert_eq!(quads.len(), 4);
        assert_eq!(quads[0].alpha, [255, 255, 0, 0]);
        assert!(fade_quads(
            ImVec2 { x: 0.0, y: 0.0 },
            ImVec2 { x: 100.0, y: 100.0 },
            ImVec2 { x: 90.0, y: 90.0 },
            ImVec2 { x: 10.0, y: 10.0 },
            8.0
        )
        .is_empty());
    }

    #[test]
    fn floor_uvs_are_world_anchored_and_clamped() {
        let uv = floor_uvs(
            [
                ImVec2 { x: 0.0, y: 0.0 },
                ImVec2 { x: 1000.0, y: 0.0 },
                ImVec2 { x: 0.0, y: 1000.0 },
            ],
            512.0,
            ImVec2 { x: 0.0, y: 0.0 },
        )
        .unwrap();
        assert!((uv[0].x - 0.5).abs() < 1e-6);
        assert_eq!(uv[1].x, 1.0);
        assert!(floor_uvs([ImVec2::default(); 3], 0.0, ImVec2::default()).is_none());
    }
}
