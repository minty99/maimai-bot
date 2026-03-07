use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use std::fmt;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SongGenre {
    PopsAnime,
    NiconicoVocaloid,
    TouhouProject,
    GameVariety,
    Maimai,
    OngekiChunithm,
    Utage,
}

impl SongGenre {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "POPS＆ANIME" | "POPS＆アニメ" => Some(Self::PopsAnime),
            "niconico＆VOCALOID™" | "niconico＆ボーカロイド" => {
                Some(Self::NiconicoVocaloid)
            }
            "東方Project" => Some(Self::TouhouProject),
            "GAME＆VARIETY" | "ゲーム＆バラエティ" => Some(Self::GameVariety),
            "maimai" => Some(Self::Maimai),
            "オンゲキ＆CHUNITHM" => Some(Self::OngekiChunithm),
            "宴会場" => Some(Self::Utage),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::PopsAnime => "POPS＆ANIME",
            Self::NiconicoVocaloid => "niconico＆VOCALOID™",
            Self::TouhouProject => "東方Project",
            Self::GameVariety => "GAME＆VARIETY",
            Self::Maimai => "maimai",
            Self::OngekiChunithm => "オンゲキ＆CHUNITHM",
            Self::Utage => "宴会場",
        }
    }
}

impl fmt::Display for SongGenre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for SongGenre {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SongGenre {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("song genre cannot be empty"));
        }
        Self::from_name(&value)
            .ok_or_else(|| D::Error::custom(format!("unknown song genre: {}", value.trim())))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumString,
)]
#[repr(u8)]
pub enum ChartType {
    #[serde(rename = "STD")]
    #[strum(serialize = "STD", serialize = "std", ascii_case_insensitive)]
    Std = 0,
    #[serde(rename = "DX")]
    #[strum(serialize = "DX", serialize = "dx", ascii_case_insensitive)]
    Dx = 1,
}

impl ChartType {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Std => "STD",
            Self::Dx => "DX",
        }
    }

    pub fn from_lowercase(s: &str) -> Option<Self> {
        s.trim().parse::<Self>().ok()
    }

    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Dx => "dx",
        }
    }
}

impl fmt::Display for ChartType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    EnumString,
    EnumIter,
)]
#[repr(u8)]
pub enum DifficultyCategory {
    #[serde(rename = "BASIC")]
    #[strum(serialize = "BASIC", serialize = "basic", ascii_case_insensitive)]
    Basic = 0,

    #[serde(rename = "ADVANCED")]
    #[strum(serialize = "ADVANCED", serialize = "advanced", ascii_case_insensitive)]
    Advanced = 1,

    #[serde(rename = "EXPERT")]
    #[strum(serialize = "EXPERT", serialize = "expert", ascii_case_insensitive)]
    Expert = 2,

    #[serde(rename = "MASTER")]
    #[strum(serialize = "MASTER", serialize = "master", ascii_case_insensitive)]
    Master = 3,

    #[serde(rename = "Re:MASTER")]
    #[strum(
        serialize = "Re:MASTER",
        serialize = "re:master",
        serialize = "RE:MASTER",
        serialize = "remaster",
        serialize = "REMASTER",
        ascii_case_insensitive
    )]
    ReMaster = 4,
}

impl DifficultyCategory {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_index(index: u8) -> Option<Self> {
        Self::iter().find(|difficulty| difficulty.as_u8() == index)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "BASIC",
            Self::Advanced => "ADVANCED",
            Self::Expert => "EXPERT",
            Self::Master => "MASTER",
            Self::ReMaster => "Re:MASTER",
        }
    }

    pub fn from_lowercase(s: &str) -> Option<Self> {
        s.trim().parse::<Self>().ok()
    }

    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
            Self::Master => "master",
            Self::ReMaster => "remaster",
        }
    }

    pub fn from_sheet_abbreviation(s: &str) -> Option<Self> {
        match s.trim() {
            "EXP" => Some(Self::Expert),
            "MAS" => Some(Self::Master),
            "ReMAS" => Some(Self::ReMaster),
            _ => None,
        }
    }
}

impl fmt::Display for DifficultyCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
#[repr(u8)]
pub enum MaimaiVersion {
    Maimai = 0,
    MaimaiPlus = 1,
    Green = 2,
    GreenPlus = 3,
    Orange = 4,
    OrangePlus = 5,
    Pink = 6,
    PinkPlus = 7,
    Murasaki = 8,
    MurasakiPlus = 9,
    Milk = 10,
    MilkPlus = 11,
    Finale = 12,
    Deluxe = 13,
    DeluxePlus = 14,
    Splash = 15,
    SplashPlus = 16,
    Universe = 17,
    UniversePlus = 18,
    Festival = 19,
    FestivalPlus = 20,
    Buddies = 21,
    BuddiesPlus = 22,
    Prism = 23,
    PrismPlus = 24,
    Circle = 25,
}

