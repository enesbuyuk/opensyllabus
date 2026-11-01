use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Json;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub cover_file_id: Option<String>,
    pub difficulty: String,
    pub duration_weeks: i64,
    pub days_per_week: i64,
    pub instructor_id: Option<String>,
    pub status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[schema(value_type = Vec<String>)]
    pub objectives: Json<Vec<String>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct CourseRef {
    pub id: String,
    pub title: String,
    pub slug: String,
}

#[derive(Serialize, ToSchema)]
pub struct CourseListItem {
    #[serde(flatten)]
    pub course: Course,
    pub instructor_name: Option<String>,
    pub enrolled_count: i64,
    pub published_weeks: i64,
    pub total_weeks: i64,
    /// Present for students: their completion percentage.
    pub completion_percent: Option<f64>,
    pub current_week: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCourseRequest {
    pub title: String,
    pub slug: Option<String>,
    #[serde(default)]
    pub description: String,
    pub difficulty: Option<String>,
    pub duration_weeks: Option<i64>,
    pub days_per_week: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub objectives: Vec<String>,
    /// Create empty week shells for the full duration.
    #[serde(default)]
    pub generate_weeks: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCourseRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub duration_weeks: Option<i64>,
    pub days_per_week: Option<i64>,
    pub status: Option<String>,
    /// Empty string clears the date.
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub objectives: Option<Vec<String>>,
    pub instructor_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct EnrollRequest {
    pub user_ids: Vec<String>,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct EnrollmentItem {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub enrolled_at: String,
}

/// One weekly class: course day `day_number` (the lessons' day) meets on `weekday`.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct ScheduleSlot {
    /// Course day, 1 to `days_per_week`.
    pub day_number: i64,
    /// ISO weekday: 1 = Monday ... 7 = Sunday.
    pub weekday: i64,
    /// HH:MM in the schedule timezone.
    pub start_time: String,
    pub end_time: Option<String>,
    #[serde(default)]
    pub location: String,
    /// Default live class link (https) for this weekly class; a lesson may override it.
    #[serde(default)]
    pub meeting_url: Option<String>,
}

/// Weekly class times of a course. Lesson dates follow from the course start date.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CourseSchedule {
    /// IANA timezone of the times.
    pub timezone: String,
    pub slots: Vec<ScheduleSlot>,
}
