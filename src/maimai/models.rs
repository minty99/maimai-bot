use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChartType {
    Std,
    Dx,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, EnumString, Display,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub playlog_idx: Option<String>,
    pub track: Option<u8>,
    pub played_at: Option<String>,

    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub level: Option<String>,

    pub achievement_percent: Option<f32>,
    pub score_rank: Option<ScoreRank>,
    pub fc: Option<String>,
    pub sync: Option<String>,
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
    pub fc: Option<String>,
    pub sync: Option<String>,
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
