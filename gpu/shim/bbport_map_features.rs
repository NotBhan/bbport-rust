//! Hunter's Atlas world features, ported from `legacy/gpu/shim/bbport_map_features.h` and
//! `bbport_map_features.cpp`: the JSON feature data, availability rules and pickups.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use serde_json::Value;

pub const GAME_EBOOT_SHA256: &str =
    "d65f0b4f01d59166aed16f8604196d8b7dd805abbf0758b356e8f1354c9429f9";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NativeDestination {
    pub arrival: [f32; 3],
    pub return_point: i32,
    pub unlock_flag: u32,
    pub on: Vec<u32>,
    pub off: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Destination {
    pub id: u32,
    pub area: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub available: bool,
    pub discovered: bool,
    pub native: NativeDestination,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pickup {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub category: String,
    pub label: String,
}

struct FeatureState {
    areas: Value,
    destinations: Vec<Destination>,
    flags: HashMap<u32, i32>,
    travel: bool,
    items: bool,
    fog: bool,
    valid: bool,
    npc_mode: String,
    categories: String,
    error: String,
    reported: bool,
    last_poll: u64,
    player: u64,
    epoch: Instant,
}

impl Default for FeatureState {
    fn default() -> Self {
        Self {
            areas: Value::Object(serde_json::Map::new()),
            destinations: Vec::new(),
            flags: HashMap::new(),
            travel: false,
            items: false,
            fog: false,
            valid: false,
            npc_mode: "all".into(),
            categories: "healing,insight,equipment,echoes".into(),
            error: String::new(),
            reported: false,
            last_poll: 0,
            player: 0,
            epoch: Instant::now(),
        }
    }
}

fn state() -> &'static Mutex<FeatureState> {
    static STATE: OnceLock<Mutex<FeatureState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(FeatureState::default()))
}

fn lock() -> MutexGuard<'static, FeatureState> {
    state().lock().unwrap_or_else(|error| error.into_inner())
}

/// `runtime_atlas_flag`: the guest's world-flag reads.
pub fn set_flag_provider(provider: fn(u32) -> i32) {
    if let Ok(mut slot) = flag_provider().lock() {
        *slot = provider;
    }
}

/// `runtime_atlas_player_token`: a token that changes when the player moves between areas.
pub fn set_player_token_provider(provider: fn() -> u64) {
    if let Ok(mut slot) = player_provider().lock() {
        *slot = provider;
    }
}

fn flag(state: &mut FeatureState, id: u32) -> i32 {
    if let Some(&value) = state.flags.get(&id) {
        return value;
    }
    let value = if state.valid {
        flag_provider()
            .lock()
            .map(|provider| provider(id))
            .unwrap_or(-1)
    } else {
        -1
    };
    state.flags.insert(id, value);
    value
}

type FlagProvider = fn(u32) -> i32;
type PlayerProvider = fn() -> u64;

fn flag_provider() -> &'static Mutex<FlagProvider> {
    static PROVIDER: OnceLock<Mutex<FlagProvider>> = OnceLock::new();
    PROVIDER.get_or_init(|| Mutex::new(|_| -1))
}

fn player_provider() -> &'static Mutex<PlayerProvider> {
    static PROVIDER: OnceLock<Mutex<PlayerProvider>> = OnceLock::new();
    PROVIDER.get_or_init(|| Mutex::new(|| 0))
}

fn env_is_one(key: &str) -> bool {
    std::env::var(key)
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}

fn finite(value: f32) -> Result<f32, String> {
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "Invalid destination coordinate".to_string())
}

