mod health;
mod player;
mod rating;
mod recent;
mod responses;
mod scores;
mod today;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::LatencyUnit;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

use crate::state::AppState;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/ready", get(health::ready))
        .route("/api/scores/rated", get(scores::get_all_rated_scores))
        .route("/api/scores/refresh", post(scores::refresh_song_scores))
        .route("/api/songs/scores", get(scores::get_song_detail_scores))
        .route("/api/player", get(player::get_player))
        .route("/api/rating/targets", get(rating::get_rating_targets))
        .route("/api/recent", get(recent::get_recent))
        .route("/api/today", get(today::get_today))
        .layer(CorsLayer::permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .with_state(state)
}
