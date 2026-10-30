//! Patching of JSON settings blobs stored in the registry by Unity games.

use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct Setting {
    pub value: Value,
    /// The stored blob ended with NUL bytes; we write them back so the game
    /// sees exactly the shape it produced.
    pub trailing_nul: bool,
    /// Fallback sanitising was needed to parse the blob.
    pub sanitized: bool,
}

/// Parses a registry value. Strategy (same tiers as KeqingUnlock):
/// 1. as-is, with trailing NULs removed;
/// 2. with every NUL byte removed;
/// 3. with every control character removed.
pub fn parse_setting_bytes(raw: &[u8]) -> Result<Setting> {
    let end = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let trailing_nul = end != raw.len();

    if let Ok(value) = serde_json::from_slice::<Value>(&raw[..end]) {
        return Ok(Setting { value, trailing_nul, sanitized: false });
    }
    let no_nul: Vec<u8> = raw.iter().copied().filter(|&b| b != 0).collect();
    if let Ok(value) = serde_json::from_slice::<Value>(&no_nul) {
        return Ok(Setting { value, trailing_nul, sanitized: true });
    }
    let printable: Vec<u8> = raw
        .iter()
        .copied()
        .filter(|&b| b >= 32 || matches!(b, 9 | 10 | 13))
        .collect();
    if let Ok(value) = serde_json::from_slice::<Value>(&printable) {
        return Ok(Setting { value, trailing_nul, sanitized: true });
    }
    Err(Error::InvalidData("registry value is not valid JSON".into()))
}

/// Sets every listed top-level field that exists. Errors if none exist, so we
/// never write back a blob we did not actually change.
pub fn set_fields(value: &mut Value, fields: &[&str], fps: u32) -> Result<usize> {
    let obj = value
        .as_object_mut()
        .ok_or_else(|| Error::InvalidData("settings blob is not a JSON object".into()))?;
    let mut patched = 0;
    for field in fields {
        if let Some(slot) = obj.get_mut(*field) {
            *slot = Value::from(fps);
            patched += 1;
        }
    }
    if patched == 0 {
        return Err(Error::NotFound(format!(
            "none of the expected FPS fields ({}) exist in the settings blob",
            fields.join(", ")
        )));
    }
    Ok(patched)
}

pub fn encode_setting(value: &Value, trailing_nul: bool) -> Result<Vec<u8>> {
    let mut out = serde_json::to_vec(value)?;
    if trailing_nul {
        out.push(0);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_nul_terminated() {
        let s = parse_setting_bytes(br#"{"FPS":60,"A":1}"#).unwrap();
        assert!(!s.trailing_nul && !s.sanitized);
        let s = parse_setting_bytes(b"{\"FPS\":60}\0").unwrap();
        assert!(s.trailing_nul && !s.sanitized);
    }

    #[test]
    fn falls_back_to_sanitising() {
        let s = parse_setting_bytes(b"{\"FPS\":\x0060}\0").unwrap();
        assert!(s.sanitized);
        assert_eq!(s.value["FPS"], 60);
        assert!(parse_setting_bytes(b"not json").is_err());
        assert!(parse_setting_bytes(b"").is_err());
        assert!(parse_setting_bytes(&[0, 0, 0]).is_err());
    }

    #[test]
    fn patches_only_existing_fields_and_round_trips() {
        let mut s = parse_setting_bytes(br#"{"B":1,"FPS":30,"A":"x"}"#).unwrap();
        assert_eq!(set_fields(&mut s.value, &["FPS"], 120).unwrap(), 1);
        let out = encode_setting(&s.value, s.trailing_nul).unwrap();
        // key order preserved
        assert_eq!(String::from_utf8(out).unwrap(), r#"{"B":1,"FPS":120,"A":"x"}"#);
    }

    #[test]
    fn nul_terminator_is_preserved() {
        let mut s = parse_setting_bytes(b"{\"FPS\":30}\0").unwrap();
        set_fields(&mut s.value, &["FPS"], 60).unwrap();
        assert_eq!(encode_setting(&s.value, s.trailing_nul).unwrap(), b"{\"FPS\":60}\0");
    }

    #[test]
    fn errors_when_no_field_present_or_not_object() {
        let mut v = serde_json::json!({"X": 1});
        assert!(set_fields(&mut v, &["FPS"], 60).is_err());
        let mut v = serde_json::json!([1, 2]);
        assert!(set_fields(&mut v, &["FPS"], 60).is_err());
    }

    #[test]
    fn patches_multiple_fields() {
        let mut v = serde_json::json!({"TargetFrameRateForInLevel": 60, "TargetFrameRateForOthers": 60});
        let n = set_fields(&mut v, &["TargetFrameRateForInLevel", "TargetFrameRateForOthers"], 144).unwrap();
        assert_eq!(n, 2);
        assert_eq!(v["TargetFrameRateForOthers"], 144);
    }
}
