use scraper::{Html, Selector};

use models::ParsedPlaylogDetail;

pub fn parse_playlog_detail_html(html: &str) -> eyre::Result<ParsedPlaylogDetail> {
    let document = Html::parse_document(html);

    let title_selectors = [
        "div.basic_block div.f_15.break",
        ".music_name_block",
        ".playlog_music_title",
    ];
    let form_selector = Selector::parse(r#"form[action*="/record/musicDetail/"]"#).unwrap();
    let idx_selector = Selector::parse(r#"input[name="idx"]"#).unwrap();

    let title = title_selectors
        .iter()
        .find_map(|selector| {
            let selector = Selector::parse(selector).ok()?;
            document
                .select(&selector)
                .next()
                .map(|e| e.text().collect::<Vec<_>>().join(""))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| eyre::eyre!("missing playlogDetail title"))?;

    let music_detail_idx = document
        .select(&form_selector)
        .find_map(|form| {
            form.select(&idx_selector)
                .next()
                .and_then(|e| e.value().attr("value"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .ok_or_else(|| eyre::eyre!("missing MY RECORD musicDetail idx"))?;

    Ok(ParsedPlaylogDetail {
        title,
        music_detail_idx,
    })
}
