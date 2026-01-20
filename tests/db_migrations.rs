#[tokio::test]
async fn migrations_run_on_memory_db() -> eyre::Result<()> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::migrate!().run(&pool).await?;

    // Basic sanity: tables exist.
    let (count,): (i64,) = sqlx::query_as(
        r#"
SELECT COUNT(*) FROM sqlite_master
WHERE type = 'table' AND name IN ('scores', 'playlogs', 'app_state')
"#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 3);

    Ok(())
}
