use axum::{
    extract::{Path, State},
    response::Response,
    Extension,
};
use chrono::NaiveDate;

use super::{json, ApiJson};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        attendance::{
            AttendanceInput, AttendanceRecord, AttendanceSession, AttendanceSummary, CourseAttendance, MyAttendance, MyAttendanceSession,
            STATUSES,
        },
        course::Course,
        user::UserSummary,
    },
    services::{audit, schedule},
    util::{fmt_ts, now},
    AppState,
};

/// Every class of the course: each scheduled course day in each course week.
async fn sessions(state: &AppState, course: &Course) -> AppResult<Vec<AttendanceSession>> {
    let sched = super::courses::load_schedule(state, &course.id).await?;
    let tz: chrono_tz::Tz = sched.timezone.parse().unwrap_or(chrono_tz::UTC);
    let start = course.start_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let marked: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT week_number, day_number, COUNT(*) FROM attendance WHERE course_id = ? GROUP BY week_number, day_number")
            .bind(&course.id)
            .fetch_all(&state.pool)
            .await?;
    let mut out = Vec::new();
    for week_number in 1..=course.duration_weeks {
        for slot in &sched.slots {
            let date = start.map(|s| schedule::class_date(s, week_number, slot.weekday as u32));
            let times = match (date, schedule::parse_time(&slot.start_time)) {
                (Some(d), Some(t)) => Some(schedule::class_times(d, t, slot.end_time.as_deref().and_then(schedule::parse_time), tz)),
                _ => None,
            };
            out.push(AttendanceSession {
                week_number,
                day_number: slot.day_number,
                date: date.map(|d| d.format("%Y-%m-%d").to_string()),
                starts_at: times.map(|(s, _)| fmt_ts(s)),
                ends_at: times.and_then(|(_, e)| e.map(fmt_ts)),
                location: slot.location.clone(),
                marked: marked.iter().find(|(w, d, _)| *w == week_number && *d == slot.day_number).map_or(0, |m| m.2),
            });
        }
    }
    Ok(out)
}

async fn course_attendance(state: &AppState, course: &Course) -> AppResult<CourseAttendance> {
    let students: Vec<UserSummary> = sqlx::query_as(
        "SELECT u.id, u.name, u.email FROM users u JOIN course_enrollments e ON e.user_id = u.id
         WHERE e.course_id = ? AND u.role = 'student' AND u.is_active = 1 ORDER BY u.name COLLATE NOCASE",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let records: Vec<AttendanceRecord> = sqlx::query_as(
        "SELECT week_number, day_number, student_id, status FROM attendance WHERE course_id = ? ORDER BY week_number, day_number",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let summaries = students.iter().map(|s| AttendanceSummary::from_records(&s.id, records.iter())).collect();
    Ok(CourseAttendance { sessions: sessions(state, course).await?, students, records, summaries })
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/attendance", tag = "attendance", params(("id" = String, Path)), responses((status = 200, body = CourseAttendance)))]
pub async fn get(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    Ok(json(course_attendance(&state, &course).await?))
}

/// Marks one class for any number of students; a missing status clears that student's record.
#[utoipa::path(put, path = "/api/v1/courses/{id}/attendance", tag = "attendance", params(("id" = String, Path)), request_body = AttendanceInput, responses((status = 200, body = CourseAttendance)))]
pub async fn mark(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<AttendanceInput>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let all = sessions(&state, &course).await?;
    if !all.iter().any(|s| s.week_number == req.week_number && s.day_number == req.day_number) {
        return Err(AppError::bad_request("This class is not on the course schedule"));
    }
    for r in &req.records {
        if let Some(st) = r.status.as_deref().filter(|s| !s.is_empty()) {
            if !STATUSES.contains(&st) {
                return Err(AppError::field("records", "Unknown attendance status"));
            }
        }
        let enrolled: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM course_enrollments e JOIN users u ON u.id = e.user_id WHERE e.course_id = ? AND e.user_id = ? AND u.role = 'student'",
        )
        .bind(&course.id)
        .bind(&r.student_id)
        .fetch_one(&state.pool)
        .await?;
        if enrolled == 0 {
            return Err(AppError::field("records", "Only students enrolled in this course can be marked"));
        }
    }
    let ts = now();
    let mut tx = state.pool.begin().await?;
    for r in &req.records {
        match r.status.as_deref().filter(|s| !s.is_empty()) {
            Some(status) => {
                sqlx::query(
                    "INSERT INTO attendance (course_id, week_number, day_number, student_id, status, marked_by, marked_at) VALUES (?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(course_id, week_number, day_number, student_id) DO UPDATE SET
                       status = excluded.status, marked_by = excluded.marked_by, marked_at = excluded.marked_at
                     WHERE attendance.status <> excluded.status",
                )
                .bind(&course.id)
                .bind(req.week_number)
                .bind(req.day_number)
                .bind(&r.student_id)
                .bind(status)
                .bind(&admin.id)
                .bind(&ts)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM attendance WHERE course_id = ? AND week_number = ? AND day_number = ? AND student_id = ?")
                    .bind(&course.id)
                    .bind(req.week_number)
                    .bind(req.day_number)
                    .bind(&r.student_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "attendance.mark",
        "course",
        Some(&course.id),
        serde_json::json!({ "week": req.week_number, "day": req.day_number, "records": req.records.len() }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(course_attendance(&state, &course).await?))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/attendance/me", tag = "attendance", params(("id" = String, Path)), responses((status = 200, body = MyAttendance)))]
pub async fn mine(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &user, &id).await?;
    let records: Vec<AttendanceRecord> = sqlx::query_as(
        "SELECT week_number, day_number, student_id, status FROM attendance WHERE course_id = ? AND student_id = ? ORDER BY week_number, day_number",
    )
    .bind(&course.id)
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    let ts = now();
    let sessions = sessions(&state, &course)
        .await?
        .into_iter()
        .filter_map(|mut s| {
            let status = records.iter().find(|r| r.week_number == s.week_number && r.day_number == s.day_number).map(|r| r.status.clone());
            let started = s.starts_at.as_deref().is_some_and(|t| t <= ts.as_str());
            // Students see their own record, not how many classmates were marked.
            s.marked = 0;
            (status.is_some() || started).then_some(MyAttendanceSession { session: s, status })
        })
        .collect();
    Ok(json(MyAttendance {
        course_id: course.id.clone(),
        course_title: course.title.clone(),
        sessions,
        summary: AttendanceSummary::from_records(&user.id, records.iter()),
    }))
}
