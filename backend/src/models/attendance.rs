use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::user::UserSummary;

pub const STATUSES: &[&str] = &["present", "late", "absent", "excused"];

/// Something attendance is taken for: a scheduled class (course day `day_number` of week
/// `week_number`) or one occurrence of a calendar event that takes attendance.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttendanceSession {
    /// Stable id: `class:{week}-{day}` or `event:{event_id}:{occurrence_date}`.
    pub key: String,
    /// `class` or `event`.
    pub kind: String,
    pub week_number: Option<i64>,
    pub day_number: Option<i64>,
    pub event_id: Option<String>,
    /// Event occurrence date (YYYY-MM-DD in the event timezone).
    pub occurrence_date: Option<String>,
    /// Event title; none for classes.
    pub title: Option<String>,
    /// Class date (YYYY-MM-DD); absent while the course has no start date.
    pub date: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub location: String,
    /// Number of students with a record for this class.
    pub marked: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, ToSchema)]
pub struct AttendanceRecord {
    /// Key of the session (see `AttendanceSession::key`).
    pub key: String,
    pub student_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct AttendanceSummary {
    pub student_id: String,
    pub present: i64,
    pub late: i64,
    pub absent: i64,
    pub excused: i64,
    /// Present or late out of all classes except excused ones; none before anything is marked.
    pub rate_percent: Option<f64>,
}

impl AttendanceSummary {
    pub fn from_records<'a>(student_id: &str, records: impl Iterator<Item = &'a AttendanceRecord>) -> Self {
        let mut s = AttendanceSummary { student_id: student_id.to_string(), ..Default::default() };
        for r in records.filter(|r| r.student_id == student_id) {
            match r.status.as_str() {
                "present" => s.present += 1,
                "late" => s.late += 1,
                "absent" => s.absent += 1,
                _ => s.excused += 1,
            }
        }
        let counted = s.present + s.late + s.absent;
        s.rate_percent = (counted > 0).then(|| ((s.present + s.late) as f64 / counted as f64 * 1000.0).round() / 10.0);
        s
    }
}

/// Instructor view of a course's attendance.
#[derive(Serialize, ToSchema)]
pub struct CourseAttendance {
    pub sessions: Vec<AttendanceSession>,
    pub students: Vec<UserSummary>,
    pub records: Vec<AttendanceRecord>,
    pub summaries: Vec<AttendanceSummary>,
}

/// A student's own attendance in one course.
#[derive(Serialize, ToSchema)]
pub struct MyAttendance {
    pub course_id: String,
    pub course_title: String,
    /// Classes that have started or were marked, with the student's status if any.
    pub sessions: Vec<MyAttendanceSession>,
    pub summary: AttendanceSummary,
}

#[derive(Serialize, ToSchema)]
pub struct MyAttendanceSession {
    #[serde(flatten)]
    pub session: AttendanceSession,
    pub status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AttendanceMark {
    pub student_id: String,
    /// One of `present`, `late`, `absent`, `excused`; empty or null clears the record.
    pub status: Option<String>,
}

/// Marks one session: either a class (`week_number` + `day_number`) or an event occurrence
/// (`event_id` + `occurrence_date`). `key` may be sent instead of either pair.
#[derive(Deserialize, ToSchema)]
pub struct AttendanceInput {
    pub key: Option<String>,
    pub week_number: Option<i64>,
    pub day_number: Option<i64>,
    pub event_id: Option<String>,
    pub occurrence_date: Option<String>,
    pub records: Vec<AttendanceMark>,
}

impl AttendanceInput {
    pub fn session_key(&self) -> Option<String> {
        if let Some(k) = self.key.as_deref().filter(|k| !k.is_empty()) {
            return Some(k.to_string());
        }
        match (self.week_number, self.day_number, self.event_id.as_deref(), self.occurrence_date.as_deref()) {
            (Some(w), Some(d), None, _) => Some(class_key(w, d)),
            (None, None, Some(e), Some(o)) => Some(event_key(e, o)),
            _ => None,
        }
    }
}

pub fn class_key(week: i64, day: i64) -> String {
    format!("class:{week}-{day}")
}

pub fn event_key(event_id: &str, date: &str) -> String {
    format!("event:{event_id}:{date}")
}
