use eyre::WrapErr;
use maistats_record_collector::{logging::init_tracing, run_server};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let log_buffer = init_tracing().wrap_err("initialize tracing")?;

    run_server(log_buffer)
        .await
        .wrap_err("run record collector server")
}
