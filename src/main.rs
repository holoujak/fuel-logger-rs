use anyhow::Result;
use tokio::signal;
use tracing::info;

mod config;
mod gpio;
mod routes;
mod state;

use config::Config;
use gpio::GpioController;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fuel_logger=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    info!("Starting fuel-logger-rs with config: {:?}", config);

    // GPIO controller
    let gpio = GpioController::new()?;

    // Axum web server
    let shared = state::SharedState::new(state::AppState::new().into());
    let app = routes::router(shared.clone());
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    info!("Web server listening on {}", config.listen_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down...");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
