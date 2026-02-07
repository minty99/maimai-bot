use std::path::PathBuf;

use maimai_parsers::parse_recent_html;
use models::{ChartType, FcStatus, SyncStatus};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/maimai/recent")
        .join(name)
}

#[test]
fn parse_recent_record_fixture() {
    let html = std::fs::read_to_string(fixture_path("record.html")).unwrap();
    let entries = parse_recent_html(&html).unwrap();

    assert!(!entries.is_empty());
    assert!(entries.len() <= 50);
    assert!(entries.iter().all(|e| e.diff_category.is_some()));
    assert!(entries.iter().all(|e| e.level.is_some()));
    assert!(entries
        .iter()
        .all(|e| e.played_at.as_deref().unwrap_or("").len() >= 10));
    assert!(entries
        .iter()
        .filter(|e| e.dx_score.is_some())
        .all(|e| e.dx_score_max.is_some()));
    assert!(entries.iter().any(|e| e.chart_type == ChartType::Std));
    assert!(entries.iter().any(|e| e.chart_type == ChartType::Dx));
    assert!(entries.iter().any(|e| e.achievement_new_record));

    assert!(entries.iter().any(|e| e.fc.is_some()));
    assert!(entries.iter().any(|e| e.sync.is_some()));

    println!("recent entries={}", entries.len());
    for e in entries.iter().take(5) {
        println!(
            "  track={:?} played_at={:?} chart={:?} diff={:?} lv={:?} title={:?} achv={:?} newrec={} rank={:?} fc={:?} sync={:?} dx={:?}/{:?} played_at_unixtime={:?}",
            e.track, e.played_at, e.chart_type, e.diff_category, e.level,
            e.title, e.achievement_percent, e.achievement_new_record,
            e.score_rank, e.fc, e.sync, e.dx_score, e.dx_score_max, e.played_at_unixtime
        );
    }

    let html = std::fs::read_to_string(fixture_path("record.html")).unwrap();
    let html = html.replacen("fc_dummy.png", "fc_fc.png", 1);
    let html = html.replacen("sync_dummy.png", "sync_fs.png", 1);
    let entries = parse_recent_html(&html).unwrap();
    assert!(entries.iter().any(|e| e.fc == Some(FcStatus::Fc)));
    assert!(entries.iter().any(|e| e.sync == Some(SyncStatus::Fs)));
}
