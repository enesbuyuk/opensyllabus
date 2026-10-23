//! Global search across content the user is allowed to see.

use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;
use utoipa::IntoParams;

use super::json;
use crate::{
    auth::{access, AuthUser},
    errors::AppResult,
    models::misc::SearchResult,
    repositories::{
        assignments::{self as assignment_repo, AssignmentFilter},
        resources::{self as resource_repo, ResourceFilter},
    },
    util::now,
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct SearchQuery {
    pub q: String,
}

#[utoipa::path(get, path = "/api/v1/search", tag = "search", params(SearchQuery), responses((status = 200, body = [SearchResult])))]
pub async fn search(State(state): State<AppState>, user: AuthUser, Query(q): Query<SearchQuery>) -> AppResult<Response> {
    let term = q.q.trim().to_lowercase();
    if term.chars().count() < 2 {
        return Ok(json(Vec::<SearchResult>::new()));
    }
    let like = format!("%{}%", term.replace('%', "\\%").replace('_', "\\_"));
    let matches = |s: &str| s.to_lowercase().contains(&term);
    let today = state.today();
    let mut out: Vec<SearchResult> = Vec::new();

    let course_ids = access::visible_course_ids(&state.pool, &user).await?;
    for cid in &course_ids {
        let c = access::course_for(&state.pool, &user, cid).await?;
        if matches(&c.title) || matches(&c.description) {
            out.push(SearchResult {
                kind: "course".into(),
                id: c.id.clone(),
                title: c.title.clone(),
                subtitle: "Course".into(),
                link: format!("/courses/{}", c.id),
            });
        }
        let weeks: Vec<(String, i64, String)> = sqlx::query_as(
            "SELECT id, week_number, title FROM weeks WHERE course_id = ?1 AND (lower(title) LIKE ?2 ESCAPE '\\' OR lower(description) LIKE ?2 ESCAPE '\\')
             AND (?3 = 1 OR (status = 'published' AND (available_from IS NULL OR available_from <= ?4))) ORDER BY week_number LIMIT 10",
        )
        .bind(cid)
        .bind(&like)
        .bind(user.is_admin())
        .bind(&today)
        .fetch_all(&state.pool)
        .await?;
        for (id, n, title) in weeks {
            out.push(SearchResult {
                kind: "week".into(),
                link: format!("/weeks/{id}"),
                id,
                title: format!("Week {n}: {title}"),
                subtitle: c.title.clone(),
            });
        }
        let lessons: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT l.id, l.title, w.week_number, l.day_number FROM lessons l JOIN weeks w ON w.id = l.week_id
             WHERE w.course_id = ?1 AND (lower(l.title) LIKE ?2 ESCAPE '\\' OR lower(l.summary) LIKE ?2 ESCAPE '\\' OR lower(l.content_md) LIKE ?2 ESCAPE '\\')
             AND (?3 = 1 OR (l.status = 'published' AND w.status = 'published' AND (w.available_from IS NULL OR w.available_from <= ?4)))
             ORDER BY w.week_number, l.day_number LIMIT 10",
        )
        .bind(cid)
        .bind(&like)
        .bind(user.is_admin())
        .bind(&today)
        .fetch_all(&state.pool)
        .await?;
        for (id, title, w, d) in lessons {
            out.push(SearchResult {
                kind: "lesson".into(),
                link: format!("/lessons/{id}"),
                id,
                title,
                subtitle: format!("Week {w} · Day {d}"),
            });
        }
    }

    let assignments = assignment_repo::list(&state.pool, &user, &AssignmentFilter::default(), &now(), &today).await?;
    for a in assignments.into_iter().filter(|a| matches(&a.assignment.title) || matches(&a.assignment.description)).take(10) {
        out.push(SearchResult {
            kind: "assignment".into(),
            link: format!("/assignments/{}", a.assignment.id),
            id: a.assignment.id,
            title: a.assignment.title,
            subtitle: a.week_number.map(|w| format!("Assignment · Week {w}")).unwrap_or_else(|| "Assignment".into()),
        });
    }
    let resources =
        resource_repo::list(&state.pool, &user, &ResourceFilter { q: Some(term.clone()), ..Default::default() }, &today).await?;
    for r in resources.into_iter().take(10) {
        out.push(SearchResult {
            kind: "resource".into(),
            link: format!("/resources?highlight={}", r.id),
            id: r.id,
            title: r.title,
            subtitle: format!("Resource · {}", r.kind),
        });
    }
    let announcements = super::announcements::list_for(&state, &user, None, 200).await?;
    for a in announcements.into_iter().filter(|a| matches(&a.title) || matches(&a.body_md)).take(5) {
        out.push(SearchResult {
            kind: "announcement".into(),
            link: format!("/announcements#{}", a.id),
            id: a.id,
            title: a.title,
            subtitle: "Announcement".into(),
        });
    }
    if user.is_admin() {
        let students: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, name, email FROM users WHERE role = 'student' AND (lower(name) LIKE ?1 ESCAPE '\\' OR lower(email) LIKE ?1 ESCAPE '\\') LIMIT 10",
        )
        .bind(&like)
        .fetch_all(&state.pool)
        .await?;
        for (id, name, email) in students {
            out.push(SearchResult {
                kind: "student".into(),
                link: format!("/admin/students?highlight={id}"),
                id,
                title: name,
                subtitle: email,
            });
        }
    }
    Ok(json(out))
}
