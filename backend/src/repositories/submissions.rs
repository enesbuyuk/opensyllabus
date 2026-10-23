//! Submission queries for the review workspace.

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::{
    errors::AppResult,
    models::submission::{FileMeta, SubmissionListItem},
};

#[derive(Default)]
pub struct SubmissionFilter {
    pub course_id: Option<String>,
    pub week_id: Option<String>,
    pub assignment_id: Option<String>,
    pub student_id: Option<String>,
    pub status: Option<String>,
    /// Include drafts (never shown in the review queue by default).
    pub include_drafts: bool,
}

pub async fn list(pool: &SqlitePool, f: &SubmissionFilter) -> AppResult<Vec<SubmissionListItem>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT s.id, s.assignment_id, a.title AS assignment_title, a.course_id, c.title AS course_title, a.week_id,
                w.week_number, s.student_id, u.name AS student_name, s.status, s.attempt, s.submitted_at, s.is_late,
                a.due_at, a.max_grade, g.score, g.released_at, s.updated_at
         FROM submissions s JOIN assignments a ON a.id = s.assignment_id JOIN courses c ON c.id = a.course_id
         JOIN users u ON u.id = s.student_id LEFT JOIN weeks w ON w.id = a.week_id
         LEFT JOIN grades g ON g.submission_id = s.id WHERE 1 = 1",
    );
    if !f.include_drafts {
        qb.push(" AND s.status <> 'draft'");
    }
    for (col, val) in
        [("a.course_id", &f.course_id), ("a.week_id", &f.week_id), ("s.assignment_id", &f.assignment_id), ("s.student_id", &f.student_id)]
    {
        if let Some(v) = val {
            qb.push(format!(" AND {col} = ")).push_bind(v.clone());
        }
    }
    match f.status.as_deref() {
        Some("late") => {
            qb.push(" AND s.is_late = 1");
        }
        Some("pending") => {
            qb.push(" AND s.status IN ('submitted', 'under_review')");
        }
        Some(st) => {
            qb.push(" AND s.status = ").push_bind(st.to_string());
        }
        None => {}
    }
    qb.push(
        " ORDER BY CASE s.status WHEN 'submitted' THEN 0 WHEN 'under_review' THEN 1 WHEN 'revision_requested' THEN 2 ELSE 3 END,
          s.submitted_at DESC",
    );
    Ok(qb.build_query_as().fetch_all(pool).await?)
}

pub async fn attachments(pool: &SqlitePool, submission_id: &str) -> AppResult<Vec<FileMeta>> {
    Ok(sqlx::query_as(
        "SELECT f.id, f.original_name, f.content_type, f.size_bytes, f.created_at
         FROM submission_attachments sa JOIN files f ON f.id = sa.file_id
         WHERE sa.submission_id = ? ORDER BY sa.created_at",
    )
    .bind(submission_id)
    .fetch_all(pool)
    .await?)
}
