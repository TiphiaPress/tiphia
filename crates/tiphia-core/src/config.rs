use crate::error::AppResult;
use std::time::Duration;

#[path = "config/env.rs"]
mod env;
#[path = "config/file.rs"]
mod file;
#[path = "config/model.rs"]
mod model;
#[path = "config/validation.rs"]
mod validation;

pub use model::{
    AppConfig, AuthConfig, Config, CorsConfig, DatabaseConfig, HttpConfig, LogConfig,
    RateLimitConfig,
};

impl Config {
    pub fn load() -> AppResult<Self> {
        let file_config = file::load_file_config()?;

        let config = Self {
            app: AppConfig {
                environment: env::env_or(
                    "TIPHIA_ENV",
                    file_config
                        .app
                        .as_ref()
                        .and_then(|config| config.environment.as_deref())
                        .unwrap_or("development"),
                ),
            },
            http: HttpConfig {
                bind: env::env_or(
                    "TIPHIA_BIND",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.bind.as_deref())
                        .unwrap_or("127.0.0.1:3000"),
                ),
                request_timeout: Duration::from_secs(env::env_or_parse(
                    "TIPHIA_REQUEST_TIMEOUT_SECS",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.request_timeout_secs)
                        .unwrap_or(30),
                )),
                max_body_bytes: env::env_or_parse(
                    "TIPHIA_MAX_BODY_BYTES",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.max_body_bytes)
                        .unwrap_or(1024 * 1024),
                ),
            },
            cors: CorsConfig {
                allowed_origins: env::env_list("TIPHIA_CORS_ALLOWED_ORIGINS").unwrap_or_else(
                    || {
                        file_config
                            .cors
                            .as_ref()
                            .and_then(|config| config.allowed_origins.clone())
                            .unwrap_or_default()
                    },
                ),
            },
            database: DatabaseConfig {
                url: env::env_or(
                    "DATABASE_URL",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.url.as_deref())
                        .unwrap_or("sqlite://tiphia.db?mode=rwc"),
                ),
                max_connections: env::env_or_parse(
                    "TIPHIA_DB_MAX_CONNECTIONS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.max_connections)
                        .unwrap_or(16),
                ),
                min_connections: env::env_or_parse(
                    "TIPHIA_DB_MIN_CONNECTIONS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.min_connections)
                        .unwrap_or(1),
                ),
                connect_timeout: Duration::from_secs(env::env_or_parse(
                    "TIPHIA_DB_CONNECT_TIMEOUT_SECS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.connect_timeout_secs)
                        .unwrap_or(10),
                )),
                acquire_timeout: Duration::from_secs(env::env_or_parse(
                    "TIPHIA_DB_ACQUIRE_TIMEOUT_SECS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.acquire_timeout_secs)
                        .unwrap_or(10),
                )),
            },
            log: LogConfig {
                level: env::env_or(
                    "RUST_LOG",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.level.as_deref())
                        .unwrap_or("tiphia=info,tower_http=info"),
                ),
                directory: env::env_or(
                    "TIPHIA_LOG_DIR",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.directory.as_deref())
                        .unwrap_or("logs"),
                ),
                file_prefix: env::env_or(
                    "TIPHIA_LOG_PREFIX",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.file_prefix.as_deref())
                        .unwrap_or("tiphia"),
                ),
                json: env::env_or_parse(
                    "TIPHIA_LOG_JSON",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.json)
                        .unwrap_or(true),
                ),
            },
            auth: AuthConfig {
                jwt_secret: env::env_or(
                    "TIPHIA_JWT_SECRET",
                    file_config
                        .auth
                        .as_ref()
                        .and_then(|config| config.jwt_secret.as_deref())
                        .unwrap_or("change-me-before-production-this-is-development-only"),
                ),
                token_ttl_seconds: env::env_or_parse(
                    "TIPHIA_TOKEN_TTL_SECONDS",
                    file_config
                        .auth
                        .as_ref()
                        .and_then(|config| config.token_ttl_seconds)
                        .unwrap_or(60 * 60 * 24 * 7),
                ),
            },
            rate_limit: RateLimitConfig {
                redis_url: env::optional_env("TIPHIA_REDIS_URL").or_else(|| {
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.redis_url.clone())
                }),
                login_per_minute: env::env_or_parse(
                    "TIPHIA_RATE_LIMIT_LOGIN_PER_MINUTE",
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.login_per_minute)
                        .unwrap_or(5),
                ),
                comments_per_minute: env::env_or_parse(
                    "TIPHIA_RATE_LIMIT_COMMENTS_PER_MINUTE",
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.comments_per_minute)
                        .unwrap_or(10),
                ),
            },
        };

        validation::validate_config(&config)?;
        Ok(config)
    }
}
