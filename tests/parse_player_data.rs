use maimai_bot::maimai::parse::player_data::parse_player_data_html;

fn read_fixture(path: &str) -> String {
    std::fs::read_to_string(path).expect("read fixture")
}

#[test]
fn parse_player_data_fixture() {
    let html = read_fixture("examples/maimai/player_data/player_data.html");
    let parsed = parse_player_data_html(&html).unwrap();

    assert!(!parsed.user_name.is_empty());
    assert!(parsed.rating > 0);
    assert!(parsed.current_version_play_count > 0);
    assert!(parsed.total_play_count > 0);
}
