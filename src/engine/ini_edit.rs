//! In-place INI editing that preserves comments, ordering, duplicate keys in
//! other sections, BOM and line endings. (KeqingUnlock re-serialised the whole
//! file through an INI library, lower-casing keys and dropping comments.)

/// Sets `key=value` inside `[section]`, creating the section/key if needed.
pub fn set_key(content: &str, section: &str, key: &str, value: &str) -> String {
    let (bom, body) = match content.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", content),
    };
    let nl = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = body.lines().map(str::to_owned).collect();
    let header = format!("[{section}]");
    let new_line = format!("{key}={value}");

    match lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&header)) {
        Some(start) => {
            let end = lines
                .iter()
                .enumerate()
                .skip(start + 1)
                .find(|(_, l)| l.trim_start().starts_with('['))
                .map_or(lines.len(), |(i, _)| i);

            let matching: Vec<usize> = (start + 1..end)
                .filter(|&i| line_key(&lines[i]).map_or(false, |k| k.eq_ignore_ascii_case(key)))
                .collect();

            if let Some((&first, dups)) = matching.split_first() {
                lines[first] = new_line;
                for &d in dups.iter().rev() {
                    lines.remove(d);
                }
            } else {
                let mut at = end;
                while at > start + 1 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, new_line);
            }
        }
        None => {
            if lines.last().map_or(false, |l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(header);
            lines.push(new_line);
        }
    }

    let mut out = String::with_capacity(content.len() + 64);
    out.push_str(bom);
    out.push_str(&lines.join(nl));
    out.push_str(nl);
    out
}

fn line_key(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    t.split_once('=').map(|(k, _)| k.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: &str = "/Script/Engine.GameUserSettings";

    #[test]
    fn replaces_existing_key_preserving_the_rest() {
        let src = "[A]\r\nX=1\r\n\r\n[/Script/Engine.GameUserSettings]\r\n; keep me\r\nFramePace=30\r\nOther=2\r\n";
        let out = set_key(src, SEC, "FramePace", "120");
        assert_eq!(
            out,
            "[A]\r\nX=1\r\n\r\n[/Script/Engine.GameUserSettings]\r\n; keep me\r\nFramePace=120\r\nOther=2\r\n"
        );
    }

    #[test]
    fn matching_is_case_insensitive_and_dedupes() {
        let src = "[/script/engine.gameusersettings]\nframepace=30\nFramePace=45\nZ=1\n";
        let out = set_key(src, SEC, "FramePace", "60");
        assert_eq!(out, "[/script/engine.gameusersettings]\nFramePace=60\nZ=1\n");
    }

    #[test]
    fn inserts_missing_key_before_trailing_blank_lines() {
        let src = "[/Script/Engine.GameUserSettings]\nA=1\n\n[Next]\nB=2\n";
        let out = set_key(src, SEC, "FramePace", "90");
        assert_eq!(out, "[/Script/Engine.GameUserSettings]\nA=1\nFramePace=90\n\n[Next]\nB=2\n");
    }

    #[test]
    fn creates_missing_section() {
        assert_eq!(set_key("[A]\nX=1\n", "S", "K", "V"), "[A]\nX=1\n\n[S]\nK=V\n");
        assert_eq!(set_key("", "S", "K", "V"), "[S]\nK=V\n");
    }

    #[test]
    fn keeps_bom_and_ignores_keys_in_other_sections() {
        let src = "\u{feff}[Other]\nFramePace=1\n[/Script/Engine.GameUserSettings]\nA=1\n";
        let out = set_key(src, SEC, "FramePace", "60");
        assert!(out.starts_with('\u{feff}'));
        assert!(out.contains("[Other]\nFramePace=1\n"));
        assert!(out.ends_with("A=1\nFramePace=60\n"));
    }

    #[test]
    fn is_idempotent() {
        let once = set_key("[S]\nK=1\n", "S", "K", "2");
        assert_eq!(set_key(&once, "S", "K", "2"), once);
    }
}
