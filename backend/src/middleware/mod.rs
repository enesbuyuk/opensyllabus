//! HTTP middleware: sessions + CSRF, rate limiting, security headers.

pub mod rate_limit;
pub mod security_headers;
pub mod session;

use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::HeaderMap};

/// Client IP address, honoring proxy headers only when `TRUST_PROXY` is enabled.
#[derive(Clone, Debug)]
pub struct ClientIp(pub Option<String>);

pub fn client_ip(headers: &HeaderMap, connect: Option<&ConnectInfo<SocketAddr>>, trust_proxy: bool) -> Option<String> {
    if trust_proxy {
        // Prefer X-Real-IP (set by the proxy itself), then the first X-Forwarded-For hop.
        if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()).map(str::trim).filter(|s| !s.is_empty()) {
            return Some(ip.to_string());
        }
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(ip);
        }
    }
    connect.map(|c| c.0.ip().to_string())
}
