use eyre::WrapErr;
use scraper::{Html, Selector};

use crate::maimai::models::{ChartType, ParsedSongDetail, ParsedSongDifficultyDetail};
use crate::maimai::song_key::song_key_from_title_and_chart;

pub fn parse_song_detail_html(html: &str) -> eyre::Result<ParsedSongDetail> {
    let document = Html::parse_document(html);

    let title_selector = Selector::parse("div.basic_block div.f_15.break").unwrap();
    let page_kind_selector = Selector::parse("div.basic_block img").unwrap();
    let detail_selector =
        Selector::parse(r#"div[id][class*="music_"][class*="_score_back"]"#).unwrap();
    let level_selector = Selector::parse(".music_lv_back").unwrap();
    let score_block_selector = Selector::parse(".music_score_block").unwrap();
    let icon_selector = Selector::parse("img").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|e| e.text().collect::<Vec<_>>().join(""))
        .unwrap_or_default()
        .trim()
        .to_string();

    let page_chart_type = document
        .select(&page_kind_selector)
        .filter_map(|e| e.value().attr("src"))
        .find_map(parse_chart_type_from_icon_src)
        .unwrap_or(ChartType::Std);
    let song_key =
        song_key_from_title_and_chart(&title, page_chart_type).wrap_err("derive song_key")?;

    let mut difficulties = Vec::new();
    for section in document.select(&detail_selector) {
        let Some(diff_category) = section.value().attr("id").and_then(diff_category_from_id) else {
            continue;
        };

        let level = section
            .select(&level_selector)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| eyre::eyre!("missing level (.music_lv_back)"))?;

        let mut achievement_percent: Option<f32> = None;
        let mut dx_score: Option<i32> = None;
        let mut dx_score_max: Option<i32> = None;
        for block in section.select(&score_block_selector) {
            let text = block.text().collect::<Vec<_>>().join("");
            if achievement_percent.is_none()
                && let Some(p) = parse_percent(&text)
            {
                achievement_percent = Some(p);
                continue;
            }
            if dx_score.is_none()
                && let Some((cur, max)) = parse_dx_score_pair(&text)
            {
                dx_score = Some(cur);
                dx_score_max = Some(max);
                continue;
            }
        }

        let mut rank: Option<String> = None;
        let mut fc: Option<String> = None;
        let mut sync: Option<String> = None;
        let mut chart_type: Option<ChartType> = None;
        for img in section.select(&icon_selector) {
            let Some(src) = img.value().attr("src") else {
                continue;
            };
            if chart_type.is_none() {
                chart_type = parse_chart_type_from_icon_src(src);
            }
            if rank.is_none() {
                rank = parse_rank_from_icon_src(src);
            }
            if fc.is_none() {
                fc = parse_fc_from_icon_src(src);
            }
            sync = merge_sync(sync.take(), parse_sync_from_icon_src(src));
        }

        difficulties.push(ParsedSongDifficultyDetail {
            diff_category: diff_category.to_string(),
            level,
            chart_type: chart_type.unwrap_or(page_chart_type),
            achievement_percent,
            rank,
            fc,
            sync,
            dx_score,
            dx_score_max,
        });
    }

    difficulties.sort_by_key(|d| diff_category_order(&d.diff_category));

    Ok(ParsedSongDetail {
        song_key,
        title,
        chart_type: page_chart_type,
        difficulties,
    })
}

fn diff_category_from_id(id: &str) -> Option<&'static str> {
    match id {
        "basic" => Some("BASIC"),
        "advanced" => Some("ADVANCED"),
        "expert" => Some("EXPERT"),
        "master" => Some("MASTER"),
        "remaster" => Some("Re:MASTER"),
        _ => None,
    }
}

fn diff_category_order(category: &str) -> u8 {
    match category {
        "BASIC" => 0,
        "ADVANCED" => 1,
        "EXPERT" => 2,
        "MASTER" => 3,
        "Re:MASTER" => 4,
        _ => 255,
    }
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

fn parse_rank_from_icon_src(src: &str) -> Option<String> {
    let key = icon_key(src)?;
    Some(
        match key.as_str() {
            "s" => "S",
            "sp" => "S+",
            "ss" => "SS",
            "ssp" => "SS+",
            "sss" => "SSS",
            "sssp" => "SSS+",
            _ => return None,
        }
        .to_string(),
    )
}

fn parse_fc_from_icon_src(src: &str) -> Option<String> {
    let key = icon_key(src)?;
    Some(
        match key.as_str() {
            "fc" => "FC",
            "fcp" => "FC+",
            "ap" => "AP",
            "app" => "AP+",
            _ => return None,
        }
        .to_string(),
    )
}

fn parse_sync_from_icon_src(src: &str) -> Option<String> {
    let key = icon_key(src)?;
    Some(
        match key.as_str() {
            "fdxp" => "FDX+",
            "fdx" => "FDX",
            "fsp" => "FS+",
            "fs" => "FS",
            "sync" => "SYNC",
            _ => return None,
        }
        .to_string(),
    )
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
