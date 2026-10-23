//! Platform settings persisted in the `settings` key/value table.

use sqlx::SqlitePool;

use crate::{config::Config, models::misc::PlatformSettings, util::now};

pub async fn load(pool: &SqlitePool, cfg: &Config) -> anyhow::Result<PlatformSettings> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings").fetch_all(pool).await?;
    let get = |k: &str| rows.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone());
    Ok(PlatformSettings {
        platform_name: get("platform_name").unwrap_or_else(|| "OpenSyllabus".into()),
        default_timezone: get("default_timezone").unwrap_or_else(|| cfg.default_timezone.clone()),
        support_email: get("support_email").unwrap_or_default(),
    })
}

pub async fn save(pool: &SqlitePool, s: &PlatformSettings) -> anyhow::Result<()> {
    let ts = now();
    let mut tx = pool.begin().await?;
    for (k, v) in [("platform_name", &s.platform_name), ("default_timezone", &s.default_timezone), ("support_email", &s.support_email)] {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(k)
        .bind(v)
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_flag(pool: &SqlitePool, key: &str) -> anyhow::Result<bool> {
    let v: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?").bind(key).fetch_optional(pool).await?;
    Ok(v.as_deref() == Some("1"))
}

pub async fn set_flag(pool: &SqlitePool, key: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, '1', ?)").bind(key).bind(now()).execute(pool).await?;
    Ok(())
}
