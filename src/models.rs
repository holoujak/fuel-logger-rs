use serde::{Deserialize, Serialize};

// ─── User ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub tag: String,
    pub station1: bool,
    pub station2: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub tag: String,
    pub station1: bool,
    pub station2: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub tag: Option<String>,
    pub station1: Option<bool>,
    pub station2: Option<bool>,
}

// ─── Station ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StationInfo {
    pub id: u32,
    pub name: String,
    pub status: String,
    pub current_length_secs: Option<i64>,
    pub pulses_count: u64,
    pub active_user: Option<String>,
}
