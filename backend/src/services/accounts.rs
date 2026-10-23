//! User account creation, password resets and administrator bootstrap.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::{
    auth::password,
    errors::{AppError, AppResult},
    models::user::{Role, User},
    util::{fmt_ts, is_valid_email, new_id, now, random_token, sha256_hex},
};

pub async fn create_user(
    pool: &SqlitePool,
    email: &str,
    name: &str,
    role: Role,
    password_plain: Option<String>,
    timezone: &str,
) -> AppResult<User> {
    let email = email.trim().to_lowercase();
    let name = name.trim().to_string();
    if !is_valid_email(&email) {
        return Err(AppError::field("email", "Enter a valid email address"));
    }
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::field("name", "Name is required (max 120 characters)"));
    }
    let (hash, must_change) = match password_plain {
        Some(p) => {
            password::validate_strength(&p)
                .map_err(|_| AppError::field("password", "Password must be at least 10 characters and include a number or symbol"))?;
            (password::hash(p).await?, false)
        }
        // Unusable random password; the user sets one through a one-time setup link.
        None => (password::hash(random_token(32)).await?, true),
    };
    let ts = now();
    let id = new_id();
    sqlx::query(
        "INSERT INTO users (id, email, name, role, password_hash, timezone, must_change_password, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&email)
    .bind(&name)
    .bind(role)
    .bind(&hash)
    .bind(timezone)
    .bind(must_change)
    .bind(&ts)
    .bind(&ts)
    .execute(pool)
    .await
    .map_err(|e| match AppError::from(e) {
        AppError::Conflict(_) => AppError::field("email", "An account with this email already exists"),
        other => other,
    })?;
    Ok(sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&id).fetch_one(pool).await?)
}

/// Creates a one-time password reset token. Returns (raw token, expiry).
pub async fn create_reset_token(pool: &SqlitePool, user_id: &str, created_by: Option<&str>, hours: i64) -> AppResult<(String, String)> {
    // Invalidate earlier unused tokens for this user.
    sqlx::query("UPDATE password_reset_tokens SET used_at = ? WHERE user_id = ? AND used_at IS NULL")
        .bind(now())
        .bind(user_id)
        .execute(pool)
        .await?;
    let token = random_token(32);
    let expires = fmt_ts(Utc::now() + Duration::hours(hours));
    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token_hash, created_by, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(sha256_hex(token.as_bytes()))
    .bind(created_by)
    .bind(now())
    .bind(&expires)
    .execute(pool)
    .await?;
    Ok((token, expires))
}

/// Consumes a reset token and sets a new password. Revokes all sessions of the user.
pub async fn reset_password_with_token(pool: &SqlitePool, token: &str, new_password: String) -> AppResult<User> {
    password::validate_strength(&new_password)?;
    let ts = now();
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, user_id FROM password_reset_tokens WHERE token_hash = ? AND used_at IS NULL AND expires_at > ?")
            .bind(sha256_hex(token.as_bytes()))
            .bind(&ts)
            .fetch_optional(pool)
            .await?;
    let (token_id, user_id) = row.ok_or_else(|| AppError::bad_request("This reset link is invalid or has expired"))?;
    let hash = password::hash(new_password).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE password_reset_tokens SET used_at = ? WHERE id = ?").bind(&ts).bind(&token_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0, updated_at = ? WHERE id = ?")
        .bind(&hash)
        .bind(&ts)
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL")
        .bind(&ts)
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&user_id).fetch_one(pool).await?)
}

pub async fn set_password(pool: &SqlitePool, user_id: &str, new_password: String) -> AppResult<()> {
    password::validate_strength(&new_password)?;
    let hash = password::hash(new_password).await?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0, updated_at = ? WHERE id = ?")
        .bind(hash)
        .bind(now())
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn admin_count(pool: &SqlitePool) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'").fetch_one(pool).await?)
}

/// Assigns the first administrator as instructor of courses that have none.
pub async fn adopt_orphan_courses(pool: &SqlitePool) -> AppResult<()> {
    sqlx::query(
        "UPDATE courses SET instructor_id = (SELECT id FROM users WHERE role = 'admin' ORDER BY created_at LIMIT 1)
         WHERE instructor_id IS NULL",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Environment-based bootstrap: creates the first admin only when no admin exists yet.
pub async fn bootstrap_admin_from_env(pool: &SqlitePool, timezone: &str) -> anyhow::Result<()> {
    let (Ok(email), Ok(pw)) = (std::env::var("BOOTSTRAP_ADMIN_EMAIL"), std::env::var("BOOTSTRAP_ADMIN_PASSWORD")) else {
        return Ok(());
    };
    if email.trim().is_empty() || pw.is_empty() {
        return Ok(());
    }
    if admin_count(pool).await.map_err(|e| anyhow::anyhow!("{e}"))? > 0 {
        tracing::info!("administrator already exists; ignoring BOOTSTRAP_ADMIN_* variables (you can remove them)");
        return Ok(());
    }
    let name = std::env::var("BOOTSTRAP_ADMIN_NAME").unwrap_or_else(|_| "Instructor".into());
    let user =
        create_user(pool, &email, &name, Role::Admin, Some(pw), timezone).await.map_err(|e| anyhow::anyhow!("bootstrap admin: {e:?}"))?;
    adopt_orphan_courses(pool).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    tracing::warn!(email = %user.email, "bootstrap administrator created; remove BOOTSTRAP_ADMIN_PASSWORD from the environment");
    Ok(())
}
