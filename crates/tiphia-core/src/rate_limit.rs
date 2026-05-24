use crate::{app::AppState, config::RateLimitConfig, error::AppResult};
use axum::http::HeaderMap;
use std::time::Duration;

#[path = "rate_limit/key.rs"]
mod key;
#[path = "rate_limit/memory.rs"]
mod memory;
#[path = "rate_limit/redis.rs"]
mod redis_backend;

use memory::MemoryRateLimiter;
use redis_backend::RedisRateLimiter;

#[derive(Clone)]
pub struct RateLimiter {
    backend: RateLimiterBackend,
}

#[derive(Clone)]
enum RateLimiterBackend {
    Memory(MemoryRateLimiter),
    Redis(RedisRateLimiter),
}

impl RateLimiter {
    pub async fn new(config: &RateLimitConfig) -> AppResult<Self> {
        if let Some(redis_url) = &config.redis_url {
            return Ok(Self {
                backend: RateLimiterBackend::Redis(redis_backend::connect(redis_url).await?),
            });
        }

        Ok(Self {
            backend: RateLimiterBackend::Memory(MemoryRateLimiter::default()),
        })
    }

    pub async fn check(
        &self,
        key: impl Into<String>,
        limit: u32,
        window: Duration,
    ) -> AppResult<()> {
        if limit == 0 {
            return Ok(());
        }

        match &self.backend {
            RateLimiterBackend::Memory(memory) => memory.check(key.into(), limit, window).await,
            RateLimiterBackend::Redis(manager) => {
                redis_backend::check(manager.clone(), key.into(), limit, window).await
            }
        }
    }
}

pub async fn check_login(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    state
        .rate_limiter
        .check(
            format!("login:{}", key::client_key(headers)),
            state.config.rate_limit.login_per_minute,
            Duration::from_secs(60),
        )
        .await
}

pub async fn check_comment(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    state
        .rate_limiter
        .check(
            format!("comment:{}", key::client_key(headers)),
            state.config.rate_limit.comments_per_minute,
            Duration::from_secs(60),
        )
        .await
}
