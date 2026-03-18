use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use std::fmt;
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator};

fn normalize_ascii_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_ascii_token_with_plus(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '+')
        .flat_map(char::to_lowercase)
        .collect()
}

fn trim_file_stem(value: &str) -> &str {
    let file = value.trim().rsplit('/').next().unwrap_or(value.trim());
    let file = file.split('?').next().unwrap_or(file);
    file.strip_suffix(".png").unwrap_or(file)
}

fn parse_song_genre(value: &str) -> Option<SongGenre> {
    match value.trim() {
        "POPS＆ANIME" | "POPS＆アニメ" => Some(SongGenre::PopsAnime),
        "niconico＆VOCALOID™" | "niconico＆ボーカロイド" => {
            Some(SongGenre::NiconicoVocaloid)
        }
        "東方Project" => Some(SongGenre::TouhouProject),
        "GAME＆VARIETY" | "ゲーム＆バラエティ" => Some(SongGenre::GameVariety),
        "maimai" => Some(SongGenre::Maimai),
        "オンゲキ＆CHUNITHM" => Some(SongGenre::OngekiChunithm),
        "宴会場" => Some(SongGenre::Utage),
        _ => match normalize_ascii_token(value).as_str() {
            "popsanime" => Some(SongGenre::PopsAnime),
            "niconicovocaloid" => Some(SongGenre::NiconicoVocaloid),
            "touhouproject" => Some(SongGenre::TouhouProject),
            "gamevariety" => Some(SongGenre::GameVariety),
            "maimai" => Some(SongGenre::Maimai),
            "ongekichunithm" => Some(SongGenre::OngekiChunithm),
            "utage" => Some(SongGenre::Utage),
            _ => None,
        },
    }
}

fn parse_chart_type(value: &str) -> Option<ChartType> {
    let stem = trim_file_stem(value);
    match normalize_ascii_token(stem).as_str() {
        "std" | "standard" | "musicstandard" => Some(ChartType::Std),
        "dx" | "deluxe" | "musicdx" => Some(ChartType::Dx),
        _ => None,
    }
}

fn parse_difficulty_category(value: &str) -> Option<DifficultyCategory> {
    let trimmed = trim_file_stem(value);
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(index) = trimmed.parse::<u8>()
        && let Some(diff_category) = DifficultyCategory::from_index(index)
    {
        return Some(diff_category);
    }

    match normalize_ascii_token(trimmed).as_str() {
        "basic" => Some(DifficultyCategory::Basic),
        "advanced" => Some(DifficultyCategory::Advanced),
        "expert" | "exp" => Some(DifficultyCategory::Expert),
        "master" | "mas" => Some(DifficultyCategory::Master),
        "diffbasic" => Some(DifficultyCategory::Basic),
        "diffadvanced" => Some(DifficultyCategory::Advanced),
        "diffexpert" => Some(DifficultyCategory::Expert),
        "diffmaster" => Some(DifficultyCategory::Master),
        "remaster" => Some(DifficultyCategory::ReMaster),
        "remas" | "diffremaster" => Some(DifficultyCategory::ReMaster),
        _ => None,
    }
}

