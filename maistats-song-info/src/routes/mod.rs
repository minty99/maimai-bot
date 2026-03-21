mod cover;
mod health;
mod songs;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::LatencyUnit;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

use crate::state::AppState;

pub(crate) fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/api/songs", get(songs::list_song_info))
        .route("/api/songs/versions", get(songs::list_versions))
        .route("/api/songs/metadata", post(songs::search_song_metadata))
        .route("/api/cover/{image_name}", get(cover::get_cover))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        );

    Router::new()
        .route("/health", get(health::health))
        .route("/health/ready", get(health::ready))
        .merge(api_routes)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
