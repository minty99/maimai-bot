use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;
use tracing::{debug, warn};

use crate::db::{count_scores_rows, replace_scores};
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use crate::tasks::utils::detail_hydration::{DetailPageCache, fetch_song_detail_for_target};
use maimai_parsers::parse_scores_html;
use models::{ChartType, DifficultyCategory, ParsedPlayRecord, ParsedScoreEntry, SongTitle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SongLookupKey {
    pub(crate) title: String,
    pub(crate) chart_type: ChartType,
    pub(crate) diff_category: DifficultyCategory,
}

impl SongLookupKey {
    pub(crate) fn from_score_entry(entry: &ParsedScoreEntry) -> Self {
        Self {
            title: entry.title.clone(),
            chart_type: entry.chart_type,
            diff_category: entry.diff_category,
        }
    }

    pub(crate) fn from_recent_entry(
        entry: &ParsedPlayRecord,
        diff_category: DifficultyCategory,
    ) -> Self {
        Self {
            title: entry.title.clone(),
            chart_type: entry.chart_type,
            diff_category,
        }
    }

    pub(crate) fn diff_as_u8(&self) -> u8 {
        self.diff_category.as_u8()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScoreListSnapshot {
    pub(crate) fetched_at: i64,
    pub(crate) entries: Vec<ParsedScoreEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct DetailTarget {
    pub(crate) lookup_key: SongLookupKey,
    pub(crate) idx: String,
    pub(crate) resolved_from_snapshot_at: i64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SeedScoresOutcome {
    pub(crate) seeded: bool,
    pub(crate) rows_written: usize,
}

#[derive(Debug, Clone)]
struct QualifierResolutionRequest {
    diff: u8,
    entry_index: usize,
    source_idx: String,
    lookup_key: SongLookupKey,
    resolved_from_snapshot_at: i64,
}

pub(crate) async fn ensure_scores_seeded(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<SeedScoresOutcome> {
    let existing_rows = count_scores_rows(pool)
        .await
        .wrap_err("count scores rows")?;
    if existing_rows > 0 {
        return Ok(SeedScoresOutcome::default());
    }

    let mut snapshots = fetch_all_score_list_snapshots(client)
        .await
        .wrap_err("fetch score list snapshots for seed")?;
    let mut detail_cache = DetailPageCache::default();
    attach_duplicate_title_qualifiers(client, &mut snapshots, &mut detail_cache)
        .await
        .wrap_err("attach duplicate title qualifiers")?;

    let entries = collect_base_score_rows(&snapshots);
    replace_scores(pool, &entries)
        .await
        .wrap_err("replace seeded scores rows")?;

    Ok(SeedScoresOutcome {
        seeded: true,
        rows_written: entries.len(),
    })
}

pub(crate) async fn fetch_all_score_list_snapshots(
    client: &mut MaimaiClient,
) -> Result<HashMap<u8, ScoreListSnapshot>> {
    let mut out = HashMap::new();
    for diff in 0u8..=4 {
        let snapshot = fetch_score_list_snapshot(client, diff).await?;
        out.insert(diff, snapshot);
    }
    Ok(out)
}

pub(crate) async fn fetch_score_list_snapshot(
    client: &mut MaimaiClient,
    diff: u8,
) -> Result<ScoreListSnapshot> {
    let url = scores_url(diff).wrap_err("build scores url")?;
    let html = fetch_html_with_auth_recovery(client, &url, ExpectedPage::ScoresList { diff })
        .await
        .wrap_err("fetch scores html with auth recovery")?;
    let entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;

    Ok(ScoreListSnapshot {
        fetched_at: unix_timestamp(),
        entries,
    })
}

pub(crate) async fn reload_score_list_snapshot(
    client: &mut MaimaiClient,
    snapshots: &mut HashMap<u8, ScoreListSnapshot>,
    diff: u8,
) -> Result<()> {
    let snapshot = fetch_score_list_snapshot(client, diff)
        .await
        .wrap_err_with(|| format!("reload score list snapshot for diff={diff}"))?;
    snapshots.insert(diff, snapshot);
    Ok(())
}

pub(crate) async fn fetch_snapshots_for_lookup_keys(
    client: &mut MaimaiClient,
    lookup_keys: &[SongLookupKey],
) -> Result<HashMap<u8, ScoreListSnapshot>> {
    let mut out = HashMap::new();
    let mut diffs = HashSet::new();
    for key in lookup_keys {
        diffs.insert(key.diff_as_u8());
    }

    for diff in diffs {
        let snapshot = fetch_score_list_snapshot(client, diff)
            .await
            .wrap_err_with(|| format!("fetch snapshot for diff={diff}"))?;
        out.insert(diff, snapshot);
    }

    Ok(out)
}

pub(crate) async fn collect_incomplete_lookup_keys(
    pool: &SqlitePool,
) -> Result<Vec<SongLookupKey>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT DISTINCT title, chart_type, diff_category
        FROM scores
        WHERE achievement_x10000 IS NOT NULL
          AND (
            last_played_at IS NULL OR
            play_count IS NULL
          )
        "#,
    )
    .fetch_all(pool)
    .await
    .wrap_err("fetch incomplete score rows")?;

    let mut out = Vec::with_capacity(rows.len());
    for (title, chart_type_text, diff_category_text) in rows {
        let chart_type = match ChartType::from_str(&chart_type_text) {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    "Skipping invalid chart_type in scores row: title='{}' chart_type='{}' error={}",
                    title, chart_type_text, err
                );
                continue;
            }
        };
        let diff_category = match DifficultyCategory::from_str(&diff_category_text) {
            Ok(value) => value,
            Err(err) => {
                warn!(
                    "Skipping invalid diff_category in scores row: title='{}' diff_category='{}' error={}",
                    title, diff_category_text, err
                );
                continue;
            }
        };
        out.push(SongLookupKey {
            title,
            chart_type,
            diff_category,
        });
    }

    Ok(out)
}

pub(crate) async fn collect_outdated_lookup_keys(
    pool: &SqlitePool,
    recent_entries: &[ParsedPlayRecord],
) -> Result<Vec<SongLookupKey>> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();

    for recent in recent_entries {
        let Some(diff_category) = recent.diff_category else {
            continue;
        };

        let key = SongLookupKey::from_recent_entry(recent, diff_category);
        if !seen.insert(key.clone()) {
            continue;
        }

        let is_outdated = lookup_key_is_outdated(pool, &key, recent.played_at.as_deref())
            .await
            .wrap_err("check outdated key")?;
        if is_outdated {
            keys.push(key);
        }
    }

    Ok(keys)
}

pub(crate) fn build_detail_targets(
    lookup_keys: &[SongLookupKey],
    snapshots: &HashMap<u8, ScoreListSnapshot>,
) -> Vec<DetailTarget> {
    let mut targets = Vec::new();
    let mut seen_idx = HashSet::new();

    for key in lookup_keys {
        let Some(snapshot) = snapshots.get(&key.diff_as_u8()) else {
            continue;
        };

        let lookup_base = normalize_title_for_match(SongTitle::parse(&key.title).base_title());
        let mut matched = false;
        for entry in &snapshot.entries {
            if entry.chart_type != key.chart_type || entry.diff_category != key.diff_category {
                continue;
            }
            if normalize_title_for_match(&entry.title) != lookup_base {
                continue;
            }

            let Some(idx) = entry
                .source_idx
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            matched = true;
            if seen_idx.insert(idx.to_string()) {
                targets.push(DetailTarget {
                    lookup_key: key.clone(),
                    idx: idx.to_string(),
                    resolved_from_snapshot_at: snapshot.fetched_at,
                });
            }
        }

        if !matched {
            debug!(
                "No detail target resolved for title='{}' chart='{}' diff='{}'",
                key.title,
                key.chart_type.as_str(),
                key.diff_category.as_str()
            );
        }
    }

    targets
}

pub(crate) fn score_entries_from_song_detail(
    detail: models::ParsedSongDetail,
) -> Vec<ParsedScoreEntry> {
    let title = canonical_title_for_detail(&detail);

    detail
        .difficulties
        .into_iter()
        .map(|difficulty| ParsedScoreEntry {
            title: title.clone(),
            chart_type: difficulty.chart_type,
            diff_category: difficulty.diff_category,
            level: difficulty.level,
            achievement_percent: difficulty.achievement_percent,
            rank: difficulty.rank,
            fc: difficulty.fc,
            sync: difficulty.sync,
            dx_score: difficulty.dx_score,
            dx_score_max: difficulty.dx_score_max,
            last_played_at: difficulty.last_played_at,
            play_count: difficulty.play_count,
            source_idx: None,
        })
        .collect()
}

pub(crate) fn canonical_title_for_detail(detail: &models::ParsedSongDetail) -> String {
    let parsed = SongTitle::parse(&detail.title);

    if parsed.qualifier().is_some() {
        return parsed.canonical();
    }
    if parsed.requires_qualifier() {
        return SongTitle::from_parts(parsed.base_title(), detail.genre.as_deref()).canonical();
    }

    parsed.canonical()
}

pub(crate) fn normalize_title_for_match(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn collect_base_score_rows(snapshots: &HashMap<u8, ScoreListSnapshot>) -> Vec<ParsedScoreEntry> {
    let mut rows = Vec::new();
    let mut diffs = snapshots.keys().copied().collect::<Vec<_>>();
    diffs.sort_unstable();

    for diff in diffs {
        let Some(snapshot) = snapshots.get(&diff) else {
            continue;
        };

        for entry in &snapshot.entries {
            let mut base_entry = entry.clone();
            base_entry.last_played_at = None;
            base_entry.play_count = None;
            rows.push(base_entry);
        }
    }

    rows
}

async fn attach_duplicate_title_qualifiers(
    client: &mut MaimaiClient,
    snapshots: &mut HashMap<u8, ScoreListSnapshot>,
    detail_cache: &mut DetailPageCache,
) -> Result<()> {
    let requests = collect_qualifier_resolution_requests(snapshots)?;
    let mut canonical_title_cache: HashMap<String, String> = HashMap::new();

    for request in requests {
        let canonical = if let Some(cached) = canonical_title_cache.get(&request.source_idx) {
            cached.clone()
        } else {
            let target = DetailTarget {
                lookup_key: request.lookup_key.clone(),
                idx: request.source_idx.clone(),
                resolved_from_snapshot_at: request.resolved_from_snapshot_at,
            };
            let detail = fetch_song_detail_for_target(client, &target, snapshots, detail_cache)
                .await
                .wrap_err_with(|| {
                    format!(
                        "resolve title qualifier from idx '{}' for '{}'",
                        request.source_idx, request.lookup_key.title
                    )
                })?;

            let resolved = SongTitle::from_parts(&detail.title, detail.genre.as_deref());
            if resolved.is_ambiguous_unqualified() {
                return Err(eyre::eyre!(
                    "missing qualifier for duplicate-capable title '{}'",
                    detail.title
                ));
            }

            let canonical = resolved.canonical();
            canonical_title_cache.insert(request.source_idx.clone(), canonical.clone());
            canonical
        };

        let snapshot = snapshots
            .get_mut(&request.diff)
            .ok_or_else(|| eyre::eyre!("missing snapshot for diff={}", request.diff))?;
        let entry = resolve_qualifier_entry_mut(snapshot, &request)
            .wrap_err_with(|| format!("locate qualifier row for '{}'", request.lookup_key.title))?;
        entry.title = canonical;
    }

    Ok(())
}

fn resolve_qualifier_entry_mut<'a>(
    snapshot: &'a mut ScoreListSnapshot,
    request: &QualifierResolutionRequest,
) -> Result<&'a mut ParsedScoreEntry> {
    let resolved_index = if snapshot.fetched_at == request.resolved_from_snapshot_at
        && snapshot
            .entries
            .get(request.entry_index)
            .is_some_and(|entry| qualifier_entry_matches_source_idx(entry, request))
    {
        Some(request.entry_index)
    } else if let Some(index) = snapshot
        .entries
        .iter()
        .position(|entry| qualifier_entry_matches_source_idx(entry, request))
    {
        Some(index)
    } else {
        let candidate_indices = snapshot
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                qualifier_entry_matches_lookup(entry, request).then_some(index)
            })
            .collect::<Vec<_>>();

        match candidate_indices.as_slice() {
            [index] => Some(*index),
            [] => {
                return Err(eyre::eyre!(
                    "no qualifier row matched current snapshot for source_idx='{}' title='{}'",
                    request.source_idx,
                    request.lookup_key.title
                ));
            }
            _ => {
                return Err(eyre::eyre!(
                    "multiple qualifier rows matched current snapshot for title='{}' chart='{}' diff='{}'",
                    request.lookup_key.title,
                    request.lookup_key.chart_type.as_str(),
                    request.lookup_key.diff_category.as_str()
                ));
            }
        }
    };

    Ok(&mut snapshot.entries[resolved_index.expect("resolved index must exist")])
}