fn load_features(directory: &Path) -> Result<(Value, Vec<Destination>), String> {
    let text = std::fs::read_to_string(directory.join("features.json"))
        .map_err(|error| error.to_string())?;
    let data: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if data.get("schema").and_then(Value::as_i64) != Some(1)
        || data.get("game_eboot_sha256").and_then(Value::as_str) != Some(GAME_EBOOT_SHA256)
    {
        return Err("Feature metadata does not match this game".into());
    }
    let areas = required(&data, "areas").cloned()?;
    let object = areas.as_object().ok_or("Invalid feature areas")?;
    if object.len() > 64 {
        return Err("Invalid feature areas".into());
    }
    let mut destinations: Vec<Destination> = Vec::new();
    for (area, entry) in object {
        let lamps = entry
            .get("lamps")
            .and_then(Value::as_array)
            .ok_or("Invalid lamps")?;
        for lamp in lamps {
            let id = lamp.get("id").and_then(Value::as_u64).unwrap_or(0);
            let name = lamp.get("name").and_then(Value::as_str).unwrap_or("");
            let pos = lamp
                .get("pos")
                .and_then(Value::as_array)
                .ok_or("Invalid destination metadata")?;
            if pos.len() != 3 || destinations.len() >= 128 {
                return Err("Invalid destination metadata".into());
            }
            let mut destination = Destination {
                id: id as u32,
                area: area.clone(),
                name: name.to_string(),
                ..Destination::default()
            };
            destination.x = finite(pos[0].as_f64().unwrap_or(0.0) as f32)?;
            destination.y = finite(pos[1].as_f64().unwrap_or(0.0) as f32)?;
            destination.z = finite(pos[2].as_f64().unwrap_or(0.0) as f32)?;
            let arrival = lamp
                .get("arrival_pos")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| pos.clone());
            for (index, value) in arrival.iter().take(3).enumerate() {
                destination.native.arrival[index] = value.as_f64().unwrap_or(0.0) as f32;
            }
            destination.native.return_point = lamp
                .get("return_point")
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32;
            destination.native.unlock_flag =
                lamp.get("unlock_flag").and_then(Value::as_u64).unwrap_or(0) as u32;
            let on = lamp
                .get("available_on")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let off = lamp
                .get("available_off")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if on.len() > 8 || off.len() > 8 || destination.native.return_point <= 0 {
                return Err("Invalid travel conditions".into());
            }
            destination.native.on = on
                .iter()
                .filter_map(Value::as_u64)
                .map(|v| v as u32)
                .collect();
            destination.native.off = off
                .iter()
                .filter_map(Value::as_u64)
                .map(|v| v as u32)
                .collect();
            destinations.push(destination);
        }
    }
    destinations.sort_by(|left, right| {
        left.area
            .cmp(&right.area)
            .then(left.native.return_point.cmp(&right.native.return_point))
    });
    Ok((areas, destinations))
}

pub fn init(directory: &Path) {
    let mut state = lock();
    state.destinations.clear();
    state.areas = Value::Object(serde_json::Map::new());
    state.flags.clear();
    state.error.clear();
    state.reported = false;
    state.travel = env_is_one("BB_MAP_TRAVEL_ENABLED");
    state.items = env_is_one("BB_MAP_ITEMS_ENABLED");
    state.fog = env_is_one("BB_MAP_FOG_ENABLED");
    state.npc_mode = std::env::var("BB_MAP_NPC_MODE").unwrap_or_else(|_| "all".into());
    if !["all", "new_dialogue", "hidden"].contains(&state.npc_mode.as_str()) {
        state.npc_mode = "all".into();
    }
    state.categories = std::env::var("BB_MAP_ITEM_CATEGORIES")
        .unwrap_or_else(|_| "healing,insight,equipment,echoes".into());
    match load_features(directory) {
        Ok((areas, destinations)) => {
            state.areas = areas;
            state.destinations = destinations;
        }
        Err(message) => {
            state.error = format!("Prepare Atlas maps for feature data: {message}");
            state.destinations.clear();
            state.areas = Value::Object(serde_json::Map::new());
        }
    }
}

pub fn refresh(location_valid: bool) {
    let token = player_provider()
        .lock()
        .map(|provider| provider())
        .unwrap_or(0);
    let now_ms = lock().epoch.elapsed().as_millis() as u64;
    refresh_with(location_valid, token, now_ms);
}

