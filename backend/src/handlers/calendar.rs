use axum::{
    extract::{Path, Query, State},
    response::Response,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, no_content, ApiJson};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    models::misc::{CalendarEntry, CalendarEvent, CalendarEventInput, EventSeries, Recurrence, SkipOccurrenceRequest},
    repositories::assignments::{self as assignment_repo, AssignmentFilter},
    services::{
        progress,
        recurrence::{self, Freq, Rule},
        schedule,
    },
    util::{fmt_ts, from_json_vec, is_valid_timezone, new_id, now, parse_date, parse_opt_ts, parse_ts, to_json, Validator},
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
    e.location, e.timezone, e.created_at, e.updated_at, e.repeat_freq, e.repeat_interval, e.repeat_weekdays, e.repeat_until,
    e.repeat_count, e.repeat_exceptions FROM calendar_events e JOIN courses c ON c.id = e.course_id";

fn utc(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts).ok().map(|t| t.with_timezone(&Utc))
}

/// Fills `recurrence` / `skipped_dates` from the stored columns.
fn with_series(mut e: CalendarEvent) -> CalendarEvent {
    e.recurrence = e.repeat_freq.clone().map(|freq| Recurrence {
        freq,
        interval: e.repeat_interval,
        weekdays: e.repeat_weekdays.split(',').filter_map(|d| d.trim().parse().ok()).collect(),
        until: e.repeat_until.clone(),
        count: e.repeat_count,
    });
    e.skipped_dates = from_json_vec(&e.repeat_exceptions);
    e
}

fn rule_of(e: &CalendarEvent) -> Option<Rule> {
    let r = e.recurrence.as_ref()?;
    Some(Rule {
        freq: Freq::parse(&r.freq)?,
        interval: r.interval.clamp(1, 99) as u32,
        weekdays: r.weekdays.clone(),
        until: r.until.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
        count: r.count.map(|c| c.clamp(1, 500) as u32),
        exceptions: e.skipped_dates.iter().filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()).collect(),
    })
}

fn event_entry(e: &CalendarEvent, starts_at: String, ends_at: Option<String>, series: Option<EventSeries>) -> CalendarEntry {
    CalendarEntry {
        id: match &series {
            Some(s) => format!("event:{}:{}", e.id, s.occurrence_date),
            None => format!("event:{}", e.id),
        },
        kind: e.kind.clone(),
        title: e.title.clone(),
        description: e.description.clone(),
        starts_at,
        ends_at,
        all_day: false,
        location: e.location.clone(),
        course_id: e.course_id.clone(),
        course_title: e.course_title.clone(),
        link: None,
        meeting_url: None,
        event_id: Some(e.id.clone()),
        series,
    }
}

/// The event itself, or each occurrence of a repeating event inside `[from, to)`.
fn event_entries(e: &CalendarEvent, from: &str, to: &str) -> Vec<CalendarEntry> {
    let (Some(rule), Some(start), Some(win_from), Some(win_to)) = (rule_of(e), utc(&e.starts_at), utc(from), utc(to)) else {
        return vec![event_entry(e, e.starts_at.clone(), e.ends_at.clone(), None)];
    };
    let tz = e.timezone.parse().unwrap_or(chrono_tz::UTC);
    recurrence::expand(start, e.ends_at.as_deref().and_then(utc), tz, &rule, win_from, win_to)
        .into_iter()
        .map(|o| {
            let series = EventSeries {
                recurrence: e.recurrence.clone().expect("rule_of checked it"),
                starts_at: e.starts_at.clone(),
                ends_at: e.ends_at.clone(),
                timezone: e.timezone.clone(),
                occurrence_date: o.date.to_string(),
            };
            event_entry(e, fmt_ts(o.starts_at), o.ends_at.map(fmt_ts), Some(series))
        })
        .collect()
}

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
        // Repeating events are filtered by expanding them.
        let events: Vec<CalendarEvent> = sqlx::query_as(&format!(
            "{EVENT_SELECT} WHERE e.course_id = ? AND e.starts_at < ? AND (e.repeat_freq IS NOT NULL OR COALESCE(e.ends_at, e.starts_at) >= ?)"
        ))
        .bind(cid)
        .bind(&to)
        .bind(&from)
        .fetch_all(&state.pool)
        .await?;
        for e in events.into_iter().map(with_series) {
            out.extend(event_entries(&e, &from, &to));
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
                meeting_url: None,
                event_id: None,
                series: None,
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
        out.extend(class_entries(&state, &user, &course, &from, &to, &today).await?);
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
                meeting_url: None,
                event_id: None,
                series: None,
            });
        }
    }
    out.sort_by(|a, b| a.starts_at.cmp(&b.starts_at));
    Ok(json(out))
}

