use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::fetch_html_with_auth_recovery;
use crate::tasks::utils::source::ExpectedPage;
use maimai_parsers::parse_player_data_html;
use models::ParsedPlayerProfile;

pub(crate) const STATE_KEY_USER_NAME: &str = "player.user_name";
pub(crate) const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
pub(crate) const STATE_KEY_RATING: &str = "player.rating";
pub(crate) const STATE_KEY_CURRENT_VERSION_PLAY_COUNT: &str = "player.current_version_play_count";

#[derive(Debug, Clone, Default)]
pub(crate) struct StoredPlayerProfileState {
    user_name: Option<String>,
    rating: Option<u32>,
    current_version_play_count: Option<u32>,
    total_play_count: Option<u32>,
}

impl StoredPlayerProfileState {
    pub(crate) fn total_play_count(&self) -> Option<u32> {
        self.total_play_count
    }

    pub(crate) fn has_incomplete_fields(&self) -> bool {
        let fields_present = [
            self.user_name.is_some(),
            self.rating.is_some(),
            self.current_version_play_count.is_some(),
            self.total_play_count.is_some(),
        ];

        fields_present.iter().any(|present| *present)
            && fields_present.iter().any(|present| !present)
    }

    fn into_profile(self) -> Option<ParsedPlayerProfile> {
        let Self {
            user_name,
            rating,
            current_version_play_count,
            total_play_count,
        } = self;

        let (
            Some(user_name),
            Some(rating),
            Some(current_version_play_count),
            Some(total_play_count),
        ) = (
            user_name,
            rating,
            current_version_play_count,
            total_play_count,
        )
        else {
            return None;
        };

        Some(ParsedPlayerProfile {
            user_name,
            rating,
            current_version_play_count,
            total_play_count,
        })
    }
}

pub(crate) async fn fetch_player_data_logged_in(
    client: &mut MaimaiClient,
) -> Result<ParsedPlayerProfile> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let html = fetch_html_with_auth_recovery(client, &url, ExpectedPage::PlayerData).await?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

pub(crate) async fn load_stored_player_profile_state(
    pool: &SqlitePool,
) -> Result<StoredPlayerProfileState> {
    let rows = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT key, value
        FROM app_state
        WHERE key IN (?1, ?2, ?3, ?4)
        "#,
    )
    .bind(STATE_KEY_USER_NAME)
    .bind(STATE_KEY_RATING)
    .bind(STATE_KEY_CURRENT_VERSION_PLAY_COUNT)
    .bind(STATE_KEY_TOTAL_PLAY_COUNT)
    .fetch_all(pool)
    .await
    .wrap_err("load stored player profile state")?;

    let mut state = StoredPlayerProfileState::default();
    for (key, value) in rows {
        match key.as_str() {
            STATE_KEY_USER_NAME => state.user_name = Some(value),
            STATE_KEY_RATING => {
                state.rating = Some(
                    value
                        .parse::<u32>()
                        .wrap_err_with(|| format!("parse app_state key '{key}' as u32"))?,
                );
            }
            STATE_KEY_CURRENT_VERSION_PLAY_COUNT => {
                state.current_version_play_count = Some(
                    value
                        .parse::<u32>()
                        .wrap_err_with(|| format!("parse app_state key '{key}' as u32"))?,
                );
            }
            STATE_KEY_TOTAL_PLAY_COUNT => {
                state.total_play_count = Some(
                    value
                        .parse::<u32>()
                        .wrap_err_with(|| format!("parse app_state key '{key}' as u32"))?,
                );
            }
            _ => {}
        }
    }

    Ok(state)
}

pub(crate) async fn load_stored_player_profile(
    pool: &SqlitePool,
) -> Result<Option<ParsedPlayerProfile>> {
    Ok(load_stored_player_profile_state(pool).await?.into_profile())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        STATE_KEY_RATING, STATE_KEY_TOTAL_PLAY_COUNT, STATE_KEY_USER_NAME,
        StoredPlayerProfileState, load_stored_player_profile, load_stored_player_profile_state,
    };
    use crate::db::{connect, migrate, store_player_profile_snapshot};
    use models::ParsedPlayerProfile;

    fn test_database_url(test_name: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "maistats-record-collector-{test_name}-{}-{unique}.sqlite3",
            std::process::id()
        ));
        path.to_string_lossy().into_owned()
    }

    async fn setup_pool(test_name: &str) -> eyre::Result<crate::db::SqlitePool> {
        let database_url = test_database_url(test_name);
        let pool = connect(&database_url).await?;
        migrate(&pool).await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn stored_player_profile_state_roundtrips_complete_snapshot() -> eyre::Result<()> {
        let pool = setup_pool("player-state-roundtrip").await?;
        let expected = ParsedPlayerProfile {
            user_name: "fixture-user".to_string(),
            rating: 12_345,
            current_version_play_count: 50,
            total_play_count: 200,
        };

        store_player_profile_snapshot(&pool, &expected, 1).await?;

        let state = load_stored_player_profile_state(&pool).await?;
        assert_eq!(state.total_play_count(), Some(expected.total_play_count));
        assert!(!state.has_incomplete_fields());

        let profile = load_stored_player_profile(&pool).await?;
        let profile = profile.expect("stored player profile should be present");
        assert_eq!(profile.user_name, expected.user_name);
        assert_eq!(profile.rating, expected.rating);
        assert_eq!(
            profile.current_version_play_count,
            expected.current_version_play_count
        );
        assert_eq!(profile.total_play_count, expected.total_play_count);

        Ok(())
    }

    #[tokio::test]
    async fn stored_player_profile_state_detects_partial_snapshot() -> eyre::Result<()> {
        let pool = setup_pool("player-state-partial").await?;

        for (key, value) in [
            (STATE_KEY_USER_NAME, "fixture-user".to_string()),
            (STATE_KEY_RATING, "12345".to_string()),
            (STATE_KEY_TOTAL_PLAY_COUNT, "200".to_string()),
        ] {
            sqlx::query("INSERT INTO app_state (key, value, updated_at) VALUES (?1, ?2, ?3)")
                .bind(key)
                .bind(value)
                .bind(1_i64)
                .execute(&pool)
                .await?;
        }

        let state: StoredPlayerProfileState = load_stored_player_profile_state(&pool).await?;
        assert_eq!(state.total_play_count(), Some(200));
        assert!(state.has_incomplete_fields());
        assert!(load_stored_player_profile(&pool).await?.is_none());

        Ok(())
    }
}
