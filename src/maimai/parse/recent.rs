use scraper::{ElementRef, Html, Selector};

use crate::maimai::models::{ChartType, DifficultyCategory, ParsedPlayRecord, ScoreRank};

pub fn parse_recent_html(html: &str) -> eyre::Result<Vec<ParsedPlayRecord>> {
    let document = Html::parse_document(html);

    let top_selector = Selector::parse(".playlog_top_container").unwrap();
    let diff_selector = Selector::parse("img.playlog_diff").unwrap();
    let subtitle_selector = Selector::parse(".sub_title").unwrap();
    let container_selector =
        Selector::parse(r#"div[class*="playlog_"][class*="_container"]"#).unwrap();
    let song_title_block_selector = Selector::parse("div.basic_block").unwrap();
    let level_selector = Selector::parse(".playlog_level_icon").unwrap();
    let achievement_selector = Selector::parse(".playlog_achievement_txt").unwrap();
    let scorerank_selector = Selector::parse("img.playlog_scorerank").unwrap();
    let dx_score_selector = Selector::parse(".playlog_score_block .white").unwrap();
    let chart_type_selector = Selector::parse("img.playlog_music_kind_icon").unwrap();
    let idx_selector = Selector::parse(r#"input[name="idx"]"#).unwrap();
    let img_selector = Selector::parse("img").unwrap();

    let mut out = Vec::new();
    for top in document.select(&top_selector) {
        let Some(entry) =
            top.ancestors()
                .filter_map(ElementRef::wrap)
                .find(|ancestor| {
                    ancestor.value().attr("class").is_some_and(|c| {
                        c.contains("p_10") && c.contains("t_l") && c.contains("v_b")
                    })
                })
        else {
            continue;
        };

        let diff_category = entry
            .select(&diff_selector)
            .next()
            .and_then(|img| img.value().attr("src"))
            .and_then(parse_diff_category_from_icon_src);

        let (track, played_at) = entry
            .select(&subtitle_selector)
            .next()
            .map(|e| parse_subtitle_text(&collect_text(&e)))
            .unwrap_or((None, None));

        let container = match entry.select(&container_selector).find(|candidate| {
            candidate
                .select(&song_title_block_selector)
                .next()
                .is_some()
        }) {
            Some(c) => c,
            None => continue,
        };

        let song_block = match container.select(&song_title_block_selector).next() {
            Some(b) => b,
            None => continue,
        };

        let level = song_block
            .select(&level_selector)
            .next()
            .map(|e| collect_text(&e))
            .unwrap_or_default();
        let level = level.trim().to_string();
        let level = (!level.is_empty()).then_some(level);

        let title_raw = collect_text(&song_block);
        let title = strip_level_from_title(&title_raw, level.as_deref().unwrap_or(""));

        let playlog_idx = entry
            .select(&idx_selector)
            .next()
            .and_then(|e| e.value().attr("value"))
            .map(|s| s.to_string());

        let achievement_percent = entry
            .select(&achievement_selector)
            .next()
            .and_then(|e| parse_percent(&collect_text(&e)));

        let score_rank = entry
            .select(&scorerank_selector)
            .next()
            .and_then(|e| e.value().attr("src"))
            .and_then(parse_rank_from_playlog_icon_src);

        let (dx_score, dx_score_max) = entry
            .select(&dx_score_selector)
            .next()
            .and_then(|e| parse_dx_score_pair_from_fraction_text(&collect_text(&e)))
            .map(|(cur, max)| (Some(cur), Some(max)))
            .unwrap_or((None, None));

        let chart_type = entry
            .select(&chart_type_selector)
            .next()
            .and_then(|e| e.value().attr("src"))
            .and_then(parse_chart_type_from_icon_src)
            .unwrap_or(ChartType::Std);

        let mut fc: Option<String> = None;
        let mut sync: Option<String> = None;
        for img in entry.select(&img_selector) {
            let Some(src) = img.value().attr("src") else {
                continue;
            };
            if fc.is_none() {
                fc = parse_fc_from_playlog_icon_src(src);
            }
            sync = merge_sync(sync.take(), parse_sync_from_playlog_icon_src(src));
        }

        out.push(ParsedPlayRecord {
            playlog_idx,
            track,
            played_at,
            title,
            chart_type,
            diff_category,
            level,
            achievement_percent,
            score_rank,
            fc,
            sync,
            dx_score,
            dx_score_max,
        });
    }

    Ok(out)
}

fn collect_text(element: &ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn parse_subtitle_text(text: &str) -> (Option<u8>, Option<String>) {
    let normalized = text.replace(['\u{00A0}', '\u{3000}'], " ");
    let mut track: Option<u8> = None;
    let mut played_at: Option<String> = None;

    if let Some(i) = normalized.find("TRACK") {
        let after = &normalized[i + "TRACK".len()..];
        let digits = after
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        track = digits.parse::<u8>().ok();
    }

    // Expected format includes `YYYY/MM/DD HH:MM`.
    if let Some(pos) = normalized.find('/') {
        let candidate = normalized[pos.saturating_sub(4)..].trim();
        if !candidate.is_empty() {
            played_at = Some(candidate.to_string());
        }
    }

    (track, played_at)
}

fn strip_level_from_title(raw: &str, level: &str) -> String {
    let mut s = raw.trim().to_string();
    let level = level.trim();
    if !level.is_empty()
        && let Some(rest) = s.strip_prefix(level)
    {
        s = rest.to_string();
    }
    s.trim().to_string()
}

fn parse_percent(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if !trimmed.contains('%') {
        return None;
    }
    let number = trimmed.replace(['%', ' ', '\n'], "");
    number.parse::<f32>().ok()
}

fn parse_dx_score_pair_from_fraction_text(text: &str) -> Option<(i32, i32)> {
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

fn parse_diff_category_from_icon_src(src: &str) -> Option<DifficultyCategory> {
    let file = src.rsplit('/').next()?;
    let file = file.split('?').next().unwrap_or(file);
    if !file.starts_with("diff_") || !file.ends_with(".png") {
        return None;
    }
    match file {
        "diff_basic.png" => Some(DifficultyCategory::Basic),
        "diff_advanced.png" => Some(DifficultyCategory::Advanced),
        "diff_expert.png" => Some(DifficultyCategory::Expert),
        "diff_master.png" => Some(DifficultyCategory::Master),
        "diff_remaster.png" => Some(DifficultyCategory::ReMaster),
        _ => None,
    }
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

fn parse_rank_from_playlog_icon_src(src: &str) -> Option<ScoreRank> {
    let file = src.rsplit('/').next()?;
    let file = file.split('?').next().unwrap_or(file);
    let stem = file.strip_suffix(".png")?;
    ScoreRank::from_playlog_stem(stem)
}

fn parse_fc_from_playlog_icon_src(src: &str) -> Option<String> {
    let file = src.rsplit('/').next()?;
    let file = file.split('?').next().unwrap_or(file);
    let stem = file.strip_suffix(".png")?;
    let stem = stem.strip_prefix("fc_")?;
    if stem == "dummy" {
        return None;
    }
    Some(stem.to_ascii_uppercase())
}

fn parse_sync_from_playlog_icon_src(src: &str) -> Option<String> {
    let file = src.rsplit('/').next()?;
    let file = file.split('?').next().unwrap_or(file);
    let stem = file.strip_suffix(".png")?;
    let stem = stem.strip_prefix("sync_")?;
    if stem == "dummy" {
        return None;
    }
    Some(stem.to_ascii_uppercase())
}

fn merge_sync(existing: Option<String>, candidate: Option<String>) -> Option<String> {
    let Some(candidate) = candidate else {
        return existing;
    };
    let Some(existing) = existing else {
        return Some(candidate);
    };
    let existing_rank = sync_rank(&existing);
    let candidate_rank = sync_rank(&candidate);
    if candidate_rank > existing_rank {
        Some(candidate)
    } else {
        Some(existing)
    }
}

fn sync_rank(s: &str) -> u8 {
    match s {
        "FDX+" => 5,
        "FDX" => 4,
        "FS+" => 3,
        "FS" => 2,
        "SYNC" => 1,
        _ => 0,
    }
}
