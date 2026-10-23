//! Lightweight periodic jobs running on the Tokio runtime.

use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;

use crate::{
    services::notifications::{self, NewNotification},
    util::fmt_ts,
    AppState,
};

pub fn spawn(state: AppState, mut shutdown: watch::Receiver<bool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = run_once(&state).await {
                        tracing::warn!(error = ?e, "background job failed");
                    }
                }
                _ = shutdown.changed() => break,
            }
        }
        tracing::info!("background jobs stopped");
    })
}

pub async fn run_once(state: &AppState) -> anyhow::Result<()> {
    notify_released_assignments(state).await?;
    notify_upcoming_deadlines(state).await?;
    crate::auth::session::purge_expired(&state.pool).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    cleanup_storage(state).await?;
    Ok(())
}

/// Notifies students about assignments whose publication date has arrived.
pub async fn notify_released_assignments(state: &AppState) -> anyhow::Result<()> {
    let now = fmt_ts(Utc::now());
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, course_id, title FROM assignments
         WHERE status = 'published' AND published_notified = 0 AND (publish_at IS NULL OR publish_at <= ?)",
    )
    .bind(&now)
    .fetch_all(&state.pool)
    .await?;
    for (id, course_id, title) in rows {
        notifications::notify_course(
            &state.pool,
            &course_id,
            &NewNotification {
                kind: "assignment_published",
                title: format!("New assignment: {title}"),
                body: "A new assignment is available.".into(),
                link: Some(format!("/assignments/{id}")),
                dedupe: Some(format!("assignment:{id}")),
            },
        )
        .await?;
        sqlx::query("UPDATE assignments SET published_notified = 1 WHERE id = ?").bind(&id).execute(&state.pool).await?;
    }
    Ok(())
}

/// Reminds students 24 hours before a deadline if they have not submitted.
async fn notify_upcoming_deadlines(state: &AppState) -> anyhow::Result<()> {
    let now = fmt_ts(Utc::now());
    let soon = fmt_ts(Utc::now() + chrono::Duration::hours(24));
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT a.id, a.course_id, a.title, a.due_at FROM assignments a JOIN courses c ON c.id = a.course_id
         WHERE a.status = 'published' AND c.status = 'published' AND (a.publish_at IS NULL OR a.publish_at <= ?1)
           AND a.due_at > ?1 AND a.due_at <= ?2",
    )
    .bind(&now)
    .bind(&soon)
    .fetch_all(&state.pool)
    .await?;
    for (id, course_id, title, _due) in rows {
        let students: Vec<String> = sqlx::query_scalar(
            "SELECT u.id FROM users u JOIN course_enrollments e ON e.user_id = u.id
             WHERE e.course_id = ? AND u.role = 'student' AND u.is_active = 1
               AND NOT EXISTS (SELECT 1 FROM submissions s WHERE s.assignment_id = ? AND s.student_id = u.id
                               AND s.status IN ('submitted', 'under_review', 'graded'))",
        )
        .bind(&course_id)
        .bind(&id)
        .fetch_all(&state.pool)
        .await?;
        notifications::notify_users(
            &state.pool,
            &students,
            &NewNotification {
                kind: "deadline_approaching",
                title: format!("Due within 24 hours: {title}"),
                body: "You have not submitted this assignment yet.".into(),
                link: Some(format!("/assignments/{id}")),
                dedupe: Some(format!("deadline:{id}")),
            },
        )
        .await?;
    }
    Ok(())
}

/// Removes abandoned temporary uploads and unreferenced files older than a day.
async fn cleanup_storage(state: &AppState) -> anyhow::Result<()> {
    let tmp = state.config.storage_dir.join("tmp");
    if let Ok(mut dir) = tokio::fs::read_dir(&tmp).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                let old = meta.modified().ok().and_then(|m| m.elapsed().ok()).is_some_and(|e| e > Duration::from_secs(3600));
                if old {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }
    let cutoff = fmt_ts(Utc::now() - chrono::Duration::hours(24));
    let orphans: Vec<String> = sqlx::query_scalar(
        "SELECT f.id FROM files f WHERE f.created_at < ?
           AND NOT EXISTS (SELECT 1 FROM submission_attachments sa WHERE sa.file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM resources r WHERE r.file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM courses c WHERE c.cover_file_id = f.id)",
    )
    .bind(&cutoff)
    .fetch_all(&state.pool)
    .await?;
    for id in orphans {
        crate::services::storage::delete_if_orphaned(state, &id).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}
