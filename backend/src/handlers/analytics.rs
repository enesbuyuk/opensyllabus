//! Course analytics for the instructor. All values derive from stored records.

use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::json;
use crate::{
    auth::{access, AdminUser},
    errors::AppResult,
    services::progress::percent_f,
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct AnalyticsQuery {
    pub course_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct StudentStat {
    pub student_id: String,
    pub name: String,
    pub completion_percent: f64,
    pub lesson_percent: f64,
    pub assignment_percent: f64,
    pub average_grade_percent: Option<f64>,
    pub late_count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct AssignmentStat {
    pub assignment_id: String,
    pub title: String,
    pub week_number: Option<i64>,
    pub enrolled: i64,
    pub submitted: i64,
    pub graded: i64,
    pub late: i64,
    pub average_percent: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct WeekStat {
    pub week_number: i64,
    pub title: String,
    pub average_completion: f64,
}

#[derive(Serialize, ToSchema)]
pub struct Analytics {
    pub course_id: Option<String>,
    pub course_title: Option<String>,
    pub students: Vec<StudentStat>,
    pub assignments: Vec<AssignmentStat>,
    pub submission_status: Vec<StatusCount>,
    pub weeks: Vec<WeekStat>,
}

#[derive(sqlx::FromRow)]
struct AssignmentRow {
    id: String,
    title: String,
    week_number: Option<i64>,
    max_grade: f64,
    submitted: i64,
    graded: i64,
    late: i64,
    avg_score: Option<f64>,
}

#[utoipa::path(get, path = "/api/v1/analytics", tag = "analytics", params(AnalyticsQuery), responses((status = 200, body = Analytics)))]
pub async fn analytics(State(state): State<AppState>, admin: AdminUser, Query(q): Query<AnalyticsQuery>) -> AppResult<Response> {
    let course_id = match q.course_id {
        Some(c) => Some(access::course_for(&state.pool, &admin, &c).await?.id),
        None => {
            sqlx::query_scalar("SELECT id FROM courses ORDER BY status = 'published' DESC, created_at LIMIT 1")
                .fetch_optional(&state.pool)
                .await?
        }
    };
    let Some(course_id) = course_id else {
        return Ok(json(Analytics {
            course_id: None,
            course_title: None,
            students: vec![],
            assignments: vec![],
            submission_status: vec![],
            weeks: vec![],
        }));
    };
    let (data, rows) = super::progress::course_rows(&state, &course_id).await?;
    let enrolled = rows.len() as i64;

    let students = rows
        .iter()
        .map(|r| StudentStat {
            student_id: r.student.id.clone(),
            name: r.student.name.clone(),
            completion_percent: r.progress.completion_percent,
            lesson_percent: r.progress.lesson_percent,
            assignment_percent: r.progress.assignment_percent,
            average_grade_percent: r.progress.average_grade_percent,
            late_count: r.progress.assignments_late,
        })
        .collect();

    let arows: Vec<AssignmentRow> = sqlx::query_as(
        "SELECT a.id, a.title, w.week_number, a.max_grade,
                COUNT(CASE WHEN s.attempt > 0 THEN 1 END) AS submitted,
                COUNT(CASE WHEN s.status = 'graded' THEN 1 END) AS graded,
                COUNT(CASE WHEN s.is_late = 1 THEN 1 END) AS late,
                AVG(CASE WHEN g.released_at IS NOT NULL THEN g.score END) AS avg_score
         FROM assignments a LEFT JOIN weeks w ON w.id = a.week_id
         LEFT JOIN submissions s ON s.assignment_id = a.id LEFT JOIN grades g ON g.submission_id = s.id
         WHERE a.course_id = ? AND a.status = 'published'
         GROUP BY a.id ORDER BY w.week_number IS NULL, w.week_number, a.due_at",
    )
    .bind(&course_id)
    .fetch_all(&state.pool)
    .await?;
    let assignments = arows
        .into_iter()
        .map(|r| AssignmentStat {
            assignment_id: r.id,
            title: r.title,
            week_number: r.week_number,
            enrolled,
            submitted: r.submitted,
            graded: r.graded,
            late: r.late,
            average_percent: r.avg_score.map(|s| percent_f(s, r.max_grade)),
        })
        .collect();

    let submission_status: Vec<StatusCount> = sqlx::query_as::<_, (String, i64)>(
        "SELECT s.status, COUNT(*) FROM submissions s JOIN assignments a ON a.id = s.assignment_id WHERE a.course_id = ? GROUP BY s.status",
    )
    .bind(&course_id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|(status, count)| StatusCount { status, count })
    .collect();

    let weeks = data
        .weeks
        .iter()
        .filter(|w| w.status == "published")
        .map(|w| {
            let vals: Vec<f64> =
                rows.iter().filter_map(|r| r.progress.weeks.iter().find(|p| p.week_id == w.id)).map(|p| p.completion_percent).collect();
            let avg = if vals.is_empty() { 0.0 } else { (vals.iter().sum::<f64>() / vals.len() as f64 * 10.0).round() / 10.0 };
            WeekStat { week_number: w.week_number, title: w.title.clone(), average_completion: avg }
        })
        .collect();

    Ok(json(Analytics {
        course_id: Some(data.course.id.clone()),
        course_title: Some(data.course.title.clone()),
        students,
        assignments,
        submission_status,
        weeks,
    }))
}
