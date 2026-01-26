use maimai_bot::{
    db,
    maimai::models::{ChartType, FcStatus, ParsedPlayRecord, SyncStatus},
};

#[tokio::test]
async fn insert_playlogs_does_not_overwrite_existing_row() -> eyre::Result<()> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let scraped_at = 123;
    let id = 1_768_490_655i64;

    // First insert.
    let entries = vec![ParsedPlayRecord {
        played_at_unixtime: Some(id),
        track: Some(1),
        played_at: Some("2026/01/23 12:34".to_string()),
        credit_play_count: Some(100),
        title: "Song A".to_string(),
        chart_type: ChartType::Std,
        diff_category: None,
        level: None,
        achievement_percent: Some(99.0000),
        achievement_new_record: true,
        first_play: true,
        score_rank: None,
        fc: Some(FcStatus::Fc),
        sync: Some(SyncStatus::Fs),
        dx_score: None,
        dx_score_max: None,
    }];
    db::upsert_playlogs(&pool, scraped_at, &entries).await?;

    // Second insert with different values for the same key. Insert-only means these are ignored.
    let entries = vec![ParsedPlayRecord {
        played_at_unixtime: Some(id),
        track: Some(1),
        played_at: Some("2026/01/23 12:34".to_string()),
        credit_play_count: Some(999),
        title: "Song A - SHOULD NOT APPLY".to_string(),
        chart_type: ChartType::Std,
        diff_category: None,
        level: None,
        achievement_percent: Some(1.0000),
        achievement_new_record: false,
        first_play: false,
        score_rank: None,
        fc: None,
        sync: None,
        dx_score: None,
        dx_score_max: None,
    }];
    db::upsert_playlogs(&pool, scraped_at, &entries).await?;

    let (title, credit_play_count, first_play, achievement_new_record, fc, sync): (
        String,
        Option<i64>,
        i64,
        i64,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT title, credit_play_count, first_play, achievement_new_record, fc, sync FROM playlogs WHERE played_at_unixtime = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(title, "Song A");
    assert_eq!(credit_play_count, Some(100));
    assert_eq!(first_play, 1);
    assert_eq!(achievement_new_record, 1);
    assert_eq!(fc.as_deref(), Some("FC"));
    assert_eq!(sync.as_deref(), Some("FS"));

    Ok(())
}
