use std::path::PathBuf;

use maimai_parsers::parse_player_data_html;
use models::DifficultyCategory;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/maimai/player_data")
        .join(name)
}

#[test]
fn parse_player_data_fixture() {
    let html = std::fs::read_to_string(fixture_path("player_data.html")).unwrap();
    let parsed = parse_player_data_html(&html).unwrap();

    assert!(!parsed.user_name.is_empty());
    assert!(parsed.rating > 0);
    assert!(parsed.current_version_play_count > 0);
    assert!(parsed.total_play_count > 0);
}

#[test]
fn difficulty_category_numeric_values_are_stable() {
    assert_eq!(DifficultyCategory::Basic.as_u8(), 0);
    assert_eq!(DifficultyCategory::Advanced.as_u8(), 1);
    assert_eq!(DifficultyCategory::Expert.as_u8(), 2);
    assert_eq!(DifficultyCategory::Master.as_u8(), 3);
    assert_eq!(DifficultyCategory::ReMaster.as_u8(), 4);
}
