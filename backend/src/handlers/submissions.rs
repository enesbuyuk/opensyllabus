use axum::{
    extract::{Multipart, Path, Query, State},
    response::Response,
    Extension,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::{created, json, no_content, ApiJson};
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    middleware::ClientIp,
    models::{
        assignment::Assignment,
        submission::{FileMeta, ReviewRequest, SaveDraftRequest, Submission, SubmissionDetail, SubmissionListItem},
    },
    repositories::submissions::{self as repo, SubmissionFilter},
    services::{audit, groups, storage, submissions as svc},
    util::{new_id, now},
    AppState,
};

const MAX_ATTACHMENTS: i64 = 20;

fn student_only(user: &AuthUser) -> AppResult<()> {
    if user.is_admin() {
        Err(AppError::bad_request("Instructors cannot submit assignments"))
    } else {
        Ok(())
    }
}

async fn student_assignment(state: &AppState, user: &AuthUser, id: &str) -> AppResult<Assignment> {
    student_only(user)?;
    let (a, _) = access::assignment_for(&state.pool, user, id, &now(), &state.today()).await?;
    Ok(a)
}

#[utoipa::path(get, path = "/api/v1/assignments/{id}/submission", tag = "submissions", params(("id" = String, Path)),
    responses((status = 200, body = SubmissionDetail, description = "The student's own submission (an empty working copy when not started)")))]
pub async fn my_submission(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let a = student_assignment(&state, &user, &id).await?;
    let sub = match svc::find(&state.pool, &a.id, &user.id).await? {
        Some(s) => s,
        None => svc::empty_for(&a, &user),
    };
    Ok(json(svc::detail(&state.pool, sub, &user).await?))
}

#[utoipa::path(put, path = "/api/v1/assignments/{id}/submission", tag = "submissions", params(("id" = String, Path)), request_body = SaveDraftRequest, responses((status = 200, body = Submission)))]
pub async fn save_draft(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<SaveDraftRequest>,
) -> AppResult<Response> {
    let a = student_assignment(&state, &user, &id).await?;
    Ok(json(svc::save_draft(&state.pool, &a, &user, req).await?))
}

#[utoipa::path(post, path = "/api/v1/assignments/{id}/submission/attachments", tag = "submissions", params(("id" = String, Path)),
    request_body(content_type = "multipart/form-data", description = "Field `file`"), responses((status = 201, body = FileMeta)))]
pub async fn upload_attachment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    mut mp: Multipart,
) -> AppResult<Response> {
    let a = student_assignment(&state, &user, &id).await?;
    if matches!(a.kind.as_str(), "written" | "repository") {
        return Err(AppError::bad_request("This assignment type does not accept file attachments"));
    }
    let sub = svc::editable_submission(&state.pool, &a, &user).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM submission_attachments WHERE submission_id = ?")
        .bind(&sub.id)
        .fetch_one(&state.pool)
        .await?;
    if count >= MAX_ATTACHMENTS {
        return Err(AppError::bad_request(format!("A submission can have at most {MAX_ATTACHMENTS} attachments")));
    }
    let field = mp
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("Invalid upload: {e}")))?
        .ok_or_else(|| AppError::bad_request("No file was uploaded"))?;
    let policy = storage::UploadPolicy { allowed_exts: a.allowed_file_types.0.clone(), max_bytes: a.max_upload_mb as u64 * 1024 * 1024 };
    let meta = storage::store_field(&state, field, &user.id, &policy).await?;
    sqlx::query("INSERT INTO submission_attachments (id, submission_id, file_id, created_at) VALUES (?, ?, ?, ?)")
        .bind(new_id())
        .bind(&sub.id)
        .bind(&meta.id)
        .bind(now())
        .execute(&state.pool)
        .await?;
    if sub.status == "submitted" {
        // Changing a resubmittable submission withdraws it until submitted again.
        sqlx::query("UPDATE submissions SET status = 'draft', updated_at = ? WHERE id = ?")
            .bind(now())
            .bind(&sub.id)
            .execute(&state.pool)
            .await?;
    }
    groups::sync_for_leader(&state.pool, &a, &user.id).await?;
    Ok(created(meta))
}

#[utoipa::path(delete, path = "/api/v1/assignments/{id}/submission/attachments/{file_id}", tag = "submissions",
    params(("id" = String, Path), ("file_id" = String, Path)), responses((status = 204)))]
pub async fn delete_attachment(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, file_id)): Path<(String, String)>,
) -> AppResult<Response> {
    let a = student_assignment(&state, &user, &id).await?;
    let sub = svc::editable_submission(&state.pool, &a, &user).await?;
    let res = sqlx::query("DELETE FROM submission_attachments WHERE submission_id = ? AND file_id = ?")
        .bind(&sub.id)
        .bind(&file_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Attachment"));
    }
    if sub.status == "submitted" {
        sqlx::query("UPDATE submissions SET status = 'draft', updated_at = ? WHERE id = ?")
            .bind(now())
            .bind(&sub.id)
            .execute(&state.pool)
            .await?;
    }
    groups::sync_for_leader(&state.pool, &a, &user.id).await?;
    storage::delete_if_orphaned(&state, &file_id).await?;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/assignments/{id}/submissions", tag = "submissions", params(("id" = String, Path)),
    responses((status = 200, body = Submission, description = "Finalizes the current draft as a new attempt")))]
