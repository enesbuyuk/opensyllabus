use axum::{
    extract::{Multipart, Path, State},
    response::Response,
    Extension,
};

use super::{created, json, no_content, ApiJson, IdResponse};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        course::{
            Course, CourseListItem, CourseSchedule, CreateCourseRequest, EnrollRequest, EnrollmentItem, ScheduleSlot, UpdateCourseRequest,
        },
        user::User,
    },
    services::{audit, curriculum, progress, schedule, storage},
    util::{clean_list, is_valid_timezone, new_id, now, parse_opt_date, slugify, to_json, Validator},
    AppState,
};

const DIFFICULTIES: &[&str] = &["beginner", "intermediate", "advanced"];
const STATUSES: &[&str] = &["draft", "published", "archived"];

async fn list_item(state: &AppState, user: &User, course: Course) -> AppResult<CourseListItem> {
    let (instructor_name, enrolled_count, published_weeks, total_weeks): (Option<String>, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT name FROM users WHERE id = ?1),
                (SELECT COUNT(*) FROM course_enrollments e JOIN users u ON u.id = e.user_id WHERE e.course_id = ?2 AND u.role = 'student'),
                (SELECT COUNT(*) FROM weeks WHERE course_id = ?2 AND status = 'published'),
                (SELECT COUNT(*) FROM weeks WHERE course_id = ?2)",
    )
    .bind(&course.instructor_id)
    .bind(&course.id)
    .fetch_one(&state.pool)
    .await?;
    let (completion_percent, current_week) = if user.is_admin() {
        (None, progress::calendar_week(&course, &state.today()))
    } else {
        let data = progress::CourseData::load(&state.pool, course.clone()).await?;
        let p = data.student_progress(&state.pool, &user.id, &state.today(), &now()).await?;
        (Some(p.completion_percent), p.current_week)
    };
    Ok(CourseListItem { course, instructor_name, enrolled_count, published_weeks, total_weeks, completion_percent, current_week })
}

#[utoipa::path(get, path = "/api/v1/courses", tag = "courses", responses((status = 200, body = [CourseListItem])))]
pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    let ids = access::visible_course_ids(&state.pool, &user).await?;
    let mut items = Vec::new();
    for id in ids {
        let c: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ?").bind(&id).fetch_one(&state.pool).await?;
        items.push(list_item(&state, &user, c).await?);
    }
    Ok(json(items))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}", tag = "courses", params(("id" = String, Path, description = "Course id or slug")), responses((status = 200, body = CourseListItem)))]
pub async fn get(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &user, &id).await?;
    Ok(json(list_item(&state, &user, course).await?))
}

fn validate_common(v: &mut Validator, title: &str, slug: &str, description: &str, difficulty: &str, duration: i64, days: i64) {
    v.len(title, "title", 3, 200)
        .check(!slug.is_empty() && slug.len() <= 80 && slug == slugify(slug), "slug", "Use lowercase letters, numbers and hyphens")
        .len(description, "description", 0, 20_000)
        .check(DIFFICULTIES.contains(&difficulty), "difficulty", "Choose beginner, intermediate or advanced")
        .check((1..=104).contains(&duration), "duration_weeks", "Duration must be between 1 and 104 weeks")
        .check((1..=7).contains(&days), "days_per_week", "Days per week must be between 1 and 7");
}

