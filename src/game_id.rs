//! Supported game identifiers (kept identical to KeqingUnlock for launcher compatibility).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameId {
    GenshinGlobal,
    StarRailGlobal,
    Honkai3Global,
    WutheringWavesGlobal,
    /// Zenless Zone Zero: unlimited FPS is a built-in setting; nothing to do.
    ZenlessGlobal,
}

impl GameId {
    pub const ALL: [GameId; 5] = [
        GameId::GenshinGlobal,
        GameId::StarRailGlobal,
        GameId::Honkai3Global,
        GameId::WutheringWavesGlobal,
        GameId::ZenlessGlobal,
    ];

    pub fn id(self) -> &'static str {
        match self {
            GameId::GenshinGlobal => "hk4e_global",
            GameId::StarRailGlobal => "hkrpg_global",
            GameId::Honkai3Global => "bh3_global",
            GameId::WutheringWavesGlobal => "wuwa_global",
            GameId::ZenlessGlobal => "nap_global",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GameId::GenshinGlobal => "Genshin Impact",
            GameId::StarRailGlobal => "Honkai: Star Rail",
            GameId::Honkai3Global => "Honkai Impact 3rd",
            GameId::WutheringWavesGlobal => "Wuthering Waves",
            GameId::ZenlessGlobal => "Zenless Zone Zero",
        }
    }

    pub fn parse(s: &str) -> Option<GameId> {
        let s = s.trim();
        GameId::ALL.into_iter().find(|g| g.id().eq_ignore_ascii_case(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_ids_case_insensitively() {
        assert_eq!(GameId::parse("hk4e_global"), Some(GameId::GenshinGlobal));
        assert_eq!(GameId::parse("WUWA_GLOBAL"), Some(GameId::WutheringWavesGlobal));
        assert_eq!(GameId::parse("nope"), None);
    }

    #[test]
    fn ids_round_trip() {
        for g in GameId::ALL {
            assert_eq!(GameId::parse(g.id()), Some(g));
        }
    }
}
