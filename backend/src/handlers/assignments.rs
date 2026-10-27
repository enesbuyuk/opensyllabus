use axum::{
    extract::{Path, Query, State},
    response::Response,
    Extension,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, no_content, ApiJson, IdResponse};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::assignment::{Assignment, AssignmentDetail, AssignmentInput, AssignmentListItem, RubricCriterion, KINDS, LATE_POLICIES},
    repositories::assignments::{self as repo, AssignmentFilter},
    services::{audit, background, groups, storage, submissions},
    util::{new_id, now, parse_opt_ts, to_json, Validator},
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct AssignmentQuery {
    pub course_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    /// Instructor only: draft / published / archived.
    pub status: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/assignments", tag = "assignments", params(AssignmentQuery), responses((status = 200, body = [AssignmentListItem])))]
pub async fn list(State(state): State<AppState>, user: AuthUser, Query(q): Query<AssignmentQuery>) -> AppResult<Response> {
    let f = AssignmentFilter { course_id: q.course_id, week_id: q.week_id, lesson_id: q.lesson_id, status: q.status, ids: None };
    Ok(json(repo::list(&state.pool, &user, &f, &now(), &state.today()).await?))
}

#[utoipa::path(get, path = "/api/v1/assignments/{id}", tag = "assignments", params(("id" = String, Path)), responses((status = 200, body = AssignmentDetail)))]
pub async fn get(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let ts = now();
    let (a, _) = access::assignment_for(&state.pool, &user, &id, &ts, &state.today()).await?;
    let f = AssignmentFilter { ids: Some(vec![a.id.clone()]), ..Default::default() };
    let item = repo::list(&state.pool, &user, &f, &ts, &state.today()).await?.into_iter().next().ok_or(AppError::NotFound("Assignment"))?;
    let rubric = repo::rubric(&state.pool, &a.id).await?;
    let sub = if user.is_admin() { None } else { submissions::find(&state.pool, &a.id, &user.id).await? };
    let accepts = if user.is_admin() {
        a.status == "published"
    } else {
        submissions::can_edit(&a, sub.as_ref(), &ts) && groups::ensure_can_submit(&state.pool, &a, &user.id).await.is_ok()
    };
    Ok(json(AssignmentDetail { is_past_due: submissions::is_past_due(&a, &ts), accepts_submissions: accepts, item, rubric }))
}

struct Resolved {
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
    allowed_file_types: Vec<String>,
    max_upload_mb: i64,
    allow_resubmission: bool,
    is_group: bool,
    status: String,
}

async fn resolve(state: &AppState, req: AssignmentInput, base: Option<&Assignment>) -> AppResult<(Resolved, Option<Vec<RubricCriterion>>)> {
    fn pick<T: Clone>(v: Option<T>, base: Option<T>, default: T) -> T {
        v.or(base).unwrap_or(default)
    }
    let b = base;
    let course_id = match (&req.course_id, b) {
        (_, Some(b)) => b.course_id.clone(), // Course cannot change after creation.
        (Some(c), None) => c.clone(),
        (None, None) => return Err(AppError::field("course_id", "Select a course")),
    };
    let opt = |v: Option<String>, base: Option<Option<String>>| -> Option<String> {
        match v {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s),
            None => base.flatten(),
        }
    };
    let week_id = opt(req.week_id, b.map(|b| b.week_id.clone()));
    let lesson_id = opt(req.lesson_id, b.map(|b| b.lesson_id.clone()));
    let due_at = match req.due_at {
        Some(v) => parse_opt_ts("due_at", Some(&v))?,
        None => b.and_then(|b| b.due_at.clone()),
    };
    let publish_at = match req.publish_at {
        Some(v) => parse_opt_ts("publish_at", Some(&v))?,
        None => b.and_then(|b| b.publish_at.clone()),
    };
    let r = Resolved {
        course_id,
        week_id,
        lesson_id,
        title: pick(req.title, b.map(|b| b.title.clone()), String::new()).trim().to_string(),
        description: pick(req.description, b.map(|b| b.description.clone()), String::new()),
        instructions_md: pick(req.instructions_md, b.map(|b| b.instructions_md.clone()), String::new()),
        reference_md: pick(req.reference_md, b.map(|b| b.reference_md.clone()), String::new()),
        kind: pick(req.kind, b.map(|b| b.kind.clone()), "written".into()),
        max_grade: pick(req.max_grade, b.map(|b| b.max_grade), 100.0),
        due_at,
        publish_at,
        late_policy: pick(req.late_policy, b.map(|b| b.late_policy.clone()), "accept".into()),
        late_penalty_percent: pick(req.late_penalty_percent, b.map(|b| b.late_penalty_percent), 0),
        allowed_file_types: pick(req.allowed_file_types, b.map(|b| b.allowed_file_types.0.clone()), vec![])
            .iter()
            .map(|e| storage::normalize_ext(e))
            .filter(|e| !e.is_empty())
            .collect(),
        max_upload_mb: pick(req.max_upload_mb, b.map(|b| b.max_upload_mb), 20),
        allow_resubmission: pick(req.allow_resubmission, b.map(|b| b.allow_resubmission), false),
        is_group: pick(req.is_group, b.map(|b| b.is_group), false),
        status: pick(req.status, b.map(|b| b.status.clone()), "draft".into()),
    };
    let supported = storage::supported_extensions();
    Validator::new()
        .len(&r.title, "title", 3, 200)
        .len(&r.description, "description", 0, 2000)
        .len(&r.instructions_md, "instructions_md", 0, 100_000)
        .len(&r.reference_md, "reference_md", 0, 50_000)
        .check(KINDS.contains(&r.kind.as_str()), "kind", "Unknown assignment type")
        .check(LATE_POLICIES.contains(&r.late_policy.as_str()), "late_policy", "Unknown late policy")
        .check(r.max_grade > 0.0 && r.max_grade <= 1000.0, "max_grade", "Maximum grade must be between 0 and 1000")
        .check((0..=100).contains(&r.late_penalty_percent), "late_penalty_percent", "Penalty must be between 0 and 100")
        .check((1..=100).contains(&r.max_upload_mb), "max_upload_mb", "Upload limit must be between 1 and 100 MB")
        .check(
            r.allowed_file_types.iter().all(|e| supported.contains(&e.as_str())),
            "allowed_file_types",
            "Contains an unsupported file type",
        )
        .check(matches!(r.status.as_str(), "draft" | "published" | "archived"), "status", "Unknown status")
        .check(
            match (&r.publish_at, &r.due_at) {
                (Some(p), Some(d)) => p < d,
                _ => true,
            },
            "due_at",
            "Due date must be after the publication date",
        )
        .finish()?;
    if let Some(w) = &r.week_id {
        let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks WHERE id = ? AND course_id = ?")
            .bind(w)
            .bind(&r.course_id)
            .fetch_one(&state.pool)
            .await?;
        if ok == 0 {
            return Err(AppError::field("week_id", "Week does not belong to this course"));
        }
    }
    if let Some(l) = &r.lesson_id {
        let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lessons WHERE id = ? AND week_id IS ?")
            .bind(l)
            .bind(&r.week_id)
            .fetch_one(&state.pool)
            .await?;
        if ok == 0 {
            return Err(AppError::field("lesson_id", "Lesson must belong to the selected week"));
        }
    }
    let rubric = match req.rubric {
        Some(items) => {
            if items.len() > 30 {
                return Err(AppError::field("rubric", "At most 30 rubric criteria are allowed"));
            }
            for c in &items {
                if c.title.trim().is_empty()
                    || c.title.chars().count() > 200
                    || c.max_points.is_nan()
                    || c.max_points <= 0.0
                    || c.description.chars().count() > 2000
                {
                    return Err(AppError::field("rubric", "Each criterion needs a title and positive points"));
                }
            }
            let total: f64 = items.iter().map(|c| c.max_points).sum();
            if !items.is_empty() && (total - r.max_grade).abs() > 0.001 {
                return Err(AppError::field(
                    "rubric",
                    &format!("Rubric points ({total}) must add up to the maximum grade ({})", r.max_grade),
                ));
            }
            Some(items)
        }
        None => None,
    };
    Ok((r, rubric))
}

