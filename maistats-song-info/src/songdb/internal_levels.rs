use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eyre::WrapErr;
use maimai_auth::intl;
use maimai_parsers::parse_internal_level_page_html;
use models::{ChartType, DifficultyCategory, SongGenre};
use serde::{Deserialize, Serialize};

use super::{SheetRow, SongIdentity, SongRow, normalize_song_title_value};

const INTL_LEVEL_SEARCH_URL: &str =
    "https://maimaidx-eng.com/maimai-mobile/record/musicLevel/search/";
const MIN_SUPPORTED_BASE_LEVEL: u8 = 7;
const MAX_SUPPORTED_BASE_LEVEL: u8 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InternalLevelRow {
    pub(crate) song_identity: SongIdentity,
    pub(crate) sheet_type: ChartType,
    pub(crate) difficulty: DifficultyCategory,
    pub(crate) internal_level: String,
}

#[derive(Debug, Clone)]
struct LookupEntry {
    song_identity: SongIdentity,
    genre: SongGenre,
}

#[derive(Debug, Clone)]
struct ParsedLevelPageEntry {
    title: String,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    resolved: LookupEntry,
}

#[derive(Debug, Clone)]
struct AssignedLevelPageEntry {
    parsed: ParsedLevelPageEntry,
    bucket_index: usize,
    inferred_internal_level_tenths: u16,
}

#[derive(Debug, Clone, Copy)]
struct AssignmentCheck {
    expected_bucket_count: usize,
    observed_bucket_count: usize,
}

pub(crate) type InternalLevelKey = (SongIdentity, ChartType, DifficultyCategory);
type LookupKey = (String, ChartType, DifficultyCategory, String);

fn parse_displayed_level(displayed_level: &str) -> Option<(u8, bool)> {
    let displayed_level = displayed_level.trim();
    if displayed_level.is_empty() {
        return None;
    }

    let (number, is_plus) = match displayed_level.strip_suffix('+') {
        Some(number) => (number, true),
        None => (displayed_level, false),
    };

    let base_level = number.parse::<u8>().ok()?;
    if !(MIN_SUPPORTED_BASE_LEVEL..=MAX_SUPPORTED_BASE_LEVEL).contains(&base_level) {
        return None;
    }
    if is_plus && base_level >= MAX_SUPPORTED_BASE_LEVEL {
        return None;
    }

    Some((base_level, is_plus))
}

fn displayed_level_for_param(level_param: u8) -> eyre::Result<String> {
    if !(MIN_SUPPORTED_BASE_LEVEL..=23).contains(&level_param) {
        return Err(eyre::eyre!("unsupported level param: {level_param}"));
    }

    let offset = level_param - MIN_SUPPORTED_BASE_LEVEL;
    let base_level = MIN_SUPPORTED_BASE_LEVEL + offset / 2;
    let is_plus = offset % 2 == 1;
    if base_level > MAX_SUPPORTED_BASE_LEVEL || (base_level == MAX_SUPPORTED_BASE_LEVEL && is_plus)
    {
        return Err(eyre::eyre!("unsupported level param: {level_param}"));
    }

    if is_plus {
        Ok(format!("{base_level}+"))
    } else {
        Ok(base_level.to_string())
    }
}

fn level_param_for_displayed_level(displayed_level: &str) -> Option<u8> {
    let (base_level, is_plus) = parse_displayed_level(displayed_level)?;
    let offset = (base_level - MIN_SUPPORTED_BASE_LEVEL) * 2 + u8::from(is_plus);
    Some(MIN_SUPPORTED_BASE_LEVEL + offset)
}

fn bucket_count_for_level(base_level: u8, is_plus: bool) -> usize {
    if is_plus {
        4
    } else if base_level == MAX_SUPPORTED_BASE_LEVEL {
        1
    } else {
        6
    }
}

fn start_internal_tenths_for_level(base_level: u8, is_plus: bool) -> u16 {
    if is_plus {
        u16::from(base_level) * 10 + 6
    } else {
        u16::from(base_level) * 10
    }
}

