use crate::error::{AppError, AppResult};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::PasswordHash(err.to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|err| AppError::PasswordHash(err.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn validate_required(value: &str, field: &'static str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }

    Ok(())
}

pub fn validate_password(value: &str) -> AppResult<()> {
    if value.len() < 12 {
        return Err(AppError::Validation(
            "password must be at least 12 characters".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_policy_requires_minimum_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough-password").is_ok());
    }

    #[test]
    fn password_hash_round_trips() {
        let hash = hash_password("long-enough-password").expect("hash");
        assert!(verify_password("long-enough-password", &hash).expect("verify"));
        assert!(!verify_password("different-password", &hash).expect("verify"));
    }
}
