use crate::{
    error::{AppError, AppResult},
    services::settings::SiteSettings,
};

pub fn validate(settings: &SiteSettings) -> AppResult<()> {
    if settings.title.trim().is_empty() {
        return Err(AppError::Validation("title is required".to_owned()));
    }

    if !(1..=100).contains(&settings.default_page_size) {
        return Err(AppError::Validation(
            "default_page_size must be between 1 and 100".to_owned(),
        ));
    }

    if !settings.permalink_format.contains("{slug}") {
        return Err(AppError::Validation(
            "permalink_format must contain {slug}".to_owned(),
        ));
    }

    if let Some(avatar_url) = non_empty(settings.avatar_url.as_deref()) {
        validate_avatar_url(avatar_url)?;
    }

    if let Some(gravatar_base_url) = non_empty(settings.gravatar_base_url.as_deref()) {
        crate::services::validation::optional_http_url(
            Some(gravatar_base_url),
            "gravatar_base_url",
        )?;
    }

    Ok(())
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub fn validate_avatar_url(value: &str) -> AppResult<()> {
    if value.starts_with('/') && !value.starts_with("//") && !value.chars().any(char::is_whitespace)
    {
        return Ok(());
    }

    crate::services::validation::optional_http_url(Some(value), "avatar_url")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_url_accepts_http_https_or_site_relative_paths() {
        assert!(validate_avatar_url("https://example.com/avatar.png").is_ok());
        assert!(validate_avatar_url("/assets/avatar.png").is_ok());
        assert!(validate_avatar_url("avatar.png").is_err());
        assert!(validate_avatar_url("//example.com/avatar.png").is_err());
    }
}
