use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::db::{clear_scores, upsert_scores};
use crate::http_client::MaimaiClient;
use maimai_parsers::{parse_scores_html, parse_song_detail_html};
use models::{ParsedScoreEntry, SongTitle};

pub(crate) async fn rebuild_scores_with_client(
    pool: &SqlitePool,
    client: &MaimaiClient,
) -> Result<usize> {
    clear_scores(pool).await.wrap_err("clear scores")?;

    let mut all = Vec::new();
    let mut canonical_title_cache: HashMap<String, String> = HashMap::new();
    for diff in 0u8..=4 {
        let url = scores_url(diff).wrap_err("build scores url")?;
        let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
        let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
        let mut entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;
        attach_duplicate_title_qualifiers(client, &mut entries, &mut canonical_title_cache)
            .await
            .wrap_err("attach duplicate title qualifiers")?;
        all.append(&mut entries);
    }

    let count = all.len();
    upsert_scores(pool, &all).await.wrap_err("upsert scores")?;
    Ok(count)
}

async fn attach_duplicate_title_qualifiers(
    client: &MaimaiClient,
    entries: &mut [ParsedScoreEntry],
    canonical_title_cache: &mut HashMap<String, String>,
) -> Result<()> {
    for entry in entries {
        let parsed_title = SongTitle::parse(&entry.title);
        if !parsed_title.requires_qualifier() {
            continue;
        }

        let Some(source_idx) = entry
            .source_idx
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Err(eyre::eyre!(
                "missing source_idx for duplicate-capable title '{}'",
                entry.title
            ));
        };

        let canonical = if let Some(cached) = canonical_title_cache.get(source_idx) {
            cached.clone()
        } else {
            let resolved = fetch_canonical_title_from_music_detail(client, source_idx)
                .await
                .wrap_err_with(|| format!("resolve title qualifier from idx '{source_idx}'"))?;
            canonical_title_cache.insert(source_idx.to_string(), resolved.clone());
            resolved
        };

        entry.title = canonical;
    }

    Ok(())
}

async fn fetch_canonical_title_from_music_detail(
    client: &MaimaiClient,
    idx: &str,
) -> Result<String> {
    let url = Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/musicDetail/",
        &[("idx", idx)],
    )
    .wrap_err("build musicDetail url")?;

    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch musicDetail page")?;
    let html = String::from_utf8(bytes).wrap_err("musicDetail response is not utf-8")?;
    let parsed = parse_song_detail_html(&html).wrap_err("parse musicDetail page")?;

    let canonical_title = SongTitle::from_parts(&parsed.title, parsed.genre.as_deref());
    if canonical_title.is_ambiguous_unqualified() {
        return Err(eyre::eyre!(
            "missing qualifier for duplicate-capable title '{}'",
            parsed.title
        ));
    }

    Ok(canonical_title.canonical())
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