fn format_internal_level_tenths(internal_level_tenths: u16) -> String {
    format!(
        "{}.{}",
        internal_level_tenths / 10,
        internal_level_tenths % 10
    )
}

fn supported_level_params() -> impl Iterator<Item = u8> {
    MIN_SUPPORTED_BASE_LEVEL..=23
}

fn genre_rank(genre: &SongGenre) -> Option<usize> {
    match genre {
        SongGenre::PopsAnime => Some(0),
        SongGenre::NiconicoVocaloid => Some(1),
        SongGenre::TouhouProject => Some(2),
        SongGenre::GameVariety => Some(3),
        SongGenre::Maimai => Some(4),
        SongGenre::OngekiChunithm => Some(5),
        SongGenre::Utage => None,
    }
}

fn build_lookup(
    songs: &[SongRow],
    sheets: &[SheetRow],
) -> eyre::Result<HashMap<LookupKey, LookupEntry>> {
    let song_by_identity = songs
        .iter()
        .map(|song| (song.identity.clone(), song))
        .collect::<HashMap<_, _>>();

    let mut lookup: HashMap<LookupKey, LookupEntry> = HashMap::new();
    for sheet in sheets.iter().filter(|sheet| sheet.source.is_official()) {
        let Some((base_level, is_plus)) = parse_displayed_level(&sheet.level) else {
            continue;
        };
        let Some(song) = song_by_identity.get(&sheet.song_identity) else {
            continue;
        };
        let Some(_) = genre_rank(&song.identity.genre) else {
            continue;
        };

        let displayed_level = if is_plus {
            format!("{base_level}+")
        } else {
            base_level.to_string()
        };
        let key = (
            normalize_song_title_value(&sheet.song_identity.title),
            sheet.sheet_type,
            sheet.difficulty,
            displayed_level,
        );
        let entry = LookupEntry {
            song_identity: sheet.song_identity.clone(),
            genre: song.identity.genre.clone(),
        };

        if let Some(existing) = lookup.get(&key) {
            tracing::warn!(
                "internal levels: duplicate official lookup key for title='{}', chart_type='{}', difficulty='{}', level='{}'; keeping '{}' and ignoring '{}'",
                key.0,
                key.1.as_str(),
                key.2.as_str(),
                key.3,
                existing.song_identity.title,
                sheet.song_identity.title
            );
            continue;
        }

        lookup.insert(key, entry);
    }

    Ok(lookup)
}

fn collect_manual_override_titles(sheets: &[SheetRow]) -> HashSet<String> {
    sheets
        .iter()
        .filter(|sheet| matches!(sheet.source, super::SheetSource::ManualOverride { .. }))
        .map(|sheet| normalize_song_title_value(&sheet.song_identity.title))
        .collect()
}

fn resolve_level_page_entries(
    html: &str,
    lookup: &HashMap<LookupKey, LookupEntry>,
    ignored_titles: &HashSet<String>,
) -> eyre::Result<Vec<ParsedLevelPageEntry>> {
    let parsed_entries = parse_internal_level_page_html(html)?;
    let mut entries = Vec::with_capacity(parsed_entries.len());

    for entry in parsed_entries {
        let title = entry.title;
        let displayed_level = entry.displayed_level;
        let chart_type = entry.chart_type;
        let difficulty = entry.difficulty;

        let key = (
            normalize_song_title_value(&title),
            chart_type,
            difficulty,
            displayed_level.clone(),
        );
        if ignored_titles.contains(&key.0) {
            tracing::info!(
                "internal levels: skipping manual override title='{}' chart_type='{}' difficulty='{}' level='{}'",
                key.0,
                chart_type.as_str(),
                difficulty.as_str(),
                displayed_level
            );
            continue;
        }
        let resolved = lookup.get(&key).cloned().ok_or_else(|| {
            eyre::eyre!(
                "no song candidate for level page row: title='{}', chart_type='{}', difficulty='{}', level='{}'",
                key.0,
                chart_type.as_str(),
                difficulty.as_str(),
                displayed_level
            )
        })?;

        entries.push(ParsedLevelPageEntry {
            title,
            chart_type,
            difficulty,
            resolved,
        });
    }

    Ok(entries)
}

