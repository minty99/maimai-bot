use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

const DEFAULT_LOG_LIMIT: usize = 1000;

#[derive(Deserialize)]
pub(crate) struct LogsQuery {
    limit: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct CollectorLogEntryResponse {
    line: String,
}

#[derive(Serialize)]
pub(crate) struct CollectorLogsResponse {
    logs: Vec<CollectorLogEntryResponse>,
    total: usize,
}

/// GET /api/logs - Returns recent collector log lines captured from tracing.
pub(crate) async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Json<CollectorLogsResponse> {
    let snapshot = state
        .log_buffer
        .snapshot(query.limit.unwrap_or(DEFAULT_LOG_LIMIT));

    Json(CollectorLogsResponse {
        total: snapshot.total,
        logs: snapshot
            .lines
            .into_iter()
            .map(|line| CollectorLogEntryResponse { line })
            .collect(),
    })
}