fn parse_maimai_version(value: &str) -> Option<MaimaiVersion> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(index) = trimmed.parse::<u8>()
        && let Some(version) = MaimaiVersion::from_index(index)
    {
        return Some(version);
    }

    if let Some(version) = MaimaiVersion::iter().find(|version| version.as_str() == trimmed) {
        return Some(version);
    }

    match normalize_ascii_token(trimmed).as_str() {
        "maimai" => Some(MaimaiVersion::Maimai),
        "maimaiplus" => Some(MaimaiVersion::MaimaiPlus),
        "green" => Some(MaimaiVersion::Green),
        "greenplus" => Some(MaimaiVersion::GreenPlus),
        "orange" => Some(MaimaiVersion::Orange),
        "orangeplus" => Some(MaimaiVersion::OrangePlus),
        "pink" => Some(MaimaiVersion::Pink),
        "pinkplus" => Some(MaimaiVersion::PinkPlus),
        "murasaki" => Some(MaimaiVersion::Murasaki),
        "murasakiplus" => Some(MaimaiVersion::MurasakiPlus),
        "milk" => Some(MaimaiVersion::Milk),
        "milkplus" => Some(MaimaiVersion::MilkPlus),
        "finale" => Some(MaimaiVersion::Finale),
        "deluxe" | "maimaideluxe" => Some(MaimaiVersion::Deluxe),
        "deluxeplus" | "maimaideluxeplus" => Some(MaimaiVersion::DeluxePlus),
        "splash" => Some(MaimaiVersion::Splash),
        "splashplus" => Some(MaimaiVersion::SplashPlus),
        "universe" => Some(MaimaiVersion::Universe),
        "universeplus" => Some(MaimaiVersion::UniversePlus),
        "festival" => Some(MaimaiVersion::Festival),
        "festivalplus" => Some(MaimaiVersion::FestivalPlus),
        "buddies" => Some(MaimaiVersion::Buddies),
        "buddiesplus" => Some(MaimaiVersion::BuddiesPlus),
        "prism" => Some(MaimaiVersion::Prism),
        "prismplus" => Some(MaimaiVersion::PrismPlus),
        "circle" => Some(MaimaiVersion::Circle),
        "circleplus" => Some(MaimaiVersion::CirclePlus),
        _ => None,
    }
}

fn parse_score_rank(value: &str) -> Option<ScoreRank> {
    let normalized =
        normalize_ascii_token_with_plus(trim_file_stem(value).trim_start_matches("music_icon_"));
    match normalized.as_str() {
        "sss+" | "sssp" | "sssplus" => Some(ScoreRank::SssPlus),
        "ss+" | "ssp" | "ssplus" => Some(ScoreRank::SsPlus),
        "s+" | "sp" | "splus" => Some(ScoreRank::SPlus),
        "sss" => Some(ScoreRank::Sss),
        "ss" => Some(ScoreRank::Ss),
        "s" => Some(ScoreRank::S),
        "aaa" => Some(ScoreRank::Aaa),
        "aa" => Some(ScoreRank::Aa),
        "a" => Some(ScoreRank::A),
        "bbb" => Some(ScoreRank::Bbb),
        "bb" => Some(ScoreRank::Bb),
        "b" => Some(ScoreRank::B),
        "c" => Some(ScoreRank::C),
        "d" => Some(ScoreRank::D),
        _ => None,
    }
}

fn parse_fc_status(value: &str) -> Option<FcStatus> {
    let normalized = normalize_ascii_token_with_plus(
        trim_file_stem(value)
            .trim_start_matches("music_icon_")
            .trim_start_matches("fc_"),
    );
    match normalized.as_str() {
        "ap+" | "app" | "applus" => Some(FcStatus::ApPlus),
        "ap" => Some(FcStatus::Ap),
        "fc+" | "fcp" | "fcplus" => Some(FcStatus::FcPlus),
        "fc" => Some(FcStatus::Fc),
        _ => None,
    }
}

fn parse_sync_status(value: &str) -> Option<SyncStatus> {
    let normalized = normalize_ascii_token_with_plus(
        trim_file_stem(value)
            .trim_start_matches("music_icon_")
            .trim_start_matches("sync_"),
    );
    match normalized.as_str() {
        "fdx+" | "fdxp" | "fdxplus" => Some(SyncStatus::FdxPlus),
        "fs+" | "fsp" | "fsplus" => Some(SyncStatus::FsPlus),
        "fdx" => Some(SyncStatus::Fdx),
        "fs" => Some(SyncStatus::Fs),
        "sync" => Some(SyncStatus::Sync),
        _ => None,
    }
}

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

impl FromStr for SongGenre {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_song_genre(s).ok_or(())
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
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown song genre: {}", value.trim())))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(u8)]
pub enum ChartType {
    #[serde(rename = "STD")]
    Std = 0,
    #[serde(rename = "DX")]
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

    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Dx => "dx",
        }
    }
}

impl FromStr for ChartType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_chart_type(s).ok_or(())
    }
}

impl<'de> Deserialize<'de> for ChartType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("chart type cannot be empty"));
        }
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown chart type: {}", value.trim())))
    }
}

impl fmt::Display for ChartType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, EnumIter)]
#[repr(u8)]
pub enum DifficultyCategory {
    #[serde(rename = "BASIC")]
    Basic = 0,

