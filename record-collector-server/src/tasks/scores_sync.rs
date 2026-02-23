use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;

use crate::db::{clear_scores, upsert_scores};
use crate::http_client::MaimaiClient;
use maimai_parsers::parse_scores_html;

pub(crate) async fn rebuild_scores_with_client(
    pool: &SqlitePool,
    client: &MaimaiClient,
) -> Result<usize> {
    clear_scores(pool).await.wrap_err("clear scores")?;

    let mut all = Vec::new();
    for diff in 0u8..=4 {
        let url = scores_url(diff).wrap_err("build scores url")?;
        let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
        let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
        let mut entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;
        all.append(&mut entries);
    }

    let count = all.len();
    upsert_scores(pool, &all).await.wrap_err("upsert scores")?;
    Ok(count)
}

fn scores_url(diff: u8) -> Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}
