use maimai_bot::db;

#[tokio::test]
async fn app_state_u32_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");

    let pool = db::connect(&db_path).await.unwrap();
    db::migrate(&pool).await.unwrap();

    db::set_app_state_u32(&pool, "k", 123, 1).await.unwrap();
    assert_eq!(db::get_app_state_u32(&pool, "k").await.unwrap(), Some(123));
}
