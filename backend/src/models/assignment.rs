use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Json;

pub const KINDS: &[&str] = &["written", "file_upload", "code", "lab_report", "research", "repository", "mixed"];
pub const LATE_POLICIES: &[&str] = &["accept", "penalty", "reject"];

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Assignment {
    pub id: String,
    pub course_id: String,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub title: String,
    pub description: String,
    pub instructions_md: String,
    pub reference_md: String,
    pub kind: String,
    pub max_grade: f64,
    pub due_at: Option<String>,
    pub publish_at: Option<String>,
    pub late_policy: String,
    pub late_penalty_percent: i64,
    #[schema(value_type = Vec<String>)]
    pub allowed_file_types: Json<Vec<String>>,
    pub max_upload_mb: i64,
    pub allow_resubmission: bool,
    pub status: String,
    /// Students submit in groups; only the group leader can submit.
    pub is_group: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Assignment {
    /// Visible to students: published and past its publication date.
    pub fn is_visible_at(&self, now: &str) -> bool {
        self.status == "published" && self.publish_at.as_deref().is_none_or(|p| p <= now)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct RubricCriterion {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub position: i64,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub max_points: f64,
}

/// Student-facing assignment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StudentAssignmentState {
    NotStarted,
    Draft,
    Submitted,
    Late,
    UnderReview,
    Graded,
    RevisionRequested,
}

impl StudentAssignmentState {
    pub fn to_snake(self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }
}

#[derive(Clone, Serialize, ToSchema)]
pub struct AssignmentListItem {
    #[serde(flatten)]
    pub assignment: Assignment,
    pub course_title: String,
    pub week_number: Option<i64>,
    pub week_title: Option<String>,
    pub lesson_title: Option<String>,
    /// Student view: own state and released score.
    pub state: Option<StudentAssignmentState>,
    pub score: Option<f64>,
    pub submitted_at: Option<String>,
    /// Instructor view: submission counters.
    pub submission_count: Option<i64>,
    pub pending_review_count: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct AssignmentDetail {
    #[serde(flatten)]
    pub item: AssignmentListItem,
    pub rubric: Vec<RubricCriterion>,
    pub is_past_due: bool,
    pub accepts_submissions: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct AssignmentInput {
    pub course_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub instructions_md: Option<String>,
    pub reference_md: Option<String>,
    pub kind: Option<String>,
    pub max_grade: Option<f64>,
    /// ISO-8601 with offset; empty string clears.
    pub due_at: Option<String>,
    pub publish_at: Option<String>,
    pub late_policy: Option<String>,
    pub late_penalty_percent: Option<i64>,
    pub allowed_file_types: Option<Vec<String>>,
    pub max_upload_mb: Option<i64>,
    pub allow_resubmission: Option<bool>,
    pub is_group: Option<bool>,
    pub status: Option<String>,
    pub rubric: Option<Vec<RubricCriterion>>,
}
