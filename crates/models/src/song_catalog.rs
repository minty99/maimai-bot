use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{ChartType, DifficultyCategory};

#[derive(Debug, Serialize, Deserialize)]
pub struct SongCatalog {
    pub songs: Vec<SongCatalogSong>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongCatalogSong {
    pub title: String,
    #[serde(rename = "imageName", skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,
    pub sheets: Vec<SongCatalogChart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongCatalogChart {
    #[serde(rename = "type")]
    pub chart_type: String,
    pub difficulty: String,
    pub level: String,
    #[serde(rename = "version", skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    #[serde(rename = "internalLevel", skip_serializing_if = "Option::is_none")]
    pub internal_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_level: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SongInternalLevelIndex {
    map: HashMap<SongChartLookupKey, f32>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SongChartLookupKey {
    title_norm: String,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
}

impl SongInternalLevelIndex {
    pub fn empty() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn internal_level(
        &self,
        title: &str,
        chart_type: ChartType,
        diff_category: DifficultyCategory,
    ) -> Option<f32> {
        let key = SongChartLookupKey {
            title_norm: normalize_title(title),
            chart_type,
            diff_category,
        };
        self.map.get(&key).copied()
    }

    pub fn from_catalog(catalog: SongCatalog) -> Self {
        let mut map = HashMap::new();

        for song in catalog.songs {
            let title_norm = normalize_title(&song.title);

            for sheet in song.sheets {
                let Some(chart_type) = ChartType::from_lowercase(&sheet.chart_type) else {
                    continue;
                };

                let Some(internal_str) = &sheet.internal_level else {
                    continue;
                };

                let Ok(internal_value) = internal_str.trim().parse::<f32>() else {
                    continue;
                };

                let Some(diff_category) = DifficultyCategory::from_lowercase(&sheet.difficulty)
                else {
                    continue;
                };

                map.insert(
                    SongChartLookupKey {
                        title_norm: title_norm.clone(),
                        chart_type,
                        diff_category,
                    },
                    internal_value,
                );
            }
        }

        Self { map }
    }
}

fn normalize_title(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}
