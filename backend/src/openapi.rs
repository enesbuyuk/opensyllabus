//! OpenAPI document, served at `/api/openapi.json` with Swagger UI at `/api/docs`.

use utoipa::OpenApi;

use crate::handlers::*;

#[derive(OpenApi)]
#[openapi(
    info(title = "OpenSyllabus API", version = "0.1.0",
         description = "REST API of the OpenSyllabus learning platform. Session cookie authentication; \
                        mutating requests require the `X-CSRF-Token` header returned by `/auth/login` and `/auth/me`."),
    paths(
        health::live, health::ready,
        auth::login, auth::logout, auth::me, auth::change_password, auth::reset_password,
        auth::list_sessions, auth::revoke_session, auth::revoke_other_sessions,
        users::list, users::create, users::get, users::update, users::reset_link, users::revoke_sessions, users::update_profile,
        courses::list, courses::get, courses::create, courses::update, courses::delete, courses::upload_cover,
        courses::enrollments, courses::enroll, courses::unenroll, courses::schedule, courses::update_schedule,
        curriculum::course_curriculum, curriculum::create_module, curriculum::update_module, curriculum::delete_module,
        curriculum::create_week, curriculum::generate_weeks, curriculum::get_week, curriculum::update_week, curriculum::delete_week,
        curriculum::duplicate_week, curriculum::create_lesson, curriculum::reorder_lessons, curriculum::get_lesson,
        curriculum::update_lesson, curriculum::delete_lesson, curriculum::duplicate_lesson, curriculum::complete_lesson,
        curriculum::uncomplete_lesson, curriculum::course_lessons,
        assignments::list, assignments::get, assignments::create, assignments::update, assignments::delete,
        submissions::my_submission, submissions::save_draft, submissions::upload_attachment, submissions::delete_attachment,
        submissions::submit, submissions::list, submissions::list_for_assignment, submissions::get,
        submissions::start_review, submissions::review,
        files::download, files::upload,
        resources::list, resources::get, resources::create, resources::update, resources::delete,
        announcements::list, announcements::create, announcements::update, announcements::delete,
        notifications::list, notifications::mark_read, notifications::mark_all_read,
        calendar::entries, calendar::create, calendar::update, calendar::skip_occurrence, calendar::delete,
        progress::my_progress, progress::course_progress, progress::student_progress, progress::gradebook, progress::gradebook_csv,
        dashboard::dashboard, analytics::analytics, search::search,
        settings::get, settings::update, settings::audit_logs,
    ),
    tags(
        (name = "auth"), (name = "users"), (name = "courses"), (name = "curriculum"), (name = "assignments"),
        (name = "submissions"), (name = "files"), (name = "resources"), (name = "announcements"),
        (name = "notifications"), (name = "calendar"), (name = "progress"), (name = "dashboard"),
        (name = "analytics"), (name = "search"), (name = "settings"), (name = "health"),
    )
)]
pub struct ApiDoc;
