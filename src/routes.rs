use axum::{
    extract::{Path, Query, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{Html, Json, Response},
    routing::get,
    Router,
};
use base64::Engine;

use crate::models::*;
use crate::state::AppState;

use tower_http::{cors::CorsLayer, trace::TraceLayer};

async fn frontend() -> Html<&'static str> {
    Html(include_str!("../web/dist/index.html"))
}

/// HTTP Basic Auth middleware.
/// When `auth_user` and `auth_pass` are set in config, every request must
/// carry a valid `Authorization: Basic <base64>` header. If the credentials
/// are not configured, all requests are allowed through.
async fn basic_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, [(header::HeaderName, &'static str); 1])> {
    let (Some(user), Some(pass)) = (&state.config.auth_user, &state.config.auth_pass) else {
        return Ok(next.run(request).await);
    };

    let unauthorized = Err((
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic realm=\"fuel-logger\"")],
    ));

    let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return unauthorized;
    };

    let Some(encoded) = auth_header.strip_prefix("Basic ") else {
        return unauthorized;
    };

    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return unauthorized;
    };

    let Ok(credentials) = String::from_utf8(decoded) else {
        return unauthorized;
    };

    let expected = format!("{user}:{pass}");
    if credentials == expected {
        Ok(next.run(request).await)
    } else {
        unauthorized
    }
}

pub fn router(shared: AppState) -> Router {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route("/api/stations", get(get_stations))
        .route("/api/logs", get(list_logs))
        .route("/api/logs/{id}", get(get_log))
        .route("/api/stats", get(get_stats))
        .route("/api/snapshots/{station_id}/{filename}", get(get_snapshot))
        .route("/", get(frontend))
        .layer(middleware::from_fn_with_state(shared.clone(), basic_auth))
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

// ─── Station handlers ───────────────────────────────────────────────────────

async fn get_stations(State(state): State<AppState>) -> Json<Vec<StationInfo>> {
    Json(state.manager.get_stations_info())
}

// ─── Log handlers ───────────────────────────────────────────────────────────

async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<LogQuery>,
) -> Result<Json<Vec<Log>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // Build query dynamically to avoid duplicating 4 near-identical branches
    let mut sql = String::from(
        "SELECT id, user_id, created_at, station, length, consumption, snapshot_path FROM logs",
    );
    let mut conditions = Vec::new();

    if params.station.is_some() {
        conditions.push("station = ?");
    }
    if params.user_id.is_some() {
        conditions.push("user_id = ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<_, Log>(&sql);
    if let Some(station) = params.station {
        query = query.bind(station);
    }
    if let Some(user_id) = params.user_id {
        query = query.bind(user_id);
    }
    query = query.bind(limit).bind(offset);

    query
        .fetch_all(&state.pool)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_log(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Log>, (StatusCode, String)> {
    sqlx::query_as::<_, Log>(
        "SELECT id, user_id, created_at, station, length, consumption, snapshot_path FROM logs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map(Json)
    .ok_or((StatusCode::NOT_FOUND, "Log not found".to_string()))
}

// ─── Stats handler ──────────────────────────────────────────────────────────

async fn get_stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<Vec<UserStats>>, (StatusCode, String)> {
    let mut sql = String::from(
        r#"SELECT
            l.user_id,
            u.name AS user_name,
            COALESCE(SUM(l.consumption), 0.0) AS total_liters,
            COALESCE(SUM(l.length), 0) AS total_seconds,
            COUNT(*) AS refuel_count
        FROM logs l
        JOIN users u ON u.id = l.user_id
        WHERE 1=1"#,
    );

    if params.from.is_some() {
        sql.push_str(" AND l.created_at >= ?");
    }
    if params.to.is_some() {
        sql.push_str(" AND l.created_at <= ?");
    }
    if params.station.is_some() {
        sql.push_str(" AND l.station = ?");
    }

    sql.push_str(" GROUP BY l.user_id ORDER BY total_liters DESC");

    let mut query = sqlx::query_as::<_, UserStats>(&sql);
    if let Some(ref from) = params.from {
        query = query.bind(from);
    }
    if let Some(ref to) = params.to {
        query = query.bind(to);
    }
    if let Some(station) = params.station {
        query = query.bind(station);
    }

    query
        .fetch_all(&state.pool)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ─── Snapshot handler ───────────────────────────────────────────────────────

async fn get_snapshot(
    State(state): State<AppState>,
    Path((station_id, filename)): Path<(String, String)>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::body::Body;
    use axum::response::IntoResponse;

    // Sanitize: only allow simple components (no path traversal)
    if station_id.contains('.')
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid path".to_string()));
    }

    let path = std::path::Path::new(&state.config.snapshot_dir)
        .join(&station_id)
        .join(&filename);
    let data = tokio::fs::read(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Snapshot not found".to_string()))?;

    let content_type = if filename.ends_with(".png") {
        "image/png"
    } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };

    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        Body::from(data),
    )
        .into_response())
}
