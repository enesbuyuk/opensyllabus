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
    /// IANA timezone a repeating event keeps its wall-clock time in.
    pub timezone: String,
    /// Attendance is taken for this event (each occurrence of a repeating one).
    pub takes_attendance: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    pub repeat_freq: Option<String>,
    #[serde(skip)]
    pub repeat_interval: i64,
    #[serde(skip)]
    pub repeat_weekdays: String,
    #[serde(skip)]
    pub repeat_until: Option<String>,
    #[serde(skip)]
    pub repeat_count: Option<i64>,
    #[serde(skip)]
    pub repeat_exceptions: String,
    /// Set for a repeating event; `starts_at` / `ends_at` are its first occurrence.
    #[sqlx(skip)]
    pub recurrence: Option<Recurrence>,
    /// Occurrence dates removed from the series (YYYY-MM-DD).
    #[sqlx(skip)]
    pub skipped_dates: Vec<String>,
}

/// Repeat rule of a calendar event.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Recurrence {
    /// `daily`, `weekly` or `monthly`; in a request, `none` makes the event non-repeating.
    pub freq: String,
    /// Every N days / weeks / months (1-99).
    #[serde(default = "one")]
    pub interval: i64,
    /// Weekly rules: ISO weekdays (1 = Monday ... 7 = Sunday). Empty = the weekday of the start.
    #[serde(default)]
    pub weekdays: Vec<u32>,
    /// Last occurrence date, inclusive (YYYY-MM-DD in the event timezone). At most one of until / count.
    pub until: Option<String>,
    /// Number of occurrences (1-500).
    pub count: Option<i64>,
}

fn one() -> i64 {
    1
}

#[derive(Deserialize, ToSchema)]
pub struct SkipOccurrenceRequest {
    /// Occurrence date to remove from the series (YYYY-MM-DD).
    pub date: String,
}

/// Series details on a calendar entry that is one occurrence of a repeating event.
#[derive(Serialize, ToSchema)]
pub struct EventSeries {
    pub recurrence: Recurrence,
    /// First occurrence of the series.
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub timezone: String,
    /// This occurrence's date in the series timezone (YYYY-MM-DD).
    pub occurrence_date: String,
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
    /// Omitted: unchanged. `{"freq": "none"}` stops repeating.
    pub recurrence: Option<Recurrence>,
    /// IANA timezone for a repeating event; defaults to the platform timezone.
    pub timezone: Option<String>,
    /// Take attendance for this event; omitted leaves it unchanged.
    pub takes_attendance: Option<bool>,
}

/// Unified calendar entry: instructor-defined events, assignment deadlines and course milestones.
#[derive(Serialize, ToSchema)]
pub struct CalendarEntry {
    pub id: String,
    /// `session`, `lab`, `milestone`, `other`, `deadline`, `course_start`, `course_end`, `week_start`, `class` (scheduled lesson).
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
    /// Live class link of a scheduled class (`kind = class`).
    pub meeting_url: Option<String>,
    /// Only for instructor-defined events (editable).
    pub event_id: Option<String>,
    /// Instructor-defined event that takes attendance.
    pub takes_attendance: bool,
    /// Set when the entry is one occurrence of a repeating event.
    pub series: Option<EventSeries>,
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
