use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

mod config;
mod db;
mod gpio;
mod models;
mod routes;
mod state;
mod station;
mod wiegand;

use config::Config;
use gpio::GpioController;
use station::StationManager;

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

    // Database pool
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    // GPIO controller
    let gpio = GpioController::new()?;

    // Station manager (owns all station state + GPIO logic)
    let manager = Arc::new(StationManager::new(config.clone(), pool.clone(), gpio)?);

    // Start hardware loops (Wiegand readers, button polling, flow meters)
    let manager_hw = manager.clone();
    let hw_handle = tokio::spawn(async move {
        if let Err(e) = manager_hw.run_hardware_loop().await {
            warn!("Hardware loop error: {e}");
        }
    });

    // Axum web server
    let shared = state::AppState::new(pool, manager.clone());
    let app = routes::router(shared);
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    info!("Web server listening on {}", config.listen_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down...");
    manager.shutdown().await;
    hw_handle.abort();
    info!("Finished.");
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
