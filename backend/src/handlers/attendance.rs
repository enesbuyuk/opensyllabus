use axum::{
    extract::{Path, State},
    response::Response,
    Extension,
};
use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};

use super::{
    calendar::{self, EVENT_SELECT},
    json, ApiJson,
};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        attendance::{
            class_key, event_key, AttendanceInput, AttendanceRecord, AttendanceSession, AttendanceSummary, CourseAttendance, MyAttendance,
            MyAttendanceSession, STATUSES,
        },
        course::Course,
        misc::CalendarEvent,
        user::UserSummary,
    },
    services::{audit, schedule},
    util::{fmt_ts, now},
    AppState,
};

/// Every class of the course (each scheduled course day in each course week) and every occurrence
/// of its calendar events that take attendance, ordered by start time. `marked` is left at 0.
async fn sessions(state: &AppState, course: &Course) -> AppResult<Vec<AttendanceSession>> {
    let sched = super::courses::load_schedule(state, &course.id).await?;
    let tz: chrono_tz::Tz = sched.timezone.parse().unwrap_or(chrono_tz::UTC);
    let start = course.start_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let mut out = Vec::new();
    for week_number in 1..=course.duration_weeks {
        for slot in &sched.slots {
            let date = start.map(|s| schedule::class_date(s, week_number, slot.weekday as u32));
            let times = match (date, schedule::parse_time(&slot.start_time)) {
                (Some(d), Some(t)) => Some(schedule::class_times(d, t, slot.end_time.as_deref().and_then(schedule::parse_time), tz)),
                _ => None,
            };
            out.push(AttendanceSession {
                key: class_key(week_number, slot.day_number),
                kind: "class".into(),
                week_number: Some(week_number),
                day_number: Some(slot.day_number),
                event_id: None,
                occurrence_date: None,
                title: None,
                date: date.map(|d| d.format("%Y-%m-%d").to_string()),
                starts_at: times.map(|(s, _)| fmt_ts(s)),
                ends_at: times.and_then(|(_, e)| e.map(fmt_ts)),
                location: slot.location.clone(),
                marked: 0,
            });
        }
    }
    out.extend(event_sessions(state, course).await?);
    // Stable: classes without a date (no course start date) keep their week order.
    out.sort_by_key(|s| s.starts_at.clone().unwrap_or_default());
    Ok(out)
}

/// Occurrences of the course's events that take attendance, up to a month after the later of
/// today and the course end (and at most two years after the event's first occurrence).
async fn event_sessions(state: &AppState, course: &Course) -> AppResult<Vec<AttendanceSession>> {
    let events: Vec<CalendarEvent> = sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.course_id = ? AND e.takes_attendance = 1"))
        .bind(&course.id)
        .fetch_all(&state.pool)
        .await?;
    let course_end = course
        .end_date
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|d| d.and_utc());
    let horizon = course_end.map_or(Utc::now(), |e| e.max(Utc::now())) + Duration::days(31);
    let mut out = Vec::new();
    for e in events.into_iter().map(calendar::with_series) {
        let Ok(first) = DateTime::parse_from_rfc3339(&e.starts_at).map(|t| t.with_timezone(&Utc)) else { continue };
        let to = horizon.min(first + Duration::days(731));
        let tz: chrono_tz::Tz = e.timezone.parse().unwrap_or(chrono_tz::UTC);
        for entry in calendar::event_entries(&e, &e.starts_at, &fmt_ts(to)) {
            let date = match &entry.series {
                Some(s) => s.occurrence_date.clone(),
                None => match DateTime::parse_from_rfc3339(&entry.starts_at) {
                    Ok(t) => t.with_timezone(&tz).format("%Y-%m-%d").to_string(),
                    Err(_) => continue,
                },
            };
            out.push(AttendanceSession {
                key: event_key(&e.id, &date),
                kind: "event".into(),
                week_number: None,
                day_number: None,
                event_id: Some(e.id.clone()),
                occurrence_date: Some(date.clone()),
                title: Some(e.title.clone()),
                date: Some(date),
                starts_at: Some(entry.starts_at),
                ends_at: entry.ends_at,
                location: e.location.clone(),
                marked: 0,
            });
        }
    }
    Ok(out)
}

