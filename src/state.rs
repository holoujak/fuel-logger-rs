use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::Config;
use crate::station::StationManager;
#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub manager: Arc<StationManager>,
    pub config: Config,
}

impl AppState {
    pub fn new(pool: SqlitePool, manager: Arc<StationManager>, config: Config) -> Self {
        Self {
            pool,
            manager,
            config,
        }
    }
}
