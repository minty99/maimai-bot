mod cover;
mod health;
mod songs;

use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::state::AppState;

pub(crate) fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/ready", get(health::ready))
        .route("/api/songs/random", get(songs::random_song_by_level))
        .route("/api/songs/versions", get(songs::list_versions))
        .route(
            "/api/songs/{title}/{chart_type}/{diff_category}",
            get(songs::get_song_metadata),
        )
        .route("/api/cover/{image_name}", get(cover::get_cover))
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
