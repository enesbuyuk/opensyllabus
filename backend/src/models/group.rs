use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct GroupMember {
    pub student_id: String,
    pub name: String,
    pub email: String,
    /// Instructor only: this student's grade instead of the group grade.
    pub score_override: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GroupInfo {
    pub id: String,
    pub assignment_id: String,
    pub name: String,
    pub leader_id: String,
    pub members: Vec<GroupMember>,
    /// Instructor only: the shared grade before individual overrides.
    pub score: Option<f64>,
    /// The leader's (canonical) submission, once the leader has started.
    pub submission_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct GroupInput {
    pub name: String,
    pub leader_id: String,
    /// The leader is always a member, whether listed here or not.
    pub member_ids: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ScoreOverrideInput {
    /// Empty clears the override so the student gets the group grade again.
    pub score: Option<f64>,
}
