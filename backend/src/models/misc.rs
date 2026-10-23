use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const RESOURCE_KINDS: &[&str] = &["pdf", "image", "markdown", "code", "link", "video", "datasheet", "lab"];

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Resource {
    pub id: String,
    pub course_id: String,
    pub module_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub file_id: Option<String>,
    pub url: Option<String>,
    pub content_md: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    // Joined columns.
    pub course_title: String,
    pub week_number: Option<i64>,
    pub module_title: Option<String>,
    pub lesson_title: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub file_type: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ResourceInput {
    pub course_id: Option<String>,
    pub module_id: Option<String>,
    pub week_id: Option<String>,
    pub lesson_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub file_id: Option<String>,
    pub url: Option<String>,
    pub content_md: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Announcement {
    pub id: String,
    pub course_id: String,
    pub course_title: String,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub title: String,
    pub body_md: String,
    pub status: String,
    pub pinned: bool,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AnnouncementInput {
    pub course_id: Option<String>,
    pub title: Option<String>,
    pub body_md: Option<String>,
    pub status: Option<String>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct Notification {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub read_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct NotificationList {
    pub items: Vec<Notification>,
    pub unread_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct CalendarEvent {
    pub id: String,
    pub course_id: String,
    pub course_title: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub location: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CalendarEventInput {
    pub course_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub starts_at: Option<String>,
    /// Empty string clears.
    pub ends_at: Option<String>,
    pub location: Option<String>,
}

/// Unified calendar entry: instructor-defined events, assignment deadlines and course milestones.
#[derive(Serialize, ToSchema)]
pub struct CalendarEntry {
    pub id: String,
    /// `session`, `lab`, `milestone`, `other`, `deadline`, `course_start`, `course_end`, `week_start`.
    pub kind: String,
    pub title: String,
    pub description: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub all_day: bool,
    pub location: String,
    pub course_id: String,
    pub course_title: String,
    pub link: Option<String>,
    /// Only for instructor-defined events (editable).
    pub event_id: Option<String>,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct AuditLog {
    pub id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: String,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct PlatformSettings {
    pub platform_name: String,
    pub default_timezone: String,
    pub support_email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PlatformSettingsInput {
    pub platform_name: Option<String>,
    pub default_timezone: Option<String>,
    pub support_email: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchResult {
    /// `course`, `week`, `lesson`, `assignment`, `resource`, `announcement`, `student`.
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub link: String,
}
