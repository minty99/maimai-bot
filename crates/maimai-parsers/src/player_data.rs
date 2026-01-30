use eyre::WrapErr;
use scraper::{Html, Selector};

use models::ParsedPlayerData;

pub fn parse_player_data_html(html: &str) -> eyre::Result<ParsedPlayerData> {
    let document = Html::parse_document(html);

    let name_selector = Selector::parse(".name_block").unwrap();
    let rating_selector = Selector::parse(".rating_block").unwrap();
    let counts_selector = Selector::parse("div.m_5.m_b_5.t_r.f_12").unwrap();

    let user_name = document
        .select(&name_selector)
        .next()
        .map(|e| collect_text(&e).trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("missing user name (.name_block)"))?;

    let rating_text = document
        .select(&rating_selector)
        .next()
        .map(|e| collect_text(&e))
        .unwrap_or_default();
    let rating = parse_u32_digits(&rating_text)
        .ok_or_else(|| eyre::eyre!("missing rating (.rating_block)"))?;

    let counts_text = document
        .select(&counts_selector)
        .map(|e| collect_text(&e))
        .find(|t| t.contains("play count of current version"))
        .unwrap_or_default();
    if counts_text.is_empty() {
        return Err(eyre::eyre!("missing play count block"));
    }

    let current_version_play_count =
        extract_number_after(&counts_text, "play count of current version")
            .ok_or_else(|| eyre::eyre!("missing current version play count"))?;
    let total_play_count = extract_number_after(&counts_text, "maimaiDX total play count")
        .ok_or_else(|| eyre::eyre!("missing total play count"))?;

    Ok(ParsedPlayerData {
        user_name,
        rating,
        current_version_play_count,
        total_play_count,
    })
}

fn collect_text(element: &scraper::ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn parse_u32_digits(text: &str) -> Option<u32> {
    let digits = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

fn extract_number_after(haystack: &str, needle: &str) -> Option<u32> {
    let start = haystack.find(needle)? + needle.len();
    let after = &haystack[start..];

    let digits = after
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().wrap_err("parse number").ok()
}
