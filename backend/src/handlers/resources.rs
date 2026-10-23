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
    models::misc::{Resource, ResourceInput, RESOURCE_KINDS},
    repositories::resources::{self as repo, ResourceFilter},
    services::{audit, storage},
    util::{new_id, now, Validator},
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct ResourceQuery {
    pub course_id: Option<String>,
    pub module_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
    /// `newest`, `oldest`, `title` or `week`.
    pub sort: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/resources", tag = "resources", params(ResourceQuery), responses((status = 200, body = [Resource])))]
pub async fn list(State(state): State<AppState>, user: AuthUser, Query(q): Query<ResourceQuery>) -> AppResult<Response> {
    let f = ResourceFilter {
        course_id: q.course_id,
        module_id: q.module_id,
        week_id: q.week_id,
        lesson_id: q.lesson_id,
        kind: q.kind,
        q: q.q,
        sort: q.sort,
        id: None,
    };
    Ok(json(repo::list(&state.pool, &user, &f, &state.today()).await?))
}

async fn load(state: &AppState, user: &crate::models::user::User, id: &str) -> AppResult<Resource> {
    let f = ResourceFilter { id: Some(id.to_string()), ..Default::default() };
    repo::list(&state.pool, user, &f, &state.today()).await?.into_iter().next().ok_or(AppError::NotFound("Resource"))
}

#[utoipa::path(get, path = "/api/v1/resources/{id}", tag = "resources", params(("id" = String, Path)), responses((status = 200, body = Resource)))]
pub async fn get(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    Ok(json(load(&state, &user, &id).await?))
}

struct Resolved {
    course_id: String,
    module_id: Option<String>,
    week_id: Option<String>,
    lesson_id: Option<String>,
    title: String,
    description: String,
    kind: String,
    file_id: Option<String>,
    url: Option<String>,
    content_md: String,
    visibility: String,
}

async fn resolve(state: &AppState, req: ResourceInput, base: Option<&Resource>) -> AppResult<Resolved> {
    let opt = |v: Option<String>, b: Option<Option<String>>| match v {
        Some(s) if s.trim().is_empty() => None,
        Some(s) => Some(s.trim().to_string()),
        None => b.flatten(),
    };
    let course_id = match (base, req.course_id) {
        (Some(b), _) => b.course_id.clone(),
        (None, Some(c)) => c,
        (None, None) => return Err(AppError::field("course_id", "Select a course")),
    };
    let r = Resolved {
        course_id,
        module_id: opt(req.module_id, base.map(|b| b.module_id.clone())),
        week_id: opt(req.week_id, base.map(|b| b.week_id.clone())),
        lesson_id: opt(req.lesson_id, base.map(|b| b.lesson_id.clone())),
        title: req.title.or(base.map(|b| b.title.clone())).unwrap_or_default().trim().to_string(),
        description: req.description.or(base.map(|b| b.description.clone())).unwrap_or_default(),
        kind: req.kind.or(base.map(|b| b.kind.clone())).unwrap_or_default(),
        file_id: opt(req.file_id, base.map(|b| b.file_id.clone())),
        url: opt(req.url, base.map(|b| b.url.clone())),
        content_md: req.content_md.or(base.map(|b| b.content_md.clone())).unwrap_or_default(),
        visibility: req.visibility.or(base.map(|b| b.visibility.clone())).unwrap_or_else(|| "students".into()),
    };
    let has_content = r.file_id.is_some() || r.url.is_some() || !r.content_md.trim().is_empty();
    Validator::new()
        .len(&r.title, "title", 1, 200)
        .len(&r.description, "description", 0, 5000)
        .len(&r.content_md, "content_md", 0, 200_000)
        .check(RESOURCE_KINDS.contains(&r.kind.as_str()), "kind", "Choose a resource type")
        .check(matches!(r.visibility.as_str(), "students" | "instructor"), "visibility", "Unknown visibility")
        .url(r.url.as_deref(), "url")
        .check(!(matches!(r.kind.as_str(), "link" | "video") && r.url.is_none()), "url", "A URL is required for links and videos")
        .check(has_content, "file_id", "Attach a file, add a URL or write content")
        .finish()?;
    for (table, col, val) in [("modules", "module_id", &r.module_id), ("weeks", "week_id", &r.week_id)] {
        if let Some(v) = val {
            let ok: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE id = ? AND course_id = ?"))
                .bind(v)
                .bind(&r.course_id)
                .fetch_one(&state.pool)
                .await?;
            if ok == 0 {
                return Err(AppError::field(col, "Must belong to the selected course"));
            }
        }
    }
    if let Some(l) = &r.lesson_id {
        let ok: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM lessons l JOIN weeks w ON w.id = l.week_id WHERE l.id = ? AND w.course_id = ?")
                .bind(l)
                .bind(&r.course_id)
                .fetch_one(&state.pool)
                .await?;
        if ok == 0 {
            return Err(AppError::field("lesson_id", "Must belong to the selected course"));
        }
    }
    if let Some(f) = &r.file_id {
        let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id = ?").bind(f).fetch_one(&state.pool).await?;
        if ok == 0 {
            return Err(AppError::field("file_id", "Uploaded file not found"));
        }
    }
    Ok(r)
}

#[utoipa::path(post, path = "/api/v1/resources", tag = "resources", request_body = ResourceInput, responses((status = 201, body = Resource)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<ResourceInput>,
) -> AppResult<Response> {
    let mut r = resolve(&state, req, None).await?;
    access::course_for(&state.pool, &admin, &r.course_id).await?;
    // Lessons imply their week.
    if let (Some(l), None) = (&r.lesson_id, &r.week_id) {
        r.week_id = sqlx::query_scalar("SELECT week_id FROM lessons WHERE id = ?").bind(l).fetch_optional(&state.pool).await?;
    }
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO resources (id, course_id, module_id, week_id, lesson_id, title, description, kind, file_id, url, content_md, visibility, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&r.course_id)
    .bind(&r.module_id)
    .bind(&r.week_id)
    .bind(&r.lesson_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.kind)
    .bind(&r.file_id)
    .bind(&r.url)
    .bind(&r.content_md)
    .bind(&r.visibility)
    .bind(&admin.id)
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "resource.create",
        "resource",
        Some(&id),
        serde_json::json!({ "title": r.title }),
        ip.0.as_deref(),
    )
    .await;
    Ok(created(load(&state, &admin, &id).await?))
}

