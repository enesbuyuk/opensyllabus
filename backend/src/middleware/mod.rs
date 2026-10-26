//! HTTP middleware: sessions + CSRF, rate limiting, security headers.

pub mod rate_limit;
pub mod security_headers;
pub mod session;

use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::HeaderMap};

/// Client IP address, honoring proxy headers only when `TRUST_PROXY` is enabled.
#[derive(Clone, Debug)]
pub struct ClientIp(pub Option<String>);

pub fn client_ip(
    headers: &HeaderMap,
    connect: Option<&ConnectInfo<SocketAddr>>,
    trust_proxy: bool,
    ip_header: Option<&str>,
) -> Option<String> {
    if trust_proxy {
        // A configured CDN header wins, then X-Real-IP (set by the proxy itself), then the first X-Forwarded-For hop.
        for name in ip_header.into_iter().chain(["x-real-ip"]) {
            if let Some(ip) = headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim).filter(|s| !s.is_empty()) {
                return Some(ip.to_string());
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, v.parse().unwrap());
        }
        h
    }

    #[test]
    fn configured_header_wins_when_trusting_proxy() {
        let h = headers(&[("cf-connecting-ip", "203.0.113.7"), ("x-real-ip", "172.70.1.1"), ("x-forwarded-for", "172.70.1.1")]);
        assert_eq!(client_ip(&h, None, true, Some("cf-connecting-ip")).as_deref(), Some("203.0.113.7"));
        assert_eq!(client_ip(&h, None, true, None).as_deref(), Some("172.70.1.1"));
    }

    #[test]
    fn falls_back_when_configured_header_missing() {
        let h = headers(&[("x-forwarded-for", "198.51.100.2, 10.0.0.1")]);
        assert_eq!(client_ip(&h, None, true, Some("cf-connecting-ip")).as_deref(), Some("198.51.100.2"));
    }

    #[test]
    fn headers_ignored_without_trust_proxy() {
        let h = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        let connect = ConnectInfo("192.0.2.5:4000".parse::<SocketAddr>().unwrap());
        assert_eq!(client_ip(&h, Some(&connect), false, Some("cf-connecting-ip")).as_deref(), Some("192.0.2.5"));
    }
}
