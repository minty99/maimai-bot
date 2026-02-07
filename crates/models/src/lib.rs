use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

pub mod config;

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
    Display,
)]
#[repr(u8)]
pub enum ChartType {
    #[serde(rename = "STD")]
    #[strum(serialize = "STD")]
    Std = 0,
    #[serde(rename = "DX")]
    #[strum(serialize = "DX")]
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

    /// Parse from lowercase songdb JSON value ("std", "dx").
    pub fn from_lowercase(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "std" => Some(Self::Std),
            "dx" => Some(Self::Dx),
            _ => None,
        }
    }

    /// Return lowercase songdb JSON value ("std", "dx").
    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Std => "std",
            Self::Dx => "dx",
        }
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
    Display,
)]
#[repr(u8)]
pub enum DifficultyCategory {
    #[serde(rename = "BASIC")]
    #[strum(serialize = "BASIC")]
    Basic = 0,

    #[serde(rename = "ADVANCED")]
    #[strum(serialize = "ADVANCED")]
    Advanced = 1,

    #[serde(rename = "EXPERT")]
    #[strum(serialize = "EXPERT")]
    Expert = 2,

    #[serde(rename = "MASTER")]
    #[strum(serialize = "MASTER")]
    Master = 3,

    #[serde(rename = "Re:MASTER")]
    #[strum(serialize = "Re:MASTER")]
    ReMaster = 4,
}

impl DifficultyCategory {
    pub fn as_u8(self) -> u8 {
        self as u8
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

    /// Parse from lowercase songdb JSON value ("basic", "advanced", "expert", "master", "remaster").
    pub fn from_lowercase(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "basic" => Some(Self::Basic),
            "advanced" => Some(Self::Advanced),
            "expert" => Some(Self::Expert),
            "master" => Some(Self::Master),
            "remaster" => Some(Self::ReMaster),
            _ => None,
        }
    }