/// `Refresh` with an explicit player token and clock (the runtime supplies both).
pub fn refresh_with(location_valid: bool, token: u64, now_ms: u64) {
    let mut state = lock();
    if token == state.player
        && location_valid == state.valid
        && now_ms.saturating_sub(state.last_poll) < 200
    {
        return;
    }
    state.player = token;
    state.valid = location_valid && token != 0;
    state.last_poll = now_ms;
    state.flags.clear();
    let mut destinations = std::mem::take(&mut state.destinations);
    for destination in &mut destinations {
        destination.discovered = state.valid
            && (destination.native.unlock_flag == 0
                || flag(&mut state, destination.native.unlock_flag) == 1);
        destination.available = destination.discovered;
        let on = destination.native.on.clone();
        for id in on {
            destination.available = destination.available && flag(&mut state, id) == 1;
        }
        let off = destination.native.off.clone();
        for id in off {
            destination.available = destination.available && flag(&mut state, id) == 0;
        }
        // Hunter's Dream has no lamp-unlock bit; its authored visit/availability conditions
        // provide the discovery state. Unknown reads remain hidden.
        if destination.native.unlock_flag == 0 {
            destination.discovered = destination.available;
        }
    }
    state.destinations = destinations;
}

pub fn destinations() -> Vec<Destination> {
    lock().destinations.clone()
}

pub fn available() -> Vec<i32> {
    lock()
        .destinations
        .iter()
        .enumerate()
        .filter(|(_, destination)| destination.available)
        .map(|(index, _)| index as i32)
        .collect()
}

pub fn travel_enabled() -> bool {
    lock().travel
}

pub fn items_enabled() -> bool {
    lock().items
}

pub fn item_category_enabled(category: &str) -> bool {
    let state = lock();
    state.items && format!(",{},", state.categories).contains(&format!(",{category},"))
}

pub fn fog_enabled() -> bool {
    lock().fog
}

pub fn error() -> String {
    lock().error.clone()
}

fn requirements(state: &mut FeatureState, rule: &Value) -> bool {
    for id in rule
        .get("available_on")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let Some(id) = id.as_u64() else {
            return false;
        };
        if flag(state, id as u32) != 1 {
            return false;
        }
    }
    for id in rule
        .get("available_off")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let Some(id) = id.as_u64() else {
            return false;
        };
        if flag(state, id as u32) != 0 {
            return false;
        }
    }
    true
}

fn evaluate(state: &mut FeatureState, expression: &Value, depth: i32) -> i32 {
    if depth > 32 {
        return -1;
    }
    if let Some(value) = expression.as_bool() {
        return i32::from(value);
    }
    if let Some(value) = expression.as_i64() {
        return value as i32;
    }
    let Some(array) = expression.as_array() else {
        return -1;
    };
    if array.is_empty() {
        return -1;
    }
    let Some(op) = array[0].as_str() else {
        return -1;
    };
    if op == "flag" && array.len() == 2 {
        if let Some(id) = array[1].as_u64() {
            return flag(state, id as u32);
        }
    }
    if array.len() != 3 {
        return -1;
    }
    let left = evaluate(state, &array[1], depth + 1);
    let right = evaluate(state, &array[2], depth + 1);
    match op {
        "and" => {
            if left == 0 || right == 0 {
                0
            } else if left < 0 || right < 0 {
                -1
            } else {
                1
            }
        }
        "or" => {
            if left > 0 || right > 0 {
                1
            } else if left < 0 || right < 0 {
                -1
            } else {
                0
            }
        }
        _ => {
            if left < 0 || right < 0 {
                return -1;
            }
            match op {
                "==" => i32::from(left == right),
                "!=" => i32::from(left != right),
                _ => -1,
            }
        }
    }
}

fn number(value: &Value) -> f32 {
    value.as_f64().unwrap_or(0.0) as f32
}