async fn fetch_level_page_html_with_auth_recovery(
    client: &reqwest::Client,
    sega_id: &str,
    sega_password: &str,
    level_param: u8,
) -> eyre::Result<String> {
    let displayed_level = displayed_level_for_param(level_param)?;
    let response = client
        .get(INTL_LEVEL_SEARCH_URL)
        .query(&[("level", level_param.to_string())])
        .send()
        .await
        .wrap_err_with(|| format!("fetch INTL level page {displayed_level}"))?
        .error_for_status()
        .wrap_err_with(|| format!("INTL level page status {displayed_level}"))?;

    let final_url = response.url().clone();
    let html = response
        .text()
        .await
        .wrap_err_with(|| format!("read INTL level page html {displayed_level}"))?;

    if !intl::looks_like_login_or_expired(&final_url, &html) {
        return Ok(html);
    }

    tracing::warn!(
        "internal levels: level {} looked unauthenticated; re-logging in and retrying",
        displayed_level
    );
    intl::login(client, sega_id, sega_password)
        .await
        .wrap_err_with(|| format!("re-login after auth expiry for level {displayed_level}"))?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let retry_response = client
        .get(INTL_LEVEL_SEARCH_URL)
        .query(&[("level", level_param.to_string())])
        .send()
        .await
        .wrap_err_with(|| format!("retry fetch INTL level page {displayed_level}"))?
        .error_for_status()
        .wrap_err_with(|| format!("retry INTL level page status {displayed_level}"))?;

    let retry_final_url = retry_response.url().clone();
    let retry_html = retry_response
        .text()
        .await
        .wrap_err_with(|| format!("read retry INTL level page html {displayed_level}"))?;

    if intl::looks_like_login_or_expired(&retry_final_url, &retry_html) {
        return Err(eyre::eyre!(
            "INTL level page still looks unauthenticated or unavailable after re-login for {}: {}",
            displayed_level,
            retry_final_url
        ));
    }

    Ok(retry_html)
}

fn assign_internal_levels(
    entries: Vec<ParsedLevelPageEntry>,
    level_param: u8,
) -> eyre::Result<(Vec<AssignedLevelPageEntry>, AssignmentCheck)> {
    if entries.is_empty() {
        return Ok((
            Vec::new(),
            AssignmentCheck {
                expected_bucket_count: 0,
                observed_bucket_count: 0,
            },
        ));
    }

    let displayed_level = displayed_level_for_param(level_param)?;
    let (base_level, is_plus) = parse_displayed_level(&displayed_level)
        .ok_or_else(|| eyre::eyre!("unsupported displayed level: {displayed_level}"))?;
    let expected_bucket_count = bucket_count_for_level(base_level, is_plus);
    let mut current_internal_tenths = start_internal_tenths_for_level(base_level, is_plus);
    let mut observed_bucket_count = 1;
    let mut previous_genre_rank = None;
    let mut assigned = Vec::with_capacity(entries.len());

    for entry in entries {
        let genre_rank = genre_rank(&entry.resolved.genre).ok_or_else(|| {
            eyre::eyre!(
                "level {} row '{}' has unsupported genre '{}'",
                displayed_level,
                entry.title,
                entry.resolved.genre
            )
        })?;

        if previous_genre_rank.is_some_and(|previous| genre_rank < previous) {
            current_internal_tenths += 1;
            observed_bucket_count += 1;
        }
        previous_genre_rank = Some(genre_rank);

        assigned.push(AssignedLevelPageEntry {
            bucket_index: observed_bucket_count - 1,
            inferred_internal_level_tenths: current_internal_tenths,
            parsed: entry,
        });
    }

    Ok((
        assigned,
        AssignmentCheck {
            expected_bucket_count,
            observed_bucket_count,
        },
    ))
}

