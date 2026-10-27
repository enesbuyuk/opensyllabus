//! Role-specific dashboard payloads. Every number is computed from the database.

use axum::{extract::State, response::Response};
use chrono::{Duration, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use super::json;
use crate::{
    auth::{access, AuthUser},
    errors::AppResult,
    models::{
        assignment::{AssignmentListItem, StudentAssignmentState},
        misc::Announcement,
        submission::SubmissionListItem,
        user::User,
    },
    repositories::{
        assignments::{self as assignment_repo, AssignmentFilter},
        submissions::{self as submission_repo, SubmissionFilter},
    },
    services::progress::{self, CourseProgress, GradeEntry},
    util::{fmt_ts, now},
    AppState,
};

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct DashboardLesson {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub day_number: i64,
    pub estimated_minutes: i64,
    pub completed: bool,
}

#[derive(Serialize, ToSchema)]
pub struct CurrentWeek {
    pub course_id: String,
    pub course_title: String,
    pub week_id: String,
    pub week_number: i64,
    pub title: String,
    pub description: String,
    pub completion_percent: f64,
    pub lessons: Vec<DashboardLesson>,
}

#[derive(Serialize, ToSchema)]
pub struct ContinueLearning {
    pub lesson_id: String,
    pub lesson_title: String,
    pub week_number: i64,
    pub day_number: i64,
}

#[derive(Serialize, ToSchema)]
pub struct StudentDashboard {
    pub courses: Vec<CourseProgress>,
    pub current_week: Option<CurrentWeek>,
    pub continue_learning: Option<ContinueLearning>,
    pub pending_assignments: Vec<AssignmentListItem>,
    pub upcoming_deadlines: Vec<AssignmentListItem>,
    pub announcements: Vec<Announcement>,
    pub latest_grades: Vec<GradeEntry>,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct ActivityItem {
    /// `submission`, `lesson_completed`.
    pub kind: String,
    pub student_name: String,
    pub title: String,
    pub link: String,
    pub at: String,
}

#[derive(Serialize, ToSchema)]
pub struct StudentOverview {
    pub student_id: String,
    pub name: String,
    pub course_id: String,
    pub course_title: String,
    pub completion_percent: f64,
    pub average_grade_percent: Option<f64>,
    pub assignments_pending: i64,
    pub last_login_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct WeeklyCount {
    /// Monday (UTC) of the week, YYYY-MM-DD.
    pub week_start: String,
    pub submissions: i64,
    pub lessons_completed: i64,
}

#[derive(Serialize, ToSchema)]
pub struct AdminDashboard {
    pub total_students: i64,
    pub active_courses: i64,
    pub pending_submissions: i64,
    /// Student–assignment pairs that are past due without a submission.
    pub overdue_assignments: i64,
    pub average_completion: Option<f64>,
    pub recent_activity: Vec<ActivityItem>,
    pub upcoming_deadlines: Vec<AssignmentListItem>,
    pub review_queue: Vec<SubmissionListItem>,
    pub student_overview: Vec<StudentOverview>,
    pub weekly_activity: Vec<WeeklyCount>,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Dashboard {
    Student(StudentDashboard),
    Admin(AdminDashboard),
}

#[utoipa::path(get, path = "/api/v1/dashboard", tag = "dashboard", responses((status = 200, body = Dashboard)))]
pub async fn dashboard(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    if user.is_admin() {
        Ok(json(Dashboard::Admin(admin_dashboard(&state, &user).await?)))
    } else {
        Ok(json(Dashboard::Student(student_dashboard(&state, &user).await?)))
    }
}

async fn student_dashboard(state: &AppState, user: &User) -> AppResult<StudentDashboard> {
    let (today, ts) = (state.today(), now());
    let mut courses = Vec::new();
    for cid in access::visible_course_ids(&state.pool, user).await? {
        let data = progress::load_course_data(&state.pool, &cid).await?;
        if data.course.status == "published" {
            courses.push(data.student_progress(&state.pool, &user.id, &today, &ts).await?);
        }
    }

    // Current week of the first active course.
    let mut current_week = None;
    if let Some(p) = courses.first() {
        if let Some(wp) =
            p.weeks.iter().find(|w| Some(w.week_number) == p.current_week && w.state != crate::models::curriculum::WeekState::Locked)
        {
            let (title, description): (String, String) =
                sqlx::query_as("SELECT title, description FROM weeks WHERE id = ?").bind(&wp.week_id).fetch_one(&state.pool).await?;
            let lessons: Vec<DashboardLesson> = sqlx::query_as(
                "SELECT l.id, l.title, l.summary, l.day_number, l.estimated_minutes,
                        EXISTS (SELECT 1 FROM lesson_progress lp WHERE lp.lesson_id = l.id AND lp.user_id = ?) AS completed
                 FROM lessons l WHERE l.week_id = ? AND l.status = 'published' ORDER BY l.day_number, l.position",
            )
            .bind(&user.id)
            .bind(&wp.week_id)
            .fetch_all(&state.pool)
            .await?;
            current_week = Some(CurrentWeek {
                course_id: p.course_id.clone(),
                course_title: p.course_title.clone(),
                week_id: wp.week_id.clone(),
                week_number: wp.week_number,
                title,
                description,
                completion_percent: wp.completion_percent,
                lessons,
            });
        }
    }

    // First incomplete, open lesson, starting from the current week.
    let mut continue_learning = None;
    if let Some(p) = courses.first() {
        let from_week = p.current_week.unwrap_or(1);
        let row: Option<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT l.id, l.title, w.week_number, l.day_number FROM lessons l JOIN weeks w ON w.id = l.week_id
             WHERE w.course_id = ? AND l.status = 'published' AND w.status = 'published'
               AND (w.available_from IS NULL OR w.available_from <= ?)
               AND NOT EXISTS (SELECT 1 FROM lesson_progress lp WHERE lp.lesson_id = l.id AND lp.user_id = ?)
             ORDER BY (w.week_number < ?), w.week_number, l.day_number, l.position LIMIT 1",
        )
        .bind(&p.course_id)
        .bind(&today)
        .bind(&user.id)
        .bind(from_week)
        .fetch_optional(&state.pool)
        .await?;
        continue_learning = row.map(|(lesson_id, lesson_title, week_number, day_number)| ContinueLearning {
            lesson_id,
            lesson_title,
            week_number,
            day_number,
        });
    }

    let all = assignment_repo::list(&state.pool, user, &AssignmentFilter::default(), &ts, &today).await?;
    let pending_states = [StudentAssignmentState::NotStarted, StudentAssignmentState::Draft, StudentAssignmentState::RevisionRequested];
    let horizon = fmt_ts(Utc::now() + Duration::days(14));
    let mut pending: Vec<AssignmentListItem> = Vec::new();
    let mut upcoming: Vec<AssignmentListItem> = Vec::new();
    for a in all {
        let open = a.state.is_some_and(|s| pending_states.contains(&s));
        let due_soon = a.assignment.due_at.as_deref().is_some_and(|d| d >= ts.as_str() && d <= horizon.as_str());
        if open && due_soon {
            upcoming.push(a.clone());
        }
        if open {
            pending.push(a);
        }
    }
    pending.truncate(8);
    upcoming.truncate(8);

    let announcements = super::announcements::list_for(state, user, None, 5).await?;
    let mut latest_grades: Vec<GradeEntry> = courses.iter().flat_map(|c| c.grades.clone()).collect();
    latest_grades.sort_by(|a, b| b.released_at.cmp(&a.released_at));
    latest_grades.truncate(5);

    Ok(StudentDashboard {
        courses,
        current_week,
        continue_learning,
        pending_assignments: pending,
        upcoming_deadlines: upcoming,
        announcements,
        latest_grades,
    })
}

async fn admin_dashboard(state: &AppState, admin_user: &User) -> AppResult<AdminDashboard> {
    let pool = &state.pool;
    let ts = now();
    let total_students: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'student' AND is_active = 1").fetch_one(pool).await?;
    let active_courses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM courses WHERE status = 'published'").fetch_one(pool).await?;
    let pending_submissions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM submissions WHERE mirror_of IS NULL AND status IN ('submitted', 'under_review')")
            .fetch_one(pool)
            .await?;
    let overdue_assignments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assignments a JOIN courses c ON c.id = a.course_id
         JOIN course_enrollments e ON e.course_id = a.course_id JOIN users u ON u.id = e.user_id AND u.role = 'student' AND u.is_active = 1
         WHERE a.status = 'published' AND c.status = 'published' AND a.due_at IS NOT NULL AND a.due_at < ?
           AND NOT EXISTS (SELECT 1 FROM submissions s WHERE s.assignment_id = a.id AND s.student_id = u.id AND s.status <> 'draft')",
    )
    .bind(&ts)
    .fetch_one(pool)
    .await?;

    let mut student_overview = Vec::new();
    let course_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM courses WHERE status = 'published' ORDER BY created_at").fetch_all(pool).await?;
    for cid in &course_ids {
        let (data, rows) = super::progress::course_rows(state, cid).await?;
        for r in rows {
            student_overview.push(StudentOverview {
                student_id: r.student.id,
                name: r.student.name,
                course_id: data.course.id.clone(),
                course_title: data.course.title.clone(),
                completion_percent: r.progress.completion_percent,
                average_grade_percent: r.progress.average_grade_percent,
                assignments_pending: r.progress.assignments_pending,
                last_login_at: r.last_login_at,
            });
        }
    }
    let average_completion = if student_overview.is_empty() {
        None
    } else {
        Some((student_overview.iter().map(|s| s.completion_percent).sum::<f64>() / student_overview.len() as f64 * 10.0).round() / 10.0)
    };

    let recent_activity: Vec<ActivityItem> = sqlx::query_as(
        "SELECT * FROM (
            SELECT 'submission' AS kind, u.name AS student_name, a.title AS title, '/admin/reviews?submission=' || s.id AS link, s.submitted_at AS at
            FROM submissions s JOIN users u ON u.id = s.student_id JOIN assignments a ON a.id = s.assignment_id WHERE s.submitted_at IS NOT NULL AND s.mirror_of IS NULL
            UNION ALL
            SELECT 'lesson_completed', u.name, l.title, '/lessons/' || l.id, lp.completed_at
            FROM lesson_progress lp JOIN users u ON u.id = lp.user_id JOIN lessons l ON l.id = lp.lesson_id
         ) ORDER BY at DESC LIMIT 12",
    )
    .fetch_all(pool)
    .await?;

    let horizon = fmt_ts(Utc::now() + Duration::days(14));
    let upcoming_deadlines: Vec<AssignmentListItem> = assignment_repo::list(
        pool,
        admin_user,
        &AssignmentFilter { status: Some("published".into()), ..Default::default() },
        &ts,
        &state.today(),
    )
    .await?
    .into_iter()
    .filter(|a| a.assignment.due_at.as_deref().is_some_and(|d| d >= ts.as_str() && d <= horizon.as_str()))
    .take(8)
    .collect();

    let mut review_queue = submission_repo::list(pool, &SubmissionFilter { status: Some("pending".into()), ..Default::default() }).await?;
    review_queue.truncate(8);

    // Activity per week for the last 8 weeks (UTC weeks starting Monday).
    let today = Utc::now().date_naive();
    let monday = today - Duration::days(chrono::Datelike::weekday(&today).num_days_from_monday() as i64);
    let mut weekly_activity = Vec::new();
    for i in (0..8).rev() {
        let start = monday - Duration::weeks(i);
        let end = start + Duration::weeks(1);
        let (s, e) = (format!("{start}T00:00:00.000Z"), format!("{end}T00:00:00.000Z"));
        let submissions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM submission_revisions WHERE submitted_at >= ? AND submitted_at < ?")
            .bind(&s)
            .bind(&e)
            .fetch_one(pool)
            .await?;
        let lessons_completed: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM lesson_progress WHERE completed_at >= ? AND completed_at < ?")
                .bind(&s)
                .bind(&e)
                .fetch_one(pool)
                .await?;
        weekly_activity.push(WeeklyCount { week_start: start.to_string(), submissions, lessons_completed });
    }

    Ok(AdminDashboard {
        total_students,
        active_courses,
        pending_submissions,
        overdue_assignments,
        average_completion,
        recent_activity,
        upcoming_deadlines,
        review_queue,
        student_overview,
        weekly_activity,
    })
}
