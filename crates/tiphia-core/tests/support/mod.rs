#![allow(dead_code)]

use std::sync::Arc;

use sea_orm::{Database, DatabaseConnection};
use tiphia_core::{
    AppState, Config,
    config::{
        AppConfig, AuthConfig, CorsConfig, DatabaseConfig, HttpConfig, LogConfig, RateLimitConfig,
    },
    migration::run_core_migrations,
    plugins::{PluginRegistry, PluginRegistryBuilder},
    rate_limit::RateLimiter,
};

pub async fn state() -> AppState {
    state_with_plugins(|_| Ok(())).await
}

pub async fn state_with_plugins<F>(register_plugins: F) -> AppState
where
    F: FnOnce(&mut PluginRegistryBuilder) -> tiphia_core::AppResult<()>,
{
    let db = database().await;
    let config = config();
    let plugins = PluginRegistry::boot(db.clone(), register_plugins)
        .await
        .expect("plugin registry");
    let rate_limiter = RateLimiter::new(&config.rate_limit)
        .await
        .expect("rate limiter");

    AppState {
        db,
        plugins: Arc::new(plugins),
        config: Arc::new(config),
        rate_limiter,
    }
}

pub async fn database() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("connect sqlite");
    run_core_migrations(&db).await.expect("run migrations");
    db
}

pub fn config() -> Config {
    Config {
        app: AppConfig {
            environment: "test".to_owned(),
        },
        http: HttpConfig {
            bind: "127.0.0.1:0".to_owned(),
            request_timeout: std::time::Duration::from_secs(30),
            max_body_bytes: 1024 * 1024,
        },
        cors: CorsConfig {
            allowed_origins: Vec::new(),
        },
        database: DatabaseConfig {
            url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            min_connections: 1,
            connect_timeout: std::time::Duration::from_secs(10),
            acquire_timeout: std::time::Duration::from_secs(10),
        },
        log: LogConfig {
            level: "debug".to_owned(),
            directory: "logs".to_owned(),
            file_prefix: "tiphia-test".to_owned(),
            json: false,
        },
        auth: AuthConfig {
            jwt_secret: "test-secret-with-enough-entropy-for-hs256".to_owned(),
            token_ttl_seconds: 3600,
        },
        rate_limit: RateLimitConfig {
            redis_url: None,
            login_per_minute: 0,
            comments_per_minute: 0,
        },
    }
}

#[allow(dead_code)]
pub fn empty_plugins(_: &mut PluginRegistryBuilder) -> tiphia_core::AppResult<()> {
    Ok(())
}
