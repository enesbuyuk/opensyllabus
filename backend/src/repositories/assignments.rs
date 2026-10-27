//! Assignment list queries, shaped for students (own state) or instructors (counters).

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::{
    errors::AppResult,
    models::{
        assignment::{Assignment, AssignmentListItem, RubricCriterion},
        user::User,
        Json,
    },
    services::progress::student_state,
};

#[derive(Default)]
pub struct AssignmentFilter {
    pub course_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub status: Option<String>,
    pub ids: Option<Vec<String>>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    course_id: String,
    week_id: Option<String>,
    lesson_id: Option<String>,
    title: String,
    description: String,
    instructions_md: String,
    reference_md: String,
    kind: String,
    max_grade: f64,
    due_at: Option<String>,
    publish_at: Option<String>,
    late_policy: String,
    late_penalty_percent: i64,
    allowed_file_types: Json<Vec<String>>,
    max_upload_mb: i64,
    allow_resubmission: bool,
    status: String,
    is_group: bool,
    created_at: String,
    updated_at: String,
    course_title: String,
    week_number: Option<i64>,
    week_title: Option<String>,
    lesson_title: Option<String>,
    sub_status: Option<String>,
    sub_is_late: Option<bool>,
    sub_submitted_at: Option<String>,
    released_score: Option<f64>,
    submission_count: Option<i64>,
    pending_review_count: Option<i64>,
}

pub async fn list(pool: &SqlitePool, user: &User, filter: &AssignmentFilter, now: &str, today: &str) -> AppResult<Vec<AssignmentListItem>> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT a.*, c.title AS course_title, w.week_number, w.title AS week_title, l.title AS lesson_title, ");
    if user.is_admin() {
        qb.push(
            "NULL AS sub_status, NULL AS sub_is_late, NULL AS sub_submitted_at, NULL AS released_score,
             (SELECT COUNT(*) FROM submissions s WHERE s.assignment_id = a.id AND s.mirror_of IS NULL AND s.status <> 'draft') AS submission_count,
             (SELECT COUNT(*) FROM submissions s WHERE s.assignment_id = a.id AND s.mirror_of IS NULL AND s.status IN ('submitted', 'under_review')) AS pending_review_count
             FROM assignments a JOIN courses c ON c.id = a.course_id
             LEFT JOIN weeks w ON w.id = a.week_id LEFT JOIN lessons l ON l.id = a.lesson_id WHERE 1 = 1",
        );
    } else {
        qb.push(
            "s.status AS sub_status, s.is_late AS sub_is_late, s.submitted_at AS sub_submitted_at,
             CASE WHEN g.released_at IS NOT NULL THEN g.score END AS released_score,
             NULL AS submission_count, NULL AS pending_review_count
             FROM assignments a JOIN courses c ON c.id = a.course_id
             JOIN course_enrollments e ON e.course_id = a.course_id AND e.user_id = ",
        );
        qb.push_bind(&user.id);
        qb.push(" LEFT JOIN weeks w ON w.id = a.week_id LEFT JOIN lessons l ON l.id = a.lesson_id LEFT JOIN submissions s ON s.assignment_id = a.id AND s.student_id = ");
        qb.push_bind(&user.id);
        qb.push(" LEFT JOIN grades g ON g.submission_id = s.id WHERE c.status IN ('published', 'archived') AND a.status = 'published' AND (a.publish_at IS NULL OR a.publish_at <= ");
        qb.push_bind(now);
        // Assignments attached to a week only show once the week is open.
        qb.push(") AND (w.id IS NULL OR (w.status = 'published' AND (w.available_from IS NULL OR w.available_from <= ");
        qb.push_bind(today);
        qb.push(")))");
    }
    if let Some(v) = &filter.course_id {
        qb.push(" AND a.course_id = ").push_bind(v);
    }
    if let Some(v) = &filter.week_id {
        qb.push(" AND a.week_id = ").push_bind(v);
    }
    if let Some(v) = &filter.lesson_id {
        qb.push(" AND a.lesson_id = ").push_bind(v);
    }
    if let (Some(v), true) = (&filter.status, user.is_admin()) {
        qb.push(" AND a.status = ").push_bind(v);
    }
    if let Some(ids) = &filter.ids {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        qb.push(" AND a.id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(id);
        }
        qb.push(")");
    }
    qb.push(" ORDER BY a.due_at IS NULL, a.due_at, w.week_number, a.created_at");

    let rows: Vec<Row> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| into_item(r, user.is_admin())).collect())
}

fn into_item(r: Row, admin: bool) -> AssignmentListItem {
    let state = (!admin).then(|| student_state(r.sub_status.as_deref(), r.sub_is_late.unwrap_or(false)));
    AssignmentListItem {
        assignment: Assignment {
            id: r.id,
            course_id: r.course_id,
            week_id: r.week_id,
            lesson_id: r.lesson_id,
            title: r.title,
            description: r.description,
            instructions_md: r.instructions_md,
            reference_md: r.reference_md,
            kind: r.kind,
            max_grade: r.max_grade,
            due_at: r.due_at,
            publish_at: r.publish_at,
            late_policy: r.late_policy,
            late_penalty_percent: r.late_penalty_percent,
            allowed_file_types: r.allowed_file_types,
            max_upload_mb: r.max_upload_mb,
            allow_resubmission: r.allow_resubmission,
            status: r.status,
            is_group: r.is_group,
            created_at: r.created_at,
            updated_at: r.updated_at,
        },
        course_title: r.course_title,
        week_number: r.week_number,
        week_title: r.week_title,
        lesson_title: r.lesson_title,
        state,
        score: r.released_score,
        submitted_at: r.sub_submitted_at,
        submission_count: r.submission_count,
        pending_review_count: r.pending_review_count,
    }
}

pub async fn rubric(pool: &SqlitePool, assignment_id: &str) -> AppResult<Vec<RubricCriterion>> {
    Ok(sqlx::query_as(
        "SELECT id, position, title, description, max_points FROM assignment_rubrics WHERE assignment_id = ? ORDER BY position",
    )
    .bind(assignment_id)
    .fetch_all(pool)
    .await?)
}
