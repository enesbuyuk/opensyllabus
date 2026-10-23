use axum::{
    extract::{Path, Query, State},
    response::Response,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, no_content, ApiJson};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    models::misc::{CalendarEntry, CalendarEvent, CalendarEventInput},
    repositories::assignments::{self as assignment_repo, AssignmentFilter},
    services::progress,
    util::{new_id, now, parse_opt_ts, parse_ts, Validator},
    AppState,
};

const EVENT_KINDS: &[&str] = &["session", "lab", "milestone", "other"];

#[derive(Deserialize, IntoParams)]
pub struct CalendarQuery {
    /// Inclusive range start (ISO-8601). Defaults to 31 days ago.
    pub from: Option<String>,
    /// Exclusive range end (ISO-8601). Defaults to 62 days ahead.
    pub to: Option<String>,
    pub course_id: Option<String>,
}

const EVENT_SELECT: &str = "SELECT e.id, e.course_id, c.title AS course_title, e.title, e.description, e.kind, e.starts_at, e.ends_at,
    e.location, e.created_at, e.updated_at FROM calendar_events e JOIN courses c ON c.id = e.course_id";

#[utoipa::path(get, path = "/api/v1/calendar", tag = "calendar", params(CalendarQuery), responses((status = 200, body = [CalendarEntry])))]
pub async fn entries(State(state): State<AppState>, user: AuthUser, Query(q): Query<CalendarQuery>) -> AppResult<Response> {
    let from = match q.from.as_deref() {
        Some(f) => parse_ts("from", f)?,
        None => crate::util::fmt_ts(chrono::Utc::now() - chrono::Duration::days(31)),
    };
    let to = match q.to.as_deref() {
        Some(t) => parse_ts("to", t)?,
        None => crate::util::fmt_ts(chrono::Utc::now() + chrono::Duration::days(62)),
    };
    let mut out = Vec::new();
    let course_ids = access::visible_course_ids(&state.pool, &user).await?;
    let ts = now();
    let today = state.today();
    for cid in course_ids.iter().filter(|c| q.course_id.as_deref().is_none_or(|f| f == c.as_str())) {
        let course = access::course_for(&state.pool, &user, cid).await?;
        let events: Vec<CalendarEvent> =
            sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.course_id = ? AND e.starts_at < ? AND COALESCE(e.ends_at, e.starts_at) >= ?"))
                .bind(cid)
                .bind(&to)
                .bind(&from)
                .fetch_all(&state.pool)
                .await?;
        for e in events {
            out.push(CalendarEntry {
                id: format!("event:{}", e.id),
                kind: e.kind,
                title: e.title,
                description: e.description,
                starts_at: e.starts_at,
                ends_at: e.ends_at,
                all_day: false,
                location: e.location,
                course_id: e.course_id,
                course_title: e.course_title,
                link: None,
                event_id: Some(e.id),
            });
        }
        let assignments = assignment_repo::list(
            &state.pool,
            &user,
            &AssignmentFilter { course_id: Some(cid.clone()), ..Default::default() },
            &ts,
            &today,
        )
        .await?;
        for a in assignments {
            let Some(due) = a.assignment.due_at.clone() else { continue };
            if due < from || due >= to || a.assignment.status == "archived" {
                continue;
            }
            out.push(CalendarEntry {
                id: format!("deadline:{}", a.assignment.id),
                kind: "deadline".into(),
                title: format!("Due: {}", a.assignment.title),
                description: a.week_number.map(|w| format!("Week {w}")).unwrap_or_default(),
                starts_at: due,
                ends_at: None,
                all_day: false,
                location: String::new(),
                course_id: course.id.clone(),
                course_title: course.title.clone(),
                link: Some(format!("/assignments/{}", a.assignment.id)),
                event_id: None,
            });
        }
        // Course milestones derived from the course dates (all-day, dated in the platform timezone).
        let mut milestones: Vec<(String, String, String)> = Vec::new();
        if let Some(s) = &course.start_date {
            milestones.push(("course_start".into(), s.clone(), format!("{} begins", course.title)));
        }
        if let Some(e) = &course.end_date {
            milestones.push(("course_end".into(), e.clone(), format!("{} ends", course.title)));
        }
        if course.start_date.is_some() {
            let weeks: Vec<(String, i64, String, String, Option<String>)> =
                sqlx::query_as("SELECT id, week_number, title, status, available_from FROM weeks WHERE course_id = ? ORDER BY week_number")
                    .bind(cid)
                    .fetch_all(&state.pool)
                    .await?;
            for (wid, n, title, status, af) in weeks {
                if !user.is_admin() && (status != "published" || af.as_deref().is_some_and(|d| d > today.as_str())) {
                    continue;
                }
                if let Some(d) = progress::week_start_date(&course, n) {
                    milestones.push((format!("week_start:{wid}"), d, format!("Week {n}: {title}")));
                }
            }
        }
        for (key, date, title) in milestones {
            if date.as_str() < &from[..10] || date.as_str() >= &to[..10] {
                continue;
            }
            let (kind, link) = match key.split_once(':') {
                Some(("week_start", wid)) => ("week_start".to_string(), Some(format!("/weeks/{wid}"))),
                _ => (key.clone(), Some(format!("/courses/{}", course.id))),
            };
            out.push(CalendarEntry {
                id: format!("{key}:{}", course.id),
                kind,
                title,
                description: String::new(),
                starts_at: date,
                ends_at: None,
                all_day: true,
                location: String::new(),
                course_id: course.id.clone(),
                course_title: course.title.clone(),
                link,
                event_id: None,
            });
        }
    }
    out.sort_by(|a, b| a.starts_at.cmp(&b.starts_at));
    Ok(json(out))
}

