use crate::error::{AppError, AppResult};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct Config {
    pub app: AppConfig,
    pub http: HttpConfig,
    pub cors: CorsConfig,
    pub database: DatabaseConfig,
    pub log: LogConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub environment: String,
}

#[derive(Clone, Debug)]
pub struct HttpConfig {
    pub bind: String,
    pub request_timeout: Duration,
    pub max_body_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub acquire_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct LogConfig {
    pub level: String,
    pub directory: String,
    pub file_prefix: String,
    pub json: bool,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_ttl_seconds: i64,
}

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub redis_url: Option<String>,
    pub login_per_minute: u32,
    pub comments_per_minute: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileConfig {
    app: Option<FileAppConfig>,
    http: Option<FileHttpConfig>,
    cors: Option<FileCorsConfig>,
    database: Option<FileDatabaseConfig>,
    log: Option<FileLogConfig>,
    auth: Option<FileAuthConfig>,
    rate_limit: Option<FileRateLimitConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileAppConfig {
    environment: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileHttpConfig {
    bind: Option<String>,
    request_timeout_secs: Option<u64>,
    max_body_bytes: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileCorsConfig {
    allowed_origins: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileDatabaseConfig {
    url: Option<String>,
    max_connections: Option<u32>,
    min_connections: Option<u32>,
    connect_timeout_secs: Option<u64>,
    acquire_timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileLogConfig {
    level: Option<String>,
    directory: Option<String>,
    file_prefix: Option<String>,
    json: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileAuthConfig {
    jwt_secret: Option<String>,
    token_ttl_seconds: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FileRateLimitConfig {
    redis_url: Option<String>,
    login_per_minute: Option<u32>,
    comments_per_minute: Option<u32>,
}

impl Config {
    pub fn load() -> AppResult<Self> {
        let file_config = load_file_config()?;

        let config = Self {
            app: AppConfig {
                environment: env_or(
                    "TIPHIA_ENV",
                    file_config
                        .app
                        .as_ref()
                        .and_then(|config| config.environment.as_deref())
                        .unwrap_or("development"),
                ),
            },
            http: HttpConfig {
                bind: env_or(
                    "TIPHIA_BIND",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.bind.as_deref())
                        .unwrap_or("127.0.0.1:3000"),
                ),
                request_timeout: Duration::from_secs(env_or_parse(
                    "TIPHIA_REQUEST_TIMEOUT_SECS",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.request_timeout_secs)
                        .unwrap_or(30),
                )),
                max_body_bytes: env_or_parse(
                    "TIPHIA_MAX_BODY_BYTES",
                    file_config
                        .http
                        .as_ref()
                        .and_then(|config| config.max_body_bytes)
                        .unwrap_or(1024 * 1024),
                ),
            },
            cors: CorsConfig {
                allowed_origins: env_list("TIPHIA_CORS_ALLOWED_ORIGINS").unwrap_or_else(|| {
                    file_config
                        .cors
                        .as_ref()
                        .and_then(|config| config.allowed_origins.clone())
                        .unwrap_or_default()
                }),
            },
            database: DatabaseConfig {
                url: env_or(
                    "DATABASE_URL",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.url.as_deref())
                        .unwrap_or("sqlite://tiphia.db?mode=rwc"),
                ),
                max_connections: env_or_parse(
                    "TIPHIA_DB_MAX_CONNECTIONS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.max_connections)
                        .unwrap_or(16),
                ),
                min_connections: env_or_parse(
                    "TIPHIA_DB_MIN_CONNECTIONS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.min_connections)
                        .unwrap_or(1),
                ),
                connect_timeout: Duration::from_secs(env_or_parse(
                    "TIPHIA_DB_CONNECT_TIMEOUT_SECS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.connect_timeout_secs)
                        .unwrap_or(10),
                )),
                acquire_timeout: Duration::from_secs(env_or_parse(
                    "TIPHIA_DB_ACQUIRE_TIMEOUT_SECS",
                    file_config
                        .database
                        .as_ref()
                        .and_then(|config| config.acquire_timeout_secs)
                        .unwrap_or(10),
                )),
            },
            log: LogConfig {
                level: env_or(
                    "RUST_LOG",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.level.as_deref())
                        .unwrap_or("tiphia=info,tower_http=info"),
                ),
                directory: env_or(
                    "TIPHIA_LOG_DIR",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.directory.as_deref())
                        .unwrap_or("logs"),
                ),
                file_prefix: env_or(
                    "TIPHIA_LOG_PREFIX",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.file_prefix.as_deref())
                        .unwrap_or("tiphia"),
                ),
                json: env_or_parse(
                    "TIPHIA_LOG_JSON",
                    file_config
                        .log
                        .as_ref()
                        .and_then(|config| config.json)
                        .unwrap_or(true),
                ),
            },
            auth: AuthConfig {
                jwt_secret: env_or(
                    "TIPHIA_JWT_SECRET",
                    file_config
                        .auth
                        .as_ref()
                        .and_then(|config| config.jwt_secret.as_deref())
                        .unwrap_or("change-me-before-production-this-is-development-only"),
                ),
                token_ttl_seconds: env_or_parse(
                    "TIPHIA_TOKEN_TTL_SECONDS",
                    file_config
                        .auth
                        .as_ref()
                        .and_then(|config| config.token_ttl_seconds)
                        .unwrap_or(60 * 60 * 24 * 7),
                ),
            },
            rate_limit: RateLimitConfig {
                redis_url: optional_env("TIPHIA_REDIS_URL").or_else(|| {
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.redis_url.clone())
                }),
                login_per_minute: env_or_parse(
                    "TIPHIA_RATE_LIMIT_LOGIN_PER_MINUTE",
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.login_per_minute)
                        .unwrap_or(5),
                ),
                comments_per_minute: env_or_parse(
                    "TIPHIA_RATE_LIMIT_COMMENTS_PER_MINUTE",
                    file_config
                        .rate_limit
                        .as_ref()
                        .and_then(|config| config.comments_per_minute)
                        .unwrap_or(10),
                ),
            },
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> AppResult<()> {
        self.http
            .bind
            .parse::<SocketAddr>()
            .map_err(|err| AppError::Config(format!("invalid http.bind: {err}")))?;

        if self.database.url.trim().is_empty() {
            return Err(AppError::Config("database.url is required".to_owned()));
        }
        if self.database.max_connections == 0 {
            return Err(AppError::Config(
                "database.max_connections must be greater than 0".to_owned(),
            ));
        }
        if self.database.min_connections > self.database.max_connections {
            return Err(AppError::Config(
                "database.min_connections must be less than or equal to database.max_connections"
                    .to_owned(),
            ));
        }
        if self.database.connect_timeout.is_zero() || self.database.acquire_timeout.is_zero() {
            return Err(AppError::Config(
                "database timeouts must be greater than 0".to_owned(),
            ));
        }

        if self.http.max_body_bytes == 0 {
            return Err(AppError::Config(
                "http.max_body_bytes must be greater than 0".to_owned(),
            ));
        }

        if self.app.environment == "production"
            && self.auth.jwt_secret == "change-me-before-production-this-is-development-only"
        {
            return Err(AppError::Config(
                "auth.jwt_secret must be changed from the development default".to_owned(),
            ));
        }

        for origin in &self.cors.allowed_origins {
            if !(origin.starts_with("http://") || origin.starts_with("https://")) {
                return Err(AppError::Config(format!(
                    "invalid cors.allowed_origins entry: {origin}"
                )));
            }
        }

        if let Some(redis_url) = &self.rate_limit.redis_url
            && !redis_url.starts_with("redis://")
            && !redis_url.starts_with("rediss://")
        {
            return Err(AppError::Config(
                "rate_limit.redis_url must start with redis:// or rediss://".to_owned(),
            ));
        }

        Ok(())
    }
}

fn load_file_config() -> AppResult<FileConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(FileConfig::default());
    }

    let content = std::fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|err| AppError::Config(err.to_string()))
}

fn config_path() -> PathBuf {
    std::env::var("TIPHIA_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("tiphia.toml"))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn optional_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_list(key: &str) -> Option<Vec<String>> {
    std::env::var(key).ok().map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

fn env_or_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
{
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
