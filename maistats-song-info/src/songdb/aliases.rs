use eyre::WrapErr;
use models::SongAliases;
use std::collections::HashMap;

// The GCM-bot author granted permission to reuse these maimai alias files here.
const EN_ALIAS_URL: &str =
    "https://raw.githubusercontent.com/lomotos10/GCM-bot/main/data/aliases/en/maimai.tsv";
const KO_ALIAS_URL: &str =
    "https://raw.githubusercontent.com/lomotos10/GCM-bot/main/data/aliases/ko/maimai.tsv";

pub(crate) async fn fetch_song_aliases(
    client: &reqwest::Client,
) -> eyre::Result<HashMap<String, SongAliases>> {
    let en = fetch_aliases(client, "en", EN_ALIAS_URL).await?;
    let ko = fetch_aliases(client, "ko", KO_ALIAS_URL).await?;

    Ok(merge_alias_maps(en, ko))
}

async fn fetch_aliases(
    client: &reqwest::Client,
    language: &str,
    url: &str,
) -> eyre::Result<HashMap<String, Vec<String>>> {
    let body = fetch_alias_file(client, url)
        .await
        .wrap_err_with(|| format!("fetch {language} aliases"))?;
    parse_alias_tsv(&body).wrap_err_with(|| format!("parse fetched {language} alias file"))
}

async fn fetch_alias_file(client: &reqwest::Client, url: &str) -> eyre::Result<String> {
    client
        .get(url)
        .send()
        .await
        .wrap_err("request alias file")?
        .error_for_status()
        .wrap_err("alias file status")?
        .text()
        .await
        .wrap_err("read alias file body")
}

fn parse_alias_tsv(input: &str) -> eyre::Result<HashMap<String, Vec<String>>> {
    let mut aliases_by_title = HashMap::<String, Vec<String>>::new();

    for raw_line in input.lines() {
        if raw_line.trim().is_empty() {
            continue;
        }

        let columns = raw_line.split('\t').map(str::trim).collect::<Vec<_>>();

        let Some(title) = columns.first() else {
            continue;
        };

        let mut aliases = aliases_by_title.remove(*title).unwrap_or_default();
        for alias in columns.iter().skip(1).filter(|value| !value.is_empty()) {
            if *alias == *title || aliases.iter().any(|existing| existing == alias) {
                continue;
            }
            aliases.push((*alias).to_string());
        }
        aliases_by_title.insert((*title).to_string(), aliases);
    }

    Ok(aliases_by_title)
}

fn merge_alias_maps(
    en: HashMap<String, Vec<String>>,
    ko: HashMap<String, Vec<String>>,
) -> HashMap<String, SongAliases> {
    let mut merged = HashMap::<String, SongAliases>::new();

    for (title, aliases) in en {
        merged.entry(title).or_default().en = aliases;
    }

    for (title, aliases) in ko {
        merged.entry(title).or_default().ko = aliases;
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::parse_alias_tsv;

    #[test]
    fn parse_alias_tsv_collects_alias_columns() {
        let parsed = parse_alias_tsv("Song A\tAlias 1\tAlias 2\nSong B\n").expect("parse aliases");

        assert_eq!(
            parsed.get("Song A").cloned(),
            Some(vec!["Alias 1".to_string(), "Alias 2".to_string()])
        );
        assert_eq!(parsed.get("Song B").cloned(), Some(Vec::new()));
    }

    #[test]
    fn parse_alias_tsv_deduplicates_repeated_aliases() {
        let parsed = parse_alias_tsv("Song A\tSong A\tAlias 1\nSong A\tAlias 1\tAlias 2\n")
            .expect("parse aliases");

        assert_eq!(
            parsed.get("Song A").cloned(),
            Some(vec!["Alias 1".to_string(), "Alias 2".to_string()])
        );
    }

    #[test]
    fn parse_alias_tsv_allows_empty_title() {
        let parsed =
            parse_alias_tsv("\tAlias 1\tAlias 2\n").expect("parse aliases with empty title");

        assert_eq!(
            parsed.get("").cloned(),
            Some(vec!["Alias 1".to_string(), "Alias 2".to_string()])
        );
    }
}
