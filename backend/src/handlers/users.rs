use axum::{
    extract::{Path, Query, State},
    response::Response,
    Extension,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, ApiJson};
use crate::{
    auth::{session, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        course::CourseRef,
        user::{
            AdminUpdateUserRequest, CreateUserRequest, CreateUserResponse, ResetLinkResponse, Role, StudentListItem, UpdateProfileRequest,
            User,
        },
    },
    services::{accounts, audit},
    util::{is_valid_email, is_valid_timezone, new_id, now, Validator},
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct UserListQuery {
    /// `student` (default) or `admin`.
    pub role: Option<String>,
    pub q: Option<String>,
    pub course_id: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/users", tag = "users", params(UserListQuery), responses((status = 200, body = [StudentListItem])))]
pub async fn list(State(state): State<AppState>, _admin: AdminUser, Query(q): Query<UserListQuery>) -> AppResult<Response> {
    let role = if q.role.as_deref() == Some("admin") { "admin" } else { "student" };
    let like = q.q.as_deref().map(|s| format!("%{}%", s.trim())).unwrap_or_else(|| "%".into());
    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users u WHERE role = ? AND (name LIKE ? OR email LIKE ?)
         AND (? IS NULL OR EXISTS (SELECT 1 FROM course_enrollments e WHERE e.user_id = u.id AND e.course_id = ?))
         ORDER BY is_active DESC, name COLLATE NOCASE",
    )
    .bind(role)
    .bind(&like)
    .bind(&like)
    .bind(&q.course_id)
    .bind(&q.course_id)
    .fetch_all(&state.pool)
    .await?;
    let enrollments: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT e.user_id, c.id, c.title, c.slug FROM course_enrollments e JOIN courses c ON c.id = e.course_id ORDER BY c.title",
    )
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<StudentListItem> = users
        .into_iter()
        .map(|u| {
            let courses = enrollments
                .iter()
                .filter(|(uid, ..)| uid == &u.id)
                .map(|(_, id, title, slug)| CourseRef { id: id.clone(), title: title.clone(), slug: slug.clone() })
                .collect();
            StudentListItem { user: u, courses }
        })
        .collect();
    Ok(json(items))
}

async fn load_user(state: &AppState, id: &str) -> AppResult<User> {
    sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("User"))
}

#[utoipa::path(post, path = "/api/v1/users", tag = "users", request_body = CreateUserRequest, responses((status = 201, body = CreateUserResponse)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<CreateUserRequest>,
) -> AppResult<Response> {
    let tz = state.settings.read().map(|s| s.default_timezone.clone()).unwrap_or_else(|_| "UTC".into());
    let user =
        accounts::create_user(&state.pool, &req.email, &req.name, Role::Student, req.password.filter(|p| !p.is_empty()), &tz).await?;
    let ts = now();
    for cid in &req.course_ids {
        sqlx::query("INSERT OR IGNORE INTO course_enrollments (id, course_id, user_id, enrolled_at) VALUES (?, ?, ?, ?)")
            .bind(new_id())
            .bind(cid)
            .bind(&user.id)
            .bind(&ts)
            .execute(&state.pool)
            .await?;
    }
    let setup_link = if user.must_change_password {
        let (token, _) = accounts::create_reset_token(&state.pool, &user.id, Some(&admin.id), 72).await?;
        Some(format!("{}/reset-password?token={token}&setup=1", state.config.public_url))
    } else {
        None
    };
    audit::log(
        &state.pool,
        Some(&admin.id),
        "user.create",
        "user",
        Some(&user.id),
        serde_json::json!({ "email": user.email }),
        ip.0.as_deref(),
    )
    .await;
    Ok(created(CreateUserResponse { user, setup_link }))
}

#[utoipa::path(get, path = "/api/v1/users/{id}", tag = "users", params(("id" = String, Path)), responses((status = 200, body = User)))]
pub async fn get(State(state): State<AppState>, _admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    Ok(json(load_user(&state, &id).await?))
}