#[utoipa::path(patch, path = "/api/v1/resources/{id}", tag = "resources", params(("id" = String, Path)), request_body = ResourceInput, responses((status = 200, body = Resource)))]
pub async fn update(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ResourceInput>,
) -> AppResult<Response> {
    let existing = load(&state, &admin, &id).await?;
    let r = resolve(&state, req, Some(&existing)).await?;
    sqlx::query(
        "UPDATE resources SET module_id = ?, week_id = ?, lesson_id = ?, title = ?, description = ?, kind = ?, file_id = ?, url = ?,
         content_md = ?, visibility = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&r.module_id)
    .bind(&r.week_id)
    .bind(&r.lesson_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.kind)
    .bind(&r.file_id)
    .bind(&r.url)
    .bind(&r.content_md)
    .bind(&r.visibility)
    .bind(now())
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if let Some(old) = existing.file_id.filter(|f| Some(f) != r.file_id.as_ref()) {
        storage::delete_if_orphaned(&state, &old).await?;
    }
    Ok(json(load(&state, &admin, &id).await?))
}

#[utoipa::path(delete, path = "/api/v1/resources/{id}", tag = "resources", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let existing = load(&state, &admin, &id).await?;
    sqlx::query("DELETE FROM resources WHERE id = ?").bind(&id).execute(&state.pool).await?;
    if let Some(f) = &existing.file_id {
        storage::delete_if_orphaned(&state, f).await?;
    }
    audit::log(
        &state.pool,
        Some(&admin.id),
        "resource.delete",
        "resource",
        Some(&id),
        serde_json::json!({ "title": existing.title }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}
