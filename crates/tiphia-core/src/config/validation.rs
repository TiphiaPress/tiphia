use crate::{
    config::Config,
    error::{AppError, AppResult},
};
use std::net::SocketAddr;

pub(super) fn validate_config(config: &Config) -> AppResult<()> {
    config
        .http
        .bind
        .parse::<SocketAddr>()
        .map_err(|err| AppError::Config(format!("invalid http.bind: {err}")))?;

    if config.database.url.trim().is_empty() {
        return Err(AppError::Config("database.url is required".to_owned()));
    }
    if config.database.max_connections == 0 {
        return Err(AppError::Config(
            "database.max_connections must be greater than 0".to_owned(),
        ));
    }
    if config.database.min_connections > config.database.max_connections {
        return Err(AppError::Config(
            "database.min_connections must be less than or equal to database.max_connections"
                .to_owned(),
        ));
    }
    if config.database.connect_timeout.is_zero() || config.database.acquire_timeout.is_zero() {
        return Err(AppError::Config(
            "database timeouts must be greater than 0".to_owned(),
        ));
    }

    if config.http.max_body_bytes == 0 {
        return Err(AppError::Config(
            "http.max_body_bytes must be greater than 0".to_owned(),
        ));
    }

    if config.app.environment == "production"
        && config.auth.jwt_secret == "change-me-before-production-this-is-development-only"
    {
        return Err(AppError::Config(
            "auth.jwt_secret must be changed from the development default".to_owned(),
        ));
    }

    for origin in &config.cors.allowed_origins {
        if !(origin.starts_with("http://") || origin.starts_with("https://")) {
            return Err(AppError::Config(format!(
                "invalid cors.allowed_origins entry: {origin}"
            )));
        }
    }

    if let Some(redis_url) = &config.rate_limit.redis_url
        && !redis_url.starts_with("redis://")
        && !redis_url.starts_with("rediss://")
    {
        return Err(AppError::Config(
            "rate_limit.redis_url must start with redis:// or rediss://".to_owned(),
        ));
    }

    Ok(())
}
