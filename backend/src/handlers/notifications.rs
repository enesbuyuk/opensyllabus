use axum::{
    extract::{Path, Query, State},
    response::Response,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{json, no_content};
use crate::{
    auth::AuthUser,
    errors::{AppError, AppResult},
    models::misc::{Notification, NotificationList},
    util::now,
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct NotificationQuery {
    #[serde(default)]
    pub unread_only: bool,
    pub limit: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/notifications", tag = "notifications", params(NotificationQuery), responses((status = 200, body = NotificationList)))]
pub async fn list(State(state): State<AppState>, user: AuthUser, Query(q): Query<NotificationQuery>) -> AppResult<Response> {
    let items: Vec<Notification> = sqlx::query_as(
        "SELECT id, kind, title, body, link, read_at, created_at FROM notifications
         WHERE user_id = ? AND (? = 0 OR read_at IS NULL) ORDER BY created_at DESC LIMIT ?",
    )
    .bind(&user.id)
    .bind(q.unread_only)
    .bind(q.limit.unwrap_or(50).clamp(1, 200))
    .fetch_all(&state.pool)
    .await?;
    let unread_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read_at IS NULL")
        .bind(&user.id)
        .fetch_one(&state.pool)
        .await?;
    Ok(json(NotificationList { items, unread_count }))
}

#[utoipa::path(post, path = "/api/v1/notifications/{id}/read", tag = "notifications", params(("id" = String, Path)), responses((status = 204)))]
pub async fn mark_read(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let res = sqlx::query("UPDATE notifications SET read_at = COALESCE(read_at, ?) WHERE id = ? AND user_id = ?")
        .bind(now())
        .bind(&id)
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Notification"));
    }
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/notifications/read-all", tag = "notifications", responses((status = 204)))]
pub async fn mark_all_read(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    sqlx::query("UPDATE notifications SET read_at = ? WHERE user_id = ? AND read_at IS NULL")
        .bind(now())
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    Ok(no_content())
}
