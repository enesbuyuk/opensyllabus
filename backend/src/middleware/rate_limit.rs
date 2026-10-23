//! Minimal in-memory fixed-window rate limiter (single-instance deployment).

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::errors::{AppError, AppResult};

#[derive(Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    /// Records a hit for `key` and fails once `limit` hits occurred within `window`.
    pub fn check(&self, key: &str, limit: u32, window: Duration) -> AppResult<()> {
        let mut map = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        if map.len() > 10_000 {
            map.retain(|_, (_, start)| now.duration_since(*start) < window);
        }
        let entry = map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) >= window {
            *entry = (0, now);
        }
        entry.0 += 1;
        if entry.0 > limit {
            Err(AppError::RateLimited)
        } else {
            Ok(())
        }
    }

    pub fn reset(&self, key: &str) {
        self.buckets.lock().unwrap_or_else(|e| e.into_inner()).remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_after_threshold() {
        let rl = RateLimiter::default();
        for _ in 0..3 {
            rl.check("k", 3, Duration::from_secs(60)).unwrap();
        }
        assert!(rl.check("k", 3, Duration::from_secs(60)).is_err());
        rl.reset("k");
        assert!(rl.check("k", 3, Duration::from_secs(60)).is_ok());
    }
}
