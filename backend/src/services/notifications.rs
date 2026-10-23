//! In-app notifications. A `dedupe_key` makes background reminders idempotent.

use sqlx::SqlitePool;

use crate::{
    errors::AppResult,
    util::{new_id, now},
};

pub struct NewNotification<'a> {
    pub kind: &'a str,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    /// Combined with the user id to form a unique key; duplicates are ignored.
    pub dedupe: Option<String>,
}

pub async fn notify_users(pool: &SqlitePool, user_ids: &[String], n: &NewNotification<'_>) -> AppResult<()> {
    let ts = now();
    for uid in user_ids {
        let key = n.dedupe.as_ref().map(|d| format!("{d}:{uid}"));
        sqlx::query(
            "INSERT OR IGNORE INTO notifications (id, user_id, kind, title, body, link, dedupe_key, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(uid)
        .bind(n.kind)
        .bind(&n.title)
        .bind(&n.body)
        .bind(&n.link)
        .bind(key)
        .bind(&ts)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Active students enrolled in a course.
pub async fn course_student_ids(pool: &SqlitePool, course_id: &str) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN course_enrollments e ON e.user_id = u.id
         WHERE e.course_id = ? AND u.role = 'student' AND u.is_active = 1",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await?)
}

pub async fn notify_course(pool: &SqlitePool, course_id: &str, n: &NewNotification<'_>) -> AppResult<()> {
    // Only notify for courses students can actually see.
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM courses WHERE id = ?").bind(course_id).fetch_optional(pool).await?;
    if status.as_deref() != Some("published") {
        return Ok(());
    }
    let ids = course_student_ids(pool, course_id).await?;
    notify_users(pool, &ids, n).await
}
