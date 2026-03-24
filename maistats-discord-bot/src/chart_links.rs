use models::{ChartType, DifficultyCategory};

fn chart_type_query_token(chart_type: ChartType) -> &'static str {
    match chart_type {
        ChartType::Std => "ST",
        ChartType::Dx => "DX",
    }
}

pub(crate) fn youtube_search_url(
    title: &str,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
) -> String {
    let query = format!(
        "maimai {title} {} {}",
        chart_type_query_token(chart_type),
        difficulty.as_str()
    );

    format!(
        "https://youtube.com/results?search_query={}",
        urlencoding::encode(&query)
    )
}

pub(crate) fn plain_chart_label(
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    level_with_internal: &str,
) -> String {
    format!(
        "[{}] {} {}",
        chart_type,
        difficulty.as_str(),
        level_with_internal
    )
}

pub(crate) fn linked_chart_label(
    title: &str,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    level_with_internal: &str,
) -> String {
    let url = youtube_search_url(title, chart_type, difficulty);
    format!(
        "[{}] [{}]({}) {}",
        chart_type,
        difficulty.as_str(),
        url,
        level_with_internal
    )
}

pub(crate) fn youtube_link_emoji(
    title: &str,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
) -> String {
    let url = youtube_search_url(title, chart_type, difficulty);
    format!("[🔗]({url})")
}

pub(crate) fn linked_short_difficulty(
    title: &str,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
) -> String {
    let short = match difficulty {
        DifficultyCategory::Basic => "B",
        DifficultyCategory::Advanced => "A",
        DifficultyCategory::Expert => "E",
        DifficultyCategory::Master => "M",
        DifficultyCategory::ReMaster => "R",
    };
    let url = youtube_search_url(title, chart_type, difficulty);
    format!("**[{short}]({url})**")
}