/// Lessons placed on the course's weekly class schedule. Students see every class time; the
/// lesson title and link only when they could open the lesson.
async fn class_entries(
    state: &AppState,
    user: &crate::models::user::User,
    course: &crate::models::course::Course,
    from: &str,
    to: &str,
    today: &str,
) -> AppResult<Vec<CalendarEntry>> {
    let Some(start) = course.start_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) else {
        return Ok(vec![]);
    };
    let sched = super::courses::load_schedule(state, &course.id).await?;
    if sched.slots.is_empty() {
        return Ok(vec![]);
    }
    let (Some(win_from), Some(win_to)) = (utc(from), utc(to)) else { return Ok(vec![]) };
    let tz: chrono_tz::Tz = sched.timezone.parse().unwrap_or(chrono_tz::UTC);
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        title: String,
        day_number: i64,
        status: String,
        week_status: String,
        available_from: Option<String>,
        week_number: i64,
        meeting_url: Option<String>,
    }
    let lessons: Vec<Row> = sqlx::query_as(
        "SELECT l.id, l.title, l.day_number, l.status, w.status AS week_status, w.available_from, w.week_number, l.meeting_url
         FROM lessons l JOIN weeks w ON w.id = l.week_id WHERE w.course_id = ? ORDER BY w.week_number, l.day_number, l.position",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let mut out = Vec::new();
    for Row {
        id: lesson_id,
        title,
        day_number: day,
        status: lesson_status,
        week_status,
        available_from,
        week_number,
        meeting_url: lesson_meeting,
    } in lessons
    {
        let Some(slot) = sched.slots.iter().find(|s| s.day_number == day) else { continue };
        let Some(start_time) = schedule::parse_time(&slot.start_time) else { continue };
        let date = schedule::class_date(start, week_number, slot.weekday as u32);
        let (starts, ends) = schedule::class_times(date, start_time, slot.end_time.as_deref().and_then(schedule::parse_time), tz);
        if starts >= win_to || ends.unwrap_or(starts) < win_from {
            continue;
        }
        let open = user.is_admin()
            || (lesson_status == "published" && week_status == "published" && available_from.as_deref().is_none_or(|d| d <= today));
        out.push(CalendarEntry {
            id: format!("class:{lesson_id}"),
            kind: "class".into(),
            title: if open { format!("W{week_number} · {title}") } else { format!("Week {week_number}, day {day}") },
            description: String::new(),
            starts_at: fmt_ts(starts),
            ends_at: ends.map(fmt_ts),
            all_day: false,
            location: slot.location.clone(),
            course_id: course.id.clone(),
            course_title: course.title.clone(),
            link: open.then(|| format!("/lessons/{lesson_id}")),
            meeting_url: if open { lesson_meeting.or_else(|| slot.meeting_url.clone()) } else { slot.meeting_url.clone() },
            event_id: None,
            series: None,
        });
    }
    Ok(out)
}

struct Resolved {
    title: String,
    description: String,
    kind: String,
    starts_at: String,
    ends_at: Option<String>,
    location: String,
    timezone: String,
    recurrence: Option<Recurrence>,
}

/// Validates a repeat rule; `None` for `freq: "none"`.
fn clean_recurrence(r: Recurrence, starts_at: &str, timezone: &str) -> AppResult<Option<Recurrence>> {
    if r.freq == "none" {
        return Ok(None);
    }
    let mut weekdays = if r.freq == "weekly" { r.weekdays } else { vec![] };
    weekdays.sort_unstable();
    weekdays.dedup();
    let until = r.until.map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
    let start_date = utc(starts_at)
        .map(|t| t.with_timezone(&timezone.parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::UTC)).date_naive())
        .unwrap_or_default();
    let mut v = Validator::new();
    v.check(Freq::parse(&r.freq).is_some(), "recurrence", "Repeat must be daily, weekly or monthly")
        .check((1..=99).contains(&r.interval), "recurrence", "Repeat interval must be between 1 and 99")
        .check(weekdays.iter().all(|d| (1..=7).contains(d)), "recurrence", "Unknown weekday")
        .check(r.count.is_none_or(|c| (1..=500).contains(&c)), "recurrence", "Number of occurrences must be between 1 and 500")
        .check(until.is_none() || r.count.is_none(), "recurrence", "Choose an end date or a number of occurrences, not both");
    if let Some(u) = &until {
        let d = parse_date("recurrence", u)?;
        v.check(d >= start_date, "recurrence", "The repeat end date must be on or after the start");
    }
    v.finish()?;
    Ok(Some(Recurrence { freq: r.freq, interval: r.interval, weekdays, until, count: r.count }))
}

