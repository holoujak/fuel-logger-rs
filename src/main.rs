use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

mod config;
mod db;
mod gpio;
mod modbus;
mod models;
mod routes;
mod snapshot;
mod state;
mod station;
mod tuf2000;
mod wiegand;

use config::Config;
use gpio::GpioController;
use modbus::Rs485Modbus;
use station::StationManager;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    info!("Starting fuel-logger-rs with config: {:?}", config);

    // Database pool
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    // GPIO controller
    let gpio = GpioController::new()?;

    // RS485 / MODBUS ASCII must be initialized before StationManager takes GPIO ownership
    let modbus = if let Some(ref mb_cfg) = config.modbus {
        info!(
            "MODBUS enabled: port={}, baud={}, re_gpio={:?}, de_gpio={:?}",
            mb_cfg.port, mb_cfg.baud, mb_cfg.re_gpio, mb_cfg.de_gpio,
        );
        let mb = Rs485Modbus::new(
            &gpio,
            &mb_cfg.port,
            mb_cfg.baud,
            mb_cfg.re_gpio,
            mb_cfg.de_gpio,
        )?;
        Some(Arc::new(tokio::sync::Mutex::new(mb)))
    } else {
        warn!("MODBUS is disabled (missing [modbus] section in config.toml)");
        None
    };

    let tuf2000_client = modbus.clone().map(tuf2000::Tuf2000Client::new);

    // Station manager (owns all station state + GPIO logic)
    let manager = Arc::new(StationManager::new(
        config.clone(),
        pool.clone(),
        gpio,
        tuf2000_client,
    )?);

    // Start hardware loops (Wiegand readers, button polling, flow meters)
    let manager_hw = manager.clone();
    let hw_handle = tokio::spawn(async move {
        if let Err(e) = manager_hw.run_hardware_loop().await {
            warn!("Hardware loop error: {e}");
        }
    });

    // Axum web server
    let shared = state::AppState::new(pool, manager.clone(), config.clone());
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
