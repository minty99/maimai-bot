use std::path::PathBuf;

use maimai_bot::maimai::parse::score_list::parse_scores_html;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("maimai")
        .join("scores")
        .join(name)
}

fn run_fixture_test(diff: u8, filename: &str) {
    let html = std::fs::read_to_string(fixture_path(filename)).unwrap();
    let entries = parse_scores_html(&html, diff).unwrap();

    assert!(!entries.is_empty());
    let expected_category = match diff {
        0 => "BASIC",
        1 => "ADVANCED",
        2 => "EXPERT",
        3 => "MASTER",
        4 => "Re:MASTER",
        _ => "Unknown",
    };
    assert!(entries.iter().all(|e| e.diff_category == expected_category));
    assert!(entries.iter().all(|e| !e.level.trim().is_empty()));
    assert!(
        entries
            .iter()
            .filter(|e| e.dx_score.is_some())
            .all(|e| e.dx_score_max.is_some())
    );
    if diff == 0 {
        assert!(
            entries
                .iter()
                .any(|e| e.chart_type == maimai_bot::maimai::models::ChartType::Std)
        );
        assert!(
            entries
                .iter()
                .any(|e| e.chart_type == maimai_bot::maimai::models::ChartType::Dx)
        );
    }

    println!("diff={diff} entries={}", entries.len());
    for e in entries.iter().take(5) {
        println!(
            "  chart={:?} title={:?} diff={:?} lv={:?} achv={:?} rank={:?} fc={:?} sync={:?} dx={:?}/{:?}",
            e.chart_type,
            e.title,
            e.diff_category,
            e.level,
            e.achievement_percent,
            e.rank,
            e.fc,
            e.sync,
            e.dx_score,
            e.dx_score_max,
        );
    }
}

#[test]
fn parse_scores_diff0_fixture() {
    run_fixture_test(0, "diff0_basic.html");
}

#[test]
fn parse_scores_diff1_fixture() {
    run_fixture_test(1, "diff1_advanced.html");
}

#[test]
fn parse_scores_diff2_fixture() {
    run_fixture_test(2, "diff2_expert.html");
}

#[test]
fn parse_scores_diff3_fixture() {
    run_fixture_test(3, "diff3_master.html");
}

#[test]
fn parse_scores_diff4_fixture() {
    run_fixture_test(4, "diff4_remaster.html");
}