impl MaimaiVersion {
    pub const fn as_index(self) -> u8 {
        self as u8
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Maimai => "maimai",
            Self::MaimaiPlus => "maimai PLUS",
            Self::Green => "GreeN",
            Self::GreenPlus => "GreeN PLUS",
            Self::Orange => "ORANGE",
            Self::OrangePlus => "ORANGE PLUS",
            Self::Pink => "PiNK",
            Self::PinkPlus => "PiNK PLUS",
            Self::Murasaki => "MURASAKi",
            Self::MurasakiPlus => "MURASAKi PLUS",
            Self::Milk => "MiLK",
            Self::MilkPlus => "MiLK PLUS",
            Self::Finale => "FiNALE",
            Self::Deluxe => "maimaiでらっくす",
            Self::DeluxePlus => "maimaiでらっくす PLUS",
            Self::Splash => "Splash",
            Self::SplashPlus => "Splash PLUS",
            Self::Universe => "UNiVERSE",
            Self::UniversePlus => "UNiVERSE PLUS",
            Self::Festival => "FESTiVAL",
            Self::FestivalPlus => "FESTiVAL PLUS",
            Self::Buddies => "BUDDiES",
            Self::BuddiesPlus => "BUDDiES PLUS",
            Self::Prism => "PRiSM",
            Self::PrismPlus => "PRiSM PLUS",
            Self::Circle => "CiRCLE",
        }
    }

    pub fn from_index(index: u8) -> Option<Self> {
        Self::iter().find(|version| version.as_index() == index)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = name.trim();
        Self::iter().find(|version| version.as_str() == normalized)
    }
}

impl Serialize for MaimaiVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MaimaiVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("maimai version cannot be empty"));
        }
        Self::from_name(&value)
            .ok_or_else(|| D::Error::custom(format!("unknown maimai version: {}", value.trim())))
    }
}

#[cfg(test)]
mod song_genre_tests {
    use super::SongGenre;

    #[test]
    fn song_genre_loads_jp_and_intl_aliases() {
        assert_eq!(
            SongGenre::from_name("niconico＆ボーカロイド"),
            Some(SongGenre::NiconicoVocaloid)
        );
        assert_eq!(
            SongGenre::from_name("niconico＆VOCALOID™"),
            Some(SongGenre::NiconicoVocaloid)
        );
        assert_eq!(
            SongGenre::from_name("ゲーム＆バラエティ"),
            Some(SongGenre::GameVariety)
        );
        assert_eq!(
            SongGenre::from_name("GAME＆VARIETY"),
            Some(SongGenre::GameVariety)
        );
    }

