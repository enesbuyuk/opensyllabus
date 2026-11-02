//! Test harness: an in-process app on a temporary database, with per-user cookie clients.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use opensyllabus::{build_router, config::Config, models::user::Role, repositories::db, services::accounts, AppState};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

pub const ADMIN_EMAIL: &str = "instructor@example.com";
pub const ADMIN_PASSWORD: &str = "instructor-pass-1";

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    _dir: TempDir,
}

pub struct Client<'a> {
    app: &'a TestApp,
    cookie: Option<String>,
    pub csrf: Option<String>,
}

pub struct Resp {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Value,
    pub raw: Vec<u8>,
}

impl TestApp {
    pub async fn new() -> Self {
        Self::with_config(|_, _| {}).await
    }

    /// Like `new`, but lets a test adjust the config (the closure also gets the temp dir).
    pub async fn with_config(adjust: impl FnOnce(&mut Config, &std::path::Path)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::for_tests(dir.path());
        adjust(&mut config, dir.path());
        std::fs::create_dir_all(&config.storage_dir).unwrap();
        let pool = db::connect(&config.database_path).await.unwrap();
        db::migrate(&pool).await.unwrap();
        accounts::create_user(&pool, ADMIN_EMAIL, None, "Instructor", Role::Admin, Some(ADMIN_PASSWORD.into()), "Europe/Istanbul")
            .await
            .unwrap();
        let state = AppState::new(pool, config).await.unwrap();
        Self { router: build_router(state.clone()), state, _dir: dir }
    }

    pub fn client(&self) -> Client<'_> {
        Client { app: self, cookie: None, csrf: None }
    }

    pub async fn admin(&self) -> Client<'_> {
        let mut c = self.client();
        c.login(ADMIN_EMAIL, ADMIN_PASSWORD).await.assert(StatusCode::OK);
        c
    }
}

impl Resp {
    pub fn assert(self, status: StatusCode) -> Self {
        assert_eq!(self.status, status, "unexpected status; body: {}", self.body);
        self
    }
}

impl<'a> Client<'a> {
    pub async fn send(&mut self, method: Method, uri: &str, body: Option<Value>) -> Resp {
        let mut req = Request::builder().method(method.clone()).uri(uri);
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        if method != Method::GET {
            if let Some(t) = &self.csrf {
                req = req.header("x-csrf-token", t);
            }
        }
        let req = match body {
            Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())).unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        self.dispatch(req).await
    }

    pub async fn dispatch(&mut self, req: Request<Body>) -> Resp {
        let res = self.app.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        if let Some(sc) = headers.get(header::SET_COOKIE).and_then(|v| v.to_str().ok()) {
            let pair = sc.split(';').next().unwrap().to_string();
            if pair.ends_with('=') {
                self.cookie = None;
            } else {
                self.cookie = Some(pair);
            }
        }
        let raw = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        let body = serde_json::from_slice(&raw).unwrap_or(Value::Null);
        Resp { status, headers, body, raw }
    }

    pub async fn get(&mut self, uri: &str) -> Resp {
        self.send(Method::GET, uri, None).await
    }
    pub async fn post(&mut self, uri: &str, body: Value) -> Resp {
        self.send(Method::POST, uri, Some(body)).await
    }
    pub async fn put(&mut self, uri: &str, body: Value) -> Resp {
        self.send(Method::PUT, uri, Some(body)).await
    }
    pub async fn patch(&mut self, uri: &str, body: Value) -> Resp {
        self.send(Method::PATCH, uri, Some(body)).await
    }
    pub async fn delete(&mut self, uri: &str) -> Resp {
        self.send(Method::DELETE, uri, None).await
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Resp {
        let r = self.post("/api/v1/auth/login", json!({ "email": email, "password": password })).await;
        if r.status == StatusCode::OK {
            self.csrf = r.body["csrf_token"].as_str().map(String::from);
        }
        r
    }

    /// Multipart upload of a single `file` field.
    pub async fn upload(&mut self, uri: &str, filename: &str, content: &[u8]) -> Resp {
        let boundary = "----opensyllabus-test-boundary";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let mut req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"));
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        if let Some(t) = &self.csrf {
            req = req.header("x-csrf-token", t);
        }
        self.dispatch(req.body(Body::from(body)).unwrap()).await
    }
}

/// Creates a published course with one published week containing three published lessons.
/// Returns (course_id, week_id, lesson_ids).
pub async fn published_course(admin: &mut Client<'_>) -> (String, String, Vec<String>) {
    let course = admin
        .post("/api/v1/courses", json!({ "title": "Test Course", "duration_weeks": 4, "days_per_week": 3 }))
        .await
        .assert(StatusCode::CREATED);
    let course_id = course.body["id"].as_str().unwrap().to_string();
    admin.patch(&format!("/api/v1/courses/{course_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    let week = admin
        .post(&format!("/api/v1/courses/{course_id}/weeks"), json!({ "title": "Week 1", "create_day_lessons": true }))
        .await
        .assert(StatusCode::CREATED);
    let week_id = week.body["id"].as_str().unwrap().to_string();
    let detail = admin.get(&format!("/api/v1/weeks/{week_id}")).await.assert(StatusCode::OK);
    let lesson_ids: Vec<String> =
        detail.body["lessons"].as_array().unwrap().iter().map(|l| l["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(lesson_ids.len(), 3);
    (course_id, week_id, lesson_ids)
}

pub async fn create_student(admin: &mut Client<'_>, email: &str, course_id: &str) -> String {
    let r = admin
        .post(
            "/api/v1/users",
            json!({ "email": email, "name": email.split('@').next().unwrap(), "password": "student-pass-1", "course_ids": [course_id] }),
        )
        .await
        .assert(StatusCode::CREATED);
    r.body["user"]["id"].as_str().unwrap().to_string()
}
