//! Authentication and authorization primitives.

pub mod access;
pub mod password;
pub mod session;

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{errors::AppError, models::user::User};

/// Authenticated request context, inserted by the session middleware.
#[derive(Clone)]
pub struct AuthContext {
    pub user: User,
    pub session_id: String,
    pub csrf_token: String,
}

/// Any authenticated, active user.
pub struct AuthUser(pub AuthContext);

/// An authenticated administrator / instructor.
pub struct AdminUser(pub AuthContext);

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<AuthContext>().cloned().map(AuthUser).ok_or(AppError::Unauthorized)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts.extensions.get::<AuthContext>().cloned().ok_or(AppError::Unauthorized)?;
        if !ctx.user.is_admin() {
            return Err(AppError::forbidden());
        }
        Ok(AdminUser(ctx))
    }
}

impl std::ops::Deref for AuthUser {
    type Target = User;
    fn deref(&self) -> &User {
        &self.0.user
    }
}

impl std::ops::Deref for AdminUser {
    type Target = User;
    fn deref(&self) -> &User {
        &self.0.user
    }
}
