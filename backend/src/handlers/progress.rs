use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::json;
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    models::user::UserSummary,
    services::progress::{self, CourseData, CourseProgress},
    util::now,
    AppState,
};

#[utoipa::path(get, path = "/api/v1/me/progress", tag = "progress", responses((status = 200, body = [CourseProgress])))]
pub async fn my_progress(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    if user.is_admin() {
        return Ok(json(Vec::<CourseProgress>::new()));
    }
    let mut out = Vec::new();
    for cid in access::visible_course_ids(&state.pool, &user).await? {
        let data = progress::load_course_data(&state.pool, &cid).await?;
        out.push(data.student_progress(&state.pool, &user.id, &state.today(), &now()).await?);
    }
    Ok(json(out))
}

#[derive(Serialize, ToSchema)]
pub struct StudentProgressRow {
    pub student: UserSummary,
    pub last_login_at: Option<String>,
    pub progress: CourseProgress,
}

pub async fn course_rows(state: &AppState, course_id: &str) -> AppResult<(CourseData, Vec<StudentProgressRow>)> {
    let data = progress::load_course_data(&state.pool, course_id).await?;
    let students: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT u.id, u.name, u.email, u.last_login_at FROM users u JOIN course_enrollments e ON e.user_id = u.id
         WHERE e.course_id = ? AND u.role = 'student' ORDER BY u.name COLLATE NOCASE",
    )
    .bind(course_id)
    .fetch_all(&state.pool)
    .await?;
    let (today, ts) = (state.today(), now());
    let mut rows = Vec::new();
    for (id, name, email, last_login_at) in students {
        let p = data.student_progress(&state.pool, &id, &today, &ts).await?;
        rows.push(StudentProgressRow { student: UserSummary { id, name, email }, last_login_at, progress: p });
    }
    Ok((data, rows))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/progress", tag = "progress", params(("id" = String, Path)),
    responses((status = 200, body = [StudentProgressRow], description = "Instructor: all enrolled students. Student: only their own row.")))]
pub async fn course_progress(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &user, &id).await?;
    if user.is_admin() {
        let (_, rows) = course_rows(&state, &course.id).await?;
        return Ok(json(rows));
    }
    let data = CourseData::load(&state.pool, course).await?;
    let p = data.student_progress(&state.pool, &user.id, &state.today(), &now()).await?;
    Ok(json(vec![StudentProgressRow {
        student: UserSummary { id: user.id.clone(), name: user.name.clone(), email: user.email.clone() },
        last_login_at: user.last_login_at.clone(),
        progress: p,
    }]))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/students/{student_id}/progress", tag = "progress",
    params(("id" = String, Path), ("student_id" = String, Path)), responses((status = 200, body = CourseProgress)))]
pub async fn student_progress(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, student_id)): Path<(String, String)>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    if !access::is_enrolled(&state.pool, &course.id, &student_id).await? {
        return Err(AppError::NotFound("Student"));
    }
    let data = CourseData::load(&state.pool, course).await?;
    Ok(json(data.student_progress(&state.pool, &student_id, &state.today(), &now()).await?))
}