fn inside(rule: &Value, x: f32, y: f32, z: f32) -> bool {
    if y < rule.get("min_y").map(number).unwrap_or(-1e9)
        || y > rule.get("max_y").map(number).unwrap_or(1e9)
    {
        return false;
    }
    if let Some(bounds) = rule.get("bounds") {
        let Some(bounds) = bounds.as_array() else {
            return false;
        };
        if bounds.len() != 4
            || x < number(&bounds[0])
            || z < number(&bounds[1])
            || x > number(&bounds[2])
            || z > number(&bounds[3])
        {
            return false;
        }
    }
    if let Some(centre) = rule.get("centre") {
        let Some(centre) = centre.as_array() else {
            return false;
        };
        if centre.len() != 3 {
            return false;
        }
        let dx = x - number(&centre[0]);
        let dz = z - number(&centre[2]);
        let radius = rule.get("radius").map(number).unwrap_or(0.0);
        if dx * dx + dz * dz > radius * radius {
            return false;
        }
    }
    if let Some(triangles) = rule.get("triangles") {
        let mut hit = false;
        if let Some(triangles) = triangles.as_array() {
            for triangle in triangles {
                let Some(triangle) = triangle.as_array() else {
                    continue;
                };
                if triangle.len() != 6 {
                    continue;
                }
                let cross = |a: usize, b: usize| {
                    (number(&triangle[b]) - number(&triangle[a])) * (z - number(&triangle[a + 1]))
                        - (number(&triangle[b + 1]) - number(&triangle[a + 1]))
                            * (x - number(&triangle[a]))
                };
                let first = cross(0, 2);
                let second = cross(2, 4);
                let third = cross(4, 0);
                if (first >= 0.0 && second >= 0.0 && third >= 0.0)
                    || (first <= 0.0 && second <= 0.0 && third <= 0.0)
                {
                    hit = true;
                    break;
                }
            }
        }
        if !hit {
            return false;
        }
    }
    true
}

fn fail(state: &mut FeatureState, where_: &str, message: String) {
    state.error = format!("Map feature data ({where_}): {message}");
    if !state.reported {
        state.reported = true;
        eprintln!("Map features: {}", state.error);
    }
}

