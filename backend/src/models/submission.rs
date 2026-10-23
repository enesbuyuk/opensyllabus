use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Json;

/// Structured laboratory report (used by `lab_report` assignments).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct LabReport {
    pub experiment_title: String,
    pub objective: String,
    pub equipment: String,
    pub setup: String,
    pub procedure: String,
    pub measurements: String,
    pub observations: String,
    pub results: String,
    pub discussion: String,
    pub conclusion: String,
}

impl LabReport {
    pub fn fields(&self) -> [(&'static str, &str); 10] {
        [
            ("experiment_title", &self.experiment_title),
            ("objective", &self.objective),
            ("equipment", &self.equipment),
            ("setup", &self.setup),
            ("procedure", &self.procedure),
            ("measurements", &self.measurements),
            ("observations", &self.observations),
            ("results", &self.results),
            ("discussion", &self.discussion),
            ("conclusion", &self.conclusion),
        ]
    }

    pub fn is_empty(&self) -> bool {
        self.fields().iter().all(|(_, v)| v.trim().is_empty())
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Submission {
    pub id: String,
    pub assignment_id: String,
    pub student_id: String,
    pub status: String,
    pub body_md: String,
    pub repository_url: Option<String>,
    #[schema(value_type = Option<LabReport>)]
    pub lab_report: Option<Json<LabReport>>,
    pub attempt: i64,
    pub submitted_at: Option<String>,
    pub is_late: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Attachment metadata exposed to clients. The private storage key is never included.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct FileMeta {
    pub id: String,
    pub original_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct SubmissionRevision {
    pub id: String,
    pub attempt: i64,
    pub body_md: String,
    pub repository_url: Option<String>,
    #[schema(value_type = Option<LabReport>)]
    pub lab_report: Option<Json<LabReport>>,
    #[schema(value_type = Vec<FileMeta>)]
    pub attachments: Json<Vec<FileMeta>>,
    pub submitted_at: String,
    pub is_late: bool,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Grade {
    pub id: String,
    pub submission_id: String,
    pub score: f64,
    pub max_score: f64,
    #[schema(value_type = BTreeMap<String, f64>)]
    pub rubric_scores: Json<BTreeMap<String, f64>>,
    pub late_penalty: f64,
    pub graded_by: Option<String>,
    pub graded_at: String,
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Feedback {
    pub id: String,
    pub submission_id: String,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub attempt: i64,
    pub kind: String,
    pub body_md: String,
    pub released_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct SubmissionDetail {
    #[serde(flatten)]
    pub submission: Submission,
    pub student: super::user::UserSummary,
    pub assignment: super::assignment::Assignment,
    pub course_title: String,
    pub week_number: Option<i64>,
    pub rubric: Vec<super::assignment::RubricCriterion>,
    pub attachments: Vec<FileMeta>,
    pub revisions: Vec<SubmissionRevision>,
    /// For students, only present once the review has been returned.
    pub grade: Option<Grade>,
    pub feedback: Vec<Feedback>,
    pub can_edit: bool,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct SubmissionListItem {
    pub id: String,
    pub assignment_id: String,
    pub assignment_title: String,
    pub course_id: String,
    pub course_title: String,
    pub week_id: Option<String>,
    pub week_number: Option<i64>,
    pub student_id: String,
    pub student_name: String,
    pub status: String,
    pub attempt: i64,
    pub submitted_at: Option<String>,
    pub is_late: bool,
    pub due_at: Option<String>,
    pub max_grade: f64,
    pub score: Option<f64>,
    pub released_at: Option<String>,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveDraftRequest {
    pub body_md: Option<String>,
    /// Empty string clears.
    pub repository_url: Option<String>,
    pub lab_report: Option<LabReport>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReviewRequest {
    /// `save` keeps the review private, `return` releases grade and feedback,
    /// `request_revision` releases feedback and reopens the submission.
    pub action: String,
    pub score: Option<f64>,
    #[serde(default)]
    pub rubric_scores: BTreeMap<String, f64>,
    pub feedback: Option<String>,
}
