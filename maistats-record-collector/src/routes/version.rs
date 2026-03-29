use axum::{Json, http::StatusCode};
use models::VersionApiResponse;

/// GET /api/version - Returns the running record collector version.
pub(crate) async fn get_version() -> (StatusCode, Json<VersionApiResponse>) {
    (
        StatusCode::OK,
        Json(VersionApiResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}
