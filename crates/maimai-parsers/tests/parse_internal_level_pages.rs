use std::path::PathBuf;

use maimai_parsers::parse_internal_level_page_html;
use models::{ChartType, DifficultyCategory};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/maimai/internal_level_pages")
        .join(name)
}

fn expected_displayed_level(level_param: u8) -> &'static str {
    match level_param {
        7 => "7",
        8 => "7+",
        9 => "8",
        10 => "8+",
        11 => "9",
        12 => "9+",
        13 => "10",
        14 => "10+",
        15 => "11",
        16 => "11+",
        17 => "12",
        18 => "12+",
        19 => "13",
        20 => "13+",
        21 => "14",
        22 => "14+",
        23 => "15",
        _ => unreachable!("fixture level param must be 17..=23"),
    }
}

fn expected_entry_count(level_param: u8) -> usize {
    match level_param {
        7 => 350,
        8 => 336,
        9 => 293,
        10 => 158,
        11 => 158,
        12 => 182,
        13 => 204,
        14 => 242,
        15 => 213,
        16 => 195,
        17 => 228,
        18 => 372,
        19 => 500,
        20 => 384,
        21 => 213,
        22 => 82,
        23 => 3,
        _ => unreachable!("fixture level param must be 17..=23"),
    }
}

fn run_fixture_test(level_param: u8) {
    let html = std::fs::read_to_string(fixture_path(&format!("level{level_param}.html"))).unwrap();
    let entries = parse_internal_level_page_html(&html).unwrap();

    assert_eq!(entries.len(), expected_entry_count(level_param));
    assert!(
        entries
            .iter()
            .all(|entry| entry.displayed_level == expected_displayed_level(level_param))
    );
    assert!(entries.iter().all(|entry| !entry.title.trim().is_empty()));
    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.chart_type, ChartType::Std | ChartType::Dx))
    );
    assert!(entries.iter().all(|entry| {
        matches!(
            entry.difficulty,
            DifficultyCategory::Basic
                | DifficultyCategory::Advanced
                | DifficultyCategory::Expert
                | DifficultyCategory::Master
                | DifficultyCategory::ReMaster
        )
    }));
}

#[test]
fn parse_all_downloaded_internal_level_page_fixtures() {
    for level_param in 7..=23 {
        run_fixture_test(level_param);
    }
}
