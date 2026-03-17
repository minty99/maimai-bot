use axum::{Json, extract::State};
use eyre::WrapErr;

use crate::error::{Result, app_error_from_maimai};
use crate::state::AppState;
use maimai_parsers::parse_rating_target_music_html;
use models::ParsedRatingTargets;

pub(crate) async fn get_rating_targets(
    State(state): State<AppState>,
) -> Result<Json<ParsedRatingTargets>> {
    let mut client = state
        .maimai_client()
        .wrap_err("create HTTP client")
        .map_err(app_error_from_maimai)?;

    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")
        .map_err(app_error_from_maimai)?;

    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/home/ratingTargetMusic/")
        .wrap_err("parse ratingTargetMusic url")
        .map_err(app_error_from_maimai)?;

    let bytes = client
        .get_response(&url)
        .await
        .wrap_err("fetch ratingTargetMusic url")
        .map_err(app_error_from_maimai)?
        .body;
    let html = String::from_utf8(bytes)
        .wrap_err("ratingTargetMusic response is not utf-8")
        .map_err(app_error_from_maimai)?;

    let parsed = parse_rating_target_music_html(&html)
        .wrap_err("parse ratingTargetMusic html")
        .map_err(app_error_from_maimai)?;

    Ok(Json(parsed))
}
