use eyre::WrapErr;
use models::SongAliases;
use std::collections::HashMap;
use std::path::Path;

const EN_ALIAS_URL: &str =
    "https://raw.githubusercontent.com/lomotos10/GCM-bot/main/data/aliases/en/maimai.tsv";
const KO_ALIAS_URL: &str =
    "https://raw.githubusercontent.com/lomotos10/GCM-bot/main/data/aliases/ko/maimai.tsv";

pub(crate) async fn fetch_song_aliases(
    client: &reqwest::Client,
    cache_dir: &Path,
) -> eyre::Result<HashMap<String, SongAliases>> {
    std::fs::create_dir_all(cache_dir).wrap_err("create alias cache dir")?;

    let en = load_aliases_with_cache(client, cache_dir, "en", EN_ALIAS_URL).await;
    let ko = load_aliases_with_cache(client, cache_dir, "ko", KO_ALIAS_URL).await;

    Ok(merge_alias_maps(en, ko))
}

async fn load_aliases_with_cache(
    client: &reqwest::Client,
    cache_dir: &Path,
    language: &str,
    url: &str,
) -> HashMap<String, Vec<String>> {
    let cache_path = cache_dir.join(format!("{language}.tsv"));

    match fetch_alias_file(client, url).await {
        Ok(body) => {
            if let Err(err) = std::fs::write(&cache_path, &body) {
                tracing::warn!(
                    "failed to write alias cache {}: {err:#}",
                    cache_path.display()
                );
            }
            match parse_alias_tsv(&body) {
                Ok(map) => map,
                Err(err) => {
                    tracing::warn!("failed to parse fetched {language} alias file: {err:#}");
                    load_aliases_from_cache(&cache_path, language)
                }
            }
        }
        Err(err) => {
            tracing::warn!("failed to fetch {language} aliases: {err:#}");
            load_aliases_from_cache(&cache_path, language)
        }
    }
}

fn load_aliases_from_cache(cache_path: &Path, language: &str) -> HashMap<String, Vec<String>> {
    let cached = match std::fs::read_to_string(cache_path) {
        Ok(cached) => cached,
        Err(err) => {
            tracing::warn!(
                "failed to read cached {language} aliases {}: {err:#}",
                cache_path.display()
            );
            return HashMap::new();
        }
    };

    match parse_alias_tsv(&cached) {
        Ok(map) => map,
        Err(err) => {
            tracing::warn!(
                "failed to parse cached {language} aliases {}: {err:#}",
                cache_path.display()
            );
            HashMap::new()
        }
    }
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

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let columns = line
            .split('\t')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        let Some(title) = columns.first() else {
            continue;
        };

        if title.is_empty() {
            return Err(eyre::eyre!(
                "missing title at alias line {}",
                line_index + 1
            ));
        }

        let mut aliases = aliases_by_title.remove(*title).unwrap_or_default();
        for alias in columns.iter().skip(1) {
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
}