pub(crate) async fn fetch_internal_levels(
    sega_id: &str,
    sega_password: &str,
    songs: &[SongRow],
    sheets: &[SheetRow],
) -> eyre::Result<HashMap<InternalLevelKey, InternalLevelRow>> {
    let client = reqwest::Client::builder()
        .default_headers(intl::default_mobile_headers()?)
        .redirect(reqwest::redirect::Policy::limited(10))
        .cookie_store(true)
        .build()
        .wrap_err("build INTL internal level client")?;

    intl::ensure_logged_in(&client, sega_id, sega_password)
        .await
        .wrap_err("ensure INTL login for internal levels")?;

    let lookup = build_lookup(songs, sheets)?;
    let ignored_titles = collect_manual_override_titles(sheets);
    let mut result: HashMap<InternalLevelKey, InternalLevelRow> = HashMap::new();
    let level_params = supported_level_params().collect::<Vec<_>>();

    for (index, level_param) in level_params.iter().copied().enumerate() {
        let displayed_level = displayed_level_for_param(level_param)?;
        let html =
            fetch_level_page_html_with_auth_recovery(&client, sega_id, sega_password, level_param)
                .await?;
        let parsed_entries = resolve_level_page_entries(&html, &lookup, &ignored_titles)
            .wrap_err_with(|| format!("parse INTL level page {displayed_level}"))?;
        let (assigned_entries, check) = assign_internal_levels(parsed_entries, level_param)
            .wrap_err_with(|| format!("assign INTL level page {displayed_level}"))?;

        if check.expected_bucket_count != check.observed_bucket_count {
            tracing::warn!(
                "internal levels: level {} expected {} buckets but observed {}",
                displayed_level,
                check.expected_bucket_count,
                check.observed_bucket_count
            );
        } else {
            tracing::info!(
                "internal levels: level {} produced {} buckets",
                displayed_level,
                check.observed_bucket_count
            );
        }

        for assigned_entry in assigned_entries {
            let key = (
                assigned_entry.parsed.resolved.song_identity.clone(),
                assigned_entry.parsed.chart_type,
                assigned_entry.parsed.difficulty,
            );
            let new_row = InternalLevelRow {
                song_identity: assigned_entry.parsed.resolved.song_identity,
                sheet_type: assigned_entry.parsed.chart_type,
                difficulty: assigned_entry.parsed.difficulty,
                internal_level: format_internal_level_tenths(
                    assigned_entry.inferred_internal_level_tenths,
                ),
            };

            if let Some(existing) = result.get(&key) {
                if existing.internal_level != new_row.internal_level {
                    return Err(eyre::eyre!(
                        "conflicting inferred internal levels for '{}' / {} / {}: {} vs {}",
                        new_row.song_identity.title,
                        new_row.sheet_type.as_str(),
                        new_row.difficulty.as_str(),
                        existing.internal_level,
                        new_row.internal_level
                    ));
                }
                continue;
            }

            result.insert(key, new_row);
        }

        if index + 1 != level_params.len() {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    tracing::info!("Internal levels: {} inferred entries total", result.len());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::*;

    fn test_song(title: &str, genre: SongGenre) -> SongRow {
        SongRow {
            identity: SongIdentity::new(title, genre, ""),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        }
    }

    fn test_sheet(
        title: &str,
        genre: SongGenre,
        difficulty: DifficultyCategory,
        level: &str,
    ) -> SheetRow {
        SheetRow {
            song_identity: SongIdentity::new(title, genre, ""),
            sheet_type: ChartType::Dx,
            difficulty,
            level: level.to_string(),
            source: super::super::SheetSource::Official,
        }
    }

    fn html_for_rows(rows: &[(&str, &str, &str, &str)]) -> String {
        let mut html = String::from("<html><body>");
        for (difficulty, chart, level, title) in rows {
            html.push_str(&format!(
                r#"<div class="music_{}_score_back">
                    <img src="https://maimaidx-eng.com/maimai-mobile/img/music_{}.png" class="music_kind_icon" />
                    <div class="music_lv_block">{}</div>
                    <div class="music_name_block">{}</div>
                </div>"#,
                difficulty, chart, level, title
            ));
        }
        html.push_str("</body></html>");
        html
    }

    #[test]
    fn computed_level_rules_cover_7_to_15() {
        assert_eq!(displayed_level_for_param(7).unwrap(), "7");
        assert_eq!(displayed_level_for_param(8).unwrap(), "7+");
        assert_eq!(displayed_level_for_param(17).unwrap(), "12");
        assert_eq!(displayed_level_for_param(18).unwrap(), "12+");
        assert_eq!(displayed_level_for_param(23).unwrap(), "15");
        assert!(displayed_level_for_param(24).is_err());

        assert_eq!(level_param_for_displayed_level("7"), Some(7));
        assert_eq!(level_param_for_displayed_level("7+"), Some(8));
        assert_eq!(level_param_for_displayed_level("12"), Some(17));
        assert_eq!(level_param_for_displayed_level("12+"), Some(18));
        assert_eq!(level_param_for_displayed_level("15"), Some(23));
        assert_eq!(parse_displayed_level("7"), Some((7, false)));
        assert_eq!(parse_displayed_level("7+"), Some((7, true)));
        assert_eq!(bucket_count_for_level(7, false), 6);
        assert_eq!(bucket_count_for_level(7, true), 4);
        assert_eq!(bucket_count_for_level(15, false), 1);
        assert_eq!(start_internal_tenths_for_level(7, false), 70);
        assert_eq!(start_internal_tenths_for_level(7, true), 76);
        assert_eq!(format_internal_level_tenths(70), "7.0");
        assert_eq!(format_internal_level_tenths(76), "7.6");
    }

    #[test]
    fn resolve_level_page_entries_reads_title_chart_and_difficulty() {
        let songs = vec![test_song("Song A", SongGenre::PopsAnime)];
        let sheets = vec![test_sheet(
            "Song A",
            SongGenre::PopsAnime,
            DifficultyCategory::Expert,
            "12",
        )];
        let lookup = build_lookup(&songs, &sheets).expect("build lookup");
        let html = html_for_rows(&[("expert", "dx", "12", "Song A")]);
        let ignored_titles = HashSet::new();

        let rows = resolve_level_page_entries(&html, &lookup, &ignored_titles).expect("parse rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Song A");
        assert_eq!(rows[0].chart_type, ChartType::Dx);
        assert_eq!(rows[0].difficulty, DifficultyCategory::Expert);
        assert_eq!(rows[0].resolved.song_identity.title, "Song A");
    }

    #[test]
    fn assign_internal_levels_increments_on_genre_wrap() {
        let entries = vec![
            ParsedLevelPageEntry {
                title: "A".to_string(),
                chart_type: ChartType::Dx,
                difficulty: DifficultyCategory::Master,
                resolved: LookupEntry {
                    song_identity: SongIdentity::new("A", SongGenre::PopsAnime, ""),
                    genre: SongGenre::PopsAnime,
                },
            },
            ParsedLevelPageEntry {
                title: "B".to_string(),
                chart_type: ChartType::Dx,
                difficulty: DifficultyCategory::Master,
                resolved: LookupEntry {
                    song_identity: SongIdentity::new("B", SongGenre::NiconicoVocaloid, ""),
                    genre: SongGenre::NiconicoVocaloid,
                },
            },
            ParsedLevelPageEntry {
                title: "C".to_string(),
                chart_type: ChartType::Dx,
                difficulty: DifficultyCategory::Master,
                resolved: LookupEntry {
                    song_identity: SongIdentity::new("C", SongGenre::PopsAnime, ""),
                    genre: SongGenre::PopsAnime,
                },
            },
        ];

        let (assigned, check) = assign_internal_levels(entries, 17).expect("assign levels");
        let inferred = assigned
            .iter()
            .map(|entry| format_internal_level_tenths(entry.inferred_internal_level_tenths))
            .collect::<Vec<_>>();
        assert_eq!(inferred, vec!["12.0", "12.0", "12.1"]);
        assert_eq!(assigned[2].bucket_index, 1);
        assert_eq!(check.expected_bucket_count, 6);
        assert_eq!(check.observed_bucket_count, 2);
    }

    #[test]
    fn build_lookup_keeps_first_duplicate_official_key() {
        let songs = vec![
            test_song("Dup", SongGenre::PopsAnime),
            test_song("Dup", SongGenre::Maimai),
        ];
        let sheets = vec![
            test_sheet(
                "Dup",
                SongGenre::PopsAnime,
                DifficultyCategory::Expert,
                "12",
            ),
            test_sheet("Dup", SongGenre::Maimai, DifficultyCategory::Expert, "12"),
        ];

        let lookup = build_lookup(&songs, &sheets).expect("duplicate key should warn only");
        assert_eq!(lookup.len(), 1);
    }

    #[test]
    fn resolve_level_page_entries_skips_manual_override_title() {
        let songs = vec![test_song("Song A", SongGenre::PopsAnime)];
        let sheets = vec![test_sheet(
            "Song A",
            SongGenre::PopsAnime,
            DifficultyCategory::Expert,
            "12",
        )];
        let lookup = build_lookup(&songs, &sheets).expect("build lookup");
        let ignored_titles = HashSet::from([normalize_song_title_value("Override Song")]);
        let html = html_for_rows(&[
            ("expert", "dx", "12", "Override Song"),
            ("expert", "dx", "12", "Song A"),
        ]);

        let rows = resolve_level_page_entries(&html, &lookup, &ignored_titles).expect("parse rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Song A");
    }

    #[test]
    #[ignore = "manual debug test; set MAISTATS_INTERNAL_LEVEL_HTML_PATH to run against a downloaded page"]
    fn infer_internal_levels_from_html_path() {
        let html_path = std::env::var("MAISTATS_INTERNAL_LEVEL_HTML_PATH")
            .expect("set MAISTATS_INTERNAL_LEVEL_HTML_PATH to the downloaded HTML file path");
        let html = std::fs::read_to_string(&html_path).expect("read html fixture");
        let parsed_entries = parse_internal_level_page_html(&html).expect("parse level page html");
        assert!(
            !parsed_entries.is_empty(),
            "expected at least one parsed level page entry"
        );

        let displayed_levels = parsed_entries
            .iter()
            .map(|entry| entry.displayed_level.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            displayed_levels.len(),
            1,
            "expected a single displayed level per page"
        );
        let displayed_level = displayed_levels
            .into_iter()
            .next()
            .expect("single displayed level");
        let level_param = level_param_for_displayed_level(&displayed_level)
            .expect("derive level param from displayed level");

        let manual_override_rows =
            super::super::load_manual_override_rows().expect("load manual override rows");
        let client = reqwest::Client::builder()
            .build()
            .expect("build reqwest client for manual internal level test");
        let raw_songs = tokio::runtime::Runtime::new()
            .expect("build tokio runtime for manual internal level test")
            .block_on(super::super::fetch_maimai_songs(&client))
            .expect("fetch official songs json");
        let raw_songs = super::super::filter_official_songs_by_title(
            raw_songs,
            &manual_override_rows.overridden_titles,
        )
        .expect("filter overridden titles");
        let (mut songs, mut sheets) =
            super::super::build_official_rows(raw_songs).expect("build official rows");
        songs.extend(manual_override_rows.songs);
        sheets.extend(manual_override_rows.sheets);

        let lookup = build_lookup(&songs, &sheets).expect("build lookup");
        let ignored_titles = collect_manual_override_titles(&sheets);
        let resolved_entries = resolve_level_page_entries(&html, &lookup, &ignored_titles)
            .expect("resolve level page entries");
        let (assigned_entries, check) =
            assign_internal_levels(resolved_entries, level_param).expect("assign internal levels");

        println!(
            "html_path={html_path} displayed_level={} level_param={} expected_buckets={} observed_buckets={}",
            displayed_level, level_param, check.expected_bucket_count, check.observed_bucket_count
        );
        for entry in assigned_entries {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                format_internal_level_tenths(entry.inferred_internal_level_tenths),
                entry.parsed.chart_type.as_str(),
                entry.parsed.difficulty.as_str(),
                entry.parsed.resolved.genre,
                entry.parsed.title
            );
        }
    }
}
