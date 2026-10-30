//! Static menu presets written for Wuthering Waves (identical content to
//! KeqingUnlock; these map menu option ids to values so the in-game FPS menu
//! accepts the custom frame rate).

use serde_json::{json, Map, Value};

fn base_entries() -> Vec<(u32, f64)> {
    let mut v: Vec<(u32, f64)> = vec![
        (1, 100.0), (2, 100.0), (3, 100.0), (4, 100.0), (5, 0.0), (6, 0.0),
        (7, -0.4658685302734375), (10, 3.0), (11, 3.0),
    ];
    v.extend((20..=50).map(|k| (k, 0.0)));
    v.extend([
        (51, 1.0), (52, 1.0), (53, 0.0), (54, 3.0), (55, 1.0), (56, 2.0), (57, 1.0),
        (58, 1.0), (59, 1.0), (61, 0.0), (62, 0.0), (63, 1.0), (64, 1.0), (65, 0.0),
        (66, 0.0), (67, 3.0), (68, 2.0), (69, 100.0), (70, 100.0), (79, 1.0), (81, 0.0),
        (82, 1.0), (83, 1.0), (84, 0.0), (85, 0.0), (87, 0.0), (88, 0.0), (89, 50.0),
        (90, 50.0), (91, 50.0), (92, 50.0), (93, 1.0), (99, 0.0), (100, 30.0), (101, 0.0),
        (102, 1.0), (103, 0.0), (104, 50.0), (105, 0.0), (106, 0.3), (107, 0.0),
    ]);
    v.extend((112..=120).map(|k| (k, 0.0)));
    v.extend([(121, 1.0), (122, 1.0), (123, 0.0), (130, 0.0), (131, 0.0), (132, 1.0)]);
    v
}

/// `MenuData` row: `{"___MetaType___":"___Map___","content":[[id,value],...]}`.
pub fn menu_data_json() -> String {
    let mut entries = base_entries();
    entries.extend([(135, 1.0), (133, 0.0)]);
    let content: Vec<Value> = entries.iter().map(|(k, v)| json!([k, v])).collect();
    json!({ "___MetaType___": "___Map___", "content": content }).to_string()
}

/// `PlayMenuInfo` row: `{"1":100.0,"2":100.0,...}` (same ids, minus 133/135).
pub fn play_menu_info_json() -> String {
    let mut map = Map::new();
    for (k, v) in base_entries() {
        map.insert(k.to_string(), json!(v));
    }
    Value::Object(map).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_data_matches_original_layout() {
        let v: Value = serde_json::from_str(&menu_data_json()).unwrap();
        assert_eq!(v["___MetaType___"], "___Map___");
        let content = v["content"].as_array().unwrap();
        assert_eq!(content.len(), 98);
        assert_eq!(content[0], json!([1, 100.0]));
        assert_eq!(content[6], json!([7, -0.4658685302734375]));
        assert_eq!(content[96], json!([135, 1.0]));
        assert_eq!(content[97], json!([133, 0.0]));
    }

    #[test]
    fn play_menu_info_matches_original_layout() {
        let v: Value = serde_json::from_str(&play_menu_info_json()).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 96);
        assert_eq!(obj["7"], json!(-0.4658685302734375));
        assert_eq!(obj["132"], json!(1.0));
        assert!(!obj.contains_key("133") && !obj.contains_key("135"));
    }
}