fn qualifier_entry_matches_source_idx(
    entry: &ParsedScoreEntry,
    request: &QualifierResolutionRequest,
) -> bool {
    entry.source_idx.as_deref().map(str::trim) == Some(request.source_idx.as_str())
}

fn qualifier_entry_matches_lookup(
    entry: &ParsedScoreEntry,
    request: &QualifierResolutionRequest,
) -> bool {
    entry.chart_type == request.lookup_key.chart_type
        && entry.diff_category == request.lookup_key.diff_category
        && normalize_title_for_match(&entry.title)
            == normalize_title_for_match(SongTitle::parse(&request.lookup_key.title).base_title())
}

fn collect_qualifier_resolution_requests(
    snapshots: &HashMap<u8, ScoreListSnapshot>,
) -> Result<Vec<QualifierResolutionRequest>> {
    let mut requests = Vec::new();
    let mut diffs = snapshots.keys().copied().collect::<Vec<_>>();
    diffs.sort_unstable();

    for diff in diffs {
        let Some(snapshot) = snapshots.get(&diff) else {
            continue;
        };

        for (entry_index, entry) in snapshot.entries.iter().enumerate() {
            let parsed_title = SongTitle::parse(&entry.title);
            if !parsed_title.requires_qualifier() {
                continue;
            }

            let source_idx = entry
                .source_idx
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    eyre::eyre!(
                        "missing source_idx for duplicate-capable title '{}'",
                        entry.title
                    )
                })?;

            requests.push(QualifierResolutionRequest {
                diff,
                entry_index,
                source_idx: source_idx.to_string(),
                lookup_key: SongLookupKey::from_score_entry(entry),
                resolved_from_snapshot_at: snapshot.fetched_at,
            });
        }
    }

    Ok(requests)
}

