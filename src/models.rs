use chrono::NaiveDateTime;
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

// ─── Log ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Log {
    pub id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub station: i32,
    pub length: i32,
    pub consumption: f64,
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub station: Option<i32>,
    pub user_id: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ─── Stats ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub station: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserStats {
    pub user_id: i32,
    pub user_name: String,
    pub total_liters: f64,
    pub total_seconds: i64,
    pub refuel_count: i64,
}