#[utoipa::path(patch, path = "/api/v1/users/{id}", tag = "users", params(("id" = String, Path)), request_body = AdminUpdateUserRequest, responses((status = 200, body = User)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<AdminUpdateUserRequest>,
) -> AppResult<Response> {
    let user = load_user(&state, &id).await?;
    let name = req.name.map(|n| n.trim().to_string()).unwrap_or(user.name.clone());
    let email = req.email.map(|e| e.trim().to_lowercase()).unwrap_or(user.email.clone());
    let tz = req.timezone.unwrap_or(user.timezone.clone());
    let is_active = req.is_active.unwrap_or(user.is_active);
    let mut v = Validator::new();
    v.len(&name, "name", 1, 120)
        .check(is_valid_email(&email), "email", "Enter a valid email address")
        .check(is_valid_timezone(&tz), "timezone", "Unknown timezone")
        .check(!(user.id == admin.id && !is_active), "is_active", "You cannot deactivate your own account")
        .finish()?;
    sqlx::query("UPDATE users SET name = ?, email = ?, timezone = ?, is_active = ?, updated_at = ? WHERE id = ?")
        .bind(&name)
        .bind(&email)
        .bind(&tz)
        .bind(is_active)
        .bind(now())
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| match AppError::from(e) {
            AppError::Conflict(_) => AppError::field("email", "An account with this email already exists"),
            other => other,
        })?;
    if user.is_active && !is_active {
        session::revoke_all_for_user(&state.pool, &id, None).await?;
    }
    audit::log(
        &state.pool,
        Some(&admin.id),
        "user.update",
        "user",
        Some(&id),
        serde_json::json!({ "is_active": is_active, "email_changed": email != user.email }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(load_user(&state, &id).await?))
}

#[utoipa::path(post, path = "/api/v1/users/{id}/reset-link", tag = "users", params(("id" = String, Path)), responses((status = 200, body = ResetLinkResponse)))]
pub async fn reset_link(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let user = load_user(&state, &id).await?;
    let (token, expires_at) = accounts::create_reset_token(&state.pool, &user.id, Some(&admin.id), 24).await?;
    audit::log(&state.pool, Some(&admin.id), "user.reset_link", "user", Some(&id), serde_json::json!({}), ip.0.as_deref()).await;
    Ok(json(ResetLinkResponse { link: format!("{}/reset-password?token={token}", state.config.public_url), expires_at }))
}

#[utoipa::path(post, path = "/api/v1/users/{id}/revoke-sessions", tag = "users", params(("id" = String, Path)), responses((status = 200, body = super::CountResponse)))]
pub async fn revoke_sessions(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    load_user(&state, &id).await?;
    let except = (id == admin.id).then(|| admin.0.session_id.clone());
    let n = session::revoke_all_for_user(&state.pool, &id, except.as_deref()).await?;
    audit::log(&state.pool, Some(&admin.id), "user.revoke_sessions", "user", Some(&id), serde_json::json!({ "count": n }), ip.0.as_deref())
        .await;
    Ok(json(super::CountResponse { count: n as i64 }))
}

#[utoipa::path(patch, path = "/api/v1/me", tag = "users", request_body = UpdateProfileRequest, responses((status = 200, body = User)))]
pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    ApiJson(req): ApiJson<UpdateProfileRequest>,
) -> AppResult<Response> {
    let name = req.name.map(|n| n.trim().to_string()).unwrap_or(user.name.clone());
    let bio = req.bio.unwrap_or(user.bio.clone());
    let tz = req.timezone.unwrap_or(user.timezone.clone());
    Validator::new()
        .len(&name, "name", 1, 120)
        .len(&bio, "bio", 0, 1000)
        .check(is_valid_timezone(&tz), "timezone", "Unknown timezone")
        .finish()?;
    sqlx::query("UPDATE users SET name = ?, bio = ?, timezone = ?, updated_at = ? WHERE id = ?")
        .bind(&name)
        .bind(&bio)
        .bind(&tz)
        .bind(now())
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    Ok(json(load_user(&state, &user.id).await?))
}