async fn write_rubric(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, assignment_id: &str, items: &[RubricCriterion]) -> AppResult<()> {
    let keep: Vec<&str> = items.iter().map(|c| c.id.as_str()).filter(|id| !id.is_empty()).collect();
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT id FROM assignment_rubrics WHERE assignment_id = ?").bind(assignment_id).fetch_all(&mut **tx).await?;
    for id in existing.iter().filter(|id| !keep.contains(&id.as_str())) {
        sqlx::query("DELETE FROM assignment_rubrics WHERE id = ?").bind(id).execute(&mut **tx).await?;
    }
    for (pos, c) in items.iter().enumerate() {
        if !c.id.is_empty() && existing.contains(&c.id) {
            sqlx::query("UPDATE assignment_rubrics SET position = ?, title = ?, description = ?, max_points = ? WHERE id = ?")
                .bind(pos as i64)
                .bind(c.title.trim())
                .bind(c.description.trim())
                .bind(c.max_points)
                .bind(&c.id)
                .execute(&mut **tx)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO assignment_rubrics (id, assignment_id, position, title, description, max_points) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id())
            .bind(assignment_id)
            .bind(pos as i64)
            .bind(c.title.trim())
            .bind(c.description.trim())
            .bind(c.max_points)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

#[utoipa::path(post, path = "/api/v1/assignments", tag = "assignments", request_body = AssignmentInput, responses((status = 201, body = IdResponse)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    ApiJson(req): ApiJson<AssignmentInput>,
) -> AppResult<Response> {
    let (r, rubric) = resolve(&state, req, None).await?;
    access::course_for(&state.pool, &admin, &r.course_id).await?;
    let id = new_id();
    let ts = now();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO assignments (id, course_id, week_id, lesson_id, title, description, instructions_md, reference_md, kind, max_grade, due_at, publish_at,
            late_policy, late_penalty_percent, allowed_file_types, max_upload_mb, allow_resubmission, is_group, status, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&r.course_id)
    .bind(&r.week_id)
    .bind(&r.lesson_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.instructions_md)
    .bind(&r.reference_md)
    .bind(&r.kind)
    .bind(r.max_grade)
    .bind(&r.due_at)
    .bind(&r.publish_at)
    .bind(&r.late_policy)
    .bind(r.late_penalty_percent)
    .bind(to_json(&r.allowed_file_types))
    .bind(r.max_upload_mb)
    .bind(r.allow_resubmission)
    .bind(r.is_group)
    .bind(&r.status)
    .bind(&admin.id)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;
    if let Some(items) = &rubric {
        write_rubric(&mut tx, &id, items).await?;
    }
    tx.commit().await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "assignment.create",
        "assignment",
        Some(&id),
        serde_json::json!({ "title": r.title, "status": r.status }),
        ip.0.as_deref(),
    )
    .await;
    if r.status == "published" {
        background::notify_released_assignments(&state).await.map_err(AppError::Internal)?;
    }
    Ok(created(IdResponse { id }))
}

