//! Resource library queries.

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::{errors::AppResult, models::misc::Resource, models::user::User};

#[derive(Default)]
pub struct ResourceFilter {
    pub course_id: Option<String>,
    pub module_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
    /// `newest` (default), `oldest`, `title`, `week`.
    pub sort: Option<String>,
    pub id: Option<String>,
}

const SELECT: &str = "SELECT r.id, r.course_id, r.module_id, r.week_id, r.lesson_id, r.title, r.description, r.kind, r.file_id, r.url,
    r.content_md, r.visibility, r.created_at, r.updated_at, c.title AS course_title, w.week_number,
    m.title AS module_title, l.title AS lesson_title, f.original_name AS file_name, f.size_bytes AS file_size, f.content_type AS file_type
    FROM resources r JOIN courses c ON c.id = r.course_id
    LEFT JOIN weeks w ON w.id = r.week_id LEFT JOIN modules m ON m.id = COALESCE(r.module_id, w.module_id)
    LEFT JOIN lessons l ON l.id = r.lesson_id LEFT JOIN files f ON f.id = r.file_id";

/// Lists resources visible to the user. Students see only `students`-visibility resources of
/// enrolled, visible courses, and only when the attached week/lesson (if any) is open/published.
pub async fn list(pool: &SqlitePool, user: &User, f: &ResourceFilter, today: &str) -> AppResult<Vec<Resource>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(SELECT);
    qb.push(" WHERE 1 = 1");
    if !user.is_admin() {
        qb.push(" AND r.visibility = 'students' AND c.status IN ('published', 'archived') AND EXISTS (SELECT 1 FROM course_enrollments e WHERE e.course_id = r.course_id AND e.user_id = ");
        qb.push_bind(&user.id);
        qb.push(") AND (w.id IS NULL OR (w.status = 'published' AND (w.available_from IS NULL OR w.available_from <= ");
        qb.push_bind(today);
        qb.push("))) AND (l.id IS NULL OR l.status = 'published')");
    }
    for (col, val) in [
        ("r.course_id", &f.course_id),
        ("COALESCE(r.module_id, w.module_id)", &f.module_id),
        ("r.week_id", &f.week_id),
        ("r.lesson_id", &f.lesson_id),
        ("r.kind", &f.kind),
        ("r.id", &f.id),
    ] {
        if let Some(v) = val {
            qb.push(format!(" AND {col} = ")).push_bind(v.clone());
        }
    }
    if let Some(q) = f.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let like = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        qb.push(" AND (r.title LIKE ").push_bind(like.clone()).push(" ESCAPE '\\' OR r.description LIKE ").push_bind(like.clone());
        qb.push(" ESCAPE '\\' OR f.original_name LIKE ").push_bind(like).push(" ESCAPE '\\')");
    }
    qb.push(match f.sort.as_deref() {
        Some("oldest") => " ORDER BY r.created_at ASC",
        Some("title") => " ORDER BY r.title COLLATE NOCASE ASC",
        Some("week") => " ORDER BY w.week_number IS NULL, w.week_number, r.title COLLATE NOCASE",
        _ => " ORDER BY r.created_at DESC",
    });
    Ok(qb.build_query_as().fetch_all(pool).await?)
}
