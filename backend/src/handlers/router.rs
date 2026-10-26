//! Route table and middleware stack.

use std::time::Duration;

use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header, HeaderValue},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::*;
use crate::{
    errors::AppError,
    middleware::{security_headers::security_headers, session::session_middleware},
    openapi::ApiDoc,
    AppState,
};

fn api_routes(state: &AppState) -> Router<AppState> {
    let upload_limit = DefaultBodyLimit::max((state.config.max_upload_mb as usize + 1) * 1024 * 1024);
    Router::new()
        // Authentication
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/auth/change-password", post(auth::change_password))
        .route("/auth/reset-password", post(auth::reset_password))
        .route("/auth/sessions", get(auth::list_sessions))
        .route("/auth/sessions/revoke-others", post(auth::revoke_other_sessions))
        .route("/auth/sessions/{id}", delete(auth::revoke_session))
        // Profile & users
        .route("/me", axum::routing::patch(users::update_profile))
        .route("/me/progress", get(progress::my_progress))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", get(users::get).patch(users::update))
        .route("/users/{id}/reset-link", post(users::reset_link))
        .route("/users/{id}/revoke-sessions", post(users::revoke_sessions))
        // Courses
        .route("/courses", get(courses::list).post(courses::create))
        .route("/courses/{id}", get(courses::get).patch(courses::update).delete(courses::delete))
        .route("/courses/{id}/cover", post(courses::upload_cover).layer(upload_limit))
        .route("/courses/{id}/enrollments", get(courses::enrollments).post(courses::enroll))
        .route("/courses/{id}/enrollments/{user_id}", delete(courses::unenroll))
        .route("/courses/{id}/modules", post(curriculum::create_module))
        .route("/courses/{id}/weeks", get(curriculum::course_curriculum).post(curriculum::create_week))
        .route("/courses/{id}/weeks/generate", post(curriculum::generate_weeks))
        .route("/courses/{id}/lessons", get(curriculum::course_lessons))
        .route("/courses/{id}/progress", get(progress::course_progress))
        .route("/courses/{id}/students/{student_id}/progress", get(progress::student_progress))
        .route("/courses/{id}/gradebook", get(progress::gradebook))
        .route("/courses/{id}/gradebook.csv", get(progress::gradebook_csv))
        // Curriculum
        .route("/modules/{id}", axum::routing::patch(curriculum::update_module).delete(curriculum::delete_module))
        .route("/weeks/{id}", get(curriculum::get_week).patch(curriculum::update_week).delete(curriculum::delete_week))
        .route("/weeks/{id}/duplicate", post(curriculum::duplicate_week))
        .route("/weeks/{id}/lessons", post(curriculum::create_lesson))
        .route("/weeks/{id}/lessons/reorder", post(curriculum::reorder_lessons))
        .route("/lessons/{id}", get(curriculum::get_lesson).patch(curriculum::update_lesson).delete(curriculum::delete_lesson))
        .route("/lessons/{id}/duplicate", post(curriculum::duplicate_lesson))
        .route("/lessons/{id}/complete", post(curriculum::complete_lesson).delete(curriculum::uncomplete_lesson))
        // Assignments & submissions
        .route("/assignments", get(assignments::list).post(assignments::create))
        .route("/assignments/{id}", get(assignments::get).patch(assignments::update).delete(assignments::delete))
        .route("/assignments/{id}/submission", get(submissions::my_submission).put(submissions::save_draft))
        .route("/assignments/{id}/submission/attachments", post(submissions::upload_attachment).layer(upload_limit))
        .route("/assignments/{id}/submission/attachments/{file_id}", delete(submissions::delete_attachment))
        .route("/assignments/{id}/submissions", get(submissions::list_for_assignment).post(submissions::submit))
        .route("/submissions", get(submissions::list))
        .route("/submissions/{id}", get(submissions::get))
        .route("/submissions/{id}/start-review", post(submissions::start_review))
        .route("/submissions/{id}/review", post(submissions::review))
        // Files & resources
        .route("/files", post(files::upload).layer(upload_limit))
        .route("/files/{id}", get(files::download))
        .route("/resources", get(resources::list).post(resources::create))
        .route("/resources/{id}", get(resources::get).patch(resources::update).delete(resources::delete))
        // Communication
        .route("/announcements", get(announcements::list).post(announcements::create))
        .route("/announcements/{id}", axum::routing::patch(announcements::update).delete(announcements::delete))
        .route("/notifications", get(notifications::list))
        .route("/notifications/read-all", post(notifications::mark_all_read))
        .route("/notifications/{id}/read", post(notifications::mark_read))
        .route("/calendar", get(calendar::entries))
        .route("/calendar/events", post(calendar::create))
        .route("/calendar/events/{id}", axum::routing::patch(calendar::update).delete(calendar::delete))
        .route("/calendar/events/{id}/skip", post(calendar::skip_occurrence))
        // Overview
        .route("/dashboard", get(dashboard::dashboard))
        .route("/analytics", get(analytics::analytics))
        .route("/search", get(search::search))
        .route("/settings", get(settings::get).patch(settings::update))
        .route("/audit-logs", get(settings::audit_logs))
        // JSON bodies are small; uploads override this per route.
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
}

async fn api_not_found() -> AppError {
    AppError::NotFound("Endpoint")
}

/// Long-lived caching for fingerprinted assets, none for the HTML shell.
async fn static_cache_headers(req: Request, next: Next) -> Response {
    let is_asset = req.uri().path().starts_with("/assets/");
    let is_api = req.uri().path().starts_with("/api/");
    let mut res = next.run(req).await;
    if !is_api {
        let value = if is_asset { "public, max-age=31536000, immutable" } else { "no-cache" };
        res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    res
}

pub fn build_router(state: AppState) -> Router {
    let mut app = Router::new()
        .route("/api/health", get(health::live))
        .route("/api/health/ready", get(health::ready))
        .nest("/api/v1", api_routes(&state))
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
        .route("/api", get(api_not_found))
        .route("/api/{*rest}", axum::routing::any(api_not_found));

    if let Some(dir) = &state.config.static_dir {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(dir).not_found_service(ServeFile::new(index)));
    }

    app.layer(middleware::from_fn_with_state(state.clone(), session_middleware))
        .layer(middleware::from_fn(static_cache_headers))
        .layer(middleware::from_fn_with_state(state.clone(), security_headers))
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, Duration::from_secs(300)))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
