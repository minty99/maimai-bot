pub mod health;
pub mod scores;
pub mod player;
pub mod recent;
pub mod today;

use axum::{
    routing::get,
    Router,
};

use crate::state::AppState;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/health/ready", get(health::ready))
        .route("/api/scores/search", get(scores::search_scores))
        .route("/api/scores/:title/:chart_type/:diff_category", get(scores::get_score))
        .route("/api/player", get(player::get_player))
        .route("/api/recent", get(recent::get_recent))
        .route("/api/today", get(today::get_today))
        .with_state(state)
}
