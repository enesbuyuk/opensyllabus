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
