use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};

use crate::models::*;
use crate::state::AppState;
use axum::response::Html;

use tower_http::{cors::CorsLayer, trace::TraceLayer};

async fn frontend() -> Html<&'static str> {
    Html(include_str!("../web/dist/index.html"))
}

pub fn router(shared: AppState) -> Router {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route("/", get(frontend))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(shared)
        .fallback(get(frontend))
}

// ─── User handlers ──────────────────────────────────────────────────────────

async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    sqlx::query_as::<_, User>("SELECT id, name, tag, station1, station2 FROM users ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<User>, (StatusCode, String)> {
    sqlx::query_as::<_, User>("SELECT id, name, tag, station1, station2 FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), (StatusCode, String)> {
    let result =
        sqlx::query("INSERT INTO users (name, tag, station1, station2) VALUES (?, ?, ?, ?)")
            .bind(&payload.name)
            .bind(&payload.tag)
            .bind(payload.station1)
            .bind(payload.station2)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = result.last_insert_rowid() as i32;
    let user = User {
        id,
        name: payload.name,
        tag: payload.tag,
        station1: payload.station1,
        station2: payload.station2,
    };
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<User>, (StatusCode, String)> {
    // Fetch current
    let current = sqlx::query_as::<_, User>(
        "SELECT id, name, tag, station1, station2 FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let name = payload.name.unwrap_or(current.name);
    let tag = payload.tag.unwrap_or(current.tag);
    let station1 = payload.station1.unwrap_or(current.station1);
    let station2 = payload.station2.unwrap_or(current.station2);

    sqlx::query("UPDATE users SET name = ?, tag = ?, station1 = ?, station2 = ? WHERE id = ?")
        .bind(&name)
        .bind(&tag)
        .bind(station1)
        .bind(station2)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(User {
        id,
        name,
        tag,
        station1,
        station2,
    }))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        Err((StatusCode::NOT_FOUND, "User not found".to_string()))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
