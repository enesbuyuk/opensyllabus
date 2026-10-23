//! Audit trail for administrative and security-relevant operations.

use serde_json::Value;
use sqlx::SqlitePool;

use crate::util::{new_id, now};

/// Records an audit entry. Failures are logged but never fail the request.
pub async fn log(
    pool: &SqlitePool,
    actor_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Value,
    ip: Option<&str>,
) {
    let res = sqlx::query(
        "INSERT INTO audit_logs (id, actor_id, action, entity_type, entity_id, details, ip_address, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(actor_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details.to_string())
    .bind(ip)
    .bind(now())
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, action, "failed to write audit log");
    }
}
