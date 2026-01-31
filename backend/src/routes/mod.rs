pub mod cover;
pub mod health;
pub mod player;
pub mod recent;
pub mod responses;
pub mod scores;
pub mod today;

use axum::{routing::get, Router};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::state::AppState;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/ready", get(health::ready))
        .route("/api/scores/search", get(scores::search_scores))
        .route("/api/scores/rated", get(scores::get_all_rated_scores))
        .route(
            "/api/scores/:title/:chart_type/:diff_category",
            get(scores::get_score),
        )
        .route("/api/player", get(player::get_player))
        .route("/api/recent", get(recent::get_recent))
        .route("/api/today", get(today::get_today))
        .route("/api/cover/:image_name", get(cover::get_cover))
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