struct Resolved {
    title: String,
    description: String,
    kind: String,
    starts_at: String,
    ends_at: Option<String>,
    location: String,
}

fn resolve(req: CalendarEventInput, base: Option<&CalendarEvent>) -> AppResult<Resolved> {
    let starts_at = match (req.starts_at, base) {
        (Some(s), _) => parse_ts("starts_at", &s)?,
        (None, Some(b)) => b.starts_at.clone(),
        (None, None) => return Err(AppError::field("starts_at", "Start time is required")),
    };
    let ends_at = match req.ends_at {
        Some(e) => parse_opt_ts("ends_at", Some(&e))?,
        None => base.and_then(|b| b.ends_at.clone()),
    };
    let r = Resolved {
        title: req.title.or(base.map(|b| b.title.clone())).unwrap_or_default().trim().to_string(),
        description: req.description.or(base.map(|b| b.description.clone())).unwrap_or_default(),
        kind: req.kind.or(base.map(|b| b.kind.clone())).unwrap_or_else(|| "session".into()),
        location: req.location.or(base.map(|b| b.location.clone())).unwrap_or_default(),
        starts_at,
        ends_at,
    };
    Validator::new()
        .len(&r.title, "title", 1, 200)
        .len(&r.description, "description", 0, 5000)
        .len(&r.location, "location", 0, 200)
        .check(EVENT_KINDS.contains(&r.kind.as_str()), "kind", "Unknown event type")
        .check(r.ends_at.as_ref().is_none_or(|e| e >= &r.starts_at), "ends_at", "End must be after start")
        .finish()?;
    Ok(r)
}

async fn load(state: &AppState, id: &str) -> AppResult<CalendarEvent> {
    sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.id = ?")).bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Event"))
}

#[utoipa::path(post, path = "/api/v1/calendar/events", tag = "calendar", request_body = CalendarEventInput, responses((status = 201, body = CalendarEvent)))]
pub async fn create(State(state): State<AppState>, admin: AdminUser, ApiJson(req): ApiJson<CalendarEventInput>) -> AppResult<Response> {
    let course_id = req.course_id.clone().ok_or_else(|| AppError::field("course_id", "Select a course"))?;
    access::course_for(&state.pool, &admin, &course_id).await?;
    let r = resolve(req, None)?;
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO calendar_events (id, course_id, title, description, kind, starts_at, ends_at, location, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&course_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.kind)
    .bind(&r.starts_at)
    .bind(&r.ends_at)
    .bind(r.location.trim())
    .bind(&admin.id)
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await?;
    Ok(created(load(&state, &id).await?))
}

#[utoipa::path(patch, path = "/api/v1/calendar/events/{id}", tag = "calendar", params(("id" = String, Path)), request_body = CalendarEventInput, responses((status = 200, body = CalendarEvent)))]
pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<CalendarEventInput>,
) -> AppResult<Response> {
    let e = load(&state, &id).await?;
    let r = resolve(req, Some(&e))?;
    sqlx::query("UPDATE calendar_events SET title = ?, description = ?, kind = ?, starts_at = ?, ends_at = ?, location = ?, updated_at = ? WHERE id = ?")
        .bind(&r.title)
        .bind(r.description.trim())
        .bind(&r.kind)
        .bind(&r.starts_at)
        .bind(&r.ends_at)
        .bind(r.location.trim())
        .bind(now())
        .bind(&id)
        .execute(&state.pool)
        .await?;
    Ok(json(load(&state, &id).await?))
}

#[utoipa::path(delete, path = "/api/v1/calendar/events/{id}", tag = "calendar", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete(State(state): State<AppState>, _admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    load(&state, &id).await?;
    sqlx::query("DELETE FROM calendar_events WHERE id = ?").bind(&id).execute(&state.pool).await?;
    Ok(no_content())
}
