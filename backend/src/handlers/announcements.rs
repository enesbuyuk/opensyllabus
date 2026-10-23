use axum::{
    extract::{Path, Query, State},
    response::Response,
    Extension,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, no_content, ApiJson};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        misc::{Announcement, AnnouncementInput},
        user::User,
    },
    services::{
        audit,
        notifications::{self, NewNotification},
    },
    util::{new_id, now, Validator},
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct AnnouncementQuery {
    pub course_id: Option<String>,
    pub limit: Option<i64>,
}

const SELECT: &str = "SELECT a.id, a.course_id, c.title AS course_title, a.author_id, u.name AS author_name, a.title, a.body_md, a.status,
    a.pinned, a.published_at, a.created_at, a.updated_at
    FROM announcements a JOIN courses c ON c.id = a.course_id LEFT JOIN users u ON u.id = a.author_id";

pub async fn list_for(state: &AppState, user: &User, course_id: Option<&str>, limit: i64) -> AppResult<Vec<Announcement>> {
    let ids = access::visible_course_ids(&state.pool, user).await?;
    let mut out: Vec<Announcement> = Vec::new();
    for cid in ids.iter().filter(|c| course_id.is_none_or(|f| f == c.as_str())) {
        let rows: Vec<Announcement> = if user.is_admin() {
            sqlx::query_as(&format!("{SELECT} WHERE a.course_id = ?")).bind(cid).fetch_all(&state.pool).await?
        } else {
            sqlx::query_as(&format!("{SELECT} WHERE a.course_id = ? AND a.status = 'published'")).bind(cid).fetch_all(&state.pool).await?
        };
        out.extend(rows);
    }
    out.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.published_at.as_deref().unwrap_or(&b.created_at).cmp(a.published_at.as_deref().unwrap_or(&a.created_at)))
    });
    out.truncate(limit.clamp(1, 500) as usize);
    Ok(out)
}

#[utoipa::path(get, path = "/api/v1/announcements", tag = "announcements", params(AnnouncementQuery), responses((status = 200, body = [Announcement])))]
pub async fn list(State(state): State<AppState>, user: AuthUser, Query(q): Query<AnnouncementQuery>) -> AppResult<Response> {
    Ok(json(list_for(&state, &user, q.course_id.as_deref(), q.limit.unwrap_or(200)).await?))
}

async fn load(state: &AppState, id: &str) -> AppResult<Announcement> {
    sqlx::query_as(&format!("{SELECT} WHERE a.id = ?"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("Announcement"))
}

async fn notify_published(state: &AppState, a: &Announcement) -> AppResult<()> {
    notifications::notify_course(
        &state.pool,
        &a.course_id,
        &NewNotification {
            kind: "announcement",
            title: a.title.clone(),
            body: format!("New announcement in {}", a.course_title),
            link: Some(format!("/announcements#{}", a.id)),
            dedupe: Some(format!("announcement:{}", a.id)),
        },
    )
    .await
}

#[utoipa::path(post, path = "/api/v1/announcements", tag = "announcements", request_body = AnnouncementInput, responses((status = 201, body = Announcement)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<AnnouncementInput>,
) -> AppResult<Response> {
    let course_id = req.course_id.ok_or_else(|| AppError::field("course_id", "Select a course"))?;
    access::course_for(&state.pool, &admin, &course_id).await?;
    let title = req.title.unwrap_or_default();
    let body = req.body_md.unwrap_or_default();
    let status = req.status.unwrap_or_else(|| "draft".into());
    Validator::new()
        .len(&title, "title", 1, 200)
        .len(&body, "body_md", 0, 50_000)
        .check(matches!(status.as_str(), "draft" | "published"), "status", "Unknown status")
        .finish()?;
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO announcements (id, course_id, author_id, title, body_md, status, pinned, published_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&course_id)
    .bind(&admin.id)
    .bind(title.trim())
    .bind(&body)
    .bind(&status)
    .bind(req.pinned.unwrap_or(false))
    .bind((status == "published").then_some(&ts))
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await?;
    let a = load(&state, &id).await?;
    if status == "published" {
        notify_published(&state, &a).await?;
    }
    audit::log(
        &state.pool,
        Some(&admin.id),
        "announcement.create",
        "announcement",
        Some(&id),
        serde_json::json!({ "status": status }),
        ip.0.as_deref(),
    )
    .await;
    Ok(created(a))
}

#[utoipa::path(patch, path = "/api/v1/announcements/{id}", tag = "announcements", params(("id" = String, Path)), request_body = AnnouncementInput, responses((status = 200, body = Announcement)))]
pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<AnnouncementInput>,
) -> AppResult<Response> {
    let a = load(&state, &id).await?;
    let title = req.title.unwrap_or(a.title.clone());
    let body = req.body_md.unwrap_or(a.body_md.clone());
    let status = req.status.unwrap_or(a.status.clone());
    Validator::new()
        .len(&title, "title", 1, 200)
        .len(&body, "body_md", 0, 50_000)
        .check(matches!(status.as_str(), "draft" | "published"), "status", "Unknown status")
        .finish()?;
    let ts = now();
    let published_at = if status == "published" { a.published_at.clone().or(Some(ts.clone())) } else { a.published_at.clone() };
    sqlx::query("UPDATE announcements SET title = ?, body_md = ?, status = ?, pinned = ?, published_at = ?, updated_at = ? WHERE id = ?")
        .bind(title.trim())
        .bind(&body)
        .bind(&status)
        .bind(req.pinned.unwrap_or(a.pinned))
        .bind(&published_at)
        .bind(&ts)
        .bind(&id)
        .execute(&state.pool)
        .await?;
    let updated = load(&state, &id).await?;
    if status == "published" && a.status != "published" {
        notify_published(&state, &updated).await?;
    }
    Ok(json(updated))
}

#[utoipa::path(delete, path = "/api/v1/announcements/{id}", tag = "announcements", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    load(&state, &id).await?;
    sqlx::query("DELETE FROM announcements WHERE id = ?").bind(&id).execute(&state.pool).await?;
    audit::log(&state.pool, Some(&admin.id), "announcement.delete", "announcement", Some(&id), serde_json::json!({}), ip.0.as_deref())
        .await;
    Ok(no_content())
}