#[utoipa::path(patch, path = "/api/v1/assignments/{id}", tag = "assignments", params(("id" = String, Path)), request_body = AssignmentInput, responses((status = 200, body = Assignment)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<AssignmentInput>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    let rubric_given = req.rubric.is_some();
    let (r, rubric) = resolve(&state, req, Some(&a)).await?;
    if !rubric_given {
        // Max grade changes must stay consistent with an existing rubric.
        let existing = repo::rubric(&state.pool, &a.id).await?;
        let total: f64 = existing.iter().map(|c| c.max_points).sum();
        if !existing.is_empty() && (total - r.max_grade).abs() > 0.001 {
            return Err(AppError::field("max_grade", "Maximum grade must match the rubric total"));
        }
    }
    if a.is_group && !r.is_group {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM assignment_groups WHERE assignment_id = ?").bind(&a.id).fetch_one(&state.pool).await?;
        if n > 0 {
            return Err(AppError::field("is_group", "Delete this assignment's groups before turning off group submission"));
        }
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE assignments SET week_id = ?, lesson_id = ?, title = ?, description = ?, instructions_md = ?, reference_md = ?, kind = ?,
            max_grade = ?, due_at = ?, publish_at = ?, late_policy = ?, late_penalty_percent = ?, allowed_file_types = ?, max_upload_mb = ?,
            allow_resubmission = ?, is_group = ?, status = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&r.week_id)
    .bind(&r.lesson_id)
    .bind(&r.title)
    .bind(r.description.trim())
    .bind(&r.instructions_md)
    .bind(&r.reference_md)
    .bind(&r.kind)
    .bind(r.max_grade)
    .bind(&r.due_at)
    .bind(&r.publish_at)
    .bind(&r.late_policy)
    .bind(r.late_penalty_percent)
    .bind(to_json(&r.allowed_file_types))
    .bind(r.max_upload_mb)
    .bind(r.allow_resubmission)
    .bind(r.is_group)
    .bind(&r.status)
    .bind(now())
    .bind(&a.id)
    .execute(&mut *tx)
    .await?;
    if let Some(items) = &rubric {
        write_rubric(&mut tx, &a.id, items).await?;
    }
    tx.commit().await?;
    if r.status != a.status {
        audit::log(
            &state.pool,
            Some(&admin.id),
            &format!("assignment.{}", r.status),
            "assignment",
            Some(&a.id),
            serde_json::json!({ "title": r.title }),
            ip.0.as_deref(),
        )
        .await;
    }
    if r.status == "published" {
        background::notify_released_assignments(&state).await.map_err(AppError::Internal)?;
    }
    let updated: Assignment = sqlx::query_as("SELECT * FROM assignments WHERE id = ?").bind(&a.id).fetch_one(&state.pool).await?;
    Ok(json(updated))
}

#[utoipa::path(delete, path = "/api/v1/assignments/{id}", tag = "assignments", params(("id" = String, Path)), responses((status = 204), (status = 409)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    let subs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM submissions WHERE assignment_id = ?").bind(&a.id).fetch_one(&state.pool).await?;
    if subs > 0 {
        return Err(AppError::Conflict("This assignment has student work; archive it instead of deleting".into()));
    }
    sqlx::query("DELETE FROM assignments WHERE id = ?").bind(&a.id).execute(&state.pool).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "assignment.delete",
        "assignment",
        Some(&a.id),
        serde_json::json!({ "title": a.title }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}
