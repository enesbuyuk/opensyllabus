//! Server-side sessions. The cookie carries a random token; only its SHA-256 hash is stored.

use chrono::{Duration, Utc};
use cookie::{Cookie, SameSite};
use sqlx::SqlitePool;

use crate::{
    config::Config,
    errors::AppResult,
    models::user::User,
    util::{fmt_ts, new_id, now, random_token, sha256_hex},
};

pub const SESSION_COOKIE: &str = "opensyllabus_session";
pub const CSRF_HEADER: &str = "x-csrf-token";

pub struct NewSession {
    pub token: String,
    pub csrf_token: String,
}

pub async fn create(pool: &SqlitePool, cfg: &Config, user_id: &str, ip: Option<&str>, user_agent: Option<&str>) -> AppResult<NewSession> {
    let token = random_token(32);
    let csrf_token = random_token(32);
    let ts = now();
    let expires = fmt_ts(Utc::now() + Duration::hours(cfg.session_ttl_hours));
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, csrf_token, ip_address, user_agent, created_at, last_seen_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(sha256_hex(token.as_bytes()))
    .bind(&csrf_token)
    .bind(ip)
    .bind(user_agent.map(|s| s.chars().take(255).collect::<String>()))
    .bind(&ts)
    .bind(&ts)
    .bind(&expires)
    .execute(pool)
    .await?;
    Ok(NewSession { token, csrf_token })
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: String,
    csrf_token: String,
    last_seen_at: String,
}

pub struct ResolvedSession {
    pub session_id: String,
    pub csrf_token: String,
    pub user: User,
}

/// Looks up a live session by raw token and slides its expiry forward.
pub async fn resolve(pool: &SqlitePool, cfg: &Config, token: &str) -> AppResult<Option<ResolvedSession>> {
    let ts = now();
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT s.id AS session_id, s.csrf_token, s.last_seen_at
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ? AND u.is_active = 1",
    )
    .bind(sha256_hex(token.as_bytes()))
    .bind(&ts)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };

    let user: User = sqlx::query_as("SELECT u.* FROM users u JOIN sessions s ON s.user_id = u.id WHERE s.id = ?")
        .bind(&row.session_id)
        .fetch_one(pool)
        .await?;

    // Touch at most every five minutes to limit write amplification.
    let stale = fmt_ts(Utc::now() - Duration::minutes(5));
    if row.last_seen_at < stale {
        let expires = fmt_ts(Utc::now() + Duration::hours(cfg.session_ttl_hours));
        sqlx::query("UPDATE sessions SET last_seen_at = ?, expires_at = ? WHERE id = ?")
            .bind(&ts)
            .bind(&expires)
            .bind(&row.session_id)
            .execute(pool)
            .await?;
    }

    Ok(Some(ResolvedSession { session_id: row.session_id, csrf_token: row.csrf_token, user }))
}

pub async fn revoke(pool: &SqlitePool, session_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
        .bind(now())
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revokes all sessions of a user, optionally keeping one.
pub async fn revoke_all_for_user(pool: &SqlitePool, user_id: &str, except: Option<&str>) -> AppResult<u64> {
    let res = sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL AND id IS NOT ?")
        .bind(now())
        .bind(user_id)
        .bind(except)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn purge_expired(pool: &SqlitePool) -> AppResult<u64> {
    let cutoff = fmt_ts(Utc::now() - Duration::days(30));
    let res = sqlx::query("DELETE FROM sessions WHERE expires_at < ? OR (revoked_at IS NOT NULL AND revoked_at < ?)")
        .bind(now())
        .bind(&cutoff)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM password_reset_tokens WHERE expires_at < ?").bind(&cutoff).execute(pool).await?;
    Ok(res.rows_affected())
}

pub fn session_cookie(cfg: &Config, token: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(cfg.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::hours(cfg.session_ttl_hours))
        .build()
}

pub fn clear_cookie(cfg: &Config) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .secure(cfg.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::ZERO)
        .build()
}

/// Extracts the session token from a `Cookie` header value.
pub fn token_from_cookie_header(header: &str) -> Option<String> {
    Cookie::split_parse(header)
        .filter_map(Result::ok)
        .find(|c| c.name() == SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .filter(|v| !v.is_empty())
}
