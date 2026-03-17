use eyre::{Result, WrapErr};
use reqwest::Url;

use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use maimai_parsers::parse_playlog_detail_html;
use models::ParsedPlaylogDetail;

pub(crate) async fn fetch_playlog_detail(
    client: &mut MaimaiClient,
    playlog_detail_idx: &str,
) -> Result<ParsedPlaylogDetail> {
    let url = Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/playlogDetail/",
        &[("idx", playlog_detail_idx)],
    )
    .wrap_err("build playlogDetail url")?;
    let html = fetch_html_with_auth_recovery(
        client,
        &url,
        ExpectedPage::PlaylogDetail {
            idx: playlog_detail_idx.to_string(),
        },
    )
    .await
    .wrap_err("fetch playlogDetail html")?;
    parse_playlog_detail_html(&html).wrap_err("parse playlogDetail html")
}

#[cfg(test)]
mod tests {
    use eyre::WrapErr;

    use crate::config::RecordCollectorConfig;
    use crate::tasks::utils::auth::build_client;
    use crate::tasks::utils::playlog_detail::fetch_playlog_detail;
    use crate::tasks::utils::recent::fetch_recent_entries_logged_in;
    use crate::tasks::utils::song_detail::fetch_song_detail_by_idx;

    fn config_from_env() -> eyre::Result<RecordCollectorConfig> {
        dotenvy::dotenv().ok();
        RecordCollectorConfig::from_env().wrap_err("load record collector config from env")
    }

    #[tokio::test]
    #[ignore]
    async fn recent_to_playlog_detail_to_music_detail_titles_match_live() -> eyre::Result<()> {
        let config = config_from_env()?;
        let mut client = build_client(&config)?;
        client.ensure_logged_in().await?;

        let recent_entries = fetch_recent_entries_logged_in(&mut client).await?;
        let checked = recent_entries
            .into_iter()
            .filter_map(|entry| entry.playlog_detail_idx.map(|idx| (entry.title, idx)))
            .take(3)
            .collect::<Vec<_>>();

        eyre::ensure!(
            !checked.is_empty(),
            "recent page did not include playlog detail idx"
        );

        for (recent_title, playlog_idx) in checked {
            let playlog_detail = fetch_playlog_detail(&mut client, &playlog_idx).await?;
            let music_detail =
                fetch_song_detail_by_idx(&mut client, &playlog_detail.music_detail_idx).await?;

            eyre::ensure!(
                recent_title.trim() == playlog_detail.title.trim(),
                "recent/playlogDetail title mismatch: recent='{}' playlogDetail='{}'",
                recent_title,
                playlog_detail.title
            );
            eyre::ensure!(
                playlog_detail.title.trim() == music_detail.title.trim(),
                "playlogDetail/musicDetail title mismatch: playlogDetail='{}' musicDetail='{}'",
                playlog_detail.title,
                music_detail.title
            );
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn playlog_detail_chain_resolves_music_detail_live() -> eyre::Result<()> {
        let config = config_from_env()?;
        let mut client = build_client(&config)?;
        client.ensure_logged_in().await?;

        let recent_entries = fetch_recent_entries_logged_in(&mut client).await?;
        let playlog_idx = recent_entries
            .into_iter()
            .find_map(|entry| entry.playlog_detail_idx)
            .ok_or_else(|| eyre::eyre!("recent page did not include playlog detail idx"))?;

        let playlog_detail = fetch_playlog_detail(&mut client, &playlog_idx).await?;
        let music_detail =
            fetch_song_detail_by_idx(&mut client, &playlog_detail.music_detail_idx).await?;

        eyre::ensure!(
            !music_detail
                .genre
                .clone()
                .unwrap_or_default()
                .trim()
                .is_empty(),
            "expected musicDetail genre"
        );
        eyre::ensure!(
            !music_detail.artist.trim().is_empty(),
            "expected musicDetail artist"
        );
        eyre::ensure!(
            !music_detail.difficulties.is_empty(),
            "expected at least one difficulty row"
        );

        Ok(())
    }
}