pub async fn submit(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let a = student_assignment(&state, &user, &id).await?;
    let sub = svc::submit(&state.pool, &a, &user).await?;
    audit::log(
        &state.pool,
        Some(&user.id),
        "submission.submit",
        "submission",
        Some(&sub.id),
        serde_json::json!({ "attempt": sub.attempt, "late": sub.is_late }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(sub))
}

#[derive(Deserialize, IntoParams)]
pub struct SubmissionQuery {
    pub course_id: Option<String>,
    pub week_id: Option<String>,
    pub assignment_id: Option<String>,
    pub student_id: Option<String>,
    /// A submission status, or `pending` / `late`.
    pub status: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/submissions", tag = "submissions", params(SubmissionQuery), responses((status = 200, body = [SubmissionListItem])))]
pub async fn list(State(state): State<AppState>, _admin: AdminUser, Query(q): Query<SubmissionQuery>) -> AppResult<Response> {
    let f = SubmissionFilter {
        course_id: q.course_id,
        week_id: q.week_id,
        assignment_id: q.assignment_id,
        student_id: q.student_id,
        status: q.status,
        include_drafts: false,
    };
    Ok(json(repo::list(&state.pool, &f).await?))
}

#[utoipa::path(get, path = "/api/v1/assignments/{id}/submissions", tag = "submissions", params(("id" = String, Path)), responses((status = 200, body = [SubmissionListItem])))]
pub async fn list_for_assignment(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    let f = SubmissionFilter { assignment_id: Some(a.id), ..Default::default() };
    Ok(json(repo::list(&state.pool, &f).await?))
}

async fn load_submission(state: &AppState, user: &AuthUser, id: &str) -> AppResult<Submission> {
    let sub: Submission = sqlx::query_as("SELECT * FROM submissions WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("Submission"))?;
    // Students may only ever see their own submissions.
    if !user.is_admin() && sub.student_id != user.id {
        return Err(AppError::NotFound("Submission"));
    }
    Ok(sub)
}

#[utoipa::path(get, path = "/api/v1/submissions/{id}", tag = "submissions", params(("id" = String, Path)), responses((status = 200, body = SubmissionDetail)))]
pub async fn get(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> AppResult<Response> {
    let sub = load_submission(&state, &user, &id).await?;
    Ok(json(svc::detail(&state.pool, sub, &user).await?))
}

#[utoipa::path(post, path = "/api/v1/submissions/{id}/start-review", tag = "submissions", params(("id" = String, Path)), responses((status = 204)))]
pub async fn start_review(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let _ = &admin;
    let sub: Submission = sqlx::query_as("SELECT * FROM submissions WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("Submission"))?;
    // A group member's copy is reviewed through the leader's submission.
    let canonical = sub.mirror_of.as_deref().unwrap_or(&sub.id);
    svc::mark_under_review(&state.pool, canonical).await?;
    let (a, leader): (Assignment, String) = (
        sqlx::query_as("SELECT * FROM assignments WHERE id = ?").bind(&sub.assignment_id).fetch_one(&state.pool).await?,
        sqlx::query_scalar("SELECT student_id FROM submissions WHERE id = ?").bind(canonical).fetch_one(&state.pool).await?,
    );
    groups::sync_for_leader(&state.pool, &a, &leader).await?;
    Ok(no_content())
}

#[utoipa::path(post, path = "/api/v1/submissions/{id}/review", tag = "submissions", params(("id" = String, Path)), request_body = ReviewRequest, responses((status = 200, body = SubmissionDetail)))]
pub async fn review(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<ReviewRequest>,
) -> AppResult<Response> {
    let sub: Submission = sqlx::query_as("SELECT * FROM submissions WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("Submission"))?;
    if sub.mirror_of.is_some() {
        return Err(AppError::Conflict("This is a copy of a group submission; review the group leader's submission instead".into()));
    }
    let a: Assignment = sqlx::query_as("SELECT * FROM assignments WHERE id = ?").bind(&sub.assignment_id).fetch_one(&state.pool).await?;
    let action = req.action.clone();
    svc::review(&state.pool, &sub, &a, &admin, req).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        &format!("submission.review.{action}"),
        "submission",
        Some(&id),
        serde_json::json!({}),
        ip.0.as_deref(),
    )
    .await;
    let sub: Submission = sqlx::query_as("SELECT * FROM submissions WHERE id = ?").bind(&id).fetch_one(&state.pool).await?;
    Ok(json(svc::detail(&state.pool, sub, &admin).await?))
}
