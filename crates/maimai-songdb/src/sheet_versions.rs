use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eyre::WrapErr;
use maimai_auth::intl;
use maimai_parsers::parse_scores_html;
use models::{ChartType, MaimaiVersion, SongGenre};
use serde::Deserialize;

use crate::{
    SheetRow, SongIdentity, SongRow, normalize_identity_component, normalize_song_title_value,
};

const INTL_VERSION_SEARCH_URL: &str =
    "https://maimaidx-eng.com/maimai-mobile/record/musicVersion/search/";
const DUPLICATE_RESOLUTION_JSON: &str = include_str!("data/intl_version_duplicate_resolution.json");

pub type SheetVersionMap = HashMap<SongIdentity, HashMap<ChartType, String>>;

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
    genre: SongGenre,
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
    intl_only_titles: HashSet<String>,
    overrides: HashMap<OverrideLookupKey, DuplicateResolutionOverride>,
}

#[derive(Debug, Clone)]
struct SongCandidate {
    identity: SongIdentity,
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
        for (song_identity, chart_type) in rows {
            let dedup_key = (song_identity.clone(), chart_type);
            if !seen.insert(dedup_key) {
                return Err(eyre::eyre!(
                    "duplicate sheet version key detected: {:?}|{}",
                    song_identity,
                    chart_type.as_str()
                ));
            }

            out.entry(song_identity)
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

        let mut chart_types_by_song_identity: HashMap<&SongIdentity, HashSet<ChartType>> =
            HashMap::new();
        for sheet in sheets.iter().filter(|sheet| sheet.source.is_official()) {
            chart_types_by_song_identity
                .entry(&sheet.song_identity)
                .or_default()
                .insert(sheet.sheet_type);
        }

        let mut candidates_by_title: HashMap<String, Vec<SongCandidate>> = HashMap::new();
        let mut intl_only_titles = HashSet::new();
        for song in songs {
            if !chart_types_by_song_identity.contains_key(&song.identity) {
                intl_only_titles.insert(normalize_song_title_value(&song.identity.title));
                continue;
            }
            let normalized_title = normalize_song_title_value(&song.identity.title);
            candidates_by_title
                .entry(normalized_title)
                .or_default()
                .push(SongCandidate {
                    identity: song.identity.clone(),
                    chart_types: chart_types_by_song_identity
                        .get(&song.identity)
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
                    genre: override_row.genre,
                    artist: normalize_identity_component(&override_row.artist),
                };
                (key, normalized)
            })
            .collect();

        Ok(Self {
            candidates_by_title,
            intl_only_titles,
            overrides,
        })
    }

    fn resolve_song_identity(
        &self,
        title: &str,
        version_name: &str,
        chart_type: ChartType,
    ) -> eyre::Result<Option<SongIdentity>> {
        let title = normalize_song_title_value(title);
        let version_name = normalize_identity_component(version_name);

        let candidates = self
            .candidates_by_title
            .get(&title)
            .cloned()
            .unwrap_or_default();

        if candidates.is_empty() {
            if self.intl_only_titles.contains(&title) {
                return Ok(None);
            }
            return Err(eyre::eyre!(
                "no official song matches INTL version title '{}' ({})",
                title,
                version_name
            ));
        }

        if candidates.len() == 1 {
            return Ok(Some(candidates[0].identity.clone()));
        }

        let chart_type_matches = candidates
            .iter()
            .filter(|candidate| candidate.chart_types.contains(&chart_type))
            .collect::<Vec<_>>();
        if chart_type_matches.len() == 1 {
            return Ok(Some(chart_type_matches[0].identity.clone()));
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
                candidate.identity.genre == override_row.genre
                    && candidate.identity.artist == override_row.artist
            })
            .collect::<Vec<_>>();

        match matched.as_slice() {
            [candidate] => Ok(Some(candidate.identity.clone())),
            [] => Err(eyre::eyre!(
                "override for '{}' ({}, {}) did not match any official song: ({}, {}, {})",
                title,
                version_name,
                chart_type.as_str(),
                title,
                override_row.genre.as_str(),
                override_row.artist
            )),
            _ => Err(eyre::eyre!(
                "override for '{}' ({}, {}) matched multiple official songs: ({}, {}, {})",
                title,
                version_name,
                chart_type.as_str(),
                title,
                override_row.genre.as_str(),
                override_row.artist
            )),
        }
    }
}

