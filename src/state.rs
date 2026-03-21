use sqlx::SqlitePool;
use std::sync::Arc;

use crate::station::StationManager;
#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub manager: Arc<StationManager>,
}

impl AppState {
    pub fn new(pool: SqlitePool, manager: Arc<StationManager>) -> Self {
        Self { pool, manager }
    }
}
