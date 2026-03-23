use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct StationConfig {
    pub id: u32,
    pub name: String,
    pub keyboard_d0_gpio: u8,
    pub keyboard_d1_gpio: u8,
    pub keyboard_led_gpio: u8,
    pub relay_gpio: u8,
    pub start_gpio: u8,
    pub stop_gpio: u8,
    pub pause_gpio: Option<u8>,
    pub buzzer_gpio: Option<u8>,
    pub flow_meter_gpio: Option<u8>,
    pub camera_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_snapshot_dir")]
    pub snapshot_dir: String,
    /// Optional HTTP Basic Auth username. Auth is disabled when not set.
    pub auth_user: Option<String>,
    pub auth_pass: Option<String>,
    pub stations: Vec<StationConfig>,
}

fn default_listen_addr() -> String {
    "0.0.0.0:8000".to_string()
}

fn default_snapshot_dir() -> String {
    "./snapshots".to_string()
}

/// Hardcoded station configs matching the original Python setup.
fn default_stations() -> Vec<StationConfig> {
    vec![
        StationConfig {
            id: 1,
            name: "S1".to_string(),
            keyboard_d0_gpio: 17,
            keyboard_d1_gpio: 18,
            keyboard_led_gpio: 27,
            relay_gpio: 10,
            start_gpio: 22,
            stop_gpio: 23,
            pause_gpio: Some(24),
            buzzer_gpio: Some(9),
            flow_meter_gpio: None,
            camera_url: None,
        },
        StationConfig {
            id: 2,
            name: "S2".to_string(),
            keyboard_d0_gpio: 11,
            keyboard_d1_gpio: 8,
            keyboard_led_gpio: 7,
            relay_gpio: 13,
            start_gpio: 5,
            stop_gpio: 6,
            pause_gpio: Some(12),
            buzzer_gpio: Some(19),
            flow_meter_gpio: Some(26),
            camera_url: None,
        },
    ]
}

impl Config {
    /// Load config from `config.toml` if it exists, otherwise fall back to
    /// environment variables + hardcoded station definitions.
    pub fn from_env() -> Result<Self> {
        // Try loading from config.toml first
        if let Ok(content) = std::fs::read_to_string("config.toml") {
            let config: Config = toml::from_str(&content).context("Failed to parse config.toml")?;
            info!("Loaded configuration from config.toml");
            return Ok(config);
        }

        info!("No config.toml found, using environment variables + defaults");

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:fuelloggerrs.db?mode=rwc".to_string());
        let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| default_listen_addr());
        let stations = default_stations();
        let snapshot_dir =
            std::env::var("RTSP_SNAPSHOT_DIR").unwrap_or_else(|_| default_snapshot_dir());

        let auth_user = std::env::var("AUTH_USER").ok();
        let auth_pass = std::env::var("AUTH_PASS").ok();

        Ok(Config {
            database_url,
            listen_addr,
            stations,
            snapshot_dir,
            auth_user,
            auth_pass,
        })
    }
}
