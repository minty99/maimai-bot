use axum::extract::State;
use axum::http::StatusCode;
use tracing::error;

use crate::error::{Result, app_error_from_maimai};
use crate::state::AppState;
use crate::tasks::polling::cycle::run_cycle;

pub(crate) async fn trigger_poll(State(state): State<AppState>) -> Result<StatusCode> {
    {
        let _guard = state.cycle_lock.lock().await;
        run_cycle(&state).await.map_err(|err| {
            error!("Poll cycle triggered via /api/poll failed: {err:#}");
            app_error_from_maimai(err)
        })?;
    }
    state.timer_reset_notify.notify_one();
    Ok(StatusCode::OK)
}
