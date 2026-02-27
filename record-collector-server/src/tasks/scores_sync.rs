use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eyre::{Result, WrapErr};
use rand::Rng;
use reqwest::Url;
use sqlx::SqlitePool;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::db::{clear_scores, upsert_scores};
use crate::http_client::MaimaiClient;
use maimai_parsers::{parse_scores_html, parse_song_detail_html};
use models::{ChartType, DifficultyCategory, ParsedPlayRecord, ParsedScoreEntry, SongTitle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SongLookupKey {
    title: String,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
}

impl SongLookupKey {
    fn from_score_entry(entry: &ParsedScoreEntry) -> Self {
        Self {
            title: entry.title.clone(),
            chart_type: entry.chart_type,
            diff_category: entry.diff_category,
        }
    }

    fn from_recent_entry(entry: &ParsedPlayRecord, diff_category: DifficultyCategory) -> Self {
        Self {
            title: entry.title.clone(),
            chart_type: entry.chart_type,
            diff_category,
        }
    }

    fn diff_as_u8(&self) -> u8 {
        self.diff_category.as_u8()
    }
}

#[derive(Debug, Clone)]
struct SongDetailTarget {
    idx: String,
    lookup: SongLookupKey,
}

pub(crate) async fn bootstrap_scores_with_client(
    pool: &SqlitePool,
    client: &MaimaiClient,
) -> Result<usize> {
    clear_scores(pool).await.wrap_err("clear scores")?;

    let mut all = fetch_all_scores_entries(client)
        .await
        .wrap_err("fetch score list pages")?;

    attach_duplicate_title_qualifiers(client, &mut all)
        .await
        .wrap_err("attach duplicate title qualifiers")?;

    let total_count = all.len();
    let detail_targets = collect_bootstrap_detail_targets(&all);
    let detail_target_total = detail_targets.len();
    info!("Scores bootstrap detail hydration started: total_targets={detail_target_total}");

    let mut merged_scores: HashMap<SongLookupKey, ParsedScoreEntry> =
        HashMap::with_capacity(total_count);
    for entry in all {
        merged_scores.insert(SongLookupKey::from_score_entry(&entry), entry);
    }

    let mut detail_rows_merged = 0usize;
    for (index, target) in detail_targets.into_iter().enumerate() {
        let current = index + 1;
        let remaining = detail_target_total.saturating_sub(current);
        info!(
            "Scores bootstrap progress: {}/{} (remaining={}) title='{}'",
            current, detail_target_total, remaining, target.lookup.title
        );
        match fetch_song_detail_with_retry(client, &target.idx, &target.lookup).await {
            Ok(parsed) => {
                for detail_entry in score_entries_from_song_detail(parsed) {
                    let key = SongLookupKey::from_score_entry(&detail_entry);
                    if let Some(base_entry) = merged_scores.get_mut(&key) {
                        merge_detail_into_score(base_entry, detail_entry);
                        detail_rows_merged += 1;
                    } else {
                        warn!(
                            "Detail row did not match base score list; skipping title='{}' chart='{}' diff='{}'",
                            key.title,
                            key.chart_type.as_str(),
                            key.diff_category.as_str()
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Skipping detail hydration for idx={} (title='{}'): {e:#}",
                    target.idx, target.lookup.title
                );
            }
        }
    }

    let final_rows = merged_scores.into_values().collect::<Vec<_>>();
    upsert_scores(pool, &final_rows)
        .await
        .wrap_err("upsert merged bootstrap scores")?;

    info!(
        "Scores bootstrap complete: final_rows={} detail_rows_merged={} detail_targets={}",
        total_count, detail_rows_merged, detail_target_total
    );

    Ok(total_count)
}

pub(crate) async fn refresh_outdated_scores_from_recent(
    pool: &SqlitePool,
    client: &MaimaiClient,
    recent_entries: &[ParsedPlayRecord],
) -> Result<usize> {
    let outdated_keys = collect_outdated_lookup_keys(pool, recent_entries)
        .await
        .wrap_err("collect outdated score keys")?;

    if outdated_keys.is_empty() {
        debug!("No outdated score keys from recent playlogs");
        return Ok(0);
    }

    let indices_by_key = resolve_source_indices_for_lookup_keys(client, &outdated_keys)
        .await
        .wrap_err("resolve source indices for outdated keys")?;

    let detail_targets = build_detail_targets(&outdated_keys, &indices_by_key);
    if detail_targets.is_empty() {
        info!("No resolvable source_idx found for outdated score keys");
        return Ok(0);
    }

    let mut updates = Vec::new();
    for target in detail_targets {
        match fetch_song_detail_with_retry(client, &target.idx, &target.lookup).await {
            Ok(parsed) => updates.extend(score_entries_from_song_detail(parsed)),
            Err(e) => warn!(
                "Skipping outdated refresh for idx={} (title='{}'): {e:#}",
                target.idx, target.lookup.title
            ),
        }
    }

    if updates.is_empty() {
        return Ok(0);
    }

    let updated_rows = updates.len();
    upsert_scores(pool, &updates)
        .await
        .wrap_err("upsert outdated detail scores")?;

    Ok(updated_rows)
}

fn collect_bootstrap_detail_targets(entries: &[ParsedScoreEntry]) -> Vec<SongDetailTarget> {
    let mut targets = Vec::new();
    let mut seen_idx = HashSet::new();

    for entry in entries {
        if entry.achievement_percent.is_none() {
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

        if seen_idx.insert(idx.to_string()) {
            targets.push(SongDetailTarget {
                idx: idx.to_string(),
                lookup: SongLookupKey::from_score_entry(entry),
            });
        }
    }

    targets
}

async fn collect_outdated_lookup_keys(
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

fn is_score_last_played_older(
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

async fn fetch_all_scores_entries(client: &MaimaiClient) -> Result<Vec<ParsedScoreEntry>> {
    let mut all = Vec::new();

    for diff in 0u8..=4 {
        let mut entries = fetch_scores_entries_for_diff(client, diff).await?;
        all.append(&mut entries);
    }

    Ok(all)
}

async fn fetch_scores_entries_for_diff(
    client: &MaimaiClient,
    diff: u8,
) -> Result<Vec<ParsedScoreEntry>> {
    let url = scores_url(diff).wrap_err("build scores url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
    let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
    parse_scores_html(&html, diff).wrap_err("parse scores html")
}

async fn attach_duplicate_title_qualifiers(
    client: &MaimaiClient,
    entries: &mut [ParsedScoreEntry],
) -> Result<()> {
    let mut canonical_title_cache: HashMap<String, String> = HashMap::new();

    for entry in entries {
        let parsed_title = SongTitle::parse(&entry.title);
        if !parsed_title.requires_qualifier() {
            continue;
        }

        let Some(source_idx) = entry
            .source_idx
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(eyre::eyre!(
                "missing source_idx for duplicate-capable title '{}'",
                entry.title
            ));
        };

        let canonical = if let Some(cached) = canonical_title_cache.get(source_idx) {
            cached.clone()
        } else {
            let lookup = SongLookupKey::from_score_entry(entry);
            let detail = fetch_song_detail_with_retry(client, source_idx, &lookup)
                .await
                .wrap_err_with(|| {
                    format!(
                        "resolve title qualifier from idx '{}' for '{}'",
                        source_idx, entry.title
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
            canonical_title_cache.insert(source_idx.to_string(), canonical.clone());
            canonical
        };

        entry.title = canonical;
    }

    Ok(())
}

async fn resolve_source_indices_for_lookup_keys(
    client: &MaimaiClient,
    lookup_keys: &[SongLookupKey],
) -> Result<HashMap<SongLookupKey, Vec<String>>> {
    let mut keys_by_diff: HashMap<u8, Vec<&SongLookupKey>> = HashMap::new();
    for key in lookup_keys {
        keys_by_diff.entry(key.diff_as_u8()).or_default().push(key);
    }

    let mut out = HashMap::new();
    for (diff, keys_for_diff) in keys_by_diff {
        let entries = fetch_scores_entries_for_diff(client, diff)
            .await
            .wrap_err_with(|| format!("fetch score list for diff={diff}"))?;

        for key in keys_for_diff {
            out.insert(
                key.clone(),
                extract_source_indices_from_entries(&entries, key),
            );
        }
    }

    Ok(out)
}

async fn resolve_source_indices_for_lookup_key(
    client: &MaimaiClient,
    lookup_key: &SongLookupKey,
) -> Result<Vec<String>> {
    let entries = fetch_scores_entries_for_diff(client, lookup_key.diff_as_u8())
        .await
        .wrap_err("fetch score list for lookup key")?;
    Ok(extract_source_indices_from_entries(&entries, lookup_key))
}

fn extract_source_indices_from_entries(
    entries: &[ParsedScoreEntry],
    lookup_key: &SongLookupKey,
) -> Vec<String> {
    let lookup_base = normalize_title_for_match(SongTitle::parse(&lookup_key.title).base_title());

    let mut indices = Vec::new();
    let mut seen = HashSet::new();

    for entry in entries {
        if entry.chart_type != lookup_key.chart_type {
            continue;
        }
        if entry.diff_category != lookup_key.diff_category {
            continue;
        }

        let entry_norm = normalize_title_for_match(&entry.title);
        if entry_norm != lookup_base {
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

        if seen.insert(idx.to_string()) {
            indices.push(idx.to_string());
        }
    }

    indices
}

fn build_detail_targets(
    lookup_keys: &[SongLookupKey],
    indices_by_key: &HashMap<SongLookupKey, Vec<String>>,
) -> Vec<SongDetailTarget> {
    let mut targets = Vec::new();
    let mut seen_idx = HashSet::new();

    for key in lookup_keys {
        let Some(indices) = indices_by_key.get(key) else {
            continue;
        };

        if indices.is_empty() {
            debug!(
                "No source_idx resolved for title='{}' chart='{}' diff='{}'",
                key.title,
                key.chart_type.as_str(),
                key.diff_category.as_str()
            );
            continue;
        }

        for idx in indices {
            if seen_idx.insert(idx.clone()) {
                targets.push(SongDetailTarget {
                    idx: idx.clone(),
                    lookup: key.clone(),
                });
            }
        }
    }

    targets
}

async fn fetch_song_detail_with_retry(
    client: &MaimaiClient,
    initial_idx: &str,
    lookup_key: &SongLookupKey,
) -> Result<models::ParsedSongDetail> {
    sleep_between_detail_requests().await;
    match fetch_song_detail_by_idx(client, initial_idx).await {
        Ok(parsed) => Ok(parsed),
        Err(first_err) => {
            debug!(
                "musicDetail idx={} failed for title='{}'; retrying with refreshed source_idx: {}",
                initial_idx, lookup_key.title, first_err
            );

            let retry_indices = resolve_source_indices_for_lookup_key(client, lookup_key)
                .await
                .wrap_err("resolve source_idx after failed musicDetail access")?;

            for retry_idx in retry_indices {
                if retry_idx == initial_idx {
                    continue;
                }

                sleep_between_detail_requests().await;
                match fetch_song_detail_by_idx(client, &retry_idx).await {
                    Ok(parsed) => return Ok(parsed),
                    Err(retry_err) => {
                        debug!(
                            "musicDetail retry idx={} failed for title='{}': {}",
                            retry_idx, lookup_key.title, retry_err
                        );
                    }
                }
            }

            Err(first_err).wrap_err_with(|| {
                format!(
                    "failed musicDetail fetch for idx='{}' (title='{}') after source_idx refresh",
                    initial_idx, lookup_key.title
                )
            })
        }
    }
}

async fn fetch_song_detail_by_idx(
    client: &MaimaiClient,
    idx: &str,
) -> Result<models::ParsedSongDetail> {
    let url = Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/musicDetail/",
        &[("idx", idx)],
    )
    .wrap_err("build musicDetail url")?;

    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch musicDetail page")?;
    let html = String::from_utf8(bytes).wrap_err("musicDetail response is not utf-8")?;
    parse_song_detail_html(&html).wrap_err("parse musicDetail page")
}

fn score_entries_from_song_detail(detail: models::ParsedSongDetail) -> Vec<ParsedScoreEntry> {
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

fn merge_detail_into_score(base: &mut ParsedScoreEntry, detail: ParsedScoreEntry) {
    base.level = detail.level;
    base.achievement_percent = detail.achievement_percent;
    base.rank = detail.rank;
    base.fc = detail.fc;
    base.sync = detail.sync;
    base.dx_score = detail.dx_score;
    base.dx_score_max = detail.dx_score_max;
    base.last_played_at = detail.last_played_at;
    base.play_count = detail.play_count;
}

fn canonical_title_for_detail(detail: &models::ParsedSongDetail) -> String {
    let parsed = SongTitle::parse(&detail.title);

    if parsed.qualifier().is_some() {
        return parsed.canonical();
    }
    if parsed.requires_qualifier() {
        return SongTitle::from_parts(parsed.base_title(), detail.genre.as_deref()).canonical();
    }

    parsed.canonical()
}

async fn sleep_between_detail_requests() {
    let delay_ms = rand::thread_rng().gen_range(300..=700);
    sleep(Duration::from_millis(delay_ms)).await;
}

fn normalize_title_for_match(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
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
    fn source_idx_resolution_deduplicates_and_filters() {
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
            score_entry(
                "Other Song",
                ChartType::Dx,
                DifficultyCategory::Master,
                Some("103"),
            ),
        ];

        let lookup = SongLookupKey {
            title: "Link".to_string(),
            chart_type: ChartType::Dx,
            diff_category: DifficultyCategory::Master,
        };

        let resolved = extract_source_indices_from_entries(&entries, &lookup);
        assert_eq!(resolved, vec!["100".to_string(), "101".to_string()]);
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
}