async fn lookup_key_is_outdated(
    pool: &SqlitePool,
    lookup_key: &SongLookupKey,
    playlog_played_at: Option<&str>,
) -> Result<bool> {
    let Some(playlog_played_at) = playlog_played_at
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };

    let base_title = SongTitle::parse(&lookup_key.title).base_title().to_string();
    let like_pattern = format!("{} [[%", base_title);

    let last_played_values = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT last_played_at
        FROM scores
        WHERE chart_type = ?1
          AND diff_category = ?2
          AND (title = ?3 OR title LIKE ?4)
        "#,
    )
    .bind(lookup_key.chart_type.as_str())
    .bind(lookup_key.diff_category.as_str())
    .bind(&base_title)
    .bind(&like_pattern)
    .fetch_all(pool)
    .await
    .wrap_err("fetch last_played_at from scores")?;

    if last_played_values.is_empty() {
        return Ok(true);
    }

    Ok(last_played_values
        .iter()
        .any(|stored| is_score_last_played_older(stored.as_deref(), playlog_played_at)))
}

pub(crate) fn is_score_last_played_older(
    stored_last_played_at: Option<&str>,
    playlog_played_at: &str,
) -> bool {
    let playlog_played_at = playlog_played_at.trim();
    if playlog_played_at.is_empty() {
        return false;
    }

    match stored_last_played_at
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(stored) => stored < playlog_played_at,
        None => true,
    }
}

