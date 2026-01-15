use std::path::PathBuf;

use maimai_bot::maimai::parse::recent::parse_recent_html;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("maimai")
        .join("recent")
        .join(name)
}

#[test]
fn parse_recent_record_fixture() {
    let html = std::fs::read_to_string(fixture_path("record.html")).unwrap();
    let entries = parse_recent_html(&html).unwrap();

    assert!(!entries.is_empty());
    assert!(entries.len() <= 50);
    assert!(entries.iter().all(|e| !e.song_key.trim().is_empty()));
    assert!(
        entries
            .iter()
            .all(|e| e.played_at.as_deref().unwrap_or("").len() >= 10)
    );

    println!("recent entries={}", entries.len());
    for e in entries.iter().take(5) {
        println!(
            "  track={:?} played_at={:?} diff={:?} title={:?} achv={:?} rank={:?} fc={:?} sync={:?} dx={:?} idx={:?}",
            e.track,
            e.played_at,
            e.diff,
            e.title,
            e.achievement_percent,
            e.score_rank,
            e.fc,
            e.sync,
            e.dx_score,
            e.playlog_idx
        );
    }
}
