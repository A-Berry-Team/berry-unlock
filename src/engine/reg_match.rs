//! Picks the right registry value name. Unity PlayerPrefs append a hash suffix
//! (`GraphicsSettings_Model_h1234567890`), so we match by prefix.

/// Match quality, best first:
/// 0 = exact or `<prefix>_h<digits>`, 1 = starts with prefix, 2 = contains prefix
/// (only for prefixes longer than 5 characters, to avoid accidental hits).
pub fn find_setting_value<'a>(names: &'a [String], prefix: &str) -> Option<&'a String> {
    let p = prefix.to_ascii_lowercase();
    let mut best: Option<(u8, &String)> = None;
    for name in names {
        let l = name.to_ascii_lowercase();
        let rank = if l == p || is_hashed_variant(&l, &p) {
            0
        } else if l.starts_with(&p) {
            1
        } else if p.len() > 5 && l.contains(&p) {
            2
        } else {
            continue;
        };
        if best.map_or(true, |(r, _)| rank < r) {
            best = Some((rank, name));
        }
    }
    best.map(|(_, n)| n)
}

fn is_hashed_variant(lower: &str, prefix: &str) -> bool {
    lower
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix("_h"))
        .map_or(false, |digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prefers_hashed_variant_over_loose_matches() {
        let n = names(&["GraphicsSettings_Model_Backup", "GraphicsSettings_Model_h123456", "Other"]);
        assert_eq!(find_setting_value(&n, "GraphicsSettings_Model").unwrap(), "GraphicsSettings_Model_h123456");
    }

    #[test]
    fn is_case_insensitive_and_falls_back() {
        let n = names(&["graphicssettings_model_extra"]);
        assert_eq!(find_setting_value(&n, "GraphicsSettings_Model").unwrap(), "graphicssettings_model_extra");
        let n = names(&["xx_PersonalGraphicsSettingV2_h9"]);
        assert!(find_setting_value(&n, "PersonalGraphicsSettingV2").is_some());
    }

    #[test]
    fn returns_none_when_absent() {
        assert!(find_setting_value(&names(&["a", "b"]), "GraphicsSettings_Model").is_none());
        assert!(find_setting_value(&[], "x").is_none());
    }

    #[test]
    fn short_prefix_does_not_match_by_containment() {
        assert!(find_setting_value(&names(&["zzabczz"]), "abc").is_none());
    }
}
