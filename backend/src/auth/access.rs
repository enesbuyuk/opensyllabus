//! Resource-level authorization checks shared by handlers.
//!
//! Rules:
//! * Administrators can access everything.
//! * Students can access a course only when enrolled and the course is published or archived.
//! * Students see weeks/lessons only when published (and the week's `available_from` date has passed).
//! * Students see assignments only when published and past `publish_at`.
//! * Students see only their own submissions; grades/feedback only once released.

use sqlx::SqlitePool;

use crate::{
    errors::{AppError, AppResult},
    models::{
        assignment::Assignment,
        course::Course,
        curriculum::{Lesson, Week},
        user::User,
    },
};

pub async fn is_enrolled(pool: &SqlitePool, course_id: &str, user_id: &str) -> AppResult<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM course_enrollments WHERE course_id = ? AND user_id = ?")
        .bind(course_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(n > 0)
}

pub fn student_can_view_course(course: &Course) -> bool {
    matches!(course.status.as_str(), "published" | "archived")
}

/// Loads a course the user may view. Unauthorized access is reported as "not found"
/// so students cannot probe for the existence of other courses.
pub async fn course_for(pool: &SqlitePool, user: &User, course_id: &str) -> AppResult<Course> {
    let course: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ? OR slug = ?")
        .bind(course_id)
        .bind(course_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("Course"))?;
    if user.is_admin() {
        return Ok(course);
    }
    if student_can_view_course(&course) && is_enrolled(pool, &course.id, &user.id).await? {
        Ok(course)
    } else {
        Err(AppError::NotFound("Course"))
    }
}

pub fn week_is_open(week: &Week, today: &str) -> bool {
    week.status == "published" && week.available_from.as_deref().is_none_or(|d| d <= today)
}

pub async fn week_for(pool: &SqlitePool, user: &User, week_id: &str, today: &str) -> AppResult<(Week, Course)> {
    let week: Week =
        sqlx::query_as("SELECT * FROM weeks WHERE id = ?").bind(week_id).fetch_optional(pool).await?.ok_or(AppError::NotFound("Week"))?;
    let course = course_for(pool, user, &week.course_id).await.map_err(|_| AppError::NotFound("Week"))?;
    if !user.is_admin() && !week_is_open(&week, today) {
        return Err(AppError::NotFound("Week"));
    }
    Ok((week, course))
}

pub async fn lesson_for(pool: &SqlitePool, user: &User, lesson_id: &str, today: &str) -> AppResult<(Lesson, Week, Course)> {
    let lesson: Lesson = sqlx::query_as("SELECT * FROM lessons WHERE id = ?")
        .bind(lesson_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("Lesson"))?;
    let (week, course) = week_for(pool, user, &lesson.week_id, today).await.map_err(|_| AppError::NotFound("Lesson"))?;
    if !user.is_admin() && lesson.status != "published" {
        return Err(AppError::NotFound("Lesson"));
    }
    Ok((lesson, week, course))
}

pub async fn assignment_for(
    pool: &SqlitePool,
    user: &User,
    assignment_id: &str,
    now: &str,
    today: &str,
) -> AppResult<(Assignment, Course)> {
    let a: Assignment = sqlx::query_as("SELECT * FROM assignments WHERE id = ?")
        .bind(assignment_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("Assignment"))?;
    let course = course_for(pool, user, &a.course_id).await.map_err(|_| AppError::NotFound("Assignment"))?;
    if !user.is_admin() {
        if !a.is_visible_at(now) {
            return Err(AppError::NotFound("Assignment"));
        }
        // Assignments attached to a week are hidden until that week is open.
        if let Some(week_id) = &a.week_id {
            let week: Option<Week> = sqlx::query_as("SELECT * FROM weeks WHERE id = ?").bind(week_id).fetch_optional(pool).await?;
            if !week.is_some_and(|w| week_is_open(&w, today)) {
                return Err(AppError::NotFound("Assignment"));
            }
        }
    }
    Ok((a, course))
}

/// IDs of courses visible to the user (all for admins).
pub async fn visible_course_ids(pool: &SqlitePool, user: &User) -> AppResult<Vec<String>> {
    let ids = if user.is_admin() {
        sqlx::query_scalar("SELECT id FROM courses ORDER BY created_at").fetch_all(pool).await?
    } else {
        sqlx::query_scalar(
            "SELECT c.id FROM courses c JOIN course_enrollments e ON e.course_id = c.id
             WHERE e.user_id = ? AND c.status IN ('published', 'archived') ORDER BY c.created_at",
        )
        .bind(&user.id)
        .fetch_all(pool)
        .await?
    };
    Ok(ids)
}