    #[test]
    fn song_genre_formats_as_intl_string() {
        assert_eq!(SongGenre::PopsAnime.to_string(), "POPS＆ANIME");
        assert_eq!(
            SongGenre::NiconicoVocaloid.to_string(),
            "niconico＆VOCALOID™"
        );
        assert_eq!(SongGenre::GameVariety.to_string(), "GAME＆VARIETY");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
pub enum ScoreRank {
    #[serde(rename = "SSS+")]
    #[strum(serialize = "SSS+")]
    SssPlus,
    #[serde(rename = "SSS")]
    #[strum(serialize = "SSS")]
    Sss,
    #[serde(rename = "SS+")]
    #[strum(serialize = "SS+")]
    SsPlus,
    #[serde(rename = "SS")]
    #[strum(serialize = "SS")]
    Ss,
    #[serde(rename = "S+")]
    #[strum(serialize = "S+")]
    SPlus,
    #[serde(rename = "S")]
    #[strum(serialize = "S")]
    S,
    #[serde(rename = "AAA")]
    #[strum(serialize = "AAA")]
    Aaa,
    #[serde(rename = "AA")]
    #[strum(serialize = "AA")]
    Aa,
    #[serde(rename = "A")]
    #[strum(serialize = "A")]
    A,
    #[serde(rename = "BBB")]
    #[strum(serialize = "BBB")]
    Bbb,
    #[serde(rename = "BB")]
    #[strum(serialize = "BB")]
    Bb,
    #[serde(rename = "B")]
    #[strum(serialize = "B")]
    B,
    #[serde(rename = "C")]
    #[strum(serialize = "C")]
    C,
    #[serde(rename = "D")]
    #[strum(serialize = "D")]
    D,
}

impl ScoreRank {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SssPlus => "SSS+",
            Self::Sss => "SSS",
            Self::SsPlus => "SS+",
            Self::Ss => "SS",
            Self::SPlus => "S+",
            Self::S => "S",
            Self::Aaa => "AAA",
            Self::Aa => "AA",
            Self::A => "A",
            Self::Bbb => "BBB",
            Self::Bb => "BB",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }

    pub fn from_score_icon_key(key: &str) -> Option<Self> {
        Some(match key {
            "sssp" => Self::SssPlus,
            "sss" => Self::Sss,
            "ssp" => Self::SsPlus,
            "ss" => Self::Ss,
            "sp" => Self::SPlus,
            "s" => Self::S,
            "aaa" => Self::Aaa,
            "aa" => Self::Aa,
            "a" => Self::A,
            "bbb" => Self::Bbb,
            "bb" => Self::Bb,
            "b" => Self::B,
            "c" => Self::C,
            "d" => Self::D,
            _ => return None,
        })
    }

    pub fn from_playlog_stem(stem: &str) -> Option<Self> {
        let s = stem.trim().to_ascii_lowercase();
        Some(match s.as_str() {
            "sssplus" => Self::SssPlus,
            "sss" => Self::Sss,
            "ssplus" => Self::SsPlus,
            "ss" => Self::Ss,
            "splus" => Self::SPlus,
            "s" => Self::S,
            "aaa" => Self::Aaa,
            "aa" => Self::Aa,
            "a" => Self::A,
            "bbb" => Self::Bbb,
            "bb" => Self::Bb,
            "b" => Self::B,
            "c" => Self::C,
            "d" => Self::D,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
pub enum FcStatus {
    #[serde(rename = "AP+")]
    #[strum(serialize = "AP+")]
    ApPlus,
    #[serde(rename = "AP")]
    #[strum(serialize = "AP")]
    Ap,
    #[serde(rename = "FC+")]
    #[strum(serialize = "FC+")]
    FcPlus,
    #[serde(rename = "FC")]
    #[strum(serialize = "FC")]
    Fc,
}

impl FcStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApPlus => "AP+",
            Self::Ap => "AP",
            Self::FcPlus => "FC+",
            Self::Fc => "FC",
        }
    }

    pub fn from_score_icon_key(key: &str) -> Option<Self> {
        Some(match key {
            "app" => Self::ApPlus,
            "ap" => Self::Ap,
            "fcp" => Self::FcPlus,
            "fc" => Self::Fc,
            _ => return None,
        })
    }

    pub fn from_playlog_key(key: &str) -> Option<Self> {
        let s = key.trim().to_ascii_lowercase();
        Some(match s.as_str() {
            "app" => Self::ApPlus,
            "ap" => Self::Ap,
            "fcp" => Self::FcPlus,
            "fc" => Self::Fc,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
pub enum SyncStatus {
    #[serde(rename = "FDX+")]
    #[strum(serialize = "FDX+")]
    FdxPlus,
    #[serde(rename = "FDX")]
    #[strum(serialize = "FDX")]
    Fdx,
    #[serde(rename = "FS+")]
    #[strum(serialize = "FS+")]
    FsPlus,
    #[serde(rename = "FS")]
    #[strum(serialize = "FS")]
    Fs,
    #[serde(rename = "SYNC")]
    #[strum(serialize = "SYNC")]
    Sync,
}

impl SyncStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FdxPlus => "FDX+",
            Self::Fdx => "FDX",
            Self::FsPlus => "FS+",
            Self::Fs => "FS",
            Self::Sync => "SYNC",
        }
    }

    pub const fn priority(self) -> u8 {
        match self {
            Self::FdxPlus => 5,
            Self::Fdx => 4,
            Self::FsPlus => 3,
            Self::Fs => 2,
            Self::Sync => 1,
        }
    }

    pub fn from_score_icon_key(key: &str) -> Option<Self> {
        Some(match key {
            "fdxp" => Self::FdxPlus,
            "fdx" => Self::Fdx,
            "fsp" => Self::FsPlus,
            "fs" => Self::Fs,
            "sync" => Self::Sync,
            _ => return None,
        })
    }

    pub fn from_playlog_key(key: &str) -> Option<Self> {
        let s = key.trim().to_ascii_lowercase();
        Some(match s.as_str() {
            "fdxp" => Self::FdxPlus,
            "fdx" => Self::Fdx,
            "fsp" => Self::FsPlus,
            "fs" => Self::Fs,
            "sync" => Self::Sync,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ChartType, DifficultyCategory, MaimaiVersion};
    use strum::IntoEnumIterator;

    #[test]
    fn release_order_indices_are_stable() {
        for (expected_index, version) in MaimaiVersion::iter().enumerate() {
            assert_eq!(version.as_index() as usize, expected_index);
            assert_eq!(
                MaimaiVersion::from_index(expected_index as u8),
                Some(version)
            );
        }
    }

    #[test]
    fn version_name_roundtrip() {
        for version in MaimaiVersion::iter() {
            assert_eq!(MaimaiVersion::from_name(version.as_str()), Some(version));
        }
    }

    #[test]
    fn chart_and_difficulty_display_are_canonical() {
        assert_eq!(ChartType::Std.to_string(), "STD");
        assert_eq!(ChartType::Dx.to_string(), "DX");
        assert_eq!(DifficultyCategory::Basic.to_string(), "BASIC");
        assert_eq!(DifficultyCategory::Advanced.to_string(), "ADVANCED");
        assert_eq!(DifficultyCategory::Expert.to_string(), "EXPERT");
        assert_eq!(DifficultyCategory::Master.to_string(), "MASTER");
        assert_eq!(DifficultyCategory::ReMaster.to_string(), "Re:MASTER");
    }
}
