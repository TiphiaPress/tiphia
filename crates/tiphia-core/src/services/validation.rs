use crate::{
    error::{AppError, AppResult},
    services::auth,
};

pub fn required(value: &str, field: &'static str) -> AppResult<()> {
    auth::validate_required(value, field)
}

pub fn max_len(value: &str, max: usize, field: &'static str) -> AppResult<()> {
    if value.chars().count() > max {
        return Err(AppError::Validation(format!(
            "{field} must be at most {max} characters"
        )));
    }

    Ok(())
}

pub fn email(value: &str, field: &'static str) -> AppResult<()> {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return Err(AppError::Validation(format!(
            "{field} must be a valid email"
        )));
    };

    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
        || value.chars().any(char::is_whitespace)
    {
        return Err(AppError::Validation(format!(
            "{field} must be a valid email"
        )));
    }

    Ok(())
}

pub fn optional_http_url(value: Option<&str>, field: &'static str) -> AppResult<()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    let valid_scheme = value.starts_with("http://") || value.starts_with("https://");
    let has_host = value
        .split_once("://")
        .map(|(_, rest)| !rest.split('/').next().unwrap_or_default().is_empty())
        .unwrap_or(false);

    if !valid_scheme || !has_host || value.chars().any(char::is_whitespace) {
        return Err(AppError::Validation(format!(
            "{field} must be a valid http or https URL"
        )));
    }

    Ok(())
}

pub fn slug(value: &str) -> AppResult<()> {
    let valid = value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if value.is_empty() || !valid || value.starts_with('-') || value.ends_with('-') {
        return Err(AppError::Validation(
            "slug must use lowercase letters, numbers, and hyphens".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_email() {
        assert!(email("bad-email", "email").is_err());
        assert!(email("a@example.com", "email").is_ok());
    }

    #[test]
    fn validates_slug_shape() {
        assert!(slug("hello-world-123").is_ok());
        assert!(slug("Hello").is_err());
        assert!(slug("-hello").is_err());
    }
}
