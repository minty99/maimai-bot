use scraper::{ElementRef, Html, Selector};

use models::{ChartType, DifficultyCategory, FcStatus, ParsedScoreEntry, ScoreRank, SyncStatus};

pub fn parse_scores_html(html: &str, diff: u8) -> eyre::Result<Vec<ParsedScoreEntry>> {
    let document = Html::parse_document(html);

    let entry_selector = Selector::parse(r#"div[class*="music_"][class*="_score_back"]"#).unwrap();
    let title_selector = Selector::parse(".music_name_block").unwrap();
    let score_block_selector = Selector::parse(".music_score_block").unwrap();
    let level_selector = Selector::parse(".music_lv_block").unwrap();
    let icon_selector = Selector::parse("img").unwrap();
    let chart_type_selector = Selector::parse("img.music_kind_icon").unwrap();
    let idx_selector = Selector::parse(r#"input[name="idx"]"#).unwrap();

    let diff_category = diff_category_from_u8(diff)?;

    let mut entries = Vec::new();
    for entry in document.select(&entry_selector) {
        let title = entry
            .select(&title_selector)
            .next()
            .map(|e| collect_text(&e))
            .unwrap_or_default()
            .trim()
            .to_string();

        let source_idx = entry
            .select(&idx_selector)
            .next()
            .and_then(|e| e.value().attr("value"))
            .map(|s| s.to_string());

        let level = entry
            .select(&level_selector)
            .next()
            .map(|e| collect_text(&e).trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| eyre::eyre!("missing level (.music_lv_block)"))?;

        let mut achievement_percent: Option<f32> = None;
        let mut dx_score: Option<i32> = None;
        let mut dx_score_max: Option<i32> = None;
        for block in entry.select(&score_block_selector) {
            let text = collect_text(&block);
            if achievement_percent.is_none() {
                if let Some(p) = parse_percent(&text) {
                    achievement_percent = Some(p);
                    continue;
                }
            }
            if dx_score.is_none() {
                if let Some((cur, max)) = parse_dx_score_pair(&text) {
                    dx_score = Some(cur);
                    dx_score_max = Some(max);
                    continue;
                }
            }
        }

        let mut rank: Option<ScoreRank> = None;
        let mut fc: Option<FcStatus> = None;
        let mut sync: Option<SyncStatus> = None;
        for img in entry.select(&icon_selector) {
            let Some(src) = img.value().attr("src") else {
                continue;
            };

            if rank.is_none() {
                rank = parse_rank_from_icon_src(src);
            }
            if fc.is_none() {
                fc = parse_fc_from_icon_src(src);
            }
            sync = merge_sync(sync.take(), parse_sync_from_icon_src(src));
        }

        let chart_type = entry
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find_map(|ancestor| {
                ancestor
                    .select(&chart_type_selector)
                    .next()
                    .and_then(|e| e.value().attr("src"))
                    .and_then(parse_chart_type_from_icon_src)
            })
            .unwrap_or(ChartType::Std);

        entries.push(ParsedScoreEntry {
            title,
            chart_type,
            diff_category,
            level,
            achievement_percent,
            rank,
            fc,
            sync,
            dx_score,
            dx_score_max,
            source_idx,
        });
    }

    Ok(entries)
}

fn diff_category_from_u8(diff: u8) -> eyre::Result<DifficultyCategory> {
    match diff {
        0 => Ok(DifficultyCategory::Basic),
        1 => Ok(DifficultyCategory::Advanced),
        2 => Ok(DifficultyCategory::Expert),
        3 => Ok(DifficultyCategory::Master),
        4 => Ok(DifficultyCategory::ReMaster),
        _ => Err(eyre::eyre!("diff must be 0..4")),
    }
}

fn collect_text(element: &ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn parse_percent(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if !trimmed.contains('%') {
        return None;
    }
    let number = trimmed.replace(['%', ' ', '\n'], "");
    number.parse::<f32>().ok()
}

fn parse_dx_score_pair(text: &str) -> Option<(i32, i32)> {
    if !text.contains('/') {
        return None;
    }
    let mut iter = text.split('/');
    let left = iter.next()?.trim();
    let right = iter.next()?.trim();
    let left_digits = left
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    let right_digits = right
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    if left_digits.is_empty() || right_digits.is_empty() {
        return None;
    }
    Some((
        left_digits.parse::<i32>().ok()?,
        right_digits.parse::<i32>().ok()?,
    ))
}

fn parse_rank_from_icon_src(src: &str) -> Option<ScoreRank> {
    let key = icon_key(src)?;
    ScoreRank::from_score_icon_key(&key)
}

fn parse_fc_from_icon_src(src: &str) -> Option<FcStatus> {
    let key = icon_key(src)?;
    FcStatus::from_score_icon_key(&key)
}

fn parse_sync_from_icon_src(src: &str) -> Option<SyncStatus> {
    let key = icon_key(src)?;
    SyncStatus::from_score_icon_key(&key)
}

fn merge_sync(existing: Option<SyncStatus>, candidate: Option<SyncStatus>) -> Option<SyncStatus> {
    let Some(candidate) = candidate else {
        return existing;
    };
    let Some(existing) = existing else {
        return Some(candidate);
    };
    if candidate.priority() > existing.priority() {
        Some(candidate)
    } else {
        Some(existing)
    }
}

fn icon_key(src: &str) -> Option<String> {
    let file = src.rsplit('/').next()?;
    let file = file.split('?').next().unwrap_or(file);
    let prefix = "music_icon_";
    if !file.starts_with(prefix) {
        return None;
    }
    let key = file.strip_prefix(prefix)?;
    let key = key.strip_suffix(".png")?;
    Some(key.to_string())
}

fn parse_chart_type_from_icon_src(src: &str) -> Option<ChartType> {
    if src.contains("/img/music_dx.png") {
        return Some(ChartType::Dx);
    }
    if src.contains("/img/music_standard.png") {
        return Some(ChartType::Std);
    }
    None
}
