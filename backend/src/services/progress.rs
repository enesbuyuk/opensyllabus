//! Progress calculations. See docs/PROGRESS.md for the rules in prose.
//!
//! * A lesson counts toward progress when it is published and its week is open
//!   (published and `available_from` reached).
//! * An assignment counts when it is published and its `publish_at` has passed.
//! * A lesson is complete when the student marked it complete.
//! * An assignment is complete only when its review was returned with a grade
//!   (status `graded`). Uploading or submitting alone is not completion.
//! * Completion % = (completed lessons + completed assignments) / (countable lessons + countable assignments).
//! * Grades are reported separately (average of released grades as a percentage of max).

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::{
    auth::access::week_is_open,
    errors::AppResult,
    models::{
        assignment::StudentAssignmentState,
        course::Course,
        curriculum::{Week, WeekState},
    },
};

pub fn percent(done: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        ((done as f64 / total as f64) * 1000.0).round() / 10.0
    }
}

/// Week number by calendar, based on the course start date. `None` before start or without a start date.
pub fn calendar_week(course: &Course, today: &str) -> Option<i64> {
    let start = NaiveDate::parse_from_str(course.start_date.as_deref()?, "%Y-%m-%d").ok()?;
    let today = NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    let days = (today - start).num_days();
    if days < 0 {
        return None;
    }
    Some((days / 7 + 1).min(course.duration_weeks))
}

pub fn week_start_date(course: &Course, week_number: i64) -> Option<String> {
    let start = NaiveDate::parse_from_str(course.start_date.as_deref()?, "%Y-%m-%d").ok()?;
    Some((start + Duration::days((week_number - 1) * 7)).format("%Y-%m-%d").to_string())
}

pub fn student_state(status: Option<&str>, is_late: bool) -> StudentAssignmentState {
    match status {
        None => StudentAssignmentState::NotStarted,
        Some("draft") => StudentAssignmentState::Draft,
        Some("submitted") if is_late => StudentAssignmentState::Late,
        Some("submitted") => StudentAssignmentState::Submitted,
        Some("under_review") => StudentAssignmentState::UnderReview,
        Some("graded") => StudentAssignmentState::Graded,
        Some("revision_requested") => StudentAssignmentState::RevisionRequested,
        Some(_) => StudentAssignmentState::NotStarted,
    }
}