// ---------------------------------------------------------------------------
// Gradebook
// ---------------------------------------------------------------------------

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct GradebookColumn {
    pub id: String,
    pub title: String,
    pub week_number: Option<i64>,
    pub max_grade: f64,
    pub due_at: Option<String>,
    pub status: String,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct GradebookCell {
    pub submission_id: Option<String>,
    /// `not_started`, `draft`, `submitted`, `late`, `under_review`, `graded`, `revision_requested`, `missing`.
    pub state: String,
    pub score: Option<f64>,
    pub released: bool,
    pub is_late: bool,
    pub attempt: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GradebookRow {
    pub student: UserSummary,
    pub completion_percent: f64,
    pub lesson_percent: f64,
    pub average_grade_percent: Option<f64>,
    pub cells: HashMap<String, GradebookCell>,
    /// Week number → completion percent.
    pub weeks: HashMap<i64, f64>,
}

#[derive(Serialize, ToSchema)]
pub struct Gradebook {
    pub course_id: String,
    pub course_title: String,
    pub columns: Vec<GradebookColumn>,
    pub week_numbers: Vec<i64>,
    pub rows: Vec<GradebookRow>,
}

#[derive(Deserialize, IntoParams)]
pub struct GradebookQuery {
    pub week_id: Option<String>,
    pub student_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CellRow {
    id: String,
    assignment_id: String,
    student_id: String,
    status: String,
    is_late: bool,
    attempt: i64,
    score: Option<f64>,
    released_at: Option<String>,
}

async fn build_gradebook(state: &AppState, course_id: &str, q: &GradebookQuery) -> AppResult<Gradebook> {
    let (data, rows) = course_rows(state, course_id).await?;
    let columns: Vec<GradebookColumn> = sqlx::query_as(
        "SELECT a.id, a.title, w.week_number, a.max_grade, a.due_at, a.status FROM assignments a LEFT JOIN weeks w ON w.id = a.week_id
         WHERE a.course_id = ? AND a.status <> 'draft' AND (? IS NULL OR a.week_id = ?)
         ORDER BY w.week_number IS NULL, w.week_number, a.due_at, a.created_at",
    )
    .bind(course_id)
    .bind(&q.week_id)
    .bind(&q.week_id)
    .fetch_all(&state.pool)
    .await?;
    let cells: Vec<CellRow> = sqlx::query_as(
        "SELECT s.id, s.assignment_id, s.student_id, s.status, s.is_late, s.attempt, g.score, g.released_at
         FROM submissions s JOIN assignments a ON a.id = s.assignment_id LEFT JOIN grades g ON g.submission_id = s.id WHERE a.course_id = ?",
    )
    .bind(course_id)
    .fetch_all(&state.pool)
    .await?;
    let ts = now();
    let week_numbers: Vec<i64> = data.weeks.iter().filter(|w| w.status == "published").map(|w| w.week_number).collect();
    let rows = rows
        .into_iter()
        .filter(|r| q.student_id.as_deref().is_none_or(|s| s == r.student.id))
        .map(|r| {
            let mut map = HashMap::new();
            for c in &columns {
                let found = cells.iter().find(|x| x.assignment_id == c.id && x.student_id == r.student.id);
                let cell = match found {
                    Some(x) => GradebookCell {
                        submission_id: Some(x.id.clone()),
                        state: progress::student_state(Some(&x.status), x.is_late).to_snake(),
                        score: x.score,
                        released: x.released_at.is_some(),
                        is_late: x.is_late,
                        attempt: x.attempt,
                    },
                    None => GradebookCell {
                        submission_id: None,
                        state: if c.due_at.as_deref().is_some_and(|d| d < ts.as_str()) { "missing".into() } else { "not_started".into() },
                        score: None,
                        released: false,
                        is_late: false,
                        attempt: 0,
                    },
                };
                map.insert(c.id.clone(), cell);
            }
            GradebookRow {
                completion_percent: r.progress.completion_percent,
                lesson_percent: r.progress.lesson_percent,
                average_grade_percent: r.progress.average_grade_percent,
                weeks: r.progress.weeks.iter().map(|w| (w.week_number, w.completion_percent)).collect(),
                student: r.student,
                cells: map,
            }
        })
        .collect();
    Ok(Gradebook { course_id: data.course.id.clone(), course_title: data.course.title.clone(), columns, week_numbers, rows })
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/gradebook", tag = "progress", params(("id" = String, Path), GradebookQuery), responses((status = 200, body = Gradebook)))]
pub async fn gradebook(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    Query(q): Query<GradebookQuery>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    Ok(json(build_gradebook(&state, &course.id, &q).await?))
}

fn csv_field(s: &str) -> String {
    // Quote everything and neutralize spreadsheet formula injection.
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{s}") } else { s.to_string() };
    format!("\"{}\"", s.replace('"', "\"\""))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/gradebook.csv", tag = "progress", params(("id" = String, Path), GradebookQuery), responses((status = 200, content_type = "text/csv")))]
pub async fn gradebook_csv(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    Query(q): Query<GradebookQuery>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let gb = build_gradebook(&state, &course.id, &q).await?;
    let mut out = String::new();
    let mut header_row = vec!["Student".to_string(), "Email".into(), "Completion %".into(), "Lessons %".into(), "Average grade %".into()];
    for c in &gb.columns {
        let label = match c.week_number {
            Some(w) => format!("W{w} {} (/{})", c.title, c.max_grade),
            None => format!("{} (/{})", c.title, c.max_grade),
        };
        header_row.push(label);
        header_row.push(format!("{} status", c.title));
    }
    out.push_str(&header_row.iter().map(|h| csv_field(h)).collect::<Vec<_>>().join(","));
    out.push_str("\r\n");
    for r in &gb.rows {
        let mut row = vec![
            r.student.name.clone(),
            r.student.email.clone(),
            format!("{:.1}", r.completion_percent),
            format!("{:.1}", r.lesson_percent),
            r.average_grade_percent.map(|g| format!("{g:.1}")).unwrap_or_default(),
        ];
        for c in &gb.columns {
            let cell = &r.cells[&c.id];
            row.push(cell.score.map(|s| format!("{s}")).unwrap_or_default());
            row.push(cell.state.clone() + if cell.released || cell.score.is_none() { "" } else { " (unreleased)" });
        }
        out.push_str(&row.iter().map(|v| csv_field(v)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    let filename = format!("gradebook-{}-{}.csv", course.slug, &now()[..10]);
    let mut res = out.into_response();
    res.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv; charset=utf-8"));
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")).map_err(|e| AppError::Internal(e.into()))?,
    );
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::csv_field;

    #[test]
    fn csv_escaping() {
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_field("=SUM(A1)"), "\"'=SUM(A1)\"");
    }
}
