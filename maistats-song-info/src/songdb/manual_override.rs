use std::collections::HashSet;

use eyre::WrapErr;
use models::{
    ChartType, DifficultyCategory, MaimaiVersion, SongAliases, SongChartRegion, SongGenre,
};
use serde::Deserialize;

use super::{
    SheetRow, SheetSource, SongIdentity, SongRow, normalize_identity_component,
    normalize_song_title_value, sha256_hex,
};

const MANUAL_OVERRIDE_DATA_JSON: &str = include_str!("data/manual_override.json");

#[derive(Debug)]
pub(crate) struct ManualOverrideRows {
    pub(crate) songs: Vec<SongRow>,
    pub(crate) sheets: Vec<SheetRow>,
    pub(crate) aliases: Vec<(SongIdentity, SongAliases)>,
    pub(crate) overridden_titles: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct ManualOverrideData {
    #[serde(default)]
    songs: Vec<ManualOverrideSong>,
}

#[derive(Debug, Deserialize)]
struct ManualOverrideSong {
    title: String,
    genre: SongGenre,
    artist: String,
    image_url: String,
    #[serde(default)]
    aliases: SongAliases,
    #[serde(default)]
    sheets: Vec<ManualOverrideSheet>,
}

#[derive(Debug, Deserialize)]
struct ManualOverrideSheet {
    #[serde(rename = "type")]
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    level: String,
    version: MaimaiVersion,
    #[serde(default)]
    internal_level: Option<f32>,
    region: SongChartRegion,
}

pub(crate) fn load_manual_override_rows() -> eyre::Result<ManualOverrideRows> {
    let parsed: ManualOverrideData =
        serde_json::from_str(MANUAL_OVERRIDE_DATA_JSON).wrap_err("parse manual_override.json")?;
    map_to_rows(parsed)
}

fn map_to_rows(parsed: ManualOverrideData) -> eyre::Result<ManualOverrideRows> {
    let mut songs = Vec::new();
    let mut sheets = Vec::new();
    let mut aliases = Vec::new();
    let mut overridden_titles = HashSet::new();

    for song in parsed.songs {
        let title = normalize_song_title_value(&song.title);
        let artist = normalize_identity_component(&song.artist);
        let identity = SongIdentity::new(&title, song.genre, &artist);
        let image_url = song.image_url.trim().to_string();
        if !image_url.starts_with("http://") && !image_url.starts_with("https://") {
            return Err(eyre::eyre!(
                "manual_override song '{}' has invalid image_url: {}",
                title,
                image_url
            ));
        }
        let image_name = format!("{}.png", sha256_hex(&image_url));
        overridden_titles.insert(normalize_song_title_value(&identity.title));
        if !song.aliases.is_empty() {
            aliases.push((identity.clone(), song.aliases.clone()));
        }

        songs.push(SongRow {
            identity: identity.clone(),
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
                return Err(eyre::eyre!("manual_override '{}' has empty level", title));
            }

            sheets.push(SheetRow {
                song_identity: identity.clone(),
                sheet_type: sheet.chart_type,
                difficulty: sheet.difficulty,
                level: level.to_string(),
                source: SheetSource::ManualOverride {
                    version_name: sheet.version.as_str().to_string(),
                    internal_level: sheet.internal_level.map(|value| format!("{value:.1}")),
                    region: sheet.region,
                },
            });
        }
    }

    Ok(ManualOverrideRows {
        songs,
        sheets,
        aliases,
        overridden_titles,
    })
}

#[cfg(test)]
mod tests {
    use super::{ManualOverrideData, map_to_rows};
    use models::{ChartType, DifficultyCategory, MaimaiVersion, SongGenre};

    #[test]
    fn manual_override_requires_image_url_field() {
        let json = r#"
        {
          "songs": [
            {
              "title": "Test Song",
              "genre": "maimai",
              "artist": "",
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash",
                  "region": { "jp": false, "intl": true }
                }
              ]
            }
          ]
        }
        "#;
        let parsed = serde_json::from_str::<ManualOverrideData>(json);
        assert!(parsed.is_err(), "image_url should be required");
    }

    #[test]
    fn manual_override_sheet_uses_typed_fields() {
        let json = r#"
        {
          "songs": [
            {
              "title": "Test Song",
              "genre": "maimai",
              "artist": "",
              "image_url": "https://example.com/test.png",
              "aliases": {
                "en": ["Alias"],
                "ko": ["별칭"]
              },
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash",
                  "region": { "jp": false, "intl": true }
                }
              ]
            }
          ]
        }
        "#;
        let parsed: ManualOverrideData =
            serde_json::from_str(json).expect("parse manual_override test json");
        let song = &parsed.songs[0];
        let sheet = &song.sheets[0];
        assert_eq!(song.genre, SongGenre::Maimai);
        assert_eq!(sheet.chart_type, ChartType::Std);
        assert_eq!(sheet.difficulty, DifficultyCategory::Basic);
        assert_eq!(sheet.version, MaimaiVersion::Splash);
        assert!(!sheet.region.jp);
        assert!(sheet.region.intl);
        assert_eq!(song.aliases.en, vec!["Alias".to_string()]);
        assert_eq!(song.aliases.ko, vec!["별칭".to_string()]);
    }

    #[test]
    fn manual_override_allows_empty_title_and_artist() {
        let json = r#"
        {
          "songs": [
            {
              "title": "",
              "genre": "niconico＆VOCALOID™",
              "artist": "",
              "image_url": "https://example.com/test.png",
              "sheets": [
                {
                  "type": "STD",
                  "difficulty": "BASIC",
                  "level": "6",
                  "version": "Splash",
                  "region": { "jp": false, "intl": true }
                }
              ]
            }
          ]
        }
        "#;
        let parsed: ManualOverrideData =
            serde_json::from_str(json).expect("parse manual_override test json");
        let rows = map_to_rows(parsed).expect("map rows");
        assert_eq!(rows.songs[0].identity.title, "");
        assert_eq!(rows.songs[0].identity.artist, "");
    }

    #[test]
    fn manual_override_keeps_aliases_for_empty_title_song() {
        let json = r#"
        {
          "songs": [
            {
              "title": "",
              "genre": "POPS＆ANIME",
              "artist": "x0o0x_",
              "image_url": "https://example.com/test.png",
              "aliases": {
                "en": ["empty", "blank"],
                "ko": ["공백", "키사라기역"]
              },
              "sheets": [
                {
                  "type": "DX",
                  "difficulty": "BASIC",
                  "level": "3",
                  "version": "UNiVERSE PLUS",
                  "region": { "jp": true, "intl": true }
                }
              ]
            }
          ]
        }
        "#;
        let parsed: ManualOverrideData =
            serde_json::from_str(json).expect("parse manual_override test json");
        let rows = map_to_rows(parsed).expect("map rows");
        assert_eq!(rows.aliases.len(), 1);
        assert_eq!(rows.aliases[0].0.title, "");
        assert_eq!(
            rows.aliases[0].1.en,
            vec!["empty".to_string(), "blank".to_string()]
        );
        assert_eq!(
            rows.aliases[0].1.ko,
            vec!["공백".to_string(), "키사라기역".to_string()]
        );
    }
}
