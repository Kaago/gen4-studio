/// Game family, derived from the first three characters of the ROM game code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Family {
    /// Diamond / Pearl
    Dp,
    /// Platinum
    Pt,
    /// HeartGold / SoulSilver
    Hgss,
}

impl Family {
    /// Maps a four-character game code (e.g. `CPUD`) to its family.
    pub fn from_game_code(code: &str) -> Option<Family> {
        match code.get(..3)? {
            "ADA" | "APA" => Some(Family::Dp),
            "CPU" => Some(Family::Pt),
            "IPK" | "IPG" => Some(Family::Hgss),
            _ => None,
        }
    }
}

/// ROM language, derived from the fourth character of the game code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    French,
    German,
    Italian,
    Spanish,
    Japanese,
    Korean,
}

impl Language {
    /// Maps a four-character game code (e.g. `CPUD`) to its language.
    pub fn from_game_code(code: &str) -> Option<Language> {
        match code.chars().nth(3)? {
            'E' => Some(Language::English),
            'F' => Some(Language::French),
            'D' => Some(Language::German),
            'I' => Some(Language::Italian),
            'S' => Some(Language::Spanish),
            'J' => Some(Language::Japanese),
            'K' => Some(Language::Korean),
            _ => None,
        }
    }

    /// Locale key used by chatot and uxie text JSON (`de_DE`, `en_US`, ...).
    pub fn locale_key(self) -> &'static str {
        match self {
            Language::English => "en_US",
            Language::French => "fr_FR",
            Language::German => "de_DE",
            Language::Italian => "it_IT",
            Language::Spanish => "es_ES",
            Language::Japanese => "ja_JP",
            Language::Korean => "ko_KR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_from_game_code() {
        assert_eq!(Family::from_game_code("ADAE"), Some(Family::Dp));
        assert_eq!(Family::from_game_code("APAJ"), Some(Family::Dp));
        assert_eq!(Family::from_game_code("CPUE"), Some(Family::Pt));
        assert_eq!(Family::from_game_code("IPKE"), Some(Family::Hgss));
        assert_eq!(Family::from_game_code("IPGD"), Some(Family::Hgss));
        assert_eq!(Family::from_game_code("XXXX"), None);
        assert_eq!(Family::from_game_code("CP"), None);
    }

    #[test]
    fn language_from_game_code() {
        assert_eq!(Language::from_game_code("CPUE"), Some(Language::English));
        assert_eq!(Language::from_game_code("IPGD"), Some(Language::German));
        assert_eq!(Language::from_game_code("ADAJ"), Some(Language::Japanese));
        assert_eq!(Language::from_game_code("IPKK"), Some(Language::Korean));
        assert_eq!(Language::from_game_code("CPUX"), None);
    }
}
