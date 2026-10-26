//! Small shared helpers: identifiers, timestamps, hashing, validation.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::errors::{AppError, AppResult};

/// New time-ordered UUIDv7 identifier.
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Canonical UTC timestamp format stored in the database.
pub fn fmt_ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn now() -> String {
    fmt_ts(Utc::now())
}

/// Parse a client-provided timestamp (any RFC 3339 offset) and normalize to UTC.
pub fn parse_ts(field: &str, value: &str) -> AppResult<String> {
    DateTime::parse_from_rfc3339(value)
        .map(|t| fmt_ts(t.with_timezone(&Utc)))
        .map_err(|_| AppError::field(field, "Must be an ISO-8601 date-time with timezone"))
}

pub fn parse_opt_ts(field: &str, value: Option<&str>) -> AppResult<Option<String>> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => parse_ts(field, v).map(Some),
        None => Ok(None),
    }
}

pub fn parse_date(field: &str, value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| AppError::field(field, "Must be a date in YYYY-MM-DD format"))
}

pub fn parse_opt_date(field: &str, value: Option<&str>) -> AppResult<Option<String>> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => parse_date(field, v).map(|d| Some(d.format("%Y-%m-%d").to_string())),
        None => Ok(None),
    }
}

/// Cryptographically random URL-safe token.
pub fn random_token(bytes: usize) -> String {
    use base64::Engine;
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn sha256_hex(input: &[u8]) -> String {
    hex::encode(Sha256::digest(input))
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in input.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').chars().take(80).collect()
}

/// Collects field-level validation errors.
#[derive(Default)]
pub struct Validator(std::collections::BTreeMap<String, String>);

impl Validator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, ok: bool, field: &str, msg: &str) -> &mut Self {
        if !ok && !self.0.contains_key(field) {
            self.0.insert(field.to_string(), msg.to_string());
        }
        self
    }

    pub fn len(&mut self, value: &str, field: &str, min: usize, max: usize) -> &mut Self {
        let n = value.trim().chars().count();
        if n < min {
            let msg = if min == 1 { "This field is required".to_string() } else { format!("Must be at least {min} characters") };
            self.check(false, field, &msg);
        } else if n > max {
            self.check(false, field, &format!("Must be at most {max} characters"));
        }
        self
    }

    pub fn url(&mut self, value: Option<&str>, field: &str) -> &mut Self {
        if let Some(v) = value.filter(|v| !v.trim().is_empty()) {
            self.check(is_http_url(v), field, "Must be a valid http(s) URL");
        }
        self
    }

    pub fn finish(&mut self) -> AppResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(std::mem::take(&mut self.0)))
        }
    }
}

pub fn is_http_url(v: &str) -> bool {
    match url::Url::parse(v.trim()) {
        Ok(u) => matches!(u.scheme(), "http" | "https") && u.host_str().is_some() && v.len() <= 2048,
        Err(_) => false,
    }
}

pub fn is_valid_email(v: &str) -> bool {
    let v = v.trim();
    let re = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    v.len() <= 254 && re.is_match(v)
}

/// Login name: 3-32 characters of a-z, 0-9, ".", "_" and "-", starting with a letter or digit.
/// Never contains "@", so sign-in can tell it apart from an email address.
pub fn is_valid_username(v: &str) -> bool {
    let re = regex::Regex::new(r"^[a-z0-9][a-z0-9._-]{2,31}$").unwrap();
    re.is_match(v)
}

pub fn is_valid_timezone(v: &str) -> bool {
    v.parse::<chrono_tz::Tz>().is_ok()
}

/// Trims a string and converts empty input to None.
pub fn non_empty(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn to_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
}

pub fn from_json_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Validates a list of short text items (objectives etc.).
pub fn clean_list(items: Vec<String>, field: &str, max_items: usize) -> AppResult<Vec<String>> {
    let items: Vec<String> = items.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if items.len() > max_items {
        return Err(AppError::field(field, &format!("At most {max_items} items are allowed")));
    }
    if items.iter().any(|s| s.chars().count() > 500) {
        return Err(AppError::field(field, "Each item must be at most 500 characters"));
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_works() {
        assert_eq!(slugify("Light-Based Tactile Robotic Hand"), "light-based-tactile-robotic-hand");
        assert_eq!(slugify("  Hello,  World!  "), "hello-world");
    }

    #[test]
    fn timestamps_normalize_to_utc() {
        assert_eq!(parse_ts("x", "2026-03-01T12:00:00+03:00").unwrap(), "2026-03-01T09:00:00.000Z");
        assert!(parse_ts("x", "2026-03-01 12:00").is_err());
    }

    #[test]
    fn url_validation() {
        assert!(is_http_url("https://github.com/user/repo"));
        assert!(!is_http_url("javascript:alert(1)"));
        assert!(!is_http_url("ftp://example.com"));
    }

    #[test]
    fn email_validation() {
        assert!(is_valid_email("student@example.com"));
        assert!(!is_valid_email("not-an-email"));
        assert!(is_valid_username("enes.buyuk"));
        assert!(is_valid_username("s_01"));
        assert!(!is_valid_username("ab"));
        assert!(!is_valid_username("Enes"));
        assert!(!is_valid_username("a@b.com"));
        assert!(!is_valid_username(".hidden"));
    }
}