    /// Return lowercase songdb JSON value ("basic", "advanced", "expert", "master", "remaster").
    pub const fn as_lowercase(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
            Self::Master => "master",
            Self::ReMaster => "remaster",
        }
    }

    /// Parse from internal-level spreadsheet abbreviations ("EXP", "MAS", "ReMAS").
    pub fn from_sheet_abbreviation(s: &str) -> Option<Self> {
        match s.trim() {
            "EXP" => Some(Self::Expert),
            "MAS" => Some(Self::Master),
            "ReMAS" => Some(Self::ReMaster),
            _ => None,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub played_at_unixtime: Option<i64>,
    pub track: Option<u8>,
    pub played_at: Option<String>,
    pub credit_play_count: Option<u32>,

    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub level: Option<String>,

    pub achievement_percent: Option<f32>,
    pub achievement_new_record: bool,
    pub first_play: bool,
    pub score_rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDetail {
    pub title: String,
    pub chart_type: ChartType,
    pub difficulties: Vec<ParsedSongDifficultyDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDifficultyDetail {
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub chart_type: ChartType,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayerData {
    pub user_name: String,
    pub rating: u32,
    pub current_version_play_count: u32,
    pub total_play_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScoreEntry {
    pub title: String,
    pub chart_type: String,
    pub diff_category: String,
    pub level: String,
    pub achievement_x10000: Option<i64>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlayRecord {
    pub played_at_unixtime: i64,
    pub played_at: Option<String>,
    pub track: Option<i32>,
    pub title: String,
    pub chart_type: String,
    pub diff_category: Option<String>,
    pub level: Option<String>,
    pub achievement_x10000: Option<i64>,
    pub score_rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub credit_play_count: Option<i32>,
    pub achievement_new_record: Option<i32>,
    pub first_play: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResponse {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub achievement_x10000: Option<i64>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
    pub internal_level: Option<f32>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub rating_points: Option<u32>,
    pub bucket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayRecordResponse {
    pub played_at_unixtime: i64,
    pub played_at: Option<String>,
    pub track: Option<i32>,
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub level: Option<String>,
    pub achievement_x10000: Option<i64>,
    pub score_rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub credit_play_count: Option<i32>,
    pub achievement_new_record: Option<i32>,
    pub first_play: Option<i32>,
    pub internal_level: Option<f32>,
    pub rating_points: Option<u32>,
    pub bucket: Option<String>,
}

// Song data index for rating calculations
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SongBucket {
    New,
    Old,
}

#[derive(Debug, Clone)]
pub struct SongDataIndex {
    map: HashMap<SongKey, f32>,
    song_version: HashMap<String, String>,
    song_image_name: HashMap<String, String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SongKey {
    title_norm: String,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDataRoot {
    pub songs: Vec<SongDataSong>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDataSong {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "imageName", skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,
    pub sheets: Vec<SongDataSheet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDataSheet {
    #[serde(rename = "type")]
    pub sheet_type: String,
    pub difficulty: String,
    pub level: String,
    #[serde(rename = "internalLevel", skip_serializing_if = "Option::is_none")]
    pub internal_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_level: Option<String>,
}

impl SongDataIndex {
    pub fn empty() -> Self {
        Self {
            map: HashMap::new(),
            song_version: HashMap::new(),
            song_image_name: HashMap::new(),
        }
    }

    pub fn internal_level(
        &self,
        title: &str,
        chart_type: ChartType,
        diff_category: DifficultyCategory,
    ) -> Option<f32> {
        let key = SongKey {
            title_norm: normalize_title(title),
            chart_type,
            diff_category,
        };
        self.map.get(&key).copied()
    }

    pub fn bucket(&self, title: &str) -> Option<SongBucket> {
        let title_norm = normalize_title(title);
        let version = self.song_version.get(&title_norm)?;
        if is_new_version(version) {
            Some(SongBucket::New)
        } else {
            Some(SongBucket::Old)
        }
    }

    pub fn image_name(&self, title: &str) -> Option<&str> {
        let title_norm = normalize_title(title);
        self.song_image_name.get(&title_norm).map(|s| s.as_str())
    }

    pub fn version(&self, title: &str) -> Option<&str> {
        let title_norm = normalize_title(title);
        self.song_version.get(&title_norm).map(|s| s.as_str())
    }

    pub fn from_root(root: SongDataRoot) -> Self {
        let mut map = HashMap::new();
        let mut song_version = HashMap::new();
        let mut song_image_name = HashMap::new();

        for song in root.songs {
            let title_norm = normalize_title(&song.title);

            if let Some(version) = song.version.as_deref() {
                let version = version.trim();
                if !version.is_empty() {
                    song_version
                        .entry(title_norm.clone())
                        .or_insert_with(|| version.to_string());
                }
            }

            if let Some(image_name) = song.image_name.as_deref() {
                let image_name = image_name.trim();
                if !image_name.is_empty() {
                    song_image_name
                        .entry(title_norm.clone())
                        .or_insert_with(|| image_name.to_string());
                }
            }

            for sheet in song.sheets {
                let Some(internal_str) = &sheet.internal_level else {
                    continue;
                };

                let Ok(internal_value) = internal_str.trim().parse::<f32>() else {
                    continue;
                };

                let Some(chart_type) = ChartType::from_lowercase(&sheet.sheet_type) else {
                    continue;
                };
                let Some(diff_category) = DifficultyCategory::from_lowercase(&sheet.difficulty)
                else {
                    continue;
                };

                map.insert(
                    SongKey {
                        title_norm: title_norm.clone(),
                        chart_type,
                        diff_category,
                    },
                    internal_value,
                );
            }
        }

        Self {
            map,
            song_version,
            song_image_name,
        }
    }
}

fn normalize_title(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

fn is_new_version(version: &str) -> bool {
    matches!(version, "PRiSM PLUS" | "CiRCLE")
}
