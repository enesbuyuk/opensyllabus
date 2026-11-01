use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::Response,
    Extension,
};

use super::{created, json, no_content, ApiJson, IdResponse};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{course::Course, curriculum::*, misc::Resource, user::User},
    repositories::{
        assignments::{self as assignment_repo, AssignmentFilter},
        resources::{self as resource_repo, ResourceFilter},
    },
    services::{audit, curriculum, progress, schedule},
    util::{clean_list, fmt_ts, is_http_url, new_id, now, parse_opt_date, to_json, Validator},
    AppState,
};

// ---------------------------------------------------------------------------
// Week summaries
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct WeekCounts {
    week_id: String,
    lesson_count: i64,
    published_lesson_count: i64,
}

async fn summaries(state: &AppState, user: &User, course: &Course) -> AppResult<(Option<i64>, Vec<WeekSummary>)> {
    let today = state.today();
    let ts = now();
    let data = progress::CourseData::load(&state.pool, course.clone()).await?;
    let counts: Vec<WeekCounts> = sqlx::query_as(
        "SELECT w.id AS week_id, COUNT(l.id) AS lesson_count, COALESCE(SUM(l.status = 'published'), 0) AS published_lesson_count
         FROM weeks w LEFT JOIN lessons l ON l.week_id = w.id WHERE w.course_id = ? GROUP BY w.id",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let counts: HashMap<String, WeekCounts> = counts.into_iter().map(|c| (c.week_id.clone(), c)).collect();
    let assignment_counts: Vec<(String, i64)> = if user.is_admin() {
        sqlx::query_as("SELECT week_id, COUNT(*) FROM assignments WHERE course_id = ? AND week_id IS NOT NULL AND status <> 'archived' GROUP BY week_id")
            .bind(&course.id)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query_as(
            "SELECT week_id, COUNT(*) FROM assignments WHERE course_id = ? AND week_id IS NOT NULL AND status = 'published'
             AND (publish_at IS NULL OR publish_at <= ?) GROUP BY week_id",
        )
        .bind(&course.id)
        .bind(&ts)
        .fetch_all(&state.pool)
        .await?
    };
    let assignment_counts: HashMap<String, i64> = assignment_counts.into_iter().collect();

    let (current, week_progress) = if user.is_admin() {
        (progress::calendar_week(course, &today), None)
    } else {
        let p = data.student_progress(&state.pool, &user.id, &today, &ts).await?;
        (p.current_week, Some(p.weeks))
    };

    let mut out = Vec::new();
    for w in data.weeks {
        let c = counts.get(&w.id);
        let wp = week_progress.as_ref().and_then(|v| v.iter().find(|p| p.week_id == w.id));
        let open = access::week_is_open(&w, &today);
        let state_ = match wp {
            Some(p) => p.state,
            None if !open => WeekState::Locked,
            None if Some(w.week_number) == current => WeekState::Current,
            None => WeekState::Available,
        };
        let starts_on = progress::week_start_date(course, w.week_number);
        let mut week = w;
        if !user.is_admin() && !open {
            // Locked weeks reveal only their number and title.
            week.description.clear();
            week.objectives.0.clear();
            week.instructor_notes.clear();
        }
        out.push(WeekSummary {
            state: state_,
            lesson_count: c.map(|c| c.lesson_count).unwrap_or(0),
            published_lesson_count: c.map(|c| c.published_lesson_count).unwrap_or(0),
            completed_lesson_count: wp.map(|p| p.lessons_completed).unwrap_or(0),
            assignment_count: assignment_counts.get(&week.id).copied().unwrap_or(0),
            completion_percent: wp.map(|p| p.completion_percent).unwrap_or(0.0),
            starts_on,
            week,
        });
    }
    Ok((current, out))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/weeks", tag = "curriculum", params(("id" = String, Path)), responses((status = 200, body = CurriculumResponse)))]
pub async fn course_curriculum(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &user, &id).await?;
    let modules: Vec<Module> = sqlx::query_as("SELECT * FROM modules WHERE course_id = ? ORDER BY position, created_at")
        .bind(&course.id)
        .fetch_all(&state.pool)
        .await?;
    let (current_week, weeks) = summaries(&state, &user, &course).await?;
    Ok(json(CurriculumResponse { course_id: course.id, current_week, modules, weeks }))
}

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

#[utoipa::path(post, path = "/api/v1/courses/{id}/modules", tag = "curriculum", params(("id" = String, Path)), request_body = ModuleInput, responses((status = 201, body = Module)))]
pub async fn create_module(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ModuleInput>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    Validator::new().len(&req.title, "title", 1, 200).len(&req.description, "description", 0, 5000).finish()?;
    let position = match req.position {
        Some(p) => p,
        None => {
            sqlx::query_scalar("SELECT COALESCE(MAX(position) + 1, 0) FROM modules WHERE course_id = ?")
                .bind(&course.id)
                .fetch_one(&state.pool)
                .await?
        }
    };
    let id = new_id();
    let ts = now();
    sqlx::query("INSERT INTO modules (id, course_id, title, description, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&course.id)
        .bind(req.title.trim())
        .bind(req.description.trim())
        .bind(position)
        .bind(&ts)
        .bind(&ts)
        .execute(&state.pool)
        .await?;
    let m: Module = sqlx::query_as("SELECT * FROM modules WHERE id = ?").bind(&id).fetch_one(&state.pool).await?;
    Ok(created(m))
}

async fn load_module(state: &AppState, id: &str) -> AppResult<Module> {
    sqlx::query_as("SELECT * FROM modules WHERE id = ?").bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Module"))
}

#[utoipa::path(patch, path = "/api/v1/modules/{id}", tag = "curriculum", params(("id" = String, Path)), request_body = UpdateModuleRequest, responses((status = 200, body = Module)))]
pub async fn update_module(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<UpdateModuleRequest>,
) -> AppResult<Response> {
    let m = load_module(&state, &id).await?;
    let title = req.title.unwrap_or(m.title);
    let description = req.description.unwrap_or(m.description);
    Validator::new().len(&title, "title", 1, 200).len(&description, "description", 0, 5000).finish()?;
    sqlx::query("UPDATE modules SET title = ?, description = ?, position = ?, updated_at = ? WHERE id = ?")
        .bind(title.trim())
        .bind(description.trim())
        .bind(req.position.unwrap_or(m.position))
        .bind(now())
        .bind(&id)
        .execute(&state.pool)
        .await?;
    Ok(json(load_module(&state, &id).await?))
}

#[utoipa::path(delete, path = "/api/v1/modules/{id}", tag = "curriculum", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete_module(State(state): State<AppState>, _admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    load_module(&state, &id).await?;
    // Weeks are kept and simply detached (FK ON DELETE SET NULL).
    sqlx::query("DELETE FROM modules WHERE id = ?").bind(&id).execute(&state.pool).await?;
    Ok(no_content())
}

// ---------------------------------------------------------------------------
// Weeks
// ---------------------------------------------------------------------------

async fn validate_module(state: &AppState, course_id: &str, module_id: Option<&str>) -> AppResult<()> {
    if let Some(mid) = module_id {
        let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM modules WHERE id = ? AND course_id = ?")
            .bind(mid)
            .bind(course_id)
            .fetch_one(&state.pool)
            .await?;
        if ok == 0 {
            return Err(AppError::field("module_id", "Module does not belong to this course"));
        }
    }
    Ok(())
}

#[utoipa::path(post, path = "/api/v1/courses/{id}/weeks", tag = "curriculum", params(("id" = String, Path)), request_body = CreateWeekRequest, responses((status = 201, body = IdResponse)))]
pub async fn create_week(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<CreateWeekRequest>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let number = match req.week_number {
        Some(n) => n,
        None => {
            sqlx::query_scalar("SELECT COALESCE(MAX(week_number) + 1, 1) FROM weeks WHERE course_id = ?")
                .bind(&course.id)
                .fetch_one(&state.pool)
                .await?
        }
    };
    Validator::new()
        .len(&req.title, "title", 1, 200)
        .len(&req.description, "description", 0, 20_000)
        .len(&req.instructor_notes, "instructor_notes", 0, 20_000)
        .check((1..=104).contains(&number), "week_number", "Week number must be between 1 and 104")
        .check((0..=10_000).contains(&req.estimated_minutes), "estimated_minutes", "Invalid duration")
        .finish()?;
    let module_id = req.module_id.filter(|m| !m.is_empty());
    validate_module(&state, &course.id, module_id.as_deref()).await?;
    let objectives = clean_list(req.objectives, "objectives", 20)?;
    let wid = new_id();
    let ts = now();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO weeks (id, course_id, module_id, week_number, title, description, objectives, instructor_notes, estimated_minutes, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&wid)
    .bind(&course.id)
    .bind(&module_id)
    .bind(number)
    .bind(req.title.trim())
    .bind(req.description.trim())
    .bind(to_json(&objectives))
    .bind(req.instructor_notes.trim())
    .bind(req.estimated_minutes)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await
    .map_err(|e| match AppError::from(e) {
        AppError::Conflict(_) => AppError::field("week_number", "This week number already exists"),
        other => other,
    })?;
    if req.create_day_lessons {
        curriculum::insert_lesson_shells(&mut tx, &wid, course.days_per_week).await?;
    }
    tx.commit().await?;
    Ok(created(IdResponse { id: wid }))
}

#[utoipa::path(post, path = "/api/v1/courses/{id}/weeks/generate", tag = "curriculum", params(("id" = String, Path)), request_body = GenerateWeeksRequest, responses((status = 200, body = super::CountResponse)))]
pub async fn generate_weeks(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<GenerateWeeksRequest>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let n = curriculum::generate_weeks(&state.pool, &course, req.create_day_lessons).await?;
    Ok(json(super::CountResponse { count: n }))
}

async fn lesson_items(state: &AppState, user: &User, week_id: &str) -> AppResult<Vec<LessonListItem>> {
    let lessons: Vec<Lesson> = if user.is_admin() {
        sqlx::query_as("SELECT * FROM lessons WHERE week_id = ? ORDER BY day_number, position").bind(week_id).fetch_all(&state.pool).await?
    } else {
        sqlx::query_as("SELECT * FROM lessons WHERE week_id = ? AND status = 'published' ORDER BY day_number, position")
            .bind(week_id)
            .fetch_all(&state.pool)
            .await?
    };
    let completed: Vec<String> = sqlx::query_scalar(
        "SELECT lp.lesson_id FROM lesson_progress lp JOIN lessons l ON l.id = lp.lesson_id WHERE l.week_id = ? AND lp.user_id = ?",
    )
    .bind(week_id)
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    let mut out = Vec::new();
    for mut l in lessons {
        let unmet = if user.is_admin() { false } else { !prerequisites_met(state, &l.id, &user.id).await? };
        if !user.is_admin() {
            // Full content is served by the lesson endpoint.
            l.content_md.clear();
            l.lab_md.clear();
        }
        out.push(LessonListItem { completed: completed.contains(&l.id), locked_by_prerequisites: unmet, lesson: l });
    }
    Ok(out)
}

async fn prerequisites_met(state: &AppState, lesson_id: &str, user_id: &str) -> AppResult<bool> {
    let missing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM lesson_prerequisites p JOIN lessons l ON l.id = p.prerequisite_lesson_id
         WHERE p.lesson_id = ? AND l.status = 'published'
           AND NOT EXISTS (SELECT 1 FROM lesson_progress lp WHERE lp.lesson_id = p.prerequisite_lesson_id AND lp.user_id = ?)",
    )
    .bind(lesson_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(missing == 0)
}

async fn resources_for(state: &AppState, user: &User, week_id: Option<&str>, lesson_id: Option<&str>) -> AppResult<Vec<Resource>> {
    let f = ResourceFilter {
        week_id: week_id.map(String::from),
        lesson_id: lesson_id.map(String::from),
        sort: Some("title".into()),
        ..Default::default()
    };
    resource_repo::list(&state.pool, user, &f, &state.today()).await
}

#[utoipa::path(get, path = "/api/v1/weeks/{id}", tag = "curriculum", params(("id" = String, Path)), responses((status = 200, body = WeekDetail)))]
pub async fn get_week(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let today = state.today();
    let (week, course) = access::week_for(&state.pool, &user, &id, &today).await?;
    let (_, summaries) = summaries(&state, &user, &course).await?;
    let idx = summaries.iter().position(|s| s.week.id == week.id).ok_or(AppError::NotFound("Week"))?;
    let navigable = |s: &WeekSummary| user.is_admin() || s.state != WeekState::Locked;
    let previous_week_id = summaries[..idx].iter().rev().find(|s| navigable(s)).map(|s| s.week.id.clone());
    let next_week_id = summaries[idx + 1..].iter().find(|s| navigable(s)).map(|s| s.week.id.clone());
    let summary = summaries.into_iter().nth(idx).unwrap();
    let module = match &week.module_id {
        Some(m) => sqlx::query_as("SELECT * FROM modules WHERE id = ?").bind(m).fetch_optional(&state.pool).await?,
        None => None,
    };
    let lessons = lesson_items(&state, &user, &week.id).await?;
    let assignments = assignment_repo::list(
        &state.pool,
        &user,
        &AssignmentFilter { week_id: Some(week.id.clone()), ..Default::default() },
        &now(),
        &today,
    )
    .await?;
    let resources = resources_for(&state, &user, Some(&week.id), None).await?;
    Ok(json(WeekDetail { summary, module, lessons, assignments, resources, previous_week_id, next_week_id }))
}

async fn load_week(state: &AppState, id: &str) -> AppResult<Week> {
    sqlx::query_as("SELECT * FROM weeks WHERE id = ?").bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Week"))
}

#[utoipa::path(patch, path = "/api/v1/weeks/{id}", tag = "curriculum", params(("id" = String, Path)), request_body = UpdateWeekRequest, responses((status = 200, body = Week)))]
pub async fn update_week(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<UpdateWeekRequest>,
) -> AppResult<Response> {
    let w = load_week(&state, &id).await?;
    let title = req.title.unwrap_or(w.title.clone());
    let description = req.description.unwrap_or(w.description.clone());
    let notes = req.instructor_notes.unwrap_or(w.instructor_notes.clone());
    let minutes = req.estimated_minutes.unwrap_or(w.estimated_minutes);
    let status = req.status.unwrap_or(w.status.clone());
    Validator::new()
        .len(&title, "title", 1, 200)
        .len(&description, "description", 0, 20_000)
        .len(&notes, "instructor_notes", 0, 20_000)
        .check((0..=10_000).contains(&minutes), "estimated_minutes", "Invalid duration")
        .check(matches!(status.as_str(), "draft" | "published"), "status", "Status must be draft or published")
        .finish()?;
    let module_id = match req.module_id {
        Some(m) if m.is_empty() => None,
        Some(m) => Some(m),
        None => w.module_id.clone(),
    };
    validate_module(&state, &w.course_id, module_id.as_deref()).await?;
    let available_from = match req.available_from {
        Some(d) => parse_opt_date("available_from", Some(&d))?,
        None => w.available_from.clone(),
    };
    let objectives = match req.objectives {
        Some(o) => clean_list(o, "objectives", 20)?,
        None => w.objectives.0.clone(),
    };
    let ts = now();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE weeks SET title = ?, description = ?, module_id = ?, objectives = ?, instructor_notes = ?, estimated_minutes = ?,
         status = ?, available_from = ?, updated_at = ? WHERE id = ?",
    )
    .bind(title.trim())
    .bind(description.trim())
    .bind(&module_id)
    .bind(to_json(&objectives))
    .bind(notes.trim())
    .bind(minutes)
    .bind(&status)
    .bind(&available_from)
    .bind(&ts)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    if req.cascade_lessons && status != w.status {
        sqlx::query("UPDATE lessons SET status = ?, published_at = CASE WHEN ? = 'published' THEN COALESCE(published_at, ?) ELSE published_at END, updated_at = ? WHERE week_id = ?")
            .bind(&status)
            .bind(&status)
            .bind(&ts)
            .bind(&ts)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    let updated = load_week(&state, &id).await?;
    if status != w.status {
        audit::log(
            &state.pool,
            Some(&admin.id),
            &format!("week.{status}"),
            "week",
            Some(&id),
            serde_json::json!({ "week_number": w.week_number }),
            ip.0.as_deref(),
        )
        .await;
        if status == "published" {
            let today = state.today();
            let lessons: Vec<Lesson> =
                sqlx::query_as("SELECT * FROM lessons WHERE week_id = ? AND status = 'published'").bind(&id).fetch_all(&state.pool).await?;
            for l in &lessons {
                curriculum::notify_lesson_available(&state.pool, l, &updated, &today).await?;
            }
        }
    }
    Ok(json(updated))
}

#[utoipa::path(delete, path = "/api/v1/weeks/{id}", tag = "curriculum", params(("id" = String, Path)), responses((status = 204), (status = 409)))]
pub async fn delete_week(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let w = load_week(&state, &id).await?;
    let subs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM submissions s JOIN assignments a ON a.id = s.assignment_id WHERE a.week_id = ?")
            .bind(&id)
            .fetch_one(&state.pool)
            .await?;
    if subs > 0 {
        return Err(AppError::Conflict("This week has assignments with student submissions and cannot be deleted".into()));
    }
    sqlx::query("DELETE FROM weeks WHERE id = ?").bind(&id).execute(&state.pool).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "week.delete",
        "week",
        Some(&id),
        serde_json::json!({ "week_number": w.week_number }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/weeks/{id}/duplicate", tag = "curriculum", params(("id" = String, Path)), request_body = DuplicateWeekRequest, responses((status = 201, body = IdResponse)))]
pub async fn duplicate_week(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<DuplicateWeekRequest>,
) -> AppResult<Response> {
    let w = load_week(&state, &id).await?;
    let new_id = curriculum::duplicate_week(&state.pool, &w, req.target_week_number).await?;
    Ok(created(IdResponse { id: new_id }))
}

// ---------------------------------------------------------------------------
// Lessons
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn validate_lesson_fields(
    v: &mut Validator,
    title: &str,
    summary: &str,
    content: &str,
    lab: &str,
    day: i64,
    minutes: i64,
    video: Option<&str>,
    meeting: Option<&str>,
) {
    v.len(title, "title", 1, 200)
        .len(summary, "summary", 0, 2000)
        .len(content, "content_md", 0, 200_000)
        .len(lab, "lab_md", 0, 100_000)
        .check((1..=7).contains(&day), "day_number", "Day must be between 1 and 7")
        .check((0..=10_000).contains(&minutes), "estimated_minutes", "Invalid duration");
    if let Some(u) = video.filter(|u| !u.is_empty()) {
        v.check(is_http_url(u) && u.starts_with("https://"), "video_url", "Video URL must be an https link");
    }
    if let Some(u) = meeting.filter(|u| !u.is_empty()) {
        v.check(is_http_url(u) && u.starts_with("https://"), "meeting_url", "Class link must be an https link");
    }
}

#[utoipa::path(post, path = "/api/v1/weeks/{id}/lessons", tag = "curriculum", params(("id" = String, Path)), request_body = CreateLessonRequest, responses((status = 201, body = Lesson)))]
pub async fn create_lesson(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<CreateLessonRequest>,
) -> AppResult<Response> {
    let w = load_week(&state, &id).await?;
    let day = req.day_number.unwrap_or(1);
    let video = req.video_url.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let meeting = req.meeting_url.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let mut v = Validator::new();
    validate_lesson_fields(
        &mut v,
        &req.title,
        &req.summary,
        &req.content_md,
        &req.lab_md,
        day,
        req.estimated_minutes,
        video.as_deref(),
        meeting.as_deref(),
    );
    v.finish()?;
    let objectives = clean_list(req.objectives, "objectives", 20)?;
    let position = curriculum::next_lesson_position(&state.pool, &w.id).await?;
    let lid = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO lessons (id, week_id, day_number, position, title, summary, content_md, lab_md, video_url, meeting_url, objectives, estimated_minutes, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&lid)
    .bind(&w.id)
    .bind(day)
    .bind(position)
    .bind(req.title.trim())
    .bind(req.summary.trim())
    .bind(&req.content_md)
    .bind(&req.lab_md)
    .bind(&video)
    .bind(&meeting)
    .bind(to_json(&objectives))
    .bind(req.estimated_minutes)
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await?;
    let l: Lesson = sqlx::query_as("SELECT * FROM lessons WHERE id = ?").bind(&lid).fetch_one(&state.pool).await?;
    Ok(created(l))
}

#[utoipa::path(post, path = "/api/v1/weeks/{id}/lessons/reorder", tag = "curriculum", params(("id" = String, Path)), request_body = ReorderLessonsRequest, responses((status = 204)))]
pub async fn reorder_lessons(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ReorderLessonsRequest>,
) -> AppResult<Response> {
    load_week(&state, &id).await?;
    let mut tx = state.pool.begin().await?;
    for (pos, lid) in req.lesson_ids.iter().enumerate() {
        let res = sqlx::query("UPDATE lessons SET position = ? WHERE id = ? AND week_id = ?")
            .bind(pos as i64)
            .bind(lid)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::field("lesson_ids", "All lessons must belong to this week"));
        }
    }
    tx.commit().await?;
    Ok(no_content())
}

#[utoipa::path(get, path = "/api/v1/lessons/{id}", tag = "curriculum", params(("id" = String, Path)), responses((status = 200, body = LessonDetail)))]
pub async fn get_lesson(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let today = state.today();
    let (mut lesson, week, course) = access::lesson_for(&state.pool, &user, &id, &today).await?;
    let completed_at: Option<String> = sqlx::query_scalar("SELECT completed_at FROM lesson_progress WHERE lesson_id = ? AND user_id = ?")
        .bind(&lesson.id)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await?;
    let mut prerequisites: Vec<LessonRef> = sqlx::query_as(
        "SELECT l.id, l.title, w.week_number, l.day_number,
                EXISTS (SELECT 1 FROM lesson_progress lp WHERE lp.lesson_id = l.id AND lp.user_id = ?) AS completed
         FROM lesson_prerequisites p JOIN lessons l ON l.id = p.prerequisite_lesson_id JOIN weeks w ON w.id = l.week_id
         WHERE p.lesson_id = ? ORDER BY w.week_number, l.day_number, l.position",
    )
    .bind(&user.id)
    .bind(&lesson.id)
    .fetch_all(&state.pool)
    .await?;
    if !user.is_admin() {
        // Unpublished prerequisites are ignored for students.
        let published: Vec<String> = sqlx::query_scalar(
            "SELECT p.prerequisite_lesson_id FROM lesson_prerequisites p JOIN lessons l ON l.id = p.prerequisite_lesson_id
             WHERE p.lesson_id = ? AND l.status = 'published'",
        )
        .bind(&lesson.id)
        .fetch_all(&state.pool)
        .await?;
        prerequisites.retain(|p| published.contains(&p.id));
    }
    let prerequisites_met = user.is_admin() || prerequisites.iter().all(|p| p.completed);
    if !prerequisites_met {
        lesson.content_md.clear();
        lesson.lab_md.clear();
        lesson.video_url = None;
    }
    let resources = if prerequisites_met { resources_for(&state, &user, None, Some(&lesson.id)).await? } else { vec![] };
    let assignments = assignment_repo::list(
        &state.pool,
        &user,
        &AssignmentFilter { lesson_id: Some(lesson.id.clone()), ..Default::default() },
        &now(),
        &today,
    )
    .await?;

    // Neighbours across the whole course, in curriculum order, limited to what the user can open.
    let order: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT l.id, l.status, w.status, w.id, w.available_from FROM lessons l JOIN weeks w ON w.id = l.week_id
         WHERE w.course_id = ? ORDER BY w.week_number, l.day_number, l.position",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    let visible: Vec<&String> = order
        .iter()
        .filter(|(_, ls, ws, _, af)| {
            user.is_admin() || (ls == "published" && ws == "published" && af.as_deref().is_none_or(|d| d <= today.as_str()))
        })
        .map(|(id, ..)| id)
        .collect();
    let pos = visible.iter().position(|l| **l == lesson.id);
    let previous_lesson_id = pos.and_then(|p| p.checked_sub(1)).map(|p| visible[p].clone());
    let next_lesson_id = pos.and_then(|p| visible.get(p + 1)).map(|s| (*s).clone());

    // Live class: the lesson's own link wins over its weekly schedule slot; the time follows from
    // the course start date and the slot of the lesson's course day.
    let sched = super::courses::load_schedule(&state, &course.id).await?;
    let slot = sched.slots.iter().find(|s| s.day_number == lesson.day_number);
    let class_meeting_url = lesson.meeting_url.clone().or_else(|| slot.and_then(|s| s.meeting_url.clone()));
    let class_times = (|| {
        let slot = slot?;
        let start = chrono::NaiveDate::parse_from_str(course.start_date.as_deref()?, "%Y-%m-%d").ok()?;
        let tz: chrono_tz::Tz = sched.timezone.parse().unwrap_or(chrono_tz::UTC);
        let date = schedule::class_date(start, week.week_number, slot.weekday as u32);
        Some(schedule::class_times(
            date,
            schedule::parse_time(&slot.start_time)?,
            slot.end_time.as_deref().and_then(schedule::parse_time),
            tz,
        ))
    })();

    Ok(json(LessonDetail {
        class_meeting_url,
        class_starts_at: class_times.map(|(s, _)| fmt_ts(s)),
        class_ends_at: class_times.and_then(|(_, e)| e).map(fmt_ts),
        course_id: course.id,
        week_number: week.week_number,
        week_title: week.title,
        week_status: week.status,
        completed: completed_at.is_some(),
        completed_at,
        prerequisites,
        prerequisites_met,
        resources,
        assignments,
        previous_lesson_id,
        next_lesson_id,
        lesson,
    }))
}

async fn load_lesson(state: &AppState, id: &str) -> AppResult<Lesson> {
    sqlx::query_as("SELECT * FROM lessons WHERE id = ?").bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Lesson"))
}

#[utoipa::path(patch, path = "/api/v1/lessons/{id}", tag = "curriculum", params(("id" = String, Path)), request_body = UpdateLessonRequest, responses((status = 200, body = Lesson)))]
pub async fn update_lesson(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<UpdateLessonRequest>,
) -> AppResult<Response> {
    let l = load_lesson(&state, &id).await?;
    let week = load_week(&state, &l.week_id).await?;
    let title = req.title.unwrap_or(l.title.clone());
    let summary = req.summary.unwrap_or(l.summary.clone());
    let content = req.content_md.unwrap_or(l.content_md.clone());
    let lab = req.lab_md.unwrap_or(l.lab_md.clone());
    let day = req.day_number.unwrap_or(l.day_number);
    let minutes = req.estimated_minutes.unwrap_or(l.estimated_minutes);
    let status = req.status.unwrap_or(l.status.clone());
    let video = match req.video_url {
        Some(v) if v.trim().is_empty() => None,
        Some(v) => Some(v.trim().to_string()),
        None => l.video_url.clone(),
    };
    let meeting = match req.meeting_url {
        Some(v) if v.trim().is_empty() => None,
        Some(v) => Some(v.trim().to_string()),
        None => l.meeting_url.clone(),
    };
    let mut v = Validator::new();
    validate_lesson_fields(&mut v, &title, &summary, &content, &lab, day, minutes, video.as_deref(), meeting.as_deref());
    v.check(matches!(status.as_str(), "draft" | "published"), "status", "Status must be draft or published").finish()?;
    let objectives = match req.objectives {
        Some(o) => clean_list(o, "objectives", 20)?,
        None => l.objectives.0.clone(),
    };
    let ts = now();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE lessons SET title = ?, summary = ?, content_md = ?, lab_md = ?, video_url = ?, meeting_url = ?, objectives = ?, day_number = ?,
         estimated_minutes = ?, status = ?, published_at = CASE WHEN ? = 'published' THEN COALESCE(published_at, ?) ELSE published_at END,
         updated_at = ? WHERE id = ?",
    )
    .bind(title.trim())
    .bind(summary.trim())
    .bind(&content)
    .bind(&lab)
    .bind(&video)
    .bind(&meeting)
    .bind(to_json(&objectives))
    .bind(day)
    .bind(minutes)
    .bind(&status)
    .bind(&status)
    .bind(&ts)
    .bind(&ts)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    if let Some(prereqs) = req.prerequisite_ids {
        sqlx::query("DELETE FROM lesson_prerequisites WHERE lesson_id = ?").bind(&id).execute(&mut *tx).await?;
        for pid in prereqs.iter().filter(|p| **p != id) {
            let same_course: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM lessons l JOIN weeks w ON w.id = l.week_id WHERE l.id = ? AND w.course_id = ?")
                    .bind(pid)
                    .bind(&week.course_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if same_course == 0 {
                return Err(AppError::field("prerequisite_ids", "Prerequisites must be lessons of the same course"));
            }
            sqlx::query("INSERT OR IGNORE INTO lesson_prerequisites (lesson_id, prerequisite_lesson_id) VALUES (?, ?)")
                .bind(&id)
                .bind(pid)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    let updated = load_lesson(&state, &id).await?;
    if status != l.status {
        audit::log(
            &state.pool,
            Some(&admin.id),
            &format!("lesson.{status}"),
            "lesson",
            Some(&id),
            serde_json::json!({ "title": updated.title }),
            ip.0.as_deref(),
        )
        .await;
        curriculum::notify_lesson_available(&state.pool, &updated, &week, &state.today()).await?;
    }
    Ok(json(updated))
}

#[utoipa::path(delete, path = "/api/v1/lessons/{id}", tag = "curriculum", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete_lesson(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let l = load_lesson(&state, &id).await?;
    sqlx::query("DELETE FROM lessons WHERE id = ?").bind(&id).execute(&state.pool).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "lesson.delete",
        "lesson",
        Some(&id),
        serde_json::json!({ "title": l.title }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/lessons/{id}/duplicate", tag = "curriculum", params(("id" = String, Path)), request_body = DuplicateLessonRequest, responses((status = 201, body = IdResponse)))]
pub async fn duplicate_lesson(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<DuplicateLessonRequest>,
) -> AppResult<Response> {
    let l = load_lesson(&state, &id).await?;
    let target = match req.target_week_id.filter(|t| !t.is_empty()) {
        Some(t) => {
            let src = load_week(&state, &l.week_id).await?;
            let dst = load_week(&state, &t).await?;
            if src.course_id != dst.course_id {
                return Err(AppError::field("target_week_id", "Target week must belong to the same course"));
            }
            t
        }
        None => l.week_id.clone(),
    };
    let new_id = curriculum::duplicate_lesson(&state.pool, &l, &target).await?;
    Ok(created(IdResponse { id: new_id }))
}

#[utoipa::path(post, path = "/api/v1/lessons/{id}/complete", tag = "curriculum", params(("id" = String, Path)), responses((status = 204)))]
pub async fn complete_lesson(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    if user.is_admin() {
        return Err(AppError::bad_request("Only students track lesson completion"));
    }
    let (lesson, ..) = access::lesson_for(&state.pool, &user, &id, &state.today()).await?;
    if !prerequisites_met(&state, &lesson.id, &user.id).await? {
        return Err(AppError::Conflict("Complete the prerequisite lessons first".into()));
    }
    sqlx::query("INSERT OR IGNORE INTO lesson_progress (id, lesson_id, user_id, completed_at) VALUES (?, ?, ?, ?)")
        .bind(new_id())
        .bind(&lesson.id)
        .bind(&user.id)
        .bind(now())
        .execute(&state.pool)
        .await?;
    Ok(no_content())
}

#[utoipa::path(delete, path = "/api/v1/lessons/{id}/complete", tag = "curriculum", params(("id" = String, Path)), responses((status = 204)))]
pub async fn uncomplete_lesson(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let (lesson, ..) = access::lesson_for(&state.pool, &user, &id, &state.today()).await?;
    sqlx::query("DELETE FROM lesson_progress WHERE lesson_id = ? AND user_id = ?")
        .bind(&lesson.id)
        .bind(&user.id)
        .execute(&state.pool)
        .await?;
    Ok(no_content())
}

/// All lessons of a course (instructor helper for prerequisites and pickers).
#[utoipa::path(get, path = "/api/v1/courses/{id}/lessons", tag = "curriculum", params(("id" = String, Path)), responses((status = 200, body = [LessonRef])))]
pub async fn course_lessons(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let rows: Vec<LessonRef> = sqlx::query_as(
        "SELECT l.id, l.title, w.week_number, l.day_number, 0 AS completed FROM lessons l JOIN weeks w ON w.id = l.week_id
         WHERE w.course_id = ? ORDER BY w.week_number, l.day_number, l.position",
    )
    .bind(&course.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(json(rows))
}
