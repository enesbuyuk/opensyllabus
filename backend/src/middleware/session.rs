//! Resolves the session cookie and enforces CSRF + Origin checks on mutating requests.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::{client_ip, ClientIp};
use crate::{
    auth::{
        session::{self, CSRF_HEADER},
        AuthContext,
    },
    errors::AppError,
    AppState,
};

pub async fn session_middleware(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    // Static frontend assets need neither sessions nor CSRF checks.
    if !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }
    let ip = client_ip(
        req.headers(),
        req.extensions().get::<ConnectInfo<SocketAddr>>(),
        state.config.trust_proxy,
        state.config.client_ip_header.as_deref(),
    );
    req.extensions_mut().insert(ClientIp(ip));

    let mutating = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);

    // Reject cross-site mutating requests outright when the browser tells us the origin.
    if mutating {
        if let Some(origin) = req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
            let origin = origin.trim_end_matches('/');
            if !state.config.allowed_origins().iter().any(|o| o == origin) {
                return AppError::Forbidden("Cross-origin request rejected".into()).into_response();
            }
        }
    }

    let token = req.headers().get(header::COOKIE).and_then(|v| v.to_str().ok()).and_then(session::token_from_cookie_header);

    if let Some(token) = token {
        match session::resolve(&state.pool, &state.config, &token).await {
            Ok(Some(resolved)) => {
                if mutating {
                    let provided = req.headers().get(CSRF_HEADER).and_then(|v| v.to_str().ok()).unwrap_or("");
                    if !constant_time_eq(provided.as_bytes(), resolved.csrf_token.as_bytes()) {
                        return AppError::Forbidden("Invalid or missing CSRF token".into()).into_response();
                    }
                }
                req.extensions_mut().insert(AuthContext {
                    user: resolved.user,
                    session_id: resolved.session_id,
                    csrf_token: resolved.csrf_token,
                });
            }
            Ok(None) => {}
            Err(e) => return e.into_response(),
        }
    }

    next.run(req).await
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