/// Attendance records of the course, optionally for one student. Event records only count while
/// their occurrence is a session (the event still takes attendance and the date was not skipped).
async fn records(
    state: &AppState,
    course_id: &str,
    student_id: Option<&str>,
    sessions: &[AttendanceSession],
) -> AppResult<Vec<AttendanceRecord>> {
    let rows: Vec<AttendanceRecord> = sqlx::query_as(
        "SELECT 'class:' || week_number || '-' || day_number AS key, student_id, status FROM attendance
         WHERE course_id = ?1 AND (?2 IS NULL OR student_id = ?2)
         UNION ALL
         SELECT 'event:' || a.event_id || ':' || a.occurrence_date AS key, a.student_id, a.status
         FROM event_attendance a JOIN calendar_events e ON e.id = a.event_id
         WHERE e.course_id = ?1 AND (?2 IS NULL OR a.student_id = ?2)",
    )
    .bind(course_id)
    .bind(student_id)
    .fetch_all(&state.pool)
    .await?;
    let keys: HashSet<&str> = sessions.iter().map(|s| s.key.as_str()).collect();
    Ok(rows.into_iter().filter(|r| r.key.starts_with("class:") || keys.contains(r.key.as_str())).collect())
}

async fn course_attendance(state: &AppState, course: &Course) -> AppResult<CourseAttendance> {
    let students: Vec<UserSummary> = sqlx::query_as(
        "SELECT u.id, u.name, u.email FROM users u JOIN course_enrollments e ON e.user_id = u.id
         WHERE e.course_id = ? AND u.role = 'student' AND u.is_active = 1 ORDER BY u.name COLLATE NOCASE",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let mut sessions = sessions(state, course).await?;
    let records = records(state, &course.id, None, &sessions).await?;
    for s in &mut sessions {
        s.marked = records.iter().filter(|r| r.key == s.key).count() as i64;
    }
    let summaries = students.iter().map(|s| AttendanceSummary::from_records(&s.id, records.iter())).collect();
    Ok(CourseAttendance { sessions, students, records, summaries })
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
    let key = req.session_key().ok_or_else(|| AppError::bad_request("Choose a class or an event occurrence"))?;
    let all = sessions(&state, &course).await?;
    let Some(session) = all.into_iter().find(|s| s.key == key) else {
        return Err(AppError::bad_request("This class or event is not on the course's attendance list"));
    };
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
        let status = r.status.as_deref().filter(|s| !s.is_empty());
        match (&session.event_id, &session.occurrence_date) {
            (Some(event_id), Some(date)) => {
                let q = match status {
                    Some(status) => sqlx::query(
                        "INSERT INTO event_attendance (event_id, occurrence_date, student_id, status, marked_by, marked_at) VALUES (?, ?, ?, ?, ?, ?)
                         ON CONFLICT(event_id, occurrence_date, student_id) DO UPDATE SET
                           status = excluded.status, marked_by = excluded.marked_by, marked_at = excluded.marked_at
                         WHERE event_attendance.status <> excluded.status",
                    )
                    .bind(event_id)
                    .bind(date)
                    .bind(&r.student_id)
                    .bind(status)
                    .bind(&admin.id)
                    .bind(&ts),
                    None => sqlx::query("DELETE FROM event_attendance WHERE event_id = ? AND occurrence_date = ? AND student_id = ?")
                        .bind(event_id)
                        .bind(date)
                        .bind(&r.student_id),
                };
                q.execute(&mut *tx).await?;
            }
            _ => {
                let (week, day) = (session.week_number.unwrap_or_default(), session.day_number.unwrap_or_default());
                let q = match status {
                    Some(status) => sqlx::query(
                        "INSERT INTO attendance (course_id, week_number, day_number, student_id, status, marked_by, marked_at) VALUES (?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT(course_id, week_number, day_number, student_id) DO UPDATE SET
                           status = excluded.status, marked_by = excluded.marked_by, marked_at = excluded.marked_at
                         WHERE attendance.status <> excluded.status",
                    )
                    .bind(&course.id)
                    .bind(week)
                    .bind(day)
                    .bind(&r.student_id)
                    .bind(status)
                    .bind(&admin.id)
                    .bind(&ts),
                    None => sqlx::query("DELETE FROM attendance WHERE course_id = ? AND week_number = ? AND day_number = ? AND student_id = ?")
                        .bind(&course.id)
                        .bind(week)
                        .bind(day)
                        .bind(&r.student_id),
                };
                q.execute(&mut *tx).await?;
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
        serde_json::json!({
            "session": session.key,
            "week": session.week_number,
            "day": session.day_number,
            "event": session.title,
            "date": session.date,
            "records": req.records.len(),
        }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(course_attendance(&state, &course).await?))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/attendance/me", tag = "attendance", params(("id" = String, Path)), responses((status = 200, body = MyAttendance)))]
pub async fn mine(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &user, &id).await?;
    let all = sessions(&state, &course).await?;
    let records = records(&state, &course.id, Some(&user.id), &all).await?;
    let ts = now();
    let sessions = all
        .into_iter()
        .filter_map(|s| {
            let status = records.iter().find(|r| r.key == s.key).map(|r| r.status.clone());
            let started = s.starts_at.as_deref().is_some_and(|t| t <= ts.as_str());
            // Students see their own record, not how many classmates were marked (`marked` stays 0).
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