#[utoipa::path(post, path = "/api/v1/courses", tag = "courses", request_body = CreateCourseRequest, responses((status = 201, body = IdResponse)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<CreateCourseRequest>,
) -> AppResult<Response> {
    let title = req.title.trim().to_string();
    let slug = req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| slugify(&title));
    let difficulty = req.difficulty.unwrap_or_else(|| "intermediate".into());
    let duration = req.duration_weeks.unwrap_or(12);
    let days = req.days_per_week.unwrap_or(3);
    let mut v = Validator::new();
    validate_common(&mut v, &title, &slug, &req.description, &difficulty, duration, days);
    v.finish()?;
    let start = parse_opt_date("start_date", req.start_date.as_deref())?;
    let end = parse_opt_date("end_date", req.end_date.as_deref())?;
    if let (Some(s), Some(e)) = (&start, &end) {
        if e < s {
            return Err(AppError::field("end_date", "End date must be after the start date"));
        }
    }
    let objectives = clean_list(req.objectives, "objectives", 30)?;
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO courses (id, title, slug, description, difficulty, duration_weeks, days_per_week, instructor_id, status, start_date, end_date, objectives, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(&slug)
    .bind(req.description.trim())
    .bind(&difficulty)
    .bind(duration)
    .bind(days)
    .bind(&admin.id)
    .bind(&start)
    .bind(&end)
    .bind(to_json(&objectives))
    .bind(&ts)
    .bind(&ts)
    .execute(&state.pool)
    .await
    .map_err(|e| match AppError::from(e) {
        AppError::Conflict(_) => AppError::field("slug", "This slug is already used by another course"),
        other => other,
    })?;
    if req.generate_weeks {
        let course: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ?").bind(&id).fetch_one(&state.pool).await?;
        curriculum::generate_weeks(&state.pool, &course, true).await?;
    }
    audit::log(&state.pool, Some(&admin.id), "course.create", "course", Some(&id), serde_json::json!({ "title": title }), ip.0.as_deref())
        .await;
    Ok(created(IdResponse { id }))
}

#[utoipa::path(patch, path = "/api/v1/courses/{id}", tag = "courses", params(("id" = String, Path)), request_body = UpdateCourseRequest, responses((status = 200, body = Course)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<UpdateCourseRequest>,
) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    let title = req.title.map(|s| s.trim().to_string()).unwrap_or(c.title.clone());
    let slug = req.slug.map(|s| s.trim().to_string()).unwrap_or(c.slug.clone());
    let description = req.description.unwrap_or(c.description.clone());
    let difficulty = req.difficulty.unwrap_or(c.difficulty.clone());
    let duration = req.duration_weeks.unwrap_or(c.duration_weeks);
    let days = req.days_per_week.unwrap_or(c.days_per_week);
    let status = req.status.unwrap_or(c.status.clone());
    let mut v = Validator::new();
    validate_common(&mut v, &title, &slug, &description, &difficulty, duration, days);
    v.check(STATUSES.contains(&status.as_str()), "status", "Unknown status").finish()?;
    let start = match req.start_date {
        Some(s) => parse_opt_date("start_date", Some(&s))?,
        None => c.start_date.clone(),
    };
    let end = match req.end_date {
        Some(s) => parse_opt_date("end_date", Some(&s))?,
        None => c.end_date.clone(),
    };
    if let (Some(s), Some(e)) = (&start, &end) {
        if e < s {
            return Err(AppError::field("end_date", "End date must be after the start date"));
        }
    }
    let objectives = match req.objectives {
        Some(o) => clean_list(o, "objectives", 30)?,
        None => c.objectives.0.clone(),
    };
    let instructor_id = match req.instructor_id {
        Some(iid) => {
            let ok: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ? AND role = 'admin'").bind(&iid).fetch_one(&state.pool).await?;
            if ok == 0 {
                return Err(AppError::field("instructor_id", "Instructor must be an administrator account"));
            }
            Some(iid)
        }
        None => c.instructor_id.clone(),
    };
    sqlx::query(
        "UPDATE courses SET title = ?, slug = ?, description = ?, difficulty = ?, duration_weeks = ?, days_per_week = ?, status = ?,
         start_date = ?, end_date = ?, objectives = ?, instructor_id = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&title)
    .bind(&slug)
    .bind(description.trim())
    .bind(&difficulty)
    .bind(duration)
    .bind(days)
    .bind(&status)
    .bind(&start)
    .bind(&end)
    .bind(to_json(&objectives))
    .bind(&instructor_id)
    .bind(now())
    .bind(&c.id)
    .execute(&state.pool)
    .await
    .map_err(|e| match AppError::from(e) {
        AppError::Conflict(_) => AppError::field("slug", "This slug is already used by another course"),
        other => other,
    })?;
    if status != c.status {
        audit::log(
            &state.pool,
            Some(&admin.id),
            &format!("course.{status}"),
            "course",
            Some(&c.id),
            serde_json::json!({}),
            ip.0.as_deref(),
        )
        .await;
    } else {
        audit::log(&state.pool, Some(&admin.id), "course.update", "course", Some(&c.id), serde_json::json!({}), ip.0.as_deref()).await;
    }
    let course: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ?").bind(&c.id).fetch_one(&state.pool).await?;
    Ok(json(course))
}

