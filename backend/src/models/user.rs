use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "TEXT")]
pub enum Role {
    Admin,
    Student,
}

/// Full user row. `password_hash` is never serialized.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct User {
    pub id: String,
    pub email: String,
    /// Optional login name; sign-in accepts it instead of the email.
    pub username: Option<String>,
    pub name: String,
    pub role: Role,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub password_hash: String,
    pub timezone: String,
    pub bio: String,
    pub is_active: bool,
    pub must_change_password: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}

/// Minimal public projection of a user, safe to embed in other payloads.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Email address or username. `email` is accepted as the field name too.
    #[serde(alias = "email")]
    pub login: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user: User,
    pub csrf_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    /// Empty string removes the username.
    pub username: Option<String>,
    pub bio: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: Option<String>,
    pub name: String,
    /// Optional initial password. When omitted, a one-time setup link is returned instead.
    pub password: Option<String>,
    #[serde(default)]
    pub course_ids: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateUserResponse {
    pub user: User,
    /// Present when no password was supplied: a one-time link the student uses to set a password.
    pub setup_link: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AdminUpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    /// Empty string removes the username.
    pub username: Option<String>,
    pub is_active: Option<bool>,
    pub timezone: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ResetLinkResponse {
    pub link: String,
    pub expires_at: String,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct SessionInfo {
    pub id: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
    #[sqlx(skip)]
    pub current: bool,
}

/// Student row with enrollment and progress information for the management table.
#[derive(Serialize, ToSchema)]
pub struct StudentListItem {
    #[serde(flatten)]
    pub user: User,
    pub courses: Vec<super::course::CourseRef>,
}
