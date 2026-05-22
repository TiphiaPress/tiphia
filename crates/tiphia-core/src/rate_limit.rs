use crate::{
    app::AppState,
    config::RateLimitConfig,
    error::{AppError, AppResult},
};
use axum::http::{HeaderMap, header};
use redis::AsyncCommands;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    backend: RateLimiterBackend,
}

#[derive(Clone)]
enum RateLimiterBackend {
    Memory(MemoryRateLimiter),
    Redis(Arc<Mutex<redis::aio::ConnectionManager>>),
}

#[derive(Clone, Default)]
struct MemoryRateLimiter {
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

#[derive(Clone)]
struct Bucket {
    count: u32,
    reset_at: Instant,
}

impl RateLimiter {
    pub async fn new(config: &RateLimitConfig) -> AppResult<Self> {
        if let Some(redis_url) = &config.redis_url {
            let client = redis::Client::open(redis_url.as_str())
                .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
            let manager = client
                .get_connection_manager()
                .await
                .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
            return Ok(Self {
                backend: RateLimiterBackend::Redis(Arc::new(Mutex::new(manager))),
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
                redis_check(manager.clone(), key.into(), limit, window).await
            }
        }
    }
}

impl MemoryRateLimiter {
    async fn check(&self, key: String, limit: u32, window: Duration) -> AppResult<()> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets.entry(key).or_insert_with(|| Bucket {
            count: 0,
            reset_at: now + window,
        });

        if now >= bucket.reset_at {
            bucket.count = 0;
            bucket.reset_at = now + window;
        }

        if bucket.count >= limit {
            return Err(AppError::RateLimited);
        }

        bucket.count += 1;
        Ok(())
    }
}

async fn redis_check(
    manager: Arc<Mutex<redis::aio::ConnectionManager>>,
    key: String,
    limit: u32,
    window: Duration,
) -> AppResult<()> {
    let redis_key = format!("tiphia:rate-limit:{key}");
    let mut connection = manager.lock().await;
    let count: u32 = connection
        .incr(&redis_key, 1_u32)
        .await
        .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;

    if count == 1 {
        let _: () = connection
            .expire(&redis_key, window.as_secs() as i64)
            .await
            .map_err(|err| AppError::RateLimitBackend(err.to_string()))?;
    }

    if count > limit {
        return Err(AppError::RateLimited);
    }

    Ok(())
}

pub async fn check_login(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    state
        .rate_limiter
        .check(
            format!("login:{}", client_key(headers)),
            state.config.rate_limit.login_per_minute,
            Duration::from_secs(60),
        )
        .await
}

pub async fn check_comment(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    state
        .rate_limiter
        .check(
            format!("comment:{}", client_key(headers)),
            state.config.rate_limit.comments_per_minute,
            Duration::from_secs(60),
        )
        .await
}

fn client_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .or_else(|| headers.get(header::USER_AGENT))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}
