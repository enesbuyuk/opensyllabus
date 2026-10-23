//! Environment-based configuration.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind_addr: SocketAddr,
    /// SQLite database file path.
    pub database_path: PathBuf,
    /// Private directory for uploaded files (never served statically).
    pub storage_dir: PathBuf,
    /// Optional directory containing the built frontend (served with SPA fallback).
    pub static_dir: Option<PathBuf>,
    /// Public origin, e.g. `https://lms.example.com`.
    pub public_url: String,
    /// Mark cookies `Secure` and send HSTS. Defaults to true when `public_url` is https.
    pub cookie_secure: bool,
    /// Trust `X-Forwarded-For` / `X-Real-IP` from the reverse proxy.
    pub trust_proxy: bool,
    /// Session lifetime (sliding) in hours.
    pub session_ttl_hours: i64,
    /// Default timezone for the platform (IANA name).
    pub default_timezone: String,
    /// Upper bound for any single upload, in megabytes.
    pub max_upload_mb: u64,
    /// Extra origins allowed to send mutating requests (development only).
    pub extra_allowed_origins: Vec<String>,
    /// Seed the initial "Light-Based Tactile Robotic Hand" course on first start.
    pub seed_initial_course: bool,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let public_url = env_or("PUBLIC_URL", "http://localhost:8080").trim_end_matches('/').to_string();
        let https = public_url.starts_with("https://");
        let default_timezone = env_or("DEFAULT_TIMEZONE", "UTC");
        default_timezone.parse::<chrono_tz::Tz>().map_err(|_| anyhow::anyhow!("DEFAULT_TIMEZONE is not a valid IANA timezone"))?;

        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080").parse().context("BIND_ADDR must be host:port")?,
            database_path: env_or("DATABASE_PATH", "./data/opensyllabus.db").into(),
            storage_dir: env_or("STORAGE_DIR", "./storage").into(),
            static_dir: std::env::var("STATIC_DIR").ok().filter(|s| !s.is_empty()).map(Into::into),
            cookie_secure: env_bool("COOKIE_SECURE", https),
            trust_proxy: env_bool("TRUST_PROXY", false),
            session_ttl_hours: env_or("SESSION_TTL_HOURS", "168").parse().context("SESSION_TTL_HOURS must be an integer")?,
            default_timezone,
            max_upload_mb: env_or("MAX_UPLOAD_MB", "50").parse().context("MAX_UPLOAD_MB must be an integer")?,
            extra_allowed_origins: env_or("ALLOWED_ORIGINS", "")
                .split(',')
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            seed_initial_course: env_bool("SEED_INITIAL_COURSE", false),
            public_url,
        })
    }

    /// Configuration used by integration tests.
    pub fn for_tests(dir: &std::path::Path) -> Self {
        Self {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_path: dir.join("test.db"),
            storage_dir: dir.join("storage"),
            static_dir: None,
            public_url: "http://localhost:8080".into(),
            cookie_secure: false,
            trust_proxy: false,
            session_ttl_hours: 24,
            default_timezone: "UTC".into(),
            max_upload_mb: 10,
            extra_allowed_origins: vec![],
            seed_initial_course: false,
        }
    }

    pub fn allowed_origins(&self) -> Vec<String> {
        let mut v = vec![self.public_url.clone()];
        v.extend(self.extra_allowed_origins.iter().cloned());
        v
    }
}
