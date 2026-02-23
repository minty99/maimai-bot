use scraper::{ElementRef, Html, Selector};

use models::{
    ChartType, DifficultyCategory, ParsedRatingTargetEntry, ParsedRatingTargetMusic, ScoreRank,
};

const SECTION_NEW: &str = "Songs for Rating(New)";
const SECTION_OLD: &str = "Songs for Rating(Others)";
const NEW_TARGET_COUNT: usize = 15;
const OLD_TARGET_COUNT: usize = 35;

pub fn parse_rating_target_music_html(html: &str) -> eyre::Result<ParsedRatingTargetMusic> {
    let new_html = extract_section_html(html, SECTION_NEW, &[SECTION_OLD])?;
    let old_html = extract_section_html(html, SECTION_OLD, &[])?;

    let new_targets = take_first_n(parse_rating_entries(new_html)?, NEW_TARGET_COUNT, "new")?;
    let old_targets = take_first_n(parse_rating_entries(old_html)?, OLD_TARGET_COUNT, "old")?;

    Ok(ParsedRatingTargetMusic {
        new_targets,
        old_targets,
    })
}

fn take_first_n(
    entries: Vec<ParsedRatingTargetEntry>,
    n: usize,
    section: &str,
) -> eyre::Result<Vec<ParsedRatingTargetEntry>> {
    if entries.len() < n {
        return Err(eyre::eyre!(
            "{section} rating targets are fewer than expected: got {}, expected at least {n}",
            entries.len()
        ));
    }
    Ok(entries.into_iter().take(n).collect())
}

fn extract_section_html<'a>(
    html: &'a str,
    start_marker: &str,
    end_markers: &[&str],
) -> eyre::Result<&'a str> {
    let start = html
        .find(start_marker)
        .ok_or_else(|| eyre::eyre!("missing section marker: {start_marker}"))?;
    let body = &html[start + start_marker.len()..];

    let end = end_markers
        .iter()
        .filter_map(|marker| body.find(marker))
        .min()
        .unwrap_or(body.len());

    Ok(&body[..end])
}

fn parse_rating_entries(section_html: &str) -> eyre::Result<Vec<ParsedRatingTargetEntry>> {
    let document = Html::parse_fragment(section_html);

    let entry_selector = Selector::parse(r#"div[class*="music_"][class*="_score_back"]"#).unwrap();
    let title_selector = Selector::parse(".music_name_block").unwrap();
    let level_selector = Selector::parse(".music_lv_block").unwrap();
    let score_selector = Selector::parse(".music_score_block").unwrap();
    let rank_icon_selector = Selector::parse(".ratingtarget_scorerank_img").unwrap();
    let diff_selector = Selector::parse(r#"img[src*="/img/diff_"]"#).unwrap();
    let chart_type_selector = Selector::parse("img.music_kind_icon").unwrap();

    let mut entries = Vec::new();
    for entry in document.select(&entry_selector) {
        let title = entry
            .select(&title_selector)
            .next()
            .map(|e| collect_text(&e).trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| eyre::eyre!("missing title (.music_name_block)"))?;

        let level = entry
            .select(&level_selector)
            .next()
            .map(|e| collect_text(&e).trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| eyre::eyre!("missing level (.music_lv_block)"))?;

        let achievement_percent = entry
            .select(&score_selector)
            .next()
            .and_then(|e| parse_percent(&collect_text(&e)));

        let rank = entry
            .select(&rank_icon_selector)
            .next()
            .and_then(|img| img.value().attr("src"))
            .and_then(parse_rank_from_icon_src);

        let diff_category = entry
            .select(&diff_selector)
            .next()
            .and_then(|img| img.value().attr("src"))
            .and_then(parse_diff_category_from_icon_src)
            .ok_or_else(|| eyre::eyre!("missing difficulty icon"))?;

        let chart_type = entry
            .select(&chart_type_selector)
            .next()
            .and_then(|img| img.value().attr("src"))
            .and_then(parse_chart_type_from_icon_src)
            .unwrap_or(ChartType::Std);

        entries.push(ParsedRatingTargetEntry {
            title,
            chart_type,
            diff_category,
            level,
            achievement_percent,
            rank,
        });
    }

    Ok(entries)
}

fn collect_text(element: &ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn parse_percent(text: &str) -> Option<f32> {
    let trimmed = text.trim().replace('\u{00a0}', " ");
    if !trimmed.contains('%') {
        return None;
    }

    let digits = trimmed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();
    digits.parse::<f32>().ok()
}

fn parse_rank_from_icon_src(src: &str) -> Option<ScoreRank> {
    let key = icon_key(src)?;
    ScoreRank::from_score_icon_key(&key)
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
