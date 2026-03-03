use std::time::Duration;

use tokio::time::interval;
use tracing::{error, info};

use crate::state::AppState;
use crate::tasks::polling::cycle::run_cycle;

pub(crate) fn start_background_polling(app_state: AppState) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(600));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!("Background polling started: periodic playerData poll (every 10 minutes)");

        loop {
            timer.tick().await;

            match run_cycle(&app_state).await {
                Ok(report) => info!(
                    "Periodic poll finished: maintenance_skip={} recent_present={}",
                    report.skipped_for_maintenance,
                    report.recent_outcome.is_some()
                ),
                Err(err) => error!("Periodic poll failed: {err:#}"),
            }
        }
    });
}
