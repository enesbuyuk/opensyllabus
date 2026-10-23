mod common;

use axum::http::{header, Method, Request, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn login_me_logout() {
    let app = TestApp::new().await;
    let mut c = app.client();
    c.get("/api/v1/auth/me").await.assert(StatusCode::UNAUTHORIZED);

    let r = c.login(ADMIN_EMAIL, ADMIN_PASSWORD).await.assert(StatusCode::OK);
    assert_eq!(r.body["user"]["role"], "admin");
    assert!(r.body["user"].get("password_hash").is_none(), "password hash must never be serialized");
    let cookie = r.headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"));

    c.get("/api/v1/auth/me").await.assert(StatusCode::OK);
    c.post("/api/v1/auth/logout", json!({})).await.assert(StatusCode::NO_CONTENT);
    c.get("/api/v1/auth/me").await.assert(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_credentials_and_rate_limit() {
    let app = TestApp::new().await;
    let mut c = app.client();
    let r = c.login(ADMIN_EMAIL, "wrong-password-1").await.assert(StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "invalid_credentials");
    c.login("nobody@example.com", "whatever-123").await.assert(StatusCode::UNAUTHORIZED);
    for _ in 0..9 {
        c.login(ADMIN_EMAIL, "wrong-password-1").await;
    }
    c.login(ADMIN_EMAIL, ADMIN_PASSWORD).await.assert(StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn csrf_and_origin_are_enforced() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let token = admin.csrf.take();
    admin.post("/api/v1/courses", json!({ "title": "No CSRF" })).await.assert(StatusCode::FORBIDDEN);
    admin.csrf = Some("wrong".into());
    admin.post("/api/v1/courses", json!({ "title": "Bad CSRF" })).await.assert(StatusCode::FORBIDDEN);
    admin.csrf = token;
    admin.post("/api/v1/courses", json!({ "title": "With CSRF" })).await.assert(StatusCode::CREATED);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::ORIGIN, "https://evil.example")
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(json!({ "email": ADMIN_EMAIL, "password": ADMIN_PASSWORD }).to_string()))
        .unwrap();
    app.client().dispatch(req).await.assert(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn students_cannot_use_admin_endpoints() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    create_student(&mut admin, "s1@example.com", &course_id).await;

    let mut s = app.client();
    s.login("s1@example.com", "student-pass-1").await.assert(StatusCode::OK);
    s.get("/api/v1/users").await.assert(StatusCode::FORBIDDEN);
    s.post("/api/v1/users", json!({ "email": "x@example.com", "name": "X" })).await.assert(StatusCode::FORBIDDEN);
    s.post("/api/v1/courses", json!({ "title": "Mine" })).await.assert(StatusCode::FORBIDDEN);
    s.get("/api/v1/submissions").await.assert(StatusCode::FORBIDDEN);
    s.get(&format!("/api/v1/courses/{course_id}/gradebook")).await.assert(StatusCode::FORBIDDEN);
    s.get("/api/v1/analytics").await.assert(StatusCode::FORBIDDEN);
    s.get("/api/v1/audit-logs").await.assert(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn setup_link_reset_and_password_change() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let r = admin.post("/api/v1/users", json!({ "email": "new@example.com", "name": "New Student" })).await.assert(StatusCode::CREATED);
    let link = r.body["setup_link"].as_str().expect("setup link when no password given");
    let token = link.split("token=").nth(1).unwrap().split('&').next().unwrap();

    let mut s = app.client();
    s.post("/api/v1/auth/reset-password", json!({ "token": token, "new_password": "short" }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);
    s.post("/api/v1/auth/reset-password", json!({ "token": token, "new_password": "brand-new-pass-1" }))
        .await
        .assert(StatusCode::NO_CONTENT);
    // Tokens are single-use.
    s.post("/api/v1/auth/reset-password", json!({ "token": token, "new_password": "another-pass-1" }))
        .await
        .assert(StatusCode::BAD_REQUEST);
    s.login("new@example.com", "brand-new-pass-1").await.assert(StatusCode::OK);

    s.post("/api/v1/auth/change-password", json!({ "current_password": "nope-nope-1", "new_password": "changed-pass-1" }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);
    s.post("/api/v1/auth/change-password", json!({ "current_password": "brand-new-pass-1", "new_password": "changed-pass-1" }))
        .await
        .assert(StatusCode::NO_CONTENT);
    app.client().login("new@example.com", "changed-pass-1").await.assert(StatusCode::OK);
}

#[tokio::test]
async fn deactivated_users_lose_access() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    let sid = create_student(&mut admin, "s1@example.com", &course_id).await;
    let mut s = app.client();
    s.login("s1@example.com", "student-pass-1").await.assert(StatusCode::OK);
    admin.patch(&format!("/api/v1/users/{sid}"), json!({ "is_active": false })).await.assert(StatusCode::OK);
    s.get("/api/v1/auth/me").await.assert(StatusCode::UNAUTHORIZED);
    app.client().login("s1@example.com", "student-pass-1").await.assert(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn security_headers_present() {
    let app = TestApp::new().await;
    let r = app.client().get("/api/health").await.assert(StatusCode::OK);
    assert_eq!(r.headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(r.headers.get("x-frame-options").unwrap(), "DENY");
    assert!(r.headers.get("content-security-policy").is_some());
    app.client().get("/api/v1/does-not-exist").await.assert(StatusCode::NOT_FOUND);
    app.client().get("/api/nope").await.assert(StatusCode::NOT_FOUND);
}
