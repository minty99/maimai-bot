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

    let first_play_column = sqlx::query_scalar::<_, Option<String>>(
        r#"
SELECT name FROM pragma_table_info('playlogs')
WHERE name = 'first_play'
"#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(first_play_column, None);

    let credit_play_count_column = sqlx::query_scalar::<_, Option<String>>(
        r#"
SELECT name FROM pragma_table_info('playlogs')
WHERE name = 'credit_play_count'
"#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(credit_play_count_column, None);

    let credit_id_column = sqlx::query_scalar::<_, Option<String>>(
        r#"
SELECT name FROM pragma_table_info('playlogs')
WHERE name = 'credit_id'
"#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(credit_id_column.as_deref(), Some("credit_id"));

    Ok(())
}
