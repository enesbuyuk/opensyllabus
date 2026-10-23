use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::AppState;

#[utoipa::path(get, path = "/api/health", tag = "health", responses((status = 200, description = "Process is alive")))]
pub async fn live() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

#[utoipa::path(get, path = "/api/health/ready", tag = "health", responses((status = 200), (status = 503)))]
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let db = sqlx::query_scalar::<_, i64>("SELECT 1").fetch_one(&state.pool).await.is_ok();
    let storage = tokio::fs::metadata(&state.config.storage_dir).await.map(|m| m.is_dir()).unwrap_or(false);
    let status = if db && storage { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(json!({ "status": if status == StatusCode::OK { "ok" } else { "degraded" }, "database": db, "storage": storage })))
}