    #[serde(rename = "ADVANCED")]
    Advanced = 1,

    #[serde(rename = "EXPERT")]
    Expert = 2,

    #[serde(rename = "MASTER")]
    Master = 3,

    #[serde(rename = "Re:MASTER")]
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

    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
            Self::Master => "master",
            Self::ReMaster => "remaster",
        }
    }
}

impl FromStr for DifficultyCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_difficulty_category(s).ok_or(())
    }
}

impl<'de> Deserialize<'de> for DifficultyCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("difficulty category cannot be empty"));
        }
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown difficulty category: {}", value.trim())))
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
    CirclePlus = 26,
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
            Self::CirclePlus => "CiRCLE PLUS",
        }
    }

    pub fn from_index(index: u8) -> Option<Self> {
        Self::iter().find(|version| version.as_index() == index)
    }
}

impl FromStr for MaimaiVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_maimai_version(s).ok_or(())
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
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown maimai version: {}", value.trim())))
    }
}

#[cfg(test)]
mod song_genre_tests {
    use super::SongGenre;
    use std::collections::BTreeSet;

    #[test]
    fn song_genre_loads_jp_and_intl_aliases() {
        assert_eq!(
            "POPS＆アニメ".parse::<SongGenre>().ok(),
            Some(SongGenre::PopsAnime)
        );
        assert_eq!(
            "POPS＆ANIME".parse::<SongGenre>().ok(),
            Some(SongGenre::PopsAnime)
        );
        assert_eq!(
            "niconico＆ボーカロイド".parse::<SongGenre>().ok(),
            Some(SongGenre::NiconicoVocaloid)
        );
        assert_eq!(
            "niconico＆VOCALOID™".parse::<SongGenre>().ok(),
            Some(SongGenre::NiconicoVocaloid)
        );
        assert_eq!(
            "ゲーム＆バラエティ".parse::<SongGenre>().ok(),
            Some(SongGenre::GameVariety)
        );
        assert_eq!(
            "GAME＆VARIETY".parse::<SongGenre>().ok(),
            Some(SongGenre::GameVariety)
        );
        assert_eq!(
            "東方Project".parse::<SongGenre>().ok(),
            Some(SongGenre::TouhouProject)
        );
        assert_eq!(
            "オンゲキ＆CHUNITHM".parse::<SongGenre>().ok(),
            Some(SongGenre::OngekiChunithm)
        );
        assert_eq!("宴会場".parse::<SongGenre>().ok(), Some(SongGenre::Utage));
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

    #[test]
    fn song_genre_parses_all_official_fixture_catcodes() {
        let fixture = include_str!(
            "../../../maistats-song-info/src/examples/maimai/official/maimai_songs.json"
        );
        let rows: serde_json::Value =
            serde_json::from_str(fixture).expect("parse official songs fixture");
        let catcodes = rows
            .as_array()
            .expect("fixture should be an array")
            .iter()
            .map(|row| {
                row.get("catcode")
                    .and_then(|value| value.as_str())
                    .expect("catcode should be present")
                    .to_string()
            })
            .collect::<BTreeSet<_>>();

        for catcode in catcodes {
            assert!(
                catcode.parse::<SongGenre>().is_ok(),
                "catcode should parse: {catcode}"
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display)]
pub enum ScoreRank {
    #[serde(rename = "SSS+")]
    SssPlus,
    #[serde(rename = "SSS")]
    Sss,
    #[serde(rename = "SS+")]
    SsPlus,
    #[serde(rename = "SS")]
    Ss,
    #[serde(rename = "S+")]
    SPlus,
    #[serde(rename = "S")]
    S,
    #[serde(rename = "AAA")]
    Aaa,
    #[serde(rename = "AA")]
    Aa,
    #[serde(rename = "A")]
    A,
    #[serde(rename = "BBB")]
    Bbb,
    #[serde(rename = "BB")]
    Bb,
    #[serde(rename = "B")]
    B,
    #[serde(rename = "C")]
    C,
    #[serde(rename = "D")]
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
}

impl FromStr for ScoreRank {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_score_rank(s).ok_or(())
    }
}

impl<'de> Deserialize<'de> for ScoreRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("score rank cannot be empty"));
        }
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown score rank: {}", value.trim())))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display)]
pub enum FcStatus {
    #[serde(rename = "AP+")]
    ApPlus,
    #[serde(rename = "AP")]
    Ap,
    #[serde(rename = "FC+")]
    FcPlus,
    #[serde(rename = "FC")]
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
}

