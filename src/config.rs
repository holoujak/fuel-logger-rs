use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
}

fn default_listen_addr() -> String {
    "0.0.0.0:8000".to_string()
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

        let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| default_listen_addr());

        Ok(Config { listen_addr })
    }
}
