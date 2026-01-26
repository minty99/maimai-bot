use eyre::WrapErr;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tracing::warn;

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
    chart_type: String,
    diff_category: String,
}

#[derive(Debug, Deserialize)]
struct SongDataRoot {
    songs: Vec<SongDataSong>,
}

#[derive(Debug, Deserialize)]
struct SongDataSong {
    title: String,
    version: Option<String>,
    #[serde(rename = "imageName")]
    image_name: Option<String>,
    sheets: Vec<SongDataSheet>,
}

#[derive(Debug, Deserialize)]
struct SongDataSheet {
    #[serde(rename = "type")]
    sheet_type: String,
    difficulty: String,
    #[serde(rename = "internalLevelValue")]
    internal_level_value: f32,
}

impl SongDataIndex {
    pub fn load_from_default_locations() -> eyre::Result<Option<Self>> {
        let path = PathBuf::from("fetched_data/data.json");

        if let Some(idx) = Self::load_from_path(&path)? {
            return Ok(Some(idx));
        }

        warn!(
            "song data not found at {} (non-fatal); skipping song data",
            path.display()
        );
        Ok(None)
    }

    pub fn load_from_path(path: &Path) -> eyre::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let file =
            File::open(path).wrap_err_with(|| format!("open song data: {}", path.display()))?;
        let reader = BufReader::new(file);
        let root: SongDataRoot = serde_json::from_reader(reader)
            .wrap_err_with(|| format!("parse song data: {}", path.display()))?;
        Ok(Some(Self::from_root(root)))
    }

    pub fn internal_level(
        &self,
        title: &str,
        chart_type: &str,
        diff_category: &str,
    ) -> Option<f32> {
        let key = SongKey {
            title_norm: normalize_title(title),
            chart_type: chart_type.to_string(),
            diff_category: diff_category.to_string(),
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

    fn from_root(root: SongDataRoot) -> Self {
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
                let internal = sheet.internal_level_value;

                let Some(chart_type) = map_chart_type(&sheet.sheet_type) else {
                    continue;
                };
                let Some(diff_category) = map_diff_category(&sheet.difficulty) else {
                    continue;
                };

                map.insert(
                    SongKey {
                        title_norm: title_norm.clone(),
                        chart_type: chart_type.to_string(),
                        diff_category: diff_category.to_string(),
                    },
                    internal,
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

fn map_chart_type(sheet_type: &str) -> Option<&'static str> {
    match sheet_type.trim().to_ascii_lowercase().as_str() {
        "std" => Some("STD"),
        "dx" => Some("DX"),
        _ => None,
    }
}

fn map_diff_category(difficulty: &str) -> Option<&'static str> {
    match difficulty.trim().to_ascii_lowercase().as_str() {
        "basic" => Some("BASIC"),
        "advanced" => Some("ADVANCED"),
        "expert" => Some("EXPERT"),
        "master" => Some("MASTER"),
        "remaster" => Some("Re:MASTER"),
        _ => None,
    }
}

fn is_new_version(version: &str) -> bool {
    matches!(version, "PRiSM PLUS" | "CiRCLE")
}
