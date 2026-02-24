use std::collections::{HashMap, HashSet};
use std::time::Duration;

use eyre::WrapErr;
use maimai_auth::intl;
use maimai_parsers::parse_scores_html;
use models::{ChartType, MaimaiVersion};

const INTL_VERSION_SEARCH_URL: &str =
    "https://maimaidx-eng.com/maimai-mobile/record/musicVersion/search/";

pub type SheetVersionMap = HashMap<String, HashMap<ChartType, String>>;

pub async fn fetch_intl_sheet_versions(
    sega_id: &str,
    sega_password: &str,
) -> eyre::Result<SheetVersionMap> {
    let client = reqwest::Client::builder()
        .default_headers(intl::default_mobile_headers()?)
        .redirect(reqwest::redirect::Policy::limited(10))
        .cookie_store(true)
        .build()
        .wrap_err("build INTL sheet version client")?;

    intl::ensure_logged_in(&client, sega_id, sega_password)
        .await
        .wrap_err("ensure INTL login")?;

    let mut out: SheetVersionMap = HashMap::new();
    let mut seen = HashSet::new();

    let mut version_index = 0u8;
    while let Some(version) = MaimaiVersion::from_index(version_index) {
        let rows = fetch_rows_for_version(&client, version).await?;
        for (song_id, chart_type) in rows {
            let dedup_key = format!("{song_id}|{}", chart_type.as_str());
            if !seen.insert(dedup_key.clone()) {
                return Err(eyre::eyre!(
                    "duplicate sheet version key detected: {dedup_key}"
                ));
            }

            out.entry(song_id)
                .or_default()
                .insert(chart_type, version.as_str().to_string());
        }

        version_index = version_index.saturating_add(1);
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    Ok(out)
}

async fn fetch_rows_for_version(
    client: &reqwest::Client,
    version: MaimaiVersion,
) -> eyre::Result<Vec<(String, ChartType)>> {
    let response = client
        .get(INTL_VERSION_SEARCH_URL)
        .query(&[
            ("version", version.as_index().to_string()),
            ("diff", "0".to_string()),
        ])
        .send()
        .await
        .wrap_err_with(|| format!("fetch INTL version page for {}", version.as_str()))?
        .error_for_status()
        .wrap_err_with(|| format!("INTL version page status for {}", version.as_str()))?;

    let final_url = response.url().clone();
    let html = response
        .text()
        .await
        .wrap_err_with(|| format!("read INTL version html for {}", version.as_str()))?;

    if intl::looks_like_login_or_expired(&final_url, &html) {
        return Err(eyre::eyre!(
            "INTL version page returned login/error for {}",
            version.as_str()
        ));
    }

    parse_rows(&html, version.as_str())
}

fn parse_rows(html: &str, version_name: &str) -> eyre::Result<Vec<(String, ChartType)>> {
    let entries =
        parse_scores_html(html, 0).wrap_err("parse score blocks from INTL musicVersion page")?;

    Ok(entries
        .into_iter()
        .map(|entry| {
            (
                song_id_for_version_title(&entry.title, version_name),
                entry.chart_type,
            )
        })
        .collect())
}

fn song_id_for_version_title(title: &str, version_name: &str) -> String {
    if title == "Link" {
        if version_name == "maimai PLUS" {
            return "Link".to_string();
        }
        if version_name == "ORANGE" {
            return "Link (2)".to_string();
        }
    }
    if title == "Bad Apple!! feat nomico" {
        return "Bad Apple!! feat.nomico".to_string();
    }
    title.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn song_id_special_cases_follow_arcade_songs_fetch_logic() {
        assert_eq!(
            song_id_for_version_title("Link", "maimai PLUS"),
            "Link".to_string()
        );
        assert_eq!(
            song_id_for_version_title("Link", "ORANGE"),
            "Link (2)".to_string()
        );
        assert_eq!(
            song_id_for_version_title("Bad Apple!! feat nomico", "ORANGE"),
            "Bad Apple!! feat.nomico".to_string()
        );
    }
}
