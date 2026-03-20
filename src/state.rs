use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Default)]
pub struct AppState {}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