impl FromStr for FcStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_fc_status(s).ok_or(())
    }
}

impl<'de> Deserialize<'de> for FcStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("fc status cannot be empty"));
        }
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown fc status: {}", value.trim())))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Display)]
pub enum SyncStatus {
    #[serde(rename = "FDX+")]
    FdxPlus,
    #[serde(rename = "FDX")]
    Fdx,
    #[serde(rename = "FS+")]
    FsPlus,
    #[serde(rename = "FS")]
    Fs,
    #[serde(rename = "SYNC")]
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
}

impl FromStr for SyncStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_sync_status(s).ok_or(())
    }
}

impl<'de> Deserialize<'de> for SyncStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("sync status cannot be empty"));
        }
        value
            .parse::<Self>()
            .map_err(|_| D::Error::custom(format!("unknown sync status: {}", value.trim())))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChartType, DifficultyCategory, FcStatus, MaimaiVersion, ScoreRank, SongGenre, SyncStatus,
    };
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
            assert_eq!(
                version.as_str().parse::<MaimaiVersion>().ok(),
                Some(version)
            );
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

    #[test]
    fn raw_string_parsers_are_flexible() {
        assert_eq!(
            "pops anime".parse::<SongGenre>().ok(),
            Some(SongGenre::PopsAnime)
        );
        assert_eq!(
            "/img/music_standard.png".parse::<ChartType>().ok(),
            Some(ChartType::Std)
        );
        assert_eq!("deluxe".parse::<ChartType>().ok(), Some(ChartType::Dx));
        assert_eq!(
            "diff_remaster.png".parse::<DifficultyCategory>().ok(),
            Some(DifficultyCategory::ReMaster)
        );
        assert_eq!(
            "ReMAS".parse::<DifficultyCategory>().ok(),
            Some(DifficultyCategory::ReMaster)
        );
        assert_eq!(
            "maimai deluxe plus".parse::<MaimaiVersion>().ok(),
            Some(MaimaiVersion::DeluxePlus)
        );
        assert_eq!(
            "25".parse::<MaimaiVersion>().ok(),
            Some(MaimaiVersion::Circle)
        );
        assert_eq!(
            "circle plus".parse::<MaimaiVersion>().ok(),
            Some(MaimaiVersion::CirclePlus)
        );
        assert_eq!(
            "music_icon_sssp.png".parse::<ScoreRank>().ok(),
            Some(ScoreRank::SssPlus)
        );
        assert_eq!(
            "sssplus".parse::<ScoreRank>().ok(),
            Some(ScoreRank::SssPlus)
        );
        assert_eq!("SSS+".parse::<ScoreRank>().ok(), Some(ScoreRank::SssPlus));
        assert_eq!("SS+".parse::<ScoreRank>().ok(), Some(ScoreRank::SsPlus));
        assert_eq!("S+".parse::<ScoreRank>().ok(), Some(ScoreRank::SPlus));
        assert_eq!("fcplus".parse::<FcStatus>().ok(), Some(FcStatus::FcPlus));
        assert_eq!("FC+".parse::<FcStatus>().ok(), Some(FcStatus::FcPlus));
        assert_eq!("AP+".parse::<FcStatus>().ok(), Some(FcStatus::ApPlus));
        assert_eq!(
            "fc_app.png".parse::<FcStatus>().ok(),
            Some(FcStatus::ApPlus)
        );
        assert_eq!(
            "sync_fdxplus.png".parse::<SyncStatus>().ok(),
            Some(SyncStatus::FdxPlus)
        );
        assert_eq!("fsp".parse::<SyncStatus>().ok(), Some(SyncStatus::FsPlus));
        assert_eq!("FS+".parse::<SyncStatus>().ok(), Some(SyncStatus::FsPlus));
        assert_eq!("FDX+".parse::<SyncStatus>().ok(), Some(SyncStatus::FdxPlus));
    }
}
