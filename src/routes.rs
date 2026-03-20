use axum::{routing::get, Router};

use crate::state::SharedState;
use axum::response::Html;

use tower_http::cors::CorsLayer;

async fn frontend() -> Html<&'static str> {
    Html(include_str!("../web/dist/index.html"))
}

pub fn router(shared: SharedState) -> Router {
    Router::new()
        .route("/", get(frontend))
        .layer(CorsLayer::permissive())
        .with_state(shared)
}
