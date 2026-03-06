use eyre::WrapErr;
use models::{ChartType, DifficultyCategory, MaimaiVersion};
use serde::Deserialize;

use crate::{SheetRow, SheetSource, SongRow, normalize_identity_component, sha256_hex};

const INTL_ONLY_DATA_JSON: &str = include_str!("data/intl_only.json");

#[derive(Debug)]
pub(crate) struct IntlOnlyRows {
    pub(crate) songs: Vec<SongRow>,
    pub(crate) sheets: Vec<SheetRow>,
}

#[derive(Debug, Deserialize)]
struct IntlOnlyData {
    #[serde(default)]
    songs: Vec<IntlOnlySong>,
}

#[derive(Debug, Deserialize)]
struct IntlOnlySong {
    song_id: String,
    title: String,
    category: String,
    artist: String,
    image_url: String,
    #[serde(default)]
    sheets: Vec<IntlOnlySheet>,
}

#[derive(Debug, Deserialize)]
struct IntlOnlySheet {
    #[serde(rename = "type")]
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    level: String,
    version: MaimaiVersion,
    #[serde(default)]
    internal_level: Option<f32>,
}

pub(crate) fn load_intl_only_rows() -> eyre::Result<IntlOnlyRows> {
    let parsed: IntlOnlyData =
        serde_json::from_str(INTL_ONLY_DATA_JSON).wrap_err("parse intl_only.json")?;
    map_to_rows(parsed)
}

fn map_to_rows(parsed: IntlOnlyData) -> eyre::Result<IntlOnlyRows> {
    let mut songs = Vec::new();
    let mut sheets = Vec::new();

    for song in parsed.songs {
        let song_id = song.song_id.trim().to_string();
        let title = normalize_identity_component(&song.title);
        let category = normalize_identity_component(&song.category);
        let artist = normalize_identity_component(&song.artist);
        if song_id.is_empty() {
            return Err(eyre::eyre!("intl_only song requires non-empty song_id"));
        }
        if category.is_empty() {
            return Err(eyre::eyre!(
                "intl_only song '{}' requires non-empty category",
                title
            ));
        }
        let image_url = song.image_url.trim().to_string();
        if !image_url.starts_with("http://") && !image_url.starts_with("https://") {
            return Err(eyre::eyre!(
                "intl_only song '{}' has invalid image_url: {}",
                title,
                image_url
            ));
        }
        let image_name = format!("{}.png", sha256_hex(&image_url));

        songs.push(SongRow {
            song_id: song_id.clone(),
            category,
            title: title.clone(),
            artist,
            image_name,
            image_url,
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        });

        for sheet in song.sheets {
            let level = sheet.level.trim();
            if level.is_empty() {
                return Err(eyre::eyre!("intl_only '{}' has empty level", title));
            }

            sheets.push(SheetRow {
                song_id: song_id.clone(),
                sheet_type: sheet.chart_type,
                difficulty: sheet.difficulty,
                level: level.to_string(),
                source: SheetSource::IntlOnly {
                    version_name: sheet.version.as_str().to_string(),
                    internal_level: sheet.internal_level.map(|value| format!("{value:.1}")),
                },
            });
        }
    }

    Ok(IntlOnlyRows { songs, sheets })
}

#[cfg(test)]
mod tests {
    use super::{IntlOnlyData, map_to_rows};
    use models::{ChartType, DifficultyCategory, MaimaiVersion};

    #[test]
    fn intl_only_requires_image_url_field() {
        let json = r#"
        {
          "songs": [
            {
              "song_id": "test-song",
              "title": "Test Song",
              "category": "maimai",
              "artist": "",
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash"
                }
              ]
            }
          ]
        }
        "#;
        let parsed = serde_json::from_str::<IntlOnlyData>(json);
        assert!(parsed.is_err(), "image_url should be required");
    }

    #[test]
    fn intl_only_sheet_uses_typed_fields() {
        let json = r#"
        {
          "songs": [
            {
              "song_id": "test-song",
              "title": "Test Song",
              "category": "maimai",
              "artist": "",
              "image_url": "https://example.com/test.png",
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash"
                }
              ]
            }
          ]
        }
        "#;
        let parsed: IntlOnlyData = serde_json::from_str(json).expect("parse intl_only test json");
        let sheet = &parsed.songs[0].sheets[0];
        assert_eq!(sheet.chart_type, ChartType::Std);
        assert_eq!(sheet.difficulty, DifficultyCategory::Basic);
        assert_eq!(sheet.version, MaimaiVersion::Splash);
    }

    #[test]
    fn intl_only_allows_empty_title_and_artist() {
        let json = r#"
        {
          "songs": [
            {
              "song_id": "test-song",
              "title": "",
              "category": "niconico＆ボーカロイド",
              "artist": "",
              "image_url": "https://example.com/test.png",
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash"
                }
              ]
            }
          ]
        }
        "#;
        let parsed: IntlOnlyData = serde_json::from_str(json).expect("parse intl_only test json");
        let rows = map_to_rows(parsed).expect("map rows");
        assert_eq!(rows.songs[0].title, "");
        assert_eq!(rows.songs[0].artist, "");
    }
}