fn scores_url(diff: u8) -> Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_entry(
        title: &str,
        chart_type: ChartType,
        diff_category: DifficultyCategory,
        source_idx: Option<&str>,
    ) -> ParsedScoreEntry {
        ParsedScoreEntry {
            title: title.to_string(),
            chart_type,
            diff_category,
            level: "12+".to_string(),
            achievement_percent: Some(99.0),
            rank: None,
            fc: None,
            sync: None,
            dx_score: None,
            dx_score_max: None,
            last_played_at: None,
            play_count: None,
            source_idx: source_idx.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn outdated_rule_is_strictly_less_than() {
        assert!(is_score_last_played_older(None, "2026/01/23 01:14"));
        assert!(is_score_last_played_older(
            Some("2026/01/23 01:13"),
            "2026/01/23 01:14"
        ));
        assert!(!is_score_last_played_older(
            Some("2026/01/23 01:14"),
            "2026/01/23 01:14"
        ));
        assert!(!is_score_last_played_older(
            Some("2026/01/23 01:15"),
            "2026/01/23 01:14"
        ));
    }

    #[test]
    fn build_detail_targets_deduplicates_and_filters() {
        let entries = vec![
            score_entry(
                "Link",
                ChartType::Dx,
                DifficultyCategory::Master,
                Some("100"),
            ),
            score_entry(
                "Link",
                ChartType::Dx,
                DifficultyCategory::Master,
                Some("100"),
            ),
            score_entry(
                "Link",
                ChartType::Dx,
                DifficultyCategory::Master,
                Some("101"),
            ),
            score_entry(
                "Link",
                ChartType::Std,
                DifficultyCategory::Master,
                Some("102"),
            ),
        ];
        let mut snapshots = HashMap::new();
        snapshots.insert(
            3,
            ScoreListSnapshot {
                fetched_at: 1,
                entries,
            },
        );

        let lookup = SongLookupKey {
            title: "Link".to_string(),
            chart_type: ChartType::Dx,
            diff_category: DifficultyCategory::Master,
        };

        let resolved = build_detail_targets(&[lookup], &snapshots);
        let indices = resolved
            .into_iter()
            .map(|target| target.idx)
            .collect::<Vec<_>>();
        assert_eq!(indices, vec!["100".to_string(), "101".to_string()]);
    }

    #[test]
    fn canonical_title_for_non_duplicate_ignores_genre_qualifier() {
        let detail = models::ParsedSongDetail {
            title: "Technicians High".to_string(),
            genre: Some("POPS&ANIME".to_string()),
            chart_type: ChartType::Std,
            difficulties: vec![],
        };

        assert_eq!(canonical_title_for_detail(&detail), "Technicians High");
    }

    #[test]
    fn canonical_title_for_duplicate_uses_genre_qualifier() {
        let detail = models::ParsedSongDetail {
            title: "Link".to_string(),
            genre: Some("maimai PLUS".to_string()),
            chart_type: ChartType::Std,
            difficulties: vec![],
        };

        assert_eq!(canonical_title_for_detail(&detail), "Link [[maimai]]");
    }

    #[test]
    fn resolve_qualifier_entry_uses_current_source_idx_after_reload() -> eyre::Result<()> {
        let mut snapshot = ScoreListSnapshot {
            fetched_at: 2,
            entries: vec![
                score_entry(
                    "Other Song",
                    ChartType::Dx,
                    DifficultyCategory::Master,
                    Some("999"),
                ),
                score_entry(
                    "Link",
                    ChartType::Dx,
                    DifficultyCategory::Master,
                    Some("101"),
                ),
            ],
        };
        let request = QualifierResolutionRequest {
            diff: 3,
            entry_index: 0,
            source_idx: "101".to_string(),
            lookup_key: SongLookupKey {
                title: "Link".to_string(),
                chart_type: ChartType::Dx,
                diff_category: DifficultyCategory::Master,
            },
            resolved_from_snapshot_at: 1,
        };

        let entry = resolve_qualifier_entry_mut(&mut snapshot, &request)?;
        assert_eq!(entry.source_idx.as_deref(), Some("101"));

        Ok(())
    }

    #[test]
    fn resolve_qualifier_entry_falls_back_to_unique_lookup_match() -> eyre::Result<()> {
        let mut snapshot = ScoreListSnapshot {
            fetched_at: 2,
            entries: vec![score_entry(
                "Link",
                ChartType::Dx,
                DifficultyCategory::Master,
                Some("202"),
            )],
        };
        let request = QualifierResolutionRequest {
            diff: 3,
            entry_index: 0,
            source_idx: "101".to_string(),
            lookup_key: SongLookupKey {
                title: "Link".to_string(),
                chart_type: ChartType::Dx,
                diff_category: DifficultyCategory::Master,
            },
            resolved_from_snapshot_at: 1,
        };

        let entry = resolve_qualifier_entry_mut(&mut snapshot, &request)?;
        assert_eq!(entry.source_idx.as_deref(), Some("202"));

        Ok(())
    }
}
