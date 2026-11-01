//! HTTP handlers. Each module owns one resource family; routes are wired in `router`.

pub mod analytics;
pub mod announcements;
pub mod assignments;
pub mod attendance;
pub mod auth;
pub mod calendar;
pub mod courses;
pub mod curriculum;
pub mod dashboard;
pub mod files;
pub mod groups;
pub mod health;
pub mod notifications;
pub mod progress;
pub mod resources;
pub mod router;
pub mod search;
pub mod settings;
pub mod submissions;
pub mod users;

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};

use crate::errors::AppError;

/// JSON body extractor that reports malformed input with the standard error shape.
pub struct ApiJson<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequest<S> for ApiJson<T> {
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(ApiJson(v)),
            Err(JsonRejection::JsonDataError(e)) => Err(AppError::BadRequest(format!("Invalid request body: {}", e.body_text()))),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(AppError::bad_request("Expected an application/json body")),
            Err(e) => Err(AppError::BadRequest(format!("Invalid JSON: {}", e.body_text()))),
        }
    }
}

pub fn json<T: Serialize>(v: T) -> Response {
    axum::Json(v).into_response()
}

pub fn created<T: Serialize>(v: T) -> Response {
    (StatusCode::CREATED, axum::Json(v)).into_response()
}

pub fn no_content() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct IdResponse {
    pub id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CountResponse {
    pub count: i64,
}
