use axum::{
    extract::{Path, State},
    response::Response,
    Extension,
};

use super::{created, json, no_content, ApiJson, IdResponse};
use crate::{
    auth::{access, AdminUser},
    errors::AppResult,
    middleware::ClientIp,
    models::group::{GroupInfo, GroupInput, ScoreOverrideInput},
    services::{audit, groups as svc},
    util::now,
    AppState,
};

#[utoipa::path(get, path = "/api/v1/assignments/{id}/groups", tag = "assignments", params(("id" = String, Path)), responses((status = 200, body = [GroupInfo])))]
pub async fn list(State(state): State<AppState>, admin: AdminUser, Path(id): Path<String>) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    Ok(json(svc::list(&state.pool, &a.id).await?))
}

#[utoipa::path(post, path = "/api/v1/assignments/{id}/groups", tag = "assignments", params(("id" = String, Path)), request_body = GroupInput, responses((status = 201, body = IdResponse)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<GroupInput>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    let name = req.name.trim().to_string();
    let gid = svc::save(&state.pool, &a, None, req).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "group.create",
        "assignment",
        Some(&a.id),
        serde_json::json!({ "group": name }),
        ip.0.as_deref(),
    )
    .await;
    Ok(created(IdResponse { id: gid }))
}

#[utoipa::path(put, path = "/api/v1/assignments/{id}/groups/{group_id}", tag = "assignments",
    params(("id" = String, Path), ("group_id" = String, Path)), request_body = GroupInput, responses((status = 200, body = GroupInfo)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path((id, group_id)): Path<(String, String)>,
    ApiJson(req): ApiJson<GroupInput>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    let name = req.name.trim().to_string();
    svc::save(&state.pool, &a, Some(&group_id), req).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "group.update",
        "assignment",
        Some(&a.id),
        serde_json::json!({ "group": name }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(svc::info(&state.pool, &group_id, true).await?))
}

#[utoipa::path(delete, path = "/api/v1/assignments/{id}/groups/{group_id}", tag = "assignments",
    params(("id" = String, Path), ("group_id" = String, Path)), responses((status = 204)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path((id, group_id)): Path<(String, String)>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    svc::delete(&state.pool, &a, &group_id).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "group.delete",
        "assignment",
        Some(&a.id),
        serde_json::json!({ "group_id": group_id }),
        ip.0.as_deref(),
    )
    .await;
    Ok(no_content())
}

#[utoipa::path(put, path = "/api/v1/assignments/{id}/groups/{group_id}/members/{student_id}/override", tag = "assignments",
    params(("id" = String, Path), ("group_id" = String, Path), ("student_id" = String, Path)), request_body = ScoreOverrideInput,
    responses((status = 200, body = GroupInfo)))]
pub async fn set_override(
    State(state): State<AppState>,
    Extension(ip): Extension<ClientIp>,
    admin: AdminUser,
    Path((id, group_id, student_id)): Path<(String, String, String)>,
    ApiJson(req): ApiJson<ScoreOverrideInput>,
) -> AppResult<Response> {
    let (a, _) = access::assignment_for(&state.pool, &admin, &id, &now(), &state.today()).await?;
    svc::set_override(&state.pool, &a, &group_id, &student_id, req.score).await?;
    audit::log(
        &state.pool,
        Some(&admin.id),
        "group.grade_override",
        "assignment",
        Some(&a.id),
        serde_json::json!({ "group_id": group_id, "student_id": student_id, "score": req.score }),
        ip.0.as_deref(),
    )
    .await;
    Ok(json(svc::info(&state.pool, &group_id, true).await?))
}
