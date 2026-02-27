#[tokio::test]
async fn migrations_create_rebuilt_scores_schema() -> eyre::Result<()> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('scores')")
        .fetch_all(&pool)
        .await?;

    assert!(columns.contains(&"title".to_string()));
    assert!(columns.contains(&"chart_type".to_string()));
    assert!(columns.contains(&"diff_category".to_string()));
    assert!(columns.contains(&"achievement_x10000".to_string()));
    assert!(columns.contains(&"rank".to_string()));
    assert!(columns.contains(&"fc".to_string()));
    assert!(columns.contains(&"sync".to_string()));
    assert!(columns.contains(&"dx_score".to_string()));
    assert!(columns.contains(&"dx_score_max".to_string()));
    assert!(columns.contains(&"last_played_at".to_string()));
    assert!(columns.contains(&"play_count".to_string()));
    assert!(!columns.contains(&"source_idx".to_string()));

    Ok(())
}