async fn fetch_rows_for_version(
    client: &reqwest::Client,
    version: MaimaiVersion,
    resolver: &SongVersionResolver,
) -> eyre::Result<Vec<(SongIdentity, ChartType)>> {
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
) -> eyre::Result<Vec<(SongIdentity, ChartType)>> {
    let entries =
        parse_scores_html(html, 0).wrap_err("parse score blocks from INTL musicVersion page")?;

    entries
        .into_iter()
        .filter_map(|entry| {
            resolver
                .resolve_song_identity(&entry.title, version_name, entry.chart_type)
                .transpose()
                .map(|resolved| resolved.map(|song_identity| (song_identity, entry.chart_type)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SheetSource, SongIdentity, load_official_rows_from_json};

    const OFFICIAL_JP_SONGS_JSON: &str =
        include_str!("../examples/maimai/official/maimai_songs.json");
    const INTL_VERSION1_MAIMAI_PLUS_DIFF0_HTML: &str =
        include_str!("../examples/maimai/intl_version/version1_maimai_plus_diff0.html");
    const INTL_VERSION0_MAIMAI_DIFF0_HTML: &str =
        include_str!("../../maimai-parsers/examples/maimai/scores/version0_maimai_diff0.html");
    const INTL_VERSION4_ORANGE_DIFF0_HTML: &str =
        include_str!("../examples/maimai/intl_version/version4_orange_diff0.html");

    fn official_song(title: &str, genre: &str, artist: &str) -> SongRow {
        let genre = genre.parse::<SongGenre>().expect("known test genre");
        SongRow {
            identity: SongIdentity::new(title, genre, artist),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        }
    }

    fn official_sheet(song_identity: &SongIdentity, chart_type: ChartType) -> SheetRow {
        SheetRow {
            song_identity: song_identity.clone(),
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
            &[official_sheet(&song.identity, ChartType::Std)],
        )
        .expect("resolver");

        assert_eq!(
            resolver
                .resolve_song_identity("Technicians High", "ORANGE", ChartType::Std)
                .expect("resolve"),
            Some(song.identity)
        );
    }

    #[test]
    fn resolves_duplicate_title_with_chart_type_before_override() {
        let std_song = official_song("Link", "maimai", "");
        let dx_song = official_song("Link", "niconico＆ボーカロイド", "");
        let resolver = SongVersionResolver::new(
            &[std_song.clone(), dx_song.clone()],
            &[
                official_sheet(&std_song.identity, ChartType::Std),
                official_sheet(&dx_song.identity, ChartType::Dx),
            ],
        )
        .expect("resolver");

        assert_eq!(
            resolver
                .resolve_song_identity("Link", "ORANGE", ChartType::Dx)
                .expect("resolve"),
            Some(dx_song.identity)
        );
    }

    #[test]
    fn duplicate_title_requires_override_when_still_ambiguous() {
        let songs = vec![
            official_song("Link", "maimai", ""),
            official_song("Link", "niconico＆ボーカロイド", ""),
        ];
        let sheets = songs
            .iter()
            .map(|song| official_sheet(&song.identity, ChartType::Std))
            .collect::<Vec<_>>();
        let resolver = SongVersionResolver::new(&songs, &sheets).expect("resolver");

        let err = resolver
            .resolve_song_identity("Link", "BUDDiES", ChartType::Std)
            .expect_err("expected ambiguity");
        assert!(
            err.to_string().contains("without override"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn skips_intl_only_titles_without_failing() {
        let intl_only_song = SongRow {
            identity: SongIdentity::new(
                "全世界共通リズム感テスト",
                SongGenre::Maimai,
                "☆リズムに合わせてボタンを叩き達成率を競うゲームです☆",
            ),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        };
        let intl_only_sheet = SheetRow {
            song_identity: intl_only_song.identity.clone(),
            sheet_type: ChartType::Std,
            difficulty: models::DifficultyCategory::Basic,
            level: "6".to_string(),
            source: SheetSource::IntlOnly {
                version_name: "Splash".to_string(),
                internal_level: None,
            },
        };
        let resolver =
            SongVersionResolver::new(&[intl_only_song], &[intl_only_sheet]).expect("resolver");

        assert_eq!(
            resolver
                .resolve_song_identity("全世界共通リズム感テスト", "Splash", ChartType::Std)
                .expect("resolve"),
            None
        );
    }

    #[test]
    fn parses_intl_version_fixture_against_official_jp_fixture() {
        let (songs, sheets) =
            load_official_rows_from_json(OFFICIAL_JP_SONGS_JSON).expect("load official JP rows");
        let resolver = SongVersionResolver::new(&songs, &sheets).expect("resolver");

        let rows =
            parse_rows(INTL_VERSION0_MAIMAI_DIFF0_HTML, "maimai", &resolver).expect("parse rows");

        assert_eq!(
            rows.len(),
            43,
            "expected maimai version fixture row count changed"
        );
        assert!(!rows.is_empty());
    }

    #[test]
    fn orange_fixture_resolves_link_to_expected_official_song() {
        let (songs, sheets) =
            load_official_rows_from_json(OFFICIAL_JP_SONGS_JSON).expect("load official JP rows");
        let resolver = SongVersionResolver::new(&songs, &sheets).expect("resolver");

        let rows = parse_rows(INTL_VERSION4_ORANGE_DIFF0_HTML, "ORANGE", &resolver)
            .expect("parse ORANGE rows");
        let expected_song_identity = SongIdentity::new(
            "Link",
            SongGenre::NiconicoVocaloid,
            "Circle of friends(天月-あまつき-・un:c・伊東歌詞太郎・コニー・はしやん)",
        );

        assert!(
            rows.iter().any(
                |(song_identity, chart_type)| song_identity == &expected_song_identity
                    && *chart_type == ChartType::Std
            ),
            "expected ORANGE fixture to resolve Link STD using official JP song identity"
        );
    }

    #[test]
    fn maimai_plus_fixture_resolves_link_to_expected_official_song() {
        let (songs, sheets) =
            load_official_rows_from_json(OFFICIAL_JP_SONGS_JSON).expect("load official JP rows");
        let resolver = SongVersionResolver::new(&songs, &sheets).expect("resolver");

        let rows = parse_rows(
            INTL_VERSION1_MAIMAI_PLUS_DIFF0_HTML,
            "maimai PLUS",
            &resolver,
        )
        .expect("parse maimai PLUS rows");
        let expected_song_identity =
            SongIdentity::new("Link", SongGenre::Maimai, "Clean Tears feat. Youna");

        assert!(
            rows.iter().any(
                |(song_identity, chart_type)| song_identity == &expected_song_identity
                    && *chart_type == ChartType::Std
            ),
            "expected maimai PLUS fixture to resolve Link STD using official JP song identity"
        );
    }
}
