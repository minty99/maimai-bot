use std::sync::LazyLock;

use models::{ChartType, DifficultyCategory};
use scraper::{ElementRef, Html, Selector};

static ENTRY_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(r#"div[class*="music_"][class*="_score_back"]"#)
        .expect("valid level-page entry selector")
});
static LEVEL_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".music_lv_block").expect("valid level selector"));
static TITLE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".music_name_block").expect("valid title selector"));
static CHART_TYPE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("img.music_kind_icon").expect("valid chart type selector"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInternalLevelEntry {
    pub title: String,
    pub chart_type: ChartType,
    pub difficulty: DifficultyCategory,
    pub displayed_level: String,
}

pub fn parse_internal_level_page_html(html: &str) -> eyre::Result<Vec<ParsedInternalLevelEntry>> {
    let document = Html::parse_document(html);
    let mut entries = Vec::new();

    for entry in document.select(&ENTRY_SELECTOR) {
        let Some(title) = entry
            .select(&TITLE_SELECTOR)
            .next()
            .map(|element| collect_text(&element).trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(displayed_level) = entry
            .select(&LEVEL_SELECTOR)
            .next()
            .map(|element| collect_text(&element).trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let chart_type = parse_chart_type(&entry)?;
        let difficulty = parse_difficulty(&entry)?;

        entries.push(ParsedInternalLevelEntry {
            title,
            chart_type,
            difficulty,
            displayed_level,
        });
    }

    Ok(entries)
}

fn collect_text(element: &ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn parse_chart_type(entry: &ElementRef<'_>) -> eyre::Result<ChartType> {
    let src = entry
        .select(&CHART_TYPE_SELECTOR)
        .next()
        .and_then(|element| element.value().attr("src"))
        .ok_or_else(|| eyre::eyre!("missing chart type icon"))?;

    if src.contains("/img/music_dx.png") {
        return Ok(ChartType::Dx);
    }
    if src.contains("/img/music_standard.png") {
        return Ok(ChartType::Std);
    }

    Err(eyre::eyre!("unknown chart type icon src: {src}"))
}

fn parse_difficulty(entry: &ElementRef<'_>) -> eyre::Result<DifficultyCategory> {
    let class_attr = entry.value().attr("class").unwrap_or_default();
    if class_attr.contains("music_basic_score_back") {
        return Ok(DifficultyCategory::Basic);
    }
    if class_attr.contains("music_advanced_score_back") {
        return Ok(DifficultyCategory::Advanced);
    }
    if class_attr.contains("music_expert_score_back") {
        return Ok(DifficultyCategory::Expert);
    }
    if class_attr.contains("music_master_score_back") {
        return Ok(DifficultyCategory::Master);
    }
    if class_attr.contains("music_remaster_score_back") {
        return Ok(DifficultyCategory::ReMaster);
    }

    Err(eyre::eyre!("unknown entry difficulty class: {class_attr}"))
}
