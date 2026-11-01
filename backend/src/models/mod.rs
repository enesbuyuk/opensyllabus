//! Database row types and API data-transfer objects.

pub mod assignment;
pub mod attendance;
pub mod course;
pub mod curriculum;
pub mod group;
pub mod misc;
pub mod submission;
pub mod user;

pub use sqlx::types::Json;