fn revealed_in(state: &mut FeatureState, area: &str, x: f32, y: f32, z: f32) -> bool {
    if !state.fog {
        return true;
    }
    let Some(entry) = state.areas.get(area).cloned() else {
        return false;
    };
    if entry
        .get("always_visible")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    let rules = entry
        .get("reveal_regions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for rule in rules {
        if inside(&rule, x, y, z) {
            if !requirements(state, &rule) {
                return false;
            }
            if let Some(reveal_when) = rule.get("reveal_when") {
                if evaluate(state, reveal_when, 0) != 1 {
                    return false;
                }
            }
            for id in rule
                .get("boss_flags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                if flag(state, id.as_u64().unwrap_or(0) as u32) != 1 {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

pub fn revealed(area: &str, x: f32, y: f32, z: f32) -> bool {
    let mut state = lock();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        revealed_in(&mut state, area, x, y, z)
    })) {
        Ok(value) => value,
        Err(_) => {
            // Hand-authored features.json can be structurally valid but contain a wrong type
            // deep in a rule; fail closed and stop decorating.
            fail(&mut state, "reveal", "invalid rule".into());
            state.fog = false;
            true
        }
    }
}

pub fn npc_visible(area: &str, entity: i32) -> bool {
    let mut state = lock();
    if state.npc_mode == "hidden" {
        return false;
    }
    if state.npc_mode == "all" {
        return true;
    }
    let Some(entry) = state.areas.get(area).cloned() else {
        return false;
    };
    let Some(rules) = entry.get("npc_rules") else {
        return false;
    };
    let Some(rule) = rules.get(entity.to_string()) else {
        return false;
    };
    let rule = rule.clone();
    match evaluate(&mut state, &rule, 0) {
        1 => true,
        0 => false,
        _ => {
            fail(&mut state, "npc", "invalid rule".into());
            state.npc_mode = "all".into();
            true
        }
    }
}

pub fn pickups(area: &str) -> Vec<Pickup> {
    let mut state = lock();
    if !state.items || !state.valid {
        return Vec::new();
    }
    let Some(entry) = state.areas.get(area).cloned() else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for pickup in entry
        .get("pickups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        if !requirements(&mut state, &pickup) {
            continue;
        }
        let Some(pos) = pickup.get("pos").and_then(Value::as_array) else {
            continue;
        };
        if pos.len() != 3 {
            continue;
        }
        let (x, y, z) = (number(&pos[0]), number(&pos[1]), number(&pos[2]));
        if !revealed_in(&mut state, area, x, y, z) {
            continue;
        }
        let mut out = Pickup {
            x,
            y,
            z,
            ..Pickup::default()
        };
        for item in pickup
            .get("contents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let category = item.get("category").and_then(Value::as_str).unwrap_or("");
            let enabled =
                state.items && format!(",{},", state.categories).contains(&format!(",{category},"));
            if !enabled {
                continue;
            }
            let collected = item
                .get("collected_flag")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if flag(&mut state, collected as u32) != 0 {
                continue;
            }
            if out.category.is_empty() {
                out.category = category.to_string();
            }
            if !out.label.is_empty() {
                out.label.push('\n');
            }
            let name = item.get("name").and_then(Value::as_str).unwrap_or("");
            let quantity = item.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            out.label.push_str(&format!("{name} x{quantity}"));
        }
        if !out.category.is_empty() {
            result.push(out);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shim keeps its state in process-wide statics, so its two tests take turns rather
    /// than one reading what the other just wrote.
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_fixture() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "bbport_features_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let data = serde_json::json!({
            "schema": 1,
            "game_eboot_sha256": GAME_EBOOT_SHA256,
            "areas": {
                "central_yharnam": {
                    "lamps": [{
                        "id": 1,
                        "name": "Central Yharnam",
                        "pos": [1.0, 2.0, 3.0],
                        "return_point": 1000,
                        "unlock_flag": 100,
                        "available_on": [101]
                    }],
                    "pickups": [{
                        "pos": [4.0, 5.0, 6.0],
                        "contents": [{
                            "category": "healing",
                            "name": "Blood Vial",
                            "quantity": 2,
                            "collected_flag": 200
                        }]
                    }]
                }
            }
        });
        std::fs::write(
            directory.join("features.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        directory
    }

    #[test]
    fn lamps_and_pickups_come_from_the_feature_data() {
        let _guard = test_lock();
        let directory = write_fixture();
        set_flag_provider(|id| match id {
            100 | 101 | 200 => 1,
            _ => -1,
        });
        init(&directory);
        assert!(error().is_empty(), "{}", error());
        assert_eq!(destinations().len(), 1);
        std::env::set_var("BB_MAP_ITEMS_ENABLED", "1");
        init(&directory);
        refresh_with(true, 7, 1000);
        let destinations = destinations();
        assert!(destinations[0].discovered);
        assert!(destinations[0].available);
        let pickups = pickups("central_yharnam");
        // The only item is a collected Blood Vial, so the pickup is filtered out.
        assert!(pickups.is_empty());
        std::env::remove_var("BB_MAP_ITEMS_ENABLED");
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_wrong_eboot_hash_is_rejected() {
        let _guard = test_lock();
        let directory =
            std::env::temp_dir().join(format!("bbport_features_bad_{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("features.json"),
            r#"{"schema":1,"game_eboot_sha256":"nope","areas":{}}"#,
        )
        .unwrap();
        init(&directory);
        assert!(error().contains("Prepare Atlas maps"));
        assert!(destinations().is_empty());
        std::fs::remove_dir_all(&directory).ok();
    }
}
