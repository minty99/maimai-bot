use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eyre::WrapErr;
use maimai_auth::intl;
use maimai_parsers::parse_scores_html;
use models::{ChartType, MaimaiVersion};
use serde::Deserialize;

use crate::{SheetRow, SongRow, normalize_identity_component, normalize_song_title_value};

const INTL_VERSION_SEARCH_URL: &str =
    "https://maimaidx-eng.com/maimai-mobile/record/musicVersion/search/";
const DUPLICATE_RESOLUTION_JSON: &str = include_str!("data/intl_version_duplicate_resolution.json");

pub type SheetVersionMap = HashMap<String, HashMap<ChartType, String>>;

#[derive(Debug, Deserialize)]
struct DuplicateResolutionData {
    #[serde(default)]
    overrides: Vec<DuplicateResolutionOverride>,
}

#[derive(Debug, Clone, Deserialize)]
struct DuplicateResolutionOverride {
    title: String,
    version: String,
    chart_type: ChartType,
    genre: String,
    artist: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct OverrideLookupKey {
    title: String,
    version: String,
    chart_type: ChartType,
}

#[derive(Debug, Clone)]
struct SongVersionResolver {
    candidates_by_title: HashMap<String, Vec<SongCandidate>>,
    overrides: HashMap<OverrideLookupKey, DuplicateResolutionOverride>,
}

#[derive(Debug, Clone)]
struct SongCandidate {
    song_id: String,
    title: String,
    genre: String,
    artist: String,
    chart_types: HashSet<ChartType>,
}

pub async fn fetch_intl_sheet_versions(
    sega_id: &str,
    sega_password: &str,
    songs: &[SongRow],
    sheets: &[SheetRow],
) -> eyre::Result<SheetVersionMap> {
    let client = reqwest::Client::builder()
        .default_headers(intl::default_mobile_headers()?)
        .redirect(reqwest::redirect::Policy::limited(10))
        .cookie_store(true)
        .build()
        .wrap_err("build INTL sheet version client")?;

    intl::ensure_logged_in(&client, sega_id, sega_password)
        .await
        .wrap_err("ensure INTL login")?;

    let resolver = SongVersionResolver::new(songs, sheets)?;
    let mut out: SheetVersionMap = HashMap::new();
    let mut seen = HashSet::new();

    let mut version_index = 0u8;
    while let Some(version) = MaimaiVersion::from_index(version_index) {
        let rows = fetch_rows_for_version(&client, version, &resolver).await?;
        for (song_id, chart_type) in rows {
            let dedup_key = format!("{song_id}|{}", chart_type.as_str());
            if !seen.insert(dedup_key.clone()) {
                return Err(eyre::eyre!(
                    "duplicate sheet version key detected: {dedup_key}"
                ));
            }

            out.entry(song_id)
                .or_default()
                .insert(chart_type, version.as_str().to_string());
        }

        version_index = version_index.saturating_add(1);
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    Ok(out)
}

impl SongVersionResolver {
    fn new(songs: &[SongRow], sheets: &[SheetRow]) -> eyre::Result<Self> {
        let parsed: DuplicateResolutionData = serde_json::from_str(DUPLICATE_RESOLUTION_JSON)
            .wrap_err("parse intl_version_duplicate_resolution.json")?;

        let mut chart_types_by_song_id: HashMap<&str, HashSet<ChartType>> = HashMap::new();
        for sheet in sheets.iter().filter(|sheet| sheet.source.is_official()) {
            chart_types_by_song_id
                .entry(sheet.song_id.as_str())
                .or_default()
                .insert(sheet.sheet_type);
        }

        let mut candidates_by_title: HashMap<String, Vec<SongCandidate>> = HashMap::new();
        for song in songs {
            if !chart_types_by_song_id.contains_key(song.song_id.as_str()) {
                continue;
            }
            let normalized_title = normalize_song_title_value(&song.title);
            candidates_by_title
                .entry(normalized_title)
                .or_default()
                .push(SongCandidate {
                    song_id: song.song_id.clone(),
                    title: song.title.clone(),
                    genre: song.category.clone(),
                    artist: song.artist.clone(),
                    chart_types: chart_types_by_song_id
                        .get(song.song_id.as_str())
                        .cloned()
                        .unwrap_or_default(),
                });
        }

        let overrides = parsed
            .overrides
            .into_iter()
            .map(|override_row| {
                let key = OverrideLookupKey {
                    title: normalize_song_title_value(&override_row.title),
                    version: normalize_identity_component(&override_row.version),
                    chart_type: override_row.chart_type,
                };
                let normalized = DuplicateResolutionOverride {
                    title: key.title.clone(),
                    version: key.version.clone(),
                    chart_type: override_row.chart_type,
                    genre: normalize_identity_component(&override_row.genre),
                    artist: normalize_identity_component(&override_row.artist),
                };
                (key, normalized)
            })
            .collect();

        Ok(Self {
            candidates_by_title,
            overrides,
        })
    }

    fn resolve_song_id(
        &self,
        title: &str,
        version_name: &str,
        chart_type: ChartType,
    ) -> eyre::Result<String> {
        let title = normalize_song_title_value(title);
        let version_name = normalize_identity_component(version_name);

        let candidates = self
            .candidates_by_title
            .get(&title)
            .cloned()
            .unwrap_or_default();

        if candidates.is_empty() {
            return Err(eyre::eyre!(
                "no official song matches INTL version title '{}' ({})",
                title,
                version_name
            ));
        }

        if candidates.len() == 1 {
            return Ok(candidates[0].song_id.clone());
        }

        let chart_type_matches = candidates
            .iter()
            .filter(|candidate| candidate.chart_types.contains(&chart_type))
            .collect::<Vec<_>>();
        if chart_type_matches.len() == 1 {
            return Ok(chart_type_matches[0].song_id.clone());
        }

        let override_key = OverrideLookupKey {
            title: title.clone(),
            version: version_name.clone(),
            chart_type,
        };
        let Some(override_row) = self.overrides.get(&override_key) else {
            return Err(eyre::eyre!(
                "ambiguous INTL version title '{}' ({}, {}) without override",
                title,
                version_name,
                chart_type.as_str()
            ));
        };

        let matched = candidates
            .iter()
            .filter(|candidate| {
                candidate.genre == override_row.genre && candidate.artist == override_row.artist
            })
            .collect::<Vec<_>>();

        match matched.as_slice() {
            [candidate] => Ok(candidate.song_id.clone()),
            [] => Err(eyre::eyre!(
                "override for '{}' ({}, {}) did not match any official song: ({}, {}, {})",
                title,
                version_name,
                chart_type.as_str(),
                title,
                override_row.genre,
                override_row.artist
            )),
            _ => Err(eyre::eyre!(
                "override for '{}' ({}, {}) matched multiple official songs: ({}, {}, {})",
                title,
                version_name,
                chart_type.as_str(),
                title,
                override_row.genre,
                override_row.artist
            )),
        }
    }
}

async fn fetch_rows_for_version(
    client: &reqwest::Client,
    version: MaimaiVersion,
    resolver: &SongVersionResolver,
) -> eyre::Result<Vec<(String, ChartType)>> {
    let response = client
        .get(INTL_VERSION_SEARCH_URL)
        .query(&[
            ("version", version.as_index().to_string()),
            ("diff", "0".to_string()),
        ])
        .send()
        .await
        .wrap_err_with(|| format!("fetch INTL version page for {}", version.as_str()))?
        .error_for_status()
        .wrap_err_with(|| format!("INTL version page status for {}", version.as_str()))?;

    let final_url = response.url().clone();
    let html = response
        .text()
        .await
        .wrap_err_with(|| format!("read INTL version html for {}", version.as_str()))?;

    if intl::looks_like_login_or_expired(&final_url, &html) {
        return Err(eyre::eyre!(
            "INTL version page returned login/error for {}",
            version.as_str()
        ));
    }

    parse_rows(&html, version.as_str(), resolver)
}

fn parse_rows(
    html: &str,
    version_name: &str,
    resolver: &SongVersionResolver,
) -> eyre::Result<Vec<(String, ChartType)>> {
    let entries =
        parse_scores_html(html, 0).wrap_err("parse score blocks from INTL musicVersion page")?;

    entries
        .into_iter()
        .map(|entry| {
            resolver
                .resolve_song_id(&entry.title, version_name, entry.chart_type)
                .map(|song_id| (song_id, entry.chart_type))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SheetSource, song_id_from_identity_parts};

    fn official_song(title: &str, genre: &str, artist: &str) -> SongRow {
        SongRow {
            song_id: song_id_from_identity_parts(title, genre, artist),
            category: genre.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        }
    }

    fn official_sheet(song_id: &str, chart_type: ChartType) -> SheetRow {
        SheetRow {
            song_id: song_id.to_string(),
            sheet_type: chart_type,
            difficulty: models::DifficultyCategory::Basic,
            level: "6".to_string(),
            source: SheetSource::Official,
        }
    }

    #[test]
    fn resolves_unique_title_without_override() {
        let song = official_song("Technicians High", "maimai", "");
        let resolver = SongVersionResolver::new(
            std::slice::from_ref(&song),
            &[official_sheet(&song.song_id, ChartType::Std)],
        )
        .expect("resolver");

        assert_eq!(
            resolver
                .resolve_song_id("Technicians High", "ORANGE", ChartType::Std)
                .expect("resolve"),
            song.song_id
        );
    }

    #[test]
    fn resolves_duplicate_title_with_chart_type_before_override() {
        let std_song = official_song("Link", "maimai", "");
        let dx_song = official_song("Link", "niconico＆VOCALOID™", "");
        let resolver = SongVersionResolver::new(
            &[std_song.clone(), dx_song.clone()],
            &[
                official_sheet(&std_song.song_id, ChartType::Std),
                official_sheet(&dx_song.song_id, ChartType::Dx),
            ],
        )
        .expect("resolver");

        assert_eq!(
            resolver
                .resolve_song_id("Link", "ORANGE", ChartType::Dx)
                .expect("resolve"),
            dx_song.song_id
        );
    }

    #[test]
    fn duplicate_title_requires_override_when_still_ambiguous() {
        let songs = vec![
            official_song("Link", "maimai", ""),
            official_song("Link", "niconico＆VOCALOID™", ""),
        ];
        let sheets = songs
            .iter()
            .map(|song| official_sheet(&song.song_id, ChartType::Std))
            .collect::<Vec<_>>();
        let resolver = SongVersionResolver::new(&songs, &sheets).expect("resolver");

        let err = resolver
            .resolve_song_id("Link", "BUDDiES", ChartType::Std)
            .expect_err("expected ambiguity");
        assert!(
            err.to_string().contains("without override"),
            "unexpected error: {err:#}"
        );
    }
}