fn resolve(req: CalendarEventInput, base: Option<&CalendarEvent>, default_tz: &str) -> AppResult<Resolved> {
    let starts_at = match (req.starts_at, base) {
        (Some(s), _) => parse_ts("starts_at", &s)?,
        (None, Some(b)) => b.starts_at.clone(),
        (None, None) => return Err(AppError::field("starts_at", "Start time is required")),
    };
    let ends_at = match req.ends_at {
        Some(e) => parse_opt_ts("ends_at", Some(&e))?,
        None => base.and_then(|b| b.ends_at.clone()),
    };
    let timezone = req.timezone.or(base.map(|b| b.timezone.clone())).unwrap_or_else(|| default_tz.to_string());
    if !is_valid_timezone(&timezone) {
        return Err(AppError::field("timezone", "Unknown timezone"));
    }
    let recurrence = match req.recurrence {
        Some(r) => clean_recurrence(r, &starts_at, &timezone)?,
        None => match base.and_then(|b| b.recurrence.clone()) {
            // Re-check against a changed start or timezone.
            Some(r) => clean_recurrence(r, &starts_at, &timezone)?,
            None => None,
        },
    };
    let r = Resolved {
        title: req.title.or(base.map(|b| b.title.clone())).unwrap_or_default().trim().to_string(),
        description: req.description.or(base.map(|b| b.description.clone())).unwrap_or_default(),
        kind: req.kind.or(base.map(|b| b.kind.clone())).unwrap_or_else(|| "session".into()),
        location: req.location.or(base.map(|b| b.location.clone())).unwrap_or_default(),
        starts_at,
        ends_at,
        timezone,
        recurrence,
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
    let e: CalendarEvent = sqlx::query_as(&format!("{EVENT_SELECT} WHERE e.id = ?"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("Event"))?;
    Ok(with_series(e))
}

#[utoipa::path(post, path = "/api/v1/calendar/events", tag = "calendar", request_body = CalendarEventInput, responses((status = 201, body = CalendarEvent)))]
pub async fn create(State(state): State<AppState>, admin: AdminUser, ApiJson(req): ApiJson<CalendarEventInput>) -> AppResult<Response> {
    let course_id = req.course_id.clone().ok_or_else(|| AppError::field("course_id", "Select a course"))?;
    access::course_for(&state.pool, &admin, &course_id).await?;
    let r = resolve(req, None, state.timezone().name())?;
    let id = new_id();
    let ts = now();
    let rec = r.recurrence.as_ref();
    sqlx::query(
        "INSERT INTO calendar_events (id, course_id, title, description, kind, starts_at, ends_at, location, timezone, repeat_freq,
            repeat_interval, repeat_weekdays, repeat_until, repeat_count, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&course_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.kind)
    .bind(&r.starts_at)
    .bind(&r.ends_at)
    .bind(r.location.trim())
    .bind(&r.timezone)
    .bind(rec.map(|x| x.freq.clone()))
    .bind(rec.map_or(1, |x| x.interval))
    .bind(rec.map(|x| weekdays_column(&x.weekdays)).unwrap_or_default())
    .bind(rec.and_then(|x| x.until.clone()))
    .bind(rec.and_then(|x| x.count))
    .bind(&admin.id)
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await?;
    Ok(created(load(&state, &id).await?))
}

fn weekdays_column(days: &[u32]) -> String {
    days.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

#[utoipa::path(patch, path = "/api/v1/calendar/events/{id}", tag = "calendar", params(("id" = String, Path)), request_body = CalendarEventInput, responses((status = 200, body = CalendarEvent)))]
pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<CalendarEventInput>,
) -> AppResult<Response> {
    let e = load(&state, &id).await?;
    let r = resolve(req, Some(&e), state.timezone().name())?;
    let rec = r.recurrence.as_ref();
    // Skipped dates belong to the old pattern; a series that stops repeating drops them.
    let exceptions = if rec.is_some() { to_json(&e.skipped_dates) } else { "[]".to_string() };
    sqlx::query(
        "UPDATE calendar_events SET title = ?, description = ?, kind = ?, starts_at = ?, ends_at = ?, location = ?, timezone = ?,
            repeat_freq = ?, repeat_interval = ?, repeat_weekdays = ?, repeat_until = ?, repeat_count = ?, repeat_exceptions = ?,
            updated_at = ? WHERE id = ?",
    )
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.kind)
    .bind(&r.starts_at)
    .bind(&r.ends_at)
    .bind(r.location.trim())
    .bind(&r.timezone)
    .bind(rec.map(|x| x.freq.clone()))
    .bind(rec.map_or(1, |x| x.interval))
    .bind(rec.map(|x| weekdays_column(&x.weekdays)).unwrap_or_default())
    .bind(rec.and_then(|x| x.until.clone()))
    .bind(rec.and_then(|x| x.count))
    .bind(exceptions)
    .bind(now())
    .bind(&id)
    .execute(&state.pool)
    .await?;
    Ok(json(load(&state, &id).await?))
}

#[utoipa::path(post, path = "/api/v1/calendar/events/{id}/skip", tag = "calendar", params(("id" = String, Path)), request_body = SkipOccurrenceRequest, responses((status = 200, body = CalendarEvent)))]
pub async fn skip_occurrence(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<SkipOccurrenceRequest>,
) -> AppResult<Response> {
    let e = load(&state, &id).await?;
    if e.recurrence.is_none() {
        return Err(AppError::bad_request("This event does not repeat; delete it instead"));
    }
    let date = parse_date("date", req.date.trim())?.to_string();
    let mut skipped = e.skipped_dates.clone();
    if !skipped.contains(&date) {
        skipped.push(date);
        skipped.sort();
    }
    sqlx::query("UPDATE calendar_events SET repeat_exceptions = ?, updated_at = ? WHERE id = ?")
        .bind(to_json(&skipped))
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
