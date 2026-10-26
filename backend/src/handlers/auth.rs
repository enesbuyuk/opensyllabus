use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::Response,
    Extension,
};

use super::{json, no_content, ApiJson};
use crate::{
    auth::{password, session, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::user::{ChangePasswordRequest, LoginRequest, MeResponse, ResetPasswordRequest, SessionInfo, User},
    services::{accounts, audit},
    util::now,
    AppState,
};

const WINDOW: Duration = Duration::from_secs(15 * 60);

fn with_cookie(mut res: Response, cookie: cookie::Cookie<'static>) -> Response {
    if let Ok(v) = HeaderValue::from_str(&cookie.to_string()) {
        res.headers_mut().append(header::SET_COOKIE, v);
    }
    res
}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", request_body = LoginRequest,
    responses((status = 200, body = MeResponse), (status = 401, description = "Invalid credentials"), (status = 429)))]
pub async fn login(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> AppResult<Response> {
    // Email when it contains "@" (usernames never do), username otherwise.
    let login = req.login.trim().to_lowercase();
    let ip_key = format!("login-ip:{}", ip.0.as_deref().unwrap_or("unknown"));
    state.rate_limiter.check(&ip_key, 30, WINDOW)?;

    let sql = if login.contains('@') { "SELECT * FROM users WHERE email = ?" } else { "SELECT * FROM users WHERE username = ?" };
    let user: Option<User> = sqlx::query_as(sql).bind(&login).fetch_optional(&state.pool).await?;
    // Per account, so the email and the username share one budget.
    let account_key = match &user {
        Some(u) => format!("login-user:{}", u.id),
        None => format!("login-id:{login}"),
    };
    state.rate_limiter.check(&account_key, 10, WINDOW)?;
    let ok = password::verify(req.password.clone(), user.as_ref().map(|u| u.password_hash.clone())).await;
    let user = match user {
        Some(u) if ok && u.is_active => u,
        _ => {
            audit::log(&state.pool, None, "auth.login_failed", "user", None, serde_json::json!({ "login": login }), ip.0.as_deref()).await;
            return Err(AppError::InvalidCredentials);
        }
    };
    state.rate_limiter.reset(&account_key);

    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok());
    let s = session::create(&state.pool, &state.config, &user.id, ip.0.as_deref(), ua).await?;
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?").bind(now()).bind(&user.id).execute(&state.pool).await?;
    audit::log(&state.pool, Some(&user.id), "auth.login", "user", Some(&user.id), serde_json::json!({}), ip.0.as_deref()).await;

    let res = json(MeResponse { user, csrf_token: s.csrf_token });
    Ok(with_cookie(res, session::session_cookie(&state.config, s.token)))
}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth", responses((status = 204)))]
pub async fn logout(State(state): State<AppState>, auth: Option<Extension<crate::auth::AuthContext>>) -> AppResult<Response> {
    if let Some(Extension(ctx)) = auth {
        session::revoke(&state.pool, &ctx.session_id).await?;
    }
    Ok(with_cookie(no_content(), session::clear_cookie(&state.config)))
}

#[utoipa::path(get, path = "/api/v1/auth/me", tag = "auth", responses((status = 200, body = MeResponse), (status = 401)))]
pub async fn me(user: AuthUser) -> AppResult<Response> {
    Ok(json(MeResponse { user: user.0.user.clone(), csrf_token: user.0.csrf_token.clone() }))
}

#[utoipa::path(post, path = "/api/v1/auth/change-password", tag = "auth", request_body = ChangePasswordRequest, responses((status = 204)))]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    user: AuthUser,
    ApiJson(req): ApiJson<ChangePasswordRequest>,
) -> AppResult<Response> {
    state.rate_limiter.check(&format!("change-pw:{}", user.id), 10, WINDOW)?;
    if !password::verify(req.current_password, Some(user.password_hash.clone())).await {
        return Err(AppError::field("current_password", "Current password is incorrect"));
    }
    accounts::set_password(&state.pool, &user.id, req.new_password).await?;
    // Sign out every other device.
    session::revoke_all_for_user(&state.pool, &user.id, Some(&user.0.session_id)).await?;
    audit::log(&state.pool, Some(&user.id), "auth.password_changed", "user", Some(&user.id), serde_json::json!({}), ip.0.as_deref()).await;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/auth/reset-password", tag = "auth", request_body = ResetPasswordRequest, responses((status = 204), (status = 400)))]
pub async fn reset_password(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    ApiJson(req): ApiJson<ResetPasswordRequest>,
) -> AppResult<Response> {
    state.rate_limiter.check(&format!("reset-ip:{}", ip.0.as_deref().unwrap_or("unknown")), 10, WINDOW)?;
    let user = accounts::reset_password_with_token(&state.pool, req.token.trim(), req.new_password).await?;
    audit::log(&state.pool, Some(&user.id), "auth.password_reset", "user", Some(&user.id), serde_json::json!({}), ip.0.as_deref()).await;
    Ok(no_content())
}

#[utoipa::path(get, path = "/api/v1/auth/sessions", tag = "auth", responses((status = 200, body = [SessionInfo])))]
pub async fn list_sessions(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    let mut rows: Vec<SessionInfo> = sqlx::query_as(
        "SELECT id, ip_address, user_agent, created_at, last_seen_at, expires_at FROM sessions
         WHERE user_id = ? AND revoked_at IS NULL AND expires_at > ? ORDER BY last_seen_at DESC",
    )
    .bind(&user.id)
    .bind(now())
    .fetch_all(&state.pool)
    .await?;
    for r in &mut rows {
        r.current = r.id == user.0.session_id;
    }
    Ok(json(rows))
}

#[utoipa::path(delete, path = "/api/v1/auth/sessions/{id}", tag = "auth", params(("id" = String, Path)), responses((status = 204)))]
pub async fn revoke_session(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let res = sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND user_id = ? AND revoked_at IS NULL")
        .bind(now())
        .bind(&id)
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Session"));
    }
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/auth/sessions/revoke-others", tag = "auth", responses((status = 204)))]
pub async fn revoke_other_sessions(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    session::revoke_all_for_user(&state.pool, &user.id, Some(&user.0.session_id)).await?;
    Ok(no_content())
}
