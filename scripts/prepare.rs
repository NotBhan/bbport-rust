//! Minimal SFO parser, ported from `scripts/prepare.py` (`sfo`) plus the launcher's
//! `game_info` / `save_title_id` helpers.

use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SfoError {
    #[error("bad SFO signature")]
    BadSignature,
    #[error("truncated SFO data")]
    Truncated,
    #[error("cannot read SFO: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SfoValue {
    U32(u32),
    Text(String),
}

impl SfoValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::U32(_) => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U32(value) => Some(*value),
            Self::Text(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInfo {
    pub title: String,
    pub version: String,
}

/// Parse a PS4 `param.sfo` blob: `\0PSF`, header, index table, key table and value table.
pub fn parse_sfo(data: &[u8]) -> Result<HashMap<String, SfoValue>, SfoError> {
    let u32_at = |offset: usize| -> Result<u32, SfoError> {
        let bytes = data.get(offset..offset + 4).ok_or(SfoError::Truncated)?;
        Ok(u32::from_le_bytes(bytes.try_into().expect("4 bytes")))
    };
    if data.get(0..4) != Some(b"\0PSF") {
        return Err(SfoError::BadSignature);
    }
    let keys_offset = u32_at(8)? as usize;
    let values_offset = u32_at(12)? as usize;
    let count = u32_at(16)? as usize;

    let mut result = HashMap::new();
    for index in 0..count {
        let entry = 20 + index * 16;
        let key_offset = u16::from_le_bytes(
            data.get(entry..entry + 2)
                .ok_or(SfoError::Truncated)?
                .try_into()
                .expect("2 bytes"),
        ) as usize;
        let format = u16::from_le_bytes(
            data.get(entry + 2..entry + 4)
                .ok_or(SfoError::Truncated)?
                .try_into()
                .expect("2 bytes"),
        );
        let size = u32_at(entry + 4)? as usize;
        let data_offset = u32_at(entry + 12)? as usize;

        let key_start = keys_offset
            .checked_add(key_offset)
            .ok_or(SfoError::Truncated)?;
        let key_end = data[key_start..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|length| key_start + length)
            .ok_or(SfoError::Truncated)?;
        let key = String::from_utf8_lossy(&data[key_start..key_end]).into_owned();

        let value_start = values_offset
            .checked_add(data_offset)
            .ok_or(SfoError::Truncated)?;
        let value = data
            .get(value_start..value_start + size)
            .ok_or(SfoError::Truncated)?;
        let parsed = if format == 0x0404 {
            let bytes: [u8; 4] = value.try_into().map_err(|_| SfoError::Truncated)?;
            SfoValue::U32(u32::from_le_bytes(bytes))
        } else {
            let end = value
                .iter()
                .rposition(|byte| *byte != 0)
                .map(|last| last + 1)
                .unwrap_or(0);
            SfoValue::Text(String::from_utf8_lossy(&value[..end]).into_owned())
        };
        result.insert(key, parsed);
    }
    Ok(result)
}

/// `(title, version)` from the game's `sce_sys/param.sfo`, or `None` when it cannot be read.
pub fn game_info(game_dir: &Path) -> Option<GameInfo> {
    let data = std::fs::read(game_dir.join("sce_sys").join("param.sfo")).ok()?;
    let values = parse_sfo(&data).ok()?;
    let title = text(&values, "TITLE")
        .or_else(|| text(&values, "TITLE_ID"))
        .unwrap_or_else(|| "Bloodborne".to_string());
    let version = text(&values, "APP_VER")
        .or_else(|| text(&values, "VERSION"))
        .unwrap_or_default();
    Some(GameInfo { title, version })
}

fn text(values: &HashMap<String, SfoValue>, key: &str) -> Option<String> {
    match values.get(key) {
        Some(SfoValue::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny two-entry SFO: one string entry and one u32 entry.
    fn synthetic_sfo() -> Vec<u8> {
        let entries: [(&str, u16, Vec<u8>); 2] = [
            ("TITLE", 0x0204, b"Bloodborne\0".to_vec()),
            ("APP_VER", 0x0404, 0x0109u32.to_le_bytes().to_vec()),
        ];
        let header = 20 + entries.len() * 16;
        let mut key_table = Vec::new();
        let mut value_table = Vec::new();
        let mut index = Vec::new();
        for (key, format, value) in &entries {
            let key_offset = key_table.len() as u16;
            key_table.extend_from_slice(key.as_bytes());
            key_table.push(0);
            let data_offset = value_table.len() as u32;
            value_table.extend_from_slice(value);
            index.extend_from_slice(&key_offset.to_le_bytes());
            index.extend_from_slice(&format.to_le_bytes());
            index.extend_from_slice(&(value.len() as u32).to_le_bytes());
            index.extend_from_slice(&0u32.to_le_bytes());
            index.extend_from_slice(&data_offset.to_le_bytes());
        }
        let keys_offset = header as u32;
        let values_offset = (header + key_table.len()) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"\0PSF");
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&keys_offset.to_le_bytes());
        out.extend_from_slice(&values_offset.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&key_table);
        out.extend_from_slice(&value_table);
        out
    }

    #[test]
    fn parses_text_and_integer_entries() {
        let values = parse_sfo(&synthetic_sfo()).unwrap();
        assert_eq!(
            values.get("TITLE"),
            Some(&SfoValue::Text("Bloodborne".into()))
        );
        assert_eq!(values.get("APP_VER"), Some(&SfoValue::U32(0x0109)));
    }

    #[test]
    fn rejects_a_bad_signature() {
        assert!(matches!(parse_sfo(b"nope"), Err(SfoError::BadSignature)));
    }

    #[test]
    fn rejects_truncated_index_entries() {
        let mut data = synthetic_sfo();
        data.truncate(24);
        assert!(matches!(parse_sfo(&data), Err(SfoError::Truncated)));
    }
}
