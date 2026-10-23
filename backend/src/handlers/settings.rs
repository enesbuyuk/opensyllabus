use axum::{
    extract::{Query, State},
    response::Response,
    Extension,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{json, ApiJson};
use crate::{
    auth::AdminUser,
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::misc::{AuditLog, PlatformSettings, PlatformSettingsInput},
    services::{audit, settings, storage},
    util::{is_valid_email, is_valid_timezone, Validator},
    AppState,
};

#[derive(Serialize, ToSchema)]
pub struct PublicSettings {
    #[serde(flatten)]
    pub settings: PlatformSettings,
    pub supported_file_types: Vec<&'static str>,
    pub max_upload_mb: u64,
}

#[utoipa::path(get, path = "/api/v1/settings", tag = "settings", responses((status = 200, body = PublicSettings)))]
pub async fn get(State(state): State<AppState>) -> AppResult<Response> {
    let s = state.settings.read().map_err(|_| AppError::Internal(anyhow::anyhow!("settings lock poisoned")))?.clone();
    Ok(json(PublicSettings {
        settings: s,
        supported_file_types: storage::supported_extensions(),
        max_upload_mb: state.config.max_upload_mb,
    }))
}

#[utoipa::path(patch, path = "/api/v1/settings", tag = "settings", request_body = PlatformSettingsInput, responses((status = 200, body = PlatformSettings)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<PlatformSettingsInput>,
) -> AppResult<Response> {
    let current = state.settings.read().map_err(|_| AppError::Internal(anyhow::anyhow!("settings lock poisoned")))?.clone();
    let next = PlatformSettings {
        platform_name: req.platform_name.map(|s| s.trim().to_string()).unwrap_or(current.platform_name),
        default_timezone: req.default_timezone.unwrap_or(current.default_timezone),
        support_email: req.support_email.map(|s| s.trim().to_string()).unwrap_or(current.support_email),
    };
    Validator::new()
        .len(&next.platform_name, "platform_name", 1, 80)
        .check(is_valid_timezone(&next.default_timezone), "default_timezone", "Unknown timezone")
        .check(next.support_email.is_empty() || is_valid_email(&next.support_email), "support_email", "Enter a valid email address")
        .finish()?;
    settings::save(&state.pool, &next).await.map_err(AppError::Internal)?;
    if let Ok(mut s) = state.settings.write() {
        *s = next.clone();
    }
    audit::log(
        &state.pool,
        Some(&admin.id),
        "settings.update",
        "settings",
        None,
        serde_json::to_value(&next).unwrap_or_default(),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(next))
}

#[derive(Deserialize, IntoParams)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub action: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/audit-logs", tag = "settings", params(AuditQuery), responses((status = 200, body = [AuditLog])))]
pub async fn audit_logs(State(state): State<AppState>, _admin: AdminUser, Query(q): Query<AuditQuery>) -> AppResult<Response> {
    let rows: Vec<AuditLog> = sqlx::query_as(
        "SELECT a.id, a.actor_id, u.name AS actor_name, a.action, a.entity_type, a.entity_id, a.details, a.ip_address, a.created_at
         FROM audit_logs a LEFT JOIN users u ON u.id = a.actor_id
         WHERE (? IS NULL OR a.action LIKE ? || '%') ORDER BY a.created_at DESC LIMIT ?",
    )
    .bind(&q.action)
    .bind(&q.action)
    .bind(q.limit.unwrap_or(100).clamp(1, 500))
    .fetch_all(&state.pool)
    .await?;
    Ok(json(rows))
}
