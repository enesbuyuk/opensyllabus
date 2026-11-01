use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Json;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Module {
    pub id: String,
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Week {
    pub id: String,
    pub course_id: String,
    pub module_id: Option<String>,
    pub week_number: i64,
    pub title: String,
    pub description: String,
    #[schema(value_type = Vec<String>)]
    pub objectives: Json<Vec<String>>,
    /// Hidden from students.
    pub instructor_notes: String,
    pub estimated_minutes: i64,
    pub status: String,
    pub available_from: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Lesson {
    pub id: String,
    pub week_id: String,
    pub day_number: i64,
    pub position: i64,
    pub title: String,
    pub summary: String,
    pub content_md: String,
    pub lab_md: String,
    pub video_url: Option<String>,
    #[schema(value_type = Vec<String>)]
    pub objectives: Json<Vec<String>>,
    pub estimated_minutes: i64,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Live class link for this lesson's session; overrides the schedule slot's link.
    pub meeting_url: Option<String>,
}

/// Student-facing week state on the curriculum timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeekState {
    Completed,
    Current,
    Available,
    Locked,
}

#[derive(Serialize, ToSchema)]
pub struct WeekSummary {
    #[serde(flatten)]
    pub week: Week,
    pub state: WeekState,
    pub lesson_count: i64,
    pub published_lesson_count: i64,
    pub completed_lesson_count: i64,
    pub assignment_count: i64,
    pub completion_percent: f64,
    pub starts_on: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CurriculumResponse {
    pub course_id: String,
    pub current_week: Option<i64>,
    pub modules: Vec<Module>,
    pub weeks: Vec<WeekSummary>,
}

#[derive(Serialize, ToSchema)]
pub struct LessonListItem {
    #[serde(flatten)]
    pub lesson: Lesson,
    pub completed: bool,
    pub locked_by_prerequisites: bool,
}

#[derive(Serialize, ToSchema)]
pub struct WeekDetail {
    #[serde(flatten)]
    pub summary: WeekSummary,
    pub module: Option<Module>,
    pub lessons: Vec<LessonListItem>,
    pub assignments: Vec<super::assignment::AssignmentListItem>,
    pub resources: Vec<super::misc::Resource>,
    pub previous_week_id: Option<String>,
    pub next_week_id: Option<String>,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct LessonRef {
    pub id: String,
    pub title: String,
    pub week_number: i64,
    pub day_number: i64,
    pub completed: bool,
}

#[derive(Serialize, ToSchema)]
pub struct LessonDetail {
    #[serde(flatten)]
    pub lesson: Lesson,
    pub course_id: String,
    pub week_number: i64,
    pub week_title: String,
    pub week_status: String,
    pub completed: bool,
    pub completed_at: Option<String>,
    pub prerequisites: Vec<LessonRef>,
    pub prerequisites_met: bool,
    pub resources: Vec<super::misc::Resource>,
    pub assignments: Vec<super::assignment::AssignmentListItem>,
    pub previous_lesson_id: Option<String>,
    pub next_lesson_id: Option<String>,
    /// Effective live class link: the lesson's own link, else its schedule slot's link.
    pub class_meeting_url: Option<String>,
    /// Scheduled class time of this lesson, when the course has a start date and schedule.
    pub class_starts_at: Option<String>,
    pub class_ends_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ModuleInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub position: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateModuleRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub position: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateWeekRequest {
    pub week_number: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub module_id: Option<String>,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub instructor_notes: String,
    #[serde(default)]
    pub estimated_minutes: i64,
    /// Create one empty draft lesson per instructional day.
    #[serde(default)]
    pub create_day_lessons: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateWeekRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    /// Empty string detaches the week from its module.
    pub module_id: Option<String>,
    pub objectives: Option<Vec<String>>,
    pub instructor_notes: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub status: Option<String>,
    /// Empty string clears the date.
    pub available_from: Option<String>,
    /// Also publish/unpublish every lesson in the week.
    #[serde(default)]
    pub cascade_lessons: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct DuplicateWeekRequest {
    pub target_week_number: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateLessonRequest {
    pub title: String,
    pub day_number: Option<i64>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub content_md: String,
    #[serde(default)]
    pub lab_md: String,
    pub video_url: Option<String>,
    pub meeting_url: Option<String>,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub estimated_minutes: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateLessonRequest {
    pub title: Option<String>,
    pub day_number: Option<i64>,
    pub summary: Option<String>,
    pub content_md: Option<String>,
    pub lab_md: Option<String>,
    /// Empty string removes the video.
    pub video_url: Option<String>,
    /// Empty string removes the lesson's own class link (the schedule's link applies again).
    pub meeting_url: Option<String>,
    pub objectives: Option<Vec<String>>,
    pub estimated_minutes: Option<i64>,
    pub status: Option<String>,
    pub prerequisite_ids: Option<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReorderLessonsRequest {
    pub lesson_ids: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DuplicateLessonRequest {
    /// Destination week; defaults to the lesson's own week.
    pub target_week_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct GenerateWeeksRequest {
    #[serde(default)]
    pub create_day_lessons: bool,
}