#[utoipa::path(delete, path = "/api/v1/courses/{id}", tag = "courses", params(("id" = String, Path)), responses((status = 204), (status = 409)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    let subs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM submissions s JOIN assignments a ON a.id = s.assignment_id WHERE a.course_id = ?")
            .bind(&c.id)
            .fetch_one(&state.pool)
            .await?;
    if subs > 0 || c.status != "draft" {
        return Err(AppError::Conflict("Only draft courses without submissions can be deleted; archive it instead".into()));
    }
    sqlx::query("DELETE FROM courses WHERE id = ?").bind(&c.id).execute(&state.pool).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "course.delete",
        "course",
        Some(&c.id),
        serde_json::json!({ "title": c.title }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/courses/{id}/cover", tag = "courses", params(("id" = String, Path)), responses((status = 200, body = Course)))]
pub async fn upload_cover(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    mut mp: Multipart,
) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    let field = mp
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("Invalid upload: {e}")))?
        .ok_or_else(|| AppError::bad_request("No file was uploaded"))?;
    let policy =
        storage::UploadPolicy { allowed_exts: ["png", "jpg", "jpeg", "webp"].map(String::from).to_vec(), max_bytes: 5 * 1024 * 1024 };
    let meta = storage::store_field(&state, field, &admin.id, &policy).await?;
    sqlx::query("UPDATE courses SET cover_file_id = ?, updated_at = ? WHERE id = ?")
        .bind(&meta.id)
        .bind(now())
        .bind(&c.id)
        .execute(&state.pool)
        .await?;
    if let Some(old) = &c.cover_file_id {
        storage::delete_if_orphaned(&state, old).await?;
    }
    let course: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ?").bind(&c.id).fetch_one(&state.pool).await?;
    Ok(json(course))
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/enrollments", tag = "courses", params(("id" = String, Path)), responses((status = 200, body = [EnrollmentItem])))]
pub async fn enrollments(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    let rows: Vec<EnrollmentItem> = sqlx::query_as(
        "SELECT u.id AS user_id, u.name, u.email, u.is_active, e.enrolled_at FROM course_enrollments e
         JOIN users u ON u.id = e.user_id WHERE e.course_id = ? AND u.role = 'student' ORDER BY u.name COLLATE NOCASE",
    )
    .bind(&c.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(json(rows))
}

#[utoipa::path(post, path = "/api/v1/courses/{id}/enrollments", tag = "courses", params(("id" = String, Path)), request_body = EnrollRequest, responses((status = 204)))]
pub async fn enroll(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<EnrollRequest>,
) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    let ts = now();
    for uid in &req.user_ids {
        let is_student: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ? AND role = 'student'").bind(uid).fetch_one(&state.pool).await?;
        if is_student == 0 {
            return Err(AppError::field("user_ids", "Only student accounts can be enrolled"));
        }
        sqlx::query("INSERT OR IGNORE INTO course_enrollments (id, course_id, user_id, enrolled_at) VALUES (?, ?, ?, ?)")
            .bind(new_id())
            .bind(&c.id)
            .bind(uid)
            .bind(&ts)
            .execute(&state.pool)
            .await?;
    }
    audit::log(
        &state.pool,
        Some(&admin.id),
        "course.enroll",
        "course",
        Some(&c.id),
        serde_json::json!({ "user_ids": req.user_ids }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

#[utoipa::path(delete, path = "/api/v1/courses/{id}/enrollments/{user_id}", tag = "courses", params(("id" = String, Path), ("user_id" = String, Path)), responses((status = 204)))]
pub async fn unenroll(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path((id, user_id)): Path<(String, String)>,
) -> AppResult<Response> {
    let c = access::course_for(&state.pool, &admin, &id).await?;
    sqlx::query("DELETE FROM course_enrollments WHERE course_id = ? AND user_id = ?")
        .bind(&c.id)
        .bind(&user_id)
        .execute(&state.pool)
        .await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "course.unenroll",
        "course",
        Some(&c.id),
        serde_json::json!({ "user_id": user_id }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

/// The course's class schedule, ordered by course day. Empty slots use the platform timezone.
pub async fn load_schedule(state: &AppState, course_id: &str) -> AppResult<CourseSchedule> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT timezone FROM course_schedule WHERE course_id = ? LIMIT 1").bind(course_id).fetch_all(&state.pool).await?;
    let slots: Vec<ScheduleSlot> = sqlx::query_as(
        "SELECT day_number, weekday, start_time, end_time, location FROM course_schedule WHERE course_id = ? ORDER BY day_number",
    )
    .bind(course_id)
    .fetch_all(&state.pool)
    .await?;
    let timezone = rows.into_iter().next().map(|r| r.0).unwrap_or_else(|| state.timezone().name().to_string());
    Ok(CourseSchedule { timezone, slots })
}

#[utoipa::path(get, path = "/api/v1/courses/{id}/schedule", tag = "courses", params(("id" = String, Path)), responses((status = 200, body = CourseSchedule)))]
pub async fn schedule(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    access::course_for(&state.pool, &user, &id).await?;
    Ok(json(load_schedule(&state, &id).await?))
}

/// Replaces the whole schedule; an empty `slots` list removes it.
#[utoipa::path(put, path = "/api/v1/courses/{id}/schedule", tag = "courses", params(("id" = String, Path)), request_body = CourseSchedule, responses((status = 200, body = CourseSchedule)))]
pub async fn update_schedule(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<CourseSchedule>,
) -> AppResult<Response> {
    let course = access::course_for(&state.pool, &admin, &id).await?;
    let mut v = Validator::new();
    v.check(is_valid_timezone(&req.timezone), "timezone", "Unknown timezone");
    let mut days = std::collections::HashSet::new();
    for s in &req.slots {
        let start = schedule::parse_time(&s.start_time);
        let end = s.end_time.as_deref().filter(|e| !e.is_empty()).map(schedule::parse_time);
        let field = format!("slots.{}", s.day_number);
        v.check(
            (1..=course.days_per_week).contains(&s.day_number),
            &field,
            &format!("Course day must be between 1 and {}", course.days_per_week),
        )
        .check(days.insert(s.day_number), &field, "Each course day can only be scheduled once")
        .check((1..=7).contains(&s.weekday), &field, "Choose a weekday")
        .check(start.is_some(), &field, "Start time must be HH:MM")
        .check(end.is_none_or(|e| e.is_some_and(|e| start.is_none_or(|s| e > s))), &field, "End time must be HH:MM and after the start")
        .len(&s.location, &field, 0, 200);
    }
    v.finish()?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM course_schedule WHERE course_id = ?").bind(&id).execute(&mut *tx).await?;
    for s in &req.slots {
        sqlx::query(
            "INSERT INTO course_schedule (course_id, day_number, weekday, start_time, end_time, location, timezone) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(s.day_number)
        .bind(s.weekday)
        .bind(&s.start_time)
        .bind(s.end_time.as_deref().filter(|e| !e.is_empty()))
        .bind(s.location.trim())
        .bind(&req.timezone)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "course.schedule",
        "course",
        Some(&id),
        serde_json::json!({ "slots": req.slots.len() }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(load_schedule(&state, &id).await?))
}
