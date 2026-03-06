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
    pub genre: String,
    pub artist: String,
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
    pub region: SongChartRegion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongChartRegion {
    pub jp: bool,
    pub intl: bool,
}

#[derive(Debug, Clone)]
pub struct SongInternalLevelIndex {
    map: HashMap<SongChartLookupKey, f32>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SongChartLookupKey {
    title: String,
    genre: String,
    artist: String,
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
        genre: &str,
        artist: &str,
        chart_type: ChartType,
        diff_category: DifficultyCategory,
    ) -> Option<f32> {
        let key = SongChartLookupKey {
            title: normalize_identity_component(title),
            genre: normalize_identity_component(genre),
            artist: normalize_identity_component(artist),
            chart_type,
            diff_category,
        };
        self.map.get(&key).copied()
    }

    pub fn from_catalog(catalog: SongCatalog) -> Self {
        let mut map = HashMap::new();

        for song in catalog.songs {
            let title = normalize_identity_component(&song.title);
            let genre = normalize_identity_component(&song.genre);
            let artist = normalize_identity_component(&song.artist);

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
                        title: title.clone(),
                        genre: genre.clone(),
                        artist: artist.clone(),
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

fn normalize_identity_component(s: &str) -> String {
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> SongCatalogChart {
        SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "master".to_string(),
            level: "13+".to_string(),
            version_name: None,
            internal_level: Some("13.7".to_string()),
            user_level: None,
            region: SongChartRegion {
                jp: true,
                intl: true,
            },
        }
    }

    #[test]
    fn internal_level_index_uses_trim_only_identity() {
        let index = SongInternalLevelIndex::from_catalog(SongCatalog {
            songs: vec![SongCatalogSong {
                title: " Song A ".to_string(),
                genre: " maimai ".to_string(),
                artist: " Artist ".to_string(),
                image_name: None,
                sheets: vec![chart()],
            }],
        });

        assert_eq!(
            index.internal_level(
                "Song A",
                "maimai",
                "Artist",
                ChartType::Std,
                DifficultyCategory::Master
            ),
            Some(13.7)
        );
    }

    #[test]
    fn internal_level_index_keeps_case_distinct() {
        let index = SongInternalLevelIndex::from_catalog(SongCatalog {
            songs: vec![
                SongCatalogSong {
                    title: "Link".to_string(),
                    genre: "maimai".to_string(),
                    artist: "".to_string(),
                    image_name: None,
                    sheets: vec![chart()],
                },
                SongCatalogSong {
                    title: "link".to_string(),
                    genre: "maimai".to_string(),
                    artist: "".to_string(),
                    image_name: None,
                    sheets: vec![SongCatalogChart {
                        internal_level: Some("14.0".to_string()),
                        ..chart()
                    }],
                },
            ],
        });

        assert_eq!(
            index.internal_level(
                "Link",
                "maimai",
                "",
                ChartType::Std,
                DifficultyCategory::Master
            ),
            Some(13.7)
        );
        assert_eq!(
            index.internal_level(
                "link",
                "maimai",
                "",
                ChartType::Std,
                DifficultyCategory::Master
            ),
            Some(14.0)
        );
    }
}
