use std::path::PathBuf;

use maimai_bot::maimai::parse::song_detail::parse_song_detail_html;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("maimai")
        .join("music_detail")
        .join(name)
}

#[test]
fn parse_song_detail_example1() {
    let html = std::fs::read_to_string(fixture_path("example1.html")).unwrap();
    let parsed = parse_song_detail_html(&html).unwrap();

    assert!(!parsed.title.trim().is_empty());
    assert_eq!(format!("{:?}", parsed.chart_type), "Dx");
    assert_eq!(
        parsed
            .difficulties
            .iter()
            .map(|d| d.diff_category.as_str())
            .collect::<Vec<_>>(),
        vec!["EXPERT", "MASTER"]
    );
    assert!(
        parsed
            .difficulties
            .iter()
            .all(|d| !d.level.trim().is_empty())
    );
    assert!(
        parsed
            .difficulties
            .iter()
            .filter(|d| d.dx_score.is_some())
            .all(|d| d.dx_score_max.is_some())
    );

    println!(
        "title={:?} diffs={}",
        parsed.title,
        parsed.difficulties.len()
    );
    for d in &parsed.difficulties {
        println!(
            "  chart={:?} diff={} lv={} achv={:?} rank={:?} fc={:?} sync={:?} dx={:?}/{:?}",
            d.chart_type,
            d.diff_category,
            d.level,
            d.achievement_percent,
            d.rank,
            d.fc,
            d.sync,
            d.dx_score,
            d.dx_score_max
        );
    }
}

#[test]
fn parse_song_detail_example2() {
    let html = std::fs::read_to_string(fixture_path("example2.html")).unwrap();
    let parsed = parse_song_detail_html(&html).unwrap();

    assert!(!parsed.title.trim().is_empty());
    assert_eq!(format!("{:?}", parsed.chart_type), "Std");
    assert_eq!(
        parsed
            .difficulties
            .iter()
            .map(|d| d.diff_category.as_str())
            .collect::<Vec<_>>(),
        vec!["ADVANCED", "EXPERT", "MASTER", "Re:MASTER"]
    );
    assert!(
        parsed
            .difficulties
            .iter()
            .all(|d| !d.level.trim().is_empty())
    );
    assert!(
        parsed
            .difficulties
            .iter()
            .filter(|d| d.dx_score.is_some())
            .all(|d| d.dx_score_max.is_some())
    );

    println!(
        "title={:?} diffs={}",
        parsed.title,
        parsed.difficulties.len()
    );
    for d in &parsed.difficulties {
        println!(
            "  chart={:?} diff={} lv={} achv={:?} rank={:?} fc={:?} sync={:?} dx={:?}/{:?}",
            d.chart_type,
            d.diff_category,
            d.level,
            d.achievement_percent,
            d.rank,
            d.fc,
            d.sync,
            d.dx_score,
            d.dx_score_max
        );
    }
}
