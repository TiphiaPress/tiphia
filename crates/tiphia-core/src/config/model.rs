use serde::Deserialize;
use std::time::Duration;

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
pub(super) struct FileConfig {
    pub app: Option<FileAppConfig>,
    pub http: Option<FileHttpConfig>,
    pub cors: Option<FileCorsConfig>,
    pub database: Option<FileDatabaseConfig>,
    pub log: Option<FileLogConfig>,
    pub auth: Option<FileAuthConfig>,
    pub rate_limit: Option<FileRateLimitConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileAppConfig {
    pub environment: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileHttpConfig {
    pub bind: Option<String>,
    pub request_timeout_secs: Option<u64>,
    pub max_body_bytes: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileCorsConfig {
    pub allowed_origins: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileDatabaseConfig {
    pub url: Option<String>,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub connect_timeout_secs: Option<u64>,
    pub acquire_timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileLogConfig {
    pub level: Option<String>,
    pub directory: Option<String>,
    pub file_prefix: Option<String>,
    pub json: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileAuthConfig {
    pub jwt_secret: Option<String>,
    pub token_ttl_seconds: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct FileRateLimitConfig {
    pub redis_url: Option<String>,
    pub login_per_minute: Option<u32>,
    pub comments_per_minute: Option<u32>,
}
