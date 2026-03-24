use models::{ChartType, DifficultyCategory};

fn chart_type_query_token(chart_type: ChartType) -> &'static str {
    match chart_type {
        ChartType::Std => "ST",
        ChartType::Dx => "DX",
    }
}

fn normalize_youtube_search_title(title: &str) -> String {
    title
        .replace('-', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn youtube_search_url(
    title: &str,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
) -> String {
    let normalized_title = normalize_youtube_search_title(title);
    let query = format!(
        "maimai {normalized_title} {} {}",
        chart_type_query_token(chart_type),
        difficulty.as_str()
    );

    format!(
        "https://youtube.com/results?search_query={}",
        urlencoding::encode(&query)
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

#[cfg(test)]
mod tests {
    use super::youtube_search_url;
    use models::{ChartType, DifficultyCategory};

    #[test]
    fn youtube_search_url_removes_hyphen_from_title() {
        let url = youtube_search_url("Foo - Bar-Baz", ChartType::Std, DifficultyCategory::Master);

        assert!(
            url.contains("search_query=maimai%20Foo%20BarBaz%20ST%20MASTER"),
            "unexpected url: {url}"
        );
    }
}
