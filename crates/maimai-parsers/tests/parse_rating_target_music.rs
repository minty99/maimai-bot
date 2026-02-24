use std::path::PathBuf;

use maimai_parsers::parse_rating_target_music_html;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/maimai/rating_target")
        .join(name)
}

#[test]
fn parse_rating_target_music_fixture() {
    let html = std::fs::read_to_string(fixture_path("rating_target_music.html")).unwrap();
    let parsed = parse_rating_target_music_html(&html).unwrap();

    assert_eq!(parsed.current_targets.len(), 15);
    assert_eq!(parsed.legacy_targets.len(), 35);
}
