use std::collections::HashMap;

use eyre::{Result, WrapErr};
use reqwest::Url;

use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use maimai_parsers::parse_song_detail_html;
use models::ParsedSongDetail;

#[derive(Debug, Default)]
pub(crate) struct SongDetailCache {
    pages: HashMap<String, ParsedSongDetail>,
}

impl SongDetailCache {
    pub(crate) fn get(&self, idx: &str) -> Option<ParsedSongDetail> {
        self.pages.get(idx).cloned()
    }

    pub(crate) fn insert(&mut self, idx: String, detail: ParsedSongDetail) {
        self.pages.insert(idx, detail);
    }
}

pub(crate) async fn fetch_song_detail_by_idx(
    client: &mut MaimaiClient,
    idx: &str,
) -> Result<ParsedSongDetail> {
    let url = Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/musicDetail/",
        &[("idx", idx)],
    )
    .wrap_err("build musicDetail url")?;
    let html = fetch_html_with_auth_recovery(
        client,
        &url,
        ExpectedPage::MusicDetail {
            idx: idx.to_string(),
        },
    )
    .await
    .wrap_err("fetch musicDetail html")?;
    parse_song_detail_html(&html).wrap_err("parse musicDetail html")
}
