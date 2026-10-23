//! OpenSyllabus backend library.

pub mod auth;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod openapi;
pub mod repositories;
pub mod services;
pub mod util;

use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;

use crate::{config::Config, middleware::rate_limit::RateLimiter, models::misc::PlatformSettings};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub rate_limiter: Arc<RateLimiter>,
    pub settings: Arc<RwLock<PlatformSettings>>,
}

impl AppState {
    pub async fn new(pool: SqlitePool, config: Config) -> anyhow::Result<Self> {
        let settings = services::settings::load(&pool, &config).await?;
        Ok(Self {
            pool,
            config: Arc::new(config),
            rate_limiter: Arc::new(RateLimiter::default()),
            settings: Arc::new(RwLock::new(settings)),
        })
    }

    pub fn timezone(&self) -> chrono_tz::Tz {
        self.settings.read().ok().and_then(|s| s.default_timezone.parse().ok()).unwrap_or(chrono_tz::UTC)
    }

    /// Today's date (YYYY-MM-DD) in the platform timezone; drives week availability.
    pub fn today(&self) -> String {
        chrono::Utc::now().with_timezone(&self.timezone()).format("%Y-%m-%d").to_string()
    }
}

pub use handlers::router::build_router;