#[derive(Serialize, ToSchema, Clone)]
pub struct WeekProgress {
    pub week_id: String,
    pub week_number: i64,
    pub title: String,
    pub state: WeekState,
    pub lessons_total: i64,
    pub lessons_completed: i64,
    pub assignments_total: i64,
    pub assignments_completed: i64,
    pub completion_percent: f64,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct GradeEntry {
    pub assignment_id: String,
    pub assignment_title: String,
    pub week_number: Option<i64>,
    pub score: f64,
    pub max_score: f64,
    pub percent: f64,
    pub released_at: String,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct CourseProgress {
    pub course_id: String,
    pub course_title: String,
    pub student_id: String,
    pub current_week: Option<i64>,
    pub lessons_total: i64,
    pub lessons_completed: i64,
    pub assignments_total: i64,
    pub assignments_completed: i64,
    /// Submitted / under review, waiting for the instructor.
    pub assignments_awaiting_review: i64,
    /// Not yet submitted (not started, draft or revision requested).
    pub assignments_pending: i64,
    pub assignments_late: i64,
    pub completion_percent: f64,
    pub lesson_percent: f64,
    pub assignment_percent: f64,
    /// Mean of released grades as a percentage; `None` when nothing is graded.
    pub average_grade_percent: Option<f64>,
    pub grades: Vec<GradeEntry>,
    pub weeks: Vec<WeekProgress>,
}

#[derive(sqlx::FromRow)]
struct LessonRow {
    id: String,
    week_id: String,
    status: String,
}

#[derive(sqlx::FromRow)]
struct AssignmentRow {
    id: String,
    title: String,
    week_id: Option<String>,
    status: String,
    publish_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct SubRow {
    assignment_id: String,
    status: String,
    is_late: bool,
    score: Option<f64>,
    max_score: Option<f64>,
    released_at: Option<String>,
}

/// Raw per-course data shared by student calculations.
pub struct CourseData {
    pub course: Course,
    pub weeks: Vec<Week>,
    lessons: Vec<LessonRow>,
    assignments: Vec<AssignmentRow>,
}

impl CourseData {
    pub async fn load(pool: &SqlitePool, course: Course) -> AppResult<Self> {
        let weeks: Vec<Week> =
            sqlx::query_as("SELECT * FROM weeks WHERE course_id = ? ORDER BY week_number").bind(&course.id).fetch_all(pool).await?;
        let lessons: Vec<LessonRow> =
            sqlx::query_as("SELECT l.id, l.week_id, l.status FROM lessons l JOIN weeks w ON w.id = l.week_id WHERE w.course_id = ?")
                .bind(&course.id)
                .fetch_all(pool)
                .await?;
        let assignments: Vec<AssignmentRow> =
            sqlx::query_as("SELECT id, title, week_id, status, publish_at FROM assignments WHERE course_id = ?")
                .bind(&course.id)
                .fetch_all(pool)
                .await?;
        Ok(Self { course, weeks, lessons, assignments })
    }

    fn open_week_ids(&self, today: &str) -> HashSet<&str> {
        self.weeks.iter().filter(|w| week_is_open(w, today)).map(|w| w.id.as_str()).collect()
    }

    /// Countable lessons: published, in an open week.
    fn countable_lessons<'a>(&'a self, open: &HashSet<&str>) -> Vec<&'a LessonRow> {
        self.lessons.iter().filter(|l| l.status == "published" && open.contains(l.week_id.as_str())).collect()
    }

    fn countable_assignments(&self, now: &str) -> Vec<&AssignmentRow> {
        self.assignments.iter().filter(|a| a.status == "published" && a.publish_at.as_deref().is_none_or(|p| p <= now)).collect()
    }

    pub async fn student_progress(&self, pool: &SqlitePool, student_id: &str, today: &str, now: &str) -> AppResult<CourseProgress> {
        let completed: HashSet<String> = sqlx::query_scalar(
            "SELECT lp.lesson_id FROM lesson_progress lp
             JOIN lessons l ON l.id = lp.lesson_id JOIN weeks w ON w.id = l.week_id
             WHERE lp.user_id = ? AND w.course_id = ?",
        )
        .bind(student_id)
        .bind(&self.course.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();

        let subs: Vec<SubRow> = sqlx::query_as(
            "SELECT s.assignment_id, s.status, s.is_late, g.score, g.max_score, g.released_at
             FROM submissions s JOIN assignments a ON a.id = s.assignment_id
             LEFT JOIN grades g ON g.submission_id = s.id
             WHERE s.student_id = ? AND a.course_id = ?",
        )
        .bind(student_id)
        .bind(&self.course.id)
        .fetch_all(pool)
        .await?;
        let subs: HashMap<&str, &SubRow> = subs.iter().map(|s| (s.assignment_id.as_str(), s)).collect();

        let open = self.open_week_ids(today);
        let lessons = self.countable_lessons(&open);
        let assignments = self.countable_assignments(now);

        let lesson_done = |l: &LessonRow| completed.contains(&l.id);
        let assignment_done = |a: &AssignmentRow| subs.get(a.id.as_str()).is_some_and(|s| s.status == "graded");

        let lessons_completed = lessons.iter().filter(|l| lesson_done(l)).count() as i64;
        let assignments_completed = assignments.iter().filter(|a| assignment_done(a)).count() as i64;
        let mut awaiting = 0;
        let mut pending = 0;
        let mut late = 0;
        for a in &assignments {
            match subs.get(a.id.as_str()) {
                Some(s) if matches!(s.status.as_str(), "submitted" | "under_review") => {
                    awaiting += 1;
                    if s.is_late {
                        late += 1;
                    }
                }
                Some(s) if s.status == "graded" => {
                    if s.is_late {
                        late += 1;
                    }
                }
                _ => pending += 1,
            }
        }

        let week_numbers: HashMap<&str, i64> = self.weeks.iter().map(|w| (w.id.as_str(), w.week_number)).collect();
        let mut grades: Vec<GradeEntry> = assignments
            .iter()
            .filter_map(|a| {
                let s = subs.get(a.id.as_str())?;
                if s.status != "graded" {
                    return None;
                }
                let (score, max, rel) = (s.score?, s.max_score?, s.released_at.clone()?);
                Some(GradeEntry {
                    assignment_id: a.id.clone(),
                    assignment_title: a.title.clone(),
                    week_number: a.week_id.as_deref().and_then(|w| week_numbers.get(w).copied()),
                    score,
                    max_score: max,
                    percent: percent_f(score, max),
                    released_at: rel,
                })
            })
            .collect();
        grades.sort_by(|a, b| b.released_at.cmp(&a.released_at));
        let average_grade_percent = if grades.is_empty() {
            None
        } else {
            Some((grades.iter().map(|g| g.percent).sum::<f64>() / grades.len() as f64 * 10.0).round() / 10.0)
        };

        // Current week: by calendar when a start date exists, else first open, unfinished week.
        let mut week_rows = Vec::with_capacity(self.weeks.len());
        for w in &self.weeks {
            let wl: Vec<_> = lessons.iter().filter(|l| l.week_id == w.id).collect();
            let wa: Vec<_> = assignments.iter().filter(|a| a.week_id.as_deref() == Some(&w.id)).collect();
            let lc = wl.iter().filter(|l| lesson_done(l)).count() as i64;
            let ac = wa.iter().filter(|a| assignment_done(a)).count() as i64;
            let total = (wl.len() + wa.len()) as i64;
            week_rows.push((w, wl.len() as i64, lc, wa.len() as i64, ac, total));
        }
        let current_week = calendar_week(&self.course, today).or_else(|| {
            week_rows
                .iter()
                .find(|(w, _, lc, _, ac, total)| open.contains(w.id.as_str()) && (*total == 0 || lc + ac < *total))
                .map(|(w, ..)| w.week_number)
        });

        let weeks = week_rows
            .into_iter()
            .map(|(w, lt, lc, at, ac, total)| {
                let state = if !open.contains(w.id.as_str()) {
                    WeekState::Locked
                } else if total > 0 && lc + ac == total {
                    WeekState::Completed
                } else if Some(w.week_number) == current_week {
                    WeekState::Current
                } else {
                    WeekState::Available
                };
                WeekProgress {
                    week_id: w.id.clone(),
                    week_number: w.week_number,
                    title: w.title.clone(),
                    state,
                    lessons_total: lt,
                    lessons_completed: lc,
                    assignments_total: at,
                    assignments_completed: ac,
                    completion_percent: percent(lc + ac, total),
                }
            })
            .collect();

        let lessons_total = lessons.len() as i64;
        let assignments_total = assignments.len() as i64;
        Ok(CourseProgress {
            course_id: self.course.id.clone(),
            course_title: self.course.title.clone(),
            student_id: student_id.to_string(),
            current_week,
            lessons_total,
            lessons_completed,
            assignments_total,
            assignments_completed,
            assignments_awaiting_review: awaiting,
            assignments_pending: pending,
            assignments_late: late,
            completion_percent: percent(lessons_completed + assignments_completed, lessons_total + assignments_total),
            lesson_percent: percent(lessons_completed, lessons_total),
            assignment_percent: percent(assignments_completed, assignments_total),
            average_grade_percent,
            grades,
            weeks,
        })
    }
}

pub fn percent_f(score: f64, max: f64) -> f64 {
    if max <= 0.0 {
        0.0
    } else {
        ((score / max) * 1000.0).round() / 10.0
    }
}

pub async fn load_course_data(pool: &SqlitePool, course_id: &str) -> AppResult<CourseData> {
    let course: Course = sqlx::query_as("SELECT * FROM courses WHERE id = ?").bind(course_id).fetch_one(pool).await?;
    CourseData::load(pool, course).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Json;

    fn course(start: Option<&str>) -> Course {
        Course {
            id: "c".into(),
            title: "t".into(),
            slug: "s".into(),
            description: String::new(),
            cover_file_id: None,
            difficulty: "intermediate".into(),
            duration_weeks: 48,
            days_per_week: 3,
            instructor_id: None,
            status: "published".into(),
            start_date: start.map(Into::into),
            end_date: None,
            objectives: Json(vec![]),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn calendar_week_math() {
        let c = course(Some("2026-01-05"));
        assert_eq!(calendar_week(&c, "2026-01-04"), None);
        assert_eq!(calendar_week(&c, "2026-01-05"), Some(1));
        assert_eq!(calendar_week(&c, "2026-01-11"), Some(1));
        assert_eq!(calendar_week(&c, "2026-01-12"), Some(2));
        assert_eq!(calendar_week(&c, "2030-01-01"), Some(48));
        assert_eq!(calendar_week(&course(None), "2026-01-12"), None);
        assert_eq!(week_start_date(&c, 3).as_deref(), Some("2026-01-19"));
    }

    #[test]
    fn percentages() {
        assert_eq!(percent(1, 3), 33.3);
        assert_eq!(percent(0, 0), 0.0);
        assert_eq!(percent_f(45.0, 50.0), 90.0);
    }

    #[test]
    fn assignment_states() {
        assert_eq!(student_state(None, false), StudentAssignmentState::NotStarted);
        assert_eq!(student_state(Some("submitted"), true), StudentAssignmentState::Late);
        assert_eq!(student_state(Some("graded"), false), StudentAssignmentState::Graded);
    }
}
