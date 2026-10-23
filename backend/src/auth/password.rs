//! Argon2id password hashing.

use std::sync::LazyLock;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

use crate::errors::{AppError, AppResult};

pub const MIN_PASSWORD_LEN: usize = 10;
pub const MAX_PASSWORD_LEN: usize = 128;

fn argon2() -> Argon2<'static> {
    // OWASP-recommended Argon2id parameters (19 MiB memory, 2 iterations, 1 lane).
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(19 * 1024, 2, 1, None).expect("valid params"))
}

/// Hash used to equalize timing when the account does not exist.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| hash_sync("dummy-password-for-timing").unwrap());

pub fn validate_strength(password: &str) -> AppResult<()> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_LEN {
        return Err(AppError::field("new_password", &format!("Password must be at least {MIN_PASSWORD_LEN} characters")));
    }
    if len > MAX_PASSWORD_LEN {
        return Err(AppError::field("new_password", &format!("Password must be at most {MAX_PASSWORD_LEN} characters")));
    }
    let has_letter = password.chars().any(char::is_alphabetic);
    let has_other = password.chars().any(|c| !c.is_alphabetic());
    if !(has_letter && has_other) {
        return Err(AppError::field("new_password", "Password must contain letters and at least one number or symbol"));
    }
    Ok(())
}

fn hash_sync(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    argon2().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| anyhow::anyhow!("hashing password: {e}"))
}

fn verify_sync(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => argon2().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

pub async fn hash(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_sync(&password)).await.map_err(|e| AppError::Internal(e.into()))?.map_err(AppError::Internal)
}

/// Verifies a password; pass `None` to burn equivalent time for unknown accounts.
pub async fn verify(password: String, hash: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || match hash {
        Some(h) => verify_sync(&password, &h),
        None => {
            let _ = verify_sync(&password, &DUMMY_HASH);
            false
        }
    })
    .await
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let h = hash_sync("correct horse 1").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_sync("correct horse 1", &h));
        assert!(!verify_sync("wrong", &h));
    }

    #[test]
    fn strength_rules() {
        assert!(validate_strength("short1").is_err());
        assert!(validate_strength("onlyletterslong").is_err());
        assert!(validate_strength("letters-and-1").is_ok());
    }
}
